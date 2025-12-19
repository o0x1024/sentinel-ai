//! 视觉探索引擎模块
//!
//! 使用VLM驱动的网站全流量发现引擎，模拟人类操作行为探索网站所有功能和API接口
//!
//! ## 核心架构
//!
//! ### 单 Agent 模式 (VisionExplorer)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    VisionExplorer                           │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
//! │  │BrowserTools │  │StateManager │  │  LlmClient  │         │
//! │  │(Playwright) │  │ (状态追踪)  │  │ (VLM调用)   │         │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
//! │         └────────────────┼────────────────┘                 │
//! │                  ┌───────▼───────┐                         │
//! │                  │ 探索循环      │                         │
//! │                  └───────────────┘                         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### 多 Agent 模式 (MultiAgentExplorer)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  MultiAgentExplorer                         │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              ManagerAgent                            │   │
//! │  │  - Analyze homepage navigation                       │   │
//! │  │  - Divide into exploration scopes                    │   │
//! │  │  - Assign tasks to workers                           │   │
//! │  └──────────────────────┬──────────────────────────────┘   │
//! │                         │                                   │
//! │         ┌───────────────┼───────────────┐                  │
//! │         ▼               ▼               ▼                  │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
//! │  │WorkerAgent │  │WorkerAgent │  │WorkerAgent │           │
//! │  │ Scope: /a  │  │ Scope: /b  │  │ Scope: /c  │           │
//! │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘           │
//! │        └───────────────┼───────────────┘                   │
//! │                        ▼                                    │
//! │            ┌───────────────────────┐                       │
//! │            │   GlobalExplorerState │                       │
//! │            │  (Shared dedup/APIs)  │                       │
//! │            └───────────────────────┘                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ### 单 Agent 模式
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
//!     config, mcp_service, "anthropic".to_string(), "claude-sonnet-4-20250514".to_string(),
//! );
//! let summary = explorer.start().await?;
//! ```
//!
//! ### 多 Agent 模式
//! ```rust,ignore
//! use sentinel_ai::engines::vision_explorer::multi_agent::{
//!     MultiAgentExplorer, MultiAgentConfig, ExplorationMode,
//! };
//!
//! let multi_config = MultiAgentConfig {
//!     mode: ExplorationMode::Sequential,
//!     max_concurrent_workers: 3,
//!     ..Default::default()
//! };
//!
//! let explorer = MultiAgentExplorer::new(config, multi_config, mcp_service, llm_config);
//! let summary = explorer.start().await?;
//! ```

pub mod browser_scripts;
pub mod coverage_engine;
pub mod element_manager;

pub mod integrations;
pub mod message_emitter;
pub mod route_tracker;
pub mod state;
pub mod tool;
pub mod tools;
pub mod types;

// Refactored modules (split from explorer.rs)
pub mod action_builder;
pub mod element_formatter;
pub mod login_detector;
pub mod vlm_parser;

// Refactored modules (split from tools.rs)
pub mod playwright_bridge;
pub mod text_mode_types;

// Multi-Agent exploration architecture
pub mod multi_agent;

// 导出核心类型
pub use types::{
    get_browser_tool_definitions,
    ActionRecord,
    ActionResult,
    ApiEndpoint,
    BrowserAction,
    ContextSummary,
    ConversationMessage,
    ExplorationState,
    ExplorationStatus,
    FormField,
    // 表单相关
    FormInfo as VisionFormInfo,
    LoginCredentials,
    PageElement,
    PageState,
    // Moved from explorer.rs
    TakeoverEvent,
    TakeoverSession,
    // 新增类型
    TakeoverStatus,
    UserAction,
    VisionExplorerConfig,
    VlmAnalysisResult,
};

// 导出工具
pub use tools::BrowserTools;

// 导出状态管理
pub use state::{ExplorationSummary, StateManager};

// 导出覆盖率引擎
pub use coverage_engine::{
    CompletionCheck, CoverageEngine, CoverageReport, COVERAGE_TARGET, STABILITY_THRESHOLD,
};
pub use element_manager::{DynamicComponent, ElementFingerprint, ElementManager, ElementStats};
pub use route_tracker::{RouteStats, RouteTracker};

// 导出集成模块
pub use integrations::{
    ApiDiscoveryStats, ContextSummaryManager, PassiveProxyIntegration, ProxyRequestInfo,
    TakeoverManager,
};

// 导出消息发送器
pub use message_emitter::{
    VisionAction, VisionAnalysis, VisionCoverageUpdate, VisionExplorationStats,
    VisionExplorerMessageEmitter, VisionStep,
};

pub use tool::VisionExplorerTool;

// Multi-Agent exports
pub use multi_agent::{
    ExplorationMode, ExplorationScope, GlobalExplorerState, ManagerAgent, MultiAgentConfig,
    MultiAgentExplorer, NavigationAnalysis, WorkerAgent, WorkerResult, WorkerTask,
};

// ============================================================================
// Global Registry for Active Takeover Managers
// ============================================================================

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global registry for active TakeoverManagers (keyed by execution_id)
/// Allows credential submission to running explorers
pub static ACTIVE_TAKEOVER_MANAGERS: Lazy<RwLock<HashMap<String, Arc<RwLock<TakeoverManager>>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a TakeoverManager in the global registry
pub async fn register_takeover_manager(
    execution_id: String,
    manager: Arc<RwLock<TakeoverManager>>,
) {
    let mut registry = ACTIVE_TAKEOVER_MANAGERS.write().await;
    registry.insert(execution_id.clone(), manager);
    tracing::info!(
        "Registered TakeoverManager with execution_id: {}",
        execution_id
    );
}

/// Unregister a TakeoverManager from the global registry
pub async fn unregister_takeover_manager(execution_id: &str) {
    let mut registry = ACTIVE_TAKEOVER_MANAGERS.write().await;
    if registry.remove(execution_id).is_some() {
        tracing::info!(
            "Unregistered TakeoverManager with execution_id: {}",
            execution_id
        );
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
    let manager = get_takeover_manager(execution_id).await.ok_or_else(|| {
        format!(
            "No active explorer found with execution_id: {}",
            execution_id
        )
    })?;

    let mut manager_guard = manager.write().await;
    manager_guard.set_user_credentials(username, password, verification_code, extra_fields);

    // 关键修复：归还控制权，让探索循环继续执行
    // 将 TakeoverStatus 从 WaitingForUser 更新为 Returned
    manager_guard.return_control();
    tracing::info!(
        "Credentials submitted and control returned for execution_id: {}",
        execution_id
    );

    Ok(())
}

/// Submit a user message to a running VisionExplorer via its TakeoverManager.
/// The message will be injected into the next VLM prompt.
pub async fn submit_user_message(execution_id: &str, message: String) -> Result<(), String> {
    let manager = get_takeover_manager(execution_id).await.ok_or_else(|| {
        format!(
            "No active explorer found with execution_id: {}",
            execution_id
        )
    })?;

    let mut manager_guard = manager.write().await;
    manager_guard.push_user_message(message);
    Ok(())
}

/// Skip login for a running VisionExplorer.
/// The explorer will continue without credentials and explore public pages.
pub async fn skip_login(execution_id: &str) -> Result<(), String> {
    let manager = get_takeover_manager(execution_id).await.ok_or_else(|| {
        format!(
            "No active explorer found with execution_id: {}",
            execution_id
        )
    })?;

    let mut manager_guard = manager.write().await;
    manager_guard.mark_login_skipped();
    manager_guard.return_control();

    tracing::info!(
        "Skip login set and control returned for execution_id: {}",
        execution_id
    );
    Ok(())
}

/// Mark manual login as complete for a running VisionExplorer.
pub async fn manual_login_complete(execution_id: &str) -> Result<(), String> {
    let manager = get_takeover_manager(execution_id).await.ok_or_else(|| {
        format!(
            "No active explorer found with execution_id: {}",
            execution_id
        )
    })?;

    let mut manager_guard = manager.write().await;
    manager_guard.mark_login_manual_success();

    tracing::info!(
        "Manual login complete marked for execution_id: {}",
        execution_id
    );
    Ok(())
}
