use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::Utc;
use sentinel_tools::buildin_tools::shell::{get_shell_config, ShellExecutionMode};
use sentinel_tools::output_storage::{get_host_context_dir, CONTAINER_CONTEXT_DIR};
use sentinel_tools::DockerSandbox;
use sentinel_tools::DockerSandboxConfig;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

const TEAM_V3_ARTIFACT_DIR: &str = "team-v3-artifacts";

#[derive(Debug, Clone)]
pub struct TeamV3ArtifactFileRef {
    pub host_path: Option<String>,
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

fn build_container_artifact_path(session_dir: &str, task_dir: &str, filename: &str) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        CONTAINER_CONTEXT_DIR, TEAM_V3_ARTIFACT_DIR, session_dir, task_dir, filename
    )
}

async fn execute_checked(
    sandbox: &DockerSandbox,
    command: &str,
    timeout_secs: u64,
    action: &str,
) -> Result<()> {
    let (_stdout, stderr, exit_code) = sandbox.execute(command, timeout_secs).await?;
    if exit_code == 0 {
        return Ok(());
    }
    Err(anyhow!(
        "Failed to {} in container (exit code {}): {}",
        action,
        exit_code,
        stderr.trim()
    ))
}

async fn persist_artifact_to_container(
    docker_config: DockerSandboxConfig,
    session_dir: &str,
    task_dir: &str,
    filename: &str,
    file_body: &str,
    bytes: usize,
) -> Result<TeamV3ArtifactFileRef> {
    let sandbox = DockerSandbox::new(docker_config);
    let container_path = build_container_artifact_path(session_dir, task_dir, filename);
    let target_dir = format!(
        "{}/{}/{}",
        CONTAINER_CONTEXT_DIR, TEAM_V3_ARTIFACT_DIR, session_dir
    );
    let mkdir_cmd = format!("mkdir -p {}/{}", target_dir, task_dir);
    execute_checked(
        &sandbox,
        mkdir_cmd.as_str(),
        15,
        "create artifact directory",
    )
    .await?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(file_body.as_bytes());
    let write_cmd = format!("echo '{}' | base64 -d > {}", encoded, container_path);
    execute_checked(&sandbox, write_cmd.as_str(), 30, "write artifact file").await?;

    Ok(TeamV3ArtifactFileRef {
        host_path: None,
        container_path: Some(container_path.clone()),
        path: container_path,
        bytes,
    })
}

async fn persist_artifact_to_host(
    session_dir: &str,
    task_dir: &str,
    filename: &str,
    file_body: &str,
    bytes: usize,
) -> Result<TeamV3ArtifactFileRef> {
    let root = team_v3_artifact_root()?;
    let target_dir = root.join(session_dir).join(task_dir);
    fs::create_dir_all(&target_dir).await?;
    let path = target_dir.join(filename);
    fs::write(&path, file_body).await?;
    let host_path = path.to_string_lossy().to_string();

    Ok(TeamV3ArtifactFileRef {
        host_path: Some(host_path.clone()),
        container_path: None,
        path: host_path,
        bytes,
    })
}

pub async fn persist_team_v3_task_output_artifact(
    session_id: &str,
    task_id: &str,
    task_key: &str,
    member_id: &str,
    task_title: &str,
    content: &str,
) -> Result<TeamV3ArtifactFileRef> {
    let session_dir = sanitize_path_segment(session_id, "session");
    let task_dir = sanitize_path_segment(task_key, "task");
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string();
    let filename = format!(
        "{}-{}.md",
        timestamp,
        sanitize_path_segment(&Uuid::new_v4().to_string(), "artifact")
    );
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
    let bytes = content.as_bytes().len();

    let shell_config = get_shell_config().await;
    if shell_config.default_execution_mode == ShellExecutionMode::Docker {
        let docker_config = shell_config
            .docker_config
            .ok_or_else(|| anyhow!("Docker mode is enabled but docker config is missing"))?;
        if !DockerSandbox::is_docker_available().await {
            return Err(anyhow!(
                "Docker mode is enabled but Docker is not available, artifact will not be persisted on host"
            ));
        }
        return persist_artifact_to_container(
            docker_config,
            session_dir.as_str(),
            task_dir.as_str(),
            filename.as_str(),
            file_body.as_str(),
            bytes,
        )
        .await;
    }

    persist_artifact_to_host(
        session_dir.as_str(),
        task_dir.as_str(),
        filename.as_str(),
        file_body.as_str(),
        bytes,
    )
    .await
}
