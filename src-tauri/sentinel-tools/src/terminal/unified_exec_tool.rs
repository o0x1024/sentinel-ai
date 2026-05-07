use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

use crate::buildin_tools::shell::{
    check_shell_permission, get_shell_config, ShellExecutionMode,
};
use crate::dynamic_tool::{
    DynamicToolBuilder, DynamicToolDef, ToolCategory, ToolExecutionPolicy, ToolSource,
};
use crate::terminal::{
    decode_transport_html_entities, detect_shell_prompt, normalize_command, ExecutionMode,
    TerminalServer, TerminalSessionConfig, WaitStrategy, TERMINAL_MANAGER,
};
use crate::terminal_output::{build_terminal_session_fingerprint, sanitize_interactive_output};

pub const EXEC_COMMAND_TOOL_NAME: &str = "exec_command";
pub const WRITE_STDIN_TOOL_NAME: &str = "write_stdin";

const DEFAULT_DOCKER_IMAGE: &str = "sentinel-sandbox:latest";
const DEFAULT_EXEC_YIELD_TIME_MS: u64 = 10_000;
const DEFAULT_WRITE_STDIN_YIELD_TIME_MS: u64 = 250;
const MIN_YIELD_TIME_MS: u64 = 250;
const MIN_EMPTY_YIELD_TIME_MS: u64 = 5_000;
const MAX_YIELD_TIME_MS: u64 = 30_000;
const MAX_BACKGROUND_YIELD_TIME_MS: u64 = 300_000;

static OUTPUT_CURSORS: Lazy<RwLock<HashMap<String, usize>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
struct ResolvedTerminalSession {
    session_id: String,
    execution_mode: ExecutionMode,
    docker_image: String,
    shell: String,
    working_dir: String,
}

#[derive(Debug, Clone)]
struct CollectOptions {
    wait_strategy: WaitStrategy,
    wait_timeout_ms: u64,
    expected_lines: Option<usize>,
    max_output_tokens: Option<usize>,
}

#[derive(Debug)]
struct CollectedOutput {
    output: Vec<u8>,
    next_cursor: usize,
    completed: bool,
    timed_out: bool,
}

fn cursor_key(execution_id: Option<&str>, session_id: &str) -> String {
    format!("{}::{}", execution_id.unwrap_or("global"), session_id)
}

async fn set_cursor(execution_id: Option<&str>, session_id: &str, cursor: usize) {
    OUTPUT_CURSORS
        .write()
        .await
        .insert(cursor_key(execution_id, session_id), cursor);
}

async fn get_cursor(execution_id: Option<&str>, session_id: &str) -> Option<usize> {
    OUTPUT_CURSORS
        .read()
        .await
        .get(&cursor_key(execution_id, session_id))
        .copied()
}

fn execution_mode_from_value(value: Option<&Value>) -> ExecutionMode {
    match value.and_then(Value::as_str) {
        Some("docker") => ExecutionMode::Docker,
        Some("host") => ExecutionMode::Host,
        _ => ExecutionMode::Host,
    }
}

fn execution_mode_label(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Docker => "docker",
        ExecutionMode::Host => "host",
    }
}

fn clamp_exec_yield_time(value: u64) -> u64 {
    value.clamp(MIN_YIELD_TIME_MS, MAX_YIELD_TIME_MS)
}

fn truncate_for_tokens(text: String, max_output_tokens: Option<usize>) -> String {
    let Some(max_output_tokens) = max_output_tokens else {
        return text;
    };
    let max_chars = max_output_tokens.saturating_mul(4);
    if text.chars().count() <= max_chars {
        return text;
    }
    let mut truncated: String = text.chars().take(max_chars).collect();
    truncated.push_str("\n... [output truncated]");
    truncated
}

fn collect_options_from_interactive_args(args: &Value) -> CollectOptions {
    let wait_strategy = args
        .get("wait_strategy")
        .and_then(Value::as_str)
        .map(WaitStrategy::from_str)
        .unwrap_or_default();
    let wait_timeout_ms = args
        .get("yield_time_ms")
        .and_then(Value::as_u64)
        .map(clamp_exec_yield_time)
        .unwrap_or_else(|| {
            args.get("wait_timeout")
                .and_then(Value::as_u64)
                .unwrap_or(30)
                .min(120)
                .saturating_mul(1000)
        });
    let expected_lines = args
        .get("expected_lines")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let max_output_tokens = args.get("max_output_tokens").and_then(Value::as_u64);

    CollectOptions {
        wait_strategy,
        wait_timeout_ms,
        expected_lines,
        max_output_tokens: max_output_tokens.map(|value| value as usize),
    }
}

fn collect_options_from_exec_args(args: &Value) -> CollectOptions {
    CollectOptions {
        wait_strategy: WaitStrategy::Auto,
        wait_timeout_ms: args
            .get("yield_time_ms")
            .and_then(Value::as_u64)
            .map(clamp_exec_yield_time)
            .unwrap_or(DEFAULT_EXEC_YIELD_TIME_MS),
        expected_lines: None,
        max_output_tokens: args
            .get("max_output_tokens")
            .and_then(Value::as_u64)
            .map(|value| value as usize),
    }
}

fn collect_options_from_write_args(args: &Value, input_is_empty: bool) -> CollectOptions {
    let requested = args
        .get("yield_time_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_WRITE_STDIN_YIELD_TIME_MS)
        .max(MIN_YIELD_TIME_MS);
    let wait_timeout_ms = if input_is_empty {
        requested.clamp(MIN_EMPTY_YIELD_TIME_MS, MAX_BACKGROUND_YIELD_TIME_MS)
    } else {
        requested.min(MAX_YIELD_TIME_MS)
    };

    CollectOptions {
        wait_strategy: WaitStrategy::Auto,
        wait_timeout_ms,
        expected_lines: None,
        max_output_tokens: args
            .get("max_output_tokens")
            .and_then(Value::as_u64)
            .map(|value| value as usize),
    }
}

async fn collect_output_since(
    session_id: &str,
    cursor: usize,
    options: &CollectOptions,
) -> Result<CollectedOutput, String> {
    let deadline = Instant::now() + Duration::from_millis(options.wait_timeout_ms);
    let mut next_cursor = cursor;
    let mut output = Vec::new();
    let mut line_count = 0usize;
    let mut idle_count = 0usize;
    let mut completed = false;
    let mut timed_out = false;

    loop {
        let (chunks, cursor_after_read) = TERMINAL_MANAGER
            .read_session_output_since(session_id, next_cursor)
            .await?;

        if chunks.is_empty() {
            if Instant::now() >= deadline {
                timed_out = true;
                break;
            }
            idle_count += 1;
            if matches!(options.wait_strategy, WaitStrategy::Auto) && !output.is_empty() {
                if idle_count >= 5 {
                    let current_output = String::from_utf8_lossy(&output);
                    completed = detect_shell_prompt(&current_output);
                    break;
                }
            } else if !matches!(options.wait_strategy, WaitStrategy::Timeout)
                && !output.is_empty()
                && idle_count >= 3
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
            continue;
        }

        idle_count = 0;
        next_cursor = cursor_after_read;
        for chunk in chunks {
            line_count += String::from_utf8_lossy(&chunk).matches('\n').count();
            output.extend_from_slice(&chunk);
        }

        let current_output = String::from_utf8_lossy(&output);
        match options.wait_strategy {
            WaitStrategy::Prompt | WaitStrategy::Auto => {
                if detect_shell_prompt(&current_output) {
                    completed = true;
                    break;
                }
            }
            WaitStrategy::Lines => {
                if options
                    .expected_lines
                    .is_some_and(|expected| line_count >= expected)
                {
                    completed = true;
                    break;
                }
            }
            WaitStrategy::Timeout => {}
        }

        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }
    }

    Ok(CollectedOutput {
        output,
        next_cursor,
        completed,
        timed_out,
    })
}

async fn resolve_terminal_session(
    args: &Value,
    execution_id: Option<&str>,
    fallback_active_session_id: Option<&str>,
    working_directory: Option<&str>,
    default_new_session: bool,
) -> Result<ResolvedTerminalSession, String> {
    let config = get_shell_config().await;
    let default_mode = match config.default_execution_mode {
        ShellExecutionMode::Docker => ExecutionMode::Docker,
        ShellExecutionMode::Host => ExecutionMode::Host,
    };
    let execution_mode = args
        .get("execution_mode")
        .map(Some)
        .map(execution_mode_from_value)
        .unwrap_or(default_mode);
    let docker_image = args
        .get("docker_image")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| config.docker_config.as_ref().map(|value| value.image.clone()))
        .unwrap_or_else(|| DEFAULT_DOCKER_IMAGE.to_string());
    let requested_working_dir = args
        .get("working_dir")
        .or_else(|| args.get("workdir"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| working_directory.map(str::to_string));
    let session_policy = args
        .get("session_policy")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| {
            if default_new_session {
                "new".to_string()
            } else {
                "reuse".to_string()
            }
        });
    let active_session_id = args
        .get("active_session_id")
        .or_else(|| args.get("session_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(fallback_active_session_id);

    if session_policy != "new" {
        if let Some(session_id) = active_session_id {
            if let Some(session_lock) = TERMINAL_MANAGER.get_session(session_id).await {
                let session = session_lock.read().await;
                if session.is_healthy() {
                    return Ok(ResolvedTerminalSession {
                        session_id: session.id.clone(),
                        execution_mode: session.config.execution_mode,
                        docker_image: session.config.docker_image.clone(),
                        shell: session.config.shell.clone(),
                        working_dir: session.config.working_dir.clone().unwrap_or_default(),
                    });
                }
            }
        }
    }

    let shell = "bash".to_string();
    let working_dir = match execution_mode {
        ExecutionMode::Docker => "/workspace".to_string(),
        ExecutionMode::Host => requested_working_dir
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|path| path.to_string_lossy().to_string())
            })
            .unwrap_or_default(),
    };
    let session_config = TerminalSessionConfig {
        execution_mode,
        docker_image: docker_image.clone(),
        working_dir: if working_dir.is_empty() {
            None
        } else {
            Some(working_dir.clone())
        },
        env_vars: std::collections::HashMap::new(),
        shell: shell.clone(),
        initial_command: None,
        reuse_container: true,
        container_name: Some("sentinel-sandbox-main".to_string()),
    };

    let (session_id, _output_rx) = TERMINAL_MANAGER.create_session(session_config).await?;
    let cursor = TERMINAL_MANAGER.session_output_cursor(&session_id).await?;
    set_cursor(execution_id, &session_id, cursor).await;

    Ok(ResolvedTerminalSession {
        session_id,
        execution_mode,
        docker_image,
        shell,
        working_dir,
    })
}

async fn run_terminal_command(
    args: Value,
    execution_id_override: Option<String>,
    fallback_active_session_id: Option<String>,
    working_directory: Option<String>,
    codex_exec_shape: bool,
) -> Result<Value, String> {
    let execution_id = execution_id_override
        .as_deref()
        .or_else(|| args.get("execution_id").and_then(Value::as_str));
    let original_command = args
        .get("cmd")
        .or_else(|| args.get("command"))
        .or_else(|| args.get("initial_command"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let session = resolve_terminal_session(
        &args,
        execution_id,
        fallback_active_session_id.as_deref(),
        working_directory.as_deref(),
        codex_exec_shape,
    )
    .await?;
    let session_fingerprint = build_terminal_session_fingerprint(
        session.execution_mode,
        &session.docker_image,
        &session.shell,
        &session.working_dir,
    );

    let Some(original_command) = original_command else {
        return Ok(json!({
            "session_id": session.session_id,
            "process_id": session.session_id,
            "session_fingerprint": session_fingerprint,
            "execution_mode": execution_mode_label(session.execution_mode),
            "docker_image": session.docker_image,
            "shell": session.shell,
            "working_dir": session.working_dir,
            "completed": false,
            "message": "Connected to terminal session",
            "instructions": "Use write_stdin with this session_id to send input or poll output."
        }));
    };

    let decoded_command = decode_transport_html_entities(&original_command);
    let command_was_html_decoded = decoded_command != original_command;
    let skip_normalize = args
        .get("skip_normalize")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (command, was_normalized) = if skip_normalize {
        (decoded_command.clone(), false)
    } else {
        normalize_command(&decoded_command)
    };

    if session.execution_mode == ExecutionMode::Host {
        check_shell_permission(&command, execution_id)
            .await
            .map_err(|err| format!("Permission denied: {}", err))?;
    }

    let start_cursor = TERMINAL_MANAGER
        .session_output_cursor(&session.session_id)
        .await?;
    TERMINAL_MANAGER
        .write_to_session(&session.session_id, format!("{}\n", command).into_bytes())
        .await?;

    let options = if codex_exec_shape {
        collect_options_from_exec_args(&args)
    } else {
        collect_options_from_interactive_args(&args)
    };
    let collected = collect_output_since(&session.session_id, start_cursor, &options).await?;
    set_cursor(execution_id, &session.session_id, collected.next_cursor).await;

    let raw_output = String::from_utf8_lossy(&collected.output).to_string();
    let clean_output =
        truncate_for_tokens(sanitize_interactive_output(&raw_output, &command), options.max_output_tokens);
    let mut result = json!({
        "session_id": session.session_id,
        "process_id": session.session_id,
        "session_fingerprint": session_fingerprint,
        "execution_mode": execution_mode_label(session.execution_mode),
        "docker_image": session.docker_image,
        "shell": session.shell,
        "working_dir": session.working_dir,
        "command": command,
        "output": clean_output,
        "completed": collected.completed,
        "truncated": collected.timed_out && !collected.completed,
        "exit_code": Value::Null,
        "output_cursor": collected.next_cursor,
    });

    if codex_exec_shape {
        result["raw_output"] = result["output"].clone();
    }
    if command_was_html_decoded {
        result["input_command"] = json!(original_command);
        result["decoded_from_html_entities"] = json!(true);
    }
    if was_normalized {
        result["original_command"] = json!(decoded_command);
    }
    if collected.timed_out && !collected.completed {
        result["hint"] = json!(
            "The command is still running or waiting for input. Use write_stdin with the returned session_id to send input or poll output."
        );
    }

    Ok(result)
}

pub async fn execute_interactive_shell(
    args: Value,
    execution_id_override: Option<String>,
    fallback_active_session_id: Option<String>,
    working_directory: Option<String>,
) -> Result<Value, String> {
    run_terminal_command(
        args,
        execution_id_override,
        fallback_active_session_id,
        working_directory,
        false,
    )
    .await
}

pub async fn execute_exec_command(
    args: Value,
    execution_id_override: Option<String>,
    working_directory: Option<String>,
) -> Result<Value, String> {
    run_terminal_command(args, execution_id_override, None, working_directory, true).await
}

pub async fn execute_write_stdin(
    args: Value,
    execution_id_override: Option<String>,
) -> Result<Value, String> {
    let execution_id = execution_id_override
        .as_deref()
        .or_else(|| args.get("execution_id").and_then(Value::as_str));
    let session_id = args
        .get("session_id")
        .or_else(|| args.get("process_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "write_stdin requires session_id".to_string())?;
    let input = args
        .get("chars")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("input_text"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    let input_is_empty = input.is_empty();
    let start_cursor = if input_is_empty {
        match get_cursor(execution_id, session_id).await {
            Some(cursor) => cursor,
            None => TERMINAL_MANAGER.session_output_cursor(session_id).await?,
        }
    } else {
        TERMINAL_MANAGER.session_output_cursor(session_id).await?
    };

    if !input_is_empty {
        TERMINAL_MANAGER
            .write_to_session(session_id, input.as_bytes().to_vec())
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let options = collect_options_from_write_args(&args, input_is_empty);
    let collected = collect_output_since(session_id, start_cursor, &options).await?;
    set_cursor(execution_id, session_id, collected.next_cursor).await;

    let raw_output = String::from_utf8_lossy(&collected.output).to_string();
    let clean_output = truncate_for_tokens(raw_output, options.max_output_tokens);
    Ok(json!({
        "session_id": session_id,
        "process_id": session_id,
        "output": clean_output,
        "completed": collected.completed,
        "truncated": collected.timed_out && !collected.completed,
        "exit_code": Value::Null,
        "output_cursor": collected.next_cursor,
    }))
}

fn terminal_execution_policy() -> ToolExecutionPolicy {
    ToolExecutionPolicy {
        read_only: false,
        mutating: true,
        concurrency_safe: false,
        requires_permission: true,
        supports_background: true,
    }
}

fn interactive_shell_description() -> String {
    let base = concat!(
        "Interactive shell for iterative terminal work. ",
        "Starts or reuses a persistent PTY session, returns after a bounded wait, ",
        "and includes session_id/process_id for write_stdin. "
    );
    format!(
        "{}Use this for prompts, REPLs, TUIs, installers, scaffolding commands, and long-lived processes.",
        base
    )
}

pub fn build_interactive_shell_tool_def() -> DynamicToolDef {
    DynamicToolBuilder::new(TerminalServer::NAME.to_string())
        .description(interactive_shell_description())
        .input_schema(json!({
            "type": "object",
            "properties": {
                "execution_mode": {
                    "type": "string",
                    "enum": ["docker", "host"],
                    "description": "Execution mode: docker or host."
                },
                "docker_image": {
                    "type": "string",
                    "description": "Docker image to use when execution_mode is docker."
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for host sessions. Docker sessions use /workspace."
                },
                "command": {
                    "type": "string",
                    "description": "Command or stdin line to send to the terminal session."
                },
                "session_policy": {
                    "type": "string",
                    "enum": ["reuse", "new"],
                    "description": "reuse uses the active session; new starts a fresh session.",
                    "default": "reuse"
                },
                "wait_strategy": {
                    "type": "string",
                    "enum": ["auto", "prompt", "timeout", "lines"],
                    "description": "How to wait for output.",
                    "default": "auto"
                },
                "wait_timeout": {
                    "type": "integer",
                    "description": "Maximum wait time in seconds.",
                    "default": 30
                },
                "yield_time_ms": {
                    "type": "integer",
                    "description": "Codex-compatible bounded wait time in milliseconds."
                },
                "expected_lines": {
                    "type": "integer",
                    "description": "For lines strategy: number of output lines to wait for."
                },
                "skip_normalize": {
                    "type": "boolean",
                    "description": "Skip command normalization.",
                    "default": false
                },
                "max_output_tokens": {
                    "type": "integer",
                    "description": "Approximate maximum output tokens to return."
                }
            }
        }))
        .source(ToolSource::Builtin)
        .category(ToolCategory::System)
        .tags(vec![
            "terminal".to_string(),
            "shell".to_string(),
            "interactive".to_string(),
        ])
        .search_hint("run interactive terminal commands and continue them with write_stdin")
        .execution_policy(terminal_execution_policy())
        .executor(|args| async move { execute_interactive_shell(args, None, None, None).await })
        .build()
        .expect("Failed to build interactive_shell tool")
}

pub fn build_exec_command_tool_def() -> DynamicToolDef {
    DynamicToolBuilder::new(EXEC_COMMAND_TOOL_NAME.to_string())
        .description(concat!(
            "Codex-compatible command execution. Starts a PTY-backed shell session, ",
            "returns after yield_time_ms instead of waiting forever, and returns session_id/process_id ",
            "for write_stdin when the command is still running or waiting for input."
        ))
        .input_schema(json!({
            "type": "object",
            "properties": {
                "cmd": {"type": "string", "description": "Shell command to execute."},
                "workdir": {"type": "string", "description": "Working directory for host execution."},
                "execution_mode": {"type": "string", "enum": ["docker", "host"]},
                "yield_time_ms": {"type": "integer", "default": DEFAULT_EXEC_YIELD_TIME_MS},
                "max_output_tokens": {"type": "integer"},
                "tty": {"type": "boolean", "default": true}
            },
            "required": ["cmd"]
        }))
        .source(ToolSource::Builtin)
        .category(ToolCategory::System)
        .tags(vec!["terminal".to_string(), "shell".to_string()])
        .search_hint("codex-compatible bounded shell execution")
        .execution_policy(terminal_execution_policy())
        .executor(|args| async move { execute_exec_command(args, None, None).await })
        .build()
        .expect("Failed to build exec_command tool")
}

pub fn build_write_stdin_tool_def() -> DynamicToolDef {
    DynamicToolBuilder::new(WRITE_STDIN_TOOL_NAME.to_string())
        .description(concat!(
            "Codex-compatible stdin writer and output poller for a running terminal session. ",
            "Pass chars to send input, or chars=\"\" to poll buffered output."
        ))
        .input_schema(json!({
            "type": "object",
            "properties": {
                "session_id": {"type": "string", "description": "Session id returned by exec_command or interactive_shell."},
                "process_id": {"type": "string", "description": "Alias for session_id."},
                "chars": {"type": "string", "description": "Characters to write. Include newline when submitting a prompt answer.", "default": ""},
                "yield_time_ms": {"type": "integer", "default": DEFAULT_WRITE_STDIN_YIELD_TIME_MS},
                "max_output_tokens": {"type": "integer"}
            }
        }))
        .source(ToolSource::Builtin)
        .category(ToolCategory::System)
        .tags(vec!["terminal".to_string(), "stdin".to_string()])
        .search_hint("send input to an exec_command or interactive_shell session")
        .execution_policy(terminal_execution_policy())
        .executor(|args| async move { execute_write_stdin(args, None).await })
        .build()
        .expect("Failed to build write_stdin tool")
}
