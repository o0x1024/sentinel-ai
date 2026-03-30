use sentinel_db::Database;

use crate::services::DatabaseService;

const REMOVED_AGENT_PLUGIN_IDS: &[&str] = &["service_fingerprinter"];

pub fn is_removed_agent_plugin(plugin_id: &str) -> bool {
    REMOVED_AGENT_PLUGIN_IDS.contains(&plugin_id)
}

pub async fn cleanup_removed_agent_plugins(db: &DatabaseService) -> Result<Vec<String>, String> {
    let mut removed = Vec::new();

    for plugin_id in REMOVED_AGENT_PLUGIN_IDS {
        let exists = db
            .get_plugin_from_registry(plugin_id)
            .await
            .map_err(|e| format!("Failed to query legacy plugin {}: {}", plugin_id, e))?;

        if exists.is_none() {
            continue;
        }

        db.delete_plugin_from_registry(plugin_id)
            .await
            .map_err(|e| format!("Failed to delete legacy plugin {}: {}", plugin_id, e))?;

        tracing::info!("Removed legacy plugin '{}' from plugin registry", plugin_id);
        removed.push((*plugin_id).to_string());
    }

    Ok(removed)
}
