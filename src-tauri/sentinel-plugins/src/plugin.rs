//! 插件管理器模块
//!
//! 提供插件加载、启用/禁用、注册表管理功能

use crate::error::{PluginError, Result};
use crate::plugin_engine::PluginEngine;
use crate::types::{Finding, HttpTransaction, PluginMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 插件统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatistics {
    /// 总插件数
    pub total: usize,
    /// 已启用数量
    pub enabled: usize,
    /// 已禁用数量
    pub disabled: usize,
    /// 加载但未启用数量
    pub loaded: usize,
    /// 错误状态数量
    pub error: usize,
}

/// 插件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    /// 已加载但未启用
    Loaded,
    /// 已启用
    Enabled,
    /// 已禁用
    Disabled,
    /// 加载失败
    Error,
}

/// 插件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRecord {
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 插件路径（已废弃，插件现存储在数据库中）
    #[deprecated(note = "Plugins are now stored in database, this field is no longer used")]
    #[serde(serialize_with = "serialize_pathbuf_opt")]
    #[serde(deserialize_with = "deserialize_pathbuf_opt")]
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// 插件状态
    pub status: PluginStatus,
    /// 最后错误消息（如果有）
    pub last_error: Option<String>,
    /// 是否已收藏
    #[serde(default)]
    pub is_favorited: bool,
}

// PathBuf 序列化辅助函数（可选版本）
fn serialize_pathbuf_opt<S>(
    path: &Option<PathBuf>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match path {
        Some(p) => serializer.serialize_str(&p.to_string_lossy()),
        None => serializer.serialize_str(""),
    }
}

fn deserialize_pathbuf_opt<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(s)))
    }
}

/// 插件管理器
pub struct PluginManager {
    /// 内存插件注册表（仅用于运行时调试/临时加载）。
    /// 数据来源现在将是数据库，而非文件系统目录扫描。
    registry: Arc<RwLock<HashMap<String, PluginRecord>>>,
    /// 插件代码缓存（plugin_id -> code）
    /// 用于执行时快速访问插件代码
    code_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl PluginManager {
    /// 创建插件管理器（不再依赖文件系统目录）。
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            code_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启用插件
    pub async fn enable_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        let record = registry
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        record.status = PluginStatus::Enabled;
        info!("Plugin enabled: {}", plugin_id);
        Ok(())
    }

    /// 禁用插件
    pub async fn disable_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        let record = registry
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        record.status = PluginStatus::Disabled;
        info!("Plugin disabled: {}", plugin_id);
        Ok(())
    }

    /// 获取已启用插件列表
    pub async fn get_enabled_plugins(&self) -> Vec<String> {
        let registry = self.registry.read().await;
        registry
            .iter()
            .filter(|(_, record)| record.status == PluginStatus::Enabled)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 获取所有插件
    pub async fn get_all_plugins(&self) -> Vec<PluginRecord> {
        let registry = self.registry.read().await;
        registry.values().cloned().collect()
    }

    /// 获取插件元数据
    pub async fn get_metadata(&self, plugin_id: &str) -> Option<PluginMetadata> {
        let registry = self.registry.read().await;
        registry.get(plugin_id).map(|r| r.metadata.clone())
    }

    /// 获取插件记录（包含状态和路径）
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginRecord> {
        let registry = self.registry.read().await;
        registry.get(plugin_id).cloned()
    }

    /// 注册插件到内存（从数据库加载时使用）
    ///
    /// # 参数
    /// - `plugin_id`: 插件唯一标识
    /// - `metadata`: 插件元数据
    /// - `enabled`: 是否启用
    pub async fn register_plugin(
        &self,
        plugin_id: String,
        metadata: PluginMetadata,
        enabled: bool,
    ) -> Result<()> {
        let mut registry = self.registry.write().await;

        let status = if enabled {
            PluginStatus::Enabled
        } else {
            PluginStatus::Disabled
        };

        #[allow(deprecated)]
        let record = PluginRecord {
            metadata: metadata.clone(),
            path: None, // 插件存储在数据库中
            status,
            last_error: None,
            is_favorited: false, // 默认未收藏
        };

        registry.insert(plugin_id.clone(), record);
        info!(
            "Plugin registered in memory: {} ({})",
            metadata.name, plugin_id
        );
        Ok(())
    }

    /// 设置插件代码到缓存
    ///
    /// # 参数
    /// - `plugin_id`: 插件唯一标识
    /// - `code`: 插件 JavaScript 代码
    ///
    /// # 说明
    /// 在注册插件后调用此方法设置插件代码，用于后续执行。
    pub async fn set_plugin_code(&self, plugin_id: String, code: String) -> Result<()> {
        let mut cache = self.code_cache.write().await;
        cache.insert(plugin_id.clone(), code);
        debug!("Plugin code cached: {}", plugin_id);
        Ok(())
    }

    /// 获取插件代码
    async fn get_plugin_code(&self, plugin_id: &str) -> Result<String> {
        let cache = self.code_cache.read().await;
        cache
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(format!("Plugin code not found: {}", plugin_id)))
    }

    /// 从内存中移除插件
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        registry
            .remove(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        // 同时清除代码缓存
        let mut cache = self.code_cache.write().await;
        cache.remove(plugin_id);

        info!("Plugin unregistered from memory: {}", plugin_id);
        Ok(())
    }

    /// 更新插件错误状态
    pub async fn set_plugin_error(&self, plugin_id: &str, error: String) -> Result<()> {
        let mut registry = self.registry.write().await;
        let record = registry
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        record.status = PluginStatus::Error;
        record.last_error = Some(error.clone());
        warn!("Plugin error set: {} - {}", plugin_id, error);
        Ok(())
    }

    /// 清除插件错误状态
    pub async fn clear_plugin_error(&self, plugin_id: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        let record = registry
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if record.status == PluginStatus::Error {
            record.status = PluginStatus::Loaded;
        }
        record.last_error = None;
        info!("Plugin error cleared: {}", plugin_id);
        Ok(())
    }

    /// 检查插件是否已启用
    pub async fn is_enabled(&self, plugin_id: &str) -> bool {
        let registry = self.registry.read().await;
        registry
            .get(plugin_id)
            .map(|r| r.status == PluginStatus::Enabled)
            .unwrap_or(false)
    }

    /// 获取插件统计信息
    pub async fn get_statistics(&self) -> PluginStatistics {
        let registry = self.registry.read().await;
        let total = registry.len();
        let enabled = registry
            .values()
            .filter(|r| r.status == PluginStatus::Enabled)
            .count();
        let disabled = registry
            .values()
            .filter(|r| r.status == PluginStatus::Disabled)
            .count();
        let error = registry
            .values()
            .filter(|r| r.status == PluginStatus::Error)
            .count();

        PluginStatistics {
            total,
            enabled,
            disabled,
            error,
            loaded: registry
                .values()
                .filter(|r| r.status == PluginStatus::Loaded)
                .count(),
        }
    }

    /// （已移除）文件元数据解析。数据库模式下由外部提供完整元数据。
    #[allow(dead_code)]
    fn parse_metadata(&self, _content: &str, _path: &PathBuf) -> Result<PluginMetadata> {
        Err(PluginError::Load(
            "parse_metadata deprecated in DB-only mode".to_string(),
        ))
    }

    /// 调用插件扫描请求
    ///
    /// 在数据库模式下，插件代码存储在 DB 中。
    /// 调用插件扫描完整的 HTTP 事务
    ///
    /// 在数据库模式下，插件代码存储在 DB 中。
    /// 此方法验证插件存在性和启用状态，然后使用 PluginEngine 执行插件代码。
    pub async fn scan_transaction(
        &self,
        plugin_id: &str,
        transaction: &HttpTransaction,
    ) -> Result<Vec<Finding>> {
        info!(
            "Scanning transaction with plugin '{}': request_id={}",
            plugin_id, transaction.request.id
        );

        // 验证插件是否存在且已启用
        let (metadata, code) = {
            let registry = self.registry.read().await;
            let record = registry
                .get(plugin_id)
                .ok_or_else(|| PluginError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

            if record.status != PluginStatus::Enabled {
                return Err(PluginError::Execution(format!(
                    "Plugin '{}' is not enabled (status: {:?})",
                    plugin_id, record.status
                )));
            }

            let metadata = record.metadata.clone();
            drop(registry);

            // 获取插件代码
            let code = self.get_plugin_code(plugin_id).await?;
            (metadata, code)
        };

        // 克隆事务以移动到 spawn_blocking
        let tx_clone = transaction.clone();

        // 使用 PluginEngine 执行
        let findings = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| PluginError::Execution(format!("Failed to build runtime: {}", e)))?;

            rt.block_on(async move {
                let mut engine = PluginEngine::new()?;
                engine.load_plugin_with_metadata(&code, metadata).await?;
                engine.scan_transaction(&tx_clone).await
            })
        })
        .await
        .map_err(|e| PluginError::Execution(format!("Task join error: {}", e)))??;

        debug!(
            "Plugin {} found {} issues in transaction {}",
            plugin_id,
            findings.len(),
            transaction.request.id
        );

        Ok(findings)
    }

    /// 执行 Agent 插件通用入口
    pub async fn execute_agent(
        &self,
        plugin_id: &str,
        input: &serde_json::Value,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        // 验证插件是否存在且已启用，并获取代码与元数据
        let (metadata, code) = {
            let registry = self.registry.read().await;
            let record = registry
                .get(plugin_id)
                .ok_or_else(|| PluginError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

            if record.status != PluginStatus::Enabled {
                return Err(PluginError::Execution(format!(
                    "Plugin '{}' is not enabled (status: {:?})",
                    plugin_id, record.status
                )));
            }

            let metadata = record.metadata.clone();
            drop(registry);

            // 获取插件代码
            let code = self.get_plugin_code(plugin_id).await?;
            (metadata, code)
        };

        // 使用 PluginEngine 执行 agent 入口
        let input_clone = input.clone();
        let result_pair = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| PluginError::Execution(format!("Failed to build runtime: {}", e)))?;

            rt.block_on(async move {
                let mut engine = PluginEngine::new()?;
                engine.load_plugin_with_metadata(&code, metadata).await?;
                engine.execute_agent(&input_clone).await
            })
        })
        .await
        .map_err(|e| PluginError::Execution(format!("Task join error: {}", e)))??;

        Ok(result_pair)
    }
}
