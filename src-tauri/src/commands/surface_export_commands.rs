use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{
    DatabaseService, SurfaceAssetFilter, SurfaceInventoryCursor, SurfaceInventoryItem,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

const SURFACE_EXPORT_BATCH_SIZE: i64 = 1_000;

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceInventoryExportRequest {
    pub filter: SurfaceAssetFilter,
    pub export_type: String,
    pub format: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceInventoryExportResponse {
    pub exported: usize,
    pub output_path: String,
}

fn normalize_export_type(value: &str) -> &str {
    match value {
        "all" | "current" | "api" | "org" | "domain" | "ip" | "host" | "port" | "service"
        | "web" | "certificate" => value,
        _ => "all",
    }
}

fn normalize_export_format(value: &str) -> &str {
    match value {
        "csv" | "json" => value,
        _ => "csv",
    }
}

fn text_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.trim().to_string(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn item_detail_value(item: &SurfaceInventoryItem, key: &str) -> String {
    item.typed_details
        .as_ref()
        .and_then(|details| details.get(key))
        .map(|value| text_value(Some(value)))
        .unwrap_or_default()
}

fn is_api_like_surface_asset(item: &SurfaceInventoryItem) -> bool {
    if item
        .typed_details
        .as_ref()
        .and_then(|details| details.get("api_flag"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return true;
    }

    if !item_detail_value(item, "openapi_url").is_empty() {
        return true;
    }

    let joined = [
        item_detail_value(item, "canonical_url"),
        item_detail_value(item, "site_title"),
        item_detail_value(item, "content_summary"),
        item_detail_value(item, "application_service_name"),
        item_detail_value(item, "protocol_name"),
        item_detail_value(item, "product_name"),
        item.asset.asset_name.clone(),
        item.asset.display_name.clone().unwrap_or_default(),
    ]
    .join(" ")
    .to_lowercase();

    joined.contains("/api/")
        || joined.contains("/api")
        || joined.contains("api.")
        || joined.contains("api-")
        || joined.contains("graphql")
        || joined.contains("openapi")
        || joined.contains("swagger")
}

fn should_export_item(item: &SurfaceInventoryItem, export_type: &str) -> bool {
    match export_type {
        "all" | "current" => true,
        "api" => is_api_like_surface_asset(item),
        asset_type => item.asset.asset_type == asset_type,
    }
}

fn primary_value(item: &SurfaceInventoryItem) -> String {
    let typed = item.typed_details.as_ref();
    match item.asset.asset_type.as_str() {
        "web" => text_value(typed.and_then(|details| details.get("canonical_url"))),
        "service" => {
            let service = text_value(typed.and_then(|details| {
                details
                    .get("application_service_name")
                    .or_else(|| details.get("protocol_name"))
            }));
            if !service.is_empty() {
                return service;
            }
            item.asset.asset_name.clone()
        }
        "domain" => text_value(typed.and_then(|details| details.get("fqdn"))),
        "ip" => text_value(typed.and_then(|details| details.get("ip_address"))),
        "host" => text_value(
            typed.and_then(|details| details.get("fqdn").or_else(|| details.get("hostname"))),
        ),
        "certificate" => text_value(typed.and_then(|details| details.get("sha256"))),
        _ => item.asset.asset_name.clone(),
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', " "))
}

fn write_csv_header(writer: &mut BufWriter<File>) -> Result<(), String> {
    let headers = [
        "id",
        "program_id",
        "asset_type",
        "asset_name",
        "display_name",
        "primary_value",
        "status",
        "internet_exposure",
        "criticality",
        "risk_level",
        "source",
        "owner",
        "maintainer",
        "last_seen_at",
        "created_at",
        "updated_at",
        "typed_details_json",
    ];
    writeln!(writer, "{}", headers.join(",")).map_err(|e| e.to_string())
}

fn write_csv_item(writer: &mut BufWriter<File>, item: &SurfaceInventoryItem) -> Result<(), String> {
    let typed_details_json = item
        .typed_details
        .as_ref()
        .map(Value::to_string)
        .unwrap_or_default();
    let values = vec![
        item.asset.id.clone(),
        item.asset.program_id.clone(),
        item.asset.asset_type.clone(),
        item.asset.asset_name.clone(),
        item.asset.display_name.clone().unwrap_or_default(),
        primary_value(item),
        item.asset.status.clone(),
        item.asset.internet_exposure.clone().unwrap_or_default(),
        item.asset.criticality.clone().unwrap_or_default(),
        item.asset.risk_level.clone().unwrap_or_default(),
        item.asset.source.clone().unwrap_or_default(),
        item.asset.owner.clone().unwrap_or_default(),
        item.asset.maintainer.clone().unwrap_or_default(),
        item.asset.last_seen_at.clone(),
        item.asset.created_at.clone(),
        item.asset.updated_at.clone(),
        typed_details_json,
    ];
    let line = values
        .iter()
        .map(|value| csv_escape(value.as_str()))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(writer, "{line}").map_err(|e| e.to_string())
}

fn write_json_prefix(
    writer: &mut BufWriter<File>,
    request: &SurfaceInventoryExportRequest,
) -> Result<(), String> {
    let exported_at = Utc::now().to_rfc3339();
    write!(
        writer,
        "{{\"exported_at\":{},\"export_type\":{},\"filter\":{},\"items\":[",
        serde_json::to_string(&exported_at).map_err(|e| e.to_string())?,
        serde_json::to_string(&request.export_type).map_err(|e| e.to_string())?,
        serde_json::to_string(&request.filter).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_export_inventory(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceInventoryExportRequest,
) -> Result<SurfaceInventoryExportResponse, String> {
    let export_type = normalize_export_type(request.export_type.as_str()).to_string();
    let export_format = normalize_export_format(request.format.as_str()).to_string();
    let mut filter = request.filter.clone();
    filter.limit = Some(SURFACE_EXPORT_BATCH_SIZE);
    filter.offset = None;

    let file = File::create(&request.output_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    if export_format == "csv" {
        write_csv_header(&mut writer)?;
    } else {
        write_json_prefix(&mut writer, &request)?;
    }

    let mut exported = 0usize;
    let mut cursor: Option<SurfaceInventoryCursor> = None;
    let mut first_json_item = true;

    loop {
        let response = db_service
            .list_surface_inventory(&filter, cursor.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        for item in response.items {
            if !should_export_item(&item, &export_type) {
                continue;
            }
            if export_format == "csv" {
                write_csv_item(&mut writer, &item)?;
            } else {
                if !first_json_item {
                    write!(writer, ",").map_err(|e| e.to_string())?;
                }
                first_json_item = false;
                serde_json::to_writer(&mut writer, &item).map_err(|e| e.to_string())?;
            }
            exported += 1;
        }

        if !response.has_next {
            break;
        }
        cursor = response.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    if export_format == "json" {
        write!(writer, "],\"total\":{exported}}}").map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())?;

    Ok(SurfaceInventoryExportResponse {
        exported,
        output_path: request.output_path,
    })
}
