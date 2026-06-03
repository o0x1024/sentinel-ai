use crate::args::{DbAction, DbCommand};
use crate::output::print_json;
use crate::state::{read_db_config, save_sqlite_config};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct DbInitResult {
    config_path: String,
}

pub fn run(command: DbCommand) -> Result<()> {
    match command.action {
        DbAction::Init(args) => {
            let config_path = save_sqlite_config(args.sqlite_path)?;
            print_json(&DbInitResult {
                config_path: config_path.display().to_string(),
            })
        }
        DbAction::ShowConfig => print_json(&read_db_config()?),
    }
}
