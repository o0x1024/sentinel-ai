use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

use sentinel_db::{Database, DatabaseService, SystemAgentBindingRecord, SystemAgentProfileRecord};

fn default_profiles() -> Vec<(SystemAgentProfileRecord, Vec<SystemAgentBindingRecord>)> {
    let now = Utc::now();

    let passive_logic = SystemAgentProfileRecord {
        id: "traffic_logic_triage".to_string(),
        name: "Traffic Logic Triage".to_string(),
        description: "流量历史中的逻辑漏洞被动分诊 Agent，覆盖越权、状态机和流程异常。".to_string(),
        mode: "passive".to_string(),
        capability: "triage".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:traffic_logic_triage".to_string()),
        prompt_patch: Some(
            "重点识别越权、对象边界、跳步骤、重复提交、逆序执行和状态机异常。".to_string(),
        ),
        sop_definitions_json: "[]".to_string(),
        input_schema_json: json!({"type": "object"}).to_string(),
        output_schema_json: json!({"type": "object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec![
            "traffic_cluster_reader".to_string(),
            "auth_context_diff".to_string(),
            "workflow_state_reader".to_string(),
            "traffic_sequence_reader".to_string(),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: serde_json::to_string(&vec!["active_replay".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        trigger_events_json: serde_json::to_string(&vec!["traffic.cluster.ready".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        budget_json: json!({"maxRunsPerHour": 60, "maxTokensPerRun": 1000}).to_string(),
        safety_policy_json:
            json!({"allowActiveReplay": false, "autoMode": false, "shadowMode": false}).to_string(),
        cooldown_secs: 20,
        max_concurrency: 2,
        risk_level: "high".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    let active_verifier = SystemAgentProfileRecord {
        id: "traffic_active_verifier".to_string(),
        name: "Traffic Active Verifier".to_string(),
        description: "对 AI 分诊后的流量发现执行安全重放验证。".to_string(),
        mode: "passive".to_string(),
        capability: "verifier".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:traffic_active_verifier".to_string()),
        prompt_patch: Some("仅对已有证据的 finding 执行最小化安全重放验证。".to_string()),
        sop_definitions_json: "[]".to_string(),
        input_schema_json: json!({"type": "object"}).to_string(),
        output_schema_json: json!({"type": "object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec!["active_replay".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: serde_json::to_string(&vec!["traffic.hypothesis.ready".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        budget_json: json!({"maxRunsPerHour": 20, "maxTokensPerRun": 0}).to_string(),
        safety_policy_json: json!({"allowActiveReplay": true, "autoMode": false, "shadowMode": false, "scopeHosts": []}).to_string(),
        cooldown_secs: 5,
        max_concurrency: 1,
        risk_level: "critical".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    vec![
        (
            active_verifier.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-active-verifier".to_string(),
                profile_id: active_verifier.id.clone(),
                event_name: "traffic.hypothesis.ready".to_string(),
                filter_json: "{}".to_string(),
                priority: 80,
                enabled: false,
                created_at: now,
                updated_at: now,
            }],
        ),
        (
            passive_logic.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-logic-triage".to_string(),
                profile_id: passive_logic.id.clone(),
                event_name: "traffic.cluster.ready".to_string(),
                filter_json: "{}".to_string(),
                priority: 100,
                enabled: true,
                created_at: now,
                updated_at: now,
            }],
        ),
    ]
}

async fn hide_and_disable_profile(db: &Arc<DatabaseService>, profile_id: &str) -> Result<()> {
    if let Some(mut profile) = db.get_system_agent_profile(profile_id).await? {
        profile.enabled = false;
        profile.visibility = "hidden".to_string();
        profile.updated_at = Utc::now();
        db.save_system_agent_profile(&profile, &[]).await?;
    }

    Ok(())
}

fn auto_mode_enabled_from_profile(profile: &SystemAgentProfileRecord) -> bool {
    serde_json::from_str::<Value>(&profile.safety_policy_json)
        .ok()
        .and_then(|value| value.get("autoMode").and_then(Value::as_bool))
        .unwrap_or(false)
}

async fn sync_existing_profile_bindings(db: &Arc<DatabaseService>, profile_id: &str) -> Result<()> {
    let Some(profile) = db.get_system_agent_profile(profile_id).await? else {
        return Ok(());
    };

    let mut bindings = db.list_system_agent_bindings(Some(profile_id)).await?;
    let mut changed = false;

    if profile_id == "traffic_active_verifier" {
        let desired_enabled = profile.enabled && auto_mode_enabled_from_profile(&profile);
        if let Some(binding) = bindings
            .iter_mut()
            .find(|binding| binding.event_name == "traffic.hypothesis.ready")
        {
            if binding.enabled != desired_enabled {
                binding.enabled = desired_enabled;
                binding.updated_at = Utc::now();
                changed = true;
            }
        }
    }

    if changed {
        db.save_system_agent_profile(&profile, &bindings).await?;
    }

    Ok(())
}

pub async fn ensure_default_system_agent_profiles(db: Arc<DatabaseService>) -> Result<()> {
    hide_and_disable_profile(&db, "traffic_idor_triage").await?;

    let existing_ids = db
        .list_system_agent_profiles()
        .await?
        .into_iter()
        .map(|profile| profile.id)
        .collect::<std::collections::HashSet<_>>();

    for (profile, bindings) in default_profiles() {
        if !existing_ids.contains(&profile.id) {
            db.save_system_agent_profile(&profile, &bindings).await?;
        }
    }

    sync_existing_profile_bindings(&db, "traffic_active_verifier").await?;

    Ok(())
}
