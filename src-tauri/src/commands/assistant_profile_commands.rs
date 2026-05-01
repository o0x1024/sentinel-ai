use crate::services::ai::AiServiceManager;
use sentinel_db::Database;
use sentinel_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantProfilePayload {
    #[serde(default)]
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default = "default_profile_team_role")]
    pub team_role: String,
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
    #[serde(default)]
    pub default_team_profile_id: Option<String>,
    pub context_mode: String,
    pub run_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfilePayload {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub orchestrator_profile_id: String,
    pub specialist_profile_ids: Vec<String>,
    pub monitor_profile_id: String,
    pub default_model: Option<String>,
    pub context_mode: String,
    pub memory_policy: serde_json::Value,
    pub tool_policy_matrix: serde_json::Value,
    pub harness_policy: serde_json::Value,
    pub concurrency_policy: serde_json::Value,
    pub safety_policy: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCreateProfileRequest {
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCreatedAssistantProfileResponse {
    pub profile: AssistantProfilePayload,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiTeamProfileDraft {
    pub assistant_profiles: Vec<AssistantProfilePayload>,
    pub team_profile: TeamProfilePayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCreatedTeamProfileResponse {
    pub assistant_profiles: Vec<AssistantProfilePayload>,
    pub team_profile: TeamProfilePayload,
    pub provider: String,
    pub model: String,
}

const ASSISTANT_PROFILE_CONFIG_NAMESPACE: &str = "assistant_profiles";
const ASSISTANT_PROFILE_CONFIG_KEY: &str = "registry";
const ASSISTANT_PROFILE_DEFAULT_KEY: &str = "default_profile_id";
const TEAM_PROFILE_CONFIG_KEY: &str = "team_registry";
const TEAM_PROFILE_DEFAULT_KEY: &str = "default_team_profile_id";
const TEAM_ORCHESTRATION_PRESET_IDS: &[&str] =
    &["product_delivery_chain", "incident_response_flow"];
const TEAM_RECOVERY_PRESET_IDS: &[&str] = &["conservative", "balanced", "aggressive"];
const TOOL_SELECTION_STRATEGIES: &[&str] =
    &["Keyword", "LLM", "Hybrid", "Manual", "All", "Deferred"];
const PROFILE_TEAM_ROLES: &[&str] = &["assistant", "orchestrator", "specialist", "monitor"];
const AI_AGENT_PROFILE_SYSTEM_PROMPT: &str = r#"You create Sentinel AI interactive Agent profiles.
Return exactly one JSON object matching this camelCase schema:
{
  "label": "short display name",
  "description": "one concise Chinese description",
  "teamRole": "assistant|orchestrator|specialist|monitor",
  "defaultModel": null,
  "defaultRagEnabled": false,
  "defaultWebSearchEnabled": false,
  "defaultToolsEnabled": true,
  "defaultTenthManEnabled": false,
  "defaultToolSelectionStrategy": "Keyword|LLM|Hybrid|Manual|All",
  "defaultMaxTools": 1,
  "defaultFixedTools": ["interactive_shell", "ask_user_question"],
  "defaultDisabledTools": [],
  "defaultManualTools": [],
  "defaultTeamOrchestrationPresetId": null,
  "defaultTeamRecoveryPresetId": null,
  "defaultTeamProfileId": null,
  "contextMode": "claude-like|codex-like|sentinel-like",
  "runMode": "assistant|team"
}
Rules:
- Do not include markdown, comments, prose, or extra keys.
- Do not include an id field; the application assigns ids.
- Prefer runMode "assistant" unless the user explicitly asks for a Team entry Agent.
- Use only these tool ids when needed: interactive_shell, ask_user_question, spawn_agent, wait_agents, list_agents, close_agent, tenth_man_review.
- Use null for optional preset/model fields unless the user explicitly requires them.
"#;
const AI_TEAM_PROFILE_SYSTEM_PROMPT: &str = r#"You create Sentinel AI Team profiles and their required member Agent profiles.
Return exactly one JSON object matching this camelCase schema:
{
  "assistantProfiles": [
    {
      "label": "short display name",
      "description": "one concise Chinese description",
      "teamRole": "orchestrator|specialist|monitor",
      "defaultModel": null,
      "defaultRagEnabled": false,
      "defaultWebSearchEnabled": false,
      "defaultToolsEnabled": true,
      "defaultTenthManEnabled": false,
      "defaultToolSelectionStrategy": "Keyword|LLM|Hybrid|Manual|All",
      "defaultMaxTools": 1,
      "defaultFixedTools": ["interactive_shell", "ask_user_question"],
      "defaultDisabledTools": [],
      "defaultManualTools": [],
      "defaultTeamOrchestrationPresetId": null,
      "defaultTeamRecoveryPresetId": null,
      "defaultTeamProfileId": null,
      "contextMode": "claude-like|codex-like|sentinel-like",
      "runMode": "assistant|team"
    }
  ],
  "teamProfile": {
    "name": "short team name",
    "description": "one concise Chinese description",
    "orchestratorProfileId": "member-ref-1",
    "specialistProfileIds": ["member-ref-2"],
    "monitorProfileId": "member-ref-3",
    "defaultModel": null,
    "contextMode": "claude-like|codex-like|sentinel-like",
    "memoryPolicy": {"monitorGate":"candidate_then_orchestrator_accept","shareScope":"high_value_only","longTermMemory":true},
    "toolPolicyMatrix": {
      "orchestrator": {"tools":["ask_user_question"]},
      "specialist": {"tools":["interactive_shell","shell","file_read","grep","http_request","web_search"]},
      "monitor": {"tools":["tenth_man_review"]}
    },
    "harnessPolicy": {"heartbeatSecs":30,"leaseSecs":600,"checkpoint":"event_sequence","allowResume":true},
    "concurrencyPolicy": {"maxSpecialists":2,"maxTasksPerSpecialist":1},
    "safetyPolicy": {"orchestratorNoDangerousTools":true,"monitorReadOnly":true,"requireApprovalForHighRiskTools":true}
  }
}
Rules:
- Do not include markdown, comments, prose, or extra keys.
- Do not include id fields; the application assigns ids.
- teamProfile role ids must reference member-ref-1, member-ref-2, etc. in assistantProfiles order.
- Include exactly one orchestrator, at least one specialist, and exactly one monitor.
- Orchestrator member should use teamRole "orchestrator" and runMode "team".
- Specialist members should use teamRole "specialist" and runMode "assistant".
- Monitor member should use teamRole "monitor" and runMode "assistant".
"#;

fn default_profile_team_role() -> String {
    "assistant".to_string()
}

fn default_assistant_profiles() -> Vec<AssistantProfilePayload> {
    vec![
        AssistantProfilePayload {
            id: "assistant.default".to_string(),
            label: "Assistant".to_string(),
            description: "默认交互助手，优先使用 Claude-like 上下文和单助手执行。".to_string(),
            team_role: "specialist".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: false,
            default_tool_selection_strategy: "Deferred".to_string(),
            default_max_tools: 12,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            default_team_profile_id: None,
            context_mode: "claude-like".to_string(),
            run_mode: "assistant".to_string(),
        },
        AssistantProfilePayload {
            id: "assistant.reviewer".to_string(),
            label: "Reviewer".to_string(),
            description: "偏审查与复盘的助手，默认使用 Codex-like 上下文。".to_string(),
            team_role: "monitor".to_string(),
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
            default_team_profile_id: None,
            context_mode: "codex-like".to_string(),
            run_mode: "assistant".to_string(),
        },
        AssistantProfilePayload {
            id: "assistant.sentinel".to_string(),
            label: "Sentinel".to_string(),
            description: "长会话与多阶段任务入口，默认使用 Sentinel-like 上下文。".to_string(),
            team_role: "specialist".to_string(),
            default_model: None,
            default_rag_enabled: false,
            default_web_search_enabled: false,
            default_tools_enabled: true,
            default_tenth_man_enabled: true,
            default_tool_selection_strategy: "Deferred".to_string(),
            default_max_tools: 12,
            default_fixed_tools: vec![
                "interactive_shell".to_string(),
                "ask_user_question".to_string(),
                "tenth_man_review".to_string(),
            ],
            default_disabled_tools: vec![],
            default_manual_tools: vec![],
            default_team_orchestration_preset_id: None,
            default_team_recovery_preset_id: None,
            default_team_profile_id: None,
            context_mode: "sentinel-like".to_string(),
            run_mode: "assistant".to_string(),
        },
        AssistantProfilePayload {
            id: "team.lead".to_string(),
            label: "Team Lead".to_string(),
            description: "默认团队编排入口，使用 Claude-like 上下文和 Team 运行模式。".to_string(),
            team_role: "orchestrator".to_string(),
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
            default_team_profile_id: Some("team.profile.default".to_string()),
            context_mode: "claude-like".to_string(),
            run_mode: "team".to_string(),
        },
        AssistantProfilePayload {
            id: "team.reviewer".to_string(),
            label: "Team Reviewer".to_string(),
            description: "团队审查型入口，使用 Codex-like 上下文和 Team 运行模式。".to_string(),
            team_role: "orchestrator".to_string(),
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
            default_team_profile_id: Some("team.profile.review".to_string()),
            context_mode: "codex-like".to_string(),
            run_mode: "team".to_string(),
        },
    ]
}

fn normalize_profile(mut profile: AssistantProfilePayload) -> AssistantProfilePayload {
    profile.id = profile.id.trim().to_string();
    profile.label = profile.label.trim().to_string();
    profile.description = profile.description.trim().to_string();
    profile.team_role = profile.team_role.trim().to_lowercase();
    if profile.team_role.is_empty() {
        profile.team_role = default_profile_team_role();
    }
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
    profile.default_team_profile_id = profile
        .default_team_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    profile
}

fn normalize_profiles(profiles: Vec<AssistantProfilePayload>) -> Vec<AssistantProfilePayload> {
    profiles.into_iter().map(normalize_profile).collect()
}

fn migrate_legacy_team_role(value: &str) -> Option<&'static str> {
    match value.trim().to_lowercase().as_str() {
        "commander" => Some("orchestrator"),
        "solver" => Some("specialist"),
        "observer" => Some("monitor"),
        _ => None,
    }
}

fn decode_stored_profiles(raw: &str) -> Result<(Vec<AssistantProfilePayload>, bool), String> {
    let mut value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let mut changed = false;
    let profiles = value
        .as_array_mut()
        .ok_or_else(|| "assistant profile registry must be an array".to_string())?;

    for profile in profiles {
        let Some(profile_object) = profile.as_object_mut() else {
            return Err("assistant profile registry entries must be objects".to_string());
        };
        if let Some(team_role) = profile_object.get_mut("teamRole") {
            if let Some(role) = team_role.as_str().and_then(migrate_legacy_team_role) {
                *team_role = serde_json::Value::String(role.to_string());
                changed = true;
            }
        }
    }

    let profiles: Vec<AssistantProfilePayload> =
        serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((profiles, changed))
}

fn seed_missing_builtin_profiles(
    mut profiles: Vec<AssistantProfilePayload>,
) -> (Vec<AssistantProfilePayload>, bool) {
    let mut ids: HashSet<String> = profiles.iter().map(|profile| profile.id.clone()).collect();
    let mut changed = false;
    for profile in normalize_profiles(default_assistant_profiles()) {
        if ids.insert(profile.id.clone()) {
            profiles.push(profile);
            changed = true;
        }
    }
    (profiles, changed)
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

fn default_team_profiles() -> Vec<TeamProfilePayload> {
    vec![
        TeamProfilePayload {
            id: "team.profile.default".to_string(),
            name: "Team Lead".to_string(),
            description: "通用 Orchestrator + Specialist + Monitor 团队。".to_string(),
            orchestrator_profile_id: "team.lead".to_string(),
            specialist_profile_ids: vec!["assistant.default".to_string()],
            monitor_profile_id: "assistant.reviewer".to_string(),
            default_model: None,
            context_mode: "claude-like".to_string(),
            memory_policy: serde_json::json!({
                "monitorGate": "candidate_then_orchestrator_accept",
                "shareScope": "high_value_only",
                "longTermMemory": true
            }),
            tool_policy_matrix: serde_json::json!({
                "orchestrator": {"tools": ["ask_user_question"]},
                "specialist": {"tools": ["interactive_shell", "shell", "file_read", "grep", "http_request", "web_search"]},
                "monitor": {"tools": ["tenth_man_review"]}
            }),
            harness_policy: serde_json::json!({
                "heartbeatSecs": 30,
                "leaseSecs": 600,
                "checkpoint": "event_sequence",
                "allowResume": true
            }),
            concurrency_policy: serde_json::json!({
                "maxSpecialists": 2,
                "maxTasksPerSpecialist": 1
            }),
            safety_policy: serde_json::json!({
                "orchestratorNoDangerousTools": true,
                "monitorReadOnly": true,
                "requireApprovalForHighRiskTools": true
            }),
        },
        TeamProfilePayload {
            id: "team.profile.review".to_string(),
            name: "Security Review Team".to_string(),
            description: "安全审查 Orchestrator + 审查 Specialist + Monitor 团队。".to_string(),
            orchestrator_profile_id: "team.reviewer".to_string(),
            specialist_profile_ids: vec!["assistant.sentinel".to_string()],
            monitor_profile_id: "assistant.reviewer".to_string(),
            default_model: None,
            context_mode: "codex-like".to_string(),
            memory_policy: serde_json::json!({
                "monitorGate": "candidate_then_orchestrator_accept",
                "shareScope": "evidence_risk_decision",
                "longTermMemory": true
            }),
            tool_policy_matrix: serde_json::json!({
                "orchestrator": {"tools": ["ask_user_question", "tenth_man_review"]},
                "specialist": {"tools": ["interactive_shell", "shell", "file_read", "grep", "http_request", "web_search", "tenth_man_review"]},
                "monitor": {"tools": ["tenth_man_review"]}
            }),
            harness_policy: serde_json::json!({
                "heartbeatSecs": 30,
                "leaseSecs": 900,
                "checkpoint": "event_sequence",
                "allowResume": true
            }),
            concurrency_policy: serde_json::json!({
                "maxSpecialists": 2,
                "maxTasksPerSpecialist": 1
            }),
            safety_policy: serde_json::json!({
                "orchestratorNoDangerousTools": true,
                "monitorReadOnly": true,
                "requireApprovalForHighRiskTools": true
            }),
        },
        TeamProfilePayload {
            id: "team.profile.incident".to_string(),
            name: "Incident Team".to_string(),
            description: "故障处理 Orchestrator + Log/Fix Specialist + Monitor 团队。".to_string(),
            orchestrator_profile_id: "team.lead".to_string(),
            specialist_profile_ids: vec![
                "assistant.sentinel".to_string(),
                "assistant.default".to_string(),
            ],
            monitor_profile_id: "assistant.reviewer".to_string(),
            default_model: None,
            context_mode: "sentinel-like".to_string(),
            memory_policy: serde_json::json!({
                "monitorGate": "candidate_then_orchestrator_accept",
                "shareScope": "evidence_risk_blocker_checkpoint",
                "longTermMemory": true
            }),
            tool_policy_matrix: serde_json::json!({
                "orchestrator": {"tools": ["ask_user_question"]},
                "specialist": {"tools": ["interactive_shell", "shell", "file_read", "grep", "http_request", "web_search", "tenth_man_review"]},
                "monitor": {"tools": ["tenth_man_review"]}
            }),
            harness_policy: serde_json::json!({
                "heartbeatSecs": 30,
                "leaseSecs": 1200,
                "checkpoint": "event_sequence",
                "allowResume": true
            }),
            concurrency_policy: serde_json::json!({
                "maxSpecialists": 2,
                "maxTasksPerSpecialist": 1
            }),
            safety_policy: serde_json::json!({
                "orchestratorNoDangerousTools": true,
                "monitorReadOnly": true,
                "requireApprovalForHighRiskTools": true,
                "failFastWaitStartedAssignments": true
            }),
        },
    ]
}

fn normalize_json_object(value: serde_json::Value) -> serde_json::Value {
    if value.is_object() {
        value
    } else {
        serde_json::json!({})
    }
}

fn normalize_json_tool_ids(value: Option<&serde_json::Value>) -> Vec<String> {
    let Some(serde_json::Value::Array(items)) = value else {
        return Vec::new();
    };
    normalize_tool_ids(
        items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
    )
}

fn normalize_team_role_tools(
    matrix: &serde_json::Value,
    role: &str,
    default_tools: &[&str],
) -> serde_json::Value {
    let role_policy = matrix.get(role).and_then(|value| value.as_object());
    let tools = role_policy
        .map(|policy| {
            normalize_json_tool_ids(
                policy
                    .get("tools")
                    .or_else(|| policy.get("allowed"))
                    .or_else(|| policy.get("manualTools"))
                    .or_else(|| policy.get("fixedTools")),
            )
        })
        .unwrap_or_default();
    let final_tools = if tools.is_empty() {
        default_tools.iter().map(|tool| tool.to_string()).collect()
    } else {
        tools
    };
    serde_json::json!({ "tools": final_tools })
}

fn normalize_team_tool_policy_matrix(value: serde_json::Value) -> serde_json::Value {
    let matrix = normalize_json_object(value);
    serde_json::json!({
        "orchestrator": normalize_team_role_tools(&matrix, "orchestrator", &["ask_user_question"]),
        "specialist": normalize_team_role_tools(&matrix, "specialist", &[
            "interactive_shell",
            "shell",
            "file_read",
            "grep",
            "http_request",
            "web_search"
        ]),
        "monitor": normalize_team_role_tools(&matrix, "monitor", &["tenth_man_review"])
    })
}

fn normalize_team_profile(mut profile: TeamProfilePayload) -> TeamProfilePayload {
    profile.id = profile.id.trim().to_string();
    profile.name = profile.name.trim().to_string();
    profile.description = profile.description.trim().to_string();
    profile.orchestrator_profile_id = profile.orchestrator_profile_id.trim().to_string();
    profile.monitor_profile_id = profile.monitor_profile_id.trim().to_string();
    profile.specialist_profile_ids = normalize_tool_ids(profile.specialist_profile_ids);
    profile.default_model = profile
        .default_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    profile.context_mode = profile.context_mode.trim().to_string();
    profile.memory_policy = normalize_json_object(profile.memory_policy);
    profile.tool_policy_matrix = normalize_team_tool_policy_matrix(profile.tool_policy_matrix);
    profile.harness_policy = normalize_json_object(profile.harness_policy);
    profile.concurrency_policy = normalize_json_object(profile.concurrency_policy);
    profile.safety_policy = normalize_json_object(profile.safety_policy);
    profile
}

fn normalize_team_profiles(profiles: Vec<TeamProfilePayload>) -> Vec<TeamProfilePayload> {
    profiles.into_iter().map(normalize_team_profile).collect()
}

fn migrate_legacy_policy_literals(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(raw) => {
            let migrated = match raw.as_str() {
                "candidate_then_commander_accept" => "candidate_then_orchestrator_accept",
                "commander_only" => "orchestrator_only",
                "observer_only" => "monitor_only",
                "dynamic_with_commander_approval" => "dynamic_with_orchestrator_approval",
                _ => return false,
            };
            *raw = migrated.to_string();
            true
        }
        serde_json::Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed = migrate_legacy_policy_literals(item) || changed;
            }
            changed
        }
        serde_json::Value::Object(object) => {
            let mut changed = false;
            for item in object.values_mut() {
                changed = migrate_legacy_policy_literals(item) || changed;
            }
            changed
        }
        _ => false,
    }
}

fn migrate_legacy_team_profile_registry(value: &mut serde_json::Value) -> Result<bool, String> {
    let mut changed = false;
    let profiles = value
        .as_array_mut()
        .ok_or_else(|| "team profile registry must be an array".to_string())?;

    for profile in profiles {
        let Some(profile_object) = profile.as_object_mut() else {
            return Err("team profile registry entries must be objects".to_string());
        };
        if let Some(value) = profile_object.remove("commanderProfileId") {
            profile_object.insert("orchestratorProfileId".to_string(), value);
            changed = true;
        }
        if let Some(value) = profile_object.remove("solverProfileIds") {
            profile_object.insert("specialistProfileIds".to_string(), value);
            changed = true;
        }
        if let Some(value) = profile_object.remove("observerProfileId") {
            profile_object.insert("monitorProfileId".to_string(), value);
            changed = true;
        }
        if let Some(tool_policy_matrix) = profile_object.get_mut("toolPolicyMatrix") {
            let Some(matrix) = tool_policy_matrix.as_object_mut() else {
                continue;
            };
            if let Some(value) = matrix.remove("commander") {
                matrix.insert("orchestrator".to_string(), value);
                changed = true;
            }
            if let Some(value) = matrix.remove("solver") {
                matrix.insert("specialist".to_string(), value);
                changed = true;
            }
            if let Some(value) = matrix.remove("observer") {
                matrix.insert("monitor".to_string(), value);
                changed = true;
            }
        }
        if let Some(memory_policy) = profile_object.get_mut("memoryPolicy") {
            let Some(policy) = memory_policy.as_object_mut() else {
                continue;
            };
            if let Some(value) = policy.remove("observerGate") {
                policy.insert("monitorGate".to_string(), value);
                changed = true;
            }
        }
        if let Some(safety_policy) = profile_object.get_mut("safetyPolicy") {
            let Some(policy) = safety_policy.as_object_mut() else {
                continue;
            };
            if let Some(value) = policy.remove("commanderNoDangerousTools") {
                policy.insert("orchestratorNoDangerousTools".to_string(), value);
                changed = true;
            }
            if let Some(value) = policy.remove("observerReadOnly") {
                policy.insert("monitorReadOnly".to_string(), value);
                changed = true;
            }
        }
        changed = migrate_legacy_policy_literals(profile) || changed;
    }

    Ok(changed)
}

fn decode_stored_team_profiles(raw: &str) -> Result<(Vec<TeamProfilePayload>, bool), String> {
    let mut value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let changed = migrate_legacy_team_profile_registry(&mut value)?;
    let profiles: Vec<TeamProfilePayload> =
        serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((profiles, changed))
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
        if profile.context_mode != "claude-like"
            && profile.context_mode != "codex-like"
            && profile.context_mode != "sentinel-like"
        {
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
        if !PROFILE_TEAM_ROLES.contains(&profile.team_role.as_str()) {
            return Err(format!(
                "assistant profile {} has unsupported team role {}",
                profile.id, profile.team_role
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

fn validate_team_profiles(
    team_profiles: &[TeamProfilePayload],
    assistant_profiles: &[AssistantProfilePayload],
) -> Result<(), String> {
    if team_profiles.is_empty() {
        return Err("team profiles cannot be empty".to_string());
    }

    let assistant_ids: HashSet<String> = assistant_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect();
    let mut ids = HashSet::new();
    for profile in team_profiles {
        if profile.id.trim().is_empty() {
            return Err("team profile id cannot be empty".to_string());
        }
        if profile.name.trim().is_empty() {
            return Err(format!("team profile {} name cannot be empty", profile.id));
        }
        if !ids.insert(profile.id.clone()) {
            return Err(format!("duplicate team profile id: {}", profile.id));
        }
        if profile.orchestrator_profile_id.trim().is_empty()
            || !assistant_ids.contains(&profile.orchestrator_profile_id)
        {
            return Err(format!(
                "team profile {} has unknown orchestrator profile {}",
                profile.id, profile.orchestrator_profile_id
            ));
        }
        if profile.monitor_profile_id.trim().is_empty()
            || !assistant_ids.contains(&profile.monitor_profile_id)
        {
            return Err(format!(
                "team profile {} has unknown monitor profile {}",
                profile.id, profile.monitor_profile_id
            ));
        }
        if profile.specialist_profile_ids.is_empty() {
            return Err(format!(
                "team profile {} must include at least one specialist profile",
                profile.id
            ));
        }
        for specialist_profile_id in &profile.specialist_profile_ids {
            if !assistant_ids.contains(specialist_profile_id) {
                return Err(format!(
                    "team profile {} has unknown specialist profile {}",
                    profile.id, specialist_profile_id
                ));
            }
        }
        if profile.context_mode != "claude-like"
            && profile.context_mode != "codex-like"
            && profile.context_mode != "sentinel-like"
        {
            return Err(format!(
                "team profile {} has unsupported context mode {}",
                profile.id, profile.context_mode
            ));
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
            let (profiles, migrated) = decode_stored_profiles(&raw)?;
            let profiles = normalize_profiles(profiles);
            let (profiles, seeded) = seed_missing_builtin_profiles(profiles);
            validate_profiles(&profiles)?;
            if migrated || seeded {
                let raw = serde_json::to_string(&profiles).map_err(|e| e.to_string())?;
                db_service
                    .set_config(
                        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
                        ASSISTANT_PROFILE_CONFIG_KEY,
                        &raw,
                        Some("Assistant profile registry"),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
            }
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

async fn load_team_profiles(
    db_service: &sentinel_db::DatabaseService,
) -> Result<Vec<TeamProfilePayload>, String> {
    let assistant_profiles = load_profiles(db_service).await?;
    match db_service
        .get_config(ASSISTANT_PROFILE_CONFIG_NAMESPACE, TEAM_PROFILE_CONFIG_KEY)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) => {
            let (team_profiles, migrated) = decode_stored_team_profiles(&raw)?;
            let team_profiles = normalize_team_profiles(team_profiles);
            if team_profiles.is_empty() {
                let defaults = normalize_team_profiles(default_team_profiles());
                validate_team_profiles(&defaults, &assistant_profiles)?;
                return Ok(defaults);
            }
            validate_team_profiles(&team_profiles, &assistant_profiles)?;
            if migrated {
                persist_team_profiles_raw(db_service, &team_profiles).await?;
            }
            Ok(team_profiles)
        }
        None => {
            let team_profiles = normalize_team_profiles(default_team_profiles());
            validate_team_profiles(&team_profiles, &assistant_profiles)?;
            Ok(team_profiles)
        }
    }
}

async fn load_default_team_profile_id(
    db_service: &sentinel_db::DatabaseService,
) -> Result<String, String> {
    let team_profiles = load_team_profiles(db_service).await?;
    let fallback_id = team_profiles
        .first()
        .map(|profile| profile.id.clone())
        .ok_or_else(|| "team profiles cannot be empty".to_string())?;

    match db_service
        .get_config(ASSISTANT_PROFILE_CONFIG_NAMESPACE, TEAM_PROFILE_DEFAULT_KEY)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) if team_profiles.iter().any(|profile| profile.id == raw) => Ok(raw),
        _ => Ok(fallback_id),
    }
}

fn short_uuid() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(12)
        .collect()
}

fn ensure_description(raw: String) -> Result<String, String> {
    let description = raw.trim().to_string();
    if description.is_empty() {
        return Err("description cannot be empty".to_string());
    }
    Ok(description)
}

fn extract_json_object(raw: &str) -> Result<&str, String> {
    let start = raw
        .find('{')
        .ok_or_else(|| "AI response did not contain a JSON object".to_string())?;
    let end = raw
        .rfind('}')
        .ok_or_else(|| "AI response did not contain a complete JSON object".to_string())?;
    if end <= start {
        return Err("AI response JSON object was malformed".to_string());
    }
    Ok(&raw[start..=end])
}

fn agent_ai_user_prompt(
    description: &str,
    existing_profiles: &[AssistantProfilePayload],
) -> String {
    let existing = existing_profiles
        .iter()
        .map(|profile| {
            serde_json::json!({
                "id": profile.id,
                "label": profile.label,
                "teamRole": profile.team_role,
                "runMode": profile.run_mode,
                "contextMode": profile.context_mode
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "userDescription": description,
        "existingProfiles": existing
    })
    .to_string()
}

fn team_ai_user_prompt(
    description: &str,
    existing_profiles: &[AssistantProfilePayload],
    existing_teams: &[TeamProfilePayload],
) -> String {
    let profiles = existing_profiles
        .iter()
        .map(|profile| {
            serde_json::json!({
                "id": profile.id,
                "label": profile.label,
                "teamRole": profile.team_role,
                "runMode": profile.run_mode,
                "contextMode": profile.context_mode
            })
        })
        .collect::<Vec<_>>();
    let teams = existing_teams
        .iter()
        .map(|team| {
            serde_json::json!({
                "id": team.id,
                "name": team.name,
                "description": team.description
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "userDescription": description,
        "existingProfiles": profiles,
        "existingTeams": teams
    })
    .to_string()
}

async fn run_profile_generation(
    ai_manager: &Arc<AiServiceManager>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<(String, String, String), String> {
    let llm_config = ai_manager
        .resolve_generation_llm_config(None, None)
        .await
        .map_err(|e| e.to_string())?;
    let provider = llm_config.provider.clone();
    let model = llm_config.model.clone();
    let client = LlmClient::new(llm_config);
    let raw = client
        .completion(Some(system_prompt), user_prompt)
        .await
        .map_err(|e| format!("AI profile generation failed: {e}"))?;
    Ok((raw, provider, model))
}

fn assign_agent_profile_id(
    mut profile: AssistantProfilePayload,
    prefix: &str,
) -> AssistantProfilePayload {
    profile.id = format!("{prefix}.{}", short_uuid());
    profile
}

fn assign_team_profile_id(mut profile: TeamProfilePayload) -> TeamProfilePayload {
    profile.id = format!("team.profile.ai.{}", short_uuid());
    profile
}

fn resolve_generated_team_refs(
    draft: AiTeamProfileDraft,
) -> Result<(Vec<AssistantProfilePayload>, TeamProfilePayload), String> {
    let mut generated_profiles = Vec::new();
    let mut ref_map = std::collections::HashMap::new();

    for (index, profile) in draft.assistant_profiles.into_iter().enumerate() {
        let generated = assign_agent_profile_id(profile, "assistant.ai.team");
        ref_map.insert(format!("member-ref-{}", index + 1), generated.id.clone());
        generated_profiles.push(generated);
    }

    if generated_profiles.is_empty() {
        return Err("AI team generation must include member agent profiles".to_string());
    }

    let mut team_profile = assign_team_profile_id(draft.team_profile);
    team_profile.orchestrator_profile_id = ref_map
        .get(&team_profile.orchestrator_profile_id)
        .cloned()
        .unwrap_or(team_profile.orchestrator_profile_id);
    team_profile.monitor_profile_id = ref_map
        .get(&team_profile.monitor_profile_id)
        .cloned()
        .unwrap_or(team_profile.monitor_profile_id);
    team_profile.specialist_profile_ids = team_profile
        .specialist_profile_ids
        .into_iter()
        .map(|id| ref_map.get(&id).cloned().unwrap_or(id))
        .collect();

    Ok((generated_profiles, team_profile))
}

async fn persist_profiles_raw(
    db: &sentinel_db::DatabaseService,
    profiles: &[AssistantProfilePayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(profiles).map_err(|e| e.to_string())?;
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        ASSISTANT_PROFILE_CONFIG_KEY,
        &raw,
        Some("Assistant profile registry"),
    )
    .await
    .map_err(|e| e.to_string())
}

async fn persist_team_profiles_raw(
    db: &sentinel_db::DatabaseService,
    profiles: &[TeamProfilePayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(profiles).map_err(|e| e.to_string())?;
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        TEAM_PROFILE_CONFIG_KEY,
        &raw,
        Some("Team profile registry"),
    )
    .await
    .map_err(|e| e.to_string())
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
pub async fn ai_create_assistant_profile_from_description(
    request: AiCreateProfileRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
    ai_manager: tauri::State<'_, Arc<AiServiceManager>>,
) -> Result<AiCreatedAssistantProfileResponse, String> {
    let description = ensure_description(request.description)?;
    let db = db_service.inner().as_ref();
    let mut profiles = load_profiles(db).await?;
    let user_prompt = agent_ai_user_prompt(&description, &profiles);
    let (raw, provider, model) = run_profile_generation(
        ai_manager.inner(),
        AI_AGENT_PROFILE_SYSTEM_PROMPT,
        &user_prompt,
    )
    .await?;
    let json = extract_json_object(&raw)?;
    let generated: AssistantProfilePayload =
        serde_json::from_str(json).map_err(|e| format!("AI agent JSON parse failed: {e}"))?;
    let profile = normalize_profile(assign_agent_profile_id(generated, "assistant.ai"));

    profiles.push(profile.clone());
    validate_profiles(&profiles)?;
    persist_profiles_raw(db, &profiles).await?;

    Ok(AiCreatedAssistantProfileResponse {
        profile,
        provider,
        model,
    })
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

#[tauri::command]
pub async fn list_team_profiles(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<TeamProfilePayload>, String> {
    load_team_profiles(db_service.inner().as_ref()).await
}

#[tauri::command]
pub async fn ai_create_team_profile_from_description(
    request: AiCreateProfileRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
    ai_manager: tauri::State<'_, Arc<AiServiceManager>>,
) -> Result<AiCreatedTeamProfileResponse, String> {
    let description = ensure_description(request.description)?;
    let db = db_service.inner().as_ref();
    let mut assistant_profiles = load_profiles(db).await?;
    let mut team_profiles = load_team_profiles(db).await?;
    let user_prompt = team_ai_user_prompt(&description, &assistant_profiles, &team_profiles);
    let (raw, provider, model) = run_profile_generation(
        ai_manager.inner(),
        AI_TEAM_PROFILE_SYSTEM_PROMPT,
        &user_prompt,
    )
    .await?;
    let json = extract_json_object(&raw)?;
    let draft: AiTeamProfileDraft =
        serde_json::from_str(json).map_err(|e| format!("AI team JSON parse failed: {e}"))?;
    let (generated_profiles, team_profile) = resolve_generated_team_refs(draft)?;
    let generated_profiles = normalize_profiles(generated_profiles);
    let team_profile = normalize_team_profile(team_profile);

    assistant_profiles.extend(generated_profiles.iter().cloned());
    team_profiles.push(team_profile.clone());
    validate_profiles(&assistant_profiles)?;
    validate_team_profiles(&team_profiles, &assistant_profiles)?;

    persist_profiles_raw(db, &assistant_profiles).await?;
    persist_team_profiles_raw(db, &team_profiles).await?;

    Ok(AiCreatedTeamProfileResponse {
        assistant_profiles: generated_profiles,
        team_profile,
        provider,
        model,
    })
}

#[tauri::command]
pub async fn save_team_profiles(
    profiles: Vec<TeamProfilePayload>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    let assistant_profiles = load_profiles(db_service.inner().as_ref()).await?;
    let profiles = normalize_team_profiles(profiles);
    validate_team_profiles(&profiles, &assistant_profiles)?;
    let raw = serde_json::to_string(&profiles).map_err(|e| e.to_string())?;
    let db = db_service.inner().as_ref();
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        TEAM_PROFILE_CONFIG_KEY,
        &raw,
        Some("Team profile registry"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let current_default = db
        .get_config(ASSISTANT_PROFILE_CONFIG_NAMESPACE, TEAM_PROFILE_DEFAULT_KEY)
        .await
        .map_err(|e| e.to_string())?;
    let next_default = match current_default {
        Some(default_id) if profiles.iter().any(|profile| profile.id == default_id) => default_id,
        _ => profiles
            .first()
            .map(|profile| profile.id.clone())
            .ok_or_else(|| "team profiles cannot be empty".to_string())?,
    };
    db.set_config(
        ASSISTANT_PROFILE_CONFIG_NAMESPACE,
        TEAM_PROFILE_DEFAULT_KEY,
        &next_default,
        Some("Default team profile id"),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_team_profile_id(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    load_default_team_profile_id(db_service.inner().as_ref()).await
}

#[tauri::command]
pub async fn save_default_team_profile_id(
    profile_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    let profile_id = profile_id.trim().to_string();
    let profiles = load_team_profiles(db_service.inner().as_ref()).await?;
    if !profiles.iter().any(|profile| profile.id == profile_id) {
        return Err(format!("unknown team profile id: {}", profile_id));
    }

    db_service
        .inner()
        .as_ref()
        .set_config(
            ASSISTANT_PROFILE_CONFIG_NAMESPACE,
            TEAM_PROFILE_DEFAULT_KEY,
            &profile_id,
            Some("Default team profile id"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_stored_legacy_profile_roles_before_validation() {
        let raw = r#"[
            {
                "id":"assistant.sentinel",
                "label":"Sentinel",
                "description":"legacy",
                "teamRole":"solver",
                "defaultModel":null,
                "defaultRagEnabled":false,
                "defaultWebSearchEnabled":false,
                "defaultToolsEnabled":true,
                "defaultTenthManEnabled":true,
                "defaultToolSelectionStrategy":"Deferred",
                "defaultMaxTools":12,
                "defaultFixedTools":[],
                "defaultDisabledTools":[],
                "defaultManualTools":[],
                "defaultTeamOrchestrationPresetId":null,
                "defaultTeamRecoveryPresetId":null,
                "defaultTeamProfileId":null,
                "contextMode":"sentinel-like",
                "runMode":"assistant"
            }
        ]"#;

        let (profiles, migrated) = decode_stored_profiles(raw).unwrap();
        let profiles = normalize_profiles(profiles);

        assert!(migrated);
        assert_eq!(profiles[0].team_role, "specialist");
        validate_profiles(&profiles).unwrap();
    }

    #[test]
    fn rejects_legacy_profile_roles_from_normal_save_payloads() {
        let mut profile = normalize_profile(default_assistant_profiles()[0].clone());
        profile.team_role = "solver".to_string();

        assert!(validate_profiles(&[profile]).is_err());
    }

    #[test]
    fn migrates_stored_legacy_team_profile_fields() {
        let raw = r#"[
            {
                "id":"team.profile.default",
                "name":"Team Lead",
                "description":"legacy",
                "commanderProfileId":"team.lead",
                "solverProfileIds":["assistant.sentinel"],
                "observerProfileId":"assistant.reviewer",
                "defaultModel":null,
                "contextMode":"claude-like",
                "memoryPolicy":{"observerGate":"candidate_then_commander_accept"},
                "toolPolicyMatrix":{
                    "commander":{"tools":["ask_user_question"]},
                    "solver":{"tools":["shell"]},
                    "observer":{"tools":["tenth_man_review"]}
                },
                "harnessPolicy":{},
                "concurrencyPolicy":{},
                "safetyPolicy":{"commanderNoDangerousTools":true,"observerReadOnly":true}
            }
        ]"#;

        let (profiles, migrated) = decode_stored_team_profiles(raw).unwrap();
        let profiles = normalize_team_profiles(profiles);
        let team = &profiles[0];

        assert!(migrated);
        assert_eq!(team.orchestrator_profile_id, "team.lead");
        assert_eq!(team.specialist_profile_ids, vec!["assistant.sentinel"]);
        assert_eq!(team.monitor_profile_id, "assistant.reviewer");
        assert!(team.tool_policy_matrix.get("specialist").is_some());
        assert!(team.tool_policy_matrix.get("solver").is_none());
        assert_eq!(
            team.memory_policy
                .get("monitorGate")
                .and_then(serde_json::Value::as_str),
            Some("candidate_then_orchestrator_accept")
        );
    }

    #[test]
    fn seeds_missing_builtin_assistant_profiles() {
        let persisted = normalize_profiles(
            default_assistant_profiles()
                .into_iter()
                .filter(|profile| profile.id != "assistant.sentinel")
                .collect(),
        );

        let (profiles, changed) = seed_missing_builtin_profiles(persisted);

        assert!(changed);
        assert!(profiles
            .iter()
            .any(|profile| profile.id == "assistant.sentinel"));
    }

    #[test]
    fn default_team_profiles_validate_after_builtin_seed() {
        let persisted = normalize_profiles(
            default_assistant_profiles()
                .into_iter()
                .filter(|profile| profile.id != "assistant.sentinel")
                .collect(),
        );
        let teams = normalize_team_profiles(default_team_profiles());

        assert!(validate_team_profiles(&teams, &persisted).is_err());

        let (profiles, _) = seed_missing_builtin_profiles(persisted);

        validate_team_profiles(&teams, &profiles).unwrap();
    }
}
