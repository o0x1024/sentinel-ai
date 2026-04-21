use anyhow::{anyhow, Result};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;
use tokio_rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct MemoryLexicalDocument {
    pub id: String,
    pub collection_name: String,
    pub title: Option<String>,
    pub body: String,
    pub normalized_text: String,
    pub identifiers: String,
    pub tags: String,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
    pub importance: u8,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct MemoryLexicalHit {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
    pub importance: u8,
    pub created_at_ms: i64,
    pub bm25_score: f64,
}

#[derive(Debug, Clone)]
pub struct MemoryDurableMetadata {
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
}

pub struct MemoryLexicalIndex {
    database_path: String,
}

static IDENTIFIER_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)\bCVE-\d{4}-\d{4,}\b").expect("valid cve regex"),
        Regex::new(r"(?i)\bhttps?://[^\s]+").expect("valid url regex"),
        Regex::new(r"(?i)\b[a-z0-9._-]+\.[a-z]{2,}(?::\d+)?(?:/[^\s]*)?")
            .expect("valid host regex"),
        Regex::new(r"(?i)(?:[a-z]:)?(?:/|\\)[^\s]+").expect("valid path regex"),
        Regex::new(r"(?i)\b(?:\./)?(?:[a-z0-9_.-]+/)+[a-z0-9_.-]+\b")
            .expect("valid relative path regex"),
        Regex::new(r"(?i)\b[a-z0-9_]+::[a-z0-9_:+<>-]+\b").expect("valid rust path regex"),
        Regex::new(r"(?i)\b[a-z0-9_]+(?:[.-][a-z0-9_]+){1,}\b")
            .expect("valid dotted identifier regex"),
    ]
});

impl MemoryLexicalIndex {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }

    pub async fn initialize(&self) -> Result<()> {
        let db_path = Path::new(&self.database_path);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let conn = Connection::open(&self.database_path)
            .await
            .map_err(|e| anyhow!("Failed to open memory lexical DB: {}", e))?;

        conn.call(|conn| {
            conn.execute_batch(
                r#"
                PRAGMA journal_mode = WAL;

                CREATE TABLE IF NOT EXISTS memory_documents (
                  id TEXT PRIMARY KEY,
                  collection_name TEXT NOT NULL,
                  title TEXT,
                  body TEXT NOT NULL,
                  normalized_text TEXT,
                  identifiers TEXT,
                  tags TEXT,
                  kind TEXT,
                  scope TEXT DEFAULT 'project',
                  stability TEXT DEFAULT 'stable',
                  source TEXT DEFAULT 'unknown',
                  confidence REAL DEFAULT 0.7,
                  importance INTEGER DEFAULT 3,
                  created_at_ms INTEGER NOT NULL,
                  updated_at_ms INTEGER NOT NULL
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS memory_documents_fts USING fts5(
                  title,
                  body,
                  normalized_text,
                  identifiers,
                  tags,
                  content='memory_documents',
                  content_rowid='rowid',
                  tokenize='unicode61 remove_diacritics 2 tokenchars ''-_./:#'''
                );

                CREATE INDEX IF NOT EXISTS idx_memory_documents_collection
                ON memory_documents(collection_name);

                CREATE INDEX IF NOT EXISTS idx_memory_documents_kind
                ON memory_documents(kind);

                CREATE INDEX IF NOT EXISTS idx_memory_documents_updated
                ON memory_documents(updated_at_ms DESC);

                CREATE TRIGGER IF NOT EXISTS memory_documents_ai AFTER INSERT ON memory_documents BEGIN
                  INSERT INTO memory_documents_fts(rowid, title, body, normalized_text, identifiers, tags)
                  VALUES (new.rowid, new.title, new.body, new.normalized_text, new.identifiers, new.tags);
                END;

                CREATE TRIGGER IF NOT EXISTS memory_documents_ad AFTER DELETE ON memory_documents BEGIN
                  INSERT INTO memory_documents_fts(memory_documents_fts, rowid, title, body, normalized_text, identifiers, tags)
                  VALUES('delete', old.rowid, old.title, old.body, old.normalized_text, old.identifiers, old.tags);
                END;

                CREATE TRIGGER IF NOT EXISTS memory_documents_au AFTER UPDATE ON memory_documents BEGIN
                  INSERT INTO memory_documents_fts(memory_documents_fts, rowid, title, body, normalized_text, identifiers, tags)
                  VALUES('delete', old.rowid, old.title, old.body, old.normalized_text, old.identifiers, old.tags);
                  INSERT INTO memory_documents_fts(rowid, title, body, normalized_text, identifiers, tags)
                  VALUES (new.rowid, new.title, new.body, new.normalized_text, new.identifiers, new.tags);
                END;
                "#,
            )?;
            ensure_memory_documents_column(
                conn,
                "scope",
                "TEXT DEFAULT 'project'",
                "'project'",
            )?;
            ensure_memory_documents_column(
                conn,
                "stability",
                "TEXT DEFAULT 'stable'",
                "'stable'",
            )?;
            ensure_memory_documents_column(conn, "source", "TEXT DEFAULT 'unknown'", "'unknown'")?;
            ensure_memory_documents_column(conn, "confidence", "REAL DEFAULT 0.7", "0.7")?;
            conn.execute_batch(
                r#"
                CREATE INDEX IF NOT EXISTS idx_memory_documents_scope
                ON memory_documents(scope);

                CREATE INDEX IF NOT EXISTS idx_memory_documents_stability
                ON memory_documents(stability);
                "#,
            )?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Failed to initialize memory lexical schema: {}", e))?;

        Ok(())
    }

    pub async fn upsert_document(&self, doc: &MemoryLexicalDocument) -> Result<()> {
        let conn = Connection::open(&self.database_path)
            .await
            .map_err(|e| anyhow!("Failed to open memory lexical DB: {}", e))?;
        let doc = doc.clone();
        conn.call(move |conn| {
            conn.execute(
                r#"
                INSERT INTO memory_documents (
                  id,
                  collection_name,
                  title,
                  body,
                  normalized_text,
                  identifiers,
                  tags,
                  kind,
                  scope,
                  stability,
                  source,
                  confidence,
                  importance,
                  created_at_ms,
                  updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                ON CONFLICT(id) DO UPDATE SET
                  collection_name = excluded.collection_name,
                  title = excluded.title,
                  body = excluded.body,
                  normalized_text = excluded.normalized_text,
                  identifiers = excluded.identifiers,
                  tags = excluded.tags,
                  kind = excluded.kind,
                  scope = excluded.scope,
                  stability = excluded.stability,
                  source = excluded.source,
                  confidence = excluded.confidence,
                  importance = excluded.importance,
                  updated_at_ms = excluded.updated_at_ms
                "#,
                rusqlite::params![
                    doc.id,
                    doc.collection_name,
                    doc.title,
                    doc.body,
                    doc.normalized_text,
                    doc.identifiers,
                    doc.tags,
                    doc.kind,
                    doc.scope,
                    doc.stability,
                    doc.source,
                    doc.confidence,
                    i64::from(doc.importance),
                    doc.created_at_ms,
                    doc.updated_at_ms,
                ],
            )?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Failed to upsert lexical memory document: {}", e))?;

        Ok(())
    }

    pub async fn search(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryLexicalHit>> {
        let conn = Connection::open(&self.database_path)
            .await
            .map_err(|e| anyhow!("Failed to open memory lexical DB: {}", e))?;
        let collection_name = collection_name.to_string();
        let match_query = build_match_query(query);
        let limit = top_k.max(1) as i64;

        if match_query.is_empty() {
            return Ok(Vec::new());
        }

        let rows = conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                SELECT
                  d.id,
                  d.title,
                  d.body,
                  d.kind,
                  d.scope,
                  d.stability,
                  d.source,
                  d.confidence,
                  d.importance,
                  d.created_at_ms,
                  bm25(memory_documents_fts, 2.0, 1.2, 0.9, 1.4, 0.6) AS rank
                FROM memory_documents_fts
                JOIN memory_documents d ON d.rowid = memory_documents_fts.rowid
                WHERE d.collection_name = ?1
                  AND memory_documents_fts MATCH ?2
                ORDER BY rank ASC, d.updated_at_ms DESC
                LIMIT ?3
                "#,
                )?;

                let rows = stmt
                    .query_map(
                        rusqlite::params![collection_name, match_query, limit],
                        |row| {
                            Ok(MemoryLexicalHit {
                                id: row.get(0)?,
                                title: row.get(1)?,
                                body: row.get(2)?,
                                kind: row.get(3)?,
                                scope: row.get(4)?,
                                stability: row.get(5)?,
                                source: row.get(6)?,
                                confidence: row.get::<_, f64>(7)?,
                                importance: row.get::<_, i64>(8)?.clamp(1, 5) as u8,
                                created_at_ms: row.get(9)?,
                                bm25_score: row.get(10)?,
                            })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
            .map_err(|e| anyhow!("Failed to search lexical memory index: {}", e))?;

        Ok(rows)
    }
}

pub fn normalize_memory_text(input: &str) -> String {
    collapse_whitespace(input).to_lowercase()
}

pub fn extract_memory_identifiers(input: &str) -> String {
    let mut seen = std::collections::BTreeSet::new();
    for regex in IDENTIFIER_REGEXES.iter() {
        for matched in regex.find_iter(input) {
            let value = matched.as_str().trim();
            if !value.is_empty() {
                seen.insert(value.to_lowercase());
            }
        }
    }
    seen.into_iter().collect::<Vec<_>>().join(" ")
}

pub fn build_memory_document_id(kind: &str, body: &str) -> String {
    format!(
        "mem-{:x}",
        md5::compute(format!("{}\n{}", kind.trim(), body.trim()).as_bytes())
    )
}

pub fn canonicalize_memory_scope(scope: &str) -> String {
    match scope.trim().to_lowercase().replace(' ', "_").as_str() {
        "repo" | "workspace" | "codebase" => "project".to_string(),
        "execution" | "conversation" => "session".to_string(),
        "org" | "organization" => "team".to_string(),
        "" => "project".to_string(),
        value => value.to_string(),
    }
}

pub fn canonicalize_memory_stability(stability: &str) -> String {
    match stability.trim().to_lowercase().replace(' ', "_").as_str() {
        "ephemeral" | "temporary" | "provisional" => "tentative".to_string(),
        "verified" | "confirmed" | "durable" => "stable".to_string(),
        "" => "stable".to_string(),
        value => value.to_string(),
    }
}

pub fn canonicalize_memory_source(source: &str) -> String {
    match source.trim().to_lowercase().replace(' ', "_").as_str() {
        "tool" | "manual" => "memory_tool".to_string(),
        "run_state" | "context" | "context_engineering" => "context_engineering".to_string(),
        "backfill" | "migration" => "legacy".to_string(),
        "" => "unknown".to_string(),
        value => value.to_string(),
    }
}

pub fn canonicalize_memory_kind(kind: &str) -> String {
    let normalized = kind.trim().to_lowercase().replace(' ', "_");
    match normalized.as_str() {
        "pref" | "prefs" | "user_preference" | "preferences" => "preference".to_string(),
        "anti-pattern" | "antipattern" | "pitfall" | "avoidance" => "anti_pattern".to_string(),
        "sop_hint" | "sop-hint" | "runbook" | "playbook" | "procedure" | "checklist" => {
            "sop".to_string()
        }
        "decision_log" | "decision_record" => "decision".to_string(),
        "evidence" | "indicator" | "pattern" | "ioc" => "evidence_pattern".to_string(),
        // `todo` is preserved as a read alias for historical memory rows and
        // prompts. `task` is the only canonical kind we emit going forward.
        "todo" | "next_step" | "next-step" => "task".to_string(),
        "" => "fact".to_string(),
        _ => normalized,
    }
}

pub fn infer_memory_kind(
    explicit_kind: Option<&str>,
    title: Option<&str>,
    tags: &[String],
    content: &str,
) -> String {
    if let Some(kind) = explicit_kind {
        let canonical = canonicalize_memory_kind(kind);
        if canonical != "fact" || !kind.trim().is_empty() {
            return canonical;
        }
    }

    for tag in tags {
        let canonical = canonicalize_memory_kind(tag);
        if canonical != "fact" || tag.trim().eq_ignore_ascii_case("fact") {
            if canonical != "fact" || tag.trim().eq_ignore_ascii_case("fact") {
                return canonical;
            }
        }
    }

    if let Some(raw_title) = title.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(rest) = raw_title.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let kind = canonicalize_memory_kind(&rest[..end]);
                if kind != "fact" {
                    return kind;
                }
            }
        }
        let lower = raw_title.to_lowercase();
        if lower.contains("偏好") || lower.contains("preference") {
            return "preference".to_string();
        }
        if lower.contains("反模式")
            || lower.contains("anti-pattern")
            || lower.contains("avoid")
            || lower.contains("陷阱")
        {
            return "anti_pattern".to_string();
        }
        if lower.contains("sop")
            || lower.contains("runbook")
            || lower.contains("playbook")
            || lower.contains("checklist")
            || lower.contains("流程")
        {
            return "sop".to_string();
        }
        if lower.contains("decision") || lower.contains("决定") || lower.contains("结论") {
            return "decision".to_string();
        }
    }

    let lower = content.to_lowercase();
    if lower.contains("prefer ")
        || lower.contains("default to")
        || lower.contains("always use")
        || lower.contains("偏好")
        || lower.contains("优先使用")
    {
        return "preference".to_string();
    }
    if lower.contains("avoid ")
        || lower.contains("never ")
        || lower.contains("do not ")
        || lower.contains("不要")
        || lower.contains("避免")
    {
        return "anti_pattern".to_string();
    }
    if lower.contains("step 1")
        || lower.contains("checklist")
        || lower.contains("runbook")
        || lower.contains("playbook")
        || lower.contains("步骤")
        || lower.contains("流程")
    {
        return "sop".to_string();
    }
    if lower.contains("we decided")
        || lower.contains("decided to")
        || lower.contains("decision:")
        || lower.contains("决定")
        || lower.contains("结论")
    {
        return "decision".to_string();
    }
    if lower.contains("indicator")
        || lower.contains("iocs")
        || lower.contains("特征")
        || lower.contains("迹象")
    {
        return "evidence_pattern".to_string();
    }
    if lower.contains("todo")
        || lower.contains("task")
        || lower.contains("next step")
        || lower.contains("待办")
    {
        return "task".to_string();
    }

    "fact".to_string()
}

pub fn build_memory_durable_metadata(
    explicit_scope: Option<&str>,
    explicit_stability: Option<&str>,
    explicit_source: Option<&str>,
    explicit_confidence: Option<&str>,
    kind: &str,
    tags: &[String],
) -> MemoryDurableMetadata {
    let scope = explicit_scope
        .map(canonicalize_memory_scope)
        .or_else(|| {
            tags.iter().find_map(|tag| {
                let canonical = canonicalize_memory_scope(tag);
                matches!(
                    canonical.as_str(),
                    "user" | "team" | "project" | "global" | "session"
                )
                .then_some(canonical)
            })
        })
        .unwrap_or_else(|| "project".to_string());

    let normalized_kind = canonicalize_memory_kind(kind);
    let stability = explicit_stability
        .map(canonicalize_memory_stability)
        .unwrap_or_else(|| match normalized_kind.as_str() {
            "task" | "fact" => "tentative".to_string(),
            _ => "stable".to_string(),
        });

    let source = explicit_source
        .map(canonicalize_memory_source)
        .unwrap_or_else(|| "unknown".to_string());

    let confidence = explicit_confidence
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or_else(|| match normalized_kind.as_str() {
            "decision" => 0.90,
            "preference" | "anti_pattern" | "sop" => 0.86,
            "evidence_pattern" => 0.82,
            "fact" => 0.72,
            "task" => 0.58,
            _ => 0.70,
        });

    MemoryDurableMetadata {
        scope,
        stability,
        source,
        confidence,
    }
}

pub fn memory_kind_importance(kind: &str) -> u8 {
    match canonicalize_memory_kind(kind).as_str() {
        "decision" => 4,
        "preference" | "anti_pattern" | "sop" | "evidence_pattern" => 4,
        "fact" | "task" => 3,
        _ => 3,
    }
}

fn ensure_memory_documents_column(
    conn: &rusqlite::Connection,
    column_name: &str,
    column_definition: &str,
    default_sql: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(memory_documents)")?;
    let existing_columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if existing_columns
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(column_name))
    {
        return Ok(());
    }

    conn.execute(
        &format!(
            "ALTER TABLE memory_documents ADD COLUMN {} {}",
            column_name, column_definition
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "UPDATE memory_documents SET {0} = {1} WHERE {0} IS NULL",
            column_name, default_sql
        ),
        [],
    )?;
    Ok(())
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_match_query(input: &str) -> String {
    let normalized = normalize_memory_text(input);
    if normalized.is_empty() {
        return String::new();
    }

    let mut terms = normalized
        .split(|ch: char| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '#'))
        })
        .filter(|term| !term.trim().is_empty())
        .map(|term| term.trim().to_string())
        .collect::<Vec<_>>();

    let identifiers = extract_memory_identifiers(input);
    if !identifiers.is_empty() {
        terms.extend(identifiers.split_whitespace().map(str::to_string));
    }

    terms.sort();
    terms.dedup();

    if terms.is_empty() {
        return format!("\"{}\"", escape_fts_phrase(normalized.as_str()));
    }

    terms
        .into_iter()
        .map(|term| format!("\"{}\"", escape_fts_phrase(term.as_str())))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn escape_fts_phrase(input: &str) -> String {
    input.replace('"', "\"\"")
}

#[cfg(test)]
mod tests {
    use super::{
        build_match_query, build_memory_durable_metadata, canonicalize_memory_kind,
        canonicalize_memory_scope, canonicalize_memory_source, canonicalize_memory_stability,
        extract_memory_identifiers, infer_memory_kind, normalize_memory_text,
    };

    #[test]
    fn normalize_memory_text_collapses_whitespace() {
        assert_eq!(
            normalize_memory_text("  CVE-2024-1234   on Example.COM/path "),
            "cve-2024-1234 on example.com/path"
        );
    }

    #[test]
    fn extract_memory_identifiers_keeps_security_anchors() {
        let identifiers = extract_memory_identifiers(
            "Investigate CVE-2024-1234 on https://app.example.com/api/login using src/main.rs",
        );
        assert!(identifiers.contains("cve-2024-1234"));
        assert!(identifiers.contains("https://app.example.com/api/login"));
        assert!(identifiers.contains("src/main.rs"));
    }

    #[test]
    fn build_match_query_quotes_terms() {
        let query = build_match_query("CVE-2024-1234 app.example.com");
        assert!(query.contains("\"cve-2024-1234\""));
        assert!(query.contains("\"app.example.com\""));
    }

    #[test]
    fn canonicalize_memory_kind_maps_aliases() {
        assert_eq!(canonicalize_memory_kind("prefs"), "preference");
        assert_eq!(canonicalize_memory_kind("anti-pattern"), "anti_pattern");
        assert_eq!(canonicalize_memory_kind("runbook"), "sop");
        assert_eq!(canonicalize_memory_kind("todo"), "task");
    }

    #[test]
    fn infer_memory_kind_prefers_tags_and_content_signals() {
        let tags = vec!["playbook".to_string()];
        assert_eq!(
            infer_memory_kind(None, Some("Memory"), &tags, "step 1 verify host"),
            "sop"
        );
        assert_eq!(
            infer_memory_kind(None, None, &[], "Avoid broad wildcard scans on production"),
            "anti_pattern"
        );
        assert_eq!(
            infer_memory_kind(
                None,
                None,
                &[],
                "Prefer ripgrep for searches in large repos"
            ),
            "preference"
        );
        assert_eq!(
            infer_memory_kind(
                None,
                None,
                &[],
                "Todo: verify changed result before final answer"
            ),
            "task"
        );
    }

    #[test]
    fn canonicalize_memory_metadata_aliases() {
        assert_eq!(canonicalize_memory_scope("workspace"), "project");
        assert_eq!(canonicalize_memory_stability("verified"), "stable");
        assert_eq!(canonicalize_memory_source("manual"), "memory_tool");
    }

    #[test]
    fn build_memory_durable_metadata_defaults_from_kind() {
        let metadata =
            build_memory_durable_metadata(None, None, Some("context"), None, "task", &[]);
        assert_eq!(metadata.scope, "project");
        assert_eq!(metadata.stability, "tentative");
        assert_eq!(metadata.source, "context_engineering");
        assert!(metadata.confidence < 0.7);
    }
}
