use anyhow::Result;
use chrono::Utc;
use sentinel_tools::buildin_tools::shell::{get_shell_config, ShellExecutionMode};
use sentinel_tools::output_storage::{get_host_context_dir, CONTAINER_CONTEXT_DIR};
use sentinel_tools::DockerSandbox;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

const TEAM_V3_ARTIFACT_DIR: &str = "team-v3-artifacts";

#[derive(Debug, Clone)]
pub struct TeamV3ArtifactFileRef {
    pub host_path: String,
    pub container_path: Option<String>,
    pub path: String,
    pub bytes: usize,
}

fn sanitize_path_segment(input: &str, fallback: &str) -> String {
    let sanitized = input
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

fn team_v3_artifact_root() -> Result<PathBuf> {
    Ok(get_host_context_dir().join(TEAM_V3_ARTIFACT_DIR))
}

fn build_container_artifact_path(
    session_dir: &str,
    task_dir: &str,
    filename: &str,
) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        CONTAINER_CONTEXT_DIR, TEAM_V3_ARTIFACT_DIR, session_dir, task_dir, filename
    )
}

async fn maybe_copy_artifact_to_container(
    host_path: &str,
    session_dir: &str,
    task_dir: &str,
    filename: &str,
) -> Option<String> {
    let shell_config = get_shell_config().await;
    let docker_requested = shell_config.default_execution_mode == ShellExecutionMode::Docker
        && shell_config.docker_config.is_some();
    if !docker_requested {
        return None;
    }
    if !DockerSandbox::is_docker_available().await {
        return None;
    }
    let docker_config = shell_config.docker_config?;
    let sandbox = DockerSandbox::new(docker_config);
    let container_path = build_container_artifact_path(session_dir, task_dir, filename);
    match sandbox
        .copy_file_to_container(host_path, container_path.as_str())
        .await
    {
        Ok(_) => Some(container_path),
        Err(err) => {
            tracing::warn!("Failed to copy Team V3 artifact to container: {}", err);
            None
        }
    }
}

pub async fn persist_team_v3_task_output_artifact(
    session_id: &str,
    task_id: &str,
    task_key: &str,
    member_id: &str,
    task_title: &str,
    content: &str,
) -> Result<TeamV3ArtifactFileRef> {
    let root = team_v3_artifact_root()?;
    let session_dir = sanitize_path_segment(session_id, "session");
    let task_dir = sanitize_path_segment(task_key, "task");
    let target_dir = root.join(&session_dir).join(&task_dir);
    fs::create_dir_all(&target_dir).await?;

    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string();
    let filename = format!(
        "{}-{}.md",
        timestamp,
        sanitize_path_segment(&Uuid::new_v4().to_string(), "artifact")
    );
    let path = target_dir.join(&filename);

    let file_body = format!(
        "# Team V3 Artifact\n\nsession_id: {}\ntask_id: {}\ntask_key: {}\nmember_id: {}\ntask_title: {}\ncreated_at: {}\n\n---\n\n{}",
        session_id,
        task_id,
        task_key,
        member_id,
        task_title,
        Utc::now().to_rfc3339(),
        content
    );
    fs::write(&path, file_body).await?;
    let host_path = path.to_string_lossy().to_string();
    let container_path = maybe_copy_artifact_to_container(
        host_path.as_str(),
        session_dir.as_str(),
        task_dir.as_str(),
        filename.as_str(),
    )
    .await;
    let runtime_path = container_path
        .clone()
        .unwrap_or_else(|| host_path.clone());

    Ok(TeamV3ArtifactFileRef {
        host_path,
        container_path,
        path: runtime_path,
        bytes: content.as_bytes().len(),
    })
}
