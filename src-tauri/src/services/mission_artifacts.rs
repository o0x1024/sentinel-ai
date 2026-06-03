use std::path::PathBuf;
use std::sync::Arc;

use sentinel_core::models::mission::{MissionArtifact, MissionObservation};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::services::database::DatabaseService;

/// Save an artifact to the filesystem and record metadata in the database.
pub async fn save_mission_artifact(
    db: &Arc<DatabaseService>,
    app_data_dir: &PathBuf,
    mission_id: &str,
    run_id: &str,
    step_id: Option<&str>,
    artifact_type: &str,
    extension: &str,
    data: &[u8],
    metadata: Option<serde_json::Value>,
) -> Result<MissionArtifact, String> {
    let artifact_id = Uuid::new_v4().to_string();

    let dir = app_data_dir
        .join("missions")
        .join(mission_id)
        .join("runs")
        .join(run_id)
        .join("artifacts");

    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create artifact directory: {e}"))?;

    let filename = format!("{artifact_id}.{extension}");
    let file_path = dir.join(&filename);

    tokio::fs::write(&file_path, data)
        .await
        .map_err(|e| format!("Failed to write artifact file: {e}"))?;

    let content_hash = {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    };

    let uri = file_path.to_string_lossy().to_string();
    let size_bytes = data.len() as i64;
    let metadata_json = metadata.map(|m| serde_json::to_string(&m).unwrap_or_default());

    // Check for content change vs previous artifact of same type
    let previous_hash = db
        .get_latest_artifact_hash(mission_id, artifact_type)
        .await
        .unwrap_or(None);

    let artifact = db
        .save_mission_artifact_record(
            &artifact_id,
            mission_id,
            run_id,
            step_id,
            artifact_type,
            "filesystem",
            &uri,
            size_bytes,
            &content_hash,
            metadata_json.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to save artifact record: {e}"))?;

    // Auto-generate content_changed observation if hash differs
    if let Some(prev) = previous_hash {
        if prev != content_hash {
            let _ = save_mission_observation(
                db,
                mission_id,
                run_id,
                step_id,
                "content_changed",
                "info",
                &format!("{artifact_type} content changed"),
                Some(&format!("Previous hash: {prev}, New hash: {content_hash}")),
                None,
                Some(&[artifact_id.as_str()]),
            )
            .await;
        }
    }

    Ok(artifact)
}

/// Save an observation to the database.
pub async fn save_mission_observation(
    db: &Arc<DatabaseService>,
    mission_id: &str,
    run_id: &str,
    step_id: Option<&str>,
    observation_type: &str,
    severity: &str,
    title: &str,
    summary: Option<&str>,
    data_json: Option<&str>,
    artifact_ids: Option<&[&str]>,
) -> Result<MissionObservation, String> {
    let id = Uuid::new_v4().to_string();
    let artifact_ids_json = artifact_ids.map(|ids| serde_json::to_string(ids).unwrap_or_default());

    db.save_mission_observation_record(
        &id,
        mission_id,
        run_id,
        step_id,
        observation_type,
        severity,
        title,
        summary,
        data_json,
        artifact_ids_json.as_deref(),
    )
    .await
    .map_err(|e| format!("Failed to save observation: {e}"))
}

/// Clean up old artifacts for a mission based on retention policy.
pub async fn cleanup_mission_artifacts(
    db: &Arc<DatabaseService>,
    _app_data_dir: &PathBuf,
    mission_id: &str,
    max_artifacts: i64,
    max_total_bytes: i64,
) -> Result<u64, String> {
    let artifacts = db
        .list_mission_artifacts(mission_id, None)
        .await
        .map_err(|e| format!("Failed to list artifacts: {e}"))?;

    if artifacts.len() as i64 <= max_artifacts {
        let total_bytes: i64 = artifacts.iter().filter_map(|a| a.size_bytes).sum();
        if total_bytes <= max_total_bytes {
            return Ok(0);
        }
    }

    let mut deleted = 0u64;
    // Sort by created_at ascending (oldest first) — they're already from DB order
    let excess = artifacts.len() as i64 - max_artifacts;
    let to_remove = if excess > 0 { excess as usize } else { 0 };

    for artifact in artifacts.iter().take(to_remove) {
        // Delete file
        let path = PathBuf::from(&artifact.uri);
        if path.exists() {
            let _ = tokio::fs::remove_file(&path).await;
        }
        // Delete DB record
        let _ = db.delete_mission_artifact(&artifact.id).await;
        deleted += 1;
    }

    Ok(deleted)
}
