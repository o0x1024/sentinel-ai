use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDictionaryResponse {
    pub words: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetDictionaryRequest {
    pub words: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddWordsRequest {
    pub words: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveWordsRequest {
    pub words: Vec<String>,
}

/// 获取子域名字典
#[tauri::command]
pub async fn get_subdomain_dictionary(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<GetDictionaryResponse, String> {
    match db.get_subdomain_dictionary().await {
        Ok(words) => {
            let count = words.len();
            Ok(GetDictionaryResponse { words, count })
        }
        Err(e) => Err(format!("获取子域名字典失败: {}", e)),
    }
}

/// 设置子域名字典（完全替换）
#[tauri::command]
pub async fn set_subdomain_dictionary(
    request: SetDictionaryRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 验证输入
    if request.words.is_empty() {
        return Err("字典不能为空".to_string());
    }
    
    // 去重并排序
    let mut words = request.words;
    words.sort();
    words.dedup();
    
    match db.set_subdomain_dictionary(&words).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("设置子域名字典失败: {}", e)),
    }
}

/// 添加词汇到子域名字典
#[tauri::command]
pub async fn add_subdomain_words(
    request: AddWordsRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    if request.words.is_empty() {
        return Err("要添加的词汇不能为空".to_string());
    }
    
    // 过滤空字符串和重复项
    let words: Vec<String> = request.words
        .into_iter()
        .filter(|w| !w.trim().is_empty())
        .map(|w| w.trim().to_lowercase())
        .collect();
    
    if words.is_empty() {
        return Err("没有有效的词汇可添加".to_string());
    }
    
    match db.add_subdomain_words(&words).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("添加子域名词汇失败: {}", e)),
    }
}

/// 从子域名字典中移除词汇
#[tauri::command]
pub async fn remove_subdomain_words(
    request: RemoveWordsRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    if request.words.is_empty() {
        return Err("要移除的词汇不能为空".to_string());
    }
    
    match db.remove_subdomain_words(&request.words).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("移除子域名词汇失败: {}", e)),
    }
}

/// 重置为默认字典
#[tauri::command]
pub async fn reset_subdomain_dictionary(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 获取默认字典
    let default_dict = vec![
        "www".to_string(),
        "mail".to_string(),
        "ftp".to_string(),
        "localhost".to_string(),
        "webmail".to_string(),
        "smtp".to_string(),
        "pop".to_string(),
        "ns1".to_string(),
        "webdisk".to_string(),
        "ns2".to_string(),
        "cpanel".to_string(),
        "whm".to_string(),
        "autodiscover".to_string(),
        "autoconfig".to_string(),
        "m".to_string(),
        "imap".to_string(),
        "test".to_string(),
        "ns".to_string(),
        "blog".to_string(),
        "pop3".to_string(),
        "dev".to_string(),
        "www2".to_string(),
        "admin".to_string(),
        "forum".to_string(),
        "news".to_string(),
        "vpn".to_string(),
        "ns3".to_string(),
        "mail2".to_string(),
        "new".to_string(),
        "mysql".to_string(),
        "old".to_string(),
        "lists".to_string(),
        "support".to_string(),
        "mobile".to_string(),
        "static".to_string(),
        "docs".to_string(),
        "beta".to_string(),
        "shop".to_string(),
        "sql".to_string(),
        "secure".to_string(),
        "demo".to_string(),
        "cp".to_string(),
        "calendar".to_string(),
        "wiki".to_string(),
        "web".to_string(),
        "media".to_string(),
        "email".to_string(),
        "images".to_string(),
        "img".to_string(),
        "www1".to_string(),
        "intranet".to_string(),
        "portal".to_string(),
        "video".to_string(),
        "sip".to_string(),
        "dns2".to_string(),
        "api".to_string(),
        "cdn".to_string(),
        "stats".to_string(),
        "dns1".to_string(),
        "ns4".to_string(),
        "www3".to_string(),
        "dns".to_string(),
        "search".to_string(),
        "staging".to_string(),
        "server".to_string(),
        "mx".to_string(),
        "chat".to_string(),
        "en".to_string(),
        "wap".to_string(),
        "redmine".to_string(),
        "ftp2".to_string(),
        "db".to_string(),
        "erp".to_string(),
        "explore".to_string(),
        "download".to_string(),
        "ww1".to_string(),
        "catalog".to_string(),
        "ssh".to_string(),
        "management".to_string(),
        "www4".to_string(),
    ];
    
    match db.set_subdomain_dictionary(&default_dict).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("重置子域名字典失败: {}", e)),
    }
}

/// 导入字典文件（从文本内容）
#[tauri::command]
pub async fn import_subdomain_dictionary(
    content: String,
    replace: bool, // true: 替换现有字典, false: 追加到现有字典
    db: State<'_, Arc<DatabaseService>>,
) -> Result<usize, String> {
    // 解析文本内容，每行一个词汇
    let words: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_lowercase())
        .filter(|line| !line.is_empty() && !line.starts_with('#')) // 过滤空行和注释
        .collect();
    
    if words.is_empty() {
        return Err("没有找到有效的词汇".to_string());
    }
    
    let words_count = words.len();
    
    let result = if replace {
        // 替换现有字典
        let mut unique_words = words;
        unique_words.sort();
        unique_words.dedup();
        db.set_subdomain_dictionary(&unique_words).await
    } else {
        // 追加到现有字典
        db.add_subdomain_words(&words).await
    };
    
    match result {
        Ok(_) => Ok(words_count),
        Err(e) => Err(format!("导入字典失败: {}", e)),
    }
}

/// 导出字典为文本格式
#[tauri::command]
pub async fn export_subdomain_dictionary(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    match db.get_subdomain_dictionary().await {
        Ok(words) => {
            let mut content = String::from("# 子域名扫描字典\n# 每行一个子域名\n\n");
            for word in words {
                content.push_str(&word);
                content.push('\n');
            }
            Ok(content)
        }
        Err(e) => Err(format!("导出字典失败: {}", e)),
    }
}