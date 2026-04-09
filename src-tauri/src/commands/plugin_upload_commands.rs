use sentinel_db::{Database, TrafficPluginMetadata};
use sentinel_plugins::{PluginMetadata, Severity};
use sentinel_traffic::ScanTask;
use tauri::{AppHandle, State};

use crate::commands::command_response_support::CommandResponse;
use crate::commands::traffic::{
    refresh_active_agent_plugin_tools, resolved_store_plugin_monitor_type, TrafficAnalysisState,
};
use crate::events::{emit_plugin_changed, PluginChangedEvent};

#[tauri::command]
pub async fn upload_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    filename: String,
    content: String,
) -> Result<CommandResponse<String>, String> {
    let parsed = parse_uploaded_plugin(&filename, &content)?;
    let db = state.get_db_service();
    let existing_plugin = db
        .get_plugin_from_registry(&parsed.id)
        .await
        .map_err(|e| format!("Failed to load existing plugin metadata: {}", e))?;

    let metadata = PluginMetadata {
        id: parsed.id.clone(),
        name: parsed.name.clone(),
        version: parsed.version,
        author: parsed.author,
        main_category: parsed.main_category.clone(),
        category: parsed.category.clone(),
        default_severity: parsed.default_severity,
        tags: parsed.tags,
        description: parsed.description,
        monitor_type: parsed.monitor_type.or_else(|| {
            resolved_store_plugin_monitor_type(
                existing_plugin.as_ref(),
                &parsed.id,
                &parsed.main_category,
                &parsed.category,
            )
        }),
        target_asset_types: if parsed.target_asset_types.is_empty() {
            existing_plugin
                .as_ref()
                .map(|plugin| plugin.metadata.target_asset_types.clone())
                .unwrap_or_default()
        } else {
            parsed.target_asset_types
        },
    };

    let traffic_metadata = TrafficPluginMetadata {
        id: metadata.id.clone(),
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        author: metadata.author.clone(),
        main_category: metadata.main_category.clone(),
        category: metadata.category.clone(),
        description: metadata.description.clone(),
        default_severity: metadata.default_severity.to_string(),
        tags: metadata.tags.clone(),
    };

    db.register_traffic_plugin_with_code(&traffic_metadata, &content)
        .await
        .map_err(|e| format!("Failed to save uploaded plugin: {}", e))?;

    db.update_plugin_enabled(&metadata.id, true)
        .await
        .map_err(|e| format!("Failed to enable uploaded plugin: {}", e))?;

    let metadata_json = serde_json::to_value(&metadata)
        .map_err(|e| format!("Failed to serialize uploaded plugin metadata: {}", e))?;
    db.update_plugin(&metadata_json, &content)
        .await
        .map_err(|e| format!("Failed to persist uploaded plugin metadata: {}", e))?;

    let plugin_manager = state.get_plugin_manager();
    if let Err(error) = plugin_manager
        .set_plugin_code(metadata.id.clone(), content.clone())
        .await
    {
        tracing::warn!(
            "Failed to update plugin code cache after upload for {}: {}",
            metadata.id,
            error
        );
    }

    if metadata.main_category == "agent" {
        let refreshed = refresh_active_agent_plugin_tools(db.as_ref()).await?;
        tracing::info!(
            "Refreshed {} active agent plugin tools after uploading {}",
            refreshed,
            metadata.id
        );
    }

    if metadata.main_category == "traffic" {
        let is_running = *state.get_is_running().read().await;
        if is_running {
            let scan_tx = state.get_scan_tx();
            let scan_tx_guard = scan_tx.read().await;
            if let Some(tx) = scan_tx_guard.as_ref() {
                if let Err(error) = tx.send(ScanTask::ReloadPlugin(metadata.id.clone())) {
                    tracing::warn!(
                        "Failed to hot-reload uploaded traffic plugin {}: {}",
                        metadata.id,
                        error
                    );
                }
            }
        }
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: metadata.id.clone(),
            enabled: true,
            name: metadata.name.clone(),
        },
    );

    Ok(CommandResponse::ok(metadata.id))
}

#[derive(Debug)]
struct ParsedUploadedPlugin {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    main_category: String,
    category: String,
    default_severity: Severity,
    tags: Vec<String>,
    description: Option<String>,
    monitor_type: Option<String>,
    target_asset_types: Vec<String>,
}

fn parse_uploaded_plugin(filename: &str, content: &str) -> Result<ParsedUploadedPlugin, String> {
    let tags = extract_doc_tags(content);
    let inferred_main_category = tags
        .get("main_category")
        .cloned()
        .or_else(|| infer_main_category(content).map(str::to_string))
        .ok_or_else(|| {
            "Unable to infer plugin type. Add `@main_category agent|traffic` or export `analyze` / `scan_transaction`.".to_string()
        })?;

    let plugin_id = tags
        .get("plugin")
        .map(String::as_str)
        .unwrap_or(filename)
        .to_string();
    let plugin_id = sanitize_plugin_id(&strip_extension(&plugin_id));
    if plugin_id.is_empty() {
        return Err("Plugin id is empty. Add `@plugin your_plugin_id` in the header.".to_string());
    }

    let category = tags
        .get("category")
        .cloned()
        .unwrap_or_else(|| default_category_for(&inferred_main_category).to_string());

    Ok(ParsedUploadedPlugin {
        id: plugin_id.clone(),
        name: tags
            .get("name")
            .cloned()
            .unwrap_or_else(|| humanize_plugin_name(&plugin_id)),
        version: tags
            .get("version")
            .cloned()
            .unwrap_or_else(|| "1.0.0".to_string()),
        author: tags
            .get("author")
            .cloned()
            .filter(|value| !value.is_empty()),
        main_category: inferred_main_category,
        category,
        default_severity: parse_severity(tags.get("default_severity").map(String::as_str)),
        tags: split_csv(tags.get("tags")),
        description: tags
            .get("description")
            .cloned()
            .filter(|value| !value.is_empty()),
        monitor_type: tags
            .get("monitor_type")
            .cloned()
            .filter(|value| !value.is_empty()),
        target_asset_types: split_csv(tags.get("target_asset_types")),
    })
}

fn extract_doc_tags(content: &str) -> std::collections::HashMap<String, String> {
    let mut tags = std::collections::HashMap::new();

    for line in content.lines().take(80) {
        let trimmed = line.trim().trim_start_matches('*').trim();
        let Some(rest) = trimmed.strip_prefix('@') else {
            continue;
        };

        let mut parts = rest.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }

        tags.insert(key.to_string(), value.to_string());
    }

    tags
}

fn infer_main_category(content: &str) -> Option<&'static str> {
    if content.contains("scan_transaction")
        || content.contains("globalThis.scan_transaction")
        || content.contains("export async function scan_transaction")
        || content.contains("export function scan_transaction")
    {
        return Some("traffic");
    }

    if content.contains("analyze(")
        || content.contains("globalThis.analyze")
        || content.contains("pluginGlobals.analyze")
        || content.contains("export async function analyze")
        || content.contains("export function analyze")
    {
        return Some("agent");
    }

    None
}

fn parse_severity(value: Option<&str>) -> Severity {
    match value
        .unwrap_or("medium")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "low" => Severity::Low,
        "info" => Severity::Info,
        _ => Severity::Medium,
    }
}

fn split_csv(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn strip_extension(value: &str) -> String {
    value
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| value.to_string())
}

fn sanitize_plugin_id(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if (ch == '_' || ch == '-' || ch.is_ascii_whitespace()) && !previous_was_separator {
            result.push('_');
            previous_was_separator = true;
        }
    }

    result.trim_matches('_').to_string()
}

fn humanize_plugin_name(plugin_id: &str) -> String {
    plugin_id
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(capitalize_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut result = String::new();
            result.push(first.to_ascii_uppercase());
            result.push_str(chars.as_str());
            result
        }
        None => String::new(),
    }
}

fn default_category_for(main_category: &str) -> &'static str {
    match main_category {
        "agent" => "custom",
        _ => "custom",
    }
}
