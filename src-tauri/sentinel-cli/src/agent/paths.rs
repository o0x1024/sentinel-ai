use crate::state::sentinel_state_dir;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgentPaths {
    pub root_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub control_dir: PathBuf,
    pub supervisor_lock_path: PathBuf,
    pub supervisor_state_path: PathBuf,
    pub worker_state_path: PathBuf,
    pub stop_request_path: PathBuf,
    pub supervisor_stdout_path: PathBuf,
    pub supervisor_stderr_path: PathBuf,
    pub worker_stdout_path: PathBuf,
    pub worker_stderr_path: PathBuf,
}

impl AgentPaths {
    pub fn new() -> Self {
        let root_dir = sentinel_state_dir().join("agent");
        let logs_dir = root_dir.join("logs");
        let control_dir = root_dir.join("control");
        Self {
            supervisor_lock_path: control_dir.join("supervisor.lock"),
            stop_request_path: control_dir.join("stop.request"),
            supervisor_state_path: root_dir.join("supervisor.json"),
            worker_state_path: root_dir.join("worker.json"),
            supervisor_stdout_path: logs_dir.join("supervisor.stdout.log"),
            supervisor_stderr_path: logs_dir.join("supervisor.stderr.log"),
            worker_stdout_path: logs_dir.join("worker.stdout.log"),
            worker_stderr_path: logs_dir.join("worker.stderr.log"),
            root_dir,
            logs_dir,
            control_dir,
        }
    }
}
