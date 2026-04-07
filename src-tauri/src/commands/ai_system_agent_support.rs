use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::Value;
use sentinel_llm::ChatMessage as LlmChatMessage;
use tauri::AppHandle;

use crate::agents::executor::{execute_agent, AgentExecuteParams};
use crate::commands::tool_commands;
use crate::generators::FewShotRepository;
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::{AiServiceManager, SystemAgentRuntime};
use crate::TrafficAnalysisState;

#[derive(Debug, Clone)]
pub struct ExternalSystemAgentContext {
    pub prompt_patch: Option<String>,
    pub tool_policy: Option<SystemAgentToolPolicy>,
    pub policy_note: Option<String>,
    pub llm_provider_override: Option<String>,
    pub llm_model_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalTrackedRun {
    pub run_id: String,
    pub profile_id: String,
}

pub async fn load_external_system_agent_context(
    runtime: &Arc<SystemAgentRuntime>,
    profile_id: &str,
    required_tools: &[&str],
) -> Result<ExternalSystemAgentContext> {
    let profile = runtime
        .get_profile(profile_id)
        .await?
        .ok_or_else(|| anyhow!("System agent profile not found: {}", profile_id))?;

    let tool_policy = Some(SystemAgentToolPolicy::from_profile(&profile));
    if let Some(policy) = &tool_policy {
        policy.validate()?;
        for tool_id in required_tools {
            policy.ensure_tool_allowed(tool_id)?;
        }
    }

    let policy_note = tool_policy
        .as_ref()
        .and_then(SystemAgentToolPolicy::prompt_note);
    let prompt_patch = profile
        .prompt_patch
        .clone()
        .filter(|patch| !patch.trim().is_empty());

    Ok(ExternalSystemAgentContext {
        prompt_patch,
        tool_policy,
        policy_note,
        llm_provider_override: profile.llm_provider_override.clone(),
        llm_model_override: profile.llm_model_override.clone(),
    })
}

pub fn merge_external_system_prompt(
    base_prompt: Option<String>,
    context: &ExternalSystemAgentContext,
) -> Option<String> {
    let mut prompt = match (base_prompt, context.prompt_patch.clone()) {
        (Some(prompt), Some(patch)) => Some(format!(
            "{}\n\nAdditional system-agent guidance:\n{}",
            prompt, patch
        )),
        (Some(prompt), None) => Some(prompt),
        (None, Some(patch)) => Some(patch),
        (None, None) => None,
    };

    if let (Some(prompt), Some(policy_note)) = (&mut prompt, &context.policy_note) {
        prompt.push_str("\n\n");
        prompt.push_str(policy_note);
    }

    prompt
}

pub async fn start_external_tracked_run(
    runtime: &Arc<SystemAgentRuntime>,
    profile_id: &str,
    payload: Value,
) -> Option<ExternalTrackedRun> {
    runtime
        .start_external_run(profile_id, payload, Some("manual".to_string()))
        .await
        .ok()
        .map(|run| ExternalTrackedRun {
            run_id: run.id,
            profile_id: run.profile_id,
        })
}

pub async fn build_virtual_tool_context(
    context: &ExternalSystemAgentContext,
    traffic_state: Option<&TrafficAnalysisState>,
) -> Result<Vec<String>> {
    let mut sections = Vec::new();
    let Some(policy) = &context.tool_policy else {
        return Ok(sections);
    };

    if policy.is_tool_allowed("tool_catalog_reader") {
        let tools = tool_commands::list_unified_tools()
            .await
            .unwrap_or_default()
            .into_iter()
            .take(60)
            .map(|tool| {
                let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                let category = tool
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("misc");
                let description = tool
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("- {} ({}): {}", name, category, description)
            })
            .collect::<Vec<_>>();
        if !tools.is_empty() {
            sections.push(format!("Available unified tool list:\n{}", tools.join("\n")));
        }
    }

    if policy.is_tool_allowed("workflow_catalog_reader") {
        if let Some(traffic_state) = traffic_state {
            let nodes = tool_commands::build_node_catalog(traffic_state)
                .await
                .unwrap_or_default()
                .into_iter()
                .take(80)
                .map(|item| format!("- {} [{}]: {}", item.node_type, item.category, item.label))
                .collect::<Vec<_>>();
            if !nodes.is_empty() {
                sections.push(format!(
                    "Available workflow node type list:\n{}",
                    nodes.join("\n")
                ));
            }
        }
    }

    if policy.is_tool_allowed("plugin_prompt_reader") {
        sections.push(
            "Plugin generation baseline:\n- Generate executable TypeScript plugin code.\n- Keep detection logic generic across targets.\n- Return findings from scan_transaction.\n- Include defensive error handling and stable output structure."
                .to_string(),
        );
    }

    if policy.is_tool_allowed("plugin_example_reader") {
        let repo = FewShotRepository::new();
        let example_lines = ["idor", "sqli", "xss"]
            .into_iter()
            .flat_map(|vuln_type| repo.get_examples(vuln_type))
            .take(3)
            .map(|example| {
                format!(
                    "- {} example: context='{}', quality={:.1}",
                    example.vuln_type,
                    example.context,
                    example.quality_score
                )
            })
            .collect::<Vec<_>>();
        if !example_lines.is_empty() {
            sections.push(format!(
                "Built-in plugin generation examples:\n{}",
                example_lines.join("\n")
            ));
        }
    }

    if policy.is_tool_allowed("plugin_validator") {
        sections.push(
            "Plugin validation requirements:\n- Preserve the expected plugin interface.\n- Return stable findings from scan_transaction.\n- Avoid syntax/type/runtime errors.\n- Keep the implementation executable without extra environment assumptions."
                .to_string(),
        );
    }

    if policy.is_tool_allowed("plugin_test_result_reader") {
        sections.push(
            "Plugin execution feedback source:\n- Treat the provided error_message and error_details as the authoritative execution-test feedback.\n- Prioritize fixing the observed runtime/validation error before broad refactors."
                .to_string(),
        );
    }

    Ok(sections)
}

pub async fn resolve_external_llm_config(
    ai_manager: &Arc<AiServiceManager>,
    context: Option<&ExternalSystemAgentContext>,
    model_override: Option<&str>,
) -> Result<(sentinel_llm::LlmConfig, String)> {
    let llm_config = ai_manager
        .resolve_generation_llm_config(
            context.and_then(|ctx| ctx.llm_provider_override.as_deref()),
            model_override.or_else(|| context.and_then(|ctx| ctx.llm_model_override.as_deref())),
        )
        .await?;

    let model = llm_config.model.clone();
    Ok((llm_config, model))
}

pub async fn run_external_text_task(
    app_handle: &AppHandle,
    ai_manager: &Arc<AiServiceManager>,
    profile_id: &str,
    run_id: &str,
    context: &ExternalSystemAgentContext,
    system_prompt: Option<String>,
    user_input: String,
) -> Result<String> {
    if let Some(tool_config) = context
        .tool_policy
        .as_ref()
        .and_then(SystemAgentToolPolicy::build_runtime_tool_config)
    {
        let config = ai_manager
            .resolve_generation_llm_config(
                context.llm_provider_override.as_deref(),
                context.llm_model_override.as_deref(),
            )
            .await?;
        let params = AgentExecuteParams {
            execution_id: run_id.to_string(),
            model: config.model.clone(),
            system_prompt: system_prompt.unwrap_or_default(),
            task: user_input,
            rig_provider: config
                .rig_provider
                .clone()
                .unwrap_or_else(|| config.provider.clone()),
            api_key: config.api_key.clone(),
            api_base: config.base_url.clone(),
            max_iterations: 6,
            timeout_secs: 120,
            tool_config: Some(tool_config),
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            recursion_depth: 0,
        };

        return execute_agent(app_handle, params)
            .await
            .map_err(|error| anyhow!("External system agent '{}' execution failed: {}", profile_id, error));
    }

    let (llm_config, _model) = resolve_external_llm_config(ai_manager, Some(context), None).await?;
    let client = sentinel_llm::LlmClient::new(llm_config);
    client
        .completion(system_prompt.as_deref(), &user_input)
        .await
        .map_err(Into::into)
}

pub async fn run_external_chat_task(
    app_handle: &AppHandle,
    ai_manager: &Arc<AiServiceManager>,
    profile_id: &str,
    run_id: &str,
    context: &ExternalSystemAgentContext,
    system_prompt: Option<String>,
    history: &[LlmChatMessage],
    user_input: String,
) -> Result<String> {
    if let Some(tool_config) = context
        .tool_policy
        .as_ref()
        .and_then(SystemAgentToolPolicy::build_runtime_tool_config)
    {
        let config = ai_manager
            .resolve_generation_llm_config(
                context.llm_provider_override.as_deref(),
                context.llm_model_override.as_deref(),
            )
            .await?;
        let task = if history.is_empty() {
            user_input
        } else {
            format!(
                "Conversation history:\n{}\n\nLatest user request:\n{}",
                render_chat_history(history),
                user_input
            )
        };

        let params = AgentExecuteParams {
            execution_id: run_id.to_string(),
            model: config.model.clone(),
            system_prompt: system_prompt.unwrap_or_default(),
            task,
            rig_provider: config
                .rig_provider
                .clone()
                .unwrap_or_else(|| config.provider.clone()),
            api_key: config.api_key.clone(),
            api_base: config.base_url.clone(),
            max_iterations: 6,
            timeout_secs: 120,
            tool_config: Some(tool_config),
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            recursion_depth: 0,
        };

        return execute_agent(app_handle, params)
            .await
            .map_err(|error| anyhow!("External system agent '{}' execution failed: {}", profile_id, error));
    }

    let (llm_config, _model) = resolve_external_llm_config(ai_manager, Some(context), None).await?;
    let client = sentinel_llm::LlmClient::new(llm_config);
    client
        .chat(system_prompt.as_deref(), &user_input, history, None)
        .await
        .map_err(Into::into)
}

fn render_chat_history(history: &[LlmChatMessage]) -> String {
    history
        .iter()
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn complete_external_tracked_run_success(
    runtime: &Arc<SystemAgentRuntime>,
    tracked_run: &Option<ExternalTrackedRun>,
    output: Value,
) {
    if let Some(run) = tracked_run {
        let _ = runtime
            .complete_external_run_success(&run.run_id, &run.profile_id, output)
            .await;
    }
}

pub async fn complete_external_tracked_run_failure(
    runtime: &Arc<SystemAgentRuntime>,
    tracked_run: &Option<ExternalTrackedRun>,
    error: impl ToString,
) {
    if let Some(run) = tracked_run {
        let _ = runtime
            .complete_external_run_failure(&run.run_id, &run.profile_id, error.to_string())
            .await;
    }
}
