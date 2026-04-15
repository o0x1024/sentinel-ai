use super::paths::AgentPaths;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct SupervisorLockRecord {
    pid: u32,
    run_id: String,
    acquired_at: String,
}

pub struct SupervisorLockGuard {
    path: PathBuf,
}

impl Drop for SupervisorLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn acquire_supervisor_lock(paths: &AgentPaths, run_id: &str) -> Result<SupervisorLockGuard> {
    fs::create_dir_all(&paths.control_dir).with_context(|| {
        format!(
            "failed to create agent control dir: {}",
            paths.control_dir.display()
        )
    })?;

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&paths.supervisor_lock_path)
    {
        Ok(mut file) => {
            let record = SupervisorLockRecord {
                pid: std::process::id(),
                run_id: run_id.to_string(),
                acquired_at: Utc::now().to_rfc3339(),
            };
            file.write_all(serde_json::to_string_pretty(&record)?.as_bytes())
                .with_context(|| {
                    format!(
                        "failed to write supervisor lock: {}",
                        paths.supervisor_lock_path.display()
                    )
                })?;
            Ok(SupervisorLockGuard {
                path: paths.supervisor_lock_path.clone(),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = fs::read_to_string(&paths.supervisor_lock_path)
                .ok()
                .and_then(|raw| serde_json::from_str::<SupervisorLockRecord>(&raw).ok());
            if let Some(record) = existing {
                if process_is_running(record.pid) {
                    anyhow::bail!(
                        "supervisor already running: pid={} run_id={}",
                        record.pid,
                        record.run_id
                    );
                }
            }
            let _ = fs::remove_file(&paths.supervisor_lock_path);
            acquire_supervisor_lock(paths, run_id)
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to create supervisor lock: {}",
                paths.supervisor_lock_path.display()
            )
        }),
    }
}

#[cfg(unix)]
fn process_is_running(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    if result == 0 {
        return true;
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(code) if code == libc::EPERM => true,
        _ => false,
    }
}

#[cfg(not(unix))]
fn process_is_running(_pid: u32) -> bool {
    true
}
