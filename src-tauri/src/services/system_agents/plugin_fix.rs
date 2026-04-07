use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::commands::ai_system_agent_support::{
    build_virtual_tool_context, complete_external_tracked_run_failure,
    complete_external_tracked_run_success, load_external_system_agent_context,
    merge_external_system_prompt, resolve_external_llm_config, run_external_text_task,
    start_external_tracked_run,
};
use crate::generators::{
    ExecutionTestResult, PluginValidator, PromptTemplateBuilder, ValidationResult,
};
use crate::services::{AiServiceManager, SystemAgentRuntime};

const PLUGIN_FIX_PROFILE_ID: &str = "plugin_fix_agent";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFixAgentRequest {
    pub original_code: String,
    pub error_message: String,
    pub error_details: Option<String>,
    pub vuln_type: Option<String>,
    pub attempt: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFixAgentResult {
    pub run_id: String,
    pub profile_id: String,
    pub fixed_code: String,
    pub model: String,
    pub validation: ValidationResult,
    pub execution_test: ExecutionTestResult,
    pub prompt_patch_applied: bool,
}

pub async fn run_plugin_fix_agent(
    runtime: &Arc<SystemAgentRuntime>,
    ai_manager: &Arc<AiServiceManager>,
    request: PluginFixAgentRequest,
) -> Result<PluginFixAgentResult> {
    let payload = serde_json::to_value(&request)?;
    let tracked_run = start_external_tracked_run(runtime, PLUGIN_FIX_PROFILE_ID, payload).await;
    let run_id = tracked_run
        .as_ref()
        .map(|run| run.run_id.clone())
        .unwrap_or_else(|| format!("plugin-fix-{}", uuid::Uuid::new_v4()));

    match execute_plugin_fix(ai_manager, runtime, &request, &run_id).await {
        Ok(result) => {
            let output = serde_json::to_value(&result)?;
            complete_external_tracked_run_success(runtime, &tracked_run, output).await;
            Ok(result)
        }
        Err(error) => {
            complete_external_tracked_run_failure(runtime, &tracked_run, error.to_string()).await;
            Err(error)
        }
    }
}

async fn execute_plugin_fix(
    ai_manager: &Arc<AiServiceManager>,
    runtime: &Arc<SystemAgentRuntime>,
    request: &PluginFixAgentRequest,
    run_id: &str,
) -> Result<PluginFixAgentResult> {
    let agent_context = load_external_system_agent_context(
        runtime,
        PLUGIN_FIX_PROFILE_ID,
        &["plugin_validator", "plugin_test_result_reader"],
    )
    .await?;

    let vuln_type = request.vuln_type.as_deref().unwrap_or("generic");
    let attempt = request.attempt.unwrap_or(1);
    let prompt_builder = PromptTemplateBuilder::new();
    let base_prompt = prompt_builder.build_fix_prompt(
        &request.original_code,
        &request.error_message,
        request.error_details.as_deref(),
        vuln_type,
        attempt,
    )?;

    let virtual_tool_sections = build_virtual_tool_context(&agent_context, None).await?;
    let prompt = merge_external_system_prompt(
        Some(if virtual_tool_sections.is_empty() {
            base_prompt
        } else {
            format!(
                "{}\n\nVirtual tool context:\n{}",
                base_prompt,
                virtual_tool_sections.join("\n\n")
            )
        }),
        &agent_context,
    )
    .unwrap_or_default();

    let response = run_external_text_task(
        &runtime.app_handle(),
        ai_manager,
        PLUGIN_FIX_PROFILE_ID,
        run_id,
        &agent_context,
        None,
        prompt,
    )
    .await
    .context("Failed to call LLM for plugin fix")?;
    let (_, model) = resolve_external_llm_config(ai_manager, Some(&agent_context), None).await?;
    let fixed_code = extract_and_clean_code(&response);

    if fixed_code.trim().is_empty() {
        return Err(anyhow!("Plugin fix agent returned empty code"));
    }

    let validator = PluginValidator::new();
    let validation = validator.validate(&fixed_code).await?;
    let execution_test = validator.test_plugin_execution(&fixed_code).await;

    Ok(PluginFixAgentResult {
        run_id: run_id.to_string(),
        profile_id: PLUGIN_FIX_PROFILE_ID.to_string(),
        fixed_code,
        model,
        validation,
        execution_test,
        prompt_patch_applied: agent_context.prompt_patch.is_some(),
    })
}

fn extract_and_clean_code(response: &str) -> String {
    extract_from_markdown(response)
        .or_else(|| extract_from_json(response))
        .unwrap_or_else(|| response.trim().to_string())
}

fn extract_from_markdown(text: &str) -> Option<String> {
    let patterns = ["```typescript\n", "```ts\n", "```\n"];

    for pattern in patterns {
        if let Some(start_pos) = text.find(pattern) {
            let code_start = start_pos + pattern.len();
            if let Some(end_pos) = text[code_start..].find("\n```") {
                return Some(text[code_start..code_start + end_pos].trim().to_string());
            }
        }
    }

    None
}

fn extract_from_json(text: &str) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(text).ok()?;
    for field in ["code", "plugin_code", "typescript", "content"] {
        if let Some(code) = json.get(field).and_then(|value| value.as_str()) {
            return Some(code.to_string());
        }
    }
    None
}
