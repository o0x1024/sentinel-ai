//! Finding 去重与持久化模块

use crate::Finding;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 计算 Finding 签名（用于去重）
pub fn compute_finding_signature(finding: &Finding) -> String {
    let mut hasher = Sha256::new();
    hasher.update(finding.plugin_id.as_bytes());
    hasher.update(finding.url.as_bytes());
    hasher.update(finding.location.as_bytes());
    hasher.update(finding.evidence.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Finding 去重服务
pub struct FindingDeduplicator {
    /// 内存中的签名集合（用于快速去重）
    seen_signatures: Arc<RwLock<HashSet<String>>>,
    /// 最大缓存条目数（防止内存溢出）
    max_cache_size: usize,
}

impl FindingDeduplicator {
    /// 创建新的去重服务实例
    pub fn new() -> Self {
        Self {
            seen_signatures: Arc::new(RwLock::new(HashSet::new())),
            max_cache_size: 10000, // 默认最多缓存10000个签名
        }
    }

    /// 创建带自定义缓存大小的去重服务
    pub fn with_cache_size(max_cache_size: usize) -> Self {
        Self {
            seen_signatures: Arc::new(RwLock::new(HashSet::new())),
            max_cache_size,
        }
    }

    /// 检查是否重复
    pub async fn is_duplicate(&self, signature: &str) -> bool {
        let signatures = self.seen_signatures.read().await;
        signatures.contains(signature)
    }

    /// 标记为已处理
    pub async fn mark_as_seen(&self, signature: String) {
        let mut signatures = self.seen_signatures.write().await;

        // 如果缓存已满，清空一半（LRU 简化版）
        if signatures.len() >= self.max_cache_size {
            tracing::warn!(
                "Finding signature cache full ({}/{}), clearing half",
                signatures.len(),
                self.max_cache_size
            );
            let to_remove: Vec<_> = signatures
                .iter()
                .take(self.max_cache_size / 2)
                .cloned()
                .collect();
            for sig in to_remove {
                signatures.remove(&sig);
            }
        }

        signatures.insert(signature);
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut signatures = self.seen_signatures.write().await;
        signatures.clear();
        tracing::info!("Finding signature cache cleared");
    }

    /// 获取当前缓存大小
    pub async fn cache_size(&self) -> usize {
        let signatures = self.seen_signatures.read().await;
        signatures.len()
    }
}

impl Default for FindingDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}
