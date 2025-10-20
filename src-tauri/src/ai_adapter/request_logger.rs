//! HTTP请求日志记录器
//! 
//! 记录所有LLM HTTP请求到日志文件

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};
use url::Url;

use crate::ai_adapter::types::{HttpRequest, HttpResponse};
use crate::ai_adapter::error::{AiAdapterError, Result};

/// HTTP请求日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestLogEntry {
    /// 时间戳
    pub timestamp: String,
    /// 请求ID（用于关联请求和响应）
    pub request_id: String,
    /// 提供商名称
    pub provider: String,
    /// 模型名称
    pub model: Option<String>,
    /// 请求信息
    pub request: HttpRequest,
    /// 响应信息（可能为空，如果请求失败）
    pub response: Option<HttpResponse>,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 请求类型（chat, completion, embedding等）
    pub request_type: String,
}

/// HTTP请求日志记录器
#[derive(Debug, Clone)]
pub struct HttpRequestLogger {
    /// 日志目录路径
    log_dir: PathBuf,
    /// 是否启用日志记录
    enabled: bool,
    /// 文件写入锁
    write_lock: Arc<Mutex<()>>,
}

impl HttpRequestLogger {
    /// 创建新的HTTP请求日志记录器
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            enabled: true,
            write_lock: Arc::new(Mutex::new(())),
        }
    }
    
    /// 启用或禁用日志记录
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// 检查是否启用日志记录
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 记录HTTP请求日志
    pub async fn log_request(
        &self,
        provider: &str,
        model: Option<&str>,
        request_type: &str,
        request: &HttpRequest,
        response: Option<&HttpResponse>,
        error: Option<&str>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        // 生成请求ID
        let request_id = self.generate_request_id();
        
        // 创建日志条目
        let log_entry = HttpRequestLogEntry {
            timestamp: self.format_timestamp(request.timestamp),
            request_id,
            provider: provider.to_string(),
            model: model.map(|m| m.to_string()),
            request: request.clone(),
            response: response.cloned(),
            error: error.map(|e| e.to_string()),
            request_type: request_type.to_string(),
        };
        
        // 写入日志文件
        self.write_log_entry(&log_entry).await?;
        
        Ok(())
    }
    
    /// 生成请求ID
    fn generate_request_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        format!("req_{:x}", hasher.finish())
    }
    
    /// 格式化时间戳
    fn format_timestamp(&self, timestamp: SystemTime) -> String {
        let datetime: DateTime<Utc> = timestamp.into();
        datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string()
    }
    
    /// 获取日志文件路径
    fn get_log_file_path(&self) -> PathBuf {
        let now = chrono::Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        self.log_dir.join(format!("llm-http-requests-{}.log", date_str))
    }
    
    /// 写入日志条目到文件
    async fn write_log_entry(&self, entry: &HttpRequestLogEntry) -> Result<()> {
        let _lock = self.write_lock.lock().map_err(|_| {
            AiAdapterError::UnknownError("Failed to acquire write lock".to_string())
        })?;
        
        // 确保日志目录存在
        if let Err(e) = std::fs::create_dir_all(&self.log_dir) {
            tracing::warn!("Failed to create log directory: {}", e);
            return Err(AiAdapterError::UnknownError(format!(
                "Failed to create log directory: {}", e
            )));
        }
        
        let log_file_path = self.get_log_file_path();
        
        // 格式化为原始HTTP请求格式
        let http_log = self.format_as_http_request(entry);
        
        // 写入文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .map_err(|e| {
                tracing::error!("Failed to open log file {:?}: {}", log_file_path, e);
                AiAdapterError::UnknownError(format!("Failed to open log file: {}", e))
            })?;
        
        writeln!(file, "{}", http_log).map_err(|e| {
            tracing::error!("Failed to write to log file: {}", e);
            AiAdapterError::UnknownError(format!("Failed to write to log file: {}", e))
        })?;
        
        // 写入分隔线
        writeln!(file, "===================================================================================================================================").map_err(|e| {
            tracing::error!("Failed to write separator to log file: {}", e);
            AiAdapterError::UnknownError(format!("Failed to write separator to log file: {}", e))
        })?;
        
        writeln!(file).map_err(|e| {
            tracing::error!("Failed to write newline to log file: {}", e);
            AiAdapterError::UnknownError(format!("Failed to write newline to log file: {}", e))
        })?;
        
        file.flush().map_err(|e| {
            tracing::error!("Failed to flush log file: {}", e);
            AiAdapterError::UnknownError(format!("Failed to flush log file: {}", e))
        })?;
        
        tracing::debug!("HTTP request logged to {:?}", log_file_path);
        Ok(())
    }
    
    /// 格式化JSON字符串，如果不是有效JSON则返回原字符串
    fn format_json_if_valid(&self, content: &str) -> String {
        // 尝试解析并重新格式化JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json_value) => {
                // 使用pretty格式输出，缩进2个空格
                match serde_json::to_string_pretty(&json_value) {
                    Ok(formatted) => formatted,
                    Err(_) => content.to_string(), // 格式化失败，返回原内容
                }
            }
            Err(_) => content.to_string(), // 不是有效JSON，返回原内容
        }
    }

    /// 将日志条目格式化为原始HTTP请求格式
    fn format_as_http_request(&self, entry: &HttpRequestLogEntry) -> String {
        let mut result = String::new();
        
        // 解析URL获取路径和主机
        let url = &entry.request.url;
        let (host, path) = if let Ok(parsed_url) = Url::parse(url) {
            let host = parsed_url.host_str().unwrap_or("unknown");
            let path = if parsed_url.query().is_some() {
                format!("{}?{}", parsed_url.path(), parsed_url.query().unwrap())
            } else {
                parsed_url.path().to_string()
            };
            (host.to_string(), path)
        } else {
            ("unknown".to_string(), "/".to_string())
        };
        
        // HTTP请求行
        result.push_str(&format!("{} {} HTTP/2\n", entry.request.method, path));
        
        // Host头部
        result.push_str(&format!("Host: {}\n", host));
        
        // 其他头部
        for (key, value) in &entry.request.headers {
            if key.to_lowercase() != "host" {
                result.push_str(&format!("{}: {}\n", key, value));
            }
        }
        
        // 空行分隔头部和请求体
        result.push('\n');
        
        // 请求体 - 格式化JSON
        if let Some(body) = &entry.request.body {
            let formatted_body = self.format_json_if_valid(body);
            result.push_str(&formatted_body);
        }
        
        // 如果有响应信息，也记录响应
        if let Some(response) = &entry.response {
            result.push_str("\n\n--- RESPONSE ---\n");
            result.push_str(&format!("HTTP/2 {} {}\n", 
                response.status, 
                self.get_status_text(response.status)
            ));
            
            // 响应头部
            for (key, value) in &response.headers {
                result.push_str(&format!("{}: {}\n", key, value));
            }
            
            result.push('\n');
            
            // 响应体 - 格式化JSON
            if let Some(body) = &response.body {
                let formatted_body = self.format_json_if_valid(body);
                result.push_str(&formatted_body);
            }
        }
        
        // 如果有错误信息，记录错误
        if let Some(error) = &entry.error {
            result.push_str(&format!("\n\n--- ERROR ---\n{}", error));
        }
        
        // 添加元数据注释
        result.push_str(&format!("\n\n--- METADATA ---\n"));
        result.push_str(&format!("Timestamp: {}\n", entry.timestamp));
        result.push_str(&format!("Request ID: {}\n", entry.request_id));
        result.push_str(&format!("Provider: {}\n", entry.provider));
        if let Some(model) = &entry.model {
            result.push_str(&format!("Model: {}\n", model));
        }
        result.push_str(&format!("Request Type: {}\n", entry.request_type));
        
        if let Some(response) = &entry.response {
            result.push_str(&format!("Duration: {}ms\n", response.duration.as_millis()));
        }
        
        result
    }
    
    /// 获取HTTP状态码对应的文本
    fn get_status_text(&self, status: u16) -> &'static str {
        match status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
    
    /// 清理旧的日志文件（保留指定天数）
    pub async fn cleanup_old_logs(&self, keep_days: u32) -> Result<()> {
        if !self.log_dir.exists() {
            return Ok(());
        }
        
        let cutoff_time = SystemTime::now() - Duration::from_secs(keep_days as u64 * 24 * 60 * 60);
        
        let entries = std::fs::read_dir(&self.log_dir).map_err(|e| {
            AiAdapterError::UnknownError(format!("Failed to read log directory: {}", e))
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                AiAdapterError::UnknownError(format!("Failed to read directory entry: {}", e))
            })?;
            
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("llm-http-requests-") && file_name.ends_with(".log") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if modified < cutoff_time {
                                    if let Err(e) = std::fs::remove_file(&path) {
                                        tracing::warn!("Failed to remove old log file {:?}: {}", path, e);
                                    } else {
                                        tracing::info!("Removed old log file: {:?}", path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// 全局HTTP请求日志记录器实例
static GLOBAL_LOGGER: std::sync::OnceLock<Arc<Mutex<Option<HttpRequestLogger>>>> = std::sync::OnceLock::new();

fn global_logger() -> &'static Arc<Mutex<Option<HttpRequestLogger>>> {
    GLOBAL_LOGGER.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// 初始化全局HTTP请求日志记录器
pub fn init_global_logger(log_dir: PathBuf) -> Result<()> {
    let logger = HttpRequestLogger::new(log_dir);
    
    if let Ok(mut global) = global_logger().lock() {
        *global = Some(logger);
        tracing::info!("HTTP request logger initialized");
        Ok(())
    } else {
        Err(AiAdapterError::UnknownError("Failed to initialize global logger".to_string()))
    }
}

/// 设置全局日志记录器的启用状态
pub fn set_global_logger_enabled(enabled: bool) -> Result<()> {
    if let Ok(mut global) = global_logger().lock() {
        if let Some(ref mut logger) = *global {
            logger.set_enabled(enabled);
            tracing::info!("HTTP request logging {}", if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(AiAdapterError::UnknownError("Global logger not initialized".to_string()))
        }
    } else {
        Err(AiAdapterError::UnknownError("Failed to access global logger".to_string()))
    }
}

/// 记录HTTP请求到全局日志记录器
pub async fn log_http_request(
    provider: &str,
    model: Option<&str>,
    request_type: &str,
    request: &HttpRequest,
    response: Option<&HttpResponse>,
    error: Option<&str>,
) -> Result<()> {
    // 获取日志记录器的克隆，避免在await期间持有锁
    let logger_clone = {
        if let Ok(global) = global_logger().lock() {
            global.clone()
        } else {
            return Ok(()); // 如果无法获取锁，静默忽略
        }
    };
    
    if let Some(logger) = logger_clone {
        logger.log_request(provider, model, request_type, request, response, error).await
    } else {
        // 如果全局日志记录器未初始化，静默忽略
        Ok(())
    }
}

/// 清理旧的日志文件
pub async fn cleanup_old_logs(keep_days: u32) -> Result<()> {
    let logger_clone = {
        if let Ok(global) = global_logger().lock() {
            global.clone()
        } else {
            return Ok(());
        }
    };
    
    if let Some(logger) = logger_clone {
        logger.cleanup_old_logs(keep_days).await
    } else {
        Ok(())
    }
}

/// 检查全局日志记录器是否已启用
pub fn is_global_logger_enabled() -> bool {
    if let Ok(global) = global_logger().lock() {
        if let Some(ref logger) = *global {
            logger.is_enabled()
        } else {
            false
        }
    } else {
        false
    }
}
