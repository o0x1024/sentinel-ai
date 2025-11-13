//! 全局取消令牌管理器
//!
//! 提供统一的任务取消机制，支持所有执行架构

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, OnceCell};
use tokio_util::sync::CancellationToken;

/// 全局取消令牌存储（使用OnceCell代替lazy_static）
static CANCELLATION_TOKENS: OnceCell<Arc<RwLock<HashMap<String, CancellationToken>>>> = OnceCell::const_new();

async fn get_tokens() -> Arc<RwLock<HashMap<String, CancellationToken>>> {
    CANCELLATION_TOKENS
        .get_or_init(|| async {
            Arc::new(RwLock::new(HashMap::new()))
        })
        .await
        .clone()
}

/// 注册新的取消令牌
pub async fn register_cancellation_token(execution_id: String) -> CancellationToken {
    let token = CancellationToken::new();
    let tokens_store = get_tokens().await;
    let mut tokens = tokens_store.write().await;
    tokens.insert(execution_id.clone(), token.clone());
    tracing::info!("Registered cancellation token for execution: {}", execution_id);
    token
}

/// 取消指定执行
pub async fn cancel_execution(execution_id: &str) -> bool {
    let tokens_store = get_tokens().await;
    let tokens = tokens_store.read().await;
    if let Some(token) = tokens.get(execution_id) {
        token.cancel();
        tracing::info!("Cancelled execution via token: {}", execution_id);
        true
    } else {
        tracing::warn!("Cancellation token not found for execution: {}", execution_id);
        false
    }
}

/// 检查执行是否被取消
pub async fn is_cancelled(execution_id: &str) -> bool {
    let tokens_store = get_tokens().await;
    let tokens = tokens_store.read().await;
    if let Some(token) = tokens.get(execution_id) {
        token.is_cancelled()
    } else {
        false
    }
}

/// 获取取消令牌
pub async fn get_token(execution_id: &str) -> Option<CancellationToken> {
    let tokens_store = get_tokens().await;
    let tokens = tokens_store.read().await;
    tokens.get(execution_id).cloned()
}

/// 清理取消令牌
pub async fn cleanup_token(execution_id: &str) {
    let tokens_store = get_tokens().await;
    let mut tokens = tokens_store.write().await;
    if tokens.remove(execution_id).is_some() {
        tracing::info!("Cleaned up cancellation token for execution: {}", execution_id);
    }
}

/// 获取所有活跃的执行ID
pub async fn get_active_executions() -> Vec<String> {
    let tokens_store = get_tokens().await;
    let tokens = tokens_store.read().await;
    tokens.keys().cloned().collect()
}

/// 清理所有取消令牌（用于测试或重置）
#[allow(dead_code)]
pub async fn cleanup_all() {
    let tokens_store = get_tokens().await;
    let mut tokens = tokens_store.write().await;
    tokens.clear();
    tracing::info!("Cleaned up all cancellation tokens");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_cancel() {
        let exec_id = "test_exec_1";
        
        // Register token
        let token = register_cancellation_token(exec_id.to_string()).await;
        assert!(!token.is_cancelled());
        
        // Cancel execution
        let cancelled = cancel_execution(exec_id).await;
        assert!(cancelled);
        assert!(token.is_cancelled());
        
        // Cleanup
        cleanup_token(exec_id).await;
    }

    #[tokio::test]
    async fn test_cancel_nonexistent() {
        let cancelled = cancel_execution("nonexistent").await;
        assert!(!cancelled);
    }
}

