use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantProfilePayload {
    pub id: String,
    pub label: String,
    pub description: String,
    pub default_model: Option<String>,
    pub default_rag_enabled: bool,
    pub default_web_search_enabled: bool,
    pub default_tools_enabled: bool,
    pub default_tenth_man_enabled: bool,
    pub default_tool_selection_strategy: String,
    pub default_max_tools: u32,
    pub default_fixed_tools: Vec<String>,
    pub default_disabled_tools: Vec<String>,
    pub default_manual_tools: Vec<String>,
    pub default_team_orchestration_preset_id: Option<String>,
    pub default_team_recovery_preset_id: Option<String>,
    pub context_mode: String,
    pub run_mode: String,
}

const ASSISTANT_PROFILE_CONFIG_NAMESPACE: &str = "assistant_profiles";
const ASSISTANT_PROFILE_CONFIG_KEY: &str = "registry";
const ASSISTANT_PROFILE_DEFAULT_KEY: &str = "default_profile_id";
const TEAM_ORCHESTRATION_PRESET_IDS: &[&str] =
    &["product_delivery_chain", "incident_response_flow"];
const TEAM_RECOVERY_PRESET_IDS: &[&str] = &["conservative", "balanced", "aggressive"];
const TOOL_SELECTION_STRATEGIES: &[&str] = &["Keyword", "LLM", "Hybrid", "Manual", "All"];

fn default_assistant_profiles() -> Vec<AssistantProfilePayload> {
    vec![
        AssistantProfilePayload {
            id: "assistant.default".to_string(),
            label: "Assistant".to_string(),
            description: "默认交互助手，优先使用 Claude-like 上下文和单助手执行。".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: false,
            default_tool_selection_strategy: "Keyword".to_string(),
            default_max_tools: 5,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            context_mode: "claude-like".to_string(),
            run_mode: "assistant".to_string(),
        },
        AssistantProfilePayload {
            id: "assistant.reviewer".to_string(),
            label: "Reviewer".to_string(),
            description: "偏审查与复盘的助手，默认使用 Codex-like 上下文。".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: true,
            default_tool_selection_strategy: "Hybrid".to_string(),
            default_max_tools: 5,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
                "tenth_man_review".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            context_mode: "codex-like".to_string(),
            run_mode: "assistant".to_string(),
        },
        AssistantProfilePayload {
            id: "team.lead".to_string(),
            label: "Team Lead".to_string(),
            description: "默认团队编排入口，使用 Claude-like 上下文和 Team 运行模式。".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: false,
            default_tool_selection_strategy: "Hybrid".to_string(),
            default_max_tools: 8,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
                "spawn_agent".to_string(),
                "wait_agents".to_string(),
                "list_agents".to_string(),
                "close_agent".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: Some("product_delivery_chain".to_string()),
            default_team_recovery_preset_id: Some("balanced".to_string()),
            context_mode: "claude-like".to_string(),
            run_mode: "team".to_string(),
        },
        AssistantProfilePayload {
            id: "team.reviewer".to_string(),
            label: "Team Reviewer".to_string(),
            description: "团队审查型入口，使用 Codex-like 上下文和 Team 运行模式。".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: true,
            default_tool_selection_strategy: "Hybrid".to_string(),
            default_max_tools: 8,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
                "spawn_agent".to_string(),
                "wait_agents".to_string(),
                "list_agents".to_string(),
                "close_agent".to_string(),
                "tenth_man_review".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: Some("incident_response_flow".to_string()),
            default_team_recovery_preset_id: Some("conservative".to_string()),
            context_mode: "codex-like".to_string(),
            run_mode: "team".to_string(),
        },
    ]
}

fn normalize_profile(mut profile: AssistantProfilePayload) -> AssistantProfilePayload {
    profile.id = profile.id.trim().to_string();
    profile.label = profile.label.trim().to_string();
    profile.description = profile.description.trim().to_string();
    profile.default_model = profile
        .default_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    profile.default_tool_selection_strategy =
        profile.default_tool_selection_strategy.trim().to_string();
    profile.default_max_tools = profile.default_max_tools.max(1);
    profile.default_fixed_tools = normalize_tool_ids(profile.default_fixed_tools);
    profile.default_disabled_tools = normalize_tool_ids(profile.default_disabled_tools);
    profile.default_manual_tools = normalize_tool_ids(profile.default_manual_tools);
    profile.default_team_orchestration_preset_id = profile
        .default_team_orchestration_preset_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    profile.default_team_recovery_preset_id = profile
        .default_team_recovery_preset_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    profile
}

fn normalize_profiles(profiles: Vec<AssistantProfilePayload>) -> Vec<AssistantProfilePayload> {
    profiles.into_iter().map(normalize_profile).collect()
}

fn normalize_tool_ids(tool_ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tool_id in tool_ids {
        let normalized = tool_id.trim().replace("::", "__");
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        out.push(normalized);
    }
    out
}

fn validate_profiles(profiles: &[AssistantProfilePayload]) -> Result<(), String> {
    if profiles.is_empty() {
        return Err("assistant profiles cannot be empty".to_string());
    }

    let mut ids = HashSet::new();
    for profile in profiles {
        if profile.id.trim().is_empty() {
            return Err("assistant profile id cannot be empty".to_string());
        }
        if profile.label.trim().is_empty() {
            return Err(format!(
                "assistant profile {} label cannot be empty",
                profile.id
            ));
        }
        if !ids.insert(profile.id.clone()) {
            return Err(format!("duplicate assistant profile id: {}", profile.id));
        }
        if profile.context_mode != "claude-like" && profile.context_mode != "codex-like" {
            return Err(format!(
                "assistant profile {} has unsupported context mode {}",
                profile.id, profile.context_mode
            ));
        }
        if profile.run_mode != "assistant" && profile.run_mode != "team" {
            return Err(format!(
                "assistant profile {} has unsupported run mode {}",
                profile.id, profile.run_mode
            ));
        }
        if !TOOL_SELECTION_STRATEGIES.contains(&profile.default_tool_selection_strategy.as_str()) {
            return Err(format!(
                "assistant profile {} has unsupported tool selection strategy {}",
                profile.id, profile.default_tool_selection_strategy
            ));
        }
        if profile.default_max_tools == 0 {
            return Err(format!(
                "assistant profile {} max tools must be greater than 0",
                profile.id
            ));
        }
        if let Some(preset_id) = &profile.default_team_orchestration_preset_id {
            if !TEAM_ORCHESTRATION_PRESET_IDS.contains(&preset_id.as_str()) {
                return Err(format!(
                    "assistant profile {} has unsupported team orchestration preset {}",
                    profile.id, preset_id
                ));
            }
        }
        if let Some(preset_id) = &profile.default_team_recovery_preset_id {
            if !TEAM_RECOVERY_PRESET_IDS.contains(&preset_id.as_str()) {
                return Err(format!(
                    "assistant profile {} has unsupported team recovery preset {}",
                    profile.id, preset_id
                ));
            }
        }
    }

    Ok(())
}

async fn load_profiles(
    db_service: &sentinel_db::DatabaseService,
) -> Result<Vec<AssistantProfilePayload>, String> {
    match db_service
        .get_config(
            ASSISTANT_PROFILE_CONFIG_NAMESPACE,
            ASSISTANT_PROFILE_CONFIG_KEY,
        )
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) => {
            let profiles: Vec<AssistantProfilePayload> =
                serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let profiles = normalize_profiles(profiles);
            validate_profiles(&profiles)?;
            Ok(profiles)
        }
        None => {
            let profiles = normalize_profiles(default_assistant_profiles());
            validate_profiles(&profiles)?;
            Ok(profiles)
        }
    }
}

async fn load_default_profile_id(
    db_service: &sentinel_db::DatabaseService,
) -> Result<String, String> {
    let profiles = load_profiles(db_service).await?;
    let fallback_id = profiles
        .first()
        .map(|profile| profile.id.clone())
        .ok_or_else(|| "assistant profiles cannot be empty".to_string())?;

    match db_service
        .get_config(
            ASSISTANT_PROFILE_CONFIG_NAMESPACE,
            ASSISTANT_PROFILE_DEFAULT_KEY,
        )
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) if profiles.iter().any(|profile| profile.id == raw) => Ok(raw),
        _ => Ok(fallback_id),
    }
}

#[tauri::command]
pub async fn list_assistant_profiles(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<AssistantProfilePayload>, String> {
    load_profiles(db_service.inner().as_ref()).await
}

#[tauri::command]
pub async fn get_assistant_profile(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<AssistantProfilePayload>, String> {
    Ok(load_profiles(db_service.inner().as_ref())
        .await?
        .into_iter()
        .find(|profile| profile.id == id))
}

#[tauri::command]
pub async fn save_assistant_profiles(
    profiles: Vec<AssistantProfilePayload>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    let profiles = normalize_profiles(profiles);
    validate_profiles(&profiles)?;
    let raw = serde_json::to_string(&profiles).map_err(|e| e.to_string())?;
    let db = db_service.inner().as_ref();
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        ASSISTANT_PROFILE_CONFIG_KEY,
        &raw,
        Some("Assistant profile registry"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let current_default = db
        .get_config(
            ASSISTANT_PROFILE_CONFIG_NAMESPACE,
            ASSISTANT_PROFILE_DEFAULT_KEY,
        )
        .await
        .map_err(|e| e.to_string())?;
    let next_default = match current_default {
        Some(default_id) if profiles.iter().any(|profile| profile.id == default_id) => default_id,
        _ => profiles
            .first()
            .map(|profile| profile.id.clone())
            .ok_or_else(|| "assistant profiles cannot be empty".to_string())?,
    };
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        ASSISTANT_PROFILE_DEFAULT_KEY,
        &next_default,
        Some("Default assistant profile id"),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_assistant_profile_id(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    load_default_profile_id(db_service.inner().as_ref()).await
}

#[tauri::command]
pub async fn save_default_assistant_profile_id(
    profile_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    let profile_id = profile_id.trim().to_string();
    let profiles = load_profiles(db_service.inner().as_ref()).await?;
    if !profiles.iter().any(|profile| profile.id == profile_id) {
        return Err(format!("unknown assistant profile id: {}", profile_id));
    }

    db_service
        .inner()
        .as_ref()
        .set_config(
            ASSISTANT_PROFILE_CONFIG_NAMESPACE,
            ASSISTANT_PROFILE_DEFAULT_KEY,
            &profile_id,
            Some("Default assistant profile id"),
        )
        .await
        .map_err(|e| e.to_string())
}
