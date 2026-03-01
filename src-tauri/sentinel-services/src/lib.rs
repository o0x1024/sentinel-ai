pub use sentinel_core as core;
pub use sentinel_db as db;
pub use sentinel_rag as rag;
pub use sentinel_tools as tools;

// 导出迁移的服务模块
pub mod dictionary;
pub mod message_emitter;
pub mod performance;

// 重新导出常用类型
pub use dictionary::DictionaryService;
pub use message_emitter::{MessageEmitter, TauriMessageEmitter};
pub use performance::{
    monitor_async, PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
};
