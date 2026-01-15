//! Agent configuration commands

use std::sync::Arc;
use sentinel_db::Database;

use serde::{Deserialize, Serialize};

use sentinel_tools::buildin_tools::shell::{set_shell_config, ShellConfig};

/// Terminal configuration for interactive shell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Docker image for interactive terminal
    pub docker_image: String,
    /// Use Docker for terminal (vs host shell)
    pub use_docker: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            docker_image: "sentinel-sandbox:latest".to_string(),
            use_docker: true,
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
}

/// Get agent configuration
pub async fn get_agent_config(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AgentConfig, String> {
    // Load shell config from database
    let shell_config = load_shell_config_from_db(db_service.inner()).await;
    // Load terminal config from database
    let terminal_config = load_terminal_config_from_db(db_service.inner()).await;

    Ok(AgentConfig { 
        shell: shell_config,
        terminal: terminal_config,
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

    // Update in-memory config
    set_shell_config(config.shell).await;

    tracing::info!("Agent config saved successfully");
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

    // Load use_docker
    if let Ok(Some(value)) = db.get_config("agent", "terminal_use_docker").await {
        config.use_docker = value == "true";
    }

    config
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

    db.set_config(
        "agent",
        "terminal_use_docker",
        if config.use_docker { "true" } else { "false" },
        Some("Use Docker for interactive terminal"),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
