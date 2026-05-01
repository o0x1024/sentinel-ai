use sentinel_tools::ToolInfo;

use super::types::{ToolCategory, ToolCost, ToolExposure, ToolMetadata};

#[derive(Debug, Clone)]
struct BuiltinToolPolicy {
    cost_estimate: ToolCost,
    always_available: bool,
}

fn builtin_tool_policy(tool_name: &str) -> BuiltinToolPolicy {
    match tool_name {
        "ask_user_question" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
            always_available: true,
        },
        "plugin_authoring" => BuiltinToolPolicy {
            cost_estimate: ToolCost::High,
            always_available: true,
        },
        "tool_search" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
            always_available: true,
        },
        "shell" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Medium,
            always_available: true,
        },
        "glob" | "grep" | "file_read" | "lsp" | "interactive_shell" | "skills" | "tasks"
        | "memory" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
            always_available: matches!(tool_name, "skills" | "tasks"),
        },
        "file_edit" | "file_write" | "http_request" | "browser" | "browser_shell"
        | "web_search" | "route_discovery" | "search_exploit" | "ocr" | "tenth_man_review" => {
            BuiltinToolPolicy {
                cost_estimate: ToolCost::Medium,
                always_available: false,
            }
        }
        "spawn_agent" => BuiltinToolPolicy {
            cost_estimate: ToolCost::High,
            always_available: false,
        },
        "wait_agents" | "list_agents" | "close_agent" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
            always_available: false,
        },
        _ => BuiltinToolPolicy {
            cost_estimate: ToolCost::Medium,
            always_available: false,
        },
    }
}

pub(crate) fn parse_tool_category(raw: &str) -> ToolCategory {
    match raw.trim().to_lowercase().as_str() {
        "network" => ToolCategory::Network,
        "security" => ToolCategory::Security,
        "data" => ToolCategory::Data,
        "ai" => ToolCategory::AI,
        "system" => ToolCategory::System,
        "mcp" => ToolCategory::MCP,
        "plugin" => ToolCategory::Plugin,
        "workflow" => ToolCategory::Workflow,
        "browser" => ToolCategory::Browser,
        "utility" => ToolCategory::Utility,
        "recon" => ToolCategory::Recon,
        "scanning" => ToolCategory::Scanning,
        "exploitation" => ToolCategory::Exploitation,
        "monitoring" => ToolCategory::Monitoring,
        _ => ToolCategory::Other,
    }
}

pub(crate) fn parse_tool_exposure(raw: &str) -> ToolExposure {
    match raw.trim().to_lowercase().as_str() {
        "always" => ToolExposure::Always,
        "core" => ToolExposure::Core,
        "deferred" => ToolExposure::Deferred,
        _ => ToolExposure::Standard,
    }
}

pub(crate) fn is_configurable_tool_name(tool_name: &str) -> bool {
    tool_name != sentinel_tools::buildin_tools::SopsTool::NAME
        && tool_name != sentinel_tools::buildin_tools::SkillsTool::NAME
}

pub(crate) fn convert_tool_info_to_metadata(tool: ToolInfo) -> ToolMetadata {
    let category = if tool.source.starts_with("plugin::") {
        ToolCategory::Plugin
    } else {
        parse_tool_category(&tool.category)
    };
    let exposure = parse_tool_exposure(&tool.exposure);
    let policy = builtin_tool_policy(&tool.name);

    ToolMetadata {
        id: tool.name.clone(),
        name: tool.name,
        description: tool.description,
        category,
        tags: tool.tags,
        search_hint: tool.search_hint,
        cost_estimate: policy.cost_estimate,
        always_available: policy.always_available || matches!(exposure, ToolExposure::Always),
        exposure,
    }
}

pub(crate) fn build_builtin_tool_metadata(tools: Vec<ToolInfo>) -> Vec<ToolMetadata> {
    tools
        .into_iter()
        .filter(|tool| tool.source == "builtin")
        .filter(|tool| is_configurable_tool_name(&tool.name))
        .map(convert_tool_info_to_metadata)
        .collect()
}

#[cfg(test)]
mod tests {
    use sentinel_tools::dynamic_tool::ToolExecutionPolicy;

    use super::*;

    fn tool_info(source: &str, category: &str) -> ToolInfo {
        ToolInfo {
            name: "plugin__active_probe".to_string(),
            description: "Agent plugin tool".to_string(),
            input_schema: serde_json::json!({ "type": "object" }),
            output_schema: None,
            source: source.to_string(),
            category: category.to_string(),
            tags: vec![],
            search_hint: None,
            exposure: "deferred".to_string(),
            execution_policy: ToolExecutionPolicy::default(),
            enabled: true,
        }
    }

    #[test]
    fn plugin_tool_metadata_uses_plugin_category_even_when_plugin_has_business_category() {
        let metadata = convert_tool_info_to_metadata(tool_info("plugin::active_probe", "recon"));

        assert_eq!(metadata.category, ToolCategory::Plugin);
    }

    #[test]
    fn builtin_tool_metadata_preserves_registered_category() {
        let metadata = convert_tool_info_to_metadata(tool_info("builtin", "network"));

        assert_eq!(metadata.category, ToolCategory::Network);
    }
}
