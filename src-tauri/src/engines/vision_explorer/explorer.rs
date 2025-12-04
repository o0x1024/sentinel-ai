//! 视觉探索引擎核心实现
//!
//! 实现VLM驱动的迭代式网站探索循环
//! 
//! ## 新增功能
//! - 多模态VLM调用：支持截图图片输入
//! - 被动代理集成：实时获取发现的API
//! - Takeover模式：支持人工接管浏览器
//! - 上下文摘要：长任务时自动生成摘要避免token溢出

use super::integrations::{ContextSummaryManager, PassiveProxyIntegration, ProxyRequestInfo, TakeoverManager};
use super::message_emitter::{VisionExplorerMessageEmitter, VisionAnalysis, VisionAction, VisionExplorationStats};
use super::state::{StateManager, ExplorationSummary};
use crate::utils::ordered_message::ArchitectureType;
use super::tools::BrowserTools;
use super::types::*;
use crate::commands::passive_scan_commands::PassiveScanState;
use crate::engines::{LlmClient, LlmConfig};
use crate::models::prompt::TemplateType;
use crate::services::mcp::McpService;
use crate::services::prompt_db::PromptRepository;
use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_llm::MessageImageAttachment as ImageAttachment;
use sentinel_passive::{PassiveDatabaseService, ProxyConfig};
use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// 系统提示词模板 (默认回退模板)
const DEFAULT_SYSTEM_PROMPT_TEMPLATE: &str = include_str!("prompt.md");

/// 摘要生成提示词
const SUMMARY_PROMPT_TEMPLATE: &str = r#"You are a helpful assistant that summarizes exploration conversations.
Your job is to create concise summaries that preserve all important information.

Focus on:
- Pages visited and their key features
- Elements interacted with and their results
- APIs discovered and their parameters
- Any errors or issues encountered
- Current exploration progress and what remains

Provide a structured summary that can be used as context for continuing the exploration."#;

/// Takeover事件
#[derive(Debug, Clone)]
pub enum TakeoverEvent {
    /// 请求用户接管
    RequestTakeover { reason: String },
    /// 用户已接管
    UserTakeover,
    /// 用户操作
    UserAction { action_type: String, details: Value },
    /// 用户归还控制
    ReturnControl,
}

/// 视觉探索引擎
pub struct VisionExplorer {
    config: VisionExplorerConfig,
    browser_tools: BrowserTools,
    state_manager: Arc<RwLock<StateManager>>,
    llm_config: LlmConfig,
    is_running: Arc<RwLock<bool>>,
    // 新增：集成模块
    context_manager: Arc<RwLock<ContextSummaryManager>>,
    passive_proxy: Arc<RwLock<PassiveProxyIntegration>>,
    takeover_manager: Arc<RwLock<TakeoverManager>>,
    // Takeover事件通道
    takeover_tx: Option<mpsc::UnboundedSender<TakeoverEvent>>,
    takeover_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<TakeoverEvent>>>>,
    // 被动扫描数据库服务 (用于获取代理请求)
    passive_db: Option<Arc<PassiveDatabaseService>>,
    // 上次轮询代理请求的 ID
    last_polled_request_id: Arc<RwLock<i64>>,
    // 提示词仓库 (用于动态配置 system prompt)
    prompt_repo: Option<Arc<PromptRepository>>,
    // Tauri AppHandle (用于启动代理)
    app_handle: Option<AppHandle>,
    // 被动扫描状态 (用于启动代理)
    passive_scan_state: Option<Arc<PassiveScanState>>,
    // 消息发送器 (用于前端显示)
    message_emitter: Option<Arc<VisionExplorerMessageEmitter>>,
    // 取消令牌 (用于响应外部停止请求)
    cancellation_token: Option<CancellationToken>,
}

impl VisionExplorer {
    /// 创建新的视觉探索引擎
    pub fn new(
        config: VisionExplorerConfig,
        mcp_service: Arc<McpService>,
        llm_config: LlmConfig,
    ) -> Self {
        let browser_tools = BrowserTools::new(mcp_service, config.clone());
        let state_manager = Arc::new(RwLock::new(StateManager::new(
            config.target_url.clone(),
            config.max_iterations,
        )));
        
        // 初始化集成模块
        let context_manager = Arc::new(RwLock::new(ContextSummaryManager::new(
            config.context_summary_threshold,
        )));
        
        let target_domain = extract_domain(&config.target_url);
        let passive_proxy = Arc::new(RwLock::new(PassiveProxyIntegration::new(
            config.passive_proxy_port.unwrap_or(4201),
            target_domain,
        )));
        
        let takeover_manager = Arc::new(RwLock::new(TakeoverManager::new(
            config.enable_takeover,
        )));
        
        // 创建Takeover事件通道
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            config,
            browser_tools,
            state_manager,
            llm_config,
            is_running: Arc::new(RwLock::new(false)),
            context_manager,
            passive_proxy,
            takeover_manager,
            takeover_tx: Some(tx),
            takeover_rx: Arc::new(RwLock::new(Some(rx))),
            passive_db: None,
            last_polled_request_id: Arc::new(RwLock::new(0)),
            prompt_repo: None,
            app_handle: None,
            passive_scan_state: None,
            message_emitter: None,
            cancellation_token: None,
        }
    }

    /// 设置被动扫描数据库服务
    pub fn with_passive_db(mut self, db: Arc<PassiveDatabaseService>) -> Self {
        self.passive_db = Some(db);
        self
    }

    /// 设置提示词仓库
    pub fn with_prompt_repo(mut self, repo: Arc<PromptRepository>) -> Self {
        self.prompt_repo = Some(repo);
        self
    }

    /// 设置 Tauri AppHandle
    pub fn with_app_handle(mut self, app: AppHandle) -> Self {
        // 初始化消息发送器 (如果配置了 execution_id 和 message_id)
        if let (Some(exec_id), Some(msg_id)) = (&self.config.execution_id, &self.config.message_id) {
            info!("VisionExplorer: Initializing message emitter with exec_id={}, msg_id={}", exec_id, msg_id);
            self.message_emitter = Some(Arc::new(VisionExplorerMessageEmitter::new(
                Arc::new(app.clone()),
                exec_id.clone(),
                msg_id.clone(),
                self.config.conversation_id.clone(),
                self.config.finalize_on_complete,
            )));
        } else {
            warn!("VisionExplorer: No execution_id or message_id in config, message emitter not initialized");
        }
        self.app_handle = Some(app);
        self
    }

    /// 设置被动扫描状态
    pub fn with_passive_scan_state(mut self, state: Arc<PassiveScanState>) -> Self {
        self.passive_scan_state = Some(state);
        self
    }

    /// 设置取消令牌 (用于响应外部停止请求)
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// 设置父架构类型（当作为子流运行时，如 Travel OODA 的 Observe 阶段）
    /// 设置后，所有消息将使用父架构类型发送，保持与父流的消息顺序一致
    pub fn with_parent_architecture(self, arch: ArchitectureType) -> Self {
        if let Some(emitter) = &self.message_emitter {
            emitter.set_parent_architecture(arch);
        }
        self
    }

    /// 使用AI服务配置创建
    pub fn with_ai_config(
        config: VisionExplorerConfig,
        mcp_service: Arc<McpService>,
        provider: String,
        model: String,
    ) -> Self {
        let llm_config = LlmConfig::new(&provider, &model)
            .with_timeout(120);
        Self::new(config, mcp_service, llm_config)
    }

    /// 获取Takeover事件发送器 (用于外部触发Takeover)
    pub fn get_takeover_sender(&self) -> Option<mpsc::UnboundedSender<TakeoverEvent>> {
        self.takeover_tx.clone()
    }

    /// 开始探索
    pub async fn start(&self) -> Result<ExplorationSummary> {
        // 检查是否已在运行
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(anyhow!("Explorer is already running"));
            }
            *is_running = true;
        }

        info!("Starting vision exploration for: {}", self.config.target_url);
        info!("Features enabled - Multimodal: {}, Context Summary: {}, Takeover: {}, Passive Proxy: {}",
            self.config.enable_multimodal,
            self.config.enable_context_summary,
            self.config.enable_takeover,
            self.config.enable_passive_proxy);

        // 更新状态
        {
            let mut state = self.state_manager.write().await;
            state.update_status(ExplorationStatus::Exploring);
        }

        // 执行探索循环
        let result = self.exploration_loop().await;

        // 标记停止
        {
            let mut is_running = self.is_running.write().await;
            *is_running = false;
        }

        // 返回摘要
        let state = self.state_manager.read().await;
        let summary = state.get_summary();
        
        // 发送探索完成消息
        if let Some(emitter) = &self.message_emitter {
            let status = match &result {
                Ok(_) => "completed".to_string(),
                Err(e) => format!("failed: {}", e),
            };
            emitter.emit_complete(VisionExplorationStats {
                total_iterations: summary.total_iterations,
                pages_visited: summary.pages_visited,
                apis_discovered: summary.apis_discovered,
                elements_interacted: summary.elements_interacted,
                total_duration_ms: summary.duration_seconds as u64 * 1000,
                status,
            });
        }

        match result {
            Ok(_) => Ok(summary),
            Err(e) => {
                error!("Exploration failed: {}", e);
                Ok(summary)
            }
        }
    }

    /// 停止探索
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        let mut state = self.state_manager.write().await;
        state.update_status(ExplorationStatus::Paused);
        
        info!("Vision exploration stopped");
    }

    /// 请求Takeover
    pub async fn request_takeover(&self, reason: &str) -> bool {
        let mut manager = self.takeover_manager.write().await;
        let result = manager.request_takeover(reason);
        
        if result {
            let mut state = self.state_manager.write().await;
            state.update_status(ExplorationStatus::WaitingForInput);
        }
        
        result
    }

    /// 处理Takeover归还
    pub async fn handle_takeover_return(&self) {
        let mut manager = self.takeover_manager.write().await;
        manager.return_control();
        
        let mut state = self.state_manager.write().await;
        state.update_status(ExplorationStatus::Exploring);
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> ExplorationState {
        let state = self.state_manager.read().await;
        state.state().clone()
    }

    /// 获取探索摘要
    pub async fn get_summary(&self) -> ExplorationSummary {
        let state = self.state_manager.read().await;
        state.get_summary()
    }

    /// 获取Takeover状态
    pub async fn get_takeover_status(&self) -> TakeoverStatus {
        let manager = self.takeover_manager.read().await;
        manager.get_status().clone()
    }

    /// 获取上下文摘要信息
    pub async fn get_context_info(&self) -> (u32, usize) {
        let context = self.context_manager.read().await;
        (context.get_estimated_tokens(), context.get_summaries().len())
    }

    /// 主探索循环
    async fn exploration_loop(&self) -> Result<()> {
        // 第0步：启动被动代理监听
        if let (Some(app), Some(state)) = (&self.app_handle, &self.passive_scan_state) {
            info!("Step 0: Starting passive proxy listener");
            
            // 从配置中获取代理端口
            let proxy_port = self.config.browser_proxy
                .as_ref()
                .and_then(|proxy_url| {
                    // 解析 http://127.0.0.1:8080 格式的 URL
                    proxy_url.split(':').last().and_then(|p| p.parse::<u16>().ok())
                })
                .unwrap_or(8080);
            
            let proxy_config = ProxyConfig {
                start_port: proxy_port,
                max_port_attempts: 1,
                mitm_enabled: true,
                max_request_body_size: 2 * 1024 * 1024,
                max_response_body_size: 2 * 1024 * 1024,
                mitm_bypass_fail_threshold: 3,
            };
            
            match crate::commands::passive_scan_commands::start_passive_scan_internal(
                app,
                state.as_ref(),
                Some(proxy_config),
            ).await {
                Ok(port) => {
                    info!("Passive proxy started on port {}", port);
                }
                Err(e) => {
                    // 代理可能已经在运行
                    if e.contains("already running") {
                        info!("Passive proxy already running, continuing...");
                    } else {
                        warn!("Failed to start passive proxy: {}, continuing without proxy", e);
                    }
                }
            }
        } else {
            warn!("AppHandle or PassiveScanState not set, skipping proxy startup");
        }

        // 发送探索开始消息
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_start(&self.config.target_url);
        }

        // 第1步：导航到目标URL
        info!("Step 1: Navigating to target URL");
        let navigate_action = BrowserAction::Navigate {
            url: self.config.target_url.clone(),
        };
        let result = self.browser_tools.execute_action(&navigate_action).await?;
        
        {
            let mut state = self.state_manager.write().await;
            state.record_action(navigate_action, result, vec![]);
        }

        // 第2步：初始截图
        info!("Step 2: Taking initial screenshot");
        let page_state = self.browser_tools.capture_page_state().await?;
        
        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state);
        }

        // 第3步：迭代探索循环
        info!("Step 3: Starting exploration loop");
        let mut consecutive_errors = 0;
        let mut consecutive_screenshots: u32 = 0; // 跟踪连续截图次数
        
        loop {
            // 检查是否应该继续
            let should_continue = {
                let state = self.state_manager.read().await;
                state.should_continue()
            };

            if !should_continue {
                break;
            }

            // 检查是否被停止
            {
                let is_running = self.is_running.read().await;
                if !*is_running {
                    info!("VisionExplorer: Stopped via is_running flag");
                    break;
                }
            }
            
            // 检查取消令牌
            if let Some(token) = &self.cancellation_token {
                if token.is_cancelled() {
                    info!("VisionExplorer: Cancelled via CancellationToken");
                    let mut state = self.state_manager.write().await;
                    state.update_status(ExplorationStatus::Paused);
                    break;
                }
            }

            // 检查Takeover状态
            {
                let takeover = self.takeover_manager.read().await;
                if matches!(takeover.get_status(), TakeoverStatus::WaitingForUser | TakeoverStatus::Active) {
                    // 等待用户操作
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
            }

            // 检查是否需要生成上下文摘要
            if self.config.enable_context_summary {
                self.check_and_generate_summary().await?;
            }

            // 轮询被动代理获取新API
            if self.config.enable_passive_proxy {
                self.poll_passive_proxy().await;
            }

            // 执行一次迭代
            match self.run_iteration(consecutive_screenshots).await {
                Ok((should_stop, was_screenshot)) => {
                    consecutive_errors = 0;
                    
                    // 更新连续截图计数
                    if was_screenshot {
                        consecutive_screenshots += 1;
                    } else {
                        consecutive_screenshots = 0;
                    }
                    
                    if should_stop {
                        info!("Exploration completed by VLM decision");
                        break;
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error!("Iteration failed (consecutive: {}): {}", consecutive_errors, e);
                    
                    if consecutive_errors > 3 {
                        // 连续失败太多次
                        if self.config.enable_takeover {
                            // 请求用户接管
                            self.request_takeover(&format!("Multiple errors: {}", e)).await;
                        } else {
                            let mut state = self.state_manager.write().await;
                            state.set_error(e.to_string());
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行单次迭代，返回 (是否停止, 是否是截图操作)
    async fn run_iteration(&self, consecutive_screenshots: u32) -> Result<(bool, bool)> {
        let iteration = {
            let state = self.state_manager.read().await;
            state.state().iteration_count
        };
        
        debug!("Running iteration {}, consecutive_screenshots: {}", iteration, consecutive_screenshots);

        // 1. 获取当前页面状态
        let page_state = self.browser_tools.capture_page_state().await?;
        
        // 发送截图消息到前端
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_screenshot(
                iteration,
                &page_state.url,
                &page_state.title,
                None, // 不发送 base64 到前端，太大
            );
        }
        
        // 2. 更新状态
        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state.clone());
        }

        // 3. 构建VLM提示词 (分离 system_prompt 和 user_prompt)
        let (system_prompt, user_prompt) = self.build_vlm_prompt(&page_state).await?;

        // 4. 调用VLM获取下一步操作 (支持多模态)
        let vlm_response = self.call_vlm_multimodal(&system_prompt, &user_prompt, page_state.screenshot.as_deref()).await?;

        // 5. 记录对话到上下文管理器
        if self.config.enable_context_summary {
            let mut context = self.context_manager.write().await;
            context.add_message("user", &user_prompt, iteration, page_state.screenshot.is_some());
            context.add_message("assistant", &vlm_response, iteration, false);
        }

        // 6. 解析VLM响应（传入连续截图次数用于检测循环）
        let analysis = self.parse_vlm_response(&vlm_response, consecutive_screenshots)?;
        
        // 发送分析结果到前端
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_analysis(iteration, VisionAnalysis {
                page_analysis: analysis.page_analysis.clone(),
                estimated_apis: if analysis.estimated_apis.is_empty() {
                    None
                } else {
                    Some(analysis.estimated_apis.clone())
                },
                exploration_progress: analysis.exploration_progress,
            });
        }

        // 7. 检查是否需要Takeover
        if analysis.next_action.action_type == "needs_help" {
            if self.config.enable_takeover {
            self.request_takeover(&analysis.next_action.reason).await;
                return Ok((false, false)); // 不停止，等待用户
            } else {
                // 没有启用 takeover，记录错误并继续
                warn!("VLM needs help but takeover is disabled: {}", analysis.next_action.reason);
                let mut state = self.state_manager.write().await;
                state.set_error(analysis.next_action.reason.clone());
                return Ok((true, false)); // 停止探索
            }
        }

        // 8. 检查是否完成
        if analysis.is_exploration_complete {
            let mut state = self.state_manager.write().await;
            state.mark_completed(analysis.completion_reason.as_deref().unwrap_or("VLM decided exploration is complete"));
            return Ok((true, false));
        }

        // 9. 执行下一步操作
        let action = self.build_action_from_analysis(&analysis)?;
        let is_screenshot = matches!(action, BrowserAction::Screenshot);
        let action_start = std::time::Instant::now();
        let result = self.browser_tools.execute_action(&action).await?;
        let action_duration = action_start.elapsed().as_millis() as u64;
        
        // 发送操作结果到前端
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_action(iteration, VisionAction {
                action_type: analysis.next_action.action_type.clone(),
                element_index: analysis.next_action.element_index,
                value: analysis.next_action.value.clone(),
                reason: analysis.next_action.reason.clone(),
                success: result.success,
                duration_ms: Some(action_duration),
            });
        }

        // 10. 记录操作
        {
            let mut state = self.state_manager.write().await;
            state.record_action(action, result, analysis.estimated_apis.clone());
            
            // 标记元素已交互
            if let Some(element_id) = &analysis.next_action.element_id {
                state.mark_element_interacted(element_id);
            }
            
            // 更新进度
            state.calculate_progress();
        }

        Ok((false, is_screenshot))
    }

    /// 多模态VLM调用 (支持图片输入)
    async fn call_vlm_multimodal(&self, system_prompt: &str, user_prompt: &str, screenshot_base64: Option<&str>) -> Result<String> {
        let llm_client = LlmClient::new(self.llm_config.clone());
        
        // 构建图片附件
        let image = if self.config.enable_multimodal {
            screenshot_base64.map(|s| ImageAttachment::new(s, "png"))
        } else {
            None
        };
        
        // 调用LLM (使用completion_with_image支持多模态)
        let response = llm_client
            .completion_with_image(Some(system_prompt), user_prompt, image.as_ref())
            .await?;

        Ok(response)
    }

    /// 检查并生成上下文摘要
    async fn check_and_generate_summary(&self) -> Result<()> {
        let needs_summary = {
            let context = self.context_manager.read().await;
            context.needs_summary()
        };
        
        if !needs_summary {
            return Ok(());
        }
        
        info!("Generating context summary to reduce token usage");
        
        // 获取摘要提示词
        let summary_prompt = {
            let context = self.context_manager.read().await;
            context.get_summary_prompt()
        };
        
        // 调用LLM生成摘要
        let llm_client = LlmClient::new(self.llm_config.clone());
        let summary = llm_client
            .completion(Some(SUMMARY_PROMPT_TEMPLATE), &summary_prompt)
            .await?;
        
        // 应用摘要
        let iteration = {
            let state = self.state_manager.read().await;
            state.state().iteration_count
        };
        
        {
            let mut context = self.context_manager.write().await;
            context.apply_summary(summary, iteration);
        }
        
        Ok(())
    }

    /// 轮询被动代理获取新API
    async fn poll_passive_proxy(&self) {
        let Some(db) = &self.passive_db else {
            debug!("Passive database not configured, skipping poll");
            return;
        };
        
        // 获取目标域名用于过滤
        let target_domain = extract_domain(&self.config.target_url);
        
        // 获取上次轮询的 ID
        let last_id = *self.last_polled_request_id.read().await;
        
        // 从数据库获取新请求 (按 host 过滤，只获取目标域名的请求)
        let filters = sentinel_passive::ProxyRequestFilters {
            host: target_domain.clone(),
            limit: Some(100),
            ..Default::default()
        };
        
        let requests = match db.list_proxy_requests(filters).await {
            Ok(reqs) => reqs,
            Err(e) => {
                warn!("Failed to poll proxy requests: {}", e);
                return;
            }
        };
        
        if requests.is_empty() {
            return;
        }
        
        // 过滤出新请求 (ID > last_polled_id)
        let new_requests: Vec<_> = requests.iter()
            .filter(|r| r.id.unwrap_or(0) > last_id)
            .collect();
        
        if new_requests.is_empty() {
            return;
        }
        
        // 更新最后轮询的 ID
        if let Some(max_id) = new_requests.iter().filter_map(|r| r.id).max() {
            let mut last_id_guard = self.last_polled_request_id.write().await;
            *last_id_guard = max_id;
        }
        
        // 转换为 ProxyRequestInfo 格式
        let proxy_requests: Vec<ProxyRequestInfo> = new_requests.iter().map(|r| {
            ProxyRequestInfo {
                url: r.url.clone(),
                method: r.method.clone(),
                path: extract_path(&r.url),
                host: r.host.clone(),
                headers: parse_headers_json(r.request_headers.as_deref()),
                body: r.request_body.clone(),
                status_code: Some(r.status_code as u16),
            }
        }).collect();
        
        info!("Polled {} new proxy requests from passive scanner", proxy_requests.len());
        
        // 处理新请求，提取 API
        let mut proxy = self.passive_proxy.write().await;
        let new_apis = proxy.poll_new_apis(proxy_requests).await;
        
        if !new_apis.is_empty() {
            info!("Discovered {} new APIs from passive proxy", new_apis.len());
            let mut state = self.state_manager.write().await;
            state.add_discovered_apis(new_apis);
        }
    }

    /// 获取 system prompt 模板 (优先从数据库，回退到默认)
    async fn get_system_prompt_template(&self) -> String {
        if let Some(repo) = &self.prompt_repo {
            // 尝试从数据库获取激活的 VisionExplorerSystem 模板
            match repo.get_active_template_by_type(TemplateType::VisionExplorerSystem).await {
                Ok(Some(template)) => {
                    info!("Using database prompt template: {}", template.name);
                    return template.content;
                }
                Ok(None) => {
                    debug!("No active VisionExplorerSystem template in database, using default");
                }
                Err(e) => {
                    warn!("Failed to get prompt template from database: {}, using default", e);
                }
            }
        }
        // 回退到默认模板
        DEFAULT_SYSTEM_PROMPT_TEMPLATE.to_string()
    }

    /// 构建VLM提示词，返回 (system_prompt, user_prompt)
    async fn build_vlm_prompt(&self, page_state: &PageState) -> Result<(String, String)> {
        let state = self.state_manager.read().await;
        
        // 格式化操作历史
        let action_history = state.format_action_history(5);
        
        // 统计信息
        let visited_count = state.state().visited_urls.len();
        let api_count = state.state().discovered_apis.len();
        let interacted_count = state.state().interacted_elements.len();
        
        // 获取上下文摘要
        let context_summary = if self.config.enable_context_summary {
            let context = self.context_manager.read().await;
            let summaries = context.get_summaries();
            if let Some(latest) = summaries.last() {
                format!("\n\n[Previous exploration summary]\n{}", latest.content)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        // 可选：包含可交互元素 JSON（用于非多模态模型）
        let elements_section = if self.config.include_elements_in_prompt {
            let elements_to_show: Vec<_> = page_state.interactable_elements.iter()
                .take(50)
                .collect();
            let elements_json = serde_json::to_string_pretty(&elements_to_show)
                .unwrap_or_else(|_| "[]".to_string());
            format!(
                "\n可交互元素（{}，最多显示 50 个）：\n{}\n",
                page_state.interactable_elements.len(),
                elements_json
            )
        } else {
            String::new()
        };
        
        // 构建 system_prompt (优先从数据库读取，回退到默认模板)
        let system_template = self.get_system_prompt_template().await;
        let system_prompt = system_template
            .replace("{viewport_width}", &self.config.viewport_width.to_string())
            .replace("{viewport_height}", &self.config.viewport_height.to_string());
        
        // 根据是否包含元素信息，调整提示语
        let action_hint = if self.config.include_elements_in_prompt {
            "请根据上述元素信息，决定下一步操作。"
        } else {
            "请查看截图中的元素标注（索引号），决定下一步操作。"
        };
        
        // 构建 user_prompt
        let user_prompt = format!(
            r#"当前日期: {}
当前时间: {}

────────────────────────
当前探索状态
────────────────────────

- 目标网址: {}
- 访问页面数: {}
- 已发现 API 数: {}
- 已交互元素数: {}

最近操作（最近 5 次）：
{}

────────────────────────
当前页面
────────────────────────

URL: {}
标题: {}
{}{}
{}"#,
            Utc::now().format("%Y-%m-%d"),
            Utc::now().format("%H:%M:%S"),
            self.config.target_url,
            visited_count,
            api_count,
            interacted_count,
            action_history,
            page_state.url,
            page_state.title,
            elements_section,
            context_summary,
            action_hint
        );
        
        Ok((system_prompt, user_prompt))
    }

    /// 解析VLM响应
    fn parse_vlm_response(&self, response: &str, consecutive_screenshots: u32) -> Result<VlmAnalysisResult> {
        // 尝试提取JSON
        let json_str = self.extract_json_from_response(response)?;
        
        debug!("Extracted JSON from VLM response: {}", json_str);
        
        // 解析JSON
        let parsed: Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse VLM JSON response: {}. Raw JSON: {}", e, json_str);
                return Err(anyhow!("{}", e));
            }
        };
        
        // 提取字段
        let page_analysis = parsed.get("page_analysis")
            .and_then(|v| v.as_str())
            .unwrap_or("No analysis provided")
            .to_string();
        
        let mut next_action = parsed.get("next_action")
            .map(|v| VlmNextAction {
                action_type: v.get("type")
                    .or_else(|| v.get("action_type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("screenshot")
                    .to_string(),
                element_id: v.get("element_id")
                    .or_else(|| v.get("selector"))
                    .and_then(|e| e.as_str())
                    .map(String::from),
                // 新增：解析 element_index 字段
                element_index: v.get("element_index")
                    .or_else(|| v.get("index"))
                    .and_then(|e| e.as_u64())
                    .map(|n| n as u32),
                value: v.get("value")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                reason: v.get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("No reason provided")
                    .to_string(),
            })
            .unwrap_or(VlmNextAction {
                action_type: "screenshot".to_string(),
                element_id: None,
                element_index: None,
                value: None,
                reason: "Default action".to_string(),
            });
        
        // 检测连续截图循环：超过3次连续截图，强制报告问题
        if next_action.action_type == "screenshot" && consecutive_screenshots >= 3 {
            warn!("Detected screenshot loop ({} consecutive), forcing needs_help action", consecutive_screenshots);
            next_action = VlmNextAction {
                action_type: "needs_help".to_string(),
                element_id: None,
                element_index: None,
                value: None,
                reason: format!("Stuck in screenshot loop ({} consecutive screenshots). Page state may not be captured correctly.", consecutive_screenshots),
            };
        }
        
        let estimated_apis: Vec<String> = parsed.get("estimated_apis")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let exploration_progress = parsed.get("exploration_progress")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        
        let is_exploration_complete = parsed.get("is_exploration_complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || next_action.action_type == "completed"
            || next_action.action_type == "done";
        
        let completion_reason = parsed.get("completion_reason")
            .and_then(|v| v.as_str())
            .map(String::from);
        
        Ok(VlmAnalysisResult {
            page_analysis,
            next_action,
            estimated_apis,
            exploration_progress,
            is_exploration_complete,
            completion_reason,
        })
    }

    /// 从响应中提取JSON
    fn extract_json_from_response(&self, response: &str) -> Result<String> {
        // 尝试找到JSON块
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return Ok(response[start..=end].to_string());
                }
            }
        }
        
        // 尝试找到代码块中的JSON
        if let Some(start) = response.find("```json") {
            let json_start = start + 7;
            if let Some(end) = response[json_start..].find("```") {
                return Ok(response[json_start..json_start + end].trim().to_string());
            }
        }
        
        // 尝试找到普通代码块
        if let Some(start) = response.find("```") {
            let code_start = response[start + 3..].find('\n').map(|i| start + 4 + i).unwrap_or(start + 3);
            if let Some(end) = response[code_start..].find("```") {
                return Ok(response[code_start..code_start + end].trim().to_string());
            }
        }
        
        Err(anyhow!("No JSON found in response"))
    }

    /// 根据分析结果构建浏览器操作
    fn build_action_from_analysis(&self, analysis: &VlmAnalysisResult) -> Result<BrowserAction> {
        let action = &analysis.next_action;
        
        match action.action_type.as_str() {
            "screenshot" => Ok(BrowserAction::Screenshot),
            
            // 新增：通过索引点击（推荐方式）
            "click_by_index" => {
                if let Some(index) = action.element_index {
                    Ok(BrowserAction::ClickByIndex { index })
                } else if let Some(element_id) = &action.element_id {
                    // 兼容：尝试从 element_id 解析索引
                    if let Ok(index) = element_id.parse::<u32>() {
                        Ok(BrowserAction::ClickByIndex { index })
                    } else {
                        Err(anyhow!("click_by_index requires numeric element_index, got: {}", element_id))
                    }
                } else {
                    Err(anyhow!("click_by_index requires element_index"))
                }
            }
            
            // 新增：标注元素
            "annotate" | "annotate_elements" => Ok(BrowserAction::AnnotateElements),
            
            // 新增：获取元素列表
            "get_elements" | "get_annotated_elements" => Ok(BrowserAction::GetAnnotatedElements),
            
            // 新增：设置自动标注
            "set_auto_annotation" => {
                let enabled = action.value.as_deref()
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(true);
                Ok(BrowserAction::SetAutoAnnotation { enabled })
            }
            
            // 新增：通过索引填充输入框
            "fill_by_index" => {
                if let Some(index) = action.element_index {
                    let value = action.value.clone().unwrap_or_default();
                    Ok(BrowserAction::FillByIndex { index, value })
                } else if let Some(element_id) = &action.element_id {
                    if let Ok(index) = element_id.parse::<u32>() {
                        let value = action.value.clone().unwrap_or_default();
                        Ok(BrowserAction::FillByIndex { index, value })
                    } else {
                        Err(anyhow!("fill_by_index requires numeric element_index"))
                    }
                } else {
                    Err(anyhow!("fill_by_index requires element_index"))
                }
            }
            
            "click" | "click_mouse" | "computer_click_mouse" => {
                // 优先使用 element_index (索引点击)
                if let Some(index) = action.element_index {
                    return Ok(BrowserAction::ClickByIndex { index });
                }
                
                if let Some(element_id) = &action.element_id {
                    // 尝试解析为纯数字索引
                    if let Ok(index) = element_id.parse::<u32>() {
                        return Ok(BrowserAction::ClickByIndex { index });
                    }
                    // 尝试解析坐标
                    if element_id.contains(',') {
                        let parts: Vec<&str> = element_id.split(',').collect();
                        if parts.len() == 2 {
                            let x: i32 = parts[0].trim().parse().unwrap_or(0);
                            let y: i32 = parts[1].trim().parse().unwrap_or(0);
                            return Ok(BrowserAction::ClickMouse {
                                coordinates: Some(Coordinates { x, y }),
                                button: MouseButton::Left,
                                click_count: 1,
                            });
                        }
                    }
                    // 无法解析为索引或坐标，返回错误
                    Err(anyhow!("click requires numeric element_index or coordinate format (x,y), got: {}", element_id))
                } else {
                    // 默认点击当前位置
                    Ok(BrowserAction::ClickMouse {
                        coordinates: None,
                        button: MouseButton::Left,
                        click_count: 1,
                    })
                }
            }
            
            "type" | "type_text" | "computer_type_text" | "fill" => {
                let value = action.value.clone().unwrap_or_default();
                
                // 使用 fill_by_index 通过索引填充
                if let Some(index) = action.element_index {
                    return Ok(BrowserAction::FillByIndex { index, value });
                }
                
                if let Some(element_id) = &action.element_id {
                    // 尝试解析为纯数字索引
                    if let Ok(index) = element_id.parse::<u32>() {
                        return Ok(BrowserAction::FillByIndex { index, value });
                    }
                    // 不支持其他格式，返回错误
                    Err(anyhow!("type/fill requires numeric element_index, got: {}", element_id))
                } else {
                    Err(anyhow!("type/fill requires element_index"))
                }
            }
            
            "scroll" | "computer_scroll" => {
                let direction = action.value.as_deref()
                    .map(|v| match v.to_lowercase().as_str() {
                        "up" => ScrollDirection::Up,
                        "left" => ScrollDirection::Left,
                        "right" => ScrollDirection::Right,
                        _ => ScrollDirection::Down,
                    })
                    .unwrap_or(ScrollDirection::Down);
                
                Ok(BrowserAction::Scroll {
                    coordinates: None,
                    direction,
                    scroll_count: 3,
                })
            }
            
            "navigate" | "computer_navigate" => {
                let url = action.value.clone().unwrap_or(self.config.target_url.clone());
                Ok(BrowserAction::Navigate { url })
            }
            
            "wait" | "computer_wait" => {
                let duration_ms = action.value.as_ref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(500);
                Ok(BrowserAction::Wait { duration_ms })
            }
            
            "keys" | "type_keys" | "computer_type_keys" => {
                let keys = action.value.as_ref()
                    .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_else(|| vec!["Enter".to_string()]);
                Ok(BrowserAction::TypeKeys { keys })
            }
            
            "completed" | "done" | "set_exploration_status" | "set_status" => {
                Ok(BrowserAction::Screenshot)
            }
            
            "needs_help" => {
                Ok(BrowserAction::Screenshot)
            }
            
            _ => {
                warn!("Unknown action type: {}, defaulting to screenshot", action.action_type);
                Ok(BrowserAction::Screenshot)
            }
        }
    }
}

/// 从URL提取域名
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

/// 从URL提取路径
fn extract_path(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|u| u.path().to_string())
        .unwrap_or_else(|| "/".to_string())
}

/// 解析 JSON 格式的 headers
fn parse_headers_json(headers_json: Option<&str>) -> std::collections::HashMap<String, String> {
    headers_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}
