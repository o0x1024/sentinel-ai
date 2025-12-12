//! 视觉探索引擎模块
//!
//! 使用VLM驱动的网站全流量发现引擎，模拟人类操作行为探索网站所有功能和API接口
//!
//! ## 核心架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    VisionExplorer                           │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
//! │  │BrowserTools │  │StateManager │  │  LlmClient  │         │
//! │  │(Playwright) │  │ (状态追踪)  │  │ (VLM调用)   │         │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
//! │         │                │                │                 │
//! │         └────────────────┼────────────────┘                 │
//! │                          │                                  │
//! │                  ┌───────▼───────┐                         │
//! │                  │ 探索循环      │                         │
//! │                  │ 截图→分析→   │                         │
//! │                  │ 操作→验证    │                         │
//! │                  └───────────────┘                         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sentinel_ai::engines::vision_explorer::{VisionExplorer, VisionExplorerConfig};
//!
//! let config = VisionExplorerConfig {
//!     target_url: "https://example.com".to_string(),
//!     max_iterations: 50,
//!     ..Default::default()
//! };
//!
//! let explorer = VisionExplorer::with_ai_config(
//!     config,
//!     mcp_service,
//!     "anthropic".to_string(),
//!     "claude-sonnet-4-20250514".to_string(),
//! );
//!
//! let summary = explorer.start().await?;
//! println!("Discovered {} APIs", summary.apis_discovered);
//! ```

pub mod types;
pub mod tools;
pub mod explorer;
pub mod state;
pub mod integrations;
pub mod tool;
pub mod message_emitter;
pub mod route_tracker;
pub mod element_manager;
pub mod coverage_engine;
pub mod browser_scripts;

// 导出核心类型
pub use types::{
    VisionExplorerConfig, ExplorationState, ExplorationStatus,
    PageState, PageElement, ActionRecord, ApiEndpoint,
    BrowserAction, ActionResult, VlmAnalysisResult,
    get_browser_tool_definitions,
    // 新增类型
    TakeoverStatus, TakeoverSession, UserAction, LoginCredentials,
    ContextSummary, ConversationMessage,
    // 表单相关
    FormInfo as VisionFormInfo, FormField,
};

// 导出探索引擎
pub use explorer::{VisionExplorer, TakeoverEvent};

// 导出工具
pub use tools::BrowserTools;

// 导出状态管理
pub use state::{StateManager, ExplorationSummary};

// 导出覆盖率引擎
pub use route_tracker::{RouteTracker, RouteStats};
pub use element_manager::{ElementManager, ElementStats, ElementFingerprint, DynamicComponent};
pub use coverage_engine::{CoverageEngine, CoverageReport, CompletionCheck, COVERAGE_TARGET, STABILITY_THRESHOLD};

// 导出集成模块
pub use integrations::{
    ContextSummaryManager, PassiveProxyIntegration, TakeoverManager,
    ProxyRequestInfo, ApiDiscoveryStats,
};

// 导出消息发送器
pub use message_emitter::{
    VisionExplorerMessageEmitter, VisionStep, VisionAnalysis,
    VisionAction, VisionExplorationStats, VisionCoverageUpdate,
};

pub use tool::VisionExplorerTool;

// ============================================================================
// Global Registry for Active Takeover Managers
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

/// Global registry for active TakeoverManagers (keyed by execution_id)
/// Allows credential submission to running explorers
pub static ACTIVE_TAKEOVER_MANAGERS: Lazy<RwLock<HashMap<String, Arc<RwLock<TakeoverManager>>>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a TakeoverManager in the global registry
pub async fn register_takeover_manager(execution_id: String, manager: Arc<RwLock<TakeoverManager>>) {
    let mut registry = ACTIVE_TAKEOVER_MANAGERS.write().await;
    registry.insert(execution_id.clone(), manager);
    tracing::info!("Registered TakeoverManager with execution_id: {}", execution_id);
}

/// Unregister a TakeoverManager from the global registry
pub async fn unregister_takeover_manager(execution_id: &str) {
    let mut registry = ACTIVE_TAKEOVER_MANAGERS.write().await;
    if registry.remove(execution_id).is_some() {
        tracing::info!("Unregistered TakeoverManager with execution_id: {}", execution_id);
    }
}

/// Get a TakeoverManager by execution_id
pub async fn get_takeover_manager(execution_id: &str) -> Option<Arc<RwLock<TakeoverManager>>> {
    let registry = ACTIVE_TAKEOVER_MANAGERS.read().await;
    registry.get(execution_id).cloned()
}

/// Submit credentials to a running VisionExplorer via its TakeoverManager
pub async fn submit_credentials(
    execution_id: &str,
    username: String,
    password: String,
    verification_code: Option<String>,
    extra_fields: Option<std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    let manager = get_takeover_manager(execution_id).await
        .ok_or_else(|| format!("No active explorer found with execution_id: {}", execution_id))?;
    
    let mut manager_guard = manager.write().await;
    manager_guard.set_user_credentials(username, password, verification_code, extra_fields);
    
    // 关键修复：归还控制权，让探索循环继续执行
    // 将 TakeoverStatus 从 WaitingForUser 更新为 Returned
    manager_guard.return_control();
    tracing::info!("Credentials submitted and control returned for execution_id: {}", execution_id);
    
    Ok(())
}
