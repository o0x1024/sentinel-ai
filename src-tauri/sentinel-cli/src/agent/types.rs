use super::config::AgentRuntimeConfig;
use super::rate_limit::LlmThrottleSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorState {
    pub pid: u32,
    pub run_id: String,
    pub status: String,
    pub started_at: String,
    pub updated_at: String,
    pub worker_restarts: u32,
    pub worker_pid: Option<u32>,
    pub last_worker_reason: Option<String>,
    pub config: AgentRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerState {
    pub pid: u32,
    pub run_id: String,
    pub status: String,
    pub started_at: String,
    pub updated_at: String,
    pub cycle: u64,
    pub current_challenge: Option<String>,
    pub active_challenges: Vec<String>,
    pub last_error: Option<String>,
    pub llm_throttle: Option<LlmThrottleSnapshot>,
    pub config: AgentRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSupervisorReport {
    pub ok: bool,
    pub pid: u32,
    pub run_id: String,
    pub supervisor_stdout_log: String,
    pub supervisor_stderr_log: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopRequestReport {
    pub ok: bool,
    pub stop_request_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusReport {
    pub root_dir: String,
    pub supervisor_running: bool,
    pub worker_heartbeat_stale: Option<bool>,
    pub supervisor: Option<SupervisorState>,
    pub worker: Option<WorkerState>,
    pub stop_requested: bool,
}
