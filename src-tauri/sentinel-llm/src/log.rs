//! LLM 请求/响应日志记录模块

use std::fs::OpenOptions;
use std::io::Write;

/// 写入 LLM 日志
pub fn write_llm_log(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    log_type: &str,
    content: &str,
) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    let log_entry = format!(
        "[{}] [{}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] {}\n",
        timestamp,
        log_type,
        session_id,
        conversation_id.unwrap_or("N/A"),
        provider,
        model,
        content
    );

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    // 写入专门的 LLM 请求日志文件
    let log_file_path = format!(
        "logs/llm-http-requests-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    );

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!("Failed to write to LLM log file {}: {}", log_file_path, e);
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open LLM log file {}: {}", log_file_path, e);
        }
    }
}

/// 记录 LLM 请求
pub fn log_request(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
) {
    log_request_with_image(session_id, conversation_id, provider, model, system_prompt, user_prompt, false);
}

/// 记录 LLM 请求（含图片标记）
pub fn log_request_with_image(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    has_image: bool,
) {
    // 记录 system prompt（如果存在）
    if let Some(_sp) = system_prompt {
        write_llm_log(
            session_id,
            conversation_id,
            provider,
            model,
            "SYSTEM REQUEST",
            &format!("\n{}\n", _sp),
        );
    }
    // 记录 user prompt（含图片标记）
    let image_tag = if has_image { " [WITH IMAGE]" } else { "" };
    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        &format!("USER REQUEST{}", image_tag),
        &format!("\n{}\n", user_prompt),
    );
}

/// 记录 LLM 响应
pub fn log_response(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    response: &str,
) {
    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        "OUTPUT RESPONSE",
        &format!("\n{}\n", response),
    );
}

