use crate::services::system_agents::language::is_chinese_ui_language;
use crate::services::system_agents::pipeline::{
    TRAFFIC_CONTEXT_AGENT_PROFILE_ID, TRAFFIC_DECISION_AGENT_PROFILE_ID,
    TRAFFIC_HYPOTHESIS_AGENT_PROFILE_ID, TRAFFIC_VERIFICATION_AGENT_PROFILE_ID,
};

pub fn localize_system_agent_profile_text(
    profile_id: &str,
    name: &str,
    description: &str,
    language: &str,
) -> (String, String) {
    if !is_chinese_ui_language(language) {
        return (name.to_string(), description.to_string());
    }

    match profile_id {
        TRAFFIC_CONTEXT_AGENT_PROFILE_ID => (
            "流量上下文 Agent".to_string(),
            "对原始流量事件构建统一安全上下文的后台 Agent。".to_string(),
        ),
        TRAFFIC_HYPOTHESIS_AGENT_PROFILE_ID => (
            "流量假设 Agent".to_string(),
            "基于统一安全上下文生成漏洞假设与初始验证计划。".to_string(),
        ),
        TRAFFIC_VERIFICATION_AGENT_PROFILE_ID => (
            "流量验证 Agent".to_string(),
            "对已升格的漏洞假设执行安全重放验证。".to_string(),
        ),
        TRAFFIC_DECISION_AGENT_PROFILE_ID => (
            "流量决策 Agent".to_string(),
            "决定是否将漏洞假设升格为 finding，并驱动后续验证。".to_string(),
        ),
        "traffic_logic_triage" => (
            "流量逻辑分诊 Agent".to_string(),
            "对聚类流量执行逻辑漏洞分诊的后台 Agent。".to_string(),
        ),
        "traffic_active_verifier" => (
            "流量主动验证 Agent".to_string(),
            "对已识别的漏洞线索执行主动验证的后台 Agent。".to_string(),
        ),
        "traffic_idor_triage" => (
            "IDOR 分诊 Agent".to_string(),
            "针对 IDOR/BOLA 风险执行被动分诊的后台 Agent。".to_string(),
        ),
        _ => (name.to_string(), description.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localizes_builtin_profile_name_for_chinese_ui() {
        let (name, description) = localize_system_agent_profile_text(
            TRAFFIC_CONTEXT_AGENT_PROFILE_ID,
            "Traffic Context Agent",
            "English description",
            "zh",
        );

        assert_eq!(name, "流量上下文 Agent");
        assert!(description.contains("统一安全上下文"));
    }

    #[test]
    fn preserves_original_text_for_non_chinese_ui() {
        let (name, description) = localize_system_agent_profile_text(
            TRAFFIC_CONTEXT_AGENT_PROFILE_ID,
            "Traffic Context Agent",
            "English description",
            "en",
        );

        assert_eq!(name, "Traffic Context Agent");
        assert_eq!(description, "English description");
    }
}
