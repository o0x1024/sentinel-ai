//! Context builder for agent execution.

use anyhow::Result;
use sentinel_db::Database;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use sentinel_llm::{ChatMessage, LlmConfig};

use crate::agents::context_engineering::checkpoint::{
    load_or_init_run_state, save_run_state, ContextRunState,
};
use crate::agents::context_engineering::policy::{ContextPolicy, ContextScope};
use crate::agents::context_engineering::tool_digest::condense_text;
use crate::agents::sliding_window::{SlidingWindowConfig, SlidingWindowManager};
use crate::agents::types::DocumentAttachmentInfo;

pub struct ContextBuildInput {
    pub app_handle: AppHandle,
    pub execution_id: String,
    pub base_system_prompt: String,
    pub injected_ability_prompt: Option<String>,
    pub task: String,
    pub rig_provider: String,
    pub llm_config: LlmConfig,
    pub selected_tool_ids: Vec<String>,
    pub document_attachments: Option<Vec<DocumentAttachmentInfo>>,
    pub policy: ContextPolicy,
}

pub struct ContextBuildResult {
    pub system_prompt: String,
    pub history_messages: Vec<ChatMessage>,
}

pub async fn build_context(input: ContextBuildInput) -> Result<ContextBuildResult> {
    let mut system_prompt = input.base_system_prompt;

    if input.policy.include_ability_instructions {
        if let Some(injected) = input.injected_ability_prompt {
            system_prompt.push_str(&injected);
        }
    }

    system_prompt.push_str(&format!(
        "\n\n[SystemContext: Current Execution ID is '{}'. Use this for todos tool calls.]",
        input.execution_id
    ));

    if input.policy.include_task_mainline {
        system_prompt = inject_task_mainline_summary(system_prompt, &input.task);
    }

    if input.policy.include_run_state {
        let init_state = ContextRunState {
            task: input.task.clone(),
            task_brief: condense_text(&input.task, input.policy.task_brief_max_chars),
            selected_tools: input.selected_tool_ids.clone(),
            last_tool_digests: vec![],
            last_updated_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        let mut state =
            load_or_init_run_state(&input.app_handle, &input.execution_id, init_state).await?;
        state.task = input.task.clone();
        state.task_brief = condense_text(&input.task, input.policy.task_brief_max_chars);
        state.selected_tools = input.selected_tool_ids.clone();
        state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
        save_run_state(&input.app_handle, &input.execution_id, &state).await?;
        system_prompt.push_str(&format!(
            "\n\n[RunState]\n{}",
            render_run_state(&state, &input.policy)
        ));
    }

    if input.policy.include_working_dir {
        if let Some(db) = input.app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
            if let Ok(Some(working_dir)) = db.get_config("ai", "working_directory").await {
                if !working_dir.is_empty() {
                    system_prompt.push_str(&format!(
                        "\n\n[Working Directory: Your working directory is '{}'. When performing file operations, executing scripts, or any file system related tasks, use this directory as your base path unless explicitly specified otherwise by the user.]",
                        working_dir
                    ));
                    tracing::info!("Injected working directory into system prompt: {}", working_dir);
                }
            }
        }
    }

    if input.policy.include_context_storage {
        system_prompt.push_str(&format!(
            "\n\n[Context Storage]: All large tool outputs are automatically stored in Docker container at '{}'.\n\
            - Tool outputs exceeding threshold are saved as files (not truncated)\n\
            - Applies to: shell commands, HTTP responses, and other tools\n\
            - Your conversation history is at '{}/history.txt'\n\
            - Use shell tool with grep/tail/head/cat to access these files\n\
            \n\
            Examples:\n\
            • shell(command=\"ls -lh {}\")  (list all stored files)\n\
            • shell(command=\"grep -i 'pattern' {}/*.txt\")  (search across files)\n\
            • shell(command=\"tail -n 100 {}/http_response_*.txt\")  (view HTTP output)\n\
            • shell(command=\"cat {}/history.txt | grep 'keyword'\")  (search history)\n\
            \n\
            All files are centralized in one directory for easy management and search.",
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR
        ));
    }

    if input.policy.include_document_attachments {
        if let Some(doc_attachments) = input.document_attachments {
            if !doc_attachments.is_empty() {
                let doc_context = build_document_attachments_context(&doc_attachments);
                system_prompt.push_str(&doc_context);
                tracing::info!(
                    "Injected {} document attachment(s) into system prompt",
                    doc_attachments.len()
                );
            }
        }
    }

    system_prompt = trim_layer(system_prompt, input.policy.layer_max_chars);

    let max_context_length =
        get_provider_max_context_length(&input.app_handle, &input.rig_provider).await?;

    let sw_config = SlidingWindowConfig {
        max_context_tokens: max_context_length as usize,
        ..Default::default()
    };

    let mut sliding_window = SlidingWindowManager::new(
        &input.app_handle,
        &input.execution_id,
        Some(sw_config),
    )
    .await?;

    if let Err(e) = sliding_window.compress_if_needed(&input.llm_config).await {
        tracing::warn!("Sliding window compression failed: {}", e);
    }

    if input.policy.scope == ContextScope::Agent {
        if let Ok(history_content) = sliding_window.export_history().await {
            use sentinel_tools::shell::get_shell_config;
            let shell_config = get_shell_config().await;
            if let Some(docker_config) = shell_config.docker_config {
                let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
                if let Err(e) =
                    sentinel_tools::store_history_in_container(&sandbox, &history_content).await
                {
                    tracing::warn!("Failed to store history in container: {}", e);
                } else {
                    tracing::info!(
                        "Conversation history exported to container: {}/history.txt",
                        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR
                    );
                }
            }
        }
    }

    let context_messages = sliding_window.build_context(&system_prompt);
    let (system_prompt_content, history_messages) = split_context_messages(
        &system_prompt,
        context_messages,
    );

    let _ = input.app_handle.emit(
        "agent:context_built",
        &json!({
            "execution_id": input.execution_id,
            "history_count": history_messages.len(),
        }),
    );

    Ok(ContextBuildResult {
        system_prompt: system_prompt_content,
        history_messages,
    })
}

fn split_context_messages(
    fallback_system_prompt: &str,
    context_messages: Vec<ChatMessage>,
) -> (String, Vec<ChatMessage>) {
    if !context_messages.is_empty() && context_messages[0].role == "system" {
        (
            context_messages[0].content.clone(),
            context_messages[1..].to_vec(),
        )
    } else {
        (fallback_system_prompt.to_string(), context_messages)
    }
}

fn trim_layer(text: String, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text;
    }
    condense_text(&text, max_chars)
}

fn render_run_state(state: &ContextRunState, policy: &ContextPolicy) -> String {
    let mut out = String::new();
    out.push_str(&format!("Task Brief: {}\n", state.task_brief));
    if !state.selected_tools.is_empty() {
        out.push_str(&format!(
            "Selected Tools: {}\n",
            state.selected_tools.join(", ")
        ));
    }
    if !state.last_tool_digests.is_empty() {
        out.push_str("Recent Tool Digests:\n");
        for digest in state.last_tool_digests.iter().rev().take(policy.run_state_max_digests) {
            out.push_str(&format!(
                "- [{}] {}: {}\n",
                digest.status, digest.tool_name, digest.summary
            ));
        }
    }
    let trimmed = condense_text(&out, policy.run_state_max_chars);
    trimmed
}

fn build_document_attachments_context(attachments: &[DocumentAttachmentInfo]) -> String {
    let mut context = String::new();
    context.push_str("\n\n[Document Attachments]\n");

    for (idx, doc) in attachments.iter().enumerate() {
        context.push_str(&format!(
            "\nDocument #{}:\n- Filename: {}\n- Size: {} bytes\n- MIME Type: {}\n- Processing Mode: {}\n",
            idx + 1,
            doc.original_filename,
            doc.file_size,
            doc.mime_type,
            doc.processing_mode
        ));

        if let Some(path) = &doc.container_path {
            context.push_str(&format!("- Container Path: {}\n", path));
        }

        if let Some(text) = &doc.extracted_text {
            let truncated = condense_text(text, 1200);
            context.push_str("- Extracted Text (truncated):\n");
            context.push_str(&truncated);
            context.push('\n');
        }
    }

    context
}

fn inject_task_mainline_summary(mut system_prompt: String, task: &str) -> String {
    if system_prompt.contains("[TaskMainlineSummary]") || system_prompt.contains("任务主线摘要")
    {
        return system_prompt;
    }

    let task_trimmed = task.trim();
    if task_trimmed.is_empty() {
        return system_prompt;
    }

    system_prompt.push_str(&format!(
        "\n\n[TaskMainlineSummary]\n任务主线摘要:\n- 当前任务: {}\n- 约束: 只围绕当前任务推进，避免无关操作。",
        task_trimmed
    ));
    system_prompt
}

async fn get_provider_max_context_length(
    app_handle: &AppHandle,
    provider: &str,
) -> Result<u32> {
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        if let Ok(Some(config_str)) = db.get_config_internal("ai", "providers_config").await {
            if let Ok(providers) =
                serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(
                    &config_str,
                )
            {
                for (key, value) in providers.iter() {
                    if key.to_lowercase() == provider.to_lowercase() {
                        if let Some(max_ctx) =
                            value.get("max_context_length").and_then(|v| v.as_u64())
                        {
                            return Ok(max_ctx as u32);
                        }
                    }
                }
            }
        }
    }

    let default = match provider.to_lowercase().as_str() {
        "openai" => 128000,
        "anthropic" => 200000,
        "gemini" => 1000000,
        "deepseek" => 128000,
        "moonshot" => 128000,
        "groq" => 32000,
        "ollama" => 8192,
        "openrouter" => 128000,
        _ => 128000,
    };

    Ok(default)
}

