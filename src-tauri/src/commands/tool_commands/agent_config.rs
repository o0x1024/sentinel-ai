//! Agent configuration commands

use std::sync::Arc;
use sentinel_db::Database;

use serde::{Deserialize, Serialize};

use sentinel_tools::buildin_tools::shell::{set_shell_config, ShellConfig};

/// Agent configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AgentConfig {
    /// Shell/terminal configuration
    pub shell: ShellConfig,
    // Future: Add more agent settings here
    // pub memory: MemoryConfig,
    // pub tools: ToolsConfig,
}

/// Get agent configuration
pub async fn get_agent_config(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AgentConfig, String> {
    // Load shell config from database
    let shell_config = load_shell_config_from_db(db_service.inner()).await;

    Ok(AgentConfig { shell: shell_config })
}

/// Save agent configuration
pub async fn save_agent_config(
    config: AgentConfig,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    // Save shell config to database
    save_shell_config_to_db(&config.shell, db_service.inner()).await?;

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


