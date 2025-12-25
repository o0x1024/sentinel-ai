pub mod ordered_message;
pub mod message_emitter;
pub mod prompt_resolver;
pub mod aliyun_oss;
pub mod streaming_optimizer;

// macOS 系统代理模块已移至 sentinel_traffic::system_proxy
// 全局代理配置已移至 sentinel_core::global_proxy

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tauri::AppHandle;

static APP_HANDLE_STORE: once_cell::sync::Lazy<Mutex<Option<AppHandle>>> = once_cell::sync::Lazy::new(|| Mutex::new(None));
static SEQUENCE_MAP: once_cell::sync::Lazy<Mutex<HashMap<String, Arc<AtomicU64>>>> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
static PLAN_VERSION_MAP: once_cell::sync::Lazy<Mutex<HashMap<String, Arc<AtomicU32>>>> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// 在应用启动时注册全局 AppHandle（需在 setup 中调用）
pub fn register_app_handle(handle: &AppHandle) {
    let mut guard = APP_HANDLE_STORE.lock().unwrap();
    *guard = Some(handle.clone());
}

/// 安全获取全局 AppHandle（可能为 None）
pub fn app_handle_safe() -> Option<AppHandle> {
    APP_HANDLE_STORE.lock().unwrap().clone()
}

/// 获取某个 session 的下一个序列号（从1开始）
pub fn next_sequence(session_id: &str) -> u64 {
    let mut map = SEQUENCE_MAP.lock().unwrap();
    let counter = map.entry(session_id.to_string()).or_insert_with(|| Arc::new(AtomicU64::new(0)) ).clone();
    counter.fetch_add(1, Ordering::SeqCst) + 1
}

/// 初始化计划版本（若不存在则设为1并返回1；若已存在则返回当前版本不改变）
pub fn init_plan_version(session_id: &str) -> u32 {
    let mut map = PLAN_VERSION_MAP.lock().unwrap();
    let counter = map.entry(session_id.to_string()).or_insert_with(|| Arc::new(AtomicU32::new(1))).clone();
    counter.load(Ordering::SeqCst)
}

/// 获取当前计划版本（不存在则返回0，不会初始化）
pub fn current_plan_version(session_id: &str) -> u32 {
    let map = PLAN_VERSION_MAP.lock().unwrap();
    map.get(session_id).map(|c| c.load(Ordering::SeqCst)).unwrap_or(0)
}

/// 递增并返回新的计划版本（若不存在则从1开始返回1）
pub fn next_plan_version(session_id: &str) -> u32 {
    let mut map = PLAN_VERSION_MAP.lock().unwrap();
    let counter = map.entry(session_id.to_string()).or_insert_with(|| Arc::new(AtomicU32::new(0))).clone();
    counter.fetch_add(1, Ordering::SeqCst) + 1
}
