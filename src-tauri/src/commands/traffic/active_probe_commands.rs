use sentinel_plugins::ActiveProbeQueueSnapshot;

use crate::commands::command_response_support::CommandResponse;

#[tauri::command]
pub async fn get_active_probe_queue_snapshot(
) -> Result<CommandResponse<ActiveProbeQueueSnapshot>, String> {
    Ok(CommandResponse::ok(
        sentinel_plugins::get_active_probe_queue_snapshot(),
    ))
}
