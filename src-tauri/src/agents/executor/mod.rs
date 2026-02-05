//! Agent executor - entrypoint and orchestration.

use anyhow::Result;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use sentinel_db::Database;
use sentinel_llm::LlmConfig;
use sentinel_tools::{get_tool_server, mcp_adapter};

use crate::agents::ContextPolicy;
use crate::agents::DocumentAttachmentInfo;
use crate::agents::tenth_man::TenthManConfig;
use crate::agents::tool_router::ToolConfig;
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;

use self::run_simple::execute_agent_simple;
use self::run_with_tools::execute_agent_with_tools;

pub mod message_store;
pub mod run_simple;
pub mod run_with_tools;
pub mod tool_exec;
pub mod types;
pub mod utils;

pub use tool_exec::{execute_builtin_tool, execute_mcp_tool, execute_plugin_tool, execute_workflow_tool};
pub use types::ToolCallRecord;

/// Agent execution parameters.
#[derive(Debug, Clone)]
pub struct AgentExecuteParams {
    pub execution_id: String,
    pub model: String,
    pub system_prompt: String,
    pub task: String,
    pub rig_provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub max_iterations: usize,
    pub timeout_secs: u64,
    pub tool_config: Option<ToolConfig>,
    pub enable_tenth_man_rule: bool,
    pub tenth_man_config: Option<TenthManConfig>,
    pub document_attachments: Option<Vec<DocumentAttachmentInfo>>,
    pub image_attachments: Option<serde_json::Value>,
    pub persist_messages: bool,
    pub subagent_run_id: Option<String>,
    pub context_policy: Option<ContextPolicy>,
    pub recursion_depth: usize,
}

/// Execute agent task.
pub async fn execute_agent(app_handle: &AppHandle, params: AgentExecuteParams) -> Result<String> {
    let rig_provider = params.rig_provider.to_lowercase();
    let execution_id = params.execution_id.clone();

    tracing::info!(
        "Executing agent - rig_provider: {}, model: {}, execution_id: {}, tools_enabled: {}, recursion_depth: {}",
        rig_provider,
        params.model,
        params.execution_id,
        params
            .tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false),
        params.recursion_depth
    );

    let parent_context = crate::agents::subagent_executor::SubagentParentContext {
        rig_provider: params.rig_provider.clone(),
        model: params.model.clone(),
        api_key: params.api_key.clone(),
        api_base: params.api_base.clone(),
        system_prompt: params.system_prompt.clone(),
        tool_config: params.tool_config.clone().unwrap_or_default(),
        max_iterations: params.max_iterations,
        timeout_secs: params.timeout_secs,
        task_context: params.task.clone(),
        recursion_depth: params.recursion_depth,
    };
    crate::agents::subagent_executor::set_parent_context(execution_id.clone(), parent_context).await;

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        if let Ok(client) = db.get_db() {
            sentinel_memory::get_global_memory().set_database_client(client).await;
        }

        if let Ok(api_key) = db.get_config("ai", "tavily_api_key").await {
            sentinel_tools::tool_server::set_tavily_api_key(api_key).await;
        }
    }

    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    use sentinel_tools::buildin_tools::todos::set_todos_app_handle;
    set_todos_app_handle(app_handle.clone()).await;

    use crate::agents::tenth_man_executor;

    let mut tenth_man_llm_config = LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider)
        .with_conversation_id(&params.execution_id);

    if let Some(ref api_key) = params.api_key {
        tenth_man_llm_config = tenth_man_llm_config.with_api_key(api_key);
    }
    if let Some(ref api_base) = params.api_base {
        tenth_man_llm_config = tenth_man_llm_config.with_base_url(api_base);
    }

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        tenth_man_llm_config = apply_generation_settings_from_db(db.as_ref(), tenth_man_llm_config).await;
    }

    tenth_man_executor::set_tenth_man_config(params.execution_id.clone(), tenth_man_llm_config).await;
    tenth_man_executor::set_task_context(params.execution_id.clone(), params.task.clone()).await;

    tracing::info!(
        "Tenth Man initialized for execution_id: {} (rule_enabled: {})",
        params.execution_id,
        params.enable_tenth_man_rule
    );

    let tool_config = params.tool_config.clone().unwrap_or_default();

    let result = if tool_config.enabled {
        tracing::info!("Refreshing MCP tools before execution...");
        mcp_adapter::refresh_mcp_tools(&tool_server).await;

        if tool_config.enabled && !tool_config.disabled_tools.contains(&"web_explorer".to_string())
        {
            if let Some(_mcp_service) =
                app_handle.try_state::<std::sync::Arc<crate::services::mcp::McpService>>()
            {
                use crate::engines::web_explorer::WebExplorerTool;
                use sentinel_tools::dynamic_tool::{DynamicToolBuilder, ToolSource};
                use rig::tool::Tool;

                let rig_provider = params.rig_provider.to_lowercase();
                let mut llm_config = sentinel_llm::LlmConfig::new(&rig_provider, &params.model)
                    .with_timeout(params.timeout_secs)
                    .with_rig_provider(&rig_provider);

                if let Some(ref api_key) = params.api_key {
                    llm_config = llm_config.with_api_key(api_key);
                }
                if let Some(ref api_base) = params.api_base {
                    llm_config = llm_config.with_base_url(api_base);
                }

                if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
                    llm_config = apply_generation_settings_from_db(db.as_ref(), llm_config).await;
                }

                let we_tool = WebExplorerTool::new(llm_config)
                    .with_app_handle(app_handle.clone())
                    .with_execution_id(params.execution_id.clone());

                let def = we_tool.definition(String::new()).await;

                let tool_def = DynamicToolBuilder::new(def.name)
                    .description(def.description)
                    .input_schema(def.parameters)
                    .source(ToolSource::Builtin)
                    .executor(move |args| {
                        let tool = we_tool.clone();
                        async move {
                            let tool_args: crate::engines::web_explorer::tool::WebExplorerArgs =
                                serde_json::from_value(args).map_err(|e| e.to_string())?;

                            let result = tool.call(tool_args).await.map_err(|e| e.to_string())?;

                            Ok(serde_json::Value::String(result))
                        }
                    })
                    .build();

                if let Ok(tool_def) = tool_def {
                    tool_server.register_tool(tool_def).await;
                    tracing::info!("Registered WebExplorerTool");
                } else if let Err(e) = tool_def {
                    tracing::warn!("Failed to build WebExplorerTool definition: {}", e);
                }
            } else {
                tracing::warn!("McpService not found, skipping WebExplorerTool registration");
            }
        }

        let registered_tools = tool_server.list_tools().await;
        tracing::info!(
            "ToolServer has {} registered tools: {:?}",
            registered_tools.len(),
            registered_tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        execute_agent_with_tools(app_handle, params, &tool_server).await
    } else {
        execute_agent_simple(app_handle, params).await
    };

    crate::agents::subagent_executor::clear_parent_context(&execution_id).await;

    result
}
