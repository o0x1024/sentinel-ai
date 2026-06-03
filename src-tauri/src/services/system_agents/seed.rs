use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::services::system_agents::pipeline::{
    EVENT_TRAFFIC_CONTEXT_READY, EVENT_TRAFFIC_HYPOTHESIS_READY, EVENT_TRAFFIC_RAW_READY,
    EVENT_TRAFFIC_VERIFICATION_REQUESTED, TRAFFIC_CONTEXT_AGENT_PROFILE_ID,
    TRAFFIC_DECISION_AGENT_PROFILE_ID, TRAFFIC_HYPOTHESIS_AGENT_PROFILE_ID,
    TRAFFIC_VERIFICATION_AGENT_PROFILE_ID,
};
use sentinel_db::{Database, DatabaseService, SystemAgentBindingRecord, SystemAgentProfileRecord};

fn default_profiles() -> Vec<(SystemAgentProfileRecord, Vec<SystemAgentBindingRecord>)> {
    let now = Utc::now();

    let context_agent = SystemAgentProfileRecord {
        id: TRAFFIC_CONTEXT_AGENT_PROFILE_ID.to_string(),
        name: "Traffic Context Agent".to_string(),
        description: "对原始流量事件构建统一安全上下文的后台 Agent。".to_string(),
        mode: "passive".to_string(),
        capability: "context".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: None,
        prompt_patch: None,
        sop_definitions_json: "[]".to_string(),
        input_schema_json: json!({"type": "object"}).to_string(),
        output_schema_json: json!({"type": "object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: "[]".to_string(),
        forbidden_tools_json: serde_json::to_string(&vec!["active_replay".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        trigger_events_json: serde_json::to_string(&vec![EVENT_TRAFFIC_RAW_READY.to_string()])
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

    let hypothesis_agent = SystemAgentProfileRecord {
        id: TRAFFIC_HYPOTHESIS_AGENT_PROFILE_ID.to_string(),
        name: "Traffic Hypothesis Agent".to_string(),
        description: "基于统一安全上下文生成漏洞假设与初始验证计划。".to_string(),
        mode: "passive".to_string(),
        capability: "hypothesis".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:traffic_hypothesis_agent".to_string()),
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
        trigger_events_json: serde_json::to_string(&vec![EVENT_TRAFFIC_CONTEXT_READY.to_string()])
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

    let verification_agent = SystemAgentProfileRecord {
        id: TRAFFIC_VERIFICATION_AGENT_PROFILE_ID.to_string(),
        name: "Traffic Verification Agent".to_string(),
        description: "对已升格的漏洞假设执行安全重放验证。".to_string(),
        mode: "passive".to_string(),
        capability: "verification".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:traffic_verification_agent".to_string()),
        prompt_patch: Some("仅对已有证据的 finding 执行最小化安全重放验证。".to_string()),
        sop_definitions_json: "[]".to_string(),
        input_schema_json: json!({"type": "object"}).to_string(),
        output_schema_json: json!({"type": "object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec!["active_replay".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: serde_json::to_string(&vec![EVENT_TRAFFIC_VERIFICATION_REQUESTED.to_string()])
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

    let decision_agent = SystemAgentProfileRecord {
        id: TRAFFIC_DECISION_AGENT_PROFILE_ID.to_string(),
        name: "Traffic Decision Agent".to_string(),
        description: "决定是否将漏洞假设升格为 finding，并驱动后续验证。".to_string(),
        mode: "passive".to_string(),
        capability: "decision".to_string(),
        enabled: true,
        trigger_mode: "event".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: None,
        prompt_patch: None,
        sop_definitions_json: "[]".to_string(),
        input_schema_json: json!({"type": "object"}).to_string(),
        output_schema_json: json!({"type": "object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: "[]".to_string(),
        forbidden_tools_json: serde_json::to_string(&vec!["active_replay".to_string()])
            .unwrap_or_else(|_| "[]".to_string()),
        trigger_events_json: serde_json::to_string(&vec![
            EVENT_TRAFFIC_HYPOTHESIS_READY.to_string()
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        budget_json: json!({"maxRunsPerHour": 60, "maxTokensPerRun": 0}).to_string(),
        safety_policy_json:
            json!({"allowActiveReplay": false, "autoMode": true, "shadowMode": false}).to_string(),
        cooldown_secs: 5,
        max_concurrency: 2,
        risk_level: "high".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    vec![
        (
            context_agent.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-context-agent".to_string(),
                profile_id: context_agent.id.clone(),
                event_name: EVENT_TRAFFIC_RAW_READY.to_string(),
                filter_json: "{}".to_string(),
                priority: 120,
                enabled: true,
                created_at: now,
                updated_at: now,
            }],
        ),
        (
            hypothesis_agent.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-hypothesis-agent".to_string(),
                profile_id: hypothesis_agent.id.clone(),
                event_name: EVENT_TRAFFIC_CONTEXT_READY.to_string(),
                filter_json: "{}".to_string(),
                priority: 100,
                enabled: true,
                created_at: now,
                updated_at: now,
            }],
        ),
        (
            decision_agent.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-decision-agent".to_string(),
                profile_id: decision_agent.id.clone(),
                event_name: EVENT_TRAFFIC_HYPOTHESIS_READY.to_string(),
                filter_json: "{}".to_string(),
                priority: 90,
                enabled: true,
                created_at: now,
                updated_at: now,
            }],
        ),
        (
            verification_agent.clone(),
            vec![SystemAgentBindingRecord {
                id: "sab-traffic-verification-agent".to_string(),
                profile_id: verification_agent.id.clone(),
                event_name: EVENT_TRAFFIC_VERIFICATION_REQUESTED.to_string(),
                filter_json: "{}".to_string(),
                priority: 80,
                enabled: false,
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

    if profile_id == TRAFFIC_VERIFICATION_AGENT_PROFILE_ID {
        let desired_enabled = profile.enabled && auto_mode_enabled_from_profile(&profile);
        if let Some(binding) = bindings
            .iter_mut()
            .find(|binding| binding.event_name == EVENT_TRAFFIC_VERIFICATION_REQUESTED)
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
    hide_and_disable_profile(&db, "traffic_logic_triage").await?;
    hide_and_disable_profile(&db, "traffic_active_verifier").await?;

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

    sync_existing_profile_bindings(&db, TRAFFIC_VERIFICATION_AGENT_PROFILE_ID).await?;

    Ok(())
}
