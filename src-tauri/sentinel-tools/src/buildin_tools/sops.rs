use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use sentinel_db::{Database, DatabaseService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

pub async fn set_sops_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
}

async fn get_db_service() -> Option<Arc<DatabaseService>> {
    let handle = APP_HANDLE.read().await;
    if let Some(ref h) = *handle {
        h.try_state::<Arc<DatabaseService>>()
            .map(|s| s.inner().clone())
    } else {
        None
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SopsAction {
    List,
    Load,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SopsToolArgs {
    pub action: SopsAction,
    pub profile_id: String,
    pub sop_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SopEntry {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub procedure: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SopsToolOutput {
    pub action: String,
    pub profile_id: String,
    pub sops: Option<Vec<SopEntry>>,
    pub sop: Option<SopEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum SopsToolError {
    #[error("database service unavailable")]
    DatabaseUnavailable,
    #[error("profile not found: {0}")]
    ProfileNotFound(String),
    #[error("sops tool is only available for passive system-agent profiles")]
    UnsupportedProfileMode,
    #[error("sop_id is required for load action")]
    MissingSopId,
    #[error("sop not found: {0}")]
    SopNotFound(String),
    #[error("invalid sop_definitions_json: {0}")]
    InvalidSopsJson(String),
    #[error("database error: {0}")]
    Database(String),
}

#[derive(Debug, Clone)]
pub struct SopsTool;

impl SopsTool {
    pub const NAME: &'static str = "sops";
    pub const DESCRIPTION: &'static str =
        "Read SOP definitions registered on a passive system-agent profile. Use this in background agents when you need the current profile's SOP list or a specific SOP procedure, without relying on the skills system.";
}

impl Tool for SopsTool {
    const NAME: &'static str = Self::NAME;
    type Args = SopsToolArgs;
    type Output = SopsToolOutput;
    type Error = SopsToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SopsToolArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let db = get_db_service()
            .await
            .ok_or(SopsToolError::DatabaseUnavailable)?;
        let profile = db
            .get_system_agent_profile(&args.profile_id)
            .await
            .map_err(|e| SopsToolError::Database(e.to_string()))?
            .ok_or_else(|| SopsToolError::ProfileNotFound(args.profile_id.clone()))?;

        if profile.mode != "passive" {
            return Err(SopsToolError::UnsupportedProfileMode);
        }

        let definitions = parse_sop_entries(&profile.sop_definitions_json)?;

        match args.action {
            SopsAction::List => Ok(SopsToolOutput {
                action: "list".to_string(),
                profile_id: args.profile_id,
                sops: Some(definitions),
                sop: None,
            }),
            SopsAction::Load => {
                let sop_id = args.sop_id.ok_or(SopsToolError::MissingSopId)?;
                let sop = definitions
                    .into_iter()
                    .find(|item| item.id == sop_id)
                    .ok_or_else(|| SopsToolError::SopNotFound(sop_id.clone()))?;
                Ok(SopsToolOutput {
                    action: "load".to_string(),
                    profile_id: args.profile_id,
                    sops: None,
                    sop: Some(sop),
                })
            }
        }
    }
}

fn parse_sop_entries(raw: &str) -> Result<Vec<SopEntry>, SopsToolError> {
    let parsed = serde_json::from_str::<Vec<SopEntry>>(raw)
        .map_err(|e| SopsToolError::InvalidSopsJson(e.to_string()))?;
    Ok(parsed
        .into_iter()
        .filter_map(normalize_sop_entry)
        .collect::<Vec<_>>())
}

fn normalize_sop_entry(mut entry: SopEntry) -> Option<SopEntry> {
    entry.id = entry.id.trim().to_string();
    if entry.id.is_empty() {
        return None;
    }
    entry.name = entry.name.trim().to_string();
    entry.description = entry.description.trim().to_string();
    if entry.updated_at.trim().is_empty() {
        entry.updated_at = chrono::Utc::now().to_rfc3339();
    } else {
        entry.updated_at = entry.updated_at.trim().to_string();
    }
    Some(entry)
}
