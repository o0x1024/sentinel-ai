mod codex_context;
mod compaction;
mod config;
mod llm;
mod lock;
mod paths;
mod prompt;
mod rate_limit;
mod runtime;
mod signal;
mod tenth_man;
mod tooling;
mod types;
mod watchdog;

use crate::args::{AgentAction, AgentCommand};
use crate::output::print_json;
use anyhow::Result;
use config::AgentRuntimeConfig;

pub async fn run(command: AgentCommand) -> Result<()> {
    match command.action {
        AgentAction::Run(args) => runtime::run_supervisor(AgentRuntimeConfig::from(args)).await,
        AgentAction::InstallSupervisor(args) => {
            let report = runtime::install_supervisor(AgentRuntimeConfig::from(args)).await?;
            print_json(&report)
        }
        AgentAction::Worker(args) => runtime::run_worker(AgentRuntimeConfig::from(args)).await,
        AgentAction::Status => {
            let report = runtime::status_report().await?;
            print_json(&report)
        }
        AgentAction::Stop => {
            let report = runtime::request_stop().await?;
            print_json(&report)
        }
    }
}
