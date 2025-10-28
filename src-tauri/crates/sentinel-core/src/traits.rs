//! Core traits and interfaces

use async_trait::async_trait;
use crate::Result;

/// 基础服务trait
#[async_trait]
pub trait Service: Send + Sync {
    /// 初始化服务
    async fn initialize(&mut self) -> Result<()>;
    
    /// 关闭服务
    async fn shutdown(&mut self) -> Result<()>;
    
    /// 检查服务健康状态
    async fn health_check(&self) -> Result<bool>;
}

/// 可配置的trait
#[async_trait]
pub trait Configurable: Send + Sync {
    type Config: Send + Sync;
    
    /// 获取配置
    async fn get_config(&self) -> Result<Self::Config>;
    
    /// 设置配置
    async fn set_config(&mut self, config: Self::Config) -> Result<()>;
}

/// 可执行的trait
#[async_trait]
pub trait Executable: Send + Sync {
    type Input: Send + Sync;
    type Output: Send + Sync;
    
    /// 执行操作
    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;
}

/// 可缓存的trait
#[async_trait]
pub trait Cacheable: Send + Sync {
    type Key: Send + Sync;
    type Value: Send + Sync;
    
    /// 获取缓存值
    async fn get(&self, key: &Self::Key) -> Result<Option<Self::Value>>;
    
    /// 设置缓存值
    async fn set(&self, key: Self::Key, value: Self::Value) -> Result<()>;
    
    /// 删除缓存值
    async fn remove(&self, key: &Self::Key) -> Result<()>;
    
    /// 清空缓存
    async fn clear(&self) -> Result<()>;
}

/// 可监控的trait
#[async_trait]
pub trait Monitorable: Send + Sync {
    type Metrics: Send + Sync;
    
    /// 获取监控指标
    async fn get_metrics(&self) -> Result<Self::Metrics>;
}

/// 可持久化的trait
#[async_trait]
pub trait Persistable: Send + Sync {
    /// 保存状态
    async fn save(&self) -> Result<()>;
    
    /// 加载状态
    async fn load(&mut self) -> Result<()>;
}
