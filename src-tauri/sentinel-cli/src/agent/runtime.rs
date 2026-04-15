use super::config::AgentRuntimeConfig;
use super::llm::solve_challenge_with_llm;
use super::lock::acquire_supervisor_lock;
use super::paths::AgentPaths;
use super::rate_limit::SharedLlmThrottle;
use super::types::{
    AgentStatusReport, InstallSupervisorReport, StopRequestReport, SupervisorState, WorkerState,
};
use crate::arena::{load_arena_config, ArenaClient, ChallengeInfo};
use crate::runtime::RuntimeStateStore;
use crate::solver::select_challenges;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::sync::{oneshot, Mutex};
use tokio::time::sleep;

const SUPERVISOR_POLL_INTERVAL_SECS: u64 = 2;
const WORKER_SHUTDOWN_GRACE_SECS: u64 = 10;

#[derive(Debug)]
struct AttemptTaskResult {
    code: String,
    last_error: Option<String>,
}

pub async fn install_supervisor(config: AgentRuntimeConfig) -> Result<InstallSupervisorReport> {
    let paths = AgentPaths::new();
    ensure_agent_dirs(&paths).await?;
    clear_stop_request(&paths).await?;

    let run_id = build_run_id();
    let current_exe =
        std::env::current_exe().context("failed to resolve current executable for supervisor")?;
    let stdout = open_log_file(&paths.supervisor_stdout_path)?;
    let stderr = open_log_file(&paths.supervisor_stderr_path)?;

    let mut command = Command::new(&current_exe);
    command.args(config.to_supervisor_args());
    command.stdin(Stdio::null());
    command.stdout(Stdio::from(stdout));
    command.stderr(Stdio::from(stderr));
    command.env("SENTINEL_AGENT_RUN_ID", &run_id);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let child = command
        .spawn()
        .context("failed to spawn detached supervisor")?;

    Ok(InstallSupervisorReport {
        ok: true,
        pid: child.id(),
        run_id,
        supervisor_stdout_log: paths.supervisor_stdout_path.display().to_string(),
        supervisor_stderr_log: paths.supervisor_stderr_path.display().to_string(),
    })
}

pub async fn run_supervisor(config: AgentRuntimeConfig) -> Result<()> {
    let paths = AgentPaths::new();
    ensure_agent_dirs(&paths).await?;
    clear_stop_request(&paths).await?;

    let run_id = std::env::var("SENTINEL_AGENT_RUN_ID").unwrap_or_else(|_| build_run_id());
    let _lock = acquire_supervisor_lock(&paths, &run_id)?;

    let started_at = now_rfc3339();
    let mut restarts = 0_u32;
    let mut child: Option<Child> = None;
    let mut last_worker_reason: Option<String> = None;

    loop {
        if stop_requested(&paths).await {
            break;
        }

        let mut worker_pid = child.as_ref().map(std::process::Child::id);
        let mut restart_reason = None;

        if let Some(existing) = child.as_mut() {
            if let Some(status) = existing
                .try_wait()
                .context("failed to inspect worker status")?
            {
                restart_reason = Some(format!("worker exited with status {}", status));
            } else if worker_heartbeat_is_stale(&paths, config.heartbeat_timeout_secs).await? {
                let _ = existing.kill();
                let _ = existing.wait();
                restart_reason = Some("worker heartbeat stale".to_string());
            }
        } else {
            restart_reason = Some("worker missing".to_string());
        }

        if let Some(reason) = restart_reason {
            child = None;
            if last_worker_reason.is_some() {
                restarts += 1;
            }
            last_worker_reason = Some(reason);
            sleep(Duration::from_secs(config.restart_delay_secs)).await;
            if stop_requested(&paths).await {
                break;
            }
            let spawned = spawn_worker(&paths, &config, &run_id)?;
            worker_pid = Some(spawned.id());
            child = Some(spawned);
        }

        let state = SupervisorState {
            pid: std::process::id(),
            run_id: run_id.clone(),
            status: "running".to_string(),
            started_at: started_at.clone(),
            updated_at: now_rfc3339(),
            worker_restarts: restarts,
            worker_pid,
            last_worker_reason: last_worker_reason.clone(),
            config: config.clone(),
        };
        write_json(&paths.supervisor_state_path, &state).await?;

        sleep(Duration::from_secs(SUPERVISOR_POLL_INTERVAL_SECS)).await;
    }

    if let Some(mut running_child) = child {
        wait_for_worker_shutdown(&mut running_child).await;
    }

    let stopped_state = SupervisorState {
        pid: std::process::id(),
        run_id,
        status: "stopped".to_string(),
        started_at,
        updated_at: now_rfc3339(),
        worker_restarts: restarts,
        worker_pid: None,
        last_worker_reason,
        config,
    };
    write_json(&paths.supervisor_state_path, &stopped_state).await?;
    Ok(())
}

pub async fn run_worker(config: AgentRuntimeConfig) -> Result<()> {
    let paths = AgentPaths::new();
    ensure_agent_dirs(&paths).await?;
    let run_id = std::env::var("SENTINEL_AGENT_RUN_ID").unwrap_or_else(|_| build_run_id());
    let started_at = now_rfc3339();
    let state = Arc::new(Mutex::new(WorkerState {
        pid: std::process::id(),
        run_id,
        status: "starting".to_string(),
        started_at,
        updated_at: now_rfc3339(),
        cycle: 0,
        current_challenge: None,
        active_challenges: Vec::new(),
        last_error: None,
        llm_throttle: None,
        config: config.clone(),
    }));
    let llm_throttle = SharedLlmThrottle::default();

    let (heartbeat_stop_tx, heartbeat_stop_rx) = oneshot::channel();
    let heartbeat_handle = tokio::spawn(start_worker_heartbeat(
        paths.clone(),
        state.clone(),
        heartbeat_stop_rx,
        config.heartbeat_interval_secs,
        llm_throttle.clone(),
    ));

    let result = worker_loop(paths.clone(), state.clone(), config, llm_throttle).await;

    {
        let mut guard = state.lock().await;
        guard.status = if result.is_ok() {
            "stopped".to_string()
        } else {
            "failed".to_string()
        };
        guard.updated_at = now_rfc3339();
        if let Err(error) = &result {
            guard.last_error = Some(error.to_string());
        }
    }
    write_json(&paths.worker_state_path, &state.lock().await.clone()).await?;
    let _ = heartbeat_stop_tx.send(());
    let _ = heartbeat_handle.await;
    result
}

pub async fn status_report() -> Result<AgentStatusReport> {
    let paths = AgentPaths::new();
    let supervisor = read_json::<SupervisorState>(&paths.supervisor_state_path).await?;
    let worker = read_json::<WorkerState>(&paths.worker_state_path).await?;
    let heartbeat_timeout_secs = worker
        .as_ref()
        .map(|item| item.config.heartbeat_timeout_secs)
        .or_else(|| {
            supervisor
                .as_ref()
                .map(|item| item.config.heartbeat_timeout_secs)
        })
        .unwrap_or(30);

    Ok(AgentStatusReport {
        root_dir: paths.root_dir.display().to_string(),
        supervisor_running: supervisor
            .as_ref()
            .map(|item| item.status == "running")
            .unwrap_or(false),
        worker_heartbeat_stale: match worker.as_ref() {
            Some(_) => Some(worker_heartbeat_is_stale(&paths, heartbeat_timeout_secs).await?),
            None => None,
        },
        supervisor,
        worker,
        stop_requested: stop_requested(&paths).await,
    })
}

pub async fn request_stop() -> Result<StopRequestReport> {
    let paths = AgentPaths::new();
    ensure_agent_dirs(&paths).await?;
    tokio::fs::write(&paths.stop_request_path, b"stop\n")
        .await
        .with_context(|| {
            format!(
                "failed to create stop request: {}",
                paths.stop_request_path.display()
            )
        })?;
    Ok(StopRequestReport {
        ok: true,
        stop_request_path: paths.stop_request_path.display().to_string(),
    })
}

async fn worker_loop(
    paths: AgentPaths,
    state: Arc<Mutex<WorkerState>>,
    config: AgentRuntimeConfig,
    llm_throttle: SharedLlmThrottle,
) -> Result<()> {
    let store = RuntimeStateStore::new(crate::state::sentinel_state_dir());
    let mut tasks = JoinSet::new();
    let mut in_flight = BTreeSet::new();
    let mut arena_rate_limit_streak = 0_u32;

    loop {
        while let Some(join_result) = try_join_completed_attempt(&mut tasks).await {
            handle_completed_attempt(join_result, &state, &mut in_flight).await;
        }

        if stop_requested(&paths).await && tasks.is_empty() {
            return Ok(());
        }

        let available_slots = config
            .max_concurrent_challenges
            .saturating_sub(in_flight.len());

        if !stop_requested(&paths).await && available_slots > 0 {
            set_worker_status(
                &state,
                if in_flight.is_empty() {
                    "listing"
                } else {
                    "scheduling"
                },
                None,
                None,
            )
            .await;
            let arena = load_agent_client().await?;
            let list = match arena.list_challenges().await {
                Ok(list) => {
                    arena_rate_limit_streak = 0;
                    list
                }
                Err(error) => {
                    set_worker_status(&state, "error", None, Some(error.to_string())).await;
                    let _ = store
                        .append_event(
                            "agent_worker_error",
                            &serde_json::json!({
                                "phase": "list_challenges",
                                "error": error.to_string(),
                            }),
                        )
                        .await;
                    let backoff_secs = arena_backoff_secs(
                        &error.to_string(),
                        &mut arena_rate_limit_streak,
                        config.loop_interval_secs,
                    );
                    sleep(Duration::from_secs(backoff_secs)).await;
                    continue;
                }
            };

            reconcile_visible_challenge_state(&arena, &store, &list.challenges).await;
            let active_codes = store
                .list_active_challenge_codes()
                .await
                .unwrap_or_default();
            let challenges = filter_schedulable_challenges(
                &store,
                prioritize_active_challenges(
                    select_challenges(list.challenges, &config.solve_filters()),
                    &active_codes,
                ),
                &config,
            )
            .await;

            let mut scheduled = 0_usize;
            for challenge in challenges {
                if scheduled >= available_slots || stop_requested(&paths).await {
                    break;
                }
                if in_flight.contains(&challenge.code) {
                    continue;
                }

                in_flight.insert(challenge.code.clone());
                register_active_challenge(&state, &challenge.code).await;

                let arena = arena.clone();
                let store = store.clone();
                let config = config.clone();
                let llm_throttle = llm_throttle.clone();
                tasks.spawn(async move {
                    let code = challenge.code.clone();
                    let last_error =
                        solve_challenge_once(&arena, &store, &config, llm_throttle, challenge)
                            .await;
                    AttemptTaskResult { code, last_error }
                });
                scheduled += 1;
            }

            if in_flight.is_empty() {
                set_worker_status(&state, "idle", None, None).await;
                sleep_with_stop(&paths, config.loop_interval_secs).await;
                increment_cycle(&state).await;
                continue;
            }
        }

        if tasks.is_empty() {
            set_worker_status(&state, "idle", None, None).await;
            sleep_with_stop(&paths, config.loop_interval_secs).await;
            increment_cycle(&state).await;
            continue;
        }

        set_worker_status(&state, "solving", None, None).await;
        tokio::select! {
            _ = sleep(Duration::from_secs(config.loop_interval_secs)) => {}
            join_result = tasks.join_next() => {
                if let Some(join_result) = join_result {
                    handle_completed_attempt(join_result, &state, &mut in_flight).await;
                }
            }
        }
        increment_cycle(&state).await;
    }
}

fn prioritize_active_challenges(
    mut challenges: Vec<ChallengeInfo>,
    active_codes: &[String],
) -> Vec<ChallengeInfo> {
    if active_codes.is_empty() {
        return challenges;
    }

    let active = active_codes
        .iter()
        .map(|code| code.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    challenges.sort_by_key(|challenge| {
        if active.contains(challenge.code.as_str()) {
            0
        } else {
            1
        }
    });
    challenges
}

async fn filter_schedulable_challenges(
    store: &RuntimeStateStore,
    challenges: Vec<ChallengeInfo>,
    config: &AgentRuntimeConfig,
) -> Vec<ChallengeInfo> {
    let mut filtered = Vec::new();

    for challenge in challenges {
        if challenge_is_schedulable(store, &challenge, config).await {
            filtered.push(challenge);
        }
    }

    filtered
}

async fn challenge_is_schedulable(
    store: &RuntimeStateStore,
    challenge: &ChallengeInfo,
    config: &AgentRuntimeConfig,
) -> bool {
    let Ok(state) = store.load_challenge(&challenge.code).await else {
        return false;
    };

    if state.last_status.as_deref() == Some("running") && state.current_attempt_id.is_some() {
        return true;
    }

    if state.consecutive_failures >= config.max_attempts_per_challenge {
        return false;
    }

    let Some(next_eligible_at) = state.next_eligible_at.as_deref() else {
        return true;
    };

    DateTime::parse_from_rfc3339(next_eligible_at)
        .map(|value| value.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}

async fn solve_challenge_once(
    arena: &ArenaClient,
    store: &RuntimeStateStore,
    config: &AgentRuntimeConfig,
    llm_throttle: SharedLlmThrottle,
    challenge: ChallengeInfo,
) -> Option<String> {
    let _ = store
        .append_event(
            "agent_challenge_selected",
            &serde_json::json!({
                "code": challenge.code,
                "title": challenge.title,
                "level": challenge.level,
                "difficulty": challenge.difficulty,
            }),
        )
        .await;

    let execution_id = format!("agent:{}:{}", std::process::id(), challenge.code);
    let llm_config = match config.load_llm_config(&execution_id) {
        Ok(config) => config,
        Err(error) => {
            let _ = store
                .append_event(
                    "agent_llm_config_error",
                    &serde_json::json!({
                        "code": challenge.code,
                        "error": error.to_string(),
                    }),
                )
                .await;
            return Some(error.to_string());
        }
    };

    match solve_challenge_with_llm(
        arena,
        challenge.clone(),
        llm_config,
        config.max_steps_per_challenge,
        config.max_challenge_duration_secs,
        config.allow_hint,
        config.stop_when_done,
        Some(llm_throttle),
        config.failure_cooldown_secs,
    )
    .await
    {
        Ok(report) => {
            let _ = store.append_event("agent_challenge_result", &report).await;
            None
        }
        Err(error) => {
            let _ = arena.stop_challenge(&challenge.code).await;
            let _ = store
                .append_event(
                    "agent_challenge_error",
                    &serde_json::json!({
                        "code": challenge.code,
                        "error": error.to_string(),
                    }),
                )
                .await;
            Some(error.to_string())
        }
    }
}

async fn reconcile_visible_challenge_state(
    arena: &ArenaClient,
    store: &RuntimeStateStore,
    challenges: &[ChallengeInfo],
) {
    for challenge in challenges {
        let local_state = match store.load_challenge(&challenge.code).await {
            Ok(state) => state,
            Err(_) => continue,
        };
        let local_running = local_state.last_status.as_deref() == Some("running")
            && local_state.current_attempt_id.is_some();
        let remote_running = matches!(challenge.instance_status.as_str(), "running" | "pending");

        if local_running && !remote_running {
            let reason = format!(
                "local run state cleared because remote instance is {}",
                challenge.instance_status
            );
            let _ = store
                .mark_challenge_terminal(&challenge.code, "interrupted", Some(reason.clone()))
                .await;
            let _ = store
                .append_event(
                    "agent_local_run_reconciled",
                    &serde_json::json!({
                        "code": challenge.code,
                        "status": "interrupted",
                        "reason": reason,
                    }),
                )
                .await;
            continue;
        }

        if !local_running && remote_running {
            let stop_result = arena.stop_challenge(&challenge.code).await;
            let _ = store
                .append_event(
                    "agent_remote_instance_recovered",
                    &serde_json::json!({
                        "code": challenge.code,
                        "instance_status": challenge.instance_status,
                        "stop_ok": stop_result.is_ok(),
                        "stop_error": stop_result.err().map(|error| error.to_string()),
                    }),
                )
                .await;
        }
    }
}

async fn start_worker_heartbeat(
    paths: AgentPaths,
    state: Arc<Mutex<WorkerState>>,
    mut stop_rx: oneshot::Receiver<()>,
    interval_secs: u64,
    llm_throttle: SharedLlmThrottle,
) {
    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            _ = sleep(Duration::from_secs(interval_secs)) => {
                let throttle_snapshot = llm_throttle.snapshot().await;
                let snapshot = {
                    let mut guard = state.lock().await;
                    guard.updated_at = now_rfc3339();
                    guard.llm_throttle = Some(throttle_snapshot);
                    guard.clone()
                };
                let _ = write_json(&paths.worker_state_path, &snapshot).await;
            }
        }
    }
}

fn spawn_worker(paths: &AgentPaths, config: &AgentRuntimeConfig, run_id: &str) -> Result<Child> {
    let current_exe =
        std::env::current_exe().context("failed to resolve current executable for worker")?;
    let stdout = open_log_file(&paths.worker_stdout_path)?;
    let stderr = open_log_file(&paths.worker_stderr_path)?;

    let mut command = Command::new(current_exe);
    command.args(config.to_worker_args());
    command.stdin(Stdio::null());
    command.stdout(Stdio::from(stdout));
    command.stderr(Stdio::from(stderr));
    command.env("SENTINEL_AGENT_RUN_ID", run_id);
    command
        .spawn()
        .context("failed to spawn supervised worker process")
}

async fn wait_for_worker_shutdown(child: &mut Child) {
    let paths = AgentPaths::new();
    let _ = tokio::fs::write(&paths.stop_request_path, b"stop\n").await;

    for _ in 0..WORKER_SHUTDOWN_GRACE_SECS {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        sleep(Duration::from_secs(1)).await;
    }

    let _ = child.kill();
    let _ = child.wait();
}

async fn load_agent_client() -> Result<ArenaClient> {
    let config = load_arena_config(crate::state::sentinel_state_dir()).await?;
    ArenaClient::new(config)
}

async fn ensure_agent_dirs(paths: &AgentPaths) -> Result<()> {
    tokio::fs::create_dir_all(&paths.root_dir)
        .await
        .with_context(|| format!("failed to create agent dir: {}", paths.root_dir.display()))?;
    tokio::fs::create_dir_all(&paths.logs_dir)
        .await
        .with_context(|| format!("failed to create logs dir: {}", paths.logs_dir.display()))?;
    tokio::fs::create_dir_all(&paths.control_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create control dir: {}",
                paths.control_dir.display()
            )
        })?;
    Ok(())
}

async fn clear_stop_request(paths: &AgentPaths) -> Result<()> {
    match tokio::fs::remove_file(&paths.stop_request_path).await {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to clear stop request: {}",
                paths.stop_request_path.display()
            )
        }),
    }
}

async fn stop_requested(paths: &AgentPaths) -> bool {
    tokio::fs::metadata(&paths.stop_request_path).await.is_ok()
}

async fn worker_heartbeat_is_stale(paths: &AgentPaths, timeout_secs: u64) -> Result<bool> {
    let Some(worker) = read_json::<WorkerState>(&paths.worker_state_path).await? else {
        return Ok(false);
    };
    let updated_at = DateTime::parse_from_rfc3339(&worker.updated_at)
        .with_context(|| format!("invalid worker heartbeat timestamp: {}", worker.updated_at))?
        .with_timezone(&Utc);
    Ok((Utc::now() - updated_at).num_seconds() > timeout_secs as i64)
}

async fn read_json<T>(path: &std::path::Path) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            let value = serde_json::from_str::<T>(&content)
                .with_context(|| format!("failed to parse JSON file: {}", path.display()))?;
            Ok(Some(value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("failed to read JSON file: {}", path.display()))
        }
    }
}

async fn write_json<T>(path: &std::path::Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let content =
        serde_json::to_string_pretty(value).context("failed to serialize agent state JSON")?;
    atomic_write_json(path, content.as_bytes()).await?;
    Ok(())
}

async fn atomic_write_json(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let Some(parent) = path.parent() else {
        anyhow::bail!("path has no parent: {}", path.display());
    };
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("failed to create dir: {}", parent.display()))?;
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
    tokio::fs::write(&temp_path, bytes)
        .await
        .with_context(|| format!("failed to write temp file: {}", temp_path.display()))?;
    tokio::fs::rename(&temp_path, path)
        .await
        .with_context(|| format!("failed to replace JSON file: {}", path.display()))?;
    Ok(())
}

fn arena_backoff_secs(error: &str, streak: &mut u32, base_delay_secs: u64) -> u64 {
    if is_arena_rate_limit_error(error) {
        *streak = streak.saturating_add(1);
        let factor = 1_u64 << streak.saturating_sub(1).min(3);
        return (base_delay_secs.max(2) * factor).min(30);
    }

    *streak = 0;
    base_delay_secs.max(1)
}

fn is_arena_rate_limit_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("429")
        || normalized.contains("too many requests")
        || normalized.contains("请求频率超出限制")
        || normalized.contains("每秒最多调用3次")
}

async fn set_worker_status(
    state: &Arc<Mutex<WorkerState>>,
    status: &str,
    current_challenge: Option<String>,
    last_error: Option<String>,
) {
    let mut guard = state.lock().await;
    guard.status = status.to_string();
    guard.current_challenge = current_challenge.or_else(|| guard.active_challenges.first().cloned());
    guard.last_error = last_error;
    guard.updated_at = now_rfc3339();
}

async fn register_active_challenge(state: &Arc<Mutex<WorkerState>>, code: &str) {
    let mut guard = state.lock().await;
    if !guard.active_challenges.iter().any(|item| item == code) {
        guard.active_challenges.push(code.to_string());
    }
    guard.current_challenge = guard.active_challenges.first().cloned();
    guard.status = "solving".to_string();
    guard.updated_at = now_rfc3339();
}

async fn unregister_active_challenge(
    state: &Arc<Mutex<WorkerState>>,
    code: &str,
    last_error: Option<String>,
) {
    let mut guard = state.lock().await;
    guard.active_challenges.retain(|item| item != code);
    guard.current_challenge = guard.active_challenges.first().cloned();
    if let Some(last_error) = last_error {
        guard.last_error = Some(last_error);
    }
    guard.status = if guard.active_challenges.is_empty() {
        "sleeping".to_string()
    } else {
        "solving".to_string()
    };
    guard.updated_at = now_rfc3339();
}

async fn increment_cycle(state: &Arc<Mutex<WorkerState>>) {
    let mut guard = state.lock().await;
    guard.cycle += 1;
    guard.updated_at = now_rfc3339();
}

async fn sleep_with_stop(paths: &AgentPaths, secs: u64) {
    for _ in 0..secs {
        if stop_requested(paths).await {
            return;
        }
        sleep(Duration::from_secs(1)).await;
    }
}

async fn try_join_completed_attempt(
    tasks: &mut JoinSet<AttemptTaskResult>,
) -> Option<Result<AttemptTaskResult, tokio::task::JoinError>> {
    match tokio::time::timeout(Duration::from_millis(1), tasks.join_next()).await {
        Ok(Some(result)) => Some(result),
        _ => None,
    }
}

async fn handle_completed_attempt(
    join_result: Result<AttemptTaskResult, tokio::task::JoinError>,
    state: &Arc<Mutex<WorkerState>>,
    in_flight: &mut BTreeSet<String>,
) {
    match join_result {
        Ok(result) => {
            in_flight.remove(&result.code);
            unregister_active_challenge(state, &result.code, result.last_error).await;
        }
        Err(error) => {
            let error_message = error.to_string();
            let active_codes = {
                let guard = state.lock().await;
                guard.active_challenges.clone()
            };
            if let Some(code) = active_codes.first() {
                in_flight.remove(code);
                unregister_active_challenge(state, code, Some(error_message)).await;
            } else {
                set_worker_status(state, "error", None, Some(error_message)).await;
            }
        }
    }
}

fn open_log_file(path: &std::path::Path) -> Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log dir: {}", parent.display()))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file: {}", path.display()))
}

fn build_run_id() -> String {
    format!(
        "contest-run-{}-{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        std::process::id()
    )
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
