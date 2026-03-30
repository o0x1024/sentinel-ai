use std::sync::Arc;

use anyhow::Result;
use sentinel_db::Database;

use crate::services::{DatabaseService, DictionaryService};

pub async fn initialize_builtin_bounty_resources(db_service: &Arc<DatabaseService>) -> Result<()> {
    let pool = db_service.get_runtime_pool()?;
    let dictionary_service = DictionaryService::new(pool);
    dictionary_service.initialize_builtin_dictionaries().await?;

    ensure_default_dictionary(db_service, "subdomain", "builtin_subdomain_common").await?;
    ensure_default_dictionary(db_service, "sensitive_file", "builtin_sensitive_files_web").await?;
    ensure_default_dictionary(
        db_service,
        "fingerprint_rule",
        "builtin_web_fingerprint_rules",
    )
    .await?;
    ensure_default_dictionary(
        db_service,
        "service_probe_rule",
        "builtin_service_fingerprint_rules",
    )
    .await?;
    ensure_default_dictionary(db_service, "poc_rule", "builtin_safe_poc_rules").await?;

    tracing::info!(
        "Builtin plugin seeding is disabled; plugins must be added manually or via the plugin store"
    );

    Ok(())
}

async fn ensure_default_dictionary(
    db_service: &Arc<DatabaseService>,
    dict_type: &str,
    dictionary_id: &str,
) -> Result<()> {
    let current = db_service
        .get_config("dictionary_default", dict_type)
        .await?;
    if current.as_deref().unwrap_or_default().trim().is_empty() {
        db_service
            .set_config(
                "dictionary_default",
                dict_type,
                dictionary_id,
                Some("Builtin default dictionary"),
            )
            .await?;
    }
    Ok(())
}
