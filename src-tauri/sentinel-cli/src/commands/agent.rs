use crate::agent;
use crate::args::AgentCommand;
use anyhow::Result;

pub async fn run(command: AgentCommand) -> Result<()> {
    agent::run(command).await
}
