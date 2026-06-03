use anyhow::{Context, Result};
use chrono::Utc;
use sentinel_llm::ChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChallengeRunState {
    pub code: String,
    pub title: Option<String>,
    pub last_started_at: Option<String>,
    pub attempt_deadline_at: Option<String>,
    pub last_finished_at: Option<String>,
    pub last_status: Option<String>,
    pub current_attempt_id: Option<String>,
    pub current_step: Option<usize>,
    pub hint_used: bool,
    pub last_reason: Option<String>,
    pub last_hint_content: Option<String>,
    pub context_summary: Option<String>,
    pub recent_history: Vec<ChatMessage>,
    pub entrypoints: Vec<String>,
    pub tool_calls_count: usize,
    pub watchdog_stall_count: usize,
    pub watchdog_repeat_count: usize,
    pub watchdog_last_tool_signature: Option<String>,
    pub watchdog_last_result_signature: Option<String>,
    pub watchdog_interventions: usize,
    pub submitted_flags: BTreeSet<String>,
    pub accepted_flags: BTreeSet<String>,
    pub discovered_flags: BTreeSet<String>,
    pub probed_urls: BTreeSet<String>,
    pub total_attempts: usize,
    pub consecutive_failures: usize,
    pub next_eligible_at: Option<String>,
    pub last_progress_at: Option<String>,
}

#[derive(Clone)]
pub struct RuntimeStateStore {
    root_dir: PathBuf,
}

impl RuntimeStateStore {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub async fn load_challenge(&self, code: &str) -> Result<ChallengeRunState> {
        let path = self.challenge_path(code);
        match fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(state) => Ok(state),
                Err(initial_error) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    let retry_content = fs::read_to_string(&path).await.with_context(|| {
                        format!("failed to reread challenge state: {}", path.display())
                    })?;
                    serde_json::from_str(&retry_content).with_context(|| {
                        format!(
                            "failed to parse challenge state: {} ({})",
                            path.display(),
                            initial_error
                        )
                    })
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ChallengeRunState {
                code: code.to_string(),
                ..ChallengeRunState::default()
            }),
            Err(error) => Err(error)
                .with_context(|| format!("failed to read challenge state: {}", path.display())),
        }
    }

    pub async fn save_challenge(&self, state: &ChallengeRunState) -> Result<()> {
        let path = self.challenge_path(&state.code);
        let content =
            serde_json::to_string_pretty(state).context("failed to serialize challenge state")?;
        atomic_write(&path, content.as_bytes())
            .await
            .with_context(|| {
                format!(
                    "failed to write challenge state atomically: {}",
                    path.display()
                )
            })?;
        Ok(())
    }

    pub async fn mark_challenge_terminal(
        &self,
        code: &str,
        status: &str,
        reason: Option<String>,
    ) -> Result<ChallengeRunState> {
        let mut state = self.load_challenge(code).await?;
        state.last_finished_at = Some(Utc::now().to_rfc3339());
        state.last_status = Some(status.to_string());
        state.current_attempt_id = None;
        state.current_step = None;
        state.attempt_deadline_at = None;
        state.last_reason = reason;
        state.last_progress_at = Some(Utc::now().to_rfc3339());
        state.last_hint_content = None;
        state.recent_history.clear();
        state.watchdog_stall_count = 0;
        state.watchdog_repeat_count = 0;
        state.watchdog_last_tool_signature = None;
        state.watchdog_last_result_signature = None;
        self.save_challenge(&state).await?;
        Ok(state)
    }

    pub async fn append_event<T>(&self, event_type: &str, payload: &T) -> Result<()>
    where
        T: Serialize,
    {
        let path = self.logs_dir().join("events.jsonl");
        fs::create_dir_all(self.logs_dir())
            .await
            .with_context(|| format!("failed to create logs dir: {}", self.logs_dir().display()))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .with_context(|| format!("failed to open event log: {}", path.display()))?;

        let line = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event_type,
            "payload": payload,
        });
        file.write_all(serde_json::to_string(&line)?.as_bytes())
            .await
            .with_context(|| format!("failed to append event log: {}", path.display()))?;
        file.write_all(b"\n")
            .await
            .with_context(|| format!("failed to append newline: {}", path.display()))?;
        Ok(())
    }

    pub async fn append_attempt_event<T>(
        &self,
        code: &str,
        attempt_id: &str,
        event_type: &str,
        payload: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        let path = self
            .attempt_logs_dir(code)
            .join(format!("{}.jsonl", sanitize_file_component(attempt_id)));
        self.append_jsonl_line(
            &path,
            &serde_json::json!({
                "ts": Utc::now().to_rfc3339(),
                "event": event_type,
                "payload": payload,
            }),
        )
        .await
    }

    pub async fn append_tool_trace<T>(
        &self,
        code: &str,
        attempt_id: &str,
        payload: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        let path = self
            .tool_trace_dir(code)
            .join(format!("{}.jsonl", sanitize_file_component(attempt_id)));
        self.append_jsonl_line(
            &path,
            &serde_json::json!({
                "ts": Utc::now().to_rfc3339(),
                "record": payload,
            }),
        )
        .await
    }

    async fn append_jsonl_line<T>(&self, path: &PathBuf, payload: &T) -> Result<()>
    where
        T: Serialize,
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create log dir: {}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .with_context(|| format!("failed to open JSONL log: {}", path.display()))?;

        file.write_all(serde_json::to_string(payload)?.as_bytes())
            .await
            .with_context(|| format!("failed to append JSONL log: {}", path.display()))?;
        file.write_all(b"\n")
            .await
            .with_context(|| format!("failed to append newline: {}", path.display()))?;
        Ok(())
    }

    pub async fn list_active_challenge_codes(&self) -> Result<Vec<String>> {
        let runs_dir = self.root_dir.join("runs");
        let mut active = Vec::new();

        let mut entries = match fs::read_dir(&runs_dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(active),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read runs dir: {}", runs_dir.display()))
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("failed to iterate runs dir: {}", runs_dir.display()))?
        {
            let path = entry.path();
            if !path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
            {
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(content) => content,
                Err(_) => continue,
            };
            let state = match serde_json::from_str::<ChallengeRunState>(&content) {
                Ok(state) => state,
                Err(_) => continue,
            };

            if state.last_status.as_deref() == Some("running") && state.current_attempt_id.is_some()
            {
                active.push(state.code);
            }
        }

        Ok(active)
    }

    pub async fn list_running_challenges(&self) -> Result<Vec<ChallengeRunState>> {
        let runs_dir = self.root_dir.join("runs");
        let mut active = Vec::new();

        let mut entries = match fs::read_dir(&runs_dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(active),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read runs dir: {}", runs_dir.display()))
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("failed to iterate runs dir: {}", runs_dir.display()))?
        {
            let path = entry.path();
            if !path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
            {
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(content) => content,
                Err(_) => continue,
            };
            let state = match serde_json::from_str::<ChallengeRunState>(&content) {
                Ok(state) => state,
                Err(_) => continue,
            };

            if state.last_status.as_deref() == Some("running") && state.current_attempt_id.is_some()
            {
                active.push(state);
            }
        }

        Ok(active)
    }

    fn challenge_path(&self, code: &str) -> PathBuf {
        self.root_dir.join("runs").join(format!("{}.json", code))
    }

    fn logs_dir(&self) -> PathBuf {
        self.root_dir.join("logs")
    }

    fn attempt_logs_dir(&self, code: &str) -> PathBuf {
        self.logs_dir()
            .join("attempts")
            .join(sanitize_file_component(code))
    }

    fn tool_trace_dir(&self, code: &str) -> PathBuf {
        self.logs_dir()
            .join("tool-calls")
            .join(sanitize_file_component(code))
    }
}

async fn atomic_write(path: &PathBuf, bytes: &[u8]) -> Result<()> {
    let Some(parent) = path.parent() else {
        anyhow::bail!("path has no parent: {}", path.display());
    };
    fs::create_dir_all(parent)
        .await
        .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;

    let temp_name = format!(
        ".{}.tmp-{}-{}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("state"),
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let temp_path = parent.join(temp_name);

    fs::write(&temp_path, bytes)
        .await
        .with_context(|| format!("failed to write temp file: {}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .await
        .with_context(|| format!("failed to replace file: {}", path.display()))?;
    Ok(())
}

fn sanitize_file_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
