//! Context builder for agent execution.

use anyhow::Result;
use sentinel_db::{Database, ExecutionTaskItem};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use sentinel_llm::{ChatMessage, LlmConfig};
use sentinel_tools::output_storage::{
    get_execution_context_dir, get_history_path, get_host_context_dir, CONTAINER_CONTEXT_DIR,
};
use sentinel_tools::shell::ShellExecutionMode;

use crate::agents::context_engineering::budget::{ContextBudgetAnalysis, ContextBudgetAnalyzer};
use crate::agents::context_engineering::checkpoint::{
    load_or_init_run_state, save_run_state, ContextRunState,
};
use crate::agents::context_engineering::engine::ContextEngineMode;
use crate::agents::context_engineering::memory_index::{
    evict_low_value_items, ingest_memory_items, retrieve_memory_items_hybrid, MemoryQuery,
    RetrievedMemoryItem,
};
use crate::agents::context_engineering::observability::{record_context_snapshot, ContextSnapshot};
use crate::agents::context_engineering::policy::{ContextPolicy, ContextScope};
use crate::agents::context_engineering::render_artifact_readback_summary;
use crate::agents::context_engineering::sentinel::{
    analyze_intent, apply_sentinel_history_selection, build_sentinel_clarification_state,
    focus_compression_state_for_intent, reconcile_sentinel_clarification, render_sentinel_context,
    restore_pinned_context_for_intent, update_intent_registry, update_pinned_context,
};
use crate::agents::context_engineering::token_utils::{
    estimate_message_tokens, estimate_tokens, SYSTEM_MESSAGE_OVERHEAD_TOKENS,
};
use crate::agents::context_engineering::tool_digest::condense_text;
use crate::agents::context_engineering::types::{trim_history_preserve_tool_pairs, ContextPacket};
use crate::agents::sliding_window::{
    SlidingWindowCompressionEventContext, SlidingWindowConfig, SlidingWindowManager,
};
use crate::agents::types::DocumentAttachmentInfo;
use crate::memory::build_memory_retrieval_trace;
use sentinel_rag::canonicalize_memory_kind;

const USER_FORCED_RULES_CONFIG_CATEGORY: &str = "agent";
const USER_FORCED_RULES_CONFIG_KEY: &str = "user_forced_rules";
const USER_FORCED_RULES_BLOCK_MARKER: &str = "[User Forced Rules]";

pub struct ContextBuildInput {
    pub app_handle: AppHandle,
    pub execution_id: String,
    pub conversation_id: String,
    pub generation: Option<u64>,
    pub active_browser_shell_direct_write_enabled: bool,
    pub active_browser_shell_session_id: Option<String>,
    pub active_terminal_session_fingerprint: Option<String>,
    pub active_terminal_session_id: Option<String>,
    pub working_directory: Option<String>,
    pub base_system_prompt: String,
    pub injected_skill_prompt: Option<String>,
    pub task: String,
    pub provider_config_key: String,
    pub rig_provider: String,
    pub llm_config: LlmConfig,
    pub selected_tool_ids: Vec<String>,
    pub document_attachments: Option<Vec<DocumentAttachmentInfo>>,
    pub engine_mode: ContextEngineMode,
    pub policy: ContextPolicy,
}

pub struct ContextBuildResult {
    pub system_prompt: String,
    pub history_messages: Vec<ChatMessage>,
    pub context_packet: ContextPacket,
    pub budget_analyzer: ContextBudgetAnalyzer,
    pub budget_analysis: ContextBudgetAnalysis,
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
    docker_config: Option<sentinel_tools::DockerSandboxConfig>,
}

async fn resolve_execution_context(app_handle: &AppHandle) -> ExecutionContext {
    let shell_config = if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>()
    {
        crate::commands::tool_commands::agent_config::load_shell_config_from_db(&db).await
    } else {
        sentinel_tools::shell::get_shell_config().await
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
            docker_config: shell_config.docker_config,
        }
    } else {
        ExecutionContext {
            env: ExecutionEnvironment::Host,
            os_name: std::env::consts::OS.to_string(),
            context_dir: get_host_context_dir().display().to_string(),
            docker_config: None,
        }
    }
}

fn env_label(env: ExecutionEnvironment) -> &'static str {
    match env {
        ExecutionEnvironment::Host => "host",
        ExecutionEnvironment::Docker => "docker",
    }
}

fn tool_is_selected(selected_tool_ids: &[String], tool_id: &str) -> bool {
    selected_tool_ids.iter().any(|id| id == tool_id)
}

fn build_tool_usage_priority_block(
    selected_tool_ids: &[String],
    has_bound_browser_shell_session: bool,
) -> String {
    let has_ask_user_question = tool_is_selected(selected_tool_ids, "ask_user_question");
    let has_shell = tool_is_selected(selected_tool_ids, "shell");
    let has_browser_shell = tool_is_selected(selected_tool_ids, "browser_shell");
    let has_glob = tool_is_selected(selected_tool_ids, "glob");
    let has_grep = tool_is_selected(selected_tool_ids, "grep");
    let has_file_read = tool_is_selected(selected_tool_ids, "file_read");

    let mut lines = vec![
        "Use only tools that are actually available in this run. Do not guess or call tool names outside the active toolset.".to_string(),
    ];

    if has_ask_user_question {
        lines.push(
            "Use `ask_user_question` when requirements are ambiguous, when multiple implementation paths are viable, or when you need the user to choose between concrete options.".to_string(),
        );
    }

    let mut file_search_tools = Vec::new();
    if has_glob {
        file_search_tools.push("`glob` for filename discovery");
    }
    if has_grep {
        file_search_tools.push("`grep` for file content search");
    }
    if has_file_read {
        file_search_tools.push("`file_read` for exact file inspection");
    }
    if !file_search_tools.is_empty() {
        lines.push(format!(
            "For workspace file discovery and code/content search, prefer {} because they return structured bounded results.",
            file_search_tools.join(", ")
        ));
    }

    if has_shell {
        lines.push(
            "`shell` uses a bounded wait: short commands return `status:\"completed\"`; commands still running return `status:\"running\"` or `status:\"input_waiting\"` with a session_id.".to_string(),
        );
        if !file_search_tools.is_empty() {
            lines.push(
                "Use `shell` when the task requires command execution, project scripts, build/test, package managers, git commands, interactive terminal work, system inspection, or search semantics not supported by the active file/search tools.".to_string(),
            );
        }
        lines.push(
            "When `shell` returns `completed:false`, do not rerun the same command. Poll with `action:\"poll\"`; write raw stdin with `action:\"write\"` and explicit `chars`; send terminal navigation keys with `action:\"key\"` and `key` such as `ArrowDown` or `Enter`; use `action:\"submit\"` for TTY Enter; do not use `select_option` or depend on `prompt_state`; cancel with `action:\"cancel\"` when needed.".to_string(),
        );
        lines.push(
            "If a command starts a server, watcher, log follower, or anything expected to keep running, prefer `shell` with `run_in_background=true` so the conversation stays responsive. Example: starting a dev server should be `shell {\"command\":\"npm run dev\",\"run_in_background\":true}` instead of a foreground `shell {\"command\":\"npm run dev\"}`. Example: following logs should be `shell {\"command\":\"docker logs -f api\",\"run_in_background\":true}`.".to_string(),
        );
    }

    if has_browser_shell {
        let mut browser_shell_line = "Use `browser_shell` when the target terminal is a third-party browser WebSocket shell captured by the Sentinel Chrome extension.".to_string();
        if has_bound_browser_shell_session {
            browser_shell_line.push_str(" Prefer the currently bound `browser_shell` session when you need the actual terminal stream rather than page content.");
        } else {
            browser_shell_line.push_str(
                " Use it when you need the actual terminal stream rather than page content.",
            );
        }
        lines.push(browser_shell_line);
    }

    if has_shell {
        lines.push(
            "Never leave the conversation blocked on a long-lived foreground shell command."
                .to_string(),
        );
    }

    format!("\n\n[Tool Usage Priority]\n- {}", lines.join("\n- "))
}

pub async fn build_context(input: ContextBuildInput) -> Result<ContextBuildResult> {
    let sentinel_mode = input.engine_mode == ContextEngineMode::SentinelLike;
    let mut system_prompt = input.base_system_prompt;
    if !system_prompt.contains(USER_FORCED_RULES_BLOCK_MARKER) {
        if let Some(db) = input
            .app_handle
            .try_state::<Arc<sentinel_db::DatabaseService>>()
        {
            if let Ok(Some(raw_rules)) = db
                .get_config(
                    USER_FORCED_RULES_CONFIG_CATEGORY,
                    USER_FORCED_RULES_CONFIG_KEY,
                )
                .await
            {
                let forced_rules = raw_rules.trim();
                if !forced_rules.is_empty() {
                    system_prompt.push_str(&format!(
                        "\n\n[User Forced Rules]\n{}\n\n[Rule Priority]\n- The above rules are user-defined mandatory instructions. Follow them unless they conflict with higher-priority safety/system constraints.",
                        forced_rules
                    ));
                }
            }
        }
    }
    let execution_context = resolve_execution_context(&input.app_handle).await;
    let mut policy = input.policy.clone();
    if let Some(db) = input
        .app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
    {
        if let Ok(Some(raw)) = db.get_config("agent", "context_packet_v2_enabled").await {
            let enabled = matches!(
                raw.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
            // Caller policy can force-disable v2 retrieval; DB config only disables default-on behavior.
            policy.feature_context_packet_v2 = policy.feature_context_packet_v2 && enabled;
        }
    }
    let mut run_state_block = String::new();
    let mut retrieved_memory_lines: Vec<String> = Vec::new();
    let mut retrieved_memory_sections = Vec::new();
    let mut retrieved_memory_ids: Vec<String> = Vec::new();
    let mut memory_retrieval_trace = None;
    let mut run_state_digests = Vec::new();
    let mut sentinel_run_state: Option<ContextRunState> = None;

    if policy.include_skill_instructions {
        if let Some(injected) = input.injected_skill_prompt {
            system_prompt.push_str(&injected);
        }
    }

    if policy.include_run_state {
        system_prompt.push_str(&format!(
            "\n\n[SystemContext: Current Execution ID is '{}'. Use this for tasks tool calls.]",
            input.execution_id
        ));
        system_prompt.push_str(
            "\n\n[TaskProgressContract]\n\
            - For simple single-step tasks, complete the work directly without creating a UI plan and without calling `tasks`.\n\
            - Use the `tasks` tool to publish the current multi-step plan for the UI.\n\
            - Each call must submit the complete desired `plan`; omitted previous steps are removed from the displayed plan.\n\
            - Keep at most one step `in_progress` and update the plan as meaningful progress happens.\n\
            - Treat the plan as a display/event stream, not as the completion authority for the execution."
        );
    }

    if policy.include_stuck_resolution_rule
        && tool_is_selected(&input.selected_tool_ids, "tenth_man_review")
    {
        system_prompt.push_str(
            "\n\n[Stuck Resolution Rule]\n\
            Do not rely on your subjective feeling of being stuck. You MUST immediately call the `tenth_man_review` tool with `review_mode: { mode: 'full_history' }` and `review_type: 'full'` before another retry when any of these are true: you have already spent 3-4 turns on the same path; you are repeating the same tool family, route family, command pattern, or parameter pattern without clear new evidence; your next step is only a small variation of a failed attempt; you cannot clearly state what new information the next attempt should produce; or your current plan still depends on an unverified assumption. Before repeating a path, explicitly ask yourself: \"What new evidence will this attempt produce?\" If the answer is weak, unclear, or mostly the same as before, call `tenth_man_review` first. Do NOT continue guessing."
        );
    }

    if policy.include_task_mainline {
        system_prompt = inject_task_mainline_summary(system_prompt, &input.task);
    }

    if policy.include_run_state {
        let init_state = ContextRunState {
            task: input.task.clone(),
            task_brief: condense_text(&input.task, policy.task_brief_max_chars),
            selected_tools: input.selected_tool_ids.clone(),
            goals: Vec::new(),
            constraints: Vec::new(),
            decisions: Vec::new(),
            open_tasks: Vec::new(),
            user_preferences: Vec::new(),
            current_plan: None,
            last_tool_digests: vec![],
            tracked_artifacts: Vec::new(),
            memory_items: Vec::new(),
            sentinel_active_intent: None,
            sentinel_intent_registry: Vec::new(),
            sentinel_pinned_context: Default::default(),
            sentinel_compression_state: Default::default(),
            sentinel_last_clarification: None,
            run_state_version: 0,
            last_updated_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        let mut state =
            load_or_init_run_state(&input.app_handle, &input.execution_id, init_state).await?;
        state.task = input.task.clone();
        state.task_brief = condense_text(&input.task, policy.task_brief_max_chars);
        state.selected_tools = input.selected_tool_ids.clone();
        if !state.goals.iter().any(|goal| goal == &state.task_brief) {
            state.goals.push(state.task_brief.clone());
        }
        state.goals.truncate(8);
        state.constraints.truncate(12);
        state.decisions.truncate(16);
        state.user_preferences.truncate(10);

        if sentinel_mode {
            let intent = analyze_intent(&input.task, &state.sentinel_intent_registry);
            let clarification = build_sentinel_clarification_state(&intent);
            update_intent_registry(&mut state.sentinel_intent_registry, &intent);
            restore_pinned_context_for_intent(
                &mut state.sentinel_pinned_context,
                &state.sentinel_compression_state,
                &intent,
            );
            update_pinned_context(
                &mut state.sentinel_pinned_context,
                &input.task,
                &intent,
                &state.sentinel_compression_state,
            );
            state.sentinel_active_intent = Some(intent);
            state.sentinel_last_clarification = Some(clarification);
        }

        let tasks = load_execution_tasks(&input.app_handle, &input.execution_id).await;
        if let Some(ref items) = tasks {
            state.open_tasks = items
                .iter()
                .filter(|item| {
                    matches!(
                        item.status.to_lowercase().as_str(),
                        "pending" | "in_progress" | "inprogress"
                    )
                })
                .map(|item| item.description.trim().to_string())
                .filter(|item| !item.is_empty())
                .take(12)
                .collect();
        }
        let memory_facts = vec![state.task_brief.clone()];
        let memory_decisions = state.decisions.clone();
        let memory_tasks = state.open_tasks.clone();
        // Disable automatic long-term memory persistence:
        // fact/decision/task items are kept in run-state memory only.
        ingest_memory_items(&mut state, &memory_facts, &memory_decisions, &memory_tasks);
        evict_low_value_items(&mut state);
        run_state_digests = state.last_tool_digests.clone();
        if sentinel_mode {
            if let Some(clarification) = state.sentinel_last_clarification.take() {
                state.sentinel_last_clarification = Some(reconcile_sentinel_clarification(
                    clarification,
                    &run_state_digests,
                ));
            }
        }
        if policy.feature_context_packet_v2 {
            let retrieval_query = if sentinel_mode {
                if let Some(intent) = state.sentinel_active_intent.as_ref() {
                    format!(
                        "{}\n{}\n{}\n{}",
                        intent.goal,
                        intent.focus_objects.join(" "),
                        intent.constraints.join(" "),
                        input.task
                    )
                } else {
                    format!("{}\n{}", state.task_brief, input.task)
                }
            } else {
                format!("{}\n{}", state.task_brief, input.task)
            };
            let query = MemoryQuery {
                execution_id: input.execution_id.clone(),
                query: retrieval_query,
                top_k: 8,
                include_reflection: false,
            };
            let retrieved =
                retrieve_memory_items_hybrid(&input.app_handle, &mut state, &query).await;
            retrieved_memory_ids = retrieved.iter().map(|item| item.id.clone()).collect();
            memory_retrieval_trace = Some(build_memory_retrieval_trace(
                &query.query,
                query.top_k,
                &retrieved,
                false,
                query.include_reflection,
            ));
            let retrieved_text = retrieved
                .iter()
                .map(|item| {
                    format!(
                        "[{}|importance={}|score={:.2}] {}",
                        item.kind, item.importance, item.score, item.text
                    )
                })
                .collect::<Vec<_>>();
            retrieved_memory_lines = retrieved_text.clone();
            retrieved_memory_sections = build_retrieved_memory_sections(&retrieved);
        }
        state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
        save_run_state(&input.app_handle, &input.execution_id, &state).await?;

        if let Some(ref items) = tasks {
            if !items.is_empty() {
                run_state_block.push_str(&build_tasks_context(items, policy.run_state_max_chars));
            }
        }
        // Pass None for tasks to avoid duplicating what build_tasks_context already rendered
        run_state_block.push_str(&render_run_state(&state, &policy, None));
        if sentinel_mode {
            if let (Some(intent), Some(clarification)) = (
                state.sentinel_active_intent.as_ref(),
                state.sentinel_last_clarification.as_ref(),
            ) {
                let focused_compression =
                    focus_compression_state_for_intent(&state.sentinel_compression_state, intent);
                let sentinel_context = render_sentinel_context(
                    intent,
                    &state.sentinel_pinned_context,
                    &focused_compression,
                    clarification,
                    &state.sentinel_intent_registry,
                );
                if !sentinel_context.trim().is_empty() {
                    run_state_block.push_str("\n\n");
                    run_state_block.push_str(&sentinel_context);
                }
            }
            sentinel_run_state = Some(state.clone());
        }
    }

    if policy.include_working_dir {
        let working_dir = match execution_context.env {
            ExecutionEnvironment::Docker => execution_context.context_dir.clone(),
            ExecutionEnvironment::Host => input
                .working_directory
                .clone()
                .filter(|dir| !dir.trim().is_empty())
                .unwrap_or_else(|| execution_context.context_dir.clone()),
        };

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
        tracing::info!(
            "Injected working directory into system prompt: {}",
            working_dir
        );
    }

    let has_shell = tool_is_selected(&input.selected_tool_ids, "shell");
    let has_browser_shell = tool_is_selected(&input.selected_tool_ids, "browser_shell");

    if has_browser_shell {
        if let Some(browser_shell_session_id) = input
            .active_browser_shell_session_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let mut browser_shell_note = vec![
                "A third-party browser WebSocket shell session is already selected for this conversation.".to_string(),
                "Prefer the `browser_shell` tool when the task should operate inside that captured browser terminal.".to_string(),
            ];

            let mut alternatives = Vec::new();
            if has_shell {
                alternatives.push("local `shell`");
            }
            if !alternatives.is_empty() {
                browser_shell_note.push(format!(
                    "If the user asks to use the current browser shell / browser terminal / third-party shell, do not use {} for that command.",
                    alternatives.join(", ")
                ));
            }
            browser_shell_note.push(
                "Reuse the selected browser shell session unless the user explicitly asks for a different one.".to_string(),
            );
            browser_shell_note.push(
                "Do not invent, guess, or rotate browser shell session IDs yourself.".to_string(),
            );

            system_prompt.push_str("\n\n[Browser Shell Session]\n- ");
            system_prompt.push_str(&browser_shell_note.join("\n- "));
            system_prompt.push_str(&format!(
                "\n- Active browser shell session id: {}",
                browser_shell_session_id
            ));
            if input.active_browser_shell_direct_write_enabled {
                system_prompt.push_str(
                    "\n- The user has explicitly authorized direct execution on this browser shell session.\n- When you intentionally send input to this selected browser shell session, you may set `browser_shell.requires_approval=false`.\n- Do not use direct execution on a different browser shell session unless the user re-authorizes it.",
                );
            } else {
                system_prompt.push_str(
                    "\n- Browser shell writes still require approval. Keep `browser_shell.requires_approval=true` unless the user explicitly authorizes direct execution.",
                );
            }
        }
    }

    system_prompt.push_str(&build_tool_usage_priority_block(
        &input.selected_tool_ids,
        input
            .active_browser_shell_session_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_some(),
    ));

    system_prompt.push_str(
        "\n\n[File Editing Efficiency Rules]\n\
        When editing existing files, optimize for incremental updates to reduce token/time cost:\n\
        1) Prefer minimal, targeted edits (line-level patch/diff) over full-file rewrites.\n\
        2) If a target file already exists, do NOT regenerate the entire file unless explicitly requested.\n\
        3) Before any `file_edit`, first do a `file_read` of that file in the current execution.\n\
        4) Before `file_write` with `overwrite=true`, first do a `file_read` of that file in the current execution.\n\
        5) If the file changed after you read it, reread it before the next edit/overwrite.\n\
        6) Keep stable sections unchanged; isolate tunable values in compact config/parameter blocks where possible.\n\
        7) After each edit, run a short validation command and continue with small deltas.\n\
        8) In responses, summarize what changed instead of repeating full file content unless needed.",
    );

    if policy.include_context_storage {
        let execution_session_dir =
            get_execution_context_dir(&execution_context.context_dir, Some(&input.execution_id));
        let history_path =
            get_history_path(&execution_context.context_dir, Some(&input.execution_id));
        system_prompt.push_str(&format!(
            "\n\n[Context Storage]\n\
            - Environment: {} ({})\n\
            - Execution ID: {}\n\
            - Base context root: '{}'\n\
            - This execution's session directory: '{}'\n\
            - Tool outputs exceeding threshold are saved as files (not truncated)\n\
            - Applies to: shell commands, HTTP responses, and other tools\n\
            - Your conversation history is at '{}' (isolated per execution)\n\
            - Treat stored-output previews as hints only, not as complete evidence.\n\
            - Before making claims about a stored artifact, read it sequentially in bounded chunks until the full file is covered.\n\
            - Prefer `file_read` with increasing `offset` and bounded `limit` for host files; use shell line-range reads for container files.\n\
            - If you have only read part of a stored artifact, say the result is partial and cite the ranges inspected.\n\
            - Use local runtime context (not prompt) for detailed file exploration commands.\n\
            - Keep model responses focused on task-critical facts and artifact references.",
            env_label(execution_context.env),
            execution_context.os_name,
            input.execution_id,
            execution_context.context_dir,
            execution_session_dir,
            history_path
        ));
    }

    if policy.include_document_attachments {
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

    system_prompt = trim_layer(system_prompt, policy.layer_max_chars);

    let max_context_length =
        get_provider_max_context_length(&input.app_handle, &input.provider_config_key).await?;
    let max_tokens = max_context_length as usize;
    let budget_analyzer = ContextBudgetAnalyzer::new(max_tokens);
    let budget = policy.budget.scale_to_context(max_tokens);

    let mut packet = ContextPacket::new(system_prompt);
    packet.run_state = condense_text(
        &run_state_block,
        run_state_block_target_chars(max_tokens, budget.run_state_max_tokens),
    );
    if policy.feature_context_packet_v2 {
        packet.retrieved_memories = retrieved_memory_lines;
        packet.retrieved_memory_sections = retrieved_memory_sections;
    }
    packet.set_tool_digests(&run_state_digests);

    let sw_config = SlidingWindowConfig {
        max_context_tokens: max_context_length as usize,
        ..Default::default()
    };

    let mut sliding_window =
        SlidingWindowManager::new(&input.app_handle, &input.conversation_id, Some(sw_config))
            .await?;

    if let Err(e) = sliding_window
        .compress_if_needed(
            &input.llm_config,
            Some(SlidingWindowCompressionEventContext {
                execution_id: &input.execution_id,
                generation: input.generation,
            }),
        )
        .await
    {
        tracing::warn!("Sliding window compression failed: {}", e);
    }

    if policy.scope == ContextScope::Agent {
        if let Ok(history_content) = sliding_window.export_history().await {
            match execution_context.env {
                ExecutionEnvironment::Docker => {
                    if let Some(ref docker_config) = execution_context.docker_config {
                        let sandbox = sentinel_tools::DockerSandbox::new(docker_config.clone());
                        let history_path =
                            get_history_path(CONTAINER_CONTEXT_DIR, Some(&input.execution_id));
                        if let Err(e) =
                            sentinel_tools::output_storage::store_history_in_container_with_id(
                                &sandbox,
                                &history_content,
                                Some(&input.execution_id),
                            )
                            .await
                        {
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
                    let history_path =
                        get_history_path(&execution_context.context_dir, Some(&input.execution_id));
                    if let Err(e) = sentinel_tools::output_storage::store_history_on_host(
                        &history_content,
                        Some(&input.execution_id),
                    )
                    .await
                    {
                        tracing::warn!("Failed to store history on host: {}", e);
                    } else {
                        tracing::info!("Conversation history exported to host: {}", history_path);
                    }
                }
            }
        }
    }

    let context_messages = sliding_window.build_context(&packet.system_instructions);
    let (mut system_prompt_content, mut history_messages) =
        split_context_messages(&packet.system_instructions, context_messages);

    if history_messages.is_empty() {
        if let Some(fallback) =
            load_fallback_history(&input.app_handle, &input.conversation_id, 6).await
        {
            history_messages = fallback;
        }
    }

    let safe_limit = budget_analyzer.safe_limit_tokens;
    let mut trim_trace = Vec::new();

    // Single system prompt trim pass (after sliding window summaries are included)
    let summary_stats = sliding_window.summary_stats();
    let summary_overhead =
        summary_stats.global_summary_tokens + summary_stats.segment_summary_tokens;
    let effective_system_budget = budget.system_max_tokens + summary_overhead;
    let system_budget_cap = (safe_limit as f64 * 0.55) as usize;
    let final_system_budget = effective_system_budget.min(system_budget_cap);

    let mut system_tokens =
        estimate_tokens(&system_prompt_content) + SYSTEM_MESSAGE_OVERHEAD_TOKENS;
    if system_tokens > final_system_budget {
        let current_chars = system_prompt_content.chars().count().max(1);
        let ratio = final_system_budget as f64 / system_tokens as f64;
        let target_chars = ((current_chars as f64) * ratio).floor() as usize;
        system_prompt_content = condense_text(&system_prompt_content, target_chars.max(200));
        system_tokens = estimate_tokens(&system_prompt_content) + SYSTEM_MESSAGE_OVERHEAD_TOKENS;
        trim_trace.push("trimmed_system".to_string());
    }

    let mut run_state_tokens = estimate_tokens(&packet.run_state);
    if run_state_tokens > budget.run_state_max_tokens {
        packet.run_state = condense_text(
            &packet.run_state,
            run_state_block_target_chars(max_tokens, budget.run_state_max_tokens),
        );
        run_state_tokens = estimate_tokens(&packet.run_state);
        trim_trace.push("trimmed_run_state".to_string());
    }

    let mut retrieval_tokens = estimate_tokens(&packet.render_retrieved_memory_context());
    while retrieval_tokens > budget.retrieval_max_tokens
        && (!packet.retrieved_memory_sections.is_empty() || !packet.retrieved_memories.is_empty())
    {
        if !packet.retrieved_memory_sections.is_empty() {
            trim_retrieved_memory_sections(&mut packet.retrieved_memory_sections);
        } else if !packet.retrieved_memories.is_empty() {
            packet.retrieved_memories.pop();
        }
        retrieval_tokens = estimate_tokens(&packet.render_retrieved_memory_context());
    }
    if retrieval_tokens > budget.retrieval_max_tokens {
        trim_trace.push("trimmed_retrieval".to_string());
    }

    let mut tool_digest_tokens = estimate_tokens(
        &packet
            .tool_digests
            .iter()
            .map(|item| format!("{} {}", item.tool_name, item.summary))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    while tool_digest_tokens > budget.tool_digest_max_tokens && !packet.tool_digests.is_empty() {
        packet.tool_digests.remove(0);
        tool_digest_tokens = estimate_tokens(
            &packet
                .tool_digests
                .iter()
                .map(|item| format!("{} {}", item.tool_name, item.summary))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    if tool_digest_tokens > budget.tool_digest_max_tokens {
        trim_trace.push("trimmed_tool_digests".to_string());
    }

    packet.orchestrator_context = packet.render_orchestrator_context();
    let context_overlay_messages = packet.render_context_messages(policy.message_layout);
    let orchestrator_context_tokens: usize = context_overlay_messages
        .iter()
        .map(estimate_message_tokens)
        .sum();

    let available_for_history = safe_limit
        .saturating_sub(system_tokens)
        .saturating_sub(orchestrator_context_tokens)
        .min(budget.window_max_tokens);

    if sentinel_mode {
        if let Some(state) = sentinel_run_state.as_mut() {
            if let (Some(intent), Some(clarification)) = (
                state.sentinel_active_intent.clone(),
                state.sentinel_last_clarification.clone(),
            ) {
                let selection = apply_sentinel_history_selection(
                    &history_messages,
                    &intent,
                    &clarification,
                    available_for_history,
                    &mut state.sentinel_compression_state,
                );
                if !selection.dropped_slices.is_empty() {
                    update_pinned_context(
                        &mut state.sentinel_pinned_context,
                        &input.task,
                        &intent,
                        &state.sentinel_compression_state,
                    );
                    state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
                    save_run_state(&input.app_handle, &input.execution_id, state).await?;
                }
                history_messages = selection.kept_history;
                trim_trace.extend(selection.trim_trace);
            }
        }
    }

    let history_tokens: usize = history_messages.iter().map(estimate_message_tokens).sum();
    if history_tokens > available_for_history {
        history_messages = trim_history_preserve_tool_pairs(
            &history_messages,
            history_tokens,
            available_for_history,
            estimate_message_tokens,
        );
        trim_trace.push("trimmed_window".to_string());
    }

    if !context_overlay_messages.is_empty() {
        history_messages.splice(0..0, context_overlay_messages);
    }

    packet.system_instructions = system_prompt_content.clone();
    packet.window_messages = history_messages.clone();

    let system_prompt_tokens =
        estimate_tokens(&packet.render_system_prompt()) + SYSTEM_MESSAGE_OVERHEAD_TOKENS;
    let summary_tokens = summary_stats.global_summary_tokens + summary_stats.segment_summary_tokens;
    let history_tokens: usize = packet
        .window_messages
        .iter()
        .map(estimate_message_tokens)
        .sum();
    let used_tokens = system_prompt_tokens + history_tokens;
    let budget_analysis = budget_analyzer.analyze(used_tokens);

    let _ = input.app_handle.emit(
        "agent:context_usage",
        &json!({
            "execution_id": input.execution_id,
            "generation": input.generation,
            "used_tokens": used_tokens,
            "max_tokens": max_tokens,
            "effective_context_tokens": budget_analyzer.effective_context_tokens,
            "remaining_tokens": budget_analysis.remaining_tokens,
            "usage_percentage": budget_analysis.usage_percentage,
            "context_pressure": format!("{:?}", budget_analysis.pressure),
            "warning_threshold_tokens": budget_analyzer.warning_threshold_tokens,
            "auto_compact_threshold_tokens": budget_analyzer.auto_compact_threshold_tokens,
            "blocking_threshold_tokens": budget_analyzer.blocking_threshold_tokens,
            "output_reserve_tokens": budget_analyzer.output_reserve_tokens,
            "system_prompt_tokens": system_prompt_tokens,
            "run_state_tokens": run_state_tokens,
            "history_tokens": history_tokens,
            "history_count": packet.window_messages.len(),
            "summary_tokens": summary_tokens,
            "summary_global_tokens": summary_stats.global_summary_tokens,
            "summary_segment_tokens": summary_stats.segment_summary_tokens,
            "summary_segment_count": summary_stats.segment_count,
            "orchestrator_context_tokens": orchestrator_context_tokens,
            "trim_trace": trim_trace,
            "sentinel_mode": sentinel_mode,
            "sentinel_active_intent": sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .map(|intent| json!({
                    "intent_id": intent.intent_id,
                    "relation": format!("{:?}", intent.relation),
                    "transition": format!("{:?}", intent.last_transition),
                    "confidence": intent.confidence,
                    "parent_intent_id": intent.parent_intent_id,
                    "resumed_from_intent_id": intent.resumed_from_intent_id,
                })),
            "sentinel_clarification": sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_last_clarification.as_ref())
                .map(|clarification| json!({
                    "needed": clarification.needed,
                    "status": clarification.resolution_status,
                    "source": clarification.resolution_source,
                    "compression_aggressiveness": format!("{:?}", clarification.compression_aggressiveness),
                })),
        }),
    );
    tracing::info!(
        "Context usage - execution_id: {}, system: {}, history: {}, summary: {}, used: {}, max: {}",
        input.execution_id,
        system_prompt_tokens,
        history_tokens,
        summary_tokens,
        used_tokens,
        max_tokens
    );

    let _ = input.app_handle.emit(
        "agent:context_built",
        &json!({
            "execution_id": input.execution_id,
            "generation": input.generation,
            "history_count": packet.window_messages.len(),
        }),
    );

    record_context_snapshot(
        &input.app_handle,
        &ContextSnapshot {
            execution_id: input.execution_id.clone(),
            generation: input.generation,
            system_tokens: estimate_tokens(&packet.system_instructions),
            run_state_tokens,
            window_tokens: history_tokens,
            retrieval_tokens: estimate_tokens(&packet.render_retrieved_memory_context()),
            tool_digest_tokens: estimate_tokens(
                &packet
                    .tool_digests
                    .iter()
                    .map(|t| t.summary.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            total_tokens: used_tokens,
            max_tokens,
            trim_trace,
            retrieval_ids: retrieved_memory_ids,
            memory_retrieval: memory_retrieval_trace,
            sentinel_mode,
            sentinel_intent_id: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .map(|intent| intent.intent_id.clone()),
            sentinel_intent_confidence: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .map(|intent| intent.confidence),
            sentinel_intent_relation: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .map(|intent| format!("{:?}", intent.relation)),
            sentinel_intent_transition: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .map(|intent| format!("{:?}", intent.last_transition)),
            sentinel_parent_intent_id: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_active_intent.as_ref())
                .and_then(|intent| intent.parent_intent_id.clone()),
            sentinel_clarification_needed: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_last_clarification.as_ref())
                .map(|clarification| clarification.needed)
                .unwrap_or(false),
            sentinel_compression_aggressiveness: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_last_clarification.as_ref())
                .map(|clarification| format!("{:?}", clarification.compression_aggressiveness)),
            sentinel_clarification_status: sentinel_run_state
                .as_ref()
                .and_then(|state| state.sentinel_last_clarification.as_ref())
                .map(|clarification| clarification.resolution_status.clone()),
        },
    );

    Ok(ContextBuildResult {
        system_prompt: packet.render_system_prompt(),
        history_messages: packet.window_messages.clone(),
        context_packet: packet,
        budget_analyzer,
        budget_analysis,
    })
}

fn build_retrieved_memory_sections(
    retrieved: &[RetrievedMemoryItem],
) -> Vec<crate::agents::context_engineering::types::RetrievedMemorySection> {
    let mut preferences = Vec::new();
    let mut decisions = Vec::new();
    let mut anti_patterns = Vec::new();
    let mut sop_hints = Vec::new();
    let mut general = Vec::new();

    for item in retrieved {
        let mut qualifiers = vec![
            item.kind.clone(),
            format!("importance={}", item.importance),
            format!("score={:.2}", item.score),
        ];
        if item.scope != "project" {
            qualifiers.push(format!("scope={}", item.scope));
        }
        if item.stability != "stable" {
            qualifiers.push(format!("stability={}", item.stability));
        }
        if item.source != "context_engineering" && item.source != "unknown" {
            qualifiers.push(format!("source={}", item.source));
        }
        if item.confidence < 0.8 {
            qualifiers.push(format!("confidence={:.2}", item.confidence));
        }
        let line = format!("[{}] {}", qualifiers.join("|"), item.text);
        match normalize_memory_kind(item.kind.as_str()).as_str() {
            "preference" => preferences.push(line),
            "decision" => decisions.push(line),
            "anti_pattern" => anti_patterns.push(line),
            "sop" => sop_hints.push(line),
            _ => general.push(line),
        }
    }

    let mut sections = Vec::new();
    push_memory_section(&mut sections, "Durable Preferences", preferences, 3);
    push_memory_section(&mut sections, "Relevant Decisions", decisions, 4);
    push_memory_section(&mut sections, "Known Anti-patterns", anti_patterns, 3);
    push_memory_section(&mut sections, "Reusable SOP Hints", sop_hints, 3);
    push_memory_section(&mut sections, "Relevant Memory", general, 4);
    sections
}

fn push_memory_section(
    sections: &mut Vec<crate::agents::context_engineering::types::RetrievedMemorySection>,
    title: &str,
    mut items: Vec<String>,
    limit: usize,
) {
    if items.is_empty() || limit == 0 {
        return;
    }
    items.truncate(limit);
    sections.push(
        crate::agents::context_engineering::types::RetrievedMemorySection {
            title: title.to_string(),
            items,
        },
    );
}

fn normalize_memory_kind(kind: &str) -> String {
    canonicalize_memory_kind(kind)
}

fn trim_retrieved_memory_sections(
    sections: &mut Vec<crate::agents::context_engineering::types::RetrievedMemorySection>,
) {
    while let Some(last) = sections.last_mut() {
        if last.items.pop().is_some() {
            if last.items.is_empty() {
                sections.pop();
            }
            return;
        }
        sections.pop();
    }
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
    tasks: Option<&[ExecutionTaskItem]>,
) -> String {
    let mut out = String::new();
    if !state.goals.is_empty() {
        out.push_str("Goals:\n");
        for goal in state.goals.iter().take(6) {
            out.push_str("- ");
            out.push_str(goal.trim());
            out.push('\n');
        }
    }
    if !state.constraints.is_empty() {
        out.push_str("Constraints:\n");
        for constraint in state.constraints.iter().take(6) {
            out.push_str("- ");
            out.push_str(constraint.trim());
            out.push('\n');
        }
    }
    if !state.decisions.is_empty() {
        out.push_str("Decisions:\n");
        for decision in state.decisions.iter().take(8) {
            out.push_str("- ");
            out.push_str(decision.trim());
            out.push('\n');
        }
    }
    if let Some(items) = tasks {
        if !items.is_empty() {
            out.push_str("Todos Summary:\n");
            for item in items.iter().take(8) {
                out.push_str(&format!("- [{}] {}", item.status, item.description.trim()));
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
    if let Some(plan) = state
        .current_plan
        .as_ref()
        .filter(|plan| !plan.trim().is_empty())
    {
        out.push_str(&format!("Current Plan: {}\n", condense_text(plan, 280)));
    }
    if !state.selected_tools.is_empty() {
        out.push_str(&format!(
            "Selected Tools: {}\n",
            state.selected_tools.join(", ")
        ));
    }
    if let Some(summary) = render_artifact_readback_summary(&state.tracked_artifacts) {
        out.push_str(&summary);
        out.push('\n');
    }
    // Tool digests are rendered in ContextPacket::render_orchestrator_context
    // and injected as a user-context message to avoid dynamic data in system.
    condense_text(&out, policy.run_state_max_chars)
}

fn build_tasks_context(items: &[ExecutionTaskItem], max_chars: usize) -> String {
    let mut out = String::new();
    out.push_str("\n\n[Tasks]\n");
    for item in items.iter().take(20) {
        out.push_str(&format!("- [{}] {}", item.status, item.description.trim()));
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

async fn load_execution_tasks(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Option<Vec<ExecutionTaskItem>> {
    let db = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>()?;
    match db.get_execution_tasks(execution_id).await {
        Ok(items) if !items.is_empty() => Some(items),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Failed to load tasks for run state: {}", e);
            None
        }
    }
}

async fn load_fallback_history(
    app_handle: &AppHandle,
    conversation_id: &str,
    limit: usize,
) -> Option<Vec<ChatMessage>> {
    let db = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>()?;
    let messages = db
        .get_ai_messages_by_conversation(conversation_id)
        .await
        .ok()?;
    if messages.is_empty() {
        return None;
    }
    let chat_messages = crate::commands::ai::reconstruct_chat_history(&messages);
    if chat_messages.is_empty() {
        return None;
    }
    let start = chat_messages.len().saturating_sub(limit);
    Some(chat_messages[start..].to_vec())
}

fn build_document_attachments_context(attachments: &[DocumentAttachmentInfo]) -> String {
    let mut context = String::new();
    context.push_str("\n\n[Document Attachments]\n");

    for (idx, doc) in attachments.iter().enumerate() {
        context.push_str(&format!(
            "\nDocument #{}:\n- File ID: {}\n- Filename: {}\n- Size: {} bytes\n- MIME Type: {}\n",
            idx + 1,
            doc.id,
            doc.original_filename,
            doc.file_size,
            doc.mime_type
        ));

        if let Some(path) = &doc.file_path {
            context.push_str(&format!("- File Path: {}\n", path));
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

fn run_state_block_target_chars(max_tokens: usize, target_tokens: usize) -> usize {
    if max_tokens == 0 || target_tokens == 0 {
        return 400;
    }
    let ratio = target_tokens as f64 / max_tokens as f64;
    let chars = ((max_tokens as f64 * ratio) / 0.45).round() as usize;
    chars.max(200)
}

async fn get_provider_max_context_length(app_handle: &AppHandle, provider: &str) -> Result<u32> {
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        if let Ok(Some(config_str)) = db.get_config_internal("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<
                std::collections::HashMap<String, serde_json::Value>,
            >(&config_str)
            {
                if let Some(max_ctx) =
                    resolve_provider_max_context_length_from_config(&providers, provider)
                {
                    return Ok(max_ctx);
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

fn resolve_provider_max_context_length_from_config(
    providers: &std::collections::HashMap<String, serde_json::Value>,
    provider: &str,
) -> Option<u32> {
    let provider = provider.trim();
    if provider.is_empty() {
        return None;
    }

    providers.iter().find_map(|(key, value)| {
        let key_matches = key.eq_ignore_ascii_case(provider);
        let configured_provider_matches = value
            .get("provider")
            .and_then(|v| v.as_str())
            .map(|configured| configured.eq_ignore_ascii_case(provider))
            .unwrap_or(false);

        if !key_matches && !configured_provider_matches {
            return None;
        }

        value
            .get("max_context_length")
            .and_then(|v| v.as_u64())
            .and_then(|value| u32::try_from(value).ok())
    })
}

#[cfg(test)]
mod tests {
    use super::{build_tool_usage_priority_block, resolve_provider_max_context_length_from_config};
    use std::collections::HashMap;

    #[test]
    fn provider_context_length_uses_logical_custom_provider() {
        let providers =
            serde_json::from_value::<HashMap<String, serde_json::Value>>(serde_json::json!({
                "Mimo": {
                    "provider": "Mimo",
                    "rig_provider": "openai",
                    "max_context_length": 1_000_000
                },
                "OpenAI": {
                    "provider": "openai",
                    "max_context_length": 128_000
                }
            }))
            .unwrap();

        assert_eq!(
            resolve_provider_max_context_length_from_config(&providers, "mimo"),
            Some(1_000_000)
        );
        assert_eq!(
            resolve_provider_max_context_length_from_config(&providers, "openai"),
            Some(128_000)
        );
    }

    #[test]
    fn tool_usage_priority_omits_shell_guidance_when_shell_tools_are_unavailable() {
        let rendered = build_tool_usage_priority_block(
            &vec!["file_read".to_string(), "grep".to_string()],
            false,
        );

        assert!(rendered.contains("Use only tools that are actually available in this run"));
        assert!(rendered.contains("prefer `grep` for file content search"));
        assert!(rendered.contains("`file_read` for exact file inspection"));
        assert!(!rendered.contains("Use one-shot `shell`"));
        assert!(!rendered.contains("Use `interactive_shell`"));
        assert!(!rendered.contains("Use `browser_shell`"));
    }

    #[test]
    fn tool_usage_priority_keeps_shell_available_for_execution_work() {
        let rendered = build_tool_usage_priority_block(
            &vec![
                "shell".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "file_read".to_string(),
            ],
            false,
        );

        assert!(rendered.contains("prefer `glob` for filename discovery"));
        assert!(rendered.contains("`grep` for file content search"));
        assert!(rendered.contains("Use `shell` when the task requires command execution"));
        assert!(rendered.contains("build/test"));
        assert!(rendered.contains("git commands"));
    }

    #[test]
    fn tool_usage_priority_ignores_removed_interactive_shell_tool() {
        let rendered = build_tool_usage_priority_block(
            &vec![
                "ask_user_question".to_string(),
                "interactive_shell".to_string(),
                "browser_shell".to_string(),
            ],
            true,
        );

        assert!(rendered.contains("Use `ask_user_question`"));
        assert!(!rendered.contains("Use one-shot `shell`"));
        assert!(!rendered.contains("Use `interactive_shell`"));
        assert!(rendered.contains("Use `browser_shell`"));
        assert!(!rendered.contains("pwn"));
        assert!(!rendered.contains("reverse engineering"));
    }

    #[test]
    fn tool_usage_priority_includes_shell_to_exec_handoff_guidance() {
        let rendered = build_tool_usage_priority_block(
            &vec!["shell".to_string(), "interactive_shell".to_string()],
            false,
        );

        assert!(rendered.contains("`shell` uses a bounded wait"));
        assert!(rendered
            .contains("When `shell` returns `completed:false`, do not rerun the same command."));
        assert!(!rendered.contains("Use `interactive_shell`"));
    }
}
