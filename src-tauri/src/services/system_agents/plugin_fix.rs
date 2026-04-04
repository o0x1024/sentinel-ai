use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sentinel_llm::LlmClient;

use crate::generators::{
    ExecutionTestResult, PluginValidator, PromptTemplateBuilder, ValidationResult,
};
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
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
    let run = runtime
        .start_external_run(PLUGIN_FIX_PROFILE_ID, payload, Some("manual".to_string()))
        .await?;

    match execute_plugin_fix(ai_manager, runtime, &request, &run.id).await {
        Ok(result) => {
            let output = serde_json::to_value(&result)?;
            runtime
                .complete_external_run_success(&run.id, PLUGIN_FIX_PROFILE_ID, output)
                .await?;
            Ok(result)
        }
        Err(error) => {
            runtime
                .complete_external_run_failure(&run.id, PLUGIN_FIX_PROFILE_ID, error.to_string())
                .await?;
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
    let profile = runtime
        .get_profile(PLUGIN_FIX_PROFILE_ID)
        .await?
        .ok_or_else(|| anyhow!("System agent profile not found: {}", PLUGIN_FIX_PROFILE_ID))?;
    let tool_policy = SystemAgentToolPolicy::from_profile(&profile);
    tool_policy.validate()?;
    tool_policy.ensure_tool_allowed("plugin_validator")?;
    tool_policy.ensure_tool_allowed("plugin_test_result_reader")?;

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

    let prompt_patch = profile
        .prompt_patch
        .filter(|patch| !patch.trim().is_empty());

    let mut prompt_sections = vec![base_prompt];
    if let Some(prompt_patch) = &prompt_patch {
        prompt_sections.push(format!("Additional instructions:\n{prompt_patch}"));
    }
    if let Some(policy_note) = tool_policy.prompt_note() {
        prompt_sections.push(policy_note);
    }
    let prompt = prompt_sections.join("\n\n");

    let service = resolve_generation_service(ai_manager).await?;
    let model = service.get_config().model.clone();
    let llm_client = LlmClient::new(service.service.to_llm_config());
    let response = llm_client
        .completion(None, &prompt)
        .await
        .context("Failed to call LLM for plugin fix")?;
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
        prompt_patch_applied: prompt_patch.is_some(),
    })
}

async fn resolve_generation_service(
    ai_manager: &Arc<AiServiceManager>,
) -> Result<crate::services::ai::AiServiceWrapper> {
    if let Ok(Some((provider, _model_name))) = ai_manager.get_default_llm_model().await {
        let provider_lc = provider.to_lowercase();
        let maybe_service = ai_manager
            .list_services()
            .into_iter()
            .find_map(|service_name| {
                let service = ai_manager.get_service(&service_name)?;
                let service_provider = service.get_config().provider.to_lowercase();
                if service_provider == provider_lc || service_name.to_lowercase() == provider_lc {
                    Some(service)
                } else {
                    None
                }
            });

        if let Some(service) = maybe_service {
            return Ok(service);
        }
    }

    ai_manager
        .get_service("default")
        .or_else(|| {
            ai_manager
                .list_services()
                .first()
                .and_then(|service_name| ai_manager.get_service(service_name))
        })
        .ok_or_else(|| anyhow!("No AI service available for plugin fix agent"))
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
