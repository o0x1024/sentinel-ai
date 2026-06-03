use crate::output::print_json;
use crate::state::resolve_paths;

pub fn run() -> anyhow::Result<()> {
    print_json(&resolve_paths())
}
