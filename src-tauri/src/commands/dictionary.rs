use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

use crate::services::{AiServiceManager, DatabaseService, DictionaryService};
use sentinel_core::models::dictionary::{
    Dictionary, DictionaryExport, DictionaryFilter, DictionaryImportOptions, DictionarySet,
    DictionaryStats, DictionaryType, DictionaryWord, DictionaryWordInput, ServiceType,
};
use sentinel_db::Database;
use sentinel_services::nmap_service_probes::parse_nmap_service_probes;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct DictionaryPageResponse {
    pub items: Vec<Dictionary>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct GenerateDictionaryEntriesRequest {
    pub dictionary_name: String,
    pub dict_type: String,
    pub service_type: Option<String>,
    pub subtype: Option<String>,
    pub prompt: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateDictionaryEntriesResponse {
    pub entries: Vec<DictionaryWordInput>,
}

const DICTIONARY_AI_GENERATION_SYSTEM_PROMPT: &str = r#"You generate Sentinel dictionary entries.
Return only strict JSON, no markdown, no comments, no prose.
The JSON shape must be:
{
  "entries": [
    {
      "word": "stable_unique_identifier_or_dictionary_value",
      "weight": 1.0,
      "category": "short_category_or_null",
      "metadata": {}
    }
  ]
}

Rules:
- Generate exactly the requested number of entries unless the request is impossible.
- Do not include duplicates, placeholders, explanations, or unsafe exploit payloads.
- For normal dictionaries, keep metadata as an empty object.
- For sensitive_file rules, metadata should include path, severity, tags, enabled, safe_mode when useful.
- For fingerprint_rule and service_probe_rule, metadata should include service, product, vendor, protocol, probeName, ports, sslPorts, confidence, operator, matchers, enabled when useful.
- matchers must be an array of objects with part, type, optional key, and value.
- For poc_rule, metadata should describe safe verification: severity, safe_mode, requestMethod, requestPath, timeoutMs, matchers, enabled.
- severity must be one of critical, high, medium, low, info.
- enabled must be boolean when present.
"#;

/// 使用 AI 生成字典词条或结构化规则
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_dictionary_entries(
    ai_manager: State<'_, Arc<AiServiceManager>>,
    request: GenerateDictionaryEntriesRequest,
) -> Result<GenerateDictionaryEntriesResponse, String> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err("Generation prompt is required".to_string());
    }

    let count = request.count.clamp(1, 100);
    let user_input = serde_json::to_string_pretty(&serde_json::json!({
        "dictionaryName": request.dictionary_name,
        "dictionaryType": request.dict_type,
        "serviceType": request.service_type,
        "subtype": request.subtype,
        "requestedCount": count,
        "userRequirement": prompt,
    }))
    .map_err(|error| error.to_string())?;

    let llm_config = ai_manager
        .resolve_generation_llm_config(None, None)
        .await
        .map_err(|error| error.to_string())?;
    let client = sentinel_llm::LlmClient::new(llm_config);
    let raw = client
        .completion(Some(DICTIONARY_AI_GENERATION_SYSTEM_PROMPT), &user_input)
        .await
        .map_err(|error| error.to_string())?;

    let entries = parse_generated_dictionary_entries(&raw)?;
    if entries.is_empty() {
        return Err("AI returned no dictionary entries".to_string());
    }

    Ok(GenerateDictionaryEntriesResponse { entries })
}

fn parse_generated_dictionary_entries(raw: &str) -> Result<Vec<DictionaryWordInput>, String> {
    let json_text = strip_json_code_fence(raw.trim());
    let value = serde_json::from_str::<Value>(json_text)
        .or_else(|_| {
            let start = json_text.find('{').ok_or_else(|| {
                serde_json::Error::io(std::io::Error::other("missing JSON object"))
            })?;
            let end = json_text.rfind('}').ok_or_else(|| {
                serde_json::Error::io(std::io::Error::other("missing JSON object"))
            })?;
            serde_json::from_str::<Value>(&json_text[start..=end])
        })
        .map_err(|error| format!("Failed to parse AI JSON output: {error}"))?;

    let entries_value = value
        .get("entries")
        .ok_or_else(|| "AI JSON output must contain an entries array".to_string())?;
    let entries = serde_json::from_value::<Vec<DictionaryWordInput>>(entries_value.clone())
        .map_err(|error| format!("Failed to parse generated entries: {error}"))?;

    entries
        .into_iter()
        .map(|entry| {
            let word = entry.word.trim().to_string();
            if word.is_empty() {
                return Err("AI returned an entry with an empty word".to_string());
            }
            Ok(DictionaryWordInput {
                word,
                weight: entry.weight.or(Some(1.0)),
                category: entry
                    .category
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                metadata: entry
                    .metadata
                    .or_else(|| Some(Value::Object(Default::default()))),
            })
        })
        .collect()
}

fn strip_json_code_fence(value: &str) -> &str {
    let trimmed = value.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }

    let Some(first_line_end) = trimmed.find('\n') else {
        return trimmed;
    };
    let body = &trimmed[first_line_end + 1..];
    let Some(last_fence_start) = body.rfind("```") else {
        return trimmed;
    };
    body[..last_fence_start].trim()
}

/// 获取字典列表
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dictionaries(
    db_service: State<'_, Arc<DatabaseService>>,
    dict_type: Option<String>,
    service_type: Option<String>,
    category: Option<String>,
    is_builtin: Option<bool>,
    is_active: Option<bool>,
    search_term: Option<String>,
) -> Result<Vec<Dictionary>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let filter = if dict_type.is_some()
        || service_type.is_some()
        || category.is_some()
        || is_builtin.is_some()
        || is_active.is_some()
        || search_term.is_some()
    {
        Some(DictionaryFilter {
            dict_type: dict_type.map(DictionaryType::from),
            service_type: service_type.map(ServiceType::from),
            category,
            is_builtin,
            is_active,
            tags: None,
            search_term,
        })
    } else {
        None
    };

    dictionary_service
        .list_dictionaries(filter)
        .await
        .map_err(|e| e.to_string())
}

/// 分页获取字典列表
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dictionaries_paged(
    db_service: State<'_, Arc<DatabaseService>>,
    dict_type: Option<String>,
    dict_types: Option<Vec<String>>,
    service_type: Option<String>,
    category: Option<String>,
    is_builtin: Option<bool>,
    is_active: Option<bool>,
    search_term: Option<String>,
    subtype: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<DictionaryPageResponse, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let filter = if dict_type.is_some()
        || service_type.is_some()
        || category.is_some()
        || is_builtin.is_some()
        || is_active.is_some()
        || search_term.is_some()
    {
        Some(DictionaryFilter {
            dict_type: dict_type.map(DictionaryType::from),
            service_type: service_type.map(ServiceType::from),
            category,
            is_builtin,
            is_active,
            tags: None,
            search_term,
        })
    } else {
        None
    };

    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let items = dictionary_service
        .list_dictionaries_paged(
            filter.clone(),
            dict_types.clone(),
            subtype.clone(),
            offset,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
    let total = dictionary_service
        .count_dictionaries(filter, dict_types, subtype)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DictionaryPageResponse { items, total })
}

/// 获取单个字典
#[tauri::command]
pub async fn get_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<Dictionary>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .get_dictionary(&id)
        .await
        .map_err(|e| e.to_string())
}

/// 创建字典
#[tauri::command(rename_all = "snake_case")]
pub async fn create_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    name: String,
    dict_type: String,
    service_type: Option<String>,
    description: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    metadata: Option<String>,
) -> Result<Dictionary, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let mut dictionary = Dictionary::new(
        name,
        DictionaryType::from(dict_type),
        service_type.map(ServiceType::from),
        description,
    );

    dictionary.category = category;
    dictionary.metadata = metadata;
    if let Some(tags) = tags {
        dictionary.set_tags(tags);
    }

    dictionary_service
        .create_dictionary(dictionary)
        .await
        .map_err(|e| e.to_string())
}

/// 更新字典
#[tauri::command]
pub async fn update_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary: Dictionary,
) -> Result<Dictionary, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .update_dictionary(dictionary)
        .await
        .map_err(|e| e.to_string())
}

/// 删除字典
#[tauri::command]
pub async fn delete_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .delete_dictionary(&id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取字典词条
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dictionary_words(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .get_dictionary_words(&dictionary_id)
        .await
        .map_err(|e| e.to_string())
}

/// 分页获取字典词条（可选搜索）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_dictionary_words_paged(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    offset: Option<u32>,
    limit: Option<u32>,
    pattern: Option<String>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let off = offset.unwrap_or(0);
    let lim = limit.unwrap_or(500);

    // 如果带 pattern，则使用带 OFFSET 的搜索分页
    if let Some(p) = pattern {
        return dictionary_service
            .search_words_paged(&dictionary_id, &p, off, lim)
            .await
            .map_err(|e| e.to_string());
    }
    dictionary_service
        .get_dictionary_words_paged(&dictionary_id, off, lim)
        .await
        .map_err(|e| e.to_string())
}

/// 添加词条到字典
#[tauri::command(rename_all = "snake_case")]
pub async fn add_dictionary_words(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    words: Vec<String>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .add_words(&dictionary_id, words)
        .await
        .map_err(|e| e.to_string())
}

/// 添加结构化词条到字典
#[tauri::command(rename_all = "snake_case")]
pub async fn add_dictionary_entries(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    entries: Vec<DictionaryWordInput>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .add_word_entries(&dictionary_id, entries)
        .await
        .map_err(|e| e.to_string())
}

/// 更新单个字典词条
#[tauri::command]
pub async fn update_dictionary_word(
    db_service: State<'_, Arc<DatabaseService>>,
    word: DictionaryWord,
) -> Result<DictionaryWord, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .update_word(word)
        .await
        .map_err(|e| e.to_string())
}

/// 批量更新字典词条
#[tauri::command]
pub async fn update_dictionary_words_batch(
    db_service: State<'_, Arc<DatabaseService>>,
    words: Vec<DictionaryWord>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .update_words_batch(words)
        .await
        .map_err(|e| e.to_string())
}

/// 从字典中移除词条
#[tauri::command(rename_all = "snake_case")]
pub async fn remove_dictionary_words(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    words: Vec<String>,
) -> Result<u64, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .remove_words(&dictionary_id, words)
        .await
        .map_err(|e| e.to_string())
}

/// 搜索字典词条
#[tauri::command(rename_all = "snake_case")]
pub async fn search_dictionary_words(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    pattern: String,
    limit: Option<u32>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .search_words(&dictionary_id, &pattern, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 清空字典
#[tauri::command(rename_all = "snake_case")]
pub async fn clear_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
) -> Result<u64, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .clear_dictionary(&dictionary_id)
        .await
        .map_err(|e| e.to_string())
}

/// 导出字典
#[tauri::command(rename_all = "snake_case")]
pub async fn export_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
) -> Result<DictionaryExport, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .export_dictionary(&dictionary_id)
        .await
        .map_err(|e| e.to_string())
}

/// 导入字典
#[tauri::command(rename_all = "snake_case")]
pub async fn import_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    export_data: DictionaryExport,
    options: DictionaryImportOptions,
) -> Result<Dictionary, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .import_dictionary(export_data, options)
        .await
        .map_err(|e| e.to_string())
}

/// 从文件导入字典
#[tauri::command(rename_all = "snake_case")]
pub async fn import_dictionary_from_file(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    file_content: String,
    separator: Option<String>,
) -> Result<Vec<DictionaryWord>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let sep = separator.unwrap_or_else(|| "\n".to_string());
    let words: Vec<String> = file_content
        .split(&sep)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    dictionary_service
        .add_words(&dictionary_id, words)
        .await
        .map_err(|e| e.to_string())
}

/// 从 nmap-service-probes 导入服务识别规则
#[tauri::command(rename_all = "snake_case")]
pub async fn import_nmap_service_probes(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    file_content: String,
    replace_existing: Option<bool>,
) -> Result<usize, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());
    let entries = parse_nmap_service_probes(&file_content).map_err(|e| e.to_string())?;
    let affected_words = dictionary_service
        .sync_word_entries(&dictionary_id, entries, replace_existing.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())?;

    if let Some(mut dictionary) = dictionary_service
        .get_dictionary(&dictionary_id)
        .await
        .map_err(|e| e.to_string())?
    {
        let mut metadata = dictionary
            .metadata
            .as_deref()
            .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        metadata.insert(
            "subtype".to_string(),
            serde_json::Value::String("service_identification".to_string()),
        );
        dictionary.metadata = Some(serde_json::Value::Object(metadata).to_string());
        dictionary_service
            .update_dictionary(dictionary)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(affected_words.len())
}

/// 导出字典到文件格式
#[tauri::command(rename_all = "snake_case")]
pub async fn export_dictionary_to_file(
    db_service: State<'_, Arc<DatabaseService>>,
    dictionary_id: String,
    format: String, // "txt", "json", "csv"
    separator: Option<String>,
) -> Result<String, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let words = dictionary_service
        .get_dictionary_words(&dictionary_id)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    match format.as_str() {
        "txt" => {
            let sep = separator.unwrap_or_else(|| "\n".to_string());
            Ok(words
                .into_iter()
                .map(|w| w.word)
                .collect::<Vec<_>>()
                .join(&sep))
        }
        "json" => serde_json::to_string_pretty(&words).map_err(|e| e.to_string()),
        "csv" => {
            let mut result = "word,weight,category\n".to_string();
            for word in words {
                result.push_str(&format!(
                    "{},{},{}\n",
                    word.word,
                    word.weight,
                    word.category.unwrap_or_default()
                ));
            }
            Ok(result)
        }
        _ => Err("Unsupported format".to_string()),
    }
}

/// 获取字典统计信息
#[tauri::command]
pub async fn get_dictionary_stats(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<DictionaryStats, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .get_stats()
        .await
        .map_err(|e| e.to_string())
}

/// 创建字典集合
#[tauri::command(rename_all = "snake_case")]
pub async fn create_dictionary_set(
    db_service: State<'_, Arc<DatabaseService>>,
    name: String,
    service_type: Option<String>,
    description: Option<String>,
    scenario: Option<String>,
) -> Result<DictionarySet, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let mut set = DictionarySet::new(name, service_type.map(ServiceType::from));
    set.description = description;
    set.scenario = scenario;

    dictionary_service
        .create_dictionary_set(set)
        .await
        .map_err(|e| e.to_string())
}

/// 向字典集合添加字典
#[tauri::command(rename_all = "snake_case")]
pub async fn add_dictionary_to_set(
    db_service: State<'_, Arc<DatabaseService>>,
    set_id: String,
    dictionary_id: String,
    priority: Option<i32>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .add_dictionary_to_set(&set_id, &dictionary_id, priority)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(())
}

/// 获取字典集合中的字典
#[tauri::command(rename_all = "snake_case")]
pub async fn get_set_dictionaries(
    db_service: State<'_, Arc<DatabaseService>>,
    set_id: String,
) -> Result<Vec<Dictionary>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .get_set_dictionaries(&set_id)
        .await
        .map_err(|e| e.to_string())
}

/// 初始化内置字典
#[tauri::command]
pub async fn initialize_builtin_dictionaries(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    dictionary_service
        .initialize_builtin_dictionaries()
        .await
        .map_err(|e| e.to_string())
}

// 兼容性命令 - 保持与原有子域名字典API的兼容性

/// 获取子域名字典（兼容性命令）
#[tauri::command]
pub async fn get_subdomain_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 查找内置的子域名字典
    if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        let words = dictionary_service
            .get_dictionary_words(&dict.id)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
        Ok(words.into_iter().map(|w| w.word).collect())
    } else {
        Ok(vec![])
    }
}

/// 设置子域名字典（兼容性命令）
#[tauri::command]
pub async fn set_subdomain_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    words: Vec<String>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 查找或创建子域名字典
    let dict_id = if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        dict.id
    } else {
        let dict = Dictionary::new(
            "Common Subdomains".to_string(),
            DictionaryType::Subdomain,
            Some(ServiceType::Web),
            Some("Common subdomain names for reconnaissance".to_string()),
        );
        dictionary_service
            .create_dictionary(dict)
            .await
            .map_err(|e| e.to_string())?
            .id
    };

    // 清空现有词条并添加新的
    dictionary_service
        .clear_dictionary(&dict_id)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    dictionary_service
        .add_words(&dict_id, words)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(())
}

/// 添加子域名词条（兼容性命令）
#[tauri::command]
pub async fn add_subdomain_words(
    db_service: State<'_, Arc<DatabaseService>>,
    words: Vec<String>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 查找子域名字典
    if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        dictionary_service
            .add_words(&dict.id, words)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
    }

    Ok(())
}

/// 移除子域名词条（兼容性命令）
#[tauri::command]
pub async fn remove_subdomain_words(
    db_service: State<'_, Arc<DatabaseService>>,
    words: Vec<String>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 查找子域名字典
    if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        dictionary_service
            .remove_words(&dict.id, words)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
    }

    Ok(())
}

/// 重置子域名字典（兼容性命令）
#[tauri::command]
pub async fn reset_subdomain_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 删除现有字典并重新初始化
    if dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        dictionary_service
            .delete_dictionary("builtin_subdomain_common")
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
    }

    dictionary_service
        .initialize_builtin_dictionaries()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(())
}

/// 导入子域名字典（兼容性命令）
#[tauri::command(rename_all = "snake_case")]
pub async fn import_subdomain_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    file_content: String,
) -> Result<(), String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    let words: Vec<String> = file_content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    // 查找或创建子域名字典
    let dict_id = if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        dict.id
    } else {
        let dict = Dictionary::new(
            "Common Subdomains".to_string(),
            DictionaryType::Subdomain,
            Some(ServiceType::Web),
            Some("Common subdomain names for reconnaissance".to_string()),
        );
        dictionary_service
            .create_dictionary(dict)
            .await
            .map_err(|e| e.to_string())?
            .id
    };

    dictionary_service
        .add_words(&dict_id, words)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(())
}

/// 导出子域名字典（兼容性命令）
#[tauri::command]
pub async fn export_subdomain_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let pool = db_service.get_runtime_pool().map_err(|e| e.to_string())?;
    let dictionary_service = DictionaryService::new(pool.clone());

    // 查找子域名字典
    if let Some(dict) = dictionary_service
        .get_dictionary("builtin_subdomain_common")
        .await
        .map_err(|e| e.to_string())?
    {
        let words = dictionary_service
            .get_dictionary_words(&dict.id)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
        Ok(words
            .into_iter()
            .map(|w| w.word)
            .collect::<Vec<_>>()
            .join("\n"))
    } else {
        Ok(String::new())
    }
}

// --- 默认字典（DB存储） ---

/// 获取某类目的默认字典ID
#[tauri::command(rename_all = "snake_case")]
pub async fn get_default_dictionary_id(
    db_service: State<'_, Arc<DatabaseService>>,
    dict_type: String,
) -> Result<Option<String>, String> {
    let value = db_service
        .get_config("dictionary_default", &dict_type)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    let v = value.filter(|s| !s.is_empty());
    Ok(v)
}

/// 设置某类目的默认字典ID
#[tauri::command(rename_all = "snake_case")]
pub async fn set_default_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    dict_type: String,
    dictionary_id: String,
) -> Result<(), String> {
    db_service
        .set_config(
            "dictionary_default",
            &dict_type,
            &dictionary_id,
            Some("默认字典设置"),
        )
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    Ok(())
}

/// 清除某类目的默认字典
#[tauri::command(rename_all = "snake_case")]
pub async fn clear_default_dictionary(
    db_service: State<'_, Arc<DatabaseService>>,
    dict_type: String,
) -> Result<(), String> {
    // 设为空字符串代表清除
    db_service
        .set_config("dictionary_default", &dict_type, "", Some("默认字典设置"))
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    Ok(())
}

/// 获取所有类目的默认字典映射
#[tauri::command(rename_all = "snake_case")]
pub async fn get_default_dictionary_map(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<HashMap<String, String>, String> {
    let configs = db_service
        .get_configs_by_category("dictionary_default")
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    let mut map = HashMap::new();
    for c in configs {
        if let Some(val) = c.value.clone() {
            if !val.is_empty() {
                map.insert(c.key.clone(), val);
            }
        }
    }
    Ok(map)
}
