//! Vision Explorer 集成模块
//!
//! 将VisionExplorer作为Travel架构的侦察增强能力
//!
//! ## 集成架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Travel Architecture                          │
//! │                                                                 │
//! │  ┌─────────────────────────────────────────────────────────┐  │
//! │  │                 OODA Loop                                │  │
//! │  │                                                          │  │
//! │  │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐ │  │
//! │  │  │ Observe │ → │ Orient  │ → │ Decide  │ → │   Act   │ │  │
//! │  │  └────┬────┘   └─────────┘   └─────────┘   └─────────┘ │  │
//! │  │       │                                                   │  │
//! │  │       │ ┌─────────────────────────────────────────────┐  │  │
//! │  │       └─│         VisionExplorer Integration          │  │  │
//! │  │         │                                             │  │  │
//! │  │         │  • VLM-driven website exploration           │  │  │
//! │  │         │  • API endpoint discovery                   │  │  │
//! │  │         │  • Form/input enumeration                   │  │  │
//! │  │         │  • Passive proxy integration                │  │  │
//! │  │         └─────────────────────────────────────────────┘  │  │
//! │  └──────────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use super::types::*;
use super::threat_intel::ThreatIntelManager;
use crate::engines::vision_explorer::{
    ApiEndpoint, BrowserAction, ExplorationSummary, ExplorationState, PageElement, PageState,
    VisionExplorer, VisionExplorerConfig, VisionFormInfo,
};
use crate::services::mcp::McpService;
use crate::utils::ordered_message::ArchitectureType;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

// ============================================================================
// Vision Explorer 集成服务
// ============================================================================

/// Vision Explorer 集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionIntegrationConfig {
    /// 是否启用Vision Explorer集成
    pub enabled: bool,
    /// 最大探索迭代次数
    pub max_iterations: u32,
    /// 探索超时(秒)
    pub timeout_secs: u64,
    /// 是否自动开始探索
    pub auto_start: bool,
    /// 是否将发现的API注入威胁情报
    pub inject_to_threat_intel: bool,
    /// 是否在Observe阶段自动调用
    pub auto_observe: bool,
    /// 视口宽度
    pub viewport_width: u32,
    /// 视口高度
    pub viewport_height: u32,
    /// 执行 ID (用于前端消息)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    /// 消息 ID (用于前端消息)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// 会话 ID (用于前端消息)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

impl Default for VisionIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_iterations: 50,
            timeout_secs: 300,
            auto_start: false,
            inject_to_threat_intel: true,
            auto_observe: true,
            viewport_width: 1920,
            viewport_height: 1080,
            execution_id: None,
            message_id: None,
            conversation_id: None,
        }
    }
}

/// Vision Explorer 集成服务
pub struct VisionIntegration {
    config: VisionIntegrationConfig,
    mcp_service: Option<Arc<McpService>>,
    llm_provider: String,
    llm_model: String,
    /// 缓存的探索结果
    cached_results: Arc<RwLock<HashMap<String, CachedExplorationResult>>>,
    /// Tauri AppHandle (用于启动代理)
    app_handle: Option<tauri::AppHandle>,
    /// 被动扫描状态 (用于启动代理)
    passive_scan_state: Option<Arc<crate::commands::passive_scan_commands::PassiveScanState>>,
    /// 被动扫描数据库服务 (用于获取代理请求)
    passive_db: Option<Arc<sentinel_passive::PassiveDatabaseService>>,
    /// 动态消息参数 (使用 RwLock 支持运行时更新)
    message_info: Arc<RwLock<Option<MessageInfo>>>,
    /// 取消令牌 (使用 RwLock 支持运行时更新)
    cancellation_token: Arc<RwLock<Option<CancellationToken>>>,
}

/// 消息参数
#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub execution_id: String,
    pub message_id: String,
    pub conversation_id: Option<String>,
}

/// 缓存的探索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedExplorationResult {
    pub target_url: String,
    pub summary: ExplorationSummary,
    pub discovered_apis: Vec<ApiEndpoint>,
    pub discovered_forms: Vec<FormInfo>,
    pub cached_at: std::time::SystemTime,
}

/// 表单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub url: String,
    pub action: Option<String>,
    pub method: String,
    pub inputs: Vec<InputInfo>,
}

/// 输入字段信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInfo {
    pub name: Option<String>,
    pub input_type: String,
    pub placeholder: Option<String>,
    pub required: bool,
}

/// 侦察增强结果 - 用于OODA Observe阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconEnhancementResult {
    /// 发现的API端点
    pub api_endpoints: Vec<ApiEndpointInfo>,
    /// 发现的表单
    pub forms: Vec<FormInfo>,
    /// 页面结构摘要
    pub page_summary: String,
    /// 潜在攻击面
    pub attack_surface: AttackSurface,
    /// 探索覆盖率
    pub coverage: f32,
}

/// API端点信息 (Travel格式)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpointInfo {
    pub url: String,
    pub method: String,
    pub path: String,
    pub parameters: Vec<String>,
    pub authentication_required: bool,
    pub discovered_via: String,
}

/// 攻击面分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackSurface {
    /// 输入点数量
    pub input_points: u32,
    /// 认证端点
    pub auth_endpoints: Vec<String>,
    /// 文件上传点
    pub file_upload_points: Vec<String>,
    /// API端点
    pub api_endpoints: Vec<String>,
    /// 管理接口
    pub admin_interfaces: Vec<String>,
    /// 风险评分 (0-100)
    pub risk_score: u32,
}

impl VisionIntegration {
    /// 创建新的集成服务
    pub fn new(
        config: VisionIntegrationConfig,
        mcp_service: Option<Arc<McpService>>,
        llm_provider: String,
        llm_model: String,
    ) -> Self {
        Self {
            config,
            mcp_service,
            llm_provider,
            llm_model,
            cached_results: Arc::new(RwLock::new(HashMap::new())),
            app_handle: None,
            passive_scan_state: None,
            passive_db: None,
            message_info: Arc::new(RwLock::new(None)),
            cancellation_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置 Tauri AppHandle
    pub fn with_app_handle(mut self, app: tauri::AppHandle) -> Self {
        self.app_handle = Some(app);
        self
    }

    /// 设置被动扫描状态
    pub fn with_passive_scan_state(mut self, state: Arc<crate::commands::passive_scan_commands::PassiveScanState>) -> Self {
        self.passive_scan_state = Some(state);
        self
    }

    /// 设置被动扫描数据库服务
    pub fn with_passive_db(mut self, db: Arc<sentinel_passive::PassiveDatabaseService>) -> Self {
        self.passive_db = Some(db);
        self
    }

    /// 设置消息参数 (用于前端显示) - 支持在 Arc 内调用
    pub async fn set_message_info(&self, execution_id: &str, message_id: &str, conversation_id: Option<&str>) {
        let mut info = self.message_info.write().await;
        *info = Some(MessageInfo {
            execution_id: execution_id.to_string(),
            message_id: message_id.to_string(),
            conversation_id: conversation_id.map(|s| s.to_string()),
        });
    }

    /// 设置取消令牌 (用于响应外部停止请求) - 支持在 Arc 内调用
    pub async fn set_cancellation_token(&self, token: CancellationToken) {
        let mut ct = self.cancellation_token.write().await;
        *ct = Some(token);
    }

    /// 检查 Playwright MCP 服务器是否可用
    pub async fn is_playwright_available(&self) -> bool {
        if !self.config.enabled {
            debug!("VisionIntegration: disabled by config");
            return false;
        }
        
        let mcp_service = match &self.mcp_service {
            Some(svc) => svc,
            None => {
                debug!("VisionIntegration: no MCP service");
                return false;
            }
        };
        
        // 检查是否有 playwright 连接（大小写不敏感，与 tools.rs 保持一致）
        match mcp_service.get_connection_info().await {
            Ok(connections) => {
                let playwright_conn = connections.iter().find(|c| 
                    c.name.to_lowercase().contains("playwright") && c.status == "connected"
                );
                
                if let Some(conn) = playwright_conn {
                    info!("VisionIntegration: Playwright MCP available: {}", conn.name);
                    true
                } else {
                    debug!("VisionIntegration: No Playwright MCP connection found. Available connections: {:?}", 
                        connections.iter().map(|c| format!("{}({})", c.name, c.status)).collect::<Vec<_>>());
                    false
                }
            }
            Err(e) => {
                debug!("VisionIntegration: Failed to get MCP connections: {}", e);
                false
            }
        }
    }

    /// 执行网站探索 (作为Travel工具调用)
    pub async fn explore_website(&self, target_url: &str) -> Result<ExplorationSummary> {
        if !self.config.enabled {
            return Err(anyhow!("Vision Explorer integration is disabled"));
        }

        info!("Starting Vision Explorer for target: {}", target_url);

        // 检查缓存
        if let Some(cached) = self.get_cached_result(target_url).await {
            info!("Using cached exploration result for {}", target_url);
            return Ok(cached.summary);
        }

        // 创建VisionExplorer实例
        let mcp_service = self
            .mcp_service
            .clone()
            .ok_or_else(|| anyhow!("MCP service not available"))?;

        // 获取动态消息参数
        let msg_info = self.message_info.read().await.clone();
        let (exec_id, msg_id, conv_id) = if let Some(info) = msg_info {
            info!("VisionIntegration: Using dynamic message info: exec={}, msg={}", info.execution_id, info.message_id);
            (Some(info.execution_id), Some(info.message_id), info.conversation_id)
        } else {
            warn!("VisionIntegration: No dynamic message info, using config (exec={:?}, msg={:?})", 
                self.config.execution_id, self.config.message_id);
            (self.config.execution_id.clone(), self.config.message_id.clone(), self.config.conversation_id.clone())
        };

        let explorer_config = VisionExplorerConfig {
            target_url: target_url.to_string(),
            max_iterations: self.config.max_iterations,
            viewport_width: self.config.viewport_width,
            viewport_height: self.config.viewport_height,
            // 传递消息相关配置以支持前端显示
            execution_id: exec_id,
            message_id: msg_id,
            conversation_id: conv_id,
            // 作为 Travel 的子流，不终结整个消息流
            finalize_on_complete: false,
            ..Default::default()
        };

        let mut explorer = VisionExplorer::with_ai_config(
            explorer_config,
            mcp_service,
            self.llm_provider.clone(),
            self.llm_model.clone(),
        );

        // 传入代理启动依赖
        if let Some(app) = &self.app_handle {
            explorer = explorer.with_app_handle(app.clone());
        }
        if let Some(state) = &self.passive_scan_state {
            explorer = explorer.with_passive_scan_state(state.clone());
        }
        // 传入被动扫描数据库服务（用于获取代理捕获的流量）
        if let Some(db) = &self.passive_db {
            explorer = explorer.with_passive_db(db.clone());
        }
        // 传入取消令牌（用于响应外部停止请求）
        if let Some(token) = self.cancellation_token.read().await.clone() {
            explorer = explorer.with_cancellation_token(token);
        }
        
        // 设置父架构类型，使 Vision Explorer 的消息与 Travel 消息流保持顺序一致
        explorer = explorer.with_parent_architecture(ArchitectureType::Travel);

        // 执行探索
        let summary = explorer.start().await?;
        
        // 获取完整状态（包含discovered_apis）
        let state = explorer.get_state().await;

        // 缓存结果（包含完整的API列表）
        self.cache_result_with_state(target_url, &summary, &state).await;

        info!(
            "Vision exploration completed: {} APIs, {} actions",
            summary.apis_discovered, summary.total_iterations
        );

        Ok(summary)
    }

    /// OODA Observe阶段增强 - 获取侦察增强数据
    pub async fn enhance_observe_phase(&self, target_url: &str) -> Result<ReconEnhancementResult> {
        info!("Enhancing Observe phase for: {}", target_url);

        // 执行探索
        let summary = self.explore_website(target_url).await?;

        // 获取缓存的详细数据
        let cached = self
            .get_cached_result(target_url)
            .await
            .ok_or_else(|| anyhow!("No cached result found"))?;

        // 转换为Travel格式，过滤掉静态资源文件
        let api_endpoints: Vec<ApiEndpointInfo> = cached
            .discovered_apis
            .iter()
            .filter(|api| !self.is_static_resource(&api.path))
            .map(|api| ApiEndpointInfo {
                url: api.full_url.clone(),
                method: api.method.clone(),
                path: api.path.clone(),
                parameters: api.parameters.keys().cloned().collect(),
                authentication_required: self.detect_auth_required(&api.path),
                discovered_via: "vision_explorer".to_string(),
            })
            .collect();

        // 分析攻击面
        let attack_surface = self.analyze_attack_surface(&cached);

        // 生成页面摘要
        let page_summary = format!(
            "Explored {} pages, discovered {} API endpoints and {} forms. \
             Coverage: {}%. Risk score: {}",
            summary.pages_visited,
            summary.apis_discovered,
            cached.discovered_forms.len(),
            summary.exploration_progress as u32,
            attack_surface.risk_score
        );

        Ok(ReconEnhancementResult {
            api_endpoints,
            forms: cached.discovered_forms,
            page_summary,
            attack_surface,
            coverage: summary.exploration_progress / 100.0, // 转换为0-1范围
        })
    }

    /// 将发现的API注入威胁情报系统
    pub async fn inject_to_threat_intel(
        &self,
        threat_intel: &ThreatIntelManager,
        target_url: &str,
    ) -> Result<Vec<ThreatInfo>> {
        if !self.config.inject_to_threat_intel {
            return Ok(Vec::new());
        }

        let cached = self
            .get_cached_result(target_url)
            .await
            .ok_or_else(|| anyhow!("No exploration result to inject"))?;

        let mut threats = Vec::new();

        // 为发现的敏感端点生成威胁信息
        for api in &cached.discovered_apis {
            if let Some(threat) = self.assess_api_threat(api) {
                threats.push(threat);
            }
        }

        info!(
            "Injected {} threat entries from Vision Explorer",
            threats.len()
        );
        Ok(threats)
    }

    /// 作为Travel工具定义
    pub fn as_tool_definition() -> serde_json::Value {
        serde_json::json!({
            "name": "vision_explore",
            "description": "Use VLM-driven exploration to discover API endpoints and attack surface of a target website. Simulates human interaction (clicking, typing, navigating) to discover all accessible functionality.",
            "parameters": {
                "type": "object",
                "properties": {
                    "target_url": {
                        "type": "string",
                        "description": "The target URL to explore"
                    },
                    "max_iterations": {
                        "type": "integer",
                        "description": "Maximum exploration iterations (default: 50)"
                    },
                    "focus_areas": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Specific areas to focus on (e.g., 'authentication', 'api', 'admin')"
                    }
                },
                "required": ["target_url"]
            }
        })
    }

    /// 执行工具调用
    pub async fn execute_tool_call(
        &self,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let target_url = args
            .get("target_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("target_url is required"))?;

        let summary = self.explore_website(target_url).await?;

        // 获取缓存的完整数据
        let cached = self.get_cached_result(target_url).await;

        Ok(serde_json::json!({
            "status": "completed",
            "target_url": target_url,
            "pages_visited": summary.pages_visited,
            "apis_discovered": summary.apis_discovered,
            "total_iterations": summary.total_iterations,
            "exploration_progress": summary.exploration_progress,
            "discovered_apis": cached.as_ref().map(|c| &c.discovered_apis),
            "discovered_forms": cached.as_ref().map(|c| &c.discovered_forms),
        }))
    }

    // ========== 私有辅助方法 ==========

    /// 获取缓存结果
    async fn get_cached_result(&self, target_url: &str) -> Option<CachedExplorationResult> {
        let cache = self.cached_results.read().await;
        cache.get(target_url).cloned()
    }

    /// 缓存探索结果（使用完整状态）
    async fn cache_result_with_state(&self, target_url: &str, summary: &ExplorationSummary, state: &ExplorationState) {
        let mut cache = self.cached_results.write().await;

        // 转换表单信息
        let discovered_forms: Vec<FormInfo> = state.discovered_forms.iter().map(|f| {
            FormInfo {
                url: target_url.to_string(),
                action: f.action.clone(),
                method: f.method.clone().unwrap_or_else(|| "POST".to_string()),
                inputs: f.fields.iter().map(|field| InputInfo {
                    name: Some(field.name.clone()),
                    input_type: field.field_type.clone(),
                    placeholder: field.placeholder.clone(),
                    required: field.required,
                }).collect(),
            }
        }).collect();

        let result = CachedExplorationResult {
            target_url: target_url.to_string(),
            summary: summary.clone(),
            discovered_apis: state.discovered_apis.clone(),
            discovered_forms,
            cached_at: std::time::SystemTime::now(),
        };

        cache.insert(target_url.to_string(), result);
    }

    /// 检测端点是否需要认证
    fn detect_auth_required(&self, path: &str) -> bool {
        let auth_keywords = [
            "login",
            "auth",
            "signin",
            "admin",
            "dashboard",
            "account",
            "profile",
            "settings",
        ];
        let path_lower = path.to_lowercase();
        auth_keywords.iter().any(|kw| path_lower.contains(kw))
    }

    /// 判断路径是否为静态资源文件（应过滤掉）
    fn is_static_resource(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        
        // 静态资源文件扩展名
        let static_extensions = [
            ".ico", ".gif", ".png", ".jpg", ".jpeg", ".webp", ".svg", ".bmp",  // 图片
            ".css", ".less", ".scss", ".sass",                                  // 样式
            ".js", ".mjs", ".map",                                              // 前端脚本和 sourcemap
            ".woff", ".woff2", ".ttf", ".eot", ".otf",                         // 字体
            ".mp3", ".mp4", ".wav", ".ogg", ".webm", ".avi",                   // 媒体
            ".pdf", ".doc", ".docx", ".xls", ".xlsx",                          // 文档
            ".zip", ".tar", ".gz", ".rar",                                      // 压缩包
        ];
        
        // 静态资源路径模式
        // let static_patterns = [
        //     "/static/", "/assets/", "/images/", "/img/", "/css/", "/js/",
        //     "/fonts/", "/media/", "/public/", "/dist/", "/build/",
        // ];
        let static_patterns = [
             "/images/", "/img/", "/css/", "/js/","/fonts/", 
        ];
        
        // 检查扩展名
        if static_extensions.iter().any(|ext| path_lower.ends_with(ext)) {
            return true;
        }
        
        // 检查路径模式
        if static_patterns.iter().any(|pattern| path_lower.contains(pattern)) {
            return true;
        }
        
        false
    }

    /// 分析攻击面
    fn analyze_attack_surface(&self, cached: &CachedExplorationResult) -> AttackSurface {
        let mut auth_endpoints = Vec::new();
        let mut file_upload_points = Vec::new();
        let mut api_endpoints = Vec::new();
        let mut admin_interfaces = Vec::new();

        for api in &cached.discovered_apis {
            // 跳过静态资源
            if self.is_static_resource(&api.path) {
                continue;
            }
            
            let path_lower = api.path.to_lowercase();

            // 分类端点
            if path_lower.contains("login")
                || path_lower.contains("auth")
                || path_lower.contains("signin")
            {
                auth_endpoints.push(api.full_url.clone());
            }

            if path_lower.contains("upload") || path_lower.contains("file") {
                file_upload_points.push(api.full_url.clone());
            }

            if path_lower.contains("/api/") || path_lower.starts_with("/v") {
                api_endpoints.push(api.full_url.clone());
            }

            if path_lower.contains("admin") || path_lower.contains("manage") {
                admin_interfaces.push(api.full_url.clone());
            }
        }

        // 计算输入点
        let input_points = cached
            .discovered_forms
            .iter()
            .map(|f| f.inputs.len())
            .sum::<usize>() as u32;

        // 计算风险评分
        let risk_score = self.calculate_risk_score(
            &auth_endpoints,
            &file_upload_points,
            &admin_interfaces,
            input_points,
        );

        AttackSurface {
            input_points,
            auth_endpoints,
            file_upload_points,
            api_endpoints,
            admin_interfaces,
            risk_score,
        }
    }

    /// 计算风险评分
    fn calculate_risk_score(
        &self,
        auth_endpoints: &[String],
        file_upload_points: &[String],
        admin_interfaces: &[String],
        input_points: u32,
    ) -> u32 {
        let mut score = 0u32;

        // 基础分数
        score += (input_points * 2).min(30);

        // 认证端点 (+10 per endpoint, max 30)
        score += (auth_endpoints.len() as u32 * 10).min(30);

        // 文件上传 (+15 per point, max 30)
        score += (file_upload_points.len() as u32 * 15).min(30);

        // 管理接口 (+20 per interface, max 40)
        score += (admin_interfaces.len() as u32 * 20).min(40);

        score.min(100)
    }

    /// 评估API威胁
    fn assess_api_threat(&self, api: &ApiEndpoint) -> Option<ThreatInfo> {
        let path_lower = api.path.to_lowercase();

        // 高风险端点
        let high_risk = path_lower.contains("admin")
            || path_lower.contains("delete")
            || path_lower.contains("exec")
            || path_lower.contains("system");

        // 中风险端点
        let medium_risk = path_lower.contains("upload")
            || path_lower.contains("password")
            || path_lower.contains("token")
            || path_lower.contains("config");

        if high_risk {
            Some(ThreatInfo {
                id: format!("ve-{}", uuid::Uuid::new_v4()),
                name: format!("High-risk endpoint: {}", api.path),
                description: format!(
                    "Discovered high-risk API endpoint {} {} via Vision Explorer",
                    api.method, api.path
                ),
                level: ThreatLevel::High,
                cves: Vec::new(),
                source: ThreatSource::LLMAnalysis,
            })
        } else if medium_risk {
            Some(ThreatInfo {
                id: format!("ve-{}", uuid::Uuid::new_v4()),
                name: format!("Sensitive endpoint: {}", api.path),
                description: format!(
                    "Discovered sensitive API endpoint {} {} via Vision Explorer",
                    api.method, api.path
                ),
                level: ThreatLevel::Medium,
                cves: Vec::new(),
                source: ThreatSource::LLMAnalysis,
            })
        } else {
            None
        }
    }
}

// ============================================================================
// Travel工具适配器
// ============================================================================

/// Vision Explorer 工具适配器
/// 
/// 将VisionExplorer作为Travel可调用的工具
pub struct VisionExplorerToolAdapter {
    integration: Arc<VisionIntegration>,
}

impl VisionExplorerToolAdapter {
    pub fn new(integration: Arc<VisionIntegration>) -> Self {
        Self { integration }
    }

    /// 获取工具定义
    pub fn get_tool_definition(&self) -> serde_json::Value {
        VisionIntegration::as_tool_definition()
    }

    /// 执行工具调用
    pub async fn execute(
        &self,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.integration.execute_tool_call(args).await
    }
}

// ============================================================================
// OODA阶段增强器
// ============================================================================

/// OODA Observe 阶段增强器
pub struct ObservePhaseEnhancer {
    vision_integration: Arc<VisionIntegration>,
}

impl ObservePhaseEnhancer {
    pub fn new(vision_integration: Arc<VisionIntegration>) -> Self {
        Self { vision_integration }
    }

    /// 增强Observe阶段的观察数据
    pub async fn enhance(
        &self,
        target_url: &str,
        existing_observations: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        info!("Enhancing Observe phase with Vision Explorer");

        // 执行Vision Explorer侦察
        let recon_result = self
            .vision_integration
            .enhance_observe_phase(target_url)
            .await?;

        // 注入观察数据
        existing_observations.insert(
            "vision_explorer_apis".to_string(),
            serde_json::to_value(&recon_result.api_endpoints)?,
        );

        existing_observations.insert(
            "vision_explorer_forms".to_string(),
            serde_json::to_value(&recon_result.forms)?,
        );

        existing_observations.insert(
            "attack_surface".to_string(),
            serde_json::to_value(&recon_result.attack_surface)?,
        );

        existing_observations.insert(
            "page_summary".to_string(),
            serde_json::Value::String(recon_result.page_summary),
        );

        existing_observations.insert(
            "exploration_coverage".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(recon_result.coverage as f64).unwrap(),
            ),
        );

        info!(
            "Observe phase enhanced with {} APIs and {} forms",
            recon_result.api_endpoints.len(),
            recon_result.forms.len()
        );

        Ok(())
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = VisionIntegrationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_detect_auth_required() {
        let config = VisionIntegrationConfig::default();
        let integration = VisionIntegration::new(
            config,
            None,
            "anthropic".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        );

        assert!(integration.detect_auth_required("/api/login"));
        assert!(integration.detect_auth_required("/admin/dashboard"));
        assert!(!integration.detect_auth_required("/api/products"));
    }

    #[test]
    fn test_calculate_risk_score() {
        let config = VisionIntegrationConfig::default();
        let integration = VisionIntegration::new(
            config,
            None,
            "anthropic".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        );

        let score = integration.calculate_risk_score(
            &["/login".to_string()],
            &["/upload".to_string()],
            &["/admin".to_string()],
            5,
        );

        assert!(score > 0);
        assert!(score <= 100);
    }

    #[test]
    fn test_tool_definition() {
        let def = VisionIntegration::as_tool_definition();
        assert_eq!(def["name"], "vision_explore");
        assert!(def["parameters"]["properties"]["target_url"].is_object());
    }
}
