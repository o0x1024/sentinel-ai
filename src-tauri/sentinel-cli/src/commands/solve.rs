use crate::arena::{load_arena_config, ArenaClient};
use crate::args::{SolveAction, SolveAllArgs, SolveCommand, SolveOneArgs};
use crate::output::print_json;
use crate::solver::{solve_all, solve_one, SolveFilters, SolveOptions};
use crate::state::sentinel_state_dir;
use anyhow::Result;

pub async fn run(command: SolveCommand) -> Result<()> {
    match command.action {
        SolveAction::One(args) => solve_one_command(args).await,
        SolveAction::All(args) => solve_all_command(args).await,
    }
}

async fn solve_one_command(args: SolveOneArgs) -> Result<()> {
    let arena = load_client().await?;
    let result = solve_one(
        &arena,
        &args.code,
        SolveOptions {
            allow_hint: args.allow_hint,
            stop_when_done: args.stop_when_done,
            max_requests_per_target: args.max_requests_per_target,
        },
    )
    .await?;
    print_json(&result)
}

async fn solve_all_command(args: SolveAllArgs) -> Result<()> {
    let arena = load_client().await?;
    let result = solve_all(
        &arena,
        SolveOptions {
            allow_hint: args.allow_hint,
            stop_when_done: args.stop_when_done,
            max_requests_per_target: args.max_requests_per_target,
        },
        SolveFilters {
            max_level: args.max_level,
            difficulties: args.difficulties,
            include_solved: args.include_solved,
            limit: args.limit,
        },
    )
    .await?;
    print_json(&result)
}

async fn load_client() -> Result<ArenaClient> {
    let config = load_arena_config(sentinel_state_dir()).await?;
    ArenaClient::new(config)
}
