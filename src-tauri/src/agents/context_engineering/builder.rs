//! Context builder for agent execution.

use anyhow::Result;
use sentinel_db::{AgentTodoItem, Database};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use sentinel_llm::{ChatMessage, LlmConfig};
use sentinel_tools::output_storage::{get_host_context_dir, get_history_path, CONTAINER_CONTEXT_DIR};
use sentinel_tools::shell::{get_shell_config, ShellExecutionMode};

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

#[derive(Debug, Clone, Copy)]
enum ExecutionEnvironment {
    Host,
    Docker,
}

struct ExecutionContext {
    env: ExecutionEnvironment,
    os_name: String,
    context_dir: String,
}

async fn resolve_execution_context(app_handle: &AppHandle) -> ExecutionContext {
    let shell_config = if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        crate::commands::tool_commands::agent_config::load_shell_config_from_db(&db).await
    } else {
        get_shell_config().await
    };

    let docker_available = sentinel_tools::DockerSandbox::is_docker_available().await;
    let docker_enabled = shell_config.default_execution_mode == ShellExecutionMode::Docker
        && shell_config.docker_config.is_some()
        && docker_available;

    if docker_enabled {
        ExecutionContext {
            env: ExecutionEnvironment::Docker,
            os_name: "linux".to_string(),
            context_dir: CONTAINER_CONTEXT_DIR.to_string(),
        }
    } else {
        ExecutionContext {
            env: ExecutionEnvironment::Host,
            os_name: std::env::consts::OS.to_string(),
            context_dir: get_host_context_dir().display().to_string(),
        }
    }
}

fn build_context_storage_examples(os_name: &str, context_dir: &str) -> String {
    if os_name.eq_ignore_ascii_case("windows") {
        format!(
            "Examples:\n\
            • Get-ChildItem \"{0}\"  (list all stored files)\n\
            • Select-String -Path \"{0}\\*.txt\" -Pattern \"pattern\"  (search for pattern)\n\
            • Get-Content \"{0}\\http_response_*.txt\" -Tail 100  (view HTTP output)\n\
            • Get-Content \"{0}\\history.txt\"  (view conversation history)",
            context_dir
        )
    } else {
        format!(
            "Examples:\n\
            • ls -lh \"{0}\"  (list all stored files)\n\
            • grep -ri \"pattern\" \"{0}\"/*.txt  (search for pattern)\n\
            • tail -n 100 \"{0}\"/http_response_*.txt  (view HTTP output)\n\
            • cat \"{0}\"/history.txt  (view conversation history)",
            context_dir
        )
    }
}

fn env_label(env: ExecutionEnvironment) -> &'static str {
    match env {
        ExecutionEnvironment::Host => "host",
        ExecutionEnvironment::Docker => "docker",
    }
}

pub async fn build_context(input: ContextBuildInput) -> Result<ContextBuildResult> {
    let mut system_prompt = input.base_system_prompt;
    let execution_context = resolve_execution_context(&input.app_handle).await;

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
        let todos = load_execution_todos(&input.app_handle, &input.execution_id).await;
        if let Some(ref items) = todos {
            if !items.is_empty() {
                system_prompt.push_str(&build_todos_context(items, input.policy.run_state_max_chars));
            }
        }
        system_prompt.push_str(&format!(
            "\n\n[RunState]\n{}",
            render_run_state(&state, &input.policy, todos.as_deref())
        ));
    }

    if input.policy.include_working_dir {
        if let Some(db) = input.app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
            let configured_dir = db
                .get_config("ai", "working_directory")
                .await
                .ok()
                .flatten()
                .filter(|dir| !dir.trim().is_empty());
            let working_dir = configured_dir
                .unwrap_or_else(|| execution_context.context_dir.clone());

            system_prompt.push_str(&format!(
                "\n\n[Execution Environment]\n\
                - Environment: {}\n\
                - OS: {}\n\
                - Working Directory: {}\n\
                \n\
                [Working Directory Note: When performing file operations, executing scripts, or any file system related tasks, use this directory as your base path unless explicitly specified otherwise by the user.]",
                env_label(execution_context.env),
                execution_context.os_name,
                working_dir
            ));
            tracing::info!("Injected working directory into system prompt: {}", working_dir);
        }
    }

    if input.policy.include_context_storage {
        // Use execution_id for history file isolation
        let history_path = get_history_path(&execution_context.context_dir, Some(&input.execution_id));
        let examples = build_context_storage_examples(&execution_context.os_name, &execution_context.context_dir);
        system_prompt.push_str(&format!(
            "\n\n[Context Storage]\n\
            - Environment: {} ({})\n\
            - Execution ID: {}\n\
            - All large tool outputs are stored at '{}'\n\
            - Tool outputs exceeding threshold are saved as files (not truncated)\n\
            - Applies to: shell commands, HTTP responses, and other tools\n\
            - Your conversation history is at '{}' (isolated per execution)\n\
            \n\
            {}\n\
            \n\
            All files are centralized in one directory for easy management and search.\n\
            Note: Each execution has its own history file to prevent cross-session data leakage.",
            env_label(execution_context.env),
            execution_context.os_name,
            input.execution_id,
            execution_context.context_dir,
            history_path,
            examples
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
            // Store history based on execution environment with execution_id isolation
            match execution_context.env {
                ExecutionEnvironment::Docker => {
                    use sentinel_tools::shell::get_shell_config;
                    let shell_config = get_shell_config().await;
                    if let Some(docker_config) = shell_config.docker_config {
                        let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
                        let history_path = get_history_path(CONTAINER_CONTEXT_DIR, Some(&input.execution_id));
                        if let Err(e) = sentinel_tools::output_storage::store_history_in_container_with_id(
                            &sandbox,
                            &history_content,
                            Some(&input.execution_id),
                        ).await {
                            tracing::warn!("Failed to store history in container: {}", e);
                        } else {
                            tracing::info!(
                                "Conversation history exported to container: {}",
                                history_path
                            );
                        }
                    }
                }
                ExecutionEnvironment::Host => {
                    let history_path = get_history_path(&execution_context.context_dir, Some(&input.execution_id));
                    if let Err(e) = sentinel_tools::output_storage::store_history_on_host(
                        &history_content,
                        Some(&input.execution_id),
                    ).await {
                        tracing::warn!("Failed to store history on host: {}", e);
                    } else {
                        tracing::info!(
                            "Conversation history exported to host: {}",
                            history_path
                        );
                    }
                }
            }
        }
    }

    let context_messages = sliding_window.build_context(&system_prompt);
    let (system_prompt_content, mut history_messages) = split_context_messages(
        &system_prompt,
        context_messages,
    );

    // Enforce context limit: truncate history if total exceeds max_context_length
    // Reserve 15% for model output and safety margin
    let safe_limit = (max_context_length as f64 * 0.85) as usize;
    let system_tokens = estimate_tokens(&system_prompt_content);
    let available_for_history = safe_limit.saturating_sub(system_tokens);
    
    // Truncate history from oldest messages if needed
    let mut history_tokens: usize = history_messages.iter()
        .map(|m| {
            let mut tokens = estimate_tokens(&m.content);
            if let Some(ref tc) = m.tool_calls {
                tokens += estimate_tokens(tc);
            }
            if let Some(ref rc) = m.reasoning_content {
                tokens += estimate_tokens(rc);
            }
            if let Some(ref tid) = m.tool_call_id {
                tokens += estimate_tokens(tid);
            }
            tokens
        })
        .sum();
    
    if history_tokens > available_for_history {
        tracing::warn!(
            "Context overflow detected: history_tokens={}, available={}, truncating oldest messages",
            history_tokens,
            available_for_history
        );
        
        // Remove oldest messages until we're under the limit
        while history_tokens > available_for_history && !history_messages.is_empty() {
            if let Some(removed) = history_messages.first() {
                let removed_tokens = {
                    let mut tokens = estimate_tokens(&removed.content);
                    if let Some(ref tc) = removed.tool_calls {
                        tokens += estimate_tokens(tc);
                    }
                    if let Some(ref rc) = removed.reasoning_content {
                        tokens += estimate_tokens(rc);
                    }
                    if let Some(ref tid) = removed.tool_call_id {
                        tokens += estimate_tokens(tid);
                    }
                    tokens
                };
                history_tokens = history_tokens.saturating_sub(removed_tokens);
                history_messages.remove(0);
            }
        }
        
        tracing::info!(
            "After truncation: history_messages={}, history_tokens={}",
            history_messages.len(),
            history_tokens
        );
    }

    // Calculate context usage for frontend display
    // Include all message components: content, tool_calls, reasoning_content
    let system_prompt_tokens = estimate_tokens(&system_prompt_content);
    let summary_stats = sliding_window.summary_stats();
    let summary_tokens = summary_stats.global_summary_tokens + summary_stats.segment_summary_tokens;
    let history_tokens: usize = history_messages.iter()
        .map(|m| {
            let mut tokens = estimate_tokens(&m.content);
            // Include tool_calls if present
            if let Some(ref tc) = m.tool_calls {
                tokens += estimate_tokens(tc);
            }
            // Include reasoning_content if present
            if let Some(ref rc) = m.reasoning_content {
                tokens += estimate_tokens(rc);
            }
            // Include tool_call_id if present (small but should be counted)
            if let Some(ref tid) = m.tool_call_id {
                tokens += estimate_tokens(tid);
            }
            tokens
        })
        .sum();
    let used_tokens = system_prompt_tokens + history_tokens;
    let max_tokens = max_context_length as usize;
    let usage_percentage = if max_tokens > 0 {
        (used_tokens as f64 / max_tokens as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let _ = input.app_handle.emit(
        "agent:context_usage",
        &json!({
            "execution_id": input.execution_id,
            "used_tokens": used_tokens,
            "max_tokens": max_tokens,
            "usage_percentage": usage_percentage,
            "system_prompt_tokens": system_prompt_tokens,
            "history_tokens": history_tokens,
            "history_count": history_messages.len(),
            "summary_tokens": summary_tokens,
            "summary_global_tokens": summary_stats.global_summary_tokens,
            "summary_segment_tokens": summary_stats.segment_summary_tokens,
            "summary_segment_count": summary_stats.segment_count,
        }),
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

fn render_run_state(
    state: &ContextRunState,
    policy: &ContextPolicy,
    todos: Option<&[AgentTodoItem]>,
) -> String {
    let mut out = String::new();
    if let Some(items) = todos {
        if !items.is_empty() {
            out.push_str("Todos Summary:\n");
            for item in items.iter().take(8) {
                out.push_str(&format!(
                    "- [{}] {}",
                    item.status,
                    item.description.trim()
                ));
                if let Some(result) = item.result.as_ref().filter(|r| !r.trim().is_empty()) {
                    out.push_str(&format!(" (result: {})", condense_text(result, 120)));
                }
                out.push('\n');
            }
            if items.len() > 8 {
                out.push_str("- ...<truncated>...\n");
            }
        }
    }
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

fn build_todos_context(items: &[AgentTodoItem], max_chars: usize) -> String {
    let mut out = String::new();
    out.push_str("\n\n[Todos]\n");
    for item in items.iter().take(20) {
        out.push_str(&format!(
            "- [{}] {}",
            item.status,
            item.description.trim()
        ));
        if let Some(result) = item.result.as_ref().filter(|r| !r.trim().is_empty()) {
            out.push_str(&format!(" (result: {})", condense_text(result, 160)));
        }
        out.push('\n');
    }
    if items.len() > 20 {
        out.push_str("- ...<truncated>...\n");
    }
    condense_text(&out, max_chars)
}

async fn load_execution_todos(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Option<Vec<AgentTodoItem>> {
    let db = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>()?;
    match db.get_agent_todos(execution_id).await {
        Ok(items) if !items.is_empty() => Some(items),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Failed to load todos for run state: {}", e);
            None
        }
    }
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

/// Estimate token count for text (improved heuristic)
/// Uses a more conservative estimate to avoid context overflow
fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut total: f64 = 0.0;
    for c in text.chars() {
        if c.is_ascii() {
            // More conservative: ~0.3 tokens per ASCII char (accounts for subword tokenization)
            total += 0.35;
        } else {
            // CJK and other non-ASCII: ~1.5 tokens per char (more conservative)
            total += 1.5;
        }
    }
    // Add 10% safety margin
    (total * 1.1).ceil() as usize
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

