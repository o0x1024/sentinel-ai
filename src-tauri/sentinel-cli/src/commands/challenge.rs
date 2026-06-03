use crate::arena::{load_arena_config, save_arena_config, ArenaClient, ArenaConfig};
use crate::args::{
    ChallengeAction, ChallengeCodeArgs, ChallengeCommand, ChallengeConfigArgs, ChallengeSubmitArgs,
};
use crate::output::print_json;
use crate::state::sentinel_state_dir;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ConfigSavedReport {
    config_path: String,
    base_url: String,
}

pub async fn run(command: ChallengeCommand) -> Result<()> {
    match command.action {
        ChallengeAction::Config(args) => configure(args).await,
        ChallengeAction::ShowConfig => show_config().await,
        ChallengeAction::Doctor => doctor().await,
        ChallengeAction::List => list().await,
        ChallengeAction::Start(args) => start(args).await,
        ChallengeAction::Stop(args) => stop(args).await,
        ChallengeAction::Submit(args) => submit(args).await,
        ChallengeAction::Hint(args) => hint(args).await,
    }
}

async fn configure(args: ChallengeConfigArgs) -> Result<()> {
    let config = ArenaConfig {
        base_url: args.base_url,
        agent_token: args.agent_token,
    };
    let config_path = save_arena_config(sentinel_state_dir(), &config).await?;
    print_json(&ConfigSavedReport {
        config_path: config_path.display().to_string(),
        base_url: config.base_url,
    })
}

async fn show_config() -> Result<()> {
    let config = load_arena_config(sentinel_state_dir()).await?;
    print_json(&config)
}

async fn list() -> Result<()> {
    let arena = load_client().await?;
    let data = arena.list_challenges().await?;
    print_json(&data)
}

async fn doctor() -> Result<()> {
    let arena = load_client().await?;
    let data = arena.list_challenges().await?;
    print_json(&serde_json::json!({
        "ok": true,
        "current_level": data.current_level,
        "visible_challenges": data.total_challenges,
        "solved_challenges": data.solved_challenges,
    }))
}

async fn start(args: ChallengeCodeArgs) -> Result<()> {
    let arena = load_client().await?;
    let data = arena.start_challenge(&args.code).await?;
    print_json(&data)
}

async fn stop(args: ChallengeCodeArgs) -> Result<()> {
    let arena = load_client().await?;
    let data = arena.stop_challenge(&args.code).await?;
    print_json(&data)
}

async fn submit(args: ChallengeSubmitArgs) -> Result<()> {
    let arena = load_client().await?;
    let data = arena.submit_flag(&args.code, &args.flag).await?;
    print_json(&data)
}

async fn hint(args: ChallengeCodeArgs) -> Result<()> {
    let arena = load_client().await?;
    let data = arena.view_hint(&args.code).await?;
    print_json(&data)
}

async fn load_client() -> Result<ArenaClient> {
    let config = load_arena_config(sentinel_state_dir()).await?;
    ArenaClient::new(config)
}
