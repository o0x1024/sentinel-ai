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

// 导出核心类型
pub use types::{
    VisionExplorerConfig, ExplorationState, ExplorationStatus,
    PageState, PageElement, ActionRecord, ApiEndpoint,
    BrowserAction, ActionResult, VlmAnalysisResult,
    get_browser_tool_definitions,
    // 新增类型
    TakeoverStatus, TakeoverSession, UserAction,
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

// 导出集成模块
pub use integrations::{
    ContextSummaryManager, PassiveProxyIntegration, TakeoverManager,
    ProxyRequestInfo, ApiDiscoveryStats,
};

// 导出消息发送器
pub use message_emitter::{
    VisionExplorerMessageEmitter, VisionStep, VisionAnalysis,
    VisionAction, VisionExplorationStats,
};

pub use tool::VisionExplorerTool;

