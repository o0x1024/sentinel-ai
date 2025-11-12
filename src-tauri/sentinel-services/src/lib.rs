pub use sentinel_core as core;
pub use sentinel_db as db;
pub use sentinel_tools as tools;
pub use sentinel_rag as rag;

// 导出迁移的服务模块
pub mod message_emitter;
pub mod performance;
pub mod dictionary;

// 重新导出常用类型
pub use message_emitter::{MessageEmitter, TauriMessageEmitter};
pub use performance::{
    PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
    monitor_async,
};
pub use dictionary::DictionaryService;


