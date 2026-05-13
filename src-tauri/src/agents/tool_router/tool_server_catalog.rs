use sentinel_tools::{
    ToolCategory as SentinelToolCategory, ToolInfo, ToolSource as SentinelToolSource,
};

use super::types::{ToolCategory, ToolCost, ToolMetadata};

#[derive(Debug, Clone)]
struct BuiltinToolPolicy {
    cost_estimate: ToolCost,
}

fn builtin_tool_policy(tool_name: &str) -> BuiltinToolPolicy {
    match tool_name {
        "ask_user_question" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
        },
        "plugin_authoring" => BuiltinToolPolicy {
            cost_estimate: ToolCost::High,
        },
        "tool_search" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
        },
        "shell" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Medium,
        },
        "glob" | "grep" | "file_read" | "lsp" | "skills" | "tasks" | "memory"
        | "mission_scheduler" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
        },
        "file_edit" | "file_write" | "http_request" | "browser_shell" | "web_search"
        | "route_discovery" | "search_exploit" | "ocr" | "tenth_man_review" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Medium,
        },
        "spawn_agent" => BuiltinToolPolicy {
            cost_estimate: ToolCost::High,
        },
        "wait_agents" | "list_agents" | "close_agent" => BuiltinToolPolicy {
            cost_estimate: ToolCost::Low,
        },
        _ => BuiltinToolPolicy {
            cost_estimate: ToolCost::Medium,
        },
    }
}

pub(crate) fn parse_tool_category(raw: &str) -> ToolCategory {
    match raw.trim().to_lowercase().as_str() {
        "file_code" => ToolCategory::FileCode,
        "terminal" => ToolCategory::Terminal,
        "web_network" => ToolCategory::WebNetwork,
        "security_recon" => ToolCategory::SecurityRecon,
        "vulnerability_research" => ToolCategory::VulnerabilityResearch,
        "collaboration" => ToolCategory::Collaboration,
        "agent_orchestration" => ToolCategory::AgentOrchestration,
        "knowledge_extension" => ToolCategory::KnowledgeExtension,
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
        "traffic" => ToolCategory::Traffic,
        _ => ToolCategory::Other,
    }
}

fn convert_tool_category(category: SentinelToolCategory) -> ToolCategory {
    match category {
        SentinelToolCategory::FileCode => ToolCategory::FileCode,
        SentinelToolCategory::Terminal => ToolCategory::Terminal,
        SentinelToolCategory::WebNetwork => ToolCategory::WebNetwork,
        SentinelToolCategory::SecurityRecon => ToolCategory::SecurityRecon,
        SentinelToolCategory::VulnerabilityResearch => ToolCategory::VulnerabilityResearch,
        SentinelToolCategory::Collaboration => ToolCategory::Collaboration,
        SentinelToolCategory::AgentOrchestration => ToolCategory::AgentOrchestration,
        SentinelToolCategory::KnowledgeExtension => ToolCategory::KnowledgeExtension,
        SentinelToolCategory::Network => ToolCategory::Network,
        SentinelToolCategory::Security => ToolCategory::Security,
        SentinelToolCategory::Data => ToolCategory::Data,
        SentinelToolCategory::AI => ToolCategory::AI,
        SentinelToolCategory::System => ToolCategory::System,
        SentinelToolCategory::Mcp => ToolCategory::MCP,
        SentinelToolCategory::Plugin => ToolCategory::Plugin,
        SentinelToolCategory::Workflow => ToolCategory::Workflow,
        SentinelToolCategory::Browser => ToolCategory::Browser,
        SentinelToolCategory::Utility => ToolCategory::Utility,
        SentinelToolCategory::Recon => ToolCategory::Recon,
        SentinelToolCategory::Scanning => ToolCategory::Scanning,
        SentinelToolCategory::Exploitation => ToolCategory::Exploitation,
        SentinelToolCategory::Monitoring => ToolCategory::Monitoring,
        SentinelToolCategory::Traffic => ToolCategory::Traffic,
        SentinelToolCategory::Other => ToolCategory::Other,
    }
}

pub(crate) fn is_configurable_tool_name(tool_name: &str) -> bool {
    tool_name != sentinel_tools::buildin_tools::SopsTool::NAME
}

pub(crate) fn convert_tool_info_to_metadata(tool: ToolInfo) -> ToolMetadata {
    let category = if matches!(tool.source, SentinelToolSource::Plugin { .. }) {
        ToolCategory::Plugin
    } else {
        convert_tool_category(tool.category)
    };
    let policy = builtin_tool_policy(&tool.name);

    ToolMetadata {
        id: tool.name.clone(),
        name: tool.name,
        description: tool.description,
        category,
        tags: tool.tags,
        search_hint: tool.search_hint,
        cost_estimate: policy.cost_estimate,
        exposure: tool.exposure,
    }
}

pub(crate) fn build_builtin_tool_metadata(tools: Vec<ToolInfo>) -> Vec<ToolMetadata> {
    tools
        .into_iter()
        .filter(|tool| matches!(tool.source, SentinelToolSource::Builtin))
        .filter(|tool| is_configurable_tool_name(&tool.name))
        .map(convert_tool_info_to_metadata)
        .collect()
}

#[cfg(test)]
mod tests {
    use sentinel_tools::dynamic_tool::ToolExecutionPolicy;
    use sentinel_tools::ToolCategory as SentinelToolCategory;
    use sentinel_tools::ToolExposure;

    use super::*;

    fn tool_info(source: &str, category: &str) -> ToolInfo {
        let source = SentinelToolSource::from_wire_str(source).expect("valid test tool source");
        let category = match category {
            "file_code" => SentinelToolCategory::FileCode,
            "terminal" => SentinelToolCategory::Terminal,
            "web_network" => SentinelToolCategory::WebNetwork,
            "security_recon" => SentinelToolCategory::SecurityRecon,
            "vulnerability_research" => SentinelToolCategory::VulnerabilityResearch,
            "collaboration" => SentinelToolCategory::Collaboration,
            "agent_orchestration" => SentinelToolCategory::AgentOrchestration,
            "knowledge_extension" => SentinelToolCategory::KnowledgeExtension,
            "network" => SentinelToolCategory::Network,
            "security" => SentinelToolCategory::Security,
            "data" => SentinelToolCategory::Data,
            "ai" => SentinelToolCategory::AI,
            "system" => SentinelToolCategory::System,
            "mcp" => SentinelToolCategory::Mcp,
            "plugin" => SentinelToolCategory::Plugin,
            "workflow" => SentinelToolCategory::Workflow,
            "browser" => SentinelToolCategory::Browser,
            "utility" => SentinelToolCategory::Utility,
            "recon" => SentinelToolCategory::Recon,
            "scanning" => SentinelToolCategory::Scanning,
            "exploitation" => SentinelToolCategory::Exploitation,
            "monitoring" => SentinelToolCategory::Monitoring,
            "traffic" => SentinelToolCategory::Traffic,
            _ => SentinelToolCategory::Other,
        };
        ToolInfo {
            name: "plugin__active_probe".to_string(),
            description: "Agent plugin tool".to_string(),
            input_schema: serde_json::json!({ "type": "object" }),
            output_schema: None,
            source,
            category,
            tags: vec![],
            search_hint: None,
            exposure: ToolExposure::Deferred,
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
        let metadata = convert_tool_info_to_metadata(tool_info("builtin", "file_code"));

        assert_eq!(metadata.category, ToolCategory::FileCode);
    }

    #[test]
    fn skills_tool_is_configurable_but_sops_remains_internal() {
        assert!(is_configurable_tool_name(
            sentinel_tools::buildin_tools::SkillsTool::NAME
        ));
        assert!(!is_configurable_tool_name(
            sentinel_tools::buildin_tools::SopsTool::NAME
        ));
    }
}
