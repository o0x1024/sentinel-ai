use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use crate::buildin_tools::shell::ShellExecutionMode;
use crate::terminal::{
    detect_shell_prompt, ExecutionMode, SessionState, TerminalSessionConfig, TERMINAL_MANAGER,
};

const DEFAULT_DOCKER_IMAGE: &str = "sentinel-sandbox:latest";
const PREVIEW_LIMIT_CHARS: usize = 4000;
const MONITOR_BUFFER_LIMIT_CHARS: usize = 64_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundShellTaskStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundShellTaskRecord {
    pub id: String,
    pub execution_id: Option<String>,
    pub session_id: String,
    pub command: String,
    pub wrapped_command: String,
    pub status: BackgroundShellTaskStatus,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub output_preview: String,
    pub execution_mode: String,
}

#[derive(Debug, Clone)]
pub struct LaunchBackgroundShellTaskRequest {
    pub execution_id: Option<String>,
    pub command: String,
    pub cwd: Option<String>,
    pub execution_mode: ShellExecutionMode,
    pub docker_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundShellLaunch {
    pub task_id: String,
    pub session_id: String,
    pub execution_mode: String,
}

static SHELL_BACKGROUND_APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> =
    Lazy::new(|| RwLock::new(None));
static SHELL_BACKGROUND_TASKS: Lazy<RwLock<HashMap<String, BackgroundShellTaskRecord>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn set_shell_background_app_handle(handle: AppHandle) {
    let mut guard = SHELL_BACKGROUND_APP_HANDLE.write().await;
    *guard = Some(handle);
}

fn unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_terminal_execution_mode(mode: ShellExecutionMode) -> ExecutionMode {
    match mode {
        ShellExecutionMode::Host => ExecutionMode::Host,
        ShellExecutionMode::Docker => ExecutionMode::Docker,
    }
}

fn strip_ansi_codes(text: &str) -> String {
    let re = regex::Regex::new(
        r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\][0-9;]*[^\x07]*\x07|\x1b[=>]|\x1b\][0-9];[^\x07]*\x07",
    )
    .unwrap();
    re.replace_all(text, "").to_string()
}

fn trim_preview(text: &str) -> String {
    let normalized = strip_ansi_codes(text).replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.chars().count() <= PREVIEW_LIMIT_CHARS {
        return trimmed.to_string();
    }
    trimmed.chars().take(PREVIEW_LIMIT_CHARS).collect()
}

fn build_wrapped_command(command: &str, task_id: &str) -> (String, String, String) {
    let begin_marker = format!("__SENTINEL_BG_BEGIN_{}__", task_id);
    let exit_prefix = format!("__SENTINEL_BG_EXIT_{}__:", task_id);
    let wrapped = format!(
        "printf '%s\\n' '{begin}'; {{ {command}; }}; __sentinel_bg_code=$?; printf '\\n%s%d\\n' '{exit_prefix}' \"$__sentinel_bg_code\"",
        begin = begin_marker,
        command = command,
        exit_prefix = exit_prefix,
    );
    (wrapped, begin_marker, exit_prefix)
}

async fn emit_background_task_event(record: &BackgroundShellTaskRecord) {
    let guard = SHELL_BACKGROUND_APP_HANDLE.read().await;
    let Some(app) = guard.as_ref() else {
        return;
    };
    let _ = app.emit("shell-background-task-update", record);
}

fn parse_exit_code(output: &str, exit_prefix: &str) -> Option<i32> {
    let marker_pos = output.rfind(exit_prefix)?;
    let suffix = &output[marker_pos + exit_prefix.len()..];
    let digits: String = suffix
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit() && *ch != '-')
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect();
    digits.parse::<i32>().ok()
}

async fn update_task_record(
    task_id: &str,
    mut update: impl FnMut(&mut BackgroundShellTaskRecord),
) -> Option<BackgroundShellTaskRecord> {
    let mut tasks = SHELL_BACKGROUND_TASKS.write().await;
    let record = tasks.get_mut(task_id)?;
    update(record);
    Some(record.clone())
}

async fn monitor_background_task(
    task_id: String,
    session_id: String,
    mut output_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    begin_marker: String,
    exit_prefix: String,
) {
    let mut combined_output = String::new();
    let mut saw_begin = false;

    while let Some(chunk) = output_rx.recv().await {
        let text = String::from_utf8_lossy(&chunk);
        combined_output.push_str(&text);
        if combined_output.len() > MONITOR_BUFFER_LIMIT_CHARS {
            let drain_len = combined_output.len() - MONITOR_BUFFER_LIMIT_CHARS;
            combined_output.drain(..drain_len);
        }

        if !saw_begin && combined_output.contains(&begin_marker) {
            saw_begin = true;
        }

        if let Some(exit_code) = parse_exit_code(&combined_output, &exit_prefix) {
            let preview = trim_preview(
                &combined_output
                    .replace(&begin_marker, "")
                    .replace(&format!("{}{}", exit_prefix, exit_code), ""),
            );
            if let Some(record) = update_task_record(&task_id, |record| {
                record.status = if exit_code == 0 {
                    BackgroundShellTaskStatus::Completed
                } else {
                    BackgroundShellTaskStatus::Failed
                };
                record.exit_code = Some(exit_code);
                record.finished_at = Some(unix_timestamp_secs());
                record.output_preview = preview.clone();
            })
            .await
            {
                emit_background_task_event(&record).await;
            }
            return;
        }

        if saw_begin && detect_shell_prompt(&combined_output) {
            let preview = trim_preview(&combined_output.replace(&begin_marker, ""));
            if let Some(record) = update_task_record(&task_id, |record| {
                if record.status == BackgroundShellTaskStatus::Running {
                    record.status = BackgroundShellTaskStatus::Completed;
                    record.exit_code = Some(0);
                    record.finished_at = Some(unix_timestamp_secs());
                    record.output_preview = preview.clone();
                }
            })
            .await
            {
                emit_background_task_event(&record).await;
            }
            return;
        }
    }

    let final_state = if let Some(session) = TERMINAL_MANAGER.get_session(&session_id).await {
        let session = session.read().await;
        Some(session.state().await)
    } else {
        None
    };

    let preview = trim_preview(&combined_output.replace(&begin_marker, ""));
    if let Some(record) = update_task_record(&task_id, |record| {
        if record.status == BackgroundShellTaskStatus::Running {
            record.status = if final_state == Some(SessionState::Stopped) {
                BackgroundShellTaskStatus::Cancelled
            } else {
                BackgroundShellTaskStatus::Completed
            };
            record.finished_at = Some(unix_timestamp_secs());
            record.output_preview = preview.clone();
        }
    })
    .await
    {
        emit_background_task_event(&record).await;
    }
}

pub async fn launch_background_shell_task(
    request: LaunchBackgroundShellTaskRequest,
) -> Result<BackgroundShellLaunch, String> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let (wrapped_command, begin_marker, exit_prefix) =
        build_wrapped_command(&request.command, &task_id);

    let mut session_config = TerminalSessionConfig::default();
    session_config.execution_mode = to_terminal_execution_mode(request.execution_mode.clone());
    session_config.docker_image = request
        .docker_image
        .unwrap_or_else(|| DEFAULT_DOCKER_IMAGE.to_string());
    session_config.working_dir = request.cwd;
    session_config.shell = "bash".to_string();
    session_config.initial_command = Some(wrapped_command.clone());
    session_config.reuse_container = true;
    session_config.container_name = Some("sentinel-sandbox-main".to_string());

    let (session_id, output_rx) = TERMINAL_MANAGER.create_session(session_config).await?;
    let record = BackgroundShellTaskRecord {
        id: task_id.clone(),
        execution_id: request.execution_id.clone(),
        session_id: session_id.clone(),
        command: request.command.clone(),
        wrapped_command: wrapped_command.clone(),
        status: BackgroundShellTaskStatus::Running,
        started_at: unix_timestamp_secs(),
        finished_at: None,
        exit_code: None,
        output_preview: String::new(),
        execution_mode: match request.execution_mode {
            ShellExecutionMode::Docker => "docker".to_string(),
            ShellExecutionMode::Host => "host".to_string(),
        },
    };

    {
        let mut tasks = SHELL_BACKGROUND_TASKS.write().await;
        tasks.insert(task_id.clone(), record.clone());
    }
    emit_background_task_event(&record).await;

    tokio::spawn(monitor_background_task(
        task_id.clone(),
        session_id.clone(),
        output_rx,
        begin_marker,
        exit_prefix,
    ));

    Ok(BackgroundShellLaunch {
        task_id,
        session_id,
        execution_mode: record.execution_mode,
    })
}

pub async fn list_background_shell_tasks(
    execution_id: Option<&str>,
) -> Vec<BackgroundShellTaskRecord> {
    let tasks = SHELL_BACKGROUND_TASKS.read().await;
    let mut items: Vec<BackgroundShellTaskRecord> = tasks.values().cloned().collect();
    if let Some(id) = execution_id {
        items.retain(|item| item.execution_id.as_deref() == Some(id));
    }
    items.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    items
}

pub async fn stop_background_shell_task(task_id: &str) -> Result<(), String> {
    let session_id = {
        let tasks = SHELL_BACKGROUND_TASKS.read().await;
        tasks.get(task_id)
            .map(|item| item.session_id.clone())
            .ok_or_else(|| format!("background shell task {} not found", task_id))?
    };

    TERMINAL_MANAGER.stop_session(&session_id).await?;

    if let Some(record) = update_task_record(task_id, |record| {
        record.status = BackgroundShellTaskStatus::Cancelled;
        record.finished_at = Some(unix_timestamp_secs());
    })
    .await
    {
        emit_background_task_event(&record).await;
    }

    Ok(())
}
