use crate::models::database::Configuration;
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalProxyConfig {
    pub enabled: bool,
    pub scheme: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub no_proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    pub id: String,
    pub category: String,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_encrypted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveConfigRequest {
    pub category: String,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_encrypted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetConfigRequest {
    pub category: String,
    pub key: Option<String>,
}

// 保存配置
#[tauri::command]
pub async fn save_config(
    request: SaveConfigRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.set_config(
        &request.category,
        &request.key,
        &request.value,
        request.description.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

// 设置全局代理（并保存到DB）
#[tauri::command]
pub async fn set_global_proxy_config(
    cfg: GlobalProxyConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let json = serde_json::to_string(&cfg).map_err(|e| e.to_string())?;
    db.set_config("network", "global_proxy", &json, Some("Global HTTP proxy settings")).await.map_err(|e| e.to_string())?;
    // 生效到运行时
    let to_runtime = crate::ai_adapter::http::ProxyConfig {
        enabled: cfg.enabled,
        scheme: cfg.scheme,
        host: cfg.host,
        port: cfg.port,
        username: cfg.username,
        password: cfg.password,
        no_proxy: cfg.no_proxy,
    };
    crate::ai_adapter::http::set_global_proxy(Some(to_runtime));
    Ok(())
}

// 读取全局代理
#[tauri::command]
pub async fn get_global_proxy_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<GlobalProxyConfig, String> {
    if let Ok(Some(json)) = db.get_config("network", "global_proxy").await {
        if let Ok(cfg) = serde_json::from_str::<GlobalProxyConfig>(&json) {
            return Ok(cfg);
        }
    }
    Ok(GlobalProxyConfig { enabled: false, scheme: None, host: None, port: None, username: None, password: None, no_proxy: None })
}

// 获取配置
#[tauri::command]
pub async fn get_config(
    request: GetConfigRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ConfigItem>, String> {
    if let Some(key) = request.key {
        // 获取单个配置
        let value = db
            .get_config(&request.category, &key)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(value) = value {
            Ok(vec![ConfigItem {
                id: format!("cfg_{}_{}", request.category, key),
                category: request.category,
                key,
                value,
                description: None,
                is_encrypted: false,
            }])
        } else {
            Ok(vec![])
        }
    } else {
        // 获取分类下的所有配置
        let configs = db
            .get_configs_by_category(&request.category)
            .await
            .map_err(|e| e.to_string())?;

        let config_items: Vec<ConfigItem> = configs
            .into_iter()
            .map(|config| ConfigItem {
                id: config.id,
                category: config.category,
                key: config.key,
                value: config.value.unwrap_or_default(),
                description: config.description,
                is_encrypted: config.is_encrypted,
            })
            .collect();

        Ok(config_items)
    }
}

// 删除配置
#[tauri::command]
pub async fn delete_config(
    category: String,
    key: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 由于DatabaseService没有delete_config方法，我们需要使用execute_query
    let query = "DELETE FROM configurations WHERE category = ? AND key = ?";

    // 使用execute_query执行删除操作
    // 注意：execute_query返回结果，但我们只需要知道操作是否成功
    db.execute_query(&format!(
        "DELETE FROM configurations WHERE category = '{}' AND key = '{}'",
        category, key
    ))
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

// 获取所有配置分类
#[tauri::command]
pub async fn get_config_categories(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let results = db
        .execute_query("SELECT DISTINCT category FROM configurations ORDER BY category")
        .await
        .map_err(|e| e.to_string())?;

    let categories: Vec<String> = results
        .into_iter()
        .filter_map(|row| {
            if let serde_json::Value::Object(obj) = row {
                obj.get("category")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(categories)
}

// 批量保存配置
#[tauri::command]
pub async fn save_config_batch(
    configs: Vec<SaveConfigRequest>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    for config in configs {
        db.set_config(
            &config.category,
            &config.key,
            &config.value,
            config.description.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_config(
    category: String,
    key: String,
    value: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.set_config(&category, &key, &value, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_theme(db: State<'_, Arc<DatabaseService>>) -> Result<String, String> {
    let theme = db
        .get_config("ui", "theme")
        .await
        .map_err(|e| e.to_string())?;
    Ok(theme.unwrap_or_else(|| "light".to_string()))
}

#[tauri::command]
pub async fn set_theme(theme: String, db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    db.set_config("ui", "theme", &theme, Some("UI theme"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_language(db: State<'_, Arc<DatabaseService>>) -> Result<String, String> {
    let lang = db
        .get_config("ui", "language")
        .await
        .map_err(|e| e.to_string())?;
    Ok(lang.unwrap_or_else(|| "en".to_string()))
}

#[tauri::command]
pub async fn set_language(lang: String, db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    db.set_config("ui", "language", &lang, Some("UI language"))
        .await
        .map_err(|e| e.to_string())
}
