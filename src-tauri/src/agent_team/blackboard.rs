//! Agent Team 共享白板状态管理
//!
//! Blackboard 是结构化 KV 存储，记录阶段性讨论共识与结论。
//! 防止多轮讨论导致 Token 爆炸的核心机制。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::models::{AgentTeamBlackboardEntry, BlackboardEntryType};

/// 白板内存状态（运行时缓存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardSnapshot {
    pub session_id: String,
    /// 共识点列表
    pub consensus_items: Vec<BlackboardItem>,
    /// 分歧点列表
    pub dispute_items: Vec<BlackboardItem>,
    /// 待决事项列表
    pub action_items: Vec<BlackboardItem>,
    /// 快照版本（每次更新递增）
    pub version: u64,
}

impl BlackboardSnapshot {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            consensus_items: vec![],
            dispute_items: vec![],
            action_items: vec![],
            version: 0,
        }
    }

    /// 生成供 LLM 角色读取的摘要文本（控制 Token 用量）
    pub fn to_context_summary(&self) -> String {
        let mut parts = vec![];
        parts.push(format!("=== 共享白板摘要（版本 v{}）===", self.version));

        if !self.consensus_items.is_empty() {
            parts.push("\n【已达成共识】".to_string());
            for (i, item) in self.consensus_items.iter().enumerate() {
                parts.push(format!("{}. [{}] {}: {}", 
                    i + 1, 
                    item.contributed_by.as_deref().unwrap_or("未知"),
                    item.title, 
                    item.content
                ));
            }
        }

        if !self.dispute_items.is_empty() {
            parts.push("\n【当前分歧点】".to_string());
            for (i, item) in self.dispute_items.iter().enumerate() {
                parts.push(format!("{}. [{}] {}: {}", 
                    i + 1, 
                    item.contributed_by.as_deref().unwrap_or("未知"),
                    item.title, 
                    item.content
                ));
            }
        }

        if !self.action_items.is_empty() {
            parts.push("\n【待决事项】".to_string());
            for (i, item) in self.action_items.iter().enumerate() {
                let status = if item.is_resolved { "✓" } else { "○" };
                parts.push(format!("{}. {} {}: {}", i + 1, status, item.title, item.content));
            }
        }

        if parts.len() == 1 {
            parts.push("\n（白板暂无内容）".to_string());
        }

        parts.join("\n")
    }
}

/// 白板条目（内存态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub contributed_by: Option<String>,
    pub is_resolved: bool,
}

impl From<&AgentTeamBlackboardEntry> for BlackboardItem {
    fn from(e: &AgentTeamBlackboardEntry) -> Self {
        Self {
            id: e.id.clone(),
            title: e.title.clone(),
            content: e.content.clone(),
            contributed_by: e.contributed_by.clone(),
            is_resolved: e.is_resolved,
        }
    }
}

/// 全局白板管理器（内存缓存）
pub struct BlackboardManager {
    snapshots: RwLock<HashMap<String, BlackboardSnapshot>>,
}

impl BlackboardManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            snapshots: RwLock::new(HashMap::new()),
        })
    }

    /// 获取或初始化指定 session 的白板快照
    pub async fn get_snapshot(&self, session_id: &str) -> BlackboardSnapshot {
        let snapshots = self.snapshots.read().await;
        snapshots
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| BlackboardSnapshot::new(session_id))
    }

    /// 从数据库条目重建白板快照（启动时/恢复时使用）
    pub async fn rebuild_from_entries(
        &self,
        session_id: &str,
        entries: Vec<AgentTeamBlackboardEntry>,
    ) {
        let mut snapshot = BlackboardSnapshot::new(session_id);

        for entry in &entries {
            let item = BlackboardItem::from(entry);
            match entry.entry_type.parse::<BlackboardEntryType>() {
                Ok(BlackboardEntryType::Consensus) => snapshot.consensus_items.push(item),
                Ok(BlackboardEntryType::Dispute) => snapshot.dispute_items.push(item),
                Ok(BlackboardEntryType::ActionItem) => snapshot.action_items.push(item),
                Err(_) => {}
            }
        }
        snapshot.version = entries.len() as u64;

        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(session_id.to_string(), snapshot);
        info!("Blackboard rebuilt for session {} with {} entries", session_id, entries.len());
    }

    /// 向白板追加新条目
    pub async fn append_entry(
        &self,
        session_id: &str,
        entry_type: BlackboardEntryType,
        title: &str,
        content: &str,
        contributed_by: Option<&str>,
        entry_id: &str,
    ) {
        let mut snapshots = self.snapshots.write().await;
        let snapshot = snapshots
            .entry(session_id.to_string())
            .or_insert_with(|| BlackboardSnapshot::new(session_id));

        let item = BlackboardItem {
            id: entry_id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            contributed_by: contributed_by.map(|s| s.to_string()),
            is_resolved: false,
        };

        match entry_type {
            BlackboardEntryType::Consensus => snapshot.consensus_items.push(item),
            BlackboardEntryType::Dispute => snapshot.dispute_items.push(item),
            BlackboardEntryType::ActionItem => snapshot.action_items.push(item),
        }
        snapshot.version += 1;

        info!(
            "Blackboard updated for session {}: {:?} '{}' (v{})",
            session_id, entry_type, title, snapshot.version
        );
    }

    /// 标记待决事项为已解决
    pub async fn resolve_action_item(&self, session_id: &str, item_id: &str) {
        let mut snapshots = self.snapshots.write().await;
        if let Some(snapshot) = snapshots.get_mut(session_id) {
            if let Some(item) = snapshot.action_items.iter_mut().find(|i| i.id == item_id) {
                item.is_resolved = true;
                snapshot.version += 1;
            }
        }
    }

    /// 生成上下文摘要（供角色读取）
    pub async fn get_context_summary(&self, session_id: &str) -> String {
        let snapshot = self.get_snapshot(session_id).await;
        snapshot.to_context_summary()
    }

    /// 清理 session 白板（session 结束后调用）
    pub async fn cleanup(&self, session_id: &str) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.remove(session_id);
    }
}

impl Default for BlackboardManager {
    fn default() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
        }
    }
}
