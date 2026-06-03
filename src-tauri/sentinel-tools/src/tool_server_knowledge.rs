use crate::buildin_tools::{PluginAuthoringTool, SkillCreatorTool};
use crate::dynamic_tool::{
    DynamicToolBuilder, ToolCategory, ToolExecutionPolicy, ToolExposure, ToolRegistry, ToolSource,
};

pub(crate) async fn register_knowledge_extension_tools(registry: &ToolRegistry) {
    let plugin_authoring_def = DynamicToolBuilder::new(PluginAuthoringTool::NAME.to_string())
        .description(PluginAuthoringTool::DESCRIPTION.to_string())
        .input_schema(
            serde_json::to_value(schemars::schema_for!(
                crate::buildin_tools::plugin_authoring::PluginAuthoringArgs
            ))
            .unwrap_or_default(),
        )
        .source(ToolSource::Builtin)
        .category(ToolCategory::KnowledgeExtension)
        .tags(vec![
            "plugin".to_string(),
            "authoring".to_string(),
            "generate".to_string(),
            "draft".to_string(),
            "enable".to_string(),
        ])
        .search_hint("generate, validate, test, save draft, or enable a Sentinel plugin")
        .exposure(ToolExposure::Standard)
        .execution_policy(ToolExecutionPolicy {
            read_only: false,
            mutating: true,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: false,
        })
        .executor(|args| async move {
            use crate::buildin_tools::plugin_authoring::{
                PluginAuthoringArgs, PluginAuthoringTool,
            };
            use rig::tool::Tool;

            let tool_args: PluginAuthoringArgs = serde_json::from_value(args)
                .map_err(|error| format!("Invalid arguments: {error}"))?;

            let tool = PluginAuthoringTool;
            let result = tool
                .call(tool_args)
                .await
                .map_err(|error| format!("Plugin authoring failed: {error}"))?;

            serde_json::to_value(result)
                .map_err(|error| format!("Failed to serialize result: {error}"))
        })
        .build()
        .expect("Failed to build plugin_authoring tool");

    registry.register(plugin_authoring_def).await;

    let skill_creator_def = DynamicToolBuilder::new(SkillCreatorTool::NAME.to_string())
        .description(SkillCreatorTool::DESCRIPTION.to_string())
        .input_schema(
            serde_json::to_value(schemars::schema_for!(
                crate::buildin_tools::skill_creator::SkillCreatorArgs
            ))
            .unwrap_or_default(),
        )
        .source(ToolSource::Builtin)
        .category(ToolCategory::KnowledgeExtension)
        .tags(vec![
            "skill".to_string(),
            "authoring".to_string(),
            "create".to_string(),
            "validate".to_string(),
            "progressive-disclosure".to_string(),
        ])
        .search_hint("create, update, or validate a reusable Sentinel skill with SKILL.md metadata")
        .exposure(ToolExposure::Standard)
        .execution_policy(ToolExecutionPolicy {
            read_only: false,
            mutating: true,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: false,
        })
        .executor(|args| async move {
            use crate::buildin_tools::skill_creator::{SkillCreatorArgs, SkillCreatorTool};
            use rig::tool::Tool;

            let tool_args: SkillCreatorArgs = serde_json::from_value(args)
                .map_err(|error| format!("Invalid arguments: {error}"))?;

            let tool = SkillCreatorTool;
            let result = tool
                .call(tool_args)
                .await
                .map_err(|error| format!("Skill creator failed: {error}"))?;

            serde_json::to_value(result)
                .map_err(|error| format!("Failed to serialize result: {error}"))
        })
        .build()
        .expect("Failed to build skill_creator tool");

    registry.register(skill_creator_def).await;
}
