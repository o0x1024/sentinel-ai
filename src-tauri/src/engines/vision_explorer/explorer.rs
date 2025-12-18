//! 视觉探索引擎核心实现
//!
//! 实现VLM驱动的迭代式网站探索循环
//!
//! ## 新增功能
//! - 多模态VLM调用：支持截图图片输入
//! - 被动代理集成：实时获取发现的API
//! - Takeover模式：支持人工接管浏览器
//! - 上下文摘要：长任务时自动生成摘要避免token溢出

use super::browser_scripts;
use super::coverage_engine::CoverageEngine;
use super::element_manager::ElementManager;
use super::integrations::{
    ContextSummaryManager, PassiveProxyIntegration, ProxyRequestInfo, TakeoverManager,
};
use super::message_emitter::{
    VisionAction, VisionAnalysis, VisionCoverageUpdate, VisionExplorationStats,
    VisionExplorerMessageEmitter,
};
use super::route_tracker::RouteTracker;
use super::state::{ExplorationSummary, StateManager};
use super::tools::BrowserTools;
use super::types::*;
use crate::commands::passive_scan_commands::PassiveScanState;
use crate::engines::{LlmClient, LlmConfig};
use crate::models::prompt::TemplateType;
use crate::services::mcp::McpService;
use crate::services::prompt_db::PromptRepository;
use crate::utils::ordered_message::ArchitectureType;
use anyhow::{anyhow, Result};
use sentinel_llm::MessageImageAttachment as ImageAttachment;
use sentinel_passive::{PassiveDatabaseService, ProxyConfig};
use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

// Use refactored modules
use super::action_builder;
use super::element_formatter::{self, truncate_str};
use super::login_detector;
use super::vlm_parser;

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

/// Vision 探索阶段（用于分阶段计划与可重规划）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisionExplorePhase {
    /// Phase 0: 态势识别/准入判断（不要在此阶段做全站计划）
    Recon,
    /// Phase 1: 前台/未登录可访问区域
    Frontend,
    /// Phase 2: 登录流程（需要凭据/验证码/接管）
    Login,
    /// Phase 3: 登录后后台/控制台
    Backend,
}

impl VisionExplorePhase {
    fn as_str(&self) -> &'static str {
        match self {
            VisionExplorePhase::Recon => "recon",
            VisionExplorePhase::Frontend => "frontend",
            VisionExplorePhase::Login => "login",
            VisionExplorePhase::Backend => "backend",
        }
    }

    fn display_name_en(&self) -> &'static str {
        match self {
            VisionExplorePhase::Recon => "Reconnaissance",
            VisionExplorePhase::Frontend => "Frontend Exploration",
            VisionExplorePhase::Login => "Login Flow",
            VisionExplorePhase::Backend => "Backend Exploration",
        }
    }
}

#[derive(Debug, Clone)]
struct VisionExplorePlanState {
    phase: VisionExplorePhase,
    plan_markdown: String,
    last_plan_reason: String,
    last_progress_iteration: u32,
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
    // 新增：路由追踪器
    route_tracker: Arc<RwLock<RouteTracker>>,
    // 新增：元素管理器
    element_manager: Arc<RwLock<ElementManager>>,
    // 新增：覆盖率引擎
    coverage_engine: Arc<RwLock<CoverageEngine>>,
    // 是否已注入路由监听脚本
    route_monitor_injected: Arc<RwLock<bool>>,
    // 分阶段计划状态（可重规划）
    plan_state: Arc<RwLock<VisionExplorePlanState>>,
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

        let takeover_manager = Arc::new(RwLock::new(TakeoverManager::new(config.enable_takeover)));

        // If credentials are provided in config, pre-set them to TakeoverManager
        if let Some(ref creds) = config.credentials {
            let mut tm = takeover_manager.blocking_write();
            tm.set_user_credentials(
                creds.username.clone(),
                creds.password.clone(),
                creds.verification_code.clone(),
                creds.extra_fields.clone(),
            );
            info!(
                "Pre-set credentials from config for user: {}",
                creds.username
            );
        }

        // 创建Takeover事件通道
        let (tx, rx) = mpsc::unbounded_channel();

        // 初始化覆盖率组件
        let route_tracker = Arc::new(RwLock::new(RouteTracker::new(&config.target_url)));
        let element_manager = Arc::new(RwLock::new(ElementManager::new()));
        let coverage_engine = Arc::new(RwLock::new(CoverageEngine::new()));

        // 初始化分阶段计划状态
        let plan_state = Arc::new(RwLock::new(VisionExplorePlanState {
            phase: VisionExplorePhase::Recon,
            plan_markdown: String::new(),
            last_plan_reason: "init".to_string(),
            last_progress_iteration: 0,
        }));

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
            route_tracker,
            element_manager,
            coverage_engine,
            route_monitor_injected: Arc::new(RwLock::new(false)),
            plan_state,
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
        if let (Some(exec_id), Some(msg_id)) = (&self.config.execution_id, &self.config.message_id)
        {
            info!(
                "VisionExplorer: Initializing message emitter with exec_id={}, msg_id={}",
                exec_id, msg_id
            );
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
        self.passive_scan_state = Some(state.clone());
        // 同时更新 browser_tools 以便导航时动态获取代理配置
        self.browser_tools.set_passive_scan_state(state);
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
        let llm_config = LlmConfig::new(&provider, &model).with_timeout(120);
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

        info!(
            "Starting vision exploration for: {}",
            self.config.target_url
        );
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

        // 注册 TakeoverManager 到全局注册表，以便前端可以提交凭据
        if let Some(exec_id) = &self.config.execution_id {
            super::register_takeover_manager(exec_id.clone(), self.takeover_manager.clone()).await;
            info!("Registered TakeoverManager for execution_id: {}", exec_id);
        }

        // 执行探索循环
        let result = self.exploration_loop().await;

        // 从全局注册表注销 TakeoverManager
        if let Some(exec_id) = &self.config.execution_id {
            super::unregister_takeover_manager(exec_id).await;
            info!("Unregistered TakeoverManager for execution_id: {}", exec_id);
        }

        // Cleanup cancellation token for this execution (if any)
        if let Some(exec_id) = &self.config.execution_id {
            crate::managers::cancellation_manager::cleanup_token(exec_id).await;
        }

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

    /// 接收用户凭据（由外部调用，如 Tauri 命令）
    pub async fn receive_user_credentials(
        &self,
        username: String,
        password: String,
        verification_code: Option<String>,
        extra_fields: Option<std::collections::HashMap<String, String>>,
    ) {
        let mut manager = self.takeover_manager.write().await;
        manager.set_user_credentials(username.clone(), password, verification_code, extra_fields);

        // 发送凭据已接收通知到前端
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_credentials_received(&username);
        }

        info!("User credentials received for user: {}", username);
    }

    /// 恢复探索（用户提供凭据后调用）
    pub async fn resume_after_credentials(&self) {
        let mut manager = self.takeover_manager.write().await;

        // 归还控制权，继续探索
        manager.return_control();

        let mut state = self.state_manager.write().await;
        state.update_status(ExplorationStatus::Exploring);

        info!("Exploration resumed after receiving credentials");
    }

    /// 检查是否正在等待用户凭据
    pub async fn is_waiting_for_credentials(&self) -> bool {
        let manager = self.takeover_manager.read().await;
        manager.is_login_detected()
            && !manager.has_credentials()
            && matches!(manager.get_status(), TakeoverStatus::WaitingForUser)
    }

    /// 获取上下文摘要信息
    pub async fn get_context_info(&self) -> (u32, usize) {
        let context = self.context_manager.read().await;
        (
            context.get_estimated_tokens(),
            context.get_summaries().len(),
        )
    }

    /// 主探索循环
    async fn exploration_loop(&self) -> Result<()> {
        // 第0步：启动被动代理监听
        if let (Some(app), Some(state)) = (&self.app_handle, &self.passive_scan_state) {
            info!("Step 0: Starting passive proxy listener");

            // 从配置中获取代理端口
            let proxy_port = self
                .config
                .browser_proxy
                .as_ref()
                .and_then(|proxy_url| {
                    // 解析 http://127.0.0.1:8080 格式的 URL
                    proxy_url
                        .split(':')
                        .last()
                        .and_then(|p| p.parse::<u16>().ok())
                })
                .unwrap_or(8080);

            let proxy_config = ProxyConfig {
                start_port: proxy_port,
                max_port_attempts: 1,
                mitm_enabled: true,
                max_request_body_size: 2 * 1024 * 1024,
                max_response_body_size: 2 * 1024 * 1024,
                mitm_bypass_fail_threshold: 3,
                upstream_proxy: None,
            };

            match crate::commands::passive_scan_commands::start_passive_scan_internal(
                app,
                state.as_ref(),
                Some(proxy_config),
            )
            .await
            {
                Ok(port) => {
                    info!("Passive proxy started on port {}", port);
                }
                Err(e) => {
                    // 代理可能已经在运行
                    if e.contains("already running") {
                        info!("Passive proxy already running, continuing...");
                    } else {
                        warn!(
                            "Failed to start passive proxy: {}, continuing without proxy",
                            e
                        );
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

        // 初始化 Phase 0 计划（启动时只做态势识别，不做全站计划）
        {
            let phase = VisionExplorePhase::Recon;
            let plan_text = self.build_phase_plan_markdown(phase, "startup");
            let mut ps = self.plan_state.write().await;
            ps.phase = phase;
            ps.plan_markdown = plan_text.clone();
            ps.last_plan_reason = "startup".to_string();
            drop(ps);

            if let Some(emitter) = &self.message_emitter {
                let plan_info = self.get_phase_plan_info(phase, "startup").await;
                let steps_refs: Vec<&str> = plan_info.2.iter().map(|s| s.as_str()).collect();
                emitter.emit_plan(
                    phase.as_str(),
                    &plan_info.0,
                    &plan_info.1,
                    &steps_refs,
                    &plan_info.3,
                    "startup",
                );
            }
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

        // 注入路由监听脚本（用于 SPA 路由追踪）
        {
            let mut injected = self.route_monitor_injected.write().await;
            if !*injected {
                info!("Injecting route monitor script for SPA tracking");
                if let Err(e) = self
                    .browser_tools
                    .evaluate_js(browser_scripts::ROUTE_MONITOR_SCRIPT)
                    .await
                {
                    warn!("Failed to inject route monitor script: {}", e);
                } else {
                    *injected = true;
                }
            }
        }

        // 第2步：获取初始页面状态（根据多模态配置选择方式）
        info!(
            "Step 2: Capturing initial page state (multimodal={})",
            self.config.enable_multimodal
        );
        let page_state = if self.config.enable_multimodal {
            // 多模态模式：获取截图
            self.browser_tools.capture_page_state().await?
        } else {
            // 文本模式：获取标注元素列表，不截图
            self.browser_tools.capture_page_state_text_mode().await?
        };

        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state);
        }

        // 基于初始页面状态立即重规划（例如：一开始就是登录页/已登录后台）
        {
            let state = self.state_manager.read().await;
            let page_state = state.state().current_page.clone();
            drop(state);
            if let Some(ps) = page_state {
                self.replan_on_page_state(&ps, "initial_capture").await;
                self.emit_progress_update().await;
            }
        }

        // 第3步：迭代探索循环
        info!("Step 3: Starting exploration loop");
        let mut consecutive_errors = 0;
        let mut consecutive_screenshots: u32 = 0; // 跟踪连续截图次数（仅多模态）
        let mut consecutive_get_elements: u32 = 0; // 跟踪连续 get_elements 次数（仅文本模式）

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
                if matches!(
                    takeover.get_status(),
                    TakeoverStatus::WaitingForUser | TakeoverStatus::Active
                ) {
                    // Check for login timeout (40 seconds)
                    let is_timeout = takeover.is_login_timeout(40);
                    let has_credentials = takeover.has_credentials();
                    drop(takeover);

                    if is_timeout && !has_credentials {
                        // Auto-skip login due to timeout
                        info!("Login input timed out (40s), auto-skipping login");
                        let mut takeover = self.takeover_manager.write().await;
                        takeover.auto_skip_login();
                        takeover.return_control();
                        drop(takeover);

                        // Emit event to close takeover UI on frontend
                        if let Some(emitter) = &self.message_emitter {
                            emitter.emit_login_skipped(
                                "Login input timed out, continuing exploration",
                            );
                        }
                        // Don't continue, proceed to run_iteration which will handle navigation
                    } else {
                        // Still waiting for user input
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue;
                    }
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
            match self
                .run_iteration(consecutive_screenshots, consecutive_get_elements)
                .await
            {
                Ok((should_stop, was_screenshot, was_get_elements)) => {
                    consecutive_errors = 0;

                    // 更新连续截图计数
                    if self.config.enable_multimodal {
                        if was_screenshot {
                            consecutive_screenshots += 1;
                        } else {
                            consecutive_screenshots = 0;
                        }
                    } else {
                        consecutive_screenshots = 0;
                    }

                    // 更新连续 get_elements 计数（文本模式）
                    if !self.config.enable_multimodal {
                        if was_get_elements {
                            consecutive_get_elements += 1;
                        } else {
                            consecutive_get_elements = 0;
                        }
                    } else {
                        consecutive_get_elements = 0;
                    }

                    if should_stop {
                        info!("Exploration completed by VLM decision");
                        break;
                    }

                    // 每轮成功后发送一次进度更新（前端以 Progress 消息块展示）
                    self.emit_progress_update().await;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error!(
                        "Iteration failed (consecutive: {}): {}",
                        consecutive_errors, e
                    );

                    if consecutive_errors > 3 {
                        // 连续失败太多次
                        if self.config.enable_takeover {
                            // 请求用户接管
                            self.request_takeover(&format!("Multiple errors: {}", e))
                                .await;
                        } else {
                            let mut state = self.state_manager.write().await;
                            state.set_error(e.to_string());
                            break;
                        }
                    }
                }
            }
        }

        // 探索结束，关闭浏览器
        info!("Exploration loop ended, closing browser");
        if let Err(e) = self.browser_tools.close_browser().await {
            warn!("Failed to close browser at end of exploration: {}", e);
        }

        Ok(())
    }

    /// 执行单次迭代，返回 (是否停止, 是否是截图操作, 是否是 get_elements 操作)
    async fn run_iteration(
        &self,
        consecutive_screenshots: u32,
        consecutive_get_elements: u32,
    ) -> Result<(bool, bool, bool)> {
        let iteration = {
            let state = self.state_manager.read().await;
            state.state().iteration_count
        };

        debug!(
            "Running iteration {}, consecutive_screenshots: {}",
            iteration, consecutive_screenshots
        );

        // 1. 获取当前页面状态 (根据多模态配置选择不同方式)
        let page_state = if self.config.enable_multimodal {
            // 多模态模式：获取截图
            self.browser_tools.capture_page_state().await?
        } else {
            // 文本模式：获取标注元素列表，不截图
            self.browser_tools.capture_page_state_text_mode().await?
        };

        // 发送截图消息到前端 (仅多模态模式有截图)
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_screenshot(
                iteration,
                &page_state.url,
                &page_state.title,
                page_state.screenshot.as_deref(),
            );
        }

        // 2. 更新状态
        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state.clone());
        }

        // 2.0 根据当前页面状态进行阶段判断与重规划（登录门/登录成功等）
        self.replan_on_page_state(&page_state, "page_captured")
            .await;

        // 2.1. 检测登录页面并处理凭据/Takeover
        if let Some(login_fields) = login_detector::detect_login_page(&page_state) {
            let takeover = self.takeover_manager.read().await;
            let has_credentials = takeover.has_credentials();
            let is_login_detected = takeover.is_login_detected();
            let login_skipped = takeover.is_login_skipped();
            let login_timeout = takeover.is_login_timeout(40); // 30 seconds timeout
            drop(takeover); // 释放读锁

            if !has_credentials {
                // Check if login was skipped or timed out
                if login_skipped || login_timeout {
                    // Reduced retry budget to avoid wasting iterations
                    let skip_escape_attempts = {
                        let mut takeover = self.takeover_manager.write().await;
                        takeover.increment_login_retry()
                    };
                    // Reduced from 3/10 to 2/5 for faster termination
                    const MAX_SKIP_LOGIN_AUTO_NAV_ATTEMPTS: u32 = 2;
                    const MAX_SKIP_LOGIN_ESCAPE_ATTEMPTS: u32 = 5;

                    if login_timeout && !login_skipped {
                        // Auto-skip due to timeout
                        let mut takeover = self.takeover_manager.write().await;
                        takeover.auto_skip_login();
                        drop(takeover);
                        info!("Login input timed out, auto-skipping login");

                        // Notify frontend about auto-skip
                        if let Some(emitter) = &self.message_emitter {
                            emitter.emit_login_skipped(
                                "Login input timed out, continuing exploration",
                            );
                        }
                    }

                    info!(
                        "Login page detected at {}, login skipped; escape attempt {}/{}",
                        page_state.url, skip_escape_attempts, MAX_SKIP_LOGIN_ESCAPE_ATTEMPTS
                    );

                    // Hard stop early if we're clearly stuck
                    if skip_escape_attempts >= MAX_SKIP_LOGIN_ESCAPE_ATTEMPTS {
                        let reason = format!(
                            "Site requires authentication. Stopped after {} escape attempts.",
                            skip_escape_attempts
                        );
                        warn!("{}", reason);
                        let mut state = self.state_manager.write().await;
                        state.mark_completed(&reason);
                        return Ok((true, false, false));
                    }

                    // Try to escape login page by navigating to a non-login pending route
                    if skip_escape_attempts <= MAX_SKIP_LOGIN_AUTO_NAV_ATTEMPTS {
                        let next_route = {
                            let mut rt = self.route_tracker.write().await;
                            let mut candidate = None;
                            while let Some(r) = rt.next_pending() {
                                if !login_detector::is_login_like_route(&r) {
                                    candidate = Some(r);
                                    break;
                                }
                            }
                            candidate
                        };

                        if let Some(url) = next_route {
                            info!("Navigating to non-login pending route: {}", url);
                            let _ = self
                                .browser_tools
                                .execute_action(&BrowserAction::Navigate { url })
                                .await?;
                            return Ok((false, false, false));
                        }

                        // Fallback: try target_url if it's not a login url
                        if !login_detector::is_login_like_route(&self.config.target_url)
                            && self.config.target_url != page_state.url
                        {
                            info!("Navigating to target_url: {}", self.config.target_url);
                            let _ = self
                                .browser_tools
                                .execute_action(&BrowserAction::Navigate {
                                    url: self.config.target_url.clone(),
                                })
                                .await?;
                            return Ok((false, false, false));
                        }
                    }

                    // Let VLM try to find public access
                    info!("Letting VLM try to find public access from login page");
                } else {
                    // Case 1: No credentials, request user input
                    info!(
                        "Login page detected at {}, requesting credentials",
                        page_state.url
                    );

                    if self.config.enable_takeover {
                        let mut takeover = self.takeover_manager.write().await;
                        takeover.request_login_takeover(
                            "Login page detected, please provide credentials",
                            Some(login_fields.clone()),
                        );

                        if let Some(emitter) = &self.message_emitter {
                            emitter.emit_takeover_request(
                                iteration,
                                "login_required",
                                "Login page detected. Please enter credentials below or click 'Skip Login' to continue without authentication.",
                                Some(&login_fields),
                            );
                        }

                        return Ok((false, false, false)); // Pause exploration
                    } else {
                        warn!("Login page detected but takeover is disabled");
                    }
                }
            } else if has_credentials && is_login_detected {
                // Case 2: Has credentials but still on login page
                let retry_count = {
                    let mut takeover = self.takeover_manager.write().await;
                    takeover.increment_login_retry()
                };

                // Reduced from 5 to 3 iterations before triggering takeover
                if retry_count >= 3 && self.config.enable_takeover {
                    info!(
                        "Login page still detected after {} iterations, requesting user takeover",
                        retry_count
                    );

                    let mut takeover = self.takeover_manager.write().await;
                    takeover.clear_login_detected();
                    takeover.request_login_takeover(
                        "登录未完成，可能需要验证码/二次验证/额外步骤，请手动完成登录或补充信息",
                        Some(login_fields.clone()),
                    );

                    if let Some(emitter) = &self.message_emitter {
                        emitter.emit_takeover_request(
                            iteration,
                            "login_failed",
                            "登录未完成：可能需要验证码/二次验证/额外步骤。请手动完成登录或补充信息后继续。",
                            Some(&login_fields),
                        );
                    }

                    return Ok((false, false, false)); // Pause exploration, wait for user
                } else {
                    info!(
                        "Login page still detected during login flow (retry {}/3), continuing automated login",
                        retry_count
                    );
                }
            } else {
                // Case 3: Has credentials and login_detected is false (first time seeing login page)
                // Mark login_detected and let LLM use credentials to login
                info!(
                    "Login page detected at {}, credentials available, LLM will use them to login",
                    page_state.url
                );
                let mut takeover = self.takeover_manager.write().await;
                takeover.mark_login_detected();
            }
        } else {
            // Not a login page - if login was previously detected and we're now on a different page, login succeeded
            let takeover = self.takeover_manager.read().await;
            if takeover.is_login_detected() {
                drop(takeover);
                let mut takeover = self.takeover_manager.write().await;
                takeover.clear_login_detected();
                info!("Left login page, login appears successful");
            } else if takeover.is_login_skipped() && takeover.get_login_retry_count() > 0 {
                drop(takeover);
                let mut takeover = self.takeover_manager.write().await;
                takeover.reset_login_retry_count();
            }
        }

        // 2.5. 更新元素管理器和路由追踪器
        {
            // 更新元素管理器
            let mut em = self.element_manager.write().await;
            em.update_page_elements(&page_state.annotated_elements, &page_state.url);
        }

        // 从内部链接中提取路由
        {
            let internal_links: Vec<String> = page_state
                .links
                .iter()
                .filter_map(|el| el.attributes.get("href").cloned())
                .collect();

            let mut rt = self.route_tracker.write().await;
            let new_routes = rt.add_discovered_routes(&internal_links, &page_state.url);
            if new_routes > 0 {
                info!("Discovered {} new routes from page links", new_routes);
            }
            rt.mark_visited(&page_state.url);
        }

        // 检查 SPA 路由变化（通过注入的脚本）
        if let Ok(routes_result) = self
            .browser_tools
            .evaluate_js(browser_scripts::GET_ROUTES_SCRIPT)
            .await
        {
            if let Some(routes) = routes_result.as_array() {
                let route_strings: Vec<String> = routes
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                let mut rt = self.route_tracker.write().await;
                let new_spa_routes = rt.add_discovered_routes(&route_strings, "spa_history");
                if new_spa_routes > 0 {
                    info!(
                        "Discovered {} new SPA routes via History API",
                        new_spa_routes
                    );
                }
            }
        }

        // 3. 构建VLM提示词 (分离 system_prompt 和 user_prompt)
        let (system_prompt, user_prompt) = self.build_vlm_prompt(&page_state).await?;

        // 4. 调用VLM获取下一步操作 (支持多模态)
        let vlm_response = self
            .call_vlm_multimodal(
                &system_prompt,
                &user_prompt,
                page_state.screenshot.as_deref(),
            )
            .await?;

        // 5. 记录对话到上下文管理器
        if self.config.enable_context_summary {
            let mut context = self.context_manager.write().await;
            context.add_message(
                "user",
                &user_prompt,
                iteration,
                page_state.screenshot.is_some(),
            );
            context.add_message("assistant", &vlm_response, iteration, false);
        }

        // 6. 解析VLM响应（根据模式传入 loop 计数器：多模态用截图次数，文本模式用 get_elements 次数）
        let loop_counter = if self.config.enable_multimodal {
            consecutive_screenshots
        } else {
            consecutive_get_elements
        };
        let mut analysis =
            vlm_parser::parse_vlm_response(&vlm_response, loop_counter, self.config.enable_multimodal)?;

        // 发送分析结果到前端
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_analysis(
                iteration,
                VisionAnalysis {
                    page_analysis: analysis.page_analysis.clone(),
                    estimated_apis: if analysis.estimated_apis.is_empty() {
                        None
                    } else {
                        Some(analysis.estimated_apis.clone())
                    },
                    exploration_progress: analysis.exploration_progress,
                },
            );
        }

        // 7. 检查是否需要Takeover
        if analysis.next_action.action_type == "needs_help" {
            if self.config.enable_takeover {
                self.request_takeover(&analysis.next_action.reason).await;
                return Ok((false, false, false)); // 不停止，等待用户
            } else {
                // 没有启用 takeover，记录错误并继续
                warn!(
                    "VLM needs help but takeover is disabled: {}",
                    analysis.next_action.reason
                );
                let mut state = self.state_manager.write().await;
                state.set_error(analysis.next_action.reason.clone());
                return Ok((true, false, false)); // 停止探索
            }
        }

        // 8. 检查是否完成
        // 8a. 首先检查 VLM 是否认为完成
        let vlm_says_complete = analysis.is_exploration_complete;

        // 8b. 检查覆盖率引擎是否认为可以完成
        let coverage_allows_complete = {
            let pending_routes = {
                let rt = self.route_tracker.read().await;
                rt.pending_count()
            };
            let ce = self.coverage_engine.read().await;
            ce.is_completion_ready(pending_routes)
        };

        // 8c. 如果 VLM 说完成，验证覆盖率是否达标
        if vlm_says_complete {
            if coverage_allows_complete {
                let mut state = self.state_manager.write().await;
                state.mark_completed(
                    analysis
                        .completion_reason
                        .as_deref()
                        .unwrap_or("VLM decided exploration is complete, coverage verified"),
                );
                return Ok((true, false, false));
            } else {
                // VLM 说完成但覆盖率未达标，检查是否有待访问路由
                let pending_route = {
                    let mut rt = self.route_tracker.write().await;
                    rt.next_pending()
                };

                if let Some(route) = pending_route {
                    info!(
                        "VLM says complete but {} routes pending, navigating to next route: {}",
                        {
                            let rt = self.route_tracker.read().await;
                            rt.pending_count() + 1
                        },
                        route
                    );
                    // 不完成，继续探索
                } else {
                    // 没有待访问路由，检查稳定性
                    let ce = self.coverage_engine.read().await;
                    if ce.is_stable_complete() {
                        let mut state = self.state_manager.write().await;
                        state.mark_completed(
                            "Stable completion: no new discoveries for multiple rounds",
                        );
                        return Ok((true, false, false));
                    }
                }
            }
        }

        // 8d. 即使 VLM 没说完成，如果覆盖率系统检测到稳定完成也可以结束
        // 但需要更严格的条件：必须没有待访问路由，且稳定轮次足够多
        {
            let pending_routes = {
                let rt = self.route_tracker.read().await;
                rt.pending_count()
            };
            let ce = self.coverage_engine.read().await;
            // Only auto-complete if: stable, coverage ready, AND no pending routes
            if ce.is_stable_complete() && coverage_allows_complete && pending_routes == 0 {
                let mut state = self.state_manager.write().await;
                state.mark_completed("Coverage metrics indicate exploration complete");
                return Ok((true, false, false));
            }
        }

        // 9. Execute action with retry mechanism
        action_builder::guard_next_action(
            &mut analysis,
            &self.config,
            &self.element_manager,
            &self.route_tracker,
        )
        .await;
        let action = action_builder::build_action_from_analysis(&analysis, &self.config)?;
        let is_screenshot = matches!(action, BrowserAction::Screenshot);
        let is_get_elements = matches!(action, BrowserAction::GetAnnotatedElements);

        // 9.1 对于 index 类操作，使用快照验证防止漂移
        // 如果 page_state 有 snapshot_id，则使用快照验证
        let (result, action_duration) = if let Some(ref snapshot_id) = page_state.snapshot_id {
            if let Some(index) = analysis.next_action.element_index {
                match action {
                    BrowserAction::ClickByIndex { .. } => {
                        // 使用快照点击
                        info!(
                            "Using snapshot-based click for index {} (snapshot: {})",
                            index, snapshot_id
                        );
                        let start = std::time::Instant::now();
                        let result = self
                            .browser_tools
                            .click_by_snapshot(snapshot_id, index)
                            .await;
                        let duration = start.elapsed().as_millis() as u64;

                        match result {
                            Ok(r) => (r, duration),
                            Err(e) => {
                                // 快照点击失败，回退到普通点击
                                warn!("Snapshot click failed: {}, falling back to normal click", e);
                                self.execute_action_with_retry(&action, 2).await?
                            }
                        }
                    }
                    BrowserAction::FillByIndex {
                        index: _,
                        ref value,
                    } => {
                        // 使用快照填充
                        info!(
                            "Using snapshot-based fill for index {} (snapshot: {})",
                            index, snapshot_id
                        );
                        let start = std::time::Instant::now();
                        let result = self
                            .browser_tools
                            .fill_by_snapshot(snapshot_id, index, value)
                            .await;
                        let duration = start.elapsed().as_millis() as u64;

                        match result {
                            Ok(r) => (r, duration),
                            Err(e) => {
                                // 快照填充失败，回退到普通填充
                                warn!("Snapshot fill failed: {}, falling back to normal fill", e);
                                self.execute_action_with_retry(&action, 2).await?
                            }
                        }
                    }
                    _ => {
                        // 非 index 类操作，正常执行
                        self.execute_action_with_retry(&action, 2).await?
                    }
                }
            } else {
                // 没有 element_index，正常执行
                self.execute_action_with_retry(&action, 2).await?
            }
        } else {
            // 没有 snapshot_id，使用传统方式执行 (向后兼容)
            debug!("No snapshot_id available, using traditional action execution");
            self.execute_action_with_retry(&action, 2).await?
        };

        // Send action result to frontend
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_action(
                iteration,
                VisionAction {
                    action_type: analysis.next_action.action_type.clone(),
                    element_index: analysis.next_action.element_index,
                    value: analysis.next_action.value.clone(),
                    reason: analysis.next_action.reason.clone(),
                    success: result.success,
                    duration_ms: Some(action_duration),
                },
            );
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

        // 11. 更新元素管理器中的交互状态
        if let Some(index) = analysis.next_action.element_index {
            let mut em = self.element_manager.write().await;
            em.mark_interacted_by_index(index);
        }

        // 12. 更新覆盖率并发送到前端
        {
            let route_stats = {
                let rt = self.route_tracker.read().await;
                rt.stats()
            };

            let element_stats = {
                let em = self.element_manager.read().await;
                em.stats()
            };

            let api_count = {
                let state = self.state_manager.read().await;
                state.state().discovered_apis.len()
            };

            // 计算组件覆盖率（暂时为 100%）
            let component_coverage = {
                let em = self.element_manager.read().await;
                em.component_coverage_percentage()
            };

            // 更新覆盖率引擎
            {
                let mut ce = self.coverage_engine.write().await;
                ce.update(&route_stats, &element_stats, api_count, component_coverage);
            }

            // 发送覆盖率更新到前端
            if let Some(emitter) = &self.message_emitter {
                let ce = self.coverage_engine.read().await;
                let pending_routes = {
                    let rt = self.route_tracker.read().await;
                    rt.get_pending_routes()
                };

                let update = VisionCoverageUpdate {
                    route_coverage: route_stats.coverage,
                    element_coverage: element_stats.coverage,
                    component_coverage,
                    overall_coverage: ce.overall_coverage(),
                    api_count,
                    pending_routes,
                    stable_rounds: ce.consecutive_no_discovery,
                };

                emitter.emit_coverage_update(&update);
            }
        }

        Ok((false, is_screenshot, is_get_elements))
    }

    /// Execute action with retry mechanism
    async fn execute_action_with_retry(
        &self,
        action: &BrowserAction,
        max_retries: u32,
    ) -> Result<(ActionResult, u64)> {
        let should_retry = matches!(
            action,
            BrowserAction::ClickByIndex { .. }
                | BrowserAction::FillByIndex { .. }
                | BrowserAction::ClickMouse { .. }
        );

        let retries = if should_retry { max_retries } else { 1 };
        let mut last_error = None;
        let mut total_duration = 0u64;

        for attempt in 1..=retries {
            let action_start = std::time::Instant::now();

            match self.browser_tools.execute_action(action).await {
                Ok(result) => {
                    total_duration += action_start.elapsed().as_millis() as u64;

                    if result.success {
                        return Ok((result, total_duration));
                    }

                    // Action executed but failed (e.g., element not found)
                    if attempt < retries {
                        warn!(
                            "Action failed (attempt {}/{}): {:?}. Retrying after delay...",
                            attempt, retries, result.error
                        );
                        // Wait before retry (element might be loading)
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        last_error = result.error.clone();
                    } else {
                        return Ok((result, total_duration));
                    }
                }
                Err(e) => {
                    total_duration += action_start.elapsed().as_millis() as u64;

                    if attempt < retries {
                        warn!(
                            "Action error (attempt {}/{}): {}. Retrying...",
                            attempt, retries, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        last_error = Some(e.to_string());
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // Should not reach here, but return a failure result if it does
        Ok((
            ActionResult {
                success: false,
                error: last_error,
                screenshot: None,
                duration_ms: total_duration,
            },
            total_duration,
        ))
    }

    /// VLM/LLM call (decides whether to send image based on config)
    async fn call_vlm_multimodal(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        screenshot_base64: Option<&str>,
    ) -> Result<String> {
        let llm_client = LlmClient::new(self.llm_config.clone());

        // Decide call mode based on multimodal config
        let response = if self.config.enable_multimodal {
            // Multimodal mode: send screenshot
            let image = screenshot_base64.map(|s| ImageAttachment::new(s, "png"));
            info!(
                "VisionExplorer: Using multimodal mode, image={}",
                image.is_some()
            );
            llm_client
                .completion_with_image(Some(system_prompt), user_prompt, image.as_ref())
                .await?
        } else {
            // 文本模式：不发送任何图片，使用纯文本调用
            info!("VisionExplorer: Using text mode (no image)");
            llm_client
                .completion(Some(system_prompt), user_prompt)
                .await?
        };

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
        let new_requests: Vec<_> = requests
            .iter()
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
        let proxy_requests: Vec<ProxyRequestInfo> = new_requests
            .iter()
            .map(|r| ProxyRequestInfo {
                url: r.url.clone(),
                method: r.method.clone(),
                path: extract_path(&r.url),
                host: r.host.clone(),
                headers: parse_headers_json(r.request_headers.as_deref()),
                body: r.request_body.clone(),
                status_code: Some(r.status_code as u16),
            })
            .collect();

        info!(
            "Polled {} new proxy requests from passive scanner",
            proxy_requests.len()
        );

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
    /// 根据 enable_multimodal 配置选择多模态或文本模式的 prompt
    async fn get_system_prompt_template(&self) -> String {
        if let Some(repo) = &self.prompt_repo {
            // 根据模式选择对应的模板类型
            let template_type = if self.config.enable_multimodal {
                TemplateType::VisionExplorerVision
            } else {
                TemplateType::VisionExplorerText
            };

            match repo
                .get_active_template_by_type(template_type.clone())
                .await
            {
                Ok(Some(template)) => {
                    info!(
                        "Using database prompt template: {} (type: {:?})",
                        template.name, template_type
                    );
                    return template.content;
                }
                Ok(None) => {
                    warn!(
                        "No active {:?} template in database, using empty prompt",
                        template_type
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to get prompt template from database: {}, using empty prompt",
                        e
                    );
                }
            }
        } else {
            warn!("PromptRepository not initialized, using empty prompt");
        }
        // 回退到空模板
        "".to_string()
    }

    /// Build phase plan context (injected into VLM user_prompt)
    async fn build_plan_context(&self) -> String {
        let ps = self.plan_state.read().await.clone();
        if ps.plan_markdown.trim().is_empty() {
            return String::new();
        }

        format!(
            r#"## Current Phase: {} ({})

{}
"#,
            ps.phase.display_name_en(),
            ps.phase.as_str(),
            ps.plan_markdown
        )
    }

    /// Generate phase plan text (concise, actionable)
    fn build_phase_plan_markdown(&self, phase: VisionExplorePhase, reason: &str) -> String {
        match phase {
            VisionExplorePhase::Recon => format!(
                r#"**Goal**: Identify login gates, entry points, navigation structure (SPA/MPA), and API discovery channels.
**Steps**: Detect login signals → Find navigation entries → Minimal interaction to discover routes/APIs.
**Trigger**: {}"#,
                reason
            ),
            VisionExplorePhase::Frontend => format!(
                r#"**Goal**: Cover public pages and APIs (list/detail/search/export/register).
**Steps**: Walk core business flow → Trigger filters/pagination → Record login-required endpoints.
**Trigger**: {}"#,
                reason
            ),
            VisionExplorePhase::Login => format!(
                r#"**Goal**: Complete login or request user takeover for credentials/captcha.
**Steps**: Fill username/password/captcha → Click login → Verify success (URL change, logout button).
**Trigger**: {}"#,
                reason
            ),
            VisionExplorePhase::Backend => format!(
                r#"**Goal**: Cover backend modules and high-value APIs (users/permissions/config/audit).
**Steps**: Enumerate sidebar/menus → Trigger CRUD operations → Record request patterns.
**Trigger**: {}"#,
                reason
            ),
        }
    }

    /// 根据页面状态判断是否需要切换阶段并更新计划
    async fn replan_on_page_state(&self, page_state: &PageState, reason: &str) {
        let current = { self.plan_state.read().await.phase };

        // Multi-signal login detection
        let has_login_indicators = login_detector::has_logged_in_indicators(page_state);
        let is_login_page = login_detector::detect_login_page(page_state).is_some();
        let api_status_suggests_logged_in = self.check_api_status_for_login().await;

        debug!(
            "Phase detection - current: {:?}, has_login_indicators: {}, is_login_page: {}, api_logged_in: {}, url: {}", 
            current, has_login_indicators, is_login_page, api_status_suggests_logged_in, page_state.url
        );

        // Determine new phase with multiple signals
        let new_phase = if has_login_indicators || api_status_suggests_logged_in {
            // Strong signals of being logged in
            VisionExplorePhase::Backend
        } else if is_login_page {
            VisionExplorePhase::Login
        } else {
            match current {
                VisionExplorePhase::Recon => VisionExplorePhase::Frontend,
                VisionExplorePhase::Backend => VisionExplorePhase::Backend,
                // If in Login phase but not on login page and no login indicators,
                // likely skipped login or navigated away, switch to Frontend
                VisionExplorePhase::Login => VisionExplorePhase::Frontend,
                VisionExplorePhase::Frontend => VisionExplorePhase::Frontend,
            }
        };

        let mut ps = self.plan_state.write().await;
        let should_emit = ps.plan_markdown.trim().is_empty()
            || ps.phase != new_phase
            || ps.last_plan_reason != reason;

        if should_emit {
            ps.phase = new_phase;
            ps.last_plan_reason = reason.to_string();
            ps.plan_markdown = self.build_phase_plan_markdown(new_phase, reason);
            drop(ps);

            if let Some(emitter) = &self.message_emitter {
                let plan_info = self.get_phase_plan_info(new_phase, reason).await;
                let steps_refs: Vec<&str> = plan_info.2.iter().map(|s| s.as_str()).collect();
                emitter.emit_plan(
                    new_phase.as_str(),
                    &plan_info.0,
                    &plan_info.1,
                    &steps_refs,
                    &plan_info.3,
                    reason,
                );
            }
        }
    }

    /// Check API status codes to detect login state (401/403 -> 200 transition)
    async fn check_api_status_for_login(&self) -> bool {
        let state = self.state_manager.read().await;
        let apis = &state.state().discovered_apis;

        if apis.len() < 3 {
            return false;
        }

        // Check recent APIs for successful responses to protected endpoints
        let recent_apis: Vec<_> = apis.iter().rev().take(10).collect();

        // Look for patterns suggesting logged-in state
        let protected_paths = [
            "/api/",
            "/v1/",
            "/admin/",
            "/user/",
            "/dashboard/",
            "/manage/",
        ];
        let has_successful_protected_api = recent_apis.iter().any(|api| {
            let is_protected = protected_paths.iter().any(|p| api.path.contains(p));
            let is_success = api
                .status_code
                .map(|c| c >= 200 && c < 300)
                .unwrap_or(false);
            is_protected && is_success
        });

        // Check if we had 401/403 earlier but now have 200s
        let had_auth_errors = apis.iter().any(|api| {
            api.status_code
                .map(|c| c == 401 || c == 403)
                .unwrap_or(false)
        });

        has_successful_protected_api && had_auth_errors
    }

    /// Emit progress update (throttled: once per iteration)
    async fn emit_progress_update(&self) {
        let (iteration, visited, apis, interacted) = {
            let state = self.state_manager.read().await;
            (
                state.state().iteration_count,
                state.state().visited_pages.len(),
                state.state().discovered_apis.len(),
                state.state().interacted_elements.len(),
            )
        };

        let phase = { self.plan_state.read().await.phase };
        let max_iterations = self.config.max_iterations;

        let mut ps = self.plan_state.write().await;
        if ps.last_progress_iteration == iteration {
            return;
        }
        ps.last_progress_iteration = iteration;
        drop(ps);

        if let Some(emitter) = &self.message_emitter {
            emitter.emit_progress(
                iteration,
                max_iterations,
                phase.as_str(),
                visited,
                apis,
                interacted,
            );
        }
    }

    /// 返回阶段计划的结构化信息: (phase_name, goal, steps, completion_criteria)
    /// steps 格式: "status:text" 其中 status 可以是 done/skip/loading/pending
    async fn get_phase_plan_info(
        &self,
        phase: VisionExplorePhase,
        _reason: &str,
    ) -> (String, String, Vec<String>, String) {
        let state = self.state_manager.read().await;
        let iteration = state.state().iteration_count;
        let visited = state.state().visited_pages.len();
        let apis = state.state().discovered_apis.len();
        drop(state);

        match phase {
            VisionExplorePhase::Recon => {
                let step1_status = if visited > 0 { "done" } else { "loading" };
                let step2_status = if visited > 1 {
                    "done"
                } else if visited > 0 {
                    "loading"
                } else {
                    "pending"
                };
                let step3_status = if apis > 0 {
                    "done"
                } else if visited > 1 {
                    "loading"
                } else {
                    "pending"
                };

                (
                    "reconnaissance".to_string(),
                    "".to_string(),
                    vec![
                        format!("{}:enumerate page types", step1_status),
                        format!("{}:discover navigation entries", step2_status),
                        format!("{}:collect routing info", step3_status),
                    ],
                    "".to_string(),
                )
            }
            VisionExplorePhase::Frontend => {
                let step1_status = if visited > 2 { "done" } else { "loading" };
                let step2_status = if apis > 5 {
                    "done"
                } else if visited > 2 {
                    "loading"
                } else {
                    "pending"
                };
                let step3_status = "pending";

                (
                    "frontend exploration".to_string(),
                    "".to_string(),
                    vec![
                        format!("{}:enumerate public menus", step1_status),
                        format!("{}:trigger public apis", step2_status),
                        format!("{}:discover login entry", step3_status),
                    ],
                    "".to_string(),
                )
            }
            VisionExplorePhase::Login => {
                // Check if login was skipped
                let takeover = self.takeover_manager.read().await;
                let skip_login = takeover.is_login_skipped();
                drop(takeover);

                let (step1, step2, step3) = if skip_login {
                    (
                        "skip:fill login info",
                        "skip:click login",
                        "done:skip login",
                    )
                } else {
                    (
                        "done:fill login info",
                        "done:click login",
                        "loading:waiting for redirect",
                    )
                };

                (
                    "login flow".to_string(),
                    "".to_string(),
                    vec![step1.to_string(), step2.to_string(), step3.to_string()],
                    "".to_string(),
                )
            }
            VisionExplorePhase::Backend => {
                let step1_status = if iteration > 3 { "done" } else { "loading" };
                let step2_status = if apis > 20 { "loading" } else { "pending" };
                let step3_status = if apis > 30 { "loading" } else { "pending" };

                (
                    "后台探索".to_string(),
                    "".to_string(),
                    vec![
                        format!("{}:enumerate backend menus", step1_status),
                        format!("{}:cover sensitive modules", step2_status),
                        format!("{}:record api features", step3_status),
                    ],
                    "".to_string(),
                )
            }
        }
    }

    /// Build VLM prompt, returns (system_prompt, user_prompt)
    async fn build_vlm_prompt(&self, page_state: &PageState) -> Result<(String, String)> {
        let state = self.state_manager.read().await;

        // Stats
        let visited_count = state.state().visited_pages.len();
        let api_count = state.state().discovered_apis.len();
        let interacted_count = state.state().interacted_elements.len();

        // Format action history (last 3 only)
        let action_history = state.format_action_history(3);

        // Format discovered APIs (max 10, compact)
        let discovered_apis_list = {
            let apis: Vec<_> = state
                .state()
                .discovered_apis
                .iter()
                .take(10)
                .map(|api| format!("{} {}", api.method, api.path))
                .collect();
            if apis.is_empty() {
                "(none)".to_string()
            } else if api_count > 10 {
                format!("{} (+{} more)", apis.join(" | "), api_count - 10)
            } else {
                apis.join(" | ")
            }
        };

        drop(state);

        // Coverage context
        let coverage_context = self.build_coverage_context().await;

        // Context summary (only if enabled and available)
        let context_summary = if self.config.enable_context_summary {
            let context = self.context_manager.read().await;
            let summaries = context.get_summaries();
            if let Some(latest) = summaries.last() {
                format!("\n## Previous Summary\n{}", latest.content)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Elements section based on mode
        let elements_section = if !self.config.enable_multimodal {
            // Text mode: grouped element list is the primary way to understand the page
            let elements_grouped =
                element_formatter::format_elements_grouped(&page_state.annotated_elements, 100);
            format!(
                r#"## Page Elements ({} total)
{}
"#,
                page_state.annotated_elements.len(),
                elements_grouped
            )
        } else {
            // Multimodal mode: provide compact element list for index operations
            let elements_csv =
                element_formatter::format_elements_as_csv(&page_state.annotated_elements, 50);
            format!(
                r#"## Elements ({} total)
{}
"#,
                page_state.annotated_elements.len(),
                elements_csv
            )
        };

        // Page semantic summary (text mode only)
        let page_summary = if !self.config.enable_multimodal {
            if let Some(summary) = &page_state.visible_text_summary {
                format!("\n## Page Summary\n{}\n", summary)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // System prompt from database
        let system_template = self.get_system_prompt_template().await;
        let system_prompt = system_template
            .replace("{viewport_width}", &self.config.viewport_width.to_string())
            .replace(
                "{viewport_height}",
                &self.config.viewport_height.to_string(),
            );

        // Phase plan context
        let plan_context = self.build_plan_context().await;

        // User messages from takeover
        let user_messages_context = self.drain_user_messages_context().await;

        // Credentials context
        let credentials_context = self.build_credentials_context().await;

        // Valid index range hint for text mode
        let index_hint =
            if !self.config.enable_multimodal && !page_state.annotated_elements.is_empty() {
                let min_idx = page_state
                    .annotated_elements
                    .iter()
                    .map(|e| e.index)
                    .min()
                    .unwrap_or(0);
                let max_idx = page_state
                    .annotated_elements
                    .iter()
                    .map(|e| e.index)
                    .max()
                    .unwrap_or(0);
                format!("\nValid element indices: {} to {}", min_idx, max_idx)
            } else {
                String::new()
            };

        // Build user_prompt (English, compact)
        let user_prompt = format!(
            r#"# Exploration State
Target: {} | Pages: {} | APIs: {} | Interacted: {}

## Discovered APIs
{}

{}{}## Recent Actions (last 3)
{}
{}## Current Page
URL: {}
Title: {}
{}{}{}{}{}
Use element index for actions: click_by_index, fill_by_index, hover_by_index."#,
            self.config.target_url,
            visited_count,
            api_count,
            interacted_count,
            discovered_apis_list,
            coverage_context,
            plan_context,
            action_history,
            user_messages_context,
            page_state.url,
            page_state.title,
            page_summary,
            index_hint,
            elements_section,
            context_summary,
            credentials_context
        );

        Ok((system_prompt, user_prompt))
    }

    /// Drain queued user messages and build a prompt context block for VLM.
    async fn drain_user_messages_context(&self) -> String {
        let messages = {
            let mut takeover = self.takeover_manager.write().await;
            takeover.drain_user_messages()
        };

        if messages.is_empty() {
            return String::new();
        }

        let lines: Vec<_> = messages.iter().map(|m| format!("- {}", m)).collect();
        format!("## User Messages (priority)\n{}\n", lines.join("\n"))
    }

    /// Build coverage guidance context (for text mode, reduce blind spots)
    async fn build_coverage_context(&self) -> String {
        let route_stats = {
            let rt = self.route_tracker.read().await;
            rt.stats()
        };

        let pending_routes = {
            let rt = self.route_tracker.read().await;
            rt.get_pending_routes()
        };

        let (element_stats, mut uninteracted_indices, mut hover_candidate_indices) = {
            let em = self.element_manager.read().await;
            let stats = em.stats();
            let mut u = em.get_uninteracted_indices();
            let mut h = em.get_hover_candidate_indices();
            u.sort_unstable();
            h.sort_unstable();
            (stats, u, h)
        };

        let (stable_rounds, stability_threshold, overall_coverage) = {
            let ce = self.coverage_engine.read().await;
            (
                ce.consecutive_no_discovery,
                ce.stability_threshold,
                ce.overall_coverage(),
            )
        };

        // Pending routes (max 5)
        let pending_routes_display = if pending_routes.is_empty() {
            String::new()
        } else {
            let routes: Vec<_> = pending_routes.iter().take(5).map(|r| r.as_str()).collect();
            format!("\nPending routes: {}", routes.join(", "))
        };

        // Uninteracted indices (max 40)
        let uninteracted_display = if uninteracted_indices.is_empty() {
            String::new()
        } else {
            let indices: Vec<_> = uninteracted_indices
                .iter()
                .take(40)
                .map(|i| i.to_string())
                .collect();
            format!("\nUninteracted indices: {}", indices.join(","))
        };

        // Hover candidates (max 15)
        let hover_display = if hover_candidate_indices.is_empty() {
            String::new()
        } else {
            let indices: Vec<_> = hover_candidate_indices
                .iter()
                .take(15)
                .map(|i| i.to_string())
                .collect();
            format!("\nHover candidates: {}", indices.join(","))
        };

        format!(
            r#"## Coverage
Routes: {}/{} ({:.1}%) | Elements: {}/{} ({:.1}%) | Overall: {:.1}% | Stable: {}/{}{}{}{}
"#,
            route_stats.visited,
            route_stats.discovered,
            route_stats.coverage,
            element_stats.interacted,
            element_stats.total,
            element_stats.coverage,
            overall_coverage,
            stable_rounds,
            stability_threshold,
            pending_routes_display,
            uninteracted_display,
            hover_display
        )
    }

    /// Build credentials context (only when login detected and user provided credentials)
    async fn build_credentials_context(&self) -> String {
        let takeover = self.takeover_manager.read().await;

        if let Some(creds_info) = takeover.get_credentials_for_llm() {
            format!(
                "\n## Credentials (user provided)\n{}\nUse these to complete login.",
                creds_info
            )
        } else {
            String::new()
        }
    }

    // Login detection functions moved to login_detector module

    // Remaining functions moved to:
    // - element_formatter::format_elements_as_csv
    // - action_builder::guard_next_action
    // - action_builder::build_action_from_analysis
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
