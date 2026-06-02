use crate::buildin_tools::file_context::hash_bytes;
use crate::buildin_tools::shell::{get_shell_config, ShellExecutionMode};
use crate::docker_sandbox::{DockerSandbox, DockerSandboxConfig};
use crate::output_storage::{get_execution_context_dir, CONTAINER_CONTEXT_DIR};
use crate::terminal::{ExecutionMode as TerminalExecutionMode, TERMINAL_MANAGER};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use walkdir::WalkDir;

tokio::task_local! {
    static FILE_RUNTIME_CONTEXT: FileRuntimeContext;
}

const DEFAULT_DOCKER_WORKDIR: &str = "/workspace";
const DEFAULT_CONTAINER_NAME: &str = "sentinel-sandbox-main";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePathKind {
    Missing,
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct FileRuntimeContext {
    mode: FileRuntimeMode,
}

#[derive(Debug, Clone)]
pub struct RuntimeFileEntry {
    pub logical_path: String,
    pub display_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeFileListing {
    pub base_path: String,
    pub files: Vec<RuntimeFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileRuntimeMetadata {
    pub execution_environment: String,
    pub working_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_ref: Option<String>,
}

#[derive(Debug, Clone)]
enum FileRuntimeMode {
    Host(HostFileRuntimeContext),
    Docker(DockerFileRuntimeContext),
}

#[derive(Debug, Clone)]
struct HostFileRuntimeContext {
    working_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct DockerFileRuntimeContext {
    docker_config: DockerSandboxConfig,
    working_dir: String,
    container_ref: Option<String>,
}

impl FileRuntimeContext {
    pub fn host(working_dir: Option<String>) -> Self {
        Self {
            mode: FileRuntimeMode::Host(HostFileRuntimeContext {
                working_dir: resolve_host_working_dir(working_dir.as_deref())
                    .unwrap_or_else(|_| default_host_working_dir()),
            }),
        }
    }

    fn docker(
        docker_config: DockerSandboxConfig,
        working_dir: Option<String>,
        container_ref: Option<String>,
    ) -> Self {
        Self {
            mode: FileRuntimeMode::Docker(DockerFileRuntimeContext {
                docker_config,
                working_dir: working_dir.unwrap_or_else(|| DEFAULT_DOCKER_WORKDIR.to_string()),
                container_ref,
            }),
        }
    }
}

pub async fn with_file_runtime_context<Fut, T>(context: FileRuntimeContext, future: Fut) -> T
where
    Fut: Future<Output = T>,
{
    FILE_RUNTIME_CONTEXT.scope(context, future).await
}

/// Resolve Docker working directory for a given execution.
/// If an execution_id is provided, returns the per-session directory
/// (`/workspace/context/session_<id>`); otherwise falls back to `/workspace`.
pub fn docker_session_working_dir(execution_id: Option<&str>) -> String {
    match execution_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => get_execution_context_dir(CONTAINER_CONTEXT_DIR, Some(id)),
        None => DEFAULT_DOCKER_WORKDIR.to_string(),
    }
}

pub async fn build_default_file_runtime_context(
    host_working_directory: Option<&str>,
    execution_id: Option<&str>,
) -> FileRuntimeContext {
    let config = get_shell_config().await;
    match config.default_execution_mode {
        ShellExecutionMode::Host => {
            FileRuntimeContext::host(host_working_directory.map(str::to_string))
        }
        ShellExecutionMode::Docker => {
            let mut docker_config = config.docker_config.unwrap_or_default();
            if docker_config.container_name.is_none() {
                docker_config.container_name = Some(DEFAULT_CONTAINER_NAME.to_string());
            }
            FileRuntimeContext::docker(
                docker_config.clone(),
                Some(docker_session_working_dir(execution_id)),
                docker_config.container_name.clone(),
            )
        }
    }
}

pub async fn build_file_runtime_context(
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
    execution_id: Option<&str>,
) -> FileRuntimeContext {
    let Some(session_id) = active_terminal_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return build_default_file_runtime_context(host_working_directory, execution_id).await;
    };

    let Some(session_lock) = TERMINAL_MANAGER.get_session(session_id).await else {
        return build_default_file_runtime_context(host_working_directory, execution_id).await;
    };

    let session = session_lock.read().await;
    match session.config.execution_mode {
        TerminalExecutionMode::Host => FileRuntimeContext::host(
            host_working_directory
                .map(str::to_string)
                .or_else(|| session.config.working_dir.clone()),
        ),
        TerminalExecutionMode::Docker => {
            let mut docker_config = get_shell_config().await.docker_config.unwrap_or_default();
            docker_config.image = session.config.docker_image.clone();
            docker_config.reuse_container = session.config.reuse_container;
            docker_config.container_name = session
                .config
                .container_name
                .clone()
                .or_else(|| docker_config.container_name.clone())
                .or_else(|| Some(DEFAULT_CONTAINER_NAME.to_string()));
            let container_ref = session
                .config
                .container_name
                .clone()
                .or_else(|| session.container_id())
                .or_else(|| docker_config.container_name.clone());
            FileRuntimeContext::docker(
                docker_config,
                session.config.working_dir.clone(),
                container_ref,
            )
        }
    }
}

pub async fn get_path_kind(raw: &str) -> Result<FilePathKind, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => get_host_path_kind(&context, raw).await,
        FileRuntimeMode::Docker(context) => get_docker_path_kind(&context, raw).await,
    }
}

pub async fn path_exists(raw: &str) -> Result<bool, String> {
    Ok(!matches!(get_path_kind(raw).await?, FilePathKind::Missing))
}

pub async fn resolve_runtime_path(raw: &str) -> Result<String, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => Ok(resolve_host_path(&context, raw)?
            .to_string_lossy()
            .to_string()),
        FileRuntimeMode::Docker(context) => resolve_docker_path(&context.working_dir, raw),
    }
}

pub async fn list_files_under(base: Option<&str>) -> Result<RuntimeFileListing, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => list_host_files_under(&context, base).await,
        FileRuntimeMode::Docker(context) => list_docker_files_under(&context, base).await,
    }
}

pub async fn read_path_bytes(raw: &str) -> Result<Vec<u8>, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => {
            let path = resolve_host_path(&context, raw)?;
            tokio::fs::read(&path)
                .await
                .map_err(|error| error.to_string())
        }
        FileRuntimeMode::Docker(context) => read_docker_path_bytes(&context, raw).await,
    }
}

pub async fn write_path_bytes(
    raw: &str,
    bytes: &[u8],
    create_parent_dirs: bool,
) -> Result<(), String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => {
            let path = resolve_host_path(&context, raw)?;
            if create_parent_dirs {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|error| error.to_string())?;
                }
            }
            tokio::fs::write(path, bytes)
                .await
                .map_err(|error| error.to_string())
        }
        FileRuntimeMode::Docker(context) => {
            write_docker_path_bytes(&context, raw, bytes, create_parent_dirs).await
        }
    }
}

pub async fn snapshot_key(raw: &str) -> Result<String, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => {
            let path = resolve_host_path(&context, raw)?;
            Ok(format!("host:{}", path.to_string_lossy()))
        }
        FileRuntimeMode::Docker(context) => {
            let path = resolve_docker_path(&context.working_dir, raw)?;
            let container_ref = docker_container_ref(&context);
            Ok(format!("docker:{}:{}", container_ref, path))
        }
    }
}

pub async fn revision_token(raw: &str) -> Result<String, String> {
    let bytes = read_path_bytes(raw).await?;
    Ok(hash_bytes(&bytes))
}

pub fn current_runtime_metadata() -> FileRuntimeMetadata {
    match effective_context() {
        FileRuntimeMode::Host(context) => FileRuntimeMetadata {
            execution_environment: "host".to_string(),
            working_dir: context.working_dir.to_string_lossy().to_string(),
            container_ref: None,
        },
        FileRuntimeMode::Docker(context) => {
            let container_ref = docker_container_ref(&context);
            FileRuntimeMetadata {
                execution_environment: "docker".to_string(),
                working_dir: context.working_dir.clone(),
                container_ref: Some(container_ref),
            }
        }
    }
}

fn effective_context() -> FileRuntimeMode {
    FILE_RUNTIME_CONTEXT
        .try_with(|context| context.mode.clone())
        .unwrap_or_else(|_| {
            FileRuntimeMode::Host(HostFileRuntimeContext {
                working_dir: default_host_working_dir(),
            })
        })
}

fn default_host_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_host_working_dir(raw: Option<&str>) -> Result<PathBuf, String> {
    let trimmed = raw.unwrap_or_default().trim();
    if trimmed.is_empty() {
        return Ok(default_host_working_dir());
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(default_host_working_dir().join(path))
    }
}

fn resolve_host_path(context: &HostFileRuntimeContext, raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(context.working_dir.join(path))
    }
}

fn resolve_docker_path(working_dir: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Ok(Path::new(working_dir)
            .join(path)
            .to_string_lossy()
            .to_string())
    }
}

async fn get_host_path_kind(
    context: &HostFileRuntimeContext,
    raw: &str,
) -> Result<FilePathKind, String> {
    let path = resolve_host_path(context, raw)?;
    match tokio::fs::metadata(path).await {
        Ok(metadata) if metadata.is_dir() => Ok(FilePathKind::Directory),
        Ok(_) => Ok(FilePathKind::File),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FilePathKind::Missing),
        Err(error) => Err(error.to_string()),
    }
}

async fn list_host_files_under(
    context: &HostFileRuntimeContext,
    base: Option<&str>,
) -> Result<RuntimeFileListing, String> {
    let base_path = match base {
        Some(raw) if !raw.trim().is_empty() => resolve_host_path(context, raw)?,
        _ => context.working_dir.clone(),
    };

    let mut files = Vec::new();
    for entry in WalkDir::new(&base_path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        files.push(RuntimeFileEntry {
            logical_path: path.to_string_lossy().to_string(),
            display_path: relative_display_path(&base_path, path),
        });
    }

    files.sort_by(|left, right| left.display_path.cmp(&right.display_path));
    Ok(RuntimeFileListing {
        base_path: base_path.to_string_lossy().to_string(),
        files,
    })
}

async fn get_docker_path_kind(
    context: &DockerFileRuntimeContext,
    raw: &str,
) -> Result<FilePathKind, String> {
    let path = resolve_docker_path(&context.working_dir, raw)?;
    let output = run_docker_exec(
        context,
        &format!(
            "if [ -d {} ]; then printf directory; elif [ -e {} ]; then printf file; else printf missing; fi",
            shell_quote(&path),
            shell_quote(&path),
        ),
        None,
    )
    .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    match stdout.trim() {
        "directory" => Ok(FilePathKind::Directory),
        "file" => Ok(FilePathKind::File),
        "missing" => Ok(FilePathKind::Missing),
        other => Err(format!("unexpected docker file status '{}'", other)),
    }
}

async fn list_docker_files_under(
    context: &DockerFileRuntimeContext,
    base: Option<&str>,
) -> Result<RuntimeFileListing, String> {
    let base_path = match base {
        Some(raw) if !raw.trim().is_empty() => resolve_docker_path(&context.working_dir, raw)?,
        _ => context.working_dir.clone(),
    };
    let output = run_docker_exec(
        context,
        &format!("find {} -type f -print", shell_quote(&base_path)),
        None,
    )
    .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|logical_path| RuntimeFileEntry {
            logical_path: logical_path.to_string(),
            display_path: relative_display_path(Path::new(&base_path), Path::new(logical_path)),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.display_path.cmp(&right.display_path));
    Ok(RuntimeFileListing { base_path, files })
}

async fn read_docker_path_bytes(
    context: &DockerFileRuntimeContext,
    raw: &str,
) -> Result<Vec<u8>, String> {
    match get_docker_path_kind(context, raw).await? {
        FilePathKind::Missing => Err("No such file or directory (os error 2)".to_string()),
        FilePathKind::Directory => Err("Is a directory (os error 21)".to_string()),
        FilePathKind::File => {
            let path = resolve_docker_path(&context.working_dir, raw)?;
            let output =
                run_docker_exec(context, &format!("cat -- {}", shell_quote(&path)), None).await?;
            Ok(output.stdout)
        }
    }
}

async fn write_docker_path_bytes(
    context: &DockerFileRuntimeContext,
    raw: &str,
    bytes: &[u8],
    create_parent_dirs: bool,
) -> Result<(), String> {
    let path = resolve_docker_path(&context.working_dir, raw)?;
    let command = if create_parent_dirs {
        let parent = Path::new(&path)
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .to_string_lossy()
            .to_string();
        format!(
            "mkdir -p -- {} && cat > {}",
            shell_quote(&parent),
            shell_quote(&path),
        )
    } else {
        format!("cat > {}", shell_quote(&path))
    };

    run_docker_exec(context, &command, Some(bytes)).await?;
    Ok(())
}

fn docker_container_ref(context: &DockerFileRuntimeContext) -> String {
    context
        .container_ref
        .clone()
        .or_else(|| context.docker_config.container_name.clone())
        .unwrap_or_else(|| DEFAULT_CONTAINER_NAME.to_string())
}

async fn ensure_docker_container_ref(context: &DockerFileRuntimeContext) -> Result<String, String> {
    let requested_ref = docker_container_ref(context);
    if inspect_container_running(&requested_ref).await? {
        return Ok(requested_ref);
    }

    let sandbox = DockerSandbox::new(context.docker_config.clone());
    sandbox
        .execute("true", 5)
        .await
        .map_err(|error| format!("failed to prepare docker sandbox: {}", error))?;

    let fallback_ref = context
        .docker_config
        .container_name
        .clone()
        .unwrap_or(requested_ref);
    if inspect_container_running(&fallback_ref).await? {
        Ok(fallback_ref)
    } else {
        Err("docker sandbox container is not running".to_string())
    }
}

async fn inspect_container_running(container_ref: &str) -> Result<bool, String> {
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container_ref])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No such object") {
            return Ok(false);
        }
        return Err(stderr.trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim() == "true")
}

async fn run_docker_exec(
    context: &DockerFileRuntimeContext,
    command: &str,
    stdin_bytes: Option<&[u8]>,
) -> Result<DockerExecOutput, String> {
    let container_ref = ensure_docker_container_ref(context).await?;
    let mut process = Command::new("docker");
    process.arg("exec");
    if stdin_bytes.is_some() {
        process.arg("-i");
    }
    process
        .arg(&container_ref)
        .arg("sh")
        .arg("-lc")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin_bytes.is_some() {
        process.stdin(Stdio::piped());
    }

    let mut child = process.spawn().map_err(|error| error.to_string())?;
    if let Some(bytes) = stdin_bytes {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open docker exec stdin".to_string())?;
        stdin
            .write_all(bytes)
            .await
            .map_err(|error| error.to_string())?;
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(DockerExecOutput {
            stdout: output.stdout,
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("docker exec failed with status {}", output.status))
        } else {
            Err(stderr)
        }
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn relative_display_path(base_path: &Path, candidate: &Path) -> String {
    if candidate == base_path {
        return candidate
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
    }
    candidate
        .strip_prefix(base_path)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| candidate.to_string_lossy().to_string())
}

struct DockerExecOutput {
    stdout: Vec<u8>,
}

#[derive(Debug)]
pub struct RuntimeProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
}

/// Execute an external program in the current file-runtime context.
///
/// Host mode: spawns the process with the runtime working directory as cwd.
/// Docker mode: runs via `docker exec -w <workdir>` in the active container.
pub async fn execute_in_runtime(
    program: &str,
    args: &[String],
) -> Result<RuntimeProcessOutput, String> {
    match effective_context() {
        FileRuntimeMode::Host(context) => {
            let output = Command::new(program)
                .args(args)
                .current_dir(&context.working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("{}: {}", program, e))?;
            Ok(RuntimeProcessOutput {
                success: output.status.success(),
                stdout: output.stdout,
                stderr: output.stderr,
            })
        }
        FileRuntimeMode::Docker(context) => {
            let container_ref = ensure_docker_container_ref(&context).await?;
            let output = Command::new("docker")
                .args(["exec", "-w", &context.working_dir, &container_ref, program])
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("docker exec {}: {}", program, e))?;
            Ok(RuntimeProcessOutput {
                success: output.status.success(),
                stdout: output.stdout,
                stderr: output.stderr,
            })
        }
    }
}
