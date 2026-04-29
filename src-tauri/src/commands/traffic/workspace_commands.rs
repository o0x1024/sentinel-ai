use std::fs;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::commands::command_response_support::CommandResponse;

const TRAFFIC_WORKBENCH_DIR: &str = "traffic-workbench";
const TRAFFIC_DRAFTS_FILE: &str = "drafts.json";
const ATTACK_WORKSPACES_FILE: &str = "attack_workspaces.json";
const REPLAY_RUNS_FILE: &str = "replay_runs.json";
const INTRUDER_WORKSPACE_SESSIONS_FILE: &str = "intruder_workspace_sessions.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficWorkbenchSourceRecord {
    pub kind: String,
    pub label: String,
    pub request_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficHttpEndpointRecord {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub sni_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficDraftRecord {
    pub id: String,
    pub title: String,
    pub source: Option<TrafficWorkbenchSourceRecord>,
    pub source_snapshot_id: Option<String>,
    pub endpoint: TrafficHttpEndpointRecord,
    pub endpoint_mode: String,
    pub raw_request: String,
    pub preferred_view: String,
    pub pinned: bool,
    pub active_revision_id: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficDraftRevisionRecord {
    pub id: String,
    pub draft_id: String,
    pub endpoint: TrafficHttpEndpointRecord,
    pub raw_request: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTrafficDraftRecord {
    pub draft: TrafficDraftRecord,
    pub revisions: Vec<TrafficDraftRevisionRecord>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTrafficDraftStore {
    pub active_draft_id: Option<String>,
    pub drafts: Vec<PersistedTrafficDraftRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficIntruderTargetRecord {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficIntruderPositionRecord {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAttackWorkspaceRecord {
    pub id: String,
    pub title: String,
    pub source: Option<TrafficWorkbenchSourceRecord>,
    pub source_draft_id: Option<String>,
    pub source_draft_revision_id: Option<String>,
    pub request_text: String,
    pub target: TrafficIntruderTargetRecord,
    pub positions: Vec<TrafficIntruderPositionRecord>,
    pub run_state: String,
    pub result_count: usize,
    pub progress: PersistedAttackWorkspaceProgressRecord,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAttackWorkspaceProgressRecord {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub active: usize,
    pub truncated: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAttackWorkspaceStore {
    pub active_workspace_id: Option<String>,
    pub workspaces: Vec<PersistedAttackWorkspaceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficHttpHeaderRecord {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedReplayResponseRecord {
    pub status_code: u16,
    pub version_observed: Option<String>,
    pub status_text: Option<String>,
    pub headers: Vec<TrafficHttpHeaderRecord>,
    pub body_text: String,
    pub body_bytes_base64: Option<String>,
    pub raw_text: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedReplayRunRecord {
    pub id: String,
    pub draft_id: String,
    pub draft_revision_id: String,
    pub state: String,
    pub response: Option<PersistedReplayResponseRecord>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedReplayRunStore {
    pub replay_runs: Vec<PersistedReplayRunRecord>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedIntruderWorkspaceSessionStore {
    pub active_workspace_id: Option<String>,
    pub workspaces: Vec<serde_json::Value>,
}

fn resolve_traffic_workbench_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
    resolve_traffic_workbench_dir_from_root(&app_data_dir)
}

fn resolve_traffic_workbench_dir_from_root(root: &Path) -> Result<PathBuf, String> {
    let traffic_dir = root.join(TRAFFIC_WORKBENCH_DIR);
    fs::create_dir_all(&traffic_dir)
        .map_err(|error| format!("Failed to create traffic workbench directory: {error}"))?;
    Ok(traffic_dir)
}

fn drafts_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_traffic_workbench_dir(app)?.join(TRAFFIC_DRAFTS_FILE))
}

fn attack_workspaces_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_traffic_workbench_dir(app)?.join(ATTACK_WORKSPACES_FILE))
}

fn replay_runs_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_traffic_workbench_dir(app)?.join(REPLAY_RUNS_FILE))
}

fn intruder_workspace_sessions_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_traffic_workbench_dir(app)?.join(INTRUDER_WORKSPACE_SESSIONS_FILE))
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let bytes =
        fs::read(path).map_err(|error| format!("Failed to read '{}': {error}", path.display()))?;
    serde_json::from_slice::<T>(&bytes)
        .map_err(|error| format!("Failed to parse '{}': {error}", path.display()))
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create '{}': {error}", parent.display()))?;
    }

    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Failed to serialize '{}': {error}", path.display()))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)
        .map_err(|error| format!("Failed to write '{}': {error}", temp_path.display()))?;
    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "Failed to move persisted file '{}' into place: {error}",
            path.display()
        )
    })?;
    Ok(())
}

#[tauri::command]
pub async fn load_traffic_draft_store(
    app: AppHandle,
) -> Result<CommandResponse<PersistedTrafficDraftStore>, String> {
    let path = drafts_file_path(&app)?;
    let store: PersistedTrafficDraftStore = read_json_file(&path)?;
    Ok(CommandResponse::ok(store))
}

#[tauri::command]
pub async fn save_traffic_draft_store(
    app: AppHandle,
    store: PersistedTrafficDraftStore,
) -> Result<CommandResponse<()>, String> {
    let path = drafts_file_path(&app)?;
    write_json_file(&path, &store)?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn load_attack_workspace_store(
    app: AppHandle,
) -> Result<CommandResponse<PersistedAttackWorkspaceStore>, String> {
    let path = attack_workspaces_file_path(&app)?;
    let store: PersistedAttackWorkspaceStore = read_json_file(&path)?;
    Ok(CommandResponse::ok(store))
}

#[tauri::command]
pub async fn save_attack_workspace_store(
    app: AppHandle,
    store: PersistedAttackWorkspaceStore,
) -> Result<CommandResponse<()>, String> {
    let path = attack_workspaces_file_path(&app)?;
    write_json_file(&path, &store)?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn load_replay_run_store(
    app: AppHandle,
) -> Result<CommandResponse<PersistedReplayRunStore>, String> {
    let path = replay_runs_file_path(&app)?;
    let store: PersistedReplayRunStore = read_json_file(&path)?;
    Ok(CommandResponse::ok(store))
}

#[tauri::command]
pub async fn save_replay_run_store(
    app: AppHandle,
    store: PersistedReplayRunStore,
) -> Result<CommandResponse<()>, String> {
    let path = replay_runs_file_path(&app)?;
    write_json_file(&path, &store)?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn load_intruder_workspace_session_store(
    app: AppHandle,
) -> Result<CommandResponse<PersistedIntruderWorkspaceSessionStore>, String> {
    let path = intruder_workspace_sessions_file_path(&app)?;
    let store: PersistedIntruderWorkspaceSessionStore = read_json_file(&path)?;
    Ok(CommandResponse::ok(store))
}

#[tauri::command]
pub async fn save_intruder_workspace_session_store(
    app: AppHandle,
    store: PersistedIntruderWorkspaceSessionStore,
) -> Result<CommandResponse<()>, String> {
    let path = intruder_workspace_sessions_file_path(&app)?;
    write_json_file(&path, &store)?;
    Ok(CommandResponse::ok(()))
}

#[cfg(test)]
mod tests {
    use super::{
        read_json_file, resolve_traffic_workbench_dir_from_root, write_json_file,
        PersistedAttackWorkspaceStore, PersistedIntruderWorkspaceSessionStore,
        PersistedReplayRunStore, PersistedTrafficDraftStore, TRAFFIC_WORKBENCH_DIR,
    };
    use tempfile::tempdir;

    #[test]
    fn resolves_workbench_directory_under_root() {
        let temp = tempdir().expect("tempdir");
        let workbench_dir =
            resolve_traffic_workbench_dir_from_root(temp.path()).expect("resolve workbench dir");

        assert!(workbench_dir.exists());
        assert_eq!(
            workbench_dir.file_name().and_then(|name| name.to_str()),
            Some(TRAFFIC_WORKBENCH_DIR)
        );
    }

    #[test]
    fn reads_default_store_when_file_missing() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("missing.json");

        let drafts: PersistedTrafficDraftStore = read_json_file(&path).expect("read drafts");
        let attacks: PersistedAttackWorkspaceStore = read_json_file(&path).expect("read attacks");
        let replays: PersistedReplayRunStore = read_json_file(&path).expect("read replays");
        let sessions: PersistedIntruderWorkspaceSessionStore =
            read_json_file(&path).expect("read sessions");

        assert!(drafts.drafts.is_empty());
        assert!(drafts.active_draft_id.is_none());
        assert!(attacks.workspaces.is_empty());
        assert!(attacks.active_workspace_id.is_none());
        assert!(replays.replay_runs.is_empty());
        assert!(sessions.workspaces.is_empty());
        assert!(sessions.active_workspace_id.is_none());
    }

    #[test]
    fn round_trips_store_json() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("drafts.json");
        let input = PersistedTrafficDraftStore {
            active_draft_id: Some("draft-1".to_string()),
            drafts: Vec::new(),
        };

        write_json_file(&path, &input).expect("write");
        let restored: PersistedTrafficDraftStore = read_json_file(&path).expect("read");

        assert_eq!(restored.active_draft_id.as_deref(), Some("draft-1"));
        assert!(restored.drafts.is_empty());
    }

    #[test]
    fn round_trips_replay_store_json() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("replay_runs.json");
        let input = PersistedReplayRunStore {
            replay_runs: Vec::new(),
        };

        write_json_file(&path, &input).expect("write");
        let restored: PersistedReplayRunStore = read_json_file(&path).expect("read");

        assert!(restored.replay_runs.is_empty());
    }

    #[test]
    fn round_trips_intruder_session_store_json() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("intruder_sessions.json");
        let input = PersistedIntruderWorkspaceSessionStore {
            active_workspace_id: Some("workspace-1".to_string()),
            workspaces: vec![serde_json::json!({ "id": "workspace-1", "results": [] })],
        };

        write_json_file(&path, &input).expect("write");
        let restored: PersistedIntruderWorkspaceSessionStore = read_json_file(&path).expect("read");

        assert_eq!(restored.active_workspace_id.as_deref(), Some("workspace-1"));
        assert_eq!(restored.workspaces.len(), 1);
    }
}
