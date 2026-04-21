use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static FILE_TOOL_STATE: Lazy<Arc<RwLock<HashMap<String, HashMap<String, FileReadSnapshot>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Clone)]
struct FileReadSnapshot {
    revision_token: String,
    partial_view: bool,
}

#[derive(Debug)]
pub(crate) enum FileToolStateError {
    MissingSnapshot(String),
    PartialView(String),
    StaleSnapshot(String),
    Metadata(String),
}

impl std::fmt::Display for FileToolStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSnapshot(path) => write!(
                f,
                "file must be read with file_read before editing or overwriting: {}",
                path
            ),
            Self::PartialView(path) => write!(
                f,
                "file was only partially read; reread the full file before editing: {}",
                path
            ),
            Self::StaleSnapshot(path) => {
                write!(
                    f,
                    "file changed since it was read; reread before editing: {}",
                    path
                )
            }
            Self::Metadata(message) => write!(f, "failed to inspect file metadata: {}", message),
        }
    }
}

impl std::error::Error for FileToolStateError {}

pub(crate) async fn normalize_file_path(path: &str) -> Result<String, FileToolStateError> {
    sentinel_tools::buildin_tools::file_runtime::snapshot_key(path)
        .await
        .map_err(FileToolStateError::Metadata)
}

pub(crate) async fn record_file_read_snapshot(
    execution_id: &str,
    file_path: &str,
    revision_token: &str,
    start_line: usize,
    end_line: usize,
    total_lines: usize,
    truncated: bool,
) -> Result<(), FileToolStateError> {
    let normalized_path = normalize_file_path(file_path).await?;
    let partial_view = start_line > 1 || truncated || (total_lines > 0 && end_line < total_lines);

    let mut state = FILE_TOOL_STATE.write().await;
    state.entry(execution_id.to_string()).or_default().insert(
        normalized_path,
        FileReadSnapshot {
            revision_token: revision_token.to_string(),
            partial_view,
        },
    );
    Ok(())
}

pub(crate) async fn invalidate_file_snapshot(execution_id: &str, file_path: &str) {
    let Ok(normalized_path) = normalize_file_path(file_path).await else {
        return;
    };
    let mut state = FILE_TOOL_STATE.write().await;
    if let Some(execution_state) = state.get_mut(execution_id) {
        execution_state.remove(&normalized_path);
        if execution_state.is_empty() {
            state.remove(execution_id);
        }
    }
}

pub(crate) async fn ensure_file_snapshot_is_editable(
    execution_id: &str,
    file_path: &str,
) -> Result<(), FileToolStateError> {
    let normalized_path = normalize_file_path(file_path).await?;
    let revision_token = sentinel_tools::buildin_tools::file_runtime::revision_token(file_path)
        .await
        .map_err(FileToolStateError::Metadata)?;
    let state = FILE_TOOL_STATE.read().await;
    let Some(execution_state) = state.get(execution_id) else {
        return Err(FileToolStateError::MissingSnapshot(file_path.to_string()));
    };
    let Some(snapshot) = execution_state.get(&normalized_path) else {
        return Err(FileToolStateError::MissingSnapshot(file_path.to_string()));
    };
    if snapshot.partial_view {
        return Err(FileToolStateError::PartialView(file_path.to_string()));
    }
    if snapshot.revision_token != revision_token {
        return Err(FileToolStateError::StaleSnapshot(file_path.to_string()));
    }
    Ok(())
}

pub(crate) async fn clear_file_tool_state(execution_id: &str) {
    FILE_TOOL_STATE.write().await.remove(execution_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_snapshot_requires_full_read_and_fresh_mtime() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-tool-state-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "alpha\nbeta\ngamma\n")
            .await
            .unwrap();

        let execution_id = format!("exec-{}", uuid::Uuid::new_v4());

        let initial_revision = sentinel_tools::buildin_tools::file_runtime::revision_token(
            &file_path.to_string_lossy(),
        )
        .await
        .unwrap();
        record_file_read_snapshot(
            &execution_id,
            &file_path.to_string_lossy(),
            &initial_revision,
            1,
            2,
            3,
            true,
        )
        .await
        .unwrap();
        assert!(matches!(
            ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
                .await
                .expect_err("partial read should be rejected"),
            FileToolStateError::PartialView(_)
        ));

        record_file_read_snapshot(
            &execution_id,
            &file_path.to_string_lossy(),
            &initial_revision,
            1,
            3,
            3,
            false,
        )
        .await
        .unwrap();
        ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
            .await
            .expect("full read should allow edit");

        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        tokio::fs::write(&file_path, "alpha\nbeta\ngamma\ndelta\n")
            .await
            .unwrap();
        assert!(matches!(
            ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
                .await
                .expect_err("stale snapshot should be rejected"),
            FileToolStateError::StaleSnapshot(_)
        ));

        clear_file_tool_state(&execution_id).await;
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
