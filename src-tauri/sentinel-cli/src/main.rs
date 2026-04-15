mod agent;
mod arena;
mod args;
mod commands;
mod exploitdb;
mod output;
mod runtime;
mod solver;
mod state;

use anyhow::Result;
use args::{Cli, Command};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    if should_start_exploitdb_sync(&cli.command) {
        exploitdb::start_background_sync().await;
    }

    match cli.command {
        Command::Paths => commands::paths::run(),
        Command::Shell(command) => commands::shell::run(command).await,
        Command::Http(command) => commands::http::run(command).await,
        Command::Challenge(command) => commands::challenge::run(command).await,
        Command::Solve(command) => commands::solve::run(command).await,
        Command::Agent(command) => commands::agent::run(command).await,
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sentinel_cli=info".parse().unwrap()),
        )
        .with_target(false)
        .with_ansi(false)
        .without_time()
        .try_init();
}

fn should_start_exploitdb_sync(command: &Command) -> bool {
    match command {
        Command::Solve(_) => true,
        Command::Agent(agent_command) => matches!(
            agent_command.action,
            args::AgentAction::Run(_) | args::AgentAction::Worker(_)
        ),
        _ => false,
    }
}
