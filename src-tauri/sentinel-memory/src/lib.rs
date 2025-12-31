use anyhow::Result;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use uuid::Uuid;
use sentinel_core::models::database::MemoryExecution;
use sentinel_db::client::DatabaseClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub name: String,
    pub success: bool,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub task: String,
    pub environment: Option<String>,
    pub tool_calls: Vec<ToolCallSummary>,
    pub success: bool,
    pub error: Option<String>,
    pub response_excerpt: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContextRequest {
    pub task: String,
    pub environment: Option<String>,
    pub tool_names: Vec<String>,
    pub max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMatch {
    pub record: ExecutionRecord,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_records: usize,
    pub min_score: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_records: 2000,
            min_score: 0.35,
        }
    }
}

#[derive(Default)]
struct InMemoryStore {
    records: Vec<ExecutionRecord>,
    record_ids: HashSet<String>,
}

#[derive(Clone)]
pub struct MemoryManager {
    store: std::sync::Arc<RwLock<InMemoryStore>>,
    config: MemoryConfig,
    db: std::sync::Arc<RwLock<Option<DatabaseClient>>>,
    last_loaded_at: std::sync::Arc<RwLock<Option<i64>>>,
}

impl MemoryManager {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            store: std::sync::Arc::new(RwLock::new(InMemoryStore::default())),
            config,
            db: std::sync::Arc::new(RwLock::new(None)),
            last_loaded_at: std::sync::Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_database_client(&self, db: DatabaseClient) {
        let mut guard = self.db.write().await;
        *guard = Some(db);
    }

    pub async fn record_execution(&self, mut record: ExecutionRecord) -> Result<()> {
        if record.id.trim().is_empty() {
            record.id = Uuid::new_v4().to_string();
        }
        if record.created_at == 0 {
            record.created_at = Utc::now().timestamp();
        }

        let mut store = self.store.write().await;
        if store.record_ids.insert(record.id.clone()) {
            store.records.push(record.clone());
        }
        if store.records.len() > self.config.max_records {
            let excess = store.records.len() - self.config.max_records;
            let removed_ids: Vec<String> = store.records.drain(0..excess).map(|r| r.id).collect();
            for id in removed_ids {
                store.record_ids.remove(&id);
            }
        }

        drop(store);
        self.persist_to_db(&record).await?;
        Ok(())
    }

    pub async fn build_context(&self, request: MemoryContextRequest) -> Result<Option<String>> {
        self.sync_from_db(500).await?;
        let matches = self.query(request).await?;
        if matches.is_empty() {
            return Ok(None);
        }

        let mut lines = Vec::new();
        lines.push("[Memory Context: prior executions]".to_string());
        for (idx, entry) in matches.iter().enumerate() {
            let record = &entry.record;
            let mut line = format!(
                "{}. task='{}' success={} score={:.2}",
                idx + 1,
                truncate(&record.task, 160),
                record.success,
                entry.score
            );
            if !entry.reasons.is_empty() {
                line.push_str(&format!(" reasons={}", entry.reasons.join(",")));
            }
            lines.push(line);

            if !record.tool_calls.is_empty() {
                let tools = record
                    .tool_calls
                    .iter()
                    .map(|t| format!("{}:{}", t.name, if t.success { "ok" } else { "fail" }))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("   tools: {}", tools));
            }
            if let Some(ref excerpt) = record.response_excerpt {
                lines.push(format!("   response: {}", truncate(excerpt, 160)));
            }
        }

        Ok(Some(lines.join("\n")))
    }

    async fn query(&self, request: MemoryContextRequest) -> Result<Vec<MemoryMatch>> {
        let store = self.store.read().await;
        let mut results = Vec::new();
        let task_tokens = tokenize(&request.task);
        let env = request.environment.as_deref().unwrap_or("");

        for record in store.records.iter() {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            let record_tokens = tokenize(&record.task);
            let task_score = jaccard(&task_tokens, &record_tokens);
            if task_score > 0.0 {
                score += task_score * 0.6;
                if task_score >= 0.4 {
                    reasons.push("task".to_string());
                }
            }

            if let Some(ref record_env) = record.environment {
                if !env.is_empty() {
                    let env_score = jaccard(&tokenize(env), &tokenize(record_env));
                    score += env_score * 0.2;
                    if env_score >= 0.5 {
                        reasons.push("env".to_string());
                    }
                }
            }

            if !request.tool_names.is_empty() && !record.tool_calls.is_empty() {
                let record_tool_names: HashSet<String> = record
                    .tool_calls
                    .iter()
                    .map(|t| t.name.to_string())
                    .collect();
                let overlap = request
                    .tool_names
                    .iter()
                    .filter(|t| record_tool_names.contains(*t))
                    .count();
                if overlap > 0 {
                    score += (overlap as f64 / request.tool_names.len() as f64) * 0.2;
                    reasons.push("tools".to_string());
                }
            }

            if score >= self.config.min_score {
                results.push(MemoryMatch {
                    record: record.clone(),
                    score,
                    reasons,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(request.max_results.max(1));
        Ok(results)
    }

    async fn persist_to_db(&self, record: &ExecutionRecord) -> Result<()> {
        let db_opt = self.db.read().await.clone();
        let Some(db) = db_opt else {
            return Ok(());
        };

        let tool_calls_json = if record.tool_calls.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&record.tool_calls)?)
        };

        let db_record = MemoryExecution {
            id: record.id.clone(),
            task: record.task.clone(),
            environment: record.environment.clone(),
            tool_calls: tool_calls_json,
            success: record.success,
            error: record.error.clone(),
            response_excerpt: record.response_excerpt.clone(),
            created_at: Utc.timestamp_opt(record.created_at, 0).single().unwrap_or_else(Utc::now),
        };

        db.create_memory_execution(&db_record).await?;
        Ok(())
    }

    async fn sync_from_db(&self, limit: i64) -> Result<()> {
        let db_opt = self.db.read().await.clone();
        let Some(db) = db_opt else {
            return Ok(());
        };

        let since_ts = *self.last_loaded_at.read().await;
        let since = since_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single());
        let rows = db.get_memory_executions_since(since, limit).await?;
        if rows.is_empty() {
            return Ok(());
        }

        let mut store = self.store.write().await;
        let mut latest_ts = since_ts.unwrap_or(0);
        for row in rows.iter() {
            if store.record_ids.contains(&row.id) {
                continue;
            }
            let tool_calls = row
                .tool_calls
                .as_ref()
                .and_then(|value| serde_json::from_str::<Vec<ToolCallSummary>>(value).ok())
                .unwrap_or_default();

            let created_at = row.created_at.timestamp();
            store.records.push(ExecutionRecord {
                id: row.id.clone(),
                task: row.task.clone(),
                environment: row.environment.clone(),
                tool_calls,
                success: row.success,
                error: row.error.clone(),
                response_excerpt: row.response_excerpt.clone(),
                created_at,
            });
            store.record_ids.insert(row.id.clone());
            if created_at > latest_ts {
                latest_ts = created_at;
            }
        }

        if store.records.len() > self.config.max_records {
            let excess = store.records.len() - self.config.max_records;
            let removed_ids: Vec<String> = store.records.drain(0..excess).map(|r| r.id).collect();
            for id in removed_ids {
                store.record_ids.remove(&id);
            }
        }

        drop(store);
        if latest_ts > 0 {
            let mut last_loaded = self.last_loaded_at.write().await;
            *last_loaded = Some(latest_ts);
        }

        Ok(())
    }
}

static GLOBAL_MEMORY: OnceLock<MemoryManager> = OnceLock::new();

pub fn get_global_memory() -> MemoryManager {
    GLOBAL_MEMORY
        .get_or_init(|| MemoryManager::new(MemoryConfig::default()))
        .clone()
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split_whitespace()
        .map(|t| t.to_lowercase())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut out = text.chars().take(max_len).collect::<String>();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_record_and_query() {
        let memory = MemoryManager::new(MemoryConfig::default());
        memory
            .record_execution(ExecutionRecord {
                id: "1".to_string(),
                task: "scan login endpoint".to_string(),
                environment: Some("local".to_string()),
                tool_calls: vec![ToolCallSummary {
                    name: "http_fetch".to_string(),
                    success: true,
                    duration_ms: Some(120),
                }],
                success: true,
                error: None,
                response_excerpt: Some("ok".to_string()),
                created_at: Utc::now().timestamp(),
            })
            .await
            .unwrap();

        let context = memory
            .build_context(MemoryContextRequest {
                task: "scan login page".to_string(),
                environment: Some("local".to_string()),
                tool_names: vec!["http_fetch".to_string()],
                max_results: 3,
            })
            .await
            .unwrap();

        assert!(context.is_some());
    }
}
