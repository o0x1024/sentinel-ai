use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

use sentinel_db::{Database, DatabaseService, SystemAgentBindingRecord, SystemAgentProfileRecord};

fn default_profiles() -> Vec<(SystemAgentProfileRecord, Vec<SystemAgentBindingRecord>)> {
    let now = Utc::now();

    let manual = SystemAgentProfileRecord {
        id: "manual_traffic_audit_agent".to_string(),
        name: "Manual Traffic Audit Agent".to_string(),
        description: "用户手动触发的流量审计 Agent".to_string(),
        mode: "active".to_string(),
        capability: "reviewer".to_string(),
        enabled: true,
        trigger_mode: "manual".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:manual_traffic_audit_agent".to_string()),
        prompt_patch: Some("优先关注鉴权、对象边界和状态流转。".to_string()),
        input_schema_json: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "requests": {"type": "array"}
            }
        })
        .to_string(),
        output_schema_json: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "riskAreas": {"type": "array"},
                "interestingParameters": {"type": "array"},
                "suggestedTests": {"type": "array"}
            }
        })
        .to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec![
            "traffic_history_reader".to_string(),
            "repeater_launcher".to_string(),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: "[]".to_string(),
        budget_json: json!({"maxRunsPerHour": 20, "maxTokensPerRun": 1200}).to_string(),
        safety_policy_json: json!({"allowActiveReplay": false, "shadowMode": false}).to_string(),
        cooldown_secs: 0,
        max_concurrency: 1,
        risk_level: "medium".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    let workflow_designer = SystemAgentProfileRecord {
        id: "workflow_designer_agent".to_string(),
        name: "Workflow Designer Agent".to_string(),
        description: "负责自然语言到工作流图的主动生成。".to_string(),
        mode: "active".to_string(),
        capability: "designer".to_string(),
        enabled: true,
        trigger_mode: "manual".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:workflow_designer_agent".to_string()),
        prompt_patch: Some("生成结果必须严格满足 WorkflowGraph 结构。".to_string()),
        input_schema_json: json!({"type":"object"}).to_string(),
        output_schema_json: json!({"type":"object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec![
            "workflow_catalog_reader".to_string(),
            "tool_catalog_reader".to_string(),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: "[]".to_string(),
        budget_json: json!({"maxRunsPerHour": 30, "maxTokensPerRun": 2400}).to_string(),
        safety_policy_json: json!({"allowActiveReplay": false, "shadowMode": false}).to_string(),
        cooldown_secs: 0,
        max_concurrency: 2,
        risk_level: "medium".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    let traffic_plugin_generator = SystemAgentProfileRecord {
        id: "traffic_plugin_generator_agent".to_string(),
        name: "Traffic Plugin Generator Agent".to_string(),
        description: "负责生成流量分析插件代码。".to_string(),
        mode: "active".to_string(),
        capability: "generator".to_string(),
        enabled: true,
        trigger_mode: "manual".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:traffic_plugin_generator_agent".to_string()),
        prompt_patch: Some("优先生成可维护、可验证、可审阅的插件实现。".to_string()),
        input_schema_json: json!({"type":"object"}).to_string(),
        output_schema_json: json!({"type":"object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec![
            "plugin_prompt_reader".to_string(),
            "plugin_example_reader".to_string(),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: "[]".to_string(),
        budget_json: json!({"maxRunsPerHour": 40, "maxTokensPerRun": 3600}).to_string(),
        safety_policy_json: json!({"allowActiveReplay": false, "shadowMode": false}).to_string(),
        cooldown_secs: 0,
        max_concurrency: 2,
        risk_level: "medium".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

    let plugin_fix = SystemAgentProfileRecord {
        id: "plugin_fix_agent".to_string(),
        name: "Plugin Fix Agent".to_string(),
        description: "负责根据测试/校验结果修复插件。".to_string(),
        mode: "active".to_string(),
        capability: "generator".to_string(),
        enabled: true,
        trigger_mode: "manual".to_string(),
        llm_provider_override: None,
        llm_model_override: None,
        base_prompt_id: Some("system:plugin_fix_agent".to_string()),
        prompt_patch: Some("修复时优先保持原始接口和输出契约不变。".to_string()),
        input_schema_json: json!({"type":"object"}).to_string(),
        output_schema_json: json!({"type":"object"}).to_string(),
        required_tools_json: "[]".to_string(),
        optional_tools_json: serde_json::to_string(&vec![
            "plugin_validator".to_string(),
            "plugin_test_result_reader".to_string(),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        forbidden_tools_json: "[]".to_string(),
        trigger_events_json: "[]".to_string(),
        budget_json: json!({"maxRunsPerHour": 40, "maxTokensPerRun": 3600}).to_string(),
        safety_policy_json: json!({"allowActiveReplay": false, "shadowMode": false}).to_string(),
        cooldown_secs: 0,
        max_concurrency: 2,
        risk_level: "medium".to_string(),
        visibility: "system".to_string(),
        created_at: now,
        updated_at: now,
    };

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
        (manual, vec![]),
        (workflow_designer, vec![]),
        (traffic_plugin_generator, vec![]),
        (plugin_fix, vec![]),
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

pub async fn ensure_default_system_agent_profiles(db: Arc<DatabaseService>) -> Result<()> {
    if let Some(mut legacy_idor) = db.get_system_agent_profile("traffic_idor_triage").await? {
        legacy_idor.enabled = false;
        legacy_idor.visibility = "hidden".to_string();
        legacy_idor.updated_at = Utc::now();
        db.save_system_agent_profile(&legacy_idor, &[]).await?;
    }

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

    Ok(())
}
