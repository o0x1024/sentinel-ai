use super::types::*;
use sqlx::{Pool, Sqlite};

pub struct TrafficCodecStore {
    pool: Pool<Sqlite>,
}

impl TrafficCodecStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn initialize(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS codec_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                rule_order INTEGER NOT NULL DEFAULT 0,
                match_rule TEXT NOT NULL,
                scope TEXT NOT NULL,
                pipeline TEXT NOT NULL,
                reversible INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS codec_keys (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                key_name TEXT NOT NULL,
                key_value TEXT NOT NULL,
                format TEXT NOT NULL DEFAULT 'utf8',
                created_at TEXT NOT NULL,
                FOREIGN KEY (rule_id) REFERENCES codec_rules(id) ON DELETE CASCADE
            )"#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_rules(&self) -> Result<Vec<TrafficCodecRule>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CodecRuleRow>(
            "SELECT * FROM codec_rules ORDER BY rule_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.into_rule()).collect())
    }

    pub async fn save_rule(&self, rule: &TrafficCodecRule) -> Result<(), sqlx::Error> {
        let match_rule_json = serde_json::to_string(&rule.match_rule).unwrap_or_default();
        let scope_json = serde_json::to_string(&rule.scope).unwrap_or_default();
        let pipeline_json = serde_json::to_string(&rule.pipeline).unwrap_or_default();

        sqlx::query(
            r#"INSERT OR REPLACE INTO codec_rules
               (id, name, enabled, rule_order, match_rule, scope, pipeline, reversible, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(rule.enabled)
        .bind(rule.order)
        .bind(&match_rule_json)
        .bind(&scope_json)
        .bind(&pipeline_json)
        .bind(rule.reversible)
        .bind(&rule.created_at)
        .bind(&rule.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_rule(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM codec_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn reorder_rules(&self, ids: &[String]) -> Result<(), sqlx::Error> {
        for (index, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE codec_rules SET rule_order = ? WHERE id = ?")
                .bind(index as i32)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct CodecRuleRow {
    id: String,
    name: String,
    enabled: bool,
    rule_order: i32,
    match_rule: String,
    scope: String,
    pipeline: String,
    reversible: bool,
    created_at: String,
    updated_at: String,
}

impl CodecRuleRow {
    fn into_rule(self) -> Option<TrafficCodecRule> {
        Some(TrafficCodecRule {
            id: self.id,
            name: self.name,
            enabled: self.enabled,
            order: self.rule_order,
            match_rule: serde_json::from_str(&self.match_rule).ok()?,
            scope: serde_json::from_str(&self.scope).ok()?,
            pipeline: serde_json::from_str(&self.pipeline).ok()?,
            reversible: self.reversible,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
