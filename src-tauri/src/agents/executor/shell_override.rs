use std::sync::Arc;
use std::time::Instant;

use sentinel_tools::buildin_tools::ShellTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolCategory, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;
use serde_json::json;

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
        "suggested_action": "Use interactive_shell to continue in a persistent terminal session, or rerun the command with non-interactive flags or piped input.",
        "backgrounded": false,
        "background_task_id": serde_json::Value::Null,
        "background_session_id": serde_json::Value::Null,
        "background_status": serde_json::Value::Null,
        "note": serde_json::Value::Null,
        "stored_artifacts": [],
    })
}

pub(super) async fn build_shell_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let shell_info = tool_server.get_tool(ShellTool::NAME).await?;
    let execution_id_for_shell = execution_id.to_string();
    let shell_input_schema = shell_info.input_schema.clone();
    let shell_description = shell_info.description.clone();
    let shell_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_shell = execution_id_for_shell.clone();
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

            let raw_command = patched_args
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
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
        category: ToolCategory::System,
        tags: shell_info.tags.clone(),
        search_hint: shell_info.search_hint.clone(),
        exposure: shell_info.exposure.clone(),
        execution_policy: shell_info.execution_policy.clone(),
        executor: shell_executor,
    })
}
