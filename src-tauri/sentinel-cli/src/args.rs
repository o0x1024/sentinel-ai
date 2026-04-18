use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "sentinel-cli")]
#[command(about = "Headless Sentinel CLI for isolated and CLI-only environments")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Paths,
    Shell(ShellCommand),
    Http(HttpCommand),
    Challenge(ChallengeCommand),
    Solve(SolveCommand),
    Agent(AgentCommand),
}

#[derive(Debug, Args)]
pub struct ShellCommand {
    #[command(subcommand)]
    pub action: ShellAction,
}

#[derive(Debug, Subcommand)]
pub enum ShellAction {
    Run(ShellRunArgs),
}

#[derive(Debug, Args)]
pub struct ShellRunArgs {
    #[arg(long)]
    pub command: String,
    #[arg(long)]
    pub cwd: Option<PathBuf>,
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, Args)]
pub struct HttpCommand {
    #[command(subcommand)]
    pub action: HttpAction,
}

#[derive(Debug, Subcommand)]
pub enum HttpAction {
    Request(HttpRequestArgs),
}

#[derive(Debug, Args)]
pub struct HttpRequestArgs {
    #[arg(long)]
    pub url: String,
    #[arg(long, default_value = "GET")]
    pub method: String,
    #[arg(long = "header")]
    pub headers: Vec<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long, default_value_t = 30)]
    pub timeout_secs: u64,
    #[arg(long, default_value_t = true)]
    pub follow_redirects: bool,
    #[arg(long, default_value_t = 100_000)]
    pub max_body_chars: usize,
}

#[derive(Debug, Args)]
pub struct ChallengeCommand {
    #[command(subcommand)]
    pub action: ChallengeAction,
}

#[derive(Debug, Subcommand)]
pub enum ChallengeAction {
    Config(ChallengeConfigArgs),
    ShowConfig,
    Doctor,
    List,
    Start(ChallengeCodeArgs),
    Stop(ChallengeCodeArgs),
    Submit(ChallengeSubmitArgs),
    Hint(ChallengeCodeArgs),
}

#[derive(Debug, Args)]
pub struct ChallengeConfigArgs {
    #[arg(long)]
    pub base_url: String,
    #[arg(long)]
    pub agent_token: String,
}

#[derive(Debug, Args)]
pub struct ChallengeCodeArgs {
    #[arg(long)]
    pub code: String,
}

#[derive(Debug, Args)]
pub struct ChallengeSubmitArgs {
    #[arg(long)]
    pub code: String,
    #[arg(long)]
    pub flag: String,
}

#[derive(Debug, Args)]
pub struct SolveCommand {
    #[command(subcommand)]
    pub action: SolveAction,
}

#[derive(Debug, Subcommand)]
pub enum SolveAction {
    One(SolveOneArgs),
    All(SolveAllArgs),
}

#[derive(Debug, Args)]
pub struct SolveOneArgs {
    #[arg(long)]
    pub code: String,
    #[arg(long, default_value_t = false)]
    pub allow_hint: bool,
    #[arg(long, default_value_t = true)]
    pub stop_when_done: bool,
    #[arg(long, default_value_t = 12)]
    pub max_requests_per_target: usize,
}

#[derive(Debug, Args)]
pub struct SolveAllArgs {
    #[arg(long, default_value_t = false)]
    pub allow_hint: bool,
    #[arg(long, default_value_t = true)]
    pub stop_when_done: bool,
    #[arg(long, default_value_t = 12)]
    pub max_requests_per_target: usize,
    #[arg(long)]
    pub max_level: Option<i32>,
    #[arg(long = "difficulty")]
    pub difficulties: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub include_solved: bool,
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct AgentCommand {
    #[command(subcommand)]
    pub action: AgentAction,
}

#[derive(Debug, Subcommand)]
pub enum AgentAction {
    Run(AgentRuntimeArgs),
    InstallSupervisor(AgentRuntimeArgs),
    #[command(hide = true)]
    Worker(AgentRuntimeArgs),
    Status,
    Stop,
}

#[derive(Debug, Args, Clone)]
pub struct AgentRuntimeArgs {
    #[arg(long, default_value_t = false)]
    pub allow_hint: bool,
    #[arg(long, default_value_t = true)]
    pub stop_when_done: bool,
    #[arg(long, default_value_t = 12)]
    pub max_requests_per_target: usize,
    #[arg(long)]
    pub max_level: Option<i32>,
    #[arg(long = "difficulty")]
    pub difficulties: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub include_solved: bool,
    #[arg(long)]
    pub limit: Option<usize>,
    #[arg(long, default_value_t = 1)]
    pub max_concurrent_challenges: usize,
    #[arg(long, default_value_t = 1)]
    pub max_attempts_per_challenge: usize,
    #[arg(long, default_value_t = 300)]
    pub failure_cooldown_secs: u64,
    #[arg(long, default_value_t = 45)]
    pub loop_interval_secs: u64,
    #[arg(long, default_value_t = 5)]
    pub heartbeat_interval_secs: u64,
    #[arg(long, default_value_t = 30)]
    pub heartbeat_timeout_secs: u64,
    #[arg(long, default_value_t = 3)]
    pub restart_delay_secs: u64,
    #[arg(long, default_value_t = 10)]
    pub max_steps_per_challenge: usize,
    #[arg(long, default_value_t = 120)]
    pub llm_timeout_secs: u64,
    #[arg(long, default_value_t = 1200)]
    pub max_challenge_duration_secs: u64,
}
