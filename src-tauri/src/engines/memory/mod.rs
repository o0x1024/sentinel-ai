pub mod memory;
pub mod memory_impl;

use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

pub use memory_impl::IntelligentMemory;

/// 全局智能记忆实例（进程级，仅驻内存）
static GLOBAL_MEMORY: OnceLock<Arc<RwLock<IntelligentMemory>>> = OnceLock::new();

/// 获取全局记忆实例
pub fn get_global_memory() -> Arc<RwLock<IntelligentMemory>> {
    GLOBAL_MEMORY
        .get_or_init(|| Arc::new(RwLock::new(IntelligentMemory::new())))
        .clone()
}