use std::sync::Arc;
use std::time::Instant;

use sentinel_tools::buildin_tools::ShellTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolCategory, ToolExecutor, ToolSource};
use sentinel_tools::terminal::unified_exec_tool::{
    execute_shell_session_command, execute_shell_session_input,
};
use sentinel_tools::ToolServer;
use serde_json::json;

use crate::agents::executor::terminal_session_store::{
    get_active_terminal_session, set_active_terminal_session,
};

fn shell_execution_mode_label(
    mode: Option<&sentinel_tools::buildin_tools::shell::ShellExecutionMode>,
) -> String {
    match mode {
        Some(sentinel_tools::buildin_tools::shell::ShellExecutionMode::Host) => "host".to_string(),
        Some(sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker) => {
            "docker".to_string()
        }
        None => String::new(),
    }
}

fn build_shell_failure_result(
    command: String,
    execution_mode: String,
    error: String,
    execution_time_ms: u64,
) -> serde_json::Value {
    json!({
        "command": command,
        "stdout": "",
        "stderr": error.clone(),
        "exit_code": serde_json::Value::Null,
        "completed": false,
        "success": false,
        "execution_time_ms": execution_time_ms,
        "execution_mode": execution_mode,
        "error": error,
        "interaction_required": false,
        "interaction_kind": serde_json::Value::Null,
        "recommended_tool": serde_json::Value::Null,
        "suggested_action": serde_json::Value::Null,
        "backgrounded": false,
        "background_task_id": serde_json::Value::Null,
        "background_session_id": serde_json::Value::Null,
        "background_status": serde_json::Value::Null,
        "note": serde_json::Value::Null,
        "stored_artifacts": [],
    })
}

fn build_shell_timeout_failure_result(
    command: String,
    execution_mode: String,
    stdout: String,
    stderr: String,
    timeout_secs: u64,
    execution_time_ms: u64,
) -> serde_json::Value {
    json!({
        "command": command,
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": serde_json::Value::Null,
        "completed": false,
        "success": false,
        "execution_time_ms": execution_time_ms,
        "execution_mode": execution_mode,
        "error": format!("Command timeout after {} seconds", timeout_secs),
        "interaction_required": false,
        "interaction_kind": serde_json::Value::Null,
        "recommended_tool": serde_json::Value::Null,
        "suggested_action": serde_json::Value::Null,
        "backgrounded": false,
        "background_task_id": serde_json::Value::Null,
        "background_session_id": serde_json::Value::Null,
        "background_status": serde_json::Value::Null,
        "note": serde_json::Value::Null,
        "stored_artifacts": [],
    })
}

fn build_shell_interaction_failure_result(
    command: String,
    execution_mode: String,
    message: String,
    stdout: String,
    stderr: String,
    interaction_kind: String,
    recommended_tool: String,
    execution_time_ms: u64,
) -> serde_json::Value {
    json!({
        "command": command,
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": serde_json::Value::Null,
        "completed": false,
        "success": false,
        "execution_time_ms": execution_time_ms,
        "execution_mode": execution_mode,
        "error": message,
        "interaction_required": true,
        "interaction_kind": interaction_kind,
        "recommended_tool": recommended_tool,
        "suggested_action": "Call shell with yield_time_ms to start a prompt-capable session. Poll with action=poll, write raw stdin with action=write and chars, send terminal keys with action=key and key, or confirm prompts with action=submit.",
        "backgrounded": false,
        "background_task_id": serde_json::Value::Null,
        "background_session_id": serde_json::Value::Null,
        "background_status": serde_json::Value::Null,
        "note": serde_json::Value::Null,
        "stored_artifacts": [],
    })
}

fn has_non_empty_string(args: &serde_json::Value, key: &str) -> bool {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn has_shell_stdin_key(args: &serde_json::Value) -> bool {
    let Some(obj) = args.as_object() else {
        return false;
    };
    obj.contains_key("chars") || obj.contains_key("input") || obj.contains_key("input_text")
}

fn shell_call_has_user_operation(args: &serde_json::Value) -> bool {
    has_non_empty_string(args, "command")
        || has_non_empty_string(args, "cmd")
        || has_non_empty_string(args, "session_id")
        || has_non_empty_string(args, "process_id")
        || has_shell_stdin_key(args)
        || has_non_empty_string(args, "action")
}

fn should_continue_shell_session(args: &serde_json::Value) -> bool {
    has_non_empty_string(args, "session_id") || has_non_empty_string(args, "process_id")
}

fn has_true_bool(args: &serde_json::Value, key: &str) -> bool {
    args.get(key).and_then(|value| value.as_bool()) == Some(true)
}

fn should_start_prompt_capable_shell(args: &serde_json::Value) -> bool {
    const START_ACTION: &str = "start";

    if !(has_non_empty_string(args, "command") || has_non_empty_string(args, "cmd")) {
        return false;
    }

    if has_non_empty_string(args, "cmd") {
        return true;
    }

    if args.get("yield_time_ms").is_some() {
        return true;
    }

    if has_true_bool(args, "tty") || has_true_bool(args, "run_in_background") {
        return true;
    }

    args.get("action")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().eq_ignore_ascii_case(START_ACTION))
        .unwrap_or(false)
}

fn normalize_prompt_shell_args(args: &mut serde_json::Value) {
    let Some(obj) = args.as_object_mut() else {
        return;
    };
    if !obj.contains_key("cmd") {
        if let Some(command) = obj.get("command").cloned() {
            obj.insert("cmd".to_string(), command);
        }
    }
    if !obj.contains_key("workdir") {
        if let Some(cwd) = obj.get("cwd").cloned() {
            obj.insert("workdir".to_string(), cwd);
        }
    }
}

fn strip_prompt_shell_only_args(args: &mut serde_json::Value) {
    let Some(obj) = args.as_object_mut() else {
        return;
    };
    obj.remove("cmd");
    obj.remove("workdir");
    obj.remove("yield_time_ms");
    obj.remove("max_output_tokens");
    obj.remove("tty");
    obj.remove("session_id");
    obj.remove("process_id");
    obj.remove("chars");
}

fn patch_active_shell_session_input_args(
    args: &mut serde_json::Value,
    execution_id: &str,
) -> Result<bool, String> {
    if should_continue_shell_session(args) {
        return Ok(true);
    }

    if should_start_prompt_capable_shell(args) {
        return Ok(false);
    }

    let has_stdin = has_shell_stdin_key(args);
    let has_action = has_non_empty_string(args, "action");
    let is_empty_shell_call = !shell_call_has_user_operation(args);
    if !has_stdin && !has_action && !is_empty_shell_call {
        return Ok(false);
    }

    let Some(active_session_id) = get_active_terminal_session(execution_id) else {
        return Err(
            "shell input was provided, but no active shell session is available. Start a command with {\"command\":\"...\"}, or include session_id/process_id when continuing a previous command."
                .to_string(),
        );
    };

    let Some(obj) = args.as_object_mut() else {
        return Ok(false);
    };
    obj.insert(
        "session_id".to_string(),
        serde_json::Value::String(active_session_id),
    );
    Ok(true)
}

fn sync_active_shell_session_from_result(execution_id: &str, output: &serde_json::Value) {
    if output
        .get("completed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        set_active_terminal_session(execution_id, None);
        return;
    }

    if let Some(session_id) = output.get("session_id").and_then(|value| value.as_str()) {
        set_active_terminal_session(execution_id, Some(session_id));
    }
}

pub(super) async fn build_shell_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
    working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let shell_info = tool_server.get_tool(ShellTool::NAME).await?;
    let execution_id_for_shell = execution_id.to_string();
    let working_directory = working_directory.map(str::to_string);
    let shell_input_schema = shell_info.input_schema.clone();
    let shell_description = shell_info.description.clone();
    let shell_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_shell = execution_id_for_shell.clone();
        let working_directory = working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellError, ShellTool};

            let started_at = Instant::now();

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_shell.clone()),
                );
                obj.insert(
                    "enable_large_output_storage".to_string(),
                    serde_json::Value::Bool(true),
                );
            }

            match patch_active_shell_session_input_args(&mut patched_args, &execution_id_for_shell)
            {
                Ok(true) => {
                    let output = execute_shell_session_input(
                        patched_args,
                        Some(execution_id_for_shell.clone()),
                    )
                    .await?;
                    sync_active_shell_session_from_result(&execution_id_for_shell, &output);
                    return Ok(output);
                }
                Ok(false) => {}
                Err(error) => {
                    return Ok(build_shell_failure_result(
                        String::new(),
                        String::new(),
                        error,
                        started_at.elapsed().as_millis() as u64,
                    ));
                }
            }

            if should_start_prompt_capable_shell(&patched_args) {
                normalize_prompt_shell_args(&mut patched_args);
                let output = execute_shell_session_command(
                    patched_args,
                    Some(execution_id_for_shell.clone()),
                    working_directory,
                )
                .await?;
                sync_active_shell_session_from_result(&execution_id_for_shell, &output);
                return Ok(output);
            }

            let raw_command = patched_args
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            strip_prompt_shell_only_args(&mut patched_args);
            let tool_args: ShellArgs = match serde_json::from_value(patched_args) {
                Ok(args) => args,
                Err(e) => {
                    return Ok(build_shell_failure_result(
                        raw_command,
                        String::new(),
                        format!("Invalid arguments: {}", e),
                        started_at.elapsed().as_millis() as u64,
                    ));
                }
            };
            let execution_mode = shell_execution_mode_label(tool_args.execution_mode.as_ref());

            let tool = ShellTool::new();
            let result = match tool.call(tool_args.clone()).await {
                Ok(result) => result,
                Err(e) => match e {
                    ShellError::InteractionRequired {
                        message,
                        stdout,
                        stderr,
                        interaction_kind,
                        recommended_tool,
                    } => {
                        return Ok(build_shell_interaction_failure_result(
                            tool_args.command,
                            execution_mode,
                            message,
                            stdout,
                            stderr,
                            interaction_kind,
                            recommended_tool,
                            started_at.elapsed().as_millis() as u64,
                        ));
                    }
                    ShellError::TimeoutWithOutput {
                        timeout_secs,
                        stdout,
                        stderr,
                    } => {
                        return Ok(build_shell_timeout_failure_result(
                            tool_args.command,
                            execution_mode,
                            stdout,
                            stderr,
                            timeout_secs,
                            started_at.elapsed().as_millis() as u64,
                        ));
                    }
                    other => {
                        return Ok(build_shell_failure_result(
                            tool_args.command,
                            execution_mode,
                            format!("Shell execution failed: {}", other),
                            started_at.elapsed().as_millis() as u64,
                        ));
                    }
                },
            };

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize shell result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: ShellTool::NAME.to_string(),
        description: shell_description,
        input_schema: shell_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: ToolCategory::Terminal,
        tags: shell_info.tags.clone(),
        search_hint: shell_info.search_hint.clone(),
        exposure: shell_info.exposure.clone(),
        execution_policy: shell_info.execution_policy.clone(),
        executor: shell_executor,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        patch_active_shell_session_input_args, should_start_prompt_capable_shell,
        sync_active_shell_session_from_result,
    };
    use crate::agents::executor::terminal_session_store::{
        get_active_terminal_session, set_active_terminal_session,
    };
    use serde_json::json;

    #[test]
    fn patches_stdin_args_with_active_shell_session() {
        set_active_terminal_session("shell-exec-stdin", Some("session-1"));
        let mut args = json!({ "chars": "\n" });

        assert!(patch_active_shell_session_input_args(&mut args, "shell-exec-stdin").unwrap());
        assert_eq!(
            args.get("session_id").and_then(|value| value.as_str()),
            Some("session-1")
        );
        assert_eq!(
            args.get("chars").and_then(|value| value.as_str()),
            Some("\n")
        );

        set_active_terminal_session("shell-exec-stdin", None);
    }

    #[test]
    fn patches_empty_shell_call_as_poll_for_active_shell_session() {
        set_active_terminal_session("shell-exec-empty", Some("session-2"));
        let mut args = json!({
            "execution_id": "shell-exec-empty",
            "enable_large_output_storage": true
        });

        assert!(patch_active_shell_session_input_args(&mut args, "shell-exec-empty").unwrap());
        assert_eq!(
            args.get("session_id").and_then(|value| value.as_str()),
            Some("session-2")
        );
        assert!(args.get("chars").is_none());

        set_active_terminal_session("shell-exec-empty", None);
    }

    #[test]
    fn patches_action_args_with_active_shell_session() {
        set_active_terminal_session("shell-exec-action", Some("session-4"));
        let mut args = json!({ "action": "poll" });

        assert!(patch_active_shell_session_input_args(&mut args, "shell-exec-action").unwrap());
        assert_eq!(
            args.get("session_id").and_then(|value| value.as_str()),
            Some("session-4")
        );
        assert_eq!(
            args.get("action").and_then(|value| value.as_str()),
            Some("poll")
        );

        set_active_terminal_session("shell-exec-action", None);
    }

    #[test]
    fn stdin_args_without_active_shell_session_return_error() {
        set_active_terminal_session("shell-exec-missing", None);
        let mut args = json!({ "chars": "\n" });

        let error = patch_active_shell_session_input_args(&mut args, "shell-exec-missing")
            .expect_err("missing active session should be an execution-layer error");
        assert!(error.contains("no active shell session"));
    }

    #[test]
    fn sync_active_session_tracks_running_and_clears_completed_shell_output() {
        set_active_terminal_session("shell-exec-sync", None);
        sync_active_shell_session_from_result(
            "shell-exec-sync",
            &json!({ "session_id": "session-3", "completed": false }),
        );
        assert_eq!(
            get_active_terminal_session("shell-exec-sync").as_deref(),
            Some("session-3")
        );

        sync_active_shell_session_from_result(
            "shell-exec-sync",
            &json!({ "completed": true, "status": "completed" }),
        );
        assert!(get_active_terminal_session("shell-exec-sync").is_none());
    }

    #[test]
    fn command_only_shell_calls_use_one_shot_path() {
        assert!(!should_start_prompt_capable_shell(&json!({
            "command": "printf short-ok"
        })));
    }

    #[test]
    fn explicit_interactive_shell_markers_use_prompt_capable_path() {
        assert!(should_start_prompt_capable_shell(&json!({
            "command": "cat",
            "yield_time_ms": 100
        })));
        assert!(should_start_prompt_capable_shell(&json!({
            "command": "cat",
            "tty": true
        })));
        assert!(should_start_prompt_capable_shell(&json!({
            "command": "sleep 10",
            "run_in_background": true
        })));
        assert!(should_start_prompt_capable_shell(&json!({
            "cmd": "cat"
        })));
    }
}
