//! CPG 安全规则管理 Tauri 命令
//!
//! 提供前端调用的自定义审计规则 CRUD 命令

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use sentinel_db::DatabaseService;

/// 命令响应
#[derive(Debug, Serialize, Deserialize)]
pub struct RuleCommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> RuleCommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSpecInput {
    pub name_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg_pattern: Option<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRuleInput {
    pub id: Option<String>,
    pub name: String,
    pub cwe: String,
    pub severity: String,
    pub description: String,
    pub sources: Vec<PatternSpecInput>,
    pub sinks: Vec<PatternSpecInput>,
    pub sanitizers: Vec<PatternSpecInput>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRuleView {
    pub id: String,
    pub name: String,
    pub cwe: String,
    pub severity: String,
    pub description: String,
    pub sources: Vec<PatternSpecInput>,
    pub sinks: Vec<PatternSpecInput>,
    pub sanitizers: Vec<PatternSpecInput>,
    pub is_builtin: bool,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn record_to_view(r: sentinel_db::CpgSecurityRuleRecord) -> SecurityRuleView {
    SecurityRuleView {
        id: r.id,
        name: r.name,
        cwe: r.cwe,
        severity: r.severity,
        description: r.description,
        sources: serde_json::from_str(&r.sources_json).unwrap_or_default(),
        sinks: serde_json::from_str(&r.sinks_json).unwrap_or_default(),
        sanitizers: serde_json::from_str(&r.sanitizers_json).unwrap_or_default(),
        is_builtin: r.is_builtin,
        enabled: r.enabled,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

// ── Tauri Commands ──────────────────────────────────────────────────────────

/// 列出安全规则
#[tauri::command]
pub async fn list_cpg_security_rules(
    db_service: State<'_, Arc<DatabaseService>>,
    severity: Option<String>,
    enabled: Option<bool>,
    is_builtin: Option<bool>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<RuleCommandResponse<Vec<SecurityRuleView>>, String> {
    let filters = sentinel_db::CpgSecurityRuleFilters {
        severity,
        enabled,
        is_builtin,
        search,
        limit,
        offset,
    };

    match db_service.list_cpg_security_rules(filters).await {
        Ok(records) => Ok(RuleCommandResponse::ok(
            records.into_iter().map(record_to_view).collect(),
        )),
        Err(e) => {
            tracing::error!("Failed to list CPG security rules: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 统计安全规则数量
#[tauri::command]
pub async fn count_cpg_security_rules(
    db_service: State<'_, Arc<DatabaseService>>,
    severity: Option<String>,
    enabled: Option<bool>,
    is_builtin: Option<bool>,
) -> Result<RuleCommandResponse<i64>, String> {
    let filters = sentinel_db::CpgSecurityRuleFilters {
        severity,
        enabled,
        is_builtin,
        ..Default::default()
    };

    match db_service.count_cpg_security_rules(filters).await {
        Ok(count) => Ok(RuleCommandResponse::ok(count)),
        Err(e) => {
            tracing::error!("Failed to count CPG security rules: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 获取单个安全规则详情
#[tauri::command]
pub async fn get_cpg_security_rule(
    db_service: State<'_, Arc<DatabaseService>>,
    rule_id: String,
) -> Result<RuleCommandResponse<Option<SecurityRuleView>>, String> {
    match db_service.get_cpg_security_rule(&rule_id).await {
        Ok(record) => Ok(RuleCommandResponse::ok(record.map(record_to_view))),
        Err(e) => {
            tracing::error!("Failed to get CPG security rule: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 创建或更新安全规则
#[tauri::command]
pub async fn save_cpg_security_rule(
    db_service: State<'_, Arc<DatabaseService>>,
    rule: SecurityRuleInput,
) -> Result<RuleCommandResponse<SecurityRuleView>, String> {
    let now = Utc::now();
    let id = rule
        .id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let sources_json =
        serde_json::to_string(&rule.sources).unwrap_or_else(|_| "[]".to_string());
    let sinks_json =
        serde_json::to_string(&rule.sinks).unwrap_or_else(|_| "[]".to_string());
    let sanitizers_json =
        serde_json::to_string(&rule.sanitizers).unwrap_or_else(|_| "[]".to_string());

    let record = sentinel_db::CpgSecurityRuleRecord {
        id: id.clone(),
        name: rule.name,
        cwe: rule.cwe,
        severity: rule.severity,
        description: rule.description,
        sources_json,
        sinks_json,
        sanitizers_json,
        is_builtin: false,
        enabled: rule.enabled,
        created_at: now,
        updated_at: now,
    };

    match db_service.insert_cpg_security_rule(&record).await {
        Ok(()) => Ok(RuleCommandResponse::ok(record_to_view(record))),
        Err(e) => {
            tracing::error!("Failed to save CPG security rule: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 切换规则启用状态
#[tauri::command]
pub async fn toggle_cpg_security_rule(
    db_service: State<'_, Arc<DatabaseService>>,
    rule_id: String,
    enabled: bool,
) -> Result<RuleCommandResponse<String>, String> {
    match db_service.toggle_cpg_security_rule(&rule_id, enabled).await {
        Ok(()) => Ok(RuleCommandResponse::ok(format!(
            "Rule {} {}",
            rule_id,
            if enabled { "enabled" } else { "disabled" }
        ))),
        Err(e) => {
            tracing::error!("Failed to toggle CPG security rule: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 删除安全规则
#[tauri::command]
pub async fn delete_cpg_security_rule(
    db_service: State<'_, Arc<DatabaseService>>,
    rule_id: String,
) -> Result<RuleCommandResponse<()>, String> {
    match db_service.delete_cpg_security_rule(&rule_id).await {
        Ok(()) => Ok(RuleCommandResponse::ok(())),
        Err(e) => {
            tracing::error!("Failed to delete CPG security rule: {}", e);
            Ok(RuleCommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 将内置规则种子到数据库
#[tauri::command]
pub async fn seed_builtin_cpg_rules(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<RuleCommandResponse<usize>, String> {
    match seed_builtin_rules_impl(&db_service).await {
        Ok(count) => Ok(RuleCommandResponse::ok(count)),
        Err(e) => {
            tracing::error!("Failed to seed built-in CPG rules: {}", e);
            Ok(RuleCommandResponse::err(format!("Seed error: {}", e)))
        }
    }
}

/// Internal impl: seed all built-in rules from the hardcoded definitions
pub async fn seed_builtin_rules_impl(
    db_service: &Arc<DatabaseService>,
) -> Result<usize, String> {
    use sentinel_tools::buildin_tools::cpg::security_rules::{all_rules, SecurityRule};

    let rules: Vec<SecurityRule> = all_rules();
    let now = Utc::now();
    let mut count = 0usize;

    for rule in &rules {
        let sources_json = serde_json::to_string(&rule.sources)
            .unwrap_or_else(|_| "[]".to_string());
        let sinks_json = serde_json::to_string(&rule.sinks)
            .unwrap_or_else(|_| "[]".to_string());
        let sanitizers_json = serde_json::to_string(&rule.sanitizers)
            .unwrap_or_else(|_| "[]".to_string());

        let record = sentinel_db::CpgSecurityRuleRecord {
            id: rule.id.clone(),
            name: rule.name.clone(),
            cwe: rule.cwe.clone(),
            severity: rule.severity.label().to_string(),
            description: rule.description.clone(),
            sources_json,
            sinks_json,
            sanitizers_json,
            is_builtin: true,
            enabled: true,
            created_at: now,
            updated_at: now,
        };

        db_service
            .insert_cpg_security_rule(&record)
            .await
            .map_err(|e| format!("Failed to insert built-in rule '{}': {}", rule.id, e))?;
        count += 1;
    }

    tracing::info!("Seeded {} built-in CPG security rules", count);
    Ok(count)
}
