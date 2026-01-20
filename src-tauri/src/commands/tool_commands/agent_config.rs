//! Agent configuration commands

use std::sync::Arc;
use sentinel_db::Database;

use serde::{Deserialize, Serialize};

use sentinel_tools::buildin_tools::shell::{set_shell_config, ShellConfig, ShellExecutionMode};
use sentinel_tools::docker_sandbox::DockerSandboxConfig;

/// Execution mode for shell and terminal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Execute in Docker container
    Docker,
    /// Execute on host machine
    Host,
}

impl From<ExecutionMode> for ShellExecutionMode {
    fn from(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Docker => ShellExecutionMode::Docker,
            ExecutionMode::Host => ShellExecutionMode::Host,
        }
    }
}

impl From<ShellExecutionMode> for ExecutionMode {
    fn from(mode: ShellExecutionMode) -> Self {
        match mode {
            ShellExecutionMode::Docker => ExecutionMode::Docker,
            ShellExecutionMode::Host => ExecutionMode::Host,
        }
    }
}

/// Terminal configuration for interactive shell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Docker image for interactive terminal
    pub docker_image: String,
    /// Default execution mode (Docker or Host)
    pub default_execution_mode: ExecutionMode,
    /// Docker container memory limit (e.g., "512m", "2g")
    #[serde(default = "default_docker_memory_limit")]
    pub docker_memory_limit: String,
    /// Docker container CPU limit (e.g., "1.0", "4.0")
    #[serde(default = "default_docker_cpu_limit")]
    pub docker_cpu_limit: String,
}

fn default_docker_memory_limit() -> String {
    "2g".to_string()
}

fn default_docker_cpu_limit() -> String {
    "4.0".to_string()
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            docker_image: "sentinel-sandbox:latest".to_string(),
            default_execution_mode: ExecutionMode::Docker,
            docker_memory_limit: default_docker_memory_limit(),
            docker_cpu_limit: default_docker_cpu_limit(),
        }
    }
}

/// Image attachment processing mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageAttachmentMode {
    /// Use local OCR to extract text, do not upload images to the model
    LocalOcr,
    /// Upload images to the model for vision understanding
    ModelVision,
}

/// Image attachment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachmentConfig {
    /// Default processing mode
    pub mode: ImageAttachmentMode,
    /// Whether uploading images to model is allowed (privacy gate)
    pub allow_upload_to_model: bool,
}

impl Default for ImageAttachmentConfig {
    fn default() -> Self {
        Self {
            mode: ImageAttachmentMode::LocalOcr,
            allow_upload_to_model: false,
        }
    }
}

/// Subagent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentConfig {
    /// Default timeout for subagent tasks in seconds (default: 600 = 10 minutes)
    #[serde(default = "default_subagent_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_subagent_timeout_secs() -> u64 {
    600 // 10 minutes
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            timeout_secs: default_subagent_timeout_secs(),
        }
    }
}

/// Agent configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AgentConfig {
    /// Shell/terminal configuration
    pub shell: ShellConfig,
    /// Terminal (interactive shell) configuration
    #[serde(default)]
    pub terminal: TerminalConfig,
    /// Image attachment configuration
    #[serde(default)]
    pub image_attachments: ImageAttachmentConfig,
    /// Subagent configuration
    #[serde(default)]
    pub subagent: SubagentConfig,
}

/// Get agent configuration
pub async fn get_agent_config(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AgentConfig, String> {
    // Load shell config from database
    let shell_config = load_shell_config_from_db(db_service.inner()).await;
    // Load terminal config from database
    let terminal_config = load_terminal_config_from_db(db_service.inner()).await;
    // Load image attachment config from database
    let image_config = load_image_attachment_config_from_db(db_service.inner()).await;
    // Load subagent config from database
    let subagent_config = load_subagent_config_from_db(db_service.inner()).await;

    Ok(AgentConfig { 
        shell: shell_config,
        terminal: terminal_config,
        image_attachments: image_config,
        subagent: subagent_config,
    })
}

/// Save agent configuration
pub async fn save_agent_config(
    config: AgentConfig,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    // Save shell config to database
    save_shell_config_to_db(&config.shell, db_service.inner()).await?;
    // Save terminal config to database
    save_terminal_config_to_db(&config.terminal, db_service.inner()).await?;
    // Save image attachment config to database
    save_image_attachment_config_to_db(&config.image_attachments, db_service.inner()).await?;
    // Save subagent config to database
    save_subagent_config_to_db(&config.subagent, db_service.inner()).await?;

    // Update in-memory config
    // Important: frontend may omit docker_config; keep a valid docker_config so docker mode works.
    let mut shell_cfg = config.shell.clone();
    // Unify execution mode with terminal setting.
    shell_cfg.default_execution_mode = config.terminal.default_execution_mode.into();
    // Ensure docker_config exists and matches selected docker image.
    if shell_cfg.docker_config.is_none() {
        shell_cfg.docker_config = Some(DockerSandboxConfig::default());
    }
    if let Some(ref mut docker_cfg) = shell_cfg.docker_config {
        docker_cfg.image = config.terminal.docker_image.clone();
        docker_cfg.memory_limit = config.terminal.docker_memory_limit.clone();
        docker_cfg.cpu_limit = config.terminal.docker_cpu_limit.clone();
    }
    set_shell_config(shell_cfg).await;

    // Update global subagent timeout
    sentinel_tools::buildin_tools::subagent_tool::set_default_subagent_timeout(config.subagent.timeout_secs);

    tracing::info!("Agent config saved successfully");
    Ok(())
}

/// Initialize agent configuration from database on startup
pub async fn init_agent_config(db: &sentinel_db::DatabaseService) -> Result<(), String> {
    let mut shell_cfg = load_shell_config_from_db(db).await;
    let terminal_config = load_terminal_config_from_db(db).await;
    let subagent_config = load_subagent_config_from_db(db).await;

    // Unify execution mode with terminal setting.
    shell_cfg.default_execution_mode = terminal_config.default_execution_mode.into();
    
    // Ensure docker_config exists and matches selected docker image/resources.
    if shell_cfg.docker_config.is_none() {
        shell_cfg.docker_config = Some(DockerSandboxConfig::default());
    }
    if let Some(ref mut docker_cfg) = shell_cfg.docker_config {
        docker_cfg.image = terminal_config.docker_image.clone();
        docker_cfg.memory_limit = terminal_config.docker_memory_limit.clone();
        docker_cfg.cpu_limit = terminal_config.docker_cpu_limit.clone();
    }
    
    set_shell_config(shell_cfg).await;
    
    // Set global subagent timeout
    sentinel_tools::buildin_tools::subagent_tool::set_default_subagent_timeout(subagent_config.timeout_secs);
    
    tracing::info!("Agent configuration initialized from database");
    Ok(())
}

/// Load shell config from database
pub async fn load_shell_config_from_db(db: &sentinel_db::DatabaseService) -> ShellConfig {
    let mut config = ShellConfig::default();

    // Load default_policy
    if let Ok(Some(value)) = db.get_config("agent", "shell_default_policy").await {
        config.default_policy = match value.as_str() {
            "AlwaysProceed" => sentinel_tools::buildin_tools::shell::ShellDefaultPolicy::AlwaysProceed,
            _ => sentinel_tools::buildin_tools::shell::ShellDefaultPolicy::RequestReview,
        };
    }

    // Load allowed_commands
    if let Ok(Some(value)) = db.get_config("agent", "shell_allowed_commands").await {
        if let Ok(commands) = serde_json::from_str::<Vec<String>>(&value) {
            config.allowed_commands = commands;
        }
    }

    // Load denied_commands
    if let Ok(Some(value)) = db.get_config("agent", "shell_denied_commands").await {
        if let Ok(commands) = serde_json::from_str::<Vec<String>>(&value) {
            config.denied_commands = commands;
        }
    }

    // Load default_execution_mode (unified with terminal config)
    if let Ok(Some(value)) = db.get_config("agent", "default_execution_mode").await {
        config.default_execution_mode = match value.as_str() {
            "docker" => sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker,
            "host" => sentinel_tools::buildin_tools::shell::ShellExecutionMode::Host,
            _ => sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker,
        };
    }

    // Load docker_config with docker_image from terminal config
    // This is critical for Docker mode to work properly
    if let Ok(Some(docker_image)) = db.get_config("agent", "terminal_docker_image").await {
        let mut docker_cfg = DockerSandboxConfig::default();
        docker_cfg.image = docker_image;
        config.docker_config = Some(docker_cfg);
    } else {
        // Ensure docker_config exists with default values
        config.docker_config = Some(DockerSandboxConfig::default());
    }

    config
}

/// Save shell config to database
async fn save_shell_config_to_db(
    config: &ShellConfig,
    db: &sentinel_db::DatabaseService,
) -> Result<(), String> {
    let policy_str = match config.default_policy {
        sentinel_tools::buildin_tools::shell::ShellDefaultPolicy::AlwaysProceed => "AlwaysProceed",
        sentinel_tools::buildin_tools::shell::ShellDefaultPolicy::RequestReview => "RequestReview",
    };

    db.set_config(
        "agent",
        "shell_default_policy",
        policy_str,
        Some("Shell command execution policy"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let allowed_json = serde_json::to_string(&config.allowed_commands).unwrap_or_default();
    db.set_config(
        "agent",
        "shell_allowed_commands",
        &allowed_json,
        Some("Shell commands allowed to execute without confirmation"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let denied_json = serde_json::to_string(&config.denied_commands).unwrap_or_default();
    db.set_config(
        "agent",
        "shell_denied_commands",
        &denied_json,
        Some("Shell commands that require confirmation"),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Load terminal config from database
pub async fn load_terminal_config_from_db(db: &sentinel_db::DatabaseService) -> TerminalConfig {
    let mut config = TerminalConfig::default();

    // Load docker_image
    if let Ok(Some(value)) = db.get_config("agent", "terminal_docker_image").await {
        config.docker_image = value;
    }

    // Load default_execution_mode (unified config key)
    if let Ok(Some(value)) = db.get_config("agent", "default_execution_mode").await {
        config.default_execution_mode = match value.as_str() {
            "docker" => ExecutionMode::Docker,
            "host" => ExecutionMode::Host,
            _ => ExecutionMode::Docker,
        };
    }

    // Load docker_memory_limit
    if let Ok(Some(value)) = db.get_config("agent", "docker_memory_limit").await {
        config.docker_memory_limit = value;
    }

    // Load docker_cpu_limit
    if let Ok(Some(value)) = db.get_config("agent", "docker_cpu_limit").await {
        config.docker_cpu_limit = value;
    }

    config
}

/// Load image attachment config from database
pub async fn load_image_attachment_config_from_db(db: &sentinel_db::DatabaseService) -> ImageAttachmentConfig {
    let mut cfg = ImageAttachmentConfig::default();

    if let Ok(Some(value)) = db.get_config("agent", "image_attachment_mode").await {
        cfg.mode = match value.as_str() {
            "model_vision" => ImageAttachmentMode::ModelVision,
            _ => ImageAttachmentMode::LocalOcr,
        };
    }

    if let Ok(Some(value)) = db.get_config("agent", "allow_image_upload_to_model").await {
        cfg.allow_upload_to_model = value == "1" || value.eq_ignore_ascii_case("true");
    }

    cfg
}

/// Save image attachment config to database
async fn save_image_attachment_config_to_db(
    config: &ImageAttachmentConfig,
    db: &sentinel_db::DatabaseService,
) -> Result<(), String> {
    let mode_str = match config.mode {
        ImageAttachmentMode::LocalOcr => "local_ocr",
        ImageAttachmentMode::ModelVision => "model_vision",
    };

    db.set_config(
        "agent",
        "image_attachment_mode",
        mode_str,
        Some("Default processing mode for image attachments (local_ocr/model_vision)"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "agent",
        "allow_image_upload_to_model",
        if config.allow_upload_to_model { "true" } else { "false" },
        Some("Whether uploading image attachments to model is allowed"),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Save terminal config to database
async fn save_terminal_config_to_db(
    config: &TerminalConfig,
    db: &sentinel_db::DatabaseService,
) -> Result<(), String> {
    db.set_config(
        "agent",
        "terminal_docker_image",
        &config.docker_image,
        Some("Docker image for interactive terminal"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Save unified execution mode
    let mode_str = match config.default_execution_mode {
        ExecutionMode::Docker => "docker",
        ExecutionMode::Host => "host",
    };
    db.set_config(
        "agent",
        "default_execution_mode",
        mode_str,
        Some("Default execution mode for shell and interactive terminal (docker/host)"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Save docker memory limit
    db.set_config(
        "agent",
        "docker_memory_limit",
        &config.docker_memory_limit,
        Some("Docker container memory limit (e.g., 512m, 2g)"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Save docker cpu limit
    db.set_config(
        "agent",
        "docker_cpu_limit",
        &config.docker_cpu_limit,
        Some("Docker container CPU limit (e.g., 1.0, 4.0)"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!(
        "Terminal config saved: execution_mode={:?}, docker_image={}, memory={}, cpu={}",
        config.default_execution_mode,
        config.docker_image,
        config.docker_memory_limit,
        config.docker_cpu_limit
    );

    Ok(())
}

/// Load subagent config from database
pub async fn load_subagent_config_from_db(db: &sentinel_db::DatabaseService) -> SubagentConfig {
    let mut config = SubagentConfig::default();

    if let Ok(Some(value)) = db.get_config("agent", "subagent_timeout_secs").await {
        if let Ok(timeout) = value.parse::<u64>() {
            config.timeout_secs = timeout;
        }
    }

    config
}

/// Save subagent config to database
async fn save_subagent_config_to_db(
    config: &SubagentConfig,
    db: &sentinel_db::DatabaseService,
) -> Result<(), String> {
    db.set_config(
        "agent",
        "subagent_timeout_secs",
        &config.timeout_secs.to_string(),
        Some("Default timeout for subagent tasks in seconds"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Subagent config saved: timeout_secs={}", config.timeout_secs);

    Ok(())
}
