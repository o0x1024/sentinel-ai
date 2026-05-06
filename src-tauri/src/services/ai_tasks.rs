#[derive(Debug, Clone)]
pub struct AiTaskProfile {
    pub id: &'static str,
    pub prompt_patch: Option<&'static str>,
    pub llm_provider_override: Option<&'static str>,
    pub llm_model_override: Option<&'static str>,
    pub required_tools: &'static [&'static str],
    pub optional_tools: &'static [&'static str],
    pub forbidden_tools: &'static [&'static str],
}

const WORKFLOW_DESIGNER_TASK: AiTaskProfile = AiTaskProfile {
    id: "workflow_designer_agent",
    prompt_patch: Some("生成结果必须严格满足 WorkflowGraph 结构。"),
    llm_provider_override: None,
    llm_model_override: None,
    required_tools: &[],
    optional_tools: &["workflow_catalog_reader", "tool_catalog_reader"],
    forbidden_tools: &[],
};

const TRAFFIC_PLUGIN_GENERATOR_TASK: AiTaskProfile = AiTaskProfile {
    id: "traffic_plugin_generator_agent",
    prompt_patch: Some("优先生成可维护、可验证、可审阅的插件实现。"),
    llm_provider_override: None,
    llm_model_override: None,
    required_tools: &[],
    optional_tools: &["plugin_prompt_reader", "plugin_example_reader"],
    forbidden_tools: &[],
};

const PLUGIN_FIX_TASK: AiTaskProfile = AiTaskProfile {
    id: "plugin_fix_agent",
    prompt_patch: Some("修复时优先保持原始接口和输出契约不变。"),
    llm_provider_override: None,
    llm_model_override: None,
    required_tools: &[],
    optional_tools: &["plugin_validator", "plugin_test_result_reader"],
    forbidden_tools: &[],
};

const PLUGIN_EDITOR_ASSISTANT_TASK: AiTaskProfile = AiTaskProfile {
    id: "plugin_editor_assistant_agent",
    prompt_patch: Some("编辑插件代码时优先最小修改，避免偏离当前插件目标和接口契约。"),
    llm_provider_override: None,
    llm_model_override: None,
    required_tools: &[],
    optional_tools: &["plugin_prompt_reader", "plugin_test_result_reader"],
    forbidden_tools: &[],
};

const MANUAL_TRAFFIC_AUDIT_TASK: AiTaskProfile = AiTaskProfile {
    id: "manual_traffic_audit_agent",
    prompt_patch: Some("优先关注鉴权、对象边界和状态流转。"),
    llm_provider_override: None,
    llm_model_override: None,
    required_tools: &[],
    optional_tools: &["traffic_history_reader", "repeater_launcher"],
    forbidden_tools: &[],
};

pub fn get_ai_task_profile(task_id: &str) -> Option<&'static AiTaskProfile> {
    match task_id {
        "workflow_designer_agent" => Some(&WORKFLOW_DESIGNER_TASK),
        "traffic_plugin_generator_agent" => Some(&TRAFFIC_PLUGIN_GENERATOR_TASK),
        "plugin_fix_agent" => Some(&PLUGIN_FIX_TASK),
        "plugin_editor_assistant_agent" => Some(&PLUGIN_EDITOR_ASSISTANT_TASK),
        "manual_traffic_audit_agent" => Some(&MANUAL_TRAFFIC_AUDIT_TASK),
        _ => None,
    }
}
