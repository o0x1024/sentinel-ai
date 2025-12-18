//! 集成模块
//!
//! 提供与被动代理、上下文摘要等外部系统的集成

use super::types::*;
use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

// ============================================================================
// 被动代理集成
// ============================================================================

/// 被动代理集成服务
pub struct PassiveProxyIntegration {
    /// 代理端口
    port: u16,
    /// 目标域名过滤
    target_domain: Option<String>,
    /// 已发现的API缓存
    discovered_apis: Vec<ApiEndpoint>,
    /// 上次轮询时间
    last_poll_time: Option<chrono::DateTime<Utc>>,
}

impl PassiveProxyIntegration {
    /// 创建新的集成服务
    pub fn new(port: u16, target_domain: Option<String>) -> Self {
        Self {
            port,
            target_domain,
            discovered_apis: Vec::new(),
            last_poll_time: None,
        }
    }

    /// 从被动代理获取新发现的API
    /// 
    /// 注意：这个方法需要访问PassiveScanState，但由于跨模块限制，
    /// 实际实现需要通过回调或者事件来获取数据
    pub async fn poll_new_apis(&mut self, proxy_requests: Vec<ProxyRequestInfo>) -> Vec<ApiEndpoint> {
        let mut new_apis = Vec::new();
        
        for req in proxy_requests {
            // 过滤域名
            if let Some(ref domain) = self.target_domain {
                if !req.host.contains(domain) {
                    continue;
                }
            }
            
            // 检查是否是新发现的API
            let is_new = !self.discovered_apis.iter().any(|api| {
                api.method == req.method && api.path == req.path
            });
            
            if is_new {
                let api = ApiEndpoint {
                    method: req.method.clone(),
                    path: req.path.clone(),
                    full_url: req.url.clone(),
                    headers: req.headers.clone(),
                    parameters: Self::extract_parameters(&req.url, req.body.as_deref()),
                    body: req.body.clone(),
                    status_code: req.status_code,
                    discovered_at: Utc::now(),
                    source_action_id: None,
                };
                
                info!("New API discovered: {} {}", api.method, api.path);
                self.discovered_apis.push(api.clone());
                new_apis.push(api);
            }
        }
        
        self.last_poll_time = Some(Utc::now());
        new_apis
    }

    /// 从URL和body提取参数
    fn extract_parameters(url: &str, body: Option<&str>) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // 从URL查询参数提取
        if let Some(query_start) = url.find('?') {
            let query = &url[query_start + 1..];
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    params.insert(
                        urlencoding::decode(key).unwrap_or_default().to_string(),
                        urlencoding::decode(value).unwrap_or_default().to_string(),
                    );
                }
            }
        }
        
        // 从JSON body提取顶层参数
        if let Some(body_str) = body {
            if let Ok(json) = serde_json::from_str::<Value>(body_str) {
                if let Some(obj) = json.as_object() {
                    for (key, value) in obj {
                        params.insert(key.clone(), value.to_string());
                    }
                }
            }
        }
        
        params
    }

    /// 获取所有发现的API
    pub fn get_all_apis(&self) -> &[ApiEndpoint] {
        &self.discovered_apis
    }

    /// 获取API统计
    pub fn get_stats(&self) -> ApiDiscoveryStats {
        let mut methods = HashMap::new();
        for api in &self.discovered_apis {
            *methods.entry(api.method.clone()).or_insert(0) += 1;
        }
        
        ApiDiscoveryStats {
            total_apis: self.discovered_apis.len(),
            methods_breakdown: methods,
            last_poll_time: self.last_poll_time,
        }
    }
}

/// 代理请求信息 (简化版，用于集成)
#[derive(Debug, Clone)]
pub struct ProxyRequestInfo {
    pub method: String,
    pub url: String,
    pub path: String,
    pub host: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub status_code: Option<u16>,
}

/// API发现统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiDiscoveryStats {
    pub total_apis: usize,
    pub methods_breakdown: HashMap<String, usize>,
    pub last_poll_time: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// 上下文摘要管理
// ============================================================================

/// 上下文摘要管理器
pub struct ContextSummaryManager {
    /// 对话历史
    conversation_history: Vec<ConversationMessage>,
    /// 已生成的摘要
    summaries: Vec<ContextSummary>,
    /// token阈值
    token_threshold: u32,
    /// 当前估算token数
    estimated_tokens: u32,
}

impl ContextSummaryManager {
    /// 创建新的摘要管理器
    pub fn new(token_threshold: u32) -> Self {
        Self {
            conversation_history: Vec::new(),
            summaries: Vec::new(),
            token_threshold,
            estimated_tokens: 0,
        }
    }

    /// 添加消息到历史
    pub fn add_message(&mut self, role: &str, content: &str, iteration: u32, has_image: bool) {
        let message = ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            iteration,
            has_image,
            timestamp: Utc::now(),
        };
        
        // 估算token数 (粗略: 4个字符约1个token)
        self.estimated_tokens += (content.len() / 4) as u32;
        if has_image {
            self.estimated_tokens += 1000; // 图片约1000 tokens
        }
        
        self.conversation_history.push(message);
    }

    /// 检查是否需要生成摘要
    pub fn needs_summary(&self) -> bool {
        self.estimated_tokens >= self.token_threshold
    }

    /// 获取需要摘要的消息
    pub fn get_messages_for_summary(&self) -> Vec<&ConversationMessage> {
        // 保留最近10条消息，其余进行摘要
        let keep_count = 10.min(self.conversation_history.len());
        let summary_count = self.conversation_history.len().saturating_sub(keep_count);
        
        self.conversation_history[..summary_count].iter().collect()
    }

    /// 应用摘要，清理旧消息
    pub fn apply_summary(&mut self, summary_content: String, iteration_end: u32) {
        let messages_for_summary = self.get_messages_for_summary();
        let iteration_start = messages_for_summary
            .first()
            .map(|m| m.iteration)
            .unwrap_or(0);
        
        // 计算摘要前后的token
        let tokens_before = self.estimated_tokens;
        
        // 移除已摘要的消息
        let keep_count = 10.min(self.conversation_history.len());
        let remove_count = self.conversation_history.len().saturating_sub(keep_count);
        self.conversation_history.drain(..remove_count);
        
        // 重新计算token
        self.estimated_tokens = self.conversation_history.iter()
            .map(|m| (m.content.len() / 4) as u32 + if m.has_image { 1000 } else { 0 })
            .sum::<u32>()
            + (summary_content.len() / 4) as u32;
        
        // 保存摘要
        let summary = ContextSummary {
            id: uuid::Uuid::new_v4().to_string(),
            content: summary_content,
            iteration_range: (iteration_start, iteration_end),
            created_at: Utc::now(),
            tokens_before,
            tokens_after: self.estimated_tokens,
        };
        
        info!(
            "Context summary applied: iterations {}-{}, tokens {} -> {}",
            iteration_start, iteration_end, tokens_before, self.estimated_tokens
        );
        
        self.summaries.push(summary);
    }

    /// 构建完整的上下文 (摘要 + 最近消息)
    pub fn build_context(&self) -> String {
        let mut context_parts = Vec::new();
        
        // 添加最新摘要
        if let Some(latest_summary) = self.summaries.last() {
            context_parts.push(format!(
                "[Previous exploration summary (iterations {}-{})]\n{}",
                latest_summary.iteration_range.0,
                latest_summary.iteration_range.1,
                latest_summary.content
            ));
        }
        
        // 添加最近消息
        for msg in &self.conversation_history {
            let prefix = if msg.role == "user" { "User" } else { "Assistant" };
            let image_note = if msg.has_image { " [with screenshot]" } else { "" };
            context_parts.push(format!(
                "[{}{}] {}",
                prefix, image_note, msg.content
            ));
        }
        
        context_parts.join("\n\n")
    }

    /// 获取当前估算token数
    pub fn get_estimated_tokens(&self) -> u32 {
        self.estimated_tokens
    }

    /// 获取摘要历史
    pub fn get_summaries(&self) -> &[ContextSummary] {
        &self.summaries
    }

    /// 生成摘要提示词
    pub fn get_summary_prompt(&self) -> String {
        let messages = self.get_messages_for_summary();
        
        let conversation_text = messages.iter()
            .map(|m| {
                let role = if m.role == "user" { "User" } else { "Assistant" };
                format!("[Iteration {}] {}: {}", m.iteration, role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        
        format!(
            r#"Please summarize the following exploration conversation. Focus on:
1. Pages visited and their key features
2. Elements interacted with and their results
3. APIs discovered and their parameters
4. Any errors or issues encountered
5. Current exploration progress and what remains

Conversation to summarize:
{}

Provide a concise but comprehensive summary that preserves all important information for continuing the exploration."#,
            conversation_text
        )
    }
}

// ============================================================================
// Takeover模式管理
// ============================================================================

/// Takeover模式管理器
pub struct TakeoverManager {
    /// 当前会话
    session: TakeoverSession,
    /// 是否启用
    enabled: bool,
}

impl TakeoverManager {
    /// 创建新的Takeover管理器
    pub fn new(enabled: bool) -> Self {
        Self {
            session: TakeoverSession::default(),
            enabled,
        }
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取当前状态
    pub fn get_status(&self) -> &TakeoverStatus {
        &self.session.status
    }

    /// 请求用户接管
    pub fn request_takeover(&mut self, reason: &str) -> bool {
        if !self.enabled {
            warn!("Takeover requested but not enabled");
            return false;
        }
        
        self.session.status = TakeoverStatus::WaitingForUser;
        self.session.reason = Some(reason.to_string());
        self.session.started_at = Some(Utc::now());
        
        info!("Takeover requested: {}", reason);
        true
    }

    /// 用户接管
    pub fn user_takeover(&mut self) {
        self.session.status = TakeoverStatus::Active;
        self.session.started_at = Some(Utc::now());
        
        info!("User has taken over control");
    }

    /// 记录用户操作
    pub fn record_user_action(
        &mut self,
        action_type: &str,
        details: Value,
        screenshot: Option<String>,
    ) {
        if !matches!(self.session.status, TakeoverStatus::Active) {
            return;
        }
        
        let action = UserAction {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: action_type.to_string(),
            details,
            timestamp: Utc::now(),
            screenshot,
        };
        
        debug!("User action recorded: {}", action_type);
        self.session.user_actions.push(action);
    }

    /// 用户归还控制权
    pub fn return_control(&mut self) {
        self.session.status = TakeoverStatus::Returned;
        self.session.ended_at = Some(Utc::now());
        
        info!(
            "User returned control after {} actions",
            self.session.user_actions.len()
        );
    }

    /// 获取用户操作摘要 (用于继续探索)
    pub fn get_actions_summary(&self) -> String {
        if self.session.user_actions.is_empty() {
            return "No user actions recorded".to_string();
        }
        
        let actions: Vec<String> = self.session.user_actions.iter()
            .map(|a| format!("- {}: {:?}", a.action_type, a.details))
            .collect();
        
        format!(
            "User performed {} actions during takeover:\n{}",
            self.session.user_actions.len(),
            actions.join("\n")
        )
    }

    /// 重置会话
    pub fn reset(&mut self) {
        self.session = TakeoverSession::default();
    }

    /// 获取当前会话
    pub fn get_session(&self) -> &TakeoverSession {
        &self.session
    }

    /// Push a user message to guide the next exploration decisions.
    pub fn push_user_message(&mut self, message: String) {
        let msg = message.trim();
        if msg.is_empty() {
            return;
        }
        self.session.user_messages.push(msg.to_string());
        info!("User message queued for VisionExplorer ({} chars)", msg.len());
    }

    /// Drain all pending user messages (clears the queue).
    pub fn drain_user_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.session.user_messages)
    }

    // ========== 登录检测与凭据处理 ==========

    /// 请求用户接管（因登录需求）
    pub fn request_login_takeover(&mut self, reason: &str, fields: Option<Vec<super::types::LoginField>>) -> bool {
        if !self.enabled {
            warn!("Login takeover requested but takeover not enabled");
            return false;
        }
        
        // NOTE:
        // `login_detected` is used by the explorer loop to infer whether an automated login
        // attempt has already happened (to avoid infinite retries).
        // When we are merely waiting for the user to provide credentials, we MUST NOT set it to true,
        // otherwise the next iteration will wrongly treat it as a "login attempt already happened"
        // and report "login failed" immediately.
        self.session.status = TakeoverStatus::WaitingForUser;
        self.session.reason = Some(reason.to_string());
        self.session.started_at = Some(Utc::now());
        self.session.login_fields = fields;
        self.session.login_retry_count = 0;
        self.session.login_requested_at = Some(Utc::now());
        
        info!("Login takeover requested: {}", reason);
        true
    }

    /// 设置用户凭据（由前端调用）
    pub fn set_user_credentials(
        &mut self, 
        username: String, 
        password: String,
        verification_code: Option<String>,
        extra_fields: Option<std::collections::HashMap<String, String>>,
    ) {
        use super::types::LoginCredentials;
        self.session.user_credentials = Some(LoginCredentials { 
            username, 
            password,
            verification_code,
            extra_fields,
        });
        info!("User credentials received and stored for login");
    }

    /// 检查是否有用户凭据
    pub fn has_credentials(&self) -> bool {
        self.session.user_credentials.is_some()
    }

    /// 获取用户凭据（用于 LLM 上下文）
    pub fn get_credentials(&self) -> Option<&super::types::LoginCredentials> {
        self.session.user_credentials.as_ref()
    }

    /// 检查是否检测到登录页面
    pub fn is_login_detected(&self) -> bool {
        self.session.login_detected
    }

    /// 标记已检测到登录页面（等待 LLM 使用凭据登录）
    pub fn mark_login_detected(&mut self) {
        self.session.login_detected = true;
        self.session.login_retry_count = 0;
        info!("Login page detected, waiting for LLM to perform login");
    }

    /// 清除登录检测状态（登录成功后调用）
    pub fn clear_login_detected(&mut self) {
        self.session.login_detected = false;
        self.session.login_retry_count = 0;
        info!("Login detection cleared");
    }

    pub fn increment_login_retry(&mut self) -> u32 {
        self.session.login_retry_count = self.session.login_retry_count.saturating_add(1);
        self.session.login_retry_count
    }

    pub fn get_login_retry_count(&self) -> u32 {
        self.session.login_retry_count
    }

    pub fn reset_login_retry_count(&mut self) {
        self.session.login_retry_count = 0;
    }

    /// 获取凭据摘要（用于 LLM 上下文，不泄露密码）
    pub fn get_credentials_summary(&self) -> Option<String> {
        self.session.user_credentials.as_ref().map(|creds| {
            format!("用户已提供登录凭据 - 用户名: {}", creds.username)
        })
    }

    /// Get credentials for LLM (with security considerations)
    /// Password is masked in logs but provided to LLM for automated login
    pub fn get_credentials_for_llm(&self) -> Option<String> {
        self.session.user_credentials.as_ref().map(|creds| {
            // Build credential info for LLM to perform login
            // Note: LLM needs actual values to fill login forms
            let mut info = format!(
                "Username: {}\nPassword: {}",
                creds.username,
                creds.password
            );
            
            if let Some(ref code) = creds.verification_code {
                if !code.is_empty() {
                    info.push_str(&format!("\nVerification code: {}", code));
                }
            }
            
            if let Some(ref extras) = creds.extra_fields {
                for (key, value) in extras {
                    if !value.is_empty() {
                        info.push_str(&format!("\n{}: {}", key, value));
                    }
                }
            }
            
            // Security note: This info is only sent to the LLM, not logged
            info
        })
    }
    
    /// Get masked credentials for logging/display (password hidden)
    pub fn get_credentials_masked(&self) -> Option<String> {
        self.session.user_credentials.as_ref().map(|creds| {
            let password_mask = "*".repeat(creds.password.len().min(8));
            let mut info = format!(
                "Username: {}, Password: {}",
                creds.username,
                password_mask
            );
            
            if creds.verification_code.is_some() {
                info.push_str(", Verification code: ***");
            }
            
            if let Some(ref extras) = creds.extra_fields {
                let extra_count = extras.len();
                if extra_count > 0 {
                    info.push_str(&format!(", +{} extra fields", extra_count));
                }
            }
            
            info
        })
    }

    // ========== 跳过登录 ==========

    /// 用户选择跳过登录
    pub fn mark_login_skipped(&mut self) {
        self.session.login_skipped = true;
        self.session.login_requested_at = None;

        self.push_user_message(
            "No credentials are available. Skip login and continue exploring any publicly accessible pages. Do not ask for credentials again."
                .to_string(),
        );

        info!("Login skipped by user; continuing exploration without credentials");
    }

    /// 检查用户是否选择跳过登录
    pub fn is_login_skipped(&self) -> bool {
        self.session.login_skipped
    }

    /// 检查登录等待是否超时（默认 30 秒）
    pub fn is_login_timeout(&self, timeout_secs: i64) -> bool {
        if let Some(requested_at) = self.session.login_requested_at {
            let elapsed = Utc::now().signed_duration_since(requested_at).num_seconds();
            elapsed >= timeout_secs
        } else {
            false
        }
    }

    /// 自动跳过登录（超时触发）
    pub fn auto_skip_login(&mut self) {
        self.session.login_skipped = true;
        self.session.login_requested_at = None;

        self.push_user_message(
            "Credentials input timed out. Skipping login and continuing with publicly accessible pages."
                .to_string(),
        );

        info!("Login auto-skipped due to timeout; continuing exploration without credentials");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_summary_manager() {
        let mut manager = ContextSummaryManager::new(1000);
        
        // 添加一些消息
        for i in 0..20 {
            manager.add_message(
                if i % 2 == 0 { "user" } else { "assistant" },
                &format!("Test message {}", i),
                i,
                false,
            );
        }
        
        // 检查是否需要摘要
        assert!(manager.needs_summary() || manager.get_estimated_tokens() > 0);
    }

    #[test]
    fn test_takeover_manager() {
        let mut manager = TakeoverManager::new(true);
        
        assert!(manager.request_takeover("Test reason"));
        assert!(matches!(manager.get_status(), TakeoverStatus::WaitingForUser));
        
        manager.user_takeover();
        assert!(matches!(manager.get_status(), TakeoverStatus::Active));
        
        manager.record_user_action("click", serde_json::json!({"x": 100, "y": 200}), None);
        
        manager.return_control();
        assert!(matches!(manager.get_status(), TakeoverStatus::Returned));
    }
}

