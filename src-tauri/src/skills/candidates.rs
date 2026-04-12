use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use sentinel_db::{Database, DatabaseService};

use crate::skills::skills_root;

const SKILL_CANDIDATES_DIR: &str = "_candidates";
const SKILL_CANDIDATE_FEEDBACK_DIR: &str = "_candidate_feedback";
const DEFAULT_SUPPRESSION_RULE_TTL_DAYS: i64 = 21;
const SUPPRESSION_RULE_NOTE_MAX_LEN: usize = 160;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCandidate {
    pub id: String,
    pub title: String,
    pub suggested_skill_name: String,
    pub description: String,
    pub content: String,
    pub memory_kind: String,
    pub source_memory_id: Option<String>,
    pub scope: String,
    pub source: String,
    #[serde(default = "default_skill_candidate_stability")]
    pub stability: String,
    pub confidence: f64,
    #[serde(default)]
    pub memory_excerpt: String,
    #[serde(default)]
    pub capture_reasons: Vec<String>,
    pub status: String,
    pub promoted_skill_id: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_at_ms: Option<i64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct SkillCandidateMemoryInput {
    pub memory_id: Option<String>,
    pub text: String,
    pub kind: String,
    pub scope: String,
    pub source: String,
    pub stability: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCandidateSuppressionRule {
    pub id: String,
    pub memory_kind: String,
    pub source: String,
    #[serde(default)]
    pub category: String,
    pub note: Option<String>,
    pub excerpt: String,
    pub match_terms: Vec<String>,
    #[serde(default)]
    pub hit_count: u64,
    pub last_matched_at_ms: Option<i64>,
    pub last_matched_excerpt: Option<String>,
    #[serde(default = "default_suppression_rule_ttl_days")]
    pub ttl_days: i64,
    pub expires_at_ms: Option<i64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

fn default_suppression_rule_ttl_days() -> i64 {
    DEFAULT_SUPPRESSION_RULE_TTL_DAYS
}

pub fn skill_candidates_root(db: &DatabaseService) -> PathBuf {
    skills_root(db).join(SKILL_CANDIDATES_DIR)
}

fn skill_candidate_feedback_root(db: &DatabaseService) -> PathBuf {
    skills_root(db).join(SKILL_CANDIDATE_FEEDBACK_DIR)
}

pub fn list_skill_candidates(db: &DatabaseService) -> Result<Vec<SkillCandidate>> {
    let root = skill_candidates_root(db);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(&root)
        .with_context(|| format!("Failed to read skill candidates: {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        items.push(read_skill_candidate_file(entry.path().as_path())?);
    }

    items.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| a.title.cmp(&b.title))
    });
    Ok(items)
}

pub fn list_skill_candidate_suppression_rules(
    db: &DatabaseService,
) -> Result<Vec<SkillCandidateSuppressionRule>> {
    prune_inactive_suppression_rules(db)?;
    let root = skill_candidate_feedback_root(db);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(&root)
        .with_context(|| format!("Failed to read suppression rules: {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        items.push(read_suppression_rule_file(entry.path().as_path())?);
    }

    sort_suppression_rules(&mut items);
    Ok(items)
}

pub fn delete_skill_candidate_suppression_rule(
    db: &DatabaseService,
    rule_id: &str,
) -> Result<bool> {
    let path = suppression_rule_path(db, rule_id);
    delete_suppression_rule_file(&path)
}

pub fn extend_skill_candidate_suppression_rule(
    db: &DatabaseService,
    rule_id: &str,
    days: i64,
) -> Result<SkillCandidateSuppressionRule> {
    let path = suppression_rule_path(db, rule_id);
    extend_suppression_rule_file(path.as_path(), days)
}

pub fn expire_skill_candidate_suppression_rule(
    db: &DatabaseService,
    rule_id: &str,
) -> Result<SkillCandidateSuppressionRule> {
    let path = suppression_rule_path(db, rule_id);
    expire_suppression_rule_file(path.as_path())
}

pub fn get_skill_candidate(
    db: &DatabaseService,
    candidate_id: &str,
) -> Result<Option<SkillCandidate>> {
    let path = skill_candidate_path(db, candidate_id);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(read_skill_candidate_file(&path)?))
}

pub fn mark_skill_candidate_promoted(
    db: &DatabaseService,
    candidate_id: &str,
    skill_id: &str,
) -> Result<SkillCandidate> {
    update_skill_candidate_status(
        db,
        candidate_id,
        "promoted",
        Some(format!("Promoted to skill {}", skill_id)),
        Some(skill_id.to_string()),
    )
}

pub fn update_skill_candidate_status(
    db: &DatabaseService,
    candidate_id: &str,
    status: &str,
    review_note: Option<String>,
    promoted_skill_id: Option<String>,
) -> Result<SkillCandidate> {
    let path = skill_candidate_path(db, candidate_id);
    let mut candidate = read_skill_candidate_file(&path)?;
    let now_ms = Utc::now().timestamp_millis();
    candidate.status = status.trim().to_string();
    candidate.review_note = review_note.filter(|note| !note.trim().is_empty());
    candidate.reviewed_at_ms = Some(now_ms);
    if let Some(skill_id) = promoted_skill_id {
        candidate.promoted_skill_id = Some(skill_id);
    }
    candidate.updated_at_ms = now_ms;
    write_skill_candidate_file(&path, &candidate)?;
    Ok(candidate)
}

pub async fn backfill_skill_candidates_from_memory(
    db: &DatabaseService,
    collection_name: &str,
) -> Result<usize> {
    let Some(collection) = db.get_rag_collection_by_name(collection_name).await? else {
        return Ok(0);
    };

    let documents = db.get_rag_documents(&collection.id).await?;
    let mut captured = 0usize;

    for document in documents {
        let chunks = db.get_rag_chunks(&document.id).await?;
        let body = chunks
            .iter()
            .map(|chunk| chunk.content.trim())
            .filter(|content| !content.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if body.trim().is_empty() {
            continue;
        }

        let tags = document
            .metadata
            .get("tags")
            .cloned()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let kind = sentinel_rag::infer_memory_kind(
            document.metadata.get("kind").map(String::as_str),
            Some(document.file_name.as_str()),
            &tags,
            body.as_str(),
        );
        let durable_metadata = sentinel_rag::build_memory_durable_metadata(
            document.metadata.get("scope").map(String::as_str),
            document.metadata.get("stability").map(String::as_str),
            document.metadata.get("source").map(String::as_str),
            document.metadata.get("confidence").map(String::as_str),
            kind.as_str(),
            &tags,
        );

        if upsert_skill_candidate_from_memory(
            db,
            &SkillCandidateMemoryInput {
                memory_id: Some(document.id.clone()),
                text: body,
                kind,
                scope: durable_metadata.scope,
                source: durable_metadata.source,
                stability: durable_metadata.stability,
                confidence: durable_metadata.confidence,
            },
        )?
        .is_some()
        {
            captured += 1;
        }
    }

    Ok(captured)
}

pub fn upsert_skill_candidate_from_memory(
    db: &DatabaseService,
    input: &SkillCandidateMemoryInput,
) -> Result<Option<SkillCandidate>> {
    if !should_capture_skill_candidate(input) {
        return Ok(None);
    }
    if find_matching_suppression_rule(db, input)?.is_some() {
        return Ok(None);
    }

    let candidate_id = build_skill_candidate_id(input.kind.as_str(), input.text.as_str());
    let path = skill_candidate_path(db, candidate_id.as_str());
    let now_ms = Utc::now().timestamp_millis();

    let mut candidate = if path.exists() {
        read_skill_candidate_file(&path)?
    } else {
        build_skill_candidate(input, candidate_id.as_str(), now_ms)
    };

    candidate.source_memory_id = candidate
        .source_memory_id
        .clone()
        .or_else(|| input.memory_id.clone());
    candidate.scope = input.scope.trim().to_string();
    candidate.source = input.source.trim().to_string();
    candidate.stability = input.stability.trim().to_string();
    candidate.confidence = candidate.confidence.max(input.confidence.clamp(0.0, 1.0));
    candidate.memory_excerpt = build_memory_excerpt(input.text.as_str());
    candidate.capture_reasons = build_capture_reasons(input);
    candidate.updated_at_ms = now_ms;

    write_skill_candidate_file(&path, &candidate)?;
    Ok(Some(candidate))
}

pub fn create_suppression_rule_from_candidate(
    db: &DatabaseService,
    candidate: &SkillCandidate,
    category: Option<&str>,
    note: Option<String>,
) -> Result<SkillCandidateSuppressionRule> {
    let now_ms = Utc::now().timestamp_millis();
    let path = suppression_rule_path(db, candidate.id.as_str());
    let excerpt = if candidate.memory_excerpt.trim().is_empty() {
        build_memory_excerpt(candidate.description.as_str())
    } else {
        candidate.memory_excerpt.clone()
    };
    let mut rule = if path.exists() {
        read_suppression_rule_file(&path)?
    } else {
        SkillCandidateSuppressionRule {
            id: candidate.id.clone(),
            memory_kind: candidate.memory_kind.clone(),
            source: candidate.source.clone(),
            category: String::new(),
            note: None,
            excerpt: excerpt.clone(),
            match_terms: build_suppression_match_terms(
                candidate.memory_kind.as_str(),
                excerpt.as_str(),
            ),
            hit_count: 0,
            last_matched_at_ms: None,
            last_matched_excerpt: None,
            ttl_days: default_suppression_rule_ttl_days(),
            expires_at_ms: Some(compute_suppression_rule_expiry_ms(
                now_ms,
                default_suppression_rule_ttl_days(),
            )),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        }
    };
    rule.category = normalize_suppression_rule_category(
        category.unwrap_or(rule.category.as_str()),
        candidate.memory_kind.as_str(),
        note.as_deref(),
        excerpt.as_str(),
    );
    rule.note = normalize_suppression_rule_note(note, rule.category.as_str());
    rule.excerpt = excerpt.clone();
    rule.memory_kind = candidate.memory_kind.clone();
    rule.source = candidate.source.clone();
    rule.match_terms =
        build_suppression_match_terms(candidate.memory_kind.as_str(), excerpt.as_str());
    rule.ttl_days = normalize_suppression_rule_ttl_days(rule.ttl_days);
    rule.expires_at_ms = Some(compute_suppression_rule_expiry_ms(now_ms, rule.ttl_days));
    rule.updated_at_ms = now_ms;
    write_suppression_rule_file(&path, &rule)?;
    Ok(rule)
}

fn should_capture_skill_candidate(input: &SkillCandidateMemoryInput) -> bool {
    if input.text.trim().is_empty() {
        return false;
    }
    if input.stability.trim().eq_ignore_ascii_case("tentative") {
        return false;
    }

    matches!(
        input.kind.trim(),
        "sop" | "anti_pattern" | "decision" | "preference"
    ) && input.confidence >= 0.82
}

fn build_skill_candidate(
    input: &SkillCandidateMemoryInput,
    candidate_id: &str,
    now_ms: i64,
) -> SkillCandidate {
    let title = build_candidate_title(input.kind.as_str(), input.text.as_str());
    let suggested_skill_name = suggest_skill_name(input.kind.as_str(), input.text.as_str());
    SkillCandidate {
        id: candidate_id.to_string(),
        title,
        suggested_skill_name,
        description: build_candidate_description(input.kind.as_str(), input.text.as_str()),
        content: build_candidate_content(input.kind.as_str(), input.text.as_str()),
        memory_kind: input.kind.trim().to_string(),
        source_memory_id: input.memory_id.clone(),
        scope: input.scope.trim().to_string(),
        source: input.source.trim().to_string(),
        stability: input.stability.trim().to_string(),
        confidence: input.confidence.clamp(0.0, 1.0),
        memory_excerpt: build_memory_excerpt(input.text.as_str()),
        capture_reasons: build_capture_reasons(input),
        status: "draft".to_string(),
        promoted_skill_id: None,
        review_note: None,
        reviewed_at_ms: None,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    }
}

fn build_candidate_title(kind: &str, text: &str) -> String {
    let prefix = match kind.trim() {
        "sop" => "SOP",
        "anti_pattern" => "Anti-pattern",
        "decision" => "Decision",
        "preference" => "Preference",
        _ => "Memory",
    };
    format!("{}: {}", prefix, truncate_for_title(text, 72))
}

fn build_candidate_description(kind: &str, text: &str) -> String {
    match kind.trim() {
        "sop" => format!(
            "Reusable procedure candidate derived from durable memory: {}",
            text
        ),
        "anti_pattern" => format!(
            "Reusable caution candidate derived from durable memory: {}",
            text
        ),
        "decision" => format!(
            "Reusable decision record candidate derived from durable memory: {}",
            text
        ),
        "preference" => format!(
            "Reusable preference candidate derived from durable memory: {}",
            text
        ),
        _ => format!(
            "Reusable skill candidate derived from durable memory: {}",
            text
        ),
    }
}

fn build_candidate_content(kind: &str, text: &str) -> String {
    match kind.trim() {
        "sop" => format!(
            "When to use\n- The current task matches this operational pattern.\n\nProcedure\n- {}\n\nVerification\n- Confirm prerequisites and evidence before applying this SOP.\n- Check whether the environment or toolchain has changed since this memory was recorded.\n",
            text.trim()
        ),
        "anti_pattern" => format!(
            "When to use\n- The current task risks repeating this known mistake.\n\nAvoid\n- {}\n\nVerification\n- Confirm the risky shortcut is actually present before blocking it.\n- Document the safer alternative in the task log.\n",
            text.trim()
        ),
        "decision" => format!(
            "When to use\n- A similar task needs the same decision boundary.\n\nDecision\n- {}\n\nVerification\n- Re-check the assumptions that made this decision valid.\n- Escalate if scope, risk, or constraints have changed.\n",
            text.trim()
        ),
        "preference" => format!(
            "When to use\n- Working in the same project or team context.\n\nPreference\n- {}\n\nVerification\n- Apply only if it still matches the user's current requirements.\n- Fall back to explicit user instructions when they conflict.\n",
            text.trim()
        ),
        _ => format!(
            "When to use\n- A similar task appears again.\n\nGuidance\n- {}\n\nVerification\n- Re-validate the conditions before reuse.\n",
            text.trim()
        ),
    }
}

fn suggest_skill_name(kind: &str, text: &str) -> String {
    let mut slug = text
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        slug = "memory".to_string();
    }

    let slug = slug.trim_matches('-').to_string();
    let prefix = match kind.trim() {
        "sop" => "draft-sop",
        "anti_pattern" => "draft-anti-pattern",
        "decision" => "draft-decision",
        "preference" => "draft-preference",
        _ => "draft-memory",
    };

    let joined = format!("{}-{}", prefix, slug);
    truncate_slug(joined.as_str(), 64)
}

fn build_memory_excerpt(text: &str) -> String {
    truncate_for_title(text, 180)
}

fn build_capture_reasons(input: &SkillCandidateMemoryInput) -> Vec<String> {
    let mut reasons = Vec::new();
    reasons.push(format!(
        "Reusable memory kind detected: {}",
        input.kind.trim()
    ));
    reasons.push(format!("Scope resolved to {}", input.scope.trim()));
    reasons.push(format!("Memory source recorded as {}", input.source.trim()));
    reasons.push(format!(
        "Stability is {}, so the memory is eligible for promotion",
        input.stability.trim()
    ));
    reasons.push(format!(
        "Confidence {:.2} meets the candidate threshold of 0.82",
        input.confidence.clamp(0.0, 1.0)
    ));
    reasons
}

fn build_suppression_match_terms(kind: &str, excerpt: &str) -> Vec<String> {
    let mut terms = tokenize_for_suppression(excerpt);
    if !kind.trim().is_empty() {
        terms.push(kind.trim().to_lowercase());
    }
    terms.sort();
    terms.dedup();
    terms.truncate(8);
    terms
}

fn truncate_for_title(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let prefix = trimmed
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{}…", prefix)
}

fn truncate_slug(input: &str, max_chars: usize) -> String {
    let slug = input.trim_matches('-');
    if slug.chars().count() <= max_chars {
        return slug.to_string();
    }
    slug.chars()
        .take(max_chars)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn build_skill_candidate_id(kind: &str, text: &str) -> String {
    format!(
        "skc-{:x}",
        md5::compute(format!("{}\n{}", kind.trim(), text.trim()).as_bytes())
    )
}

fn default_skill_candidate_stability() -> String {
    "stable".to_string()
}

fn skill_candidate_path(db: &DatabaseService, candidate_id: &str) -> PathBuf {
    skill_candidates_root(db).join(format!("{}.json", candidate_id))
}

fn suppression_rule_path(db: &DatabaseService, rule_id: &str) -> PathBuf {
    skill_candidate_feedback_root(db).join(format!("{}.json", rule_id))
}

fn read_skill_candidate_file(path: &Path) -> Result<SkillCandidate> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read skill candidate: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse skill candidate: {}", path.display()))
}

fn write_skill_candidate_file(path: &Path, candidate: &SkillCandidate) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create candidate directory: {}", parent.display())
        })?;
    }
    let content = serde_json::to_string_pretty(candidate)?;
    fs::write(path, content)
        .with_context(|| format!("Failed to write skill candidate: {}", path.display()))?;
    Ok(())
}

fn read_suppression_rule_file(path: &Path) -> Result<SkillCandidateSuppressionRule> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read suppression rule: {}", path.display()))?;
    let mut rule: SkillCandidateSuppressionRule = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse suppression rule: {}", path.display()))?;
    rule.category = normalize_suppression_rule_category(
        rule.category.as_str(),
        rule.memory_kind.as_str(),
        rule.note.as_deref(),
        rule.excerpt.as_str(),
    );
    rule.note = normalize_suppression_rule_note(rule.note.take(), rule.category.as_str());
    rule.ttl_days = normalize_suppression_rule_ttl_days(rule.ttl_days);
    Ok(rule)
}

fn write_suppression_rule_file(path: &Path, rule: &SkillCandidateSuppressionRule) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create suppression directory: {}",
                parent.display()
            )
        })?;
    }
    let content = serde_json::to_string_pretty(rule)?;
    fs::write(path, content)
        .with_context(|| format!("Failed to write suppression rule: {}", path.display()))?;
    Ok(())
}

fn delete_suppression_rule_file(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path)
        .with_context(|| format!("Failed to delete suppression rule: {}", path.display()))?;
    Ok(true)
}

fn extend_suppression_rule_file(path: &Path, days: i64) -> Result<SkillCandidateSuppressionRule> {
    let mut rule = read_suppression_rule_file(path)?;
    let now_ms = Utc::now().timestamp_millis();
    let extend_days = normalize_suppression_rule_ttl_days(days);
    let base_ms = effective_suppression_rule_expiry_ms(&rule).max(now_ms);
    rule.ttl_days = normalize_suppression_rule_ttl_days(rule.ttl_days);
    rule.expires_at_ms = Some(compute_suppression_rule_expiry_ms(base_ms, extend_days));
    rule.updated_at_ms = now_ms;
    write_suppression_rule_file(path, &rule)?;
    Ok(rule)
}

fn expire_suppression_rule_file(path: &Path) -> Result<SkillCandidateSuppressionRule> {
    let mut rule = read_suppression_rule_file(path)?;
    let now_ms = Utc::now().timestamp_millis();
    rule.expires_at_ms = Some(now_ms);
    rule.updated_at_ms = now_ms;
    write_suppression_rule_file(path, &rule)?;
    Ok(rule)
}

fn prune_inactive_suppression_rules(db: &DatabaseService) -> Result<usize> {
    prune_inactive_suppression_rules_with_roots(
        skill_candidates_root(db).as_path(),
        skill_candidate_feedback_root(db).as_path(),
    )
}

fn prune_inactive_suppression_rules_with_roots(
    candidate_root: &Path,
    feedback_root: &Path,
) -> Result<usize> {
    if !feedback_root.exists() {
        return Ok(0);
    }

    let now_ms = Utc::now().timestamp_millis();
    let mut deleted = 0usize;
    for entry in fs::read_dir(feedback_root).with_context(|| {
        format!(
            "Failed to read suppression rules: {}",
            feedback_root.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let rule = read_suppression_rule_file(path.as_path())?;
        if is_suppression_rule_expired(&rule, now_ms) {
            deleted += usize::from(delete_suppression_rule_file(path.as_path())?);
            continue;
        }
        let candidate_path = candidate_root.join(format!("{}.json", rule.id));
        if !candidate_path.exists() {
            continue;
        }

        let candidate = read_skill_candidate_file(candidate_path.as_path())?;
        if candidate.status.eq_ignore_ascii_case("promoted") {
            deleted += usize::from(delete_suppression_rule_file(path.as_path())?);
        }
    }

    Ok(deleted)
}

fn normalize_suppression_rule_ttl_days(ttl_days: i64) -> i64 {
    ttl_days.max(1)
}

fn normalize_suppression_rule_category(
    raw_category: &str,
    memory_kind: &str,
    note: Option<&str>,
    excerpt: &str,
) -> String {
    let normalized = raw_category.trim().to_lowercase().replace('-', "_");
    if matches!(
        normalized.as_str(),
        "duplicate_noise" | "stale_preference" | "overfit_procedure" | "incorrect_pattern"
    ) {
        return normalized;
    }
    infer_suppression_rule_category(memory_kind, note, excerpt)
}

fn infer_suppression_rule_category(
    memory_kind: &str,
    note: Option<&str>,
    _excerpt: &str,
) -> String {
    let note = note.unwrap_or_default().trim().to_lowercase();
    if contains_any(
        note.as_str(),
        &[
            "duplicate",
            "repeat",
            "redundant",
            "noise",
            "noisy",
            "low-signal",
        ],
    ) {
        return "duplicate_noise".to_string();
    }
    if contains_any(
        note.as_str(),
        &[
            "wrong",
            "incorrect",
            "false",
            "invalid",
            "halluc",
            "mistake",
        ],
    ) {
        return "incorrect_pattern".to_string();
    }

    match memory_kind.trim() {
        "preference" => "stale_preference".to_string(),
        "sop" | "decision" => "overfit_procedure".to_string(),
        "anti_pattern" | "evidence_pattern" | "fact" => "incorrect_pattern".to_string(),
        _ => "duplicate_noise".to_string(),
    }
}

fn normalize_suppression_rule_note(note: Option<String>, category: &str) -> Option<String> {
    let normalized = note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
            if compact.len() <= SUPPRESSION_RULE_NOTE_MAX_LEN {
                compact
            } else {
                compact
                    .chars()
                    .take(SUPPRESSION_RULE_NOTE_MAX_LEN)
                    .collect()
            }
        });

    if normalized.is_some() {
        return normalized;
    }

    let default_note = match category {
        "duplicate_noise" => "Suppressed as repeated low-signal draft noise",
        "stale_preference" => "Suppressed as a stale or non-reusable preference draft",
        "overfit_procedure" => "Suppressed as an overfit procedure that should stay contextual",
        "incorrect_pattern" => "Suppressed as an incorrect or misleading reusable pattern",
        _ => "Suppressed by review gate",
    };
    Some(default_note.to_string())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn suppression_rule_ttl_ms(ttl_days: i64) -> i64 {
    normalize_suppression_rule_ttl_days(ttl_days) * 24 * 60 * 60 * 1000
}

fn compute_suppression_rule_expiry_ms(base_ms: i64, ttl_days: i64) -> i64 {
    base_ms.saturating_add(suppression_rule_ttl_ms(ttl_days))
}

fn effective_suppression_rule_expiry_ms(rule: &SkillCandidateSuppressionRule) -> i64 {
    rule.expires_at_ms.unwrap_or_else(|| {
        compute_suppression_rule_expiry_ms(
            rule.updated_at_ms.max(rule.created_at_ms),
            rule.ttl_days,
        )
    })
}

fn is_suppression_rule_expired(rule: &SkillCandidateSuppressionRule, now_ms: i64) -> bool {
    effective_suppression_rule_expiry_ms(rule) <= now_ms
}

fn sort_suppression_rules(items: &mut [SkillCandidateSuppressionRule]) {
    items.sort_by(|a, b| {
        b.hit_count
            .cmp(&a.hit_count)
            .then_with(|| {
                b.last_matched_at_ms
                    .unwrap_or_default()
                    .cmp(&a.last_matched_at_ms.unwrap_or_default())
            })
            .then_with(|| b.updated_at_ms.cmp(&a.updated_at_ms))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn find_matching_suppression_rule(
    db: &DatabaseService,
    input: &SkillCandidateMemoryInput,
) -> Result<Option<SkillCandidateSuppressionRule>> {
    prune_inactive_suppression_rules(db)?;
    let root = skill_candidate_feedback_root(db);
    if !root.exists() {
        return Ok(None);
    }

    let input_terms = tokenize_for_suppression(input.text.as_str());
    for entry in fs::read_dir(&root)
        .with_context(|| format!("Failed to read suppression rules: {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let mut rule = read_suppression_rule_file(entry.path().as_path())?;
        if !rule.memory_kind.eq_ignore_ascii_case(input.kind.as_str()) {
            continue;
        }
        if !rule.source.eq_ignore_ascii_case(input.source.as_str()) {
            continue;
        }
        if suppression_match_score(rule.match_terms.as_slice(), input_terms.as_slice()) >= 0.6 {
            record_suppression_rule_hit(entry.path().as_path(), &mut rule, input)?;
            return Ok(Some(rule));
        }
    }

    Ok(None)
}

fn record_suppression_rule_hit(
    path: &Path,
    rule: &mut SkillCandidateSuppressionRule,
    input: &SkillCandidateMemoryInput,
) -> Result<()> {
    let now_ms = Utc::now().timestamp_millis();
    rule.hit_count = rule.hit_count.saturating_add(1);
    rule.last_matched_at_ms = Some(now_ms);
    rule.last_matched_excerpt = Some(build_memory_excerpt(input.text.as_str()));
    rule.ttl_days = normalize_suppression_rule_ttl_days(rule.ttl_days);
    rule.expires_at_ms = Some(compute_suppression_rule_expiry_ms(now_ms, rule.ttl_days));
    rule.updated_at_ms = now_ms;
    write_suppression_rule_file(path, rule)
}

fn tokenize_for_suppression(input: &str) -> Vec<String> {
    input
        .to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 4)
        .map(str::to_string)
        .collect()
}

fn suppression_match_score(rule_terms: &[String], input_terms: &[String]) -> f64 {
    if rule_terms.is_empty() || input_terms.is_empty() {
        return 0.0;
    }
    let input_set = input_terms.iter().collect::<std::collections::HashSet<_>>();
    let matched = rule_terms
        .iter()
        .filter(|term| input_set.contains(term))
        .count();
    matched as f64 / rule_terms.len() as f64
}

#[cfg(test)]
mod tests {
    use super::{
        build_skill_candidate_id, build_suppression_match_terms,
        compute_suppression_rule_expiry_ms, default_suppression_rule_ttl_days,
        effective_suppression_rule_expiry_ms, expire_suppression_rule_file,
        extend_suppression_rule_file, infer_suppression_rule_category,
        normalize_suppression_rule_note, prune_inactive_suppression_rules_with_roots,
        read_suppression_rule_file, record_suppression_rule_hit, should_capture_skill_candidate,
        sort_suppression_rules, suggest_skill_name, suppression_match_score,
        write_skill_candidate_file, write_suppression_rule_file, SkillCandidate,
        SkillCandidateMemoryInput, SkillCandidateSuppressionRule,
    };
    use tempfile::TempDir;

    fn candidate_fixture(id: &str, status: &str) -> SkillCandidate {
        SkillCandidate {
            id: id.to_string(),
            title: "Test Candidate".to_string(),
            suggested_skill_name: "draft-test-candidate".to_string(),
            description: "Reusable test candidate".to_string(),
            content: "## Test Candidate".to_string(),
            memory_kind: "anti_pattern".to_string(),
            source_memory_id: Some("memory-1".to_string()),
            scope: "project".to_string(),
            source: "context_engineering".to_string(),
            stability: "stable".to_string(),
            confidence: 0.91,
            memory_excerpt: "Avoid broad wildcard scans on production assets".to_string(),
            capture_reasons: vec!["Test fixture".to_string()],
            status: status.to_string(),
            promoted_skill_id: (status == "promoted").then_some("skill-1".to_string()),
            review_note: None,
            reviewed_at_ms: None,
            created_at_ms: 1_700_000_000_000,
            updated_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn skill_candidate_id_is_stable() {
        let a = build_skill_candidate_id("sop", "Verify host before replay");
        let b = build_skill_candidate_id("sop", "Verify host before replay");
        assert_eq!(a, b);
    }

    #[test]
    fn suggested_skill_name_is_slugged() {
        let name = suggest_skill_name("anti_pattern", "Avoid broad wildcard scans on production");
        assert!(name.starts_with("draft-anti-pattern-"));
        assert!(!name.contains(' '));
    }

    #[test]
    fn capture_filter_requires_stable_high_value_memory() {
        let input = SkillCandidateMemoryInput {
            memory_id: None,
            text: "Prefer ripgrep for large repos".to_string(),
            kind: "preference".to_string(),
            scope: "project".to_string(),
            source: "context_engineering".to_string(),
            stability: "stable".to_string(),
            confidence: 0.86,
        };
        assert!(should_capture_skill_candidate(&input));
    }

    #[test]
    fn capture_reasons_explain_why_candidate_was_created() {
        let input = SkillCandidateMemoryInput {
            memory_id: None,
            text: "Verify host, path, and auth state before replay".to_string(),
            kind: "sop".to_string(),
            scope: "project".to_string(),
            source: "context_engineering".to_string(),
            stability: "stable".to_string(),
            confidence: 0.86,
        };

        let reasons = super::build_capture_reasons(&input);
        assert!(reasons
            .iter()
            .any(|item| item.contains("Reusable memory kind")));
        assert!(reasons.iter().any(|item| item.contains("0.82")));
    }

    #[test]
    fn suppression_terms_match_similar_memory_variants() {
        let rule_terms = build_suppression_match_terms(
            "anti_pattern",
            "Avoid broad wildcard scans on production assets",
        );
        let input_terms = super::tokenize_for_suppression(
            "Avoid broad wildcard scans on production services before approval",
        );
        assert!(suppression_match_score(rule_terms.as_slice(), input_terms.as_slice()) >= 0.6);
    }

    #[test]
    fn suppression_rule_hit_is_persisted_to_disk() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("rule.json");
        let mut rule = SkillCandidateSuppressionRule {
            id: "candidate-123".to_string(),
            memory_kind: "anti_pattern".to_string(),
            source: "context_engineering".to_string(),
            category: "duplicate_noise".to_string(),
            note: Some("duplicate low-signal warning".to_string()),
            excerpt: "Avoid broad wildcard scans on production assets".to_string(),
            match_terms: build_suppression_match_terms(
                "anti_pattern",
                "Avoid broad wildcard scans on production assets",
            ),
            hit_count: 0,
            last_matched_at_ms: None,
            last_matched_excerpt: None,
            ttl_days: default_suppression_rule_ttl_days(),
            expires_at_ms: Some(compute_suppression_rule_expiry_ms(
                1_700_000_000_000,
                default_suppression_rule_ttl_days(),
            )),
            created_at_ms: 1_700_000_000_000,
            updated_at_ms: 1_700_000_000_000,
        };
        write_suppression_rule_file(&path, &rule).unwrap();

        let input = SkillCandidateMemoryInput {
            memory_id: Some("memory-1".to_string()),
            text: "Avoid broad wildcard scans on production services before approval".to_string(),
            kind: "anti_pattern".to_string(),
            scope: "project".to_string(),
            source: "context_engineering".to_string(),
            stability: "stable".to_string(),
            confidence: 0.92,
        };

        record_suppression_rule_hit(&path, &mut rule, &input).unwrap();

        let persisted = read_suppression_rule_file(&path).unwrap();
        assert_eq!(persisted.hit_count, 1);
        assert!(persisted.last_matched_at_ms.is_some());
        assert_eq!(
            persisted.last_matched_excerpt.as_deref(),
            Some("Avoid broad wildcard scans on production services before approval")
        );
        assert!(persisted.updated_at_ms >= persisted.created_at_ms);
    }

    #[test]
    fn legacy_suppression_rule_defaults_new_hit_fields() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("legacy-rule.json");
        std::fs::write(
            &path,
            r#"{
  "id": "candidate-legacy",
  "memory_kind": "preference",
  "source": "context_engineering",
  "note": null,
  "excerpt": "Prefer ripgrep for large repos",
  "match_terms": ["prefer", "ripgrep", "large", "repos"],
  "created_at_ms": 1700000000000,
  "updated_at_ms": 1700000000000
}"#,
        )
        .unwrap();

        let rule = read_suppression_rule_file(&path).unwrap();
        assert_eq!(rule.hit_count, 0);
        assert_eq!(rule.last_matched_at_ms, None);
        assert_eq!(rule.last_matched_excerpt, None);
        assert_eq!(rule.category, "stale_preference");
        assert_eq!(
            rule.note.as_deref(),
            Some("Suppressed as a stale or non-reusable preference draft")
        );
        assert_eq!(rule.ttl_days, default_suppression_rule_ttl_days());
        assert_eq!(
            effective_suppression_rule_expiry_ms(&rule),
            compute_suppression_rule_expiry_ms(
                1_700_000_000_000,
                default_suppression_rule_ttl_days(),
            )
        );
    }

    #[test]
    fn suppression_rules_sort_by_hits_then_recency() {
        let mut rules = vec![
            SkillCandidateSuppressionRule {
                id: "rule-c".to_string(),
                memory_kind: "anti_pattern".to_string(),
                source: "context_engineering".to_string(),
                category: "duplicate_noise".to_string(),
                note: None,
                excerpt: "third".to_string(),
                match_terms: vec!["third".to_string()],
                hit_count: 4,
                last_matched_at_ms: Some(30),
                last_matched_excerpt: None,
                ttl_days: default_suppression_rule_ttl_days(),
                expires_at_ms: Some(30 + 100),
                created_at_ms: 10,
                updated_at_ms: 30,
            },
            SkillCandidateSuppressionRule {
                id: "rule-a".to_string(),
                memory_kind: "anti_pattern".to_string(),
                source: "context_engineering".to_string(),
                category: "incorrect_pattern".to_string(),
                note: None,
                excerpt: "first".to_string(),
                match_terms: vec!["first".to_string()],
                hit_count: 7,
                last_matched_at_ms: Some(20),
                last_matched_excerpt: None,
                ttl_days: default_suppression_rule_ttl_days(),
                expires_at_ms: Some(20 + 100),
                created_at_ms: 10,
                updated_at_ms: 20,
            },
            SkillCandidateSuppressionRule {
                id: "rule-b".to_string(),
                memory_kind: "anti_pattern".to_string(),
                source: "context_engineering".to_string(),
                category: "incorrect_pattern".to_string(),
                note: None,
                excerpt: "second".to_string(),
                match_terms: vec!["second".to_string()],
                hit_count: 7,
                last_matched_at_ms: Some(40),
                last_matched_excerpt: None,
                ttl_days: default_suppression_rule_ttl_days(),
                expires_at_ms: Some(40 + 100),
                created_at_ms: 10,
                updated_at_ms: 40,
            },
        ];

        sort_suppression_rules(&mut rules);
        let ordered_ids = rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ordered_ids, vec!["rule-b", "rule-a", "rule-c"]);
    }

    #[test]
    fn prune_removes_rules_for_promoted_candidates() {
        let temp_dir = TempDir::new().unwrap();
        let candidate_root = temp_dir.path().join("_candidates");
        let feedback_root = temp_dir.path().join("_candidate_feedback");
        let candidate = candidate_fixture("candidate-promoted", "promoted");
        let candidate_path = candidate_root.join("candidate-promoted.json");
        let rule_path = feedback_root.join("candidate-promoted.json");

        write_skill_candidate_file(&candidate_path, &candidate).unwrap();
        write_suppression_rule_file(
            &rule_path,
            &SkillCandidateSuppressionRule {
                id: candidate.id.clone(),
                memory_kind: candidate.memory_kind.clone(),
                source: candidate.source.clone(),
                category: "incorrect_pattern".to_string(),
                note: Some("obsolete after promotion".to_string()),
                excerpt: candidate.memory_excerpt.clone(),
                match_terms: build_suppression_match_terms(
                    candidate.memory_kind.as_str(),
                    candidate.memory_excerpt.as_str(),
                ),
                hit_count: 2,
                last_matched_at_ms: Some(1_700_000_000_100),
                last_matched_excerpt: Some(candidate.memory_excerpt.clone()),
                ttl_days: default_suppression_rule_ttl_days(),
                expires_at_ms: Some(i64::MAX),
                created_at_ms: 1_700_000_000_000,
                updated_at_ms: 1_700_000_000_100,
            },
        )
        .unwrap();

        let deleted =
            prune_inactive_suppression_rules_with_roots(&candidate_root, &feedback_root).unwrap();
        assert_eq!(deleted, 1);
        assert!(!rule_path.exists());
    }

    #[test]
    fn prune_keeps_rules_for_non_promoted_candidates() {
        let temp_dir = TempDir::new().unwrap();
        let candidate_root = temp_dir.path().join("_candidates");
        let feedback_root = temp_dir.path().join("_candidate_feedback");
        let candidate = candidate_fixture("candidate-rejected", "rejected");
        let candidate_path = candidate_root.join("candidate-rejected.json");
        let rule_path = feedback_root.join("candidate-rejected.json");

        write_skill_candidate_file(&candidate_path, &candidate).unwrap();
        write_suppression_rule_file(
            &rule_path,
            &SkillCandidateSuppressionRule {
                id: candidate.id.clone(),
                memory_kind: candidate.memory_kind.clone(),
                source: candidate.source.clone(),
                category: "duplicate_noise".to_string(),
                note: Some("still valid".to_string()),
                excerpt: candidate.memory_excerpt.clone(),
                match_terms: build_suppression_match_terms(
                    candidate.memory_kind.as_str(),
                    candidate.memory_excerpt.as_str(),
                ),
                hit_count: 2,
                last_matched_at_ms: Some(1_700_000_000_100),
                last_matched_excerpt: Some(candidate.memory_excerpt.clone()),
                ttl_days: default_suppression_rule_ttl_days(),
                expires_at_ms: Some(i64::MAX),
                created_at_ms: 1_700_000_000_000,
                updated_at_ms: 1_700_000_000_100,
            },
        )
        .unwrap();

        let deleted =
            prune_inactive_suppression_rules_with_roots(&candidate_root, &feedback_root).unwrap();
        assert_eq!(deleted, 0);
        assert!(rule_path.exists());
    }

    #[test]
    fn prune_removes_expired_rules_even_without_candidate_file() {
        let temp_dir = TempDir::new().unwrap();
        let candidate_root = temp_dir.path().join("_candidates");
        let feedback_root = temp_dir.path().join("_candidate_feedback");
        let rule_path = feedback_root.join("candidate-expired.json");

        write_suppression_rule_file(
            &rule_path,
            &SkillCandidateSuppressionRule {
                id: "candidate-expired".to_string(),
                memory_kind: "anti_pattern".to_string(),
                source: "context_engineering".to_string(),
                category: "duplicate_noise".to_string(),
                note: Some("expired".to_string()),
                excerpt: "Avoid broad wildcard scans on production assets".to_string(),
                match_terms: build_suppression_match_terms(
                    "anti_pattern",
                    "Avoid broad wildcard scans on production assets",
                ),
                hit_count: 1,
                last_matched_at_ms: Some(100),
                last_matched_excerpt: Some(
                    "Avoid broad wildcard scans on production assets".to_string(),
                ),
                ttl_days: 1,
                expires_at_ms: Some(1),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
        )
        .unwrap();

        let deleted =
            prune_inactive_suppression_rules_with_roots(&candidate_root, &feedback_root).unwrap();
        assert_eq!(deleted, 1);
        assert!(!rule_path.exists());
    }

    #[test]
    fn suppression_rule_hit_renews_expiry() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("rule-renew.json");
        let mut rule = SkillCandidateSuppressionRule {
            id: "candidate-renew".to_string(),
            memory_kind: "anti_pattern".to_string(),
            source: "context_engineering".to_string(),
            category: "duplicate_noise".to_string(),
            note: Some("renew on match".to_string()),
            excerpt: "Avoid broad wildcard scans on production assets".to_string(),
            match_terms: build_suppression_match_terms(
                "anti_pattern",
                "Avoid broad wildcard scans on production assets",
            ),
            hit_count: 2,
            last_matched_at_ms: Some(10),
            last_matched_excerpt: Some("old excerpt".to_string()),
            ttl_days: 3,
            expires_at_ms: Some(11),
            created_at_ms: 10,
            updated_at_ms: 10,
        };
        write_suppression_rule_file(&path, &rule).unwrap();

        let input = SkillCandidateMemoryInput {
            memory_id: Some("memory-2".to_string()),
            text: "Avoid broad wildcard scans on production services before approval".to_string(),
            kind: "anti_pattern".to_string(),
            scope: "project".to_string(),
            source: "context_engineering".to_string(),
            stability: "stable".to_string(),
            confidence: 0.91,
        };

        record_suppression_rule_hit(&path, &mut rule, &input).unwrap();
        let persisted = read_suppression_rule_file(&path).unwrap();
        assert_eq!(persisted.hit_count, 3);
        assert!(persisted.last_matched_at_ms.is_some());
        assert_eq!(persisted.ttl_days, 3);
        assert!(persisted.expires_at_ms.unwrap_or_default() > persisted.updated_at_ms);
    }

    #[test]
    fn extend_suppression_rule_pushes_expiry_forward() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("rule-extend.json");
        let rule = SkillCandidateSuppressionRule {
            id: "candidate-extend".to_string(),
            memory_kind: "anti_pattern".to_string(),
            source: "context_engineering".to_string(),
            category: "duplicate_noise".to_string(),
            note: Some("extend ttl".to_string()),
            excerpt: "Avoid broad wildcard scans on production assets".to_string(),
            match_terms: build_suppression_match_terms(
                "anti_pattern",
                "Avoid broad wildcard scans on production assets",
            ),
            hit_count: 1,
            last_matched_at_ms: Some(10),
            last_matched_excerpt: Some("old".to_string()),
            ttl_days: 21,
            expires_at_ms: Some(i64::MAX - 10 * 24 * 60 * 60 * 1000),
            created_at_ms: 10,
            updated_at_ms: 10,
        };
        write_suppression_rule_file(&path, &rule).unwrap();

        let updated = extend_suppression_rule_file(&path, 7).unwrap();
        assert_eq!(updated.ttl_days, 21);
        assert!(updated.expires_at_ms.unwrap_or_default() > rule.expires_at_ms.unwrap_or_default());
    }

    #[test]
    fn expire_suppression_rule_marks_rule_as_expired() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("rule-expire-now.json");
        let rule = SkillCandidateSuppressionRule {
            id: "candidate-expire".to_string(),
            memory_kind: "anti_pattern".to_string(),
            source: "context_engineering".to_string(),
            category: "duplicate_noise".to_string(),
            note: Some("expire now".to_string()),
            excerpt: "Avoid broad wildcard scans on production assets".to_string(),
            match_terms: build_suppression_match_terms(
                "anti_pattern",
                "Avoid broad wildcard scans on production assets",
            ),
            hit_count: 1,
            last_matched_at_ms: Some(10),
            last_matched_excerpt: Some("old".to_string()),
            ttl_days: 21,
            expires_at_ms: Some(i64::MAX),
            created_at_ms: 10,
            updated_at_ms: 10,
        };
        write_suppression_rule_file(&path, &rule).unwrap();

        let updated = expire_suppression_rule_file(&path).unwrap();
        let persisted = read_suppression_rule_file(&path).unwrap();
        assert_eq!(persisted.id, updated.id);
        assert!(persisted.expires_at_ms.unwrap_or_default() <= persisted.updated_at_ms);
    }

    #[test]
    fn infer_suppression_rule_category_prefers_explicit_note_signal() {
        assert_eq!(
            infer_suppression_rule_category(
                "sop",
                Some("Duplicate low-signal reminder already covered"),
                "Verify host before replay"
            ),
            "duplicate_noise"
        );
        assert_eq!(
            infer_suppression_rule_category(
                "preference",
                Some("Incorrect recommendation for current repo"),
                "Prefer broad wildcard scans"
            ),
            "incorrect_pattern"
        );
    }

    #[test]
    fn normalize_suppression_rule_note_compacts_and_defaults() {
        assert_eq!(
            normalize_suppression_rule_note(None, "overfit_procedure").as_deref(),
            Some("Suppressed as an overfit procedure that should stay contextual")
        );
        assert_eq!(
            normalize_suppression_rule_note(
                Some("  Duplicate    low-signal   reminder   ".to_string()),
                "duplicate_noise"
            )
            .as_deref(),
            Some("Duplicate low-signal reminder")
        );
    }
}
