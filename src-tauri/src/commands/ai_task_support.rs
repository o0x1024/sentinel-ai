use std::sync::Arc;

use anyhow::{anyhow, Result};
use sentinel_llm::ChatMessage as LlmChatMessage;
use serde_json::Value;
use tauri::AppHandle;

use crate::agents::executor::{execute_agent, AgentExecuteParams};
use crate::commands::tool_commands;
use crate::generators::FewShotRepository;
use crate::services::ai_tasks::get_ai_task_profile;
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::{AiServiceManager, SystemAgentRuntime};
use crate::TrafficAnalysisState;

#[derive(Debug, Clone)]
pub struct ExternalProfileContext {
    pub prompt_patch: Option<String>,
    pub tool_policy: Option<SystemAgentToolPolicy>,
    pub policy_note: Option<String>,
    pub llm_provider_override: Option<String>,
    pub llm_model_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalProfileRun {
    pub run_id: String,
    pub profile_id: String,
}

pub async fn load_external_profile_context(
    runtime: &Arc<SystemAgentRuntime>,
    profile_id: &str,
    required_tools: &[&str],
) -> Result<ExternalProfileContext> {
    if let Some(task_profile) = get_ai_task_profile(profile_id) {
        let tool_policy = build_task_tool_policy(task_profile);
        if let Some(policy) = &tool_policy {
            policy.validate()?;
            for tool_id in required_tools {
                policy.ensure_tool_allowed(tool_id)?;
            }
        }

        let policy_note = tool_policy
            .as_ref()
            .and_then(SystemAgentToolPolicy::prompt_note);
        let prompt_patch = task_profile
            .prompt_patch
            .map(str::trim)
            .filter(|patch| !patch.is_empty())
            .map(str::to_string);

        return Ok(ExternalProfileContext {
            prompt_patch,
            tool_policy,
            policy_note,
            llm_provider_override: task_profile.llm_provider_override.map(str::to_string),
            llm_model_override: task_profile.llm_model_override.map(str::to_string),
        });
    }

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

    Ok(ExternalProfileContext {
        prompt_patch,
        tool_policy,
        policy_note,
        llm_provider_override: profile.llm_provider_override.clone(),
        llm_model_override: profile.llm_model_override.clone(),
    })
}

pub fn merge_external_profile_prompt(
    base_prompt: Option<String>,
    context: &ExternalProfileContext,
) -> Option<String> {
    let mut prompt = match (base_prompt, context.prompt_patch.clone()) {
        (Some(prompt), Some(patch)) => Some(format!(
            "{}\n\nAdditional profile guidance:\n{}",
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

pub async fn start_external_profile_run(
    runtime: &Arc<SystemAgentRuntime>,
    profile_id: &str,
    payload: Value,
) -> Option<ExternalProfileRun> {
    if get_ai_task_profile(profile_id).is_some() {
        return None;
    }

    runtime
        .start_external_run(profile_id, payload, Some("manual".to_string()))
        .await
        .ok()
        .map(|run| ExternalProfileRun {
            run_id: run.id,
            profile_id: run.profile_id,
        })
}

fn build_task_tool_policy(
    task_profile: &crate::services::ai_tasks::AiTaskProfile,
) -> Option<SystemAgentToolPolicy> {
    let policy = SystemAgentToolPolicy {
        required: task_profile
            .required_tools
            .iter()
            .map(|tool| (*tool).to_string())
            .collect(),
        optional: task_profile
            .optional_tools
            .iter()
            .map(|tool| (*tool).to_string())
            .collect(),
        forbidden: task_profile
            .forbidden_tools
            .iter()
            .map(|tool| (*tool).to_string())
            .collect(),
    };

    if policy.required.is_empty() && policy.optional.is_empty() && policy.forbidden.is_empty() {
        None
    } else {
        Some(policy)
    }
}

pub async fn build_virtual_tool_context(
    context: &ExternalProfileContext,
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
                let name = tool
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
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
            sections.push(format!(
                "Available unified tool list:\n{}",
                tools.join("\n")
            ));
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
                    example.vuln_type, example.context, example.quality_score
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
    context: Option<&ExternalProfileContext>,
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
    context: &ExternalProfileContext,
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
            cancellation_generation: None,
            model: config.model.clone(),
            system_prompt: system_prompt.unwrap_or_default(),
            task: user_input,
            active_browser_shell_direct_write_enabled: false,
            active_browser_shell_session_id: None,
            active_terminal_session_fingerprint: None,
            active_terminal_session_id: None,
            working_directory: None,
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
            referenced_traffic: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            context_engine_mode: Some(crate::agents::ContextEngineMode::CodexLike),
            recursion_depth: 0,
        };

        return execute_agent(app_handle, params).await.map_err(|error| {
            anyhow!(
                "External profile '{}' execution failed: {}",
                profile_id,
                error
            )
        });
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
    context: &ExternalProfileContext,
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
            cancellation_generation: None,
            model: config.model.clone(),
            system_prompt: system_prompt.unwrap_or_default(),
            task,
            active_browser_shell_direct_write_enabled: false,
            active_browser_shell_session_id: None,
            active_terminal_session_fingerprint: None,
            active_terminal_session_id: None,
            working_directory: None,
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
            referenced_traffic: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            context_engine_mode: Some(crate::agents::ContextEngineMode::CodexLike),
            recursion_depth: 0,
        };

        return execute_agent(app_handle, params).await.map_err(|error| {
            anyhow!(
                "External profile '{}' execution failed: {}",
                profile_id,
                error
            )
        });
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

pub async fn complete_external_profile_run_success(
    runtime: &Arc<SystemAgentRuntime>,
    tracked_run: &Option<ExternalProfileRun>,
    output: Value,
) {
    if let Some(run) = tracked_run {
        let _ = runtime
            .complete_external_run_success(&run.run_id, &run.profile_id, output)
            .await;
    }
}

pub async fn complete_external_profile_run_failure(
    runtime: &Arc<SystemAgentRuntime>,
    tracked_run: &Option<ExternalProfileRun>,
    error: impl ToString,
) {
    if let Some(run) = tracked_run {
        let _ = runtime
            .complete_external_run_failure(&run.run_id, &run.profile_id, error.to_string())
            .await;
    }
}
