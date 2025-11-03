//! 适配器工厂和管理器
//! 
//! 负责创建和管理不同框架的工具适配器实例

use super::framework_adapters::*;
use super::unified_types::*;
use super::UnifiedToolManager;
use anyhow::{anyhow, Result};
use tracing::debug;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{info, warn};

// ============================================================================
// 适配器工厂
// ============================================================================

/// 适配器工厂 - 负责创建不同类型的框架适配器
#[derive(Debug)]
pub struct AdapterFactory {
    tool_manager: Arc<RwLock<UnifiedToolManager>>,
}

impl AdapterFactory {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        Self { tool_manager }
    }

    /// 创建指定框架类型的适配器
    pub fn create_adapter(&self, framework_type: FrameworkType) -> Arc<dyn FrameworkToolAdapter> {
        match framework_type {
            FrameworkType::PlanAndExecute => {
                debug!("Creating Plan & Execute adapter");
                Arc::new(PlanAndExecuteAdapter::new(self.tool_manager.clone()))
            }
            FrameworkType::ReWOO => {
                debug!("Creating ReWOO adapter");
                Arc::new(ReWOOAdapter::new(self.tool_manager.clone()))
            }
            FrameworkType::LLMCompiler => {
                debug!("Creating LLM Compiler adapter");
                Arc::new(LLMCompilerAdapter::new(self.tool_manager.clone()))
            }
            FrameworkType::React => {
                debug!("Creating ReAct adapter");
                // ReAct 使用与 PlanAndExecute 相同的适配器
                Arc::new(PlanAndExecuteAdapter::new(self.tool_manager.clone()))
            }
        }
    }

    /// 创建LLM Compiler兼容的EngineToolAdapter
    pub fn create_engine_adapter(&self) -> Arc<dyn EngineToolAdapter> {
        debug!("Creating Engine Tool Adapter for LLM Compiler compatibility");
        Arc::new(LLMCompilerAdapter::new(self.tool_manager.clone()))
    }

    /// 创建基础适配器
    pub fn create_base_adapter(&self, config: AdapterConfig) -> Arc<dyn FrameworkToolAdapter> {
        debug!("Creating base adapter with custom config");
        Arc::new(BaseFrameworkAdapter::new(self.tool_manager.clone(), config))
    }
}

// ============================================================================
// 适配器注册表
// ============================================================================

/// 适配器注册表 - 管理所有已创建的适配器实例
#[derive(Debug)]
pub struct AdapterRegistry {
    adapters: HashMap<FrameworkType, Arc<dyn FrameworkToolAdapter>>,
    engine_adapter: Option<Arc<dyn EngineToolAdapter>>,
    factory: AdapterFactory,
}

impl AdapterRegistry {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        Self {
            adapters: HashMap::new(),
            engine_adapter: None,
            factory: AdapterFactory::new(tool_manager),
        }
    }

    /// 注册框架适配器
    pub fn register_adapter(&mut self, framework_type: FrameworkType) -> Arc<dyn FrameworkToolAdapter> {
        if let Some(adapter) = self.adapters.get(&framework_type) {
            return adapter.clone();
        }

        let adapter = self.factory.create_adapter(framework_type);
        self.adapters.insert(framework_type, adapter.clone());
        debug!("Registered adapter for framework: {}", framework_type);
        adapter
    }

    /// 获取框架适配器
    pub fn get_adapter(&self, framework_type: FrameworkType) -> Option<Arc<dyn FrameworkToolAdapter>> {
        self.adapters.get(&framework_type).cloned()
    }

    /// 获取或创建框架适配器
    pub fn get_or_create_adapter(&mut self, framework_type: FrameworkType) -> Arc<dyn FrameworkToolAdapter> {
        if let Some(adapter) = self.get_adapter(framework_type) {
            adapter
        } else {
            self.register_adapter(framework_type)
        }
    }

    /// 注册引擎适配器(LLM Compiler兼容)
    pub fn register_engine_adapter(&mut self) -> Arc<dyn EngineToolAdapter> {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.clone();
        }

        let adapter = self.factory.create_engine_adapter();
        self.engine_adapter = Some(adapter.clone());
        debug!("Registered engine adapter for LLM Compiler compatibility");
        adapter
    }

    /// 获取引擎适配器
    pub fn get_engine_adapter(&self) -> Option<Arc<dyn EngineToolAdapter>> {
        self.engine_adapter.clone()
    }

    /// 获取或创建引擎适配器
    pub fn get_or_create_engine_adapter(&mut self) -> Arc<dyn EngineToolAdapter> {
        if let Some(adapter) = self.get_engine_adapter() {
            adapter
        } else {
            self.register_engine_adapter()
        }
    }

    /// 列出所有已注册的适配器
    pub fn list_registered_adapters(&self) -> Vec<FrameworkType> {
        self.adapters.keys().cloned().collect()
    }

    /// 清理未使用的适配器
    pub fn cleanup_unused(&mut self) {
        // 这里可以实现基于引用计数的清理逻辑
        warn!("Adapter cleanup not implemented yet");
    }

    /// 预注册所有框架适配器
    pub fn preregister_all(&mut self) {
        debug!("Pre-registering all framework adapters");
        
        self.register_adapter(FrameworkType::PlanAndExecute);
        self.register_adapter(FrameworkType::ReWOO);
        self.register_adapter(FrameworkType::LLMCompiler);
        self.register_engine_adapter();
        
        debug!("All framework adapters pre-registered successfully");
    }
}

// ============================================================================
// 全局适配器管理器
// ============================================================================

/// 全局适配器管理器 - 单例模式
pub struct GlobalAdapterManager {
    registry: RwLock<AdapterRegistry>,
}

impl GlobalAdapterManager {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        Self {
            registry: RwLock::new(AdapterRegistry::new(tool_manager)),
        }
    }

    /// 获取框架适配器
    pub async fn get_framework_adapter(&self, framework_type: FrameworkType) -> Arc<dyn FrameworkToolAdapter> {
        let mut registry = self.registry.write().await;
        registry.get_or_create_adapter(framework_type)
    }

    /// 获取引擎适配器
    pub async fn get_engine_adapter(&self) -> Arc<dyn EngineToolAdapter> {
        let mut registry = self.registry.write().await;
        registry.get_or_create_engine_adapter()
    }

    /// 预初始化所有适配器
    pub async fn initialize_all(&self) -> Result<()> {
        debug!("Initializing global adapter manager");
        
        let mut registry = self.registry.write().await;
        registry.preregister_all();
        
        debug!("Global adapter manager initialized successfully");
        Ok(())
    }

    /// 获取适配器统计信息
    pub async fn get_adapter_stats(&self) -> HashMap<String, String> {
        let registry = self.registry.read().await;
        let registered = registry.list_registered_adapters();
        
        let mut stats = HashMap::new();
        stats.insert("total_adapters".to_string(), registered.len().to_string());
        stats.insert("frameworks".to_string(), 
                    registered.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", "));
        stats.insert("engine_adapter_registered".to_string(), 
                    registry.get_engine_adapter().is_some().to_string());
        
        stats
    }
}

// ============================================================================
// 全局实例和便捷函数
// ============================================================================

/// 全局适配器管理器实例
static GLOBAL_ADAPTER_MANAGER: OnceLock<Arc<GlobalAdapterManager>> = OnceLock::new();

/// 初始化全局适配器管理器
pub async fn initialize_global_adapter_manager(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Result<()> {
    let manager = Arc::new(GlobalAdapterManager::new(tool_manager));
    manager.initialize_all().await?;
    
    GLOBAL_ADAPTER_MANAGER.set(manager)
        .map_err(|_| anyhow!("Global adapter manager already initialized"))?;
    
    debug!("Global adapter manager initialized successfully");
    Ok(())
}

/// 获取全局适配器管理器
pub fn get_global_adapter_manager() -> Result<Arc<GlobalAdapterManager>> {
    GLOBAL_ADAPTER_MANAGER.get()
        .cloned()
        .ok_or_else(|| anyhow!("Global adapter manager not initialized. Call initialize_global_adapter_manager first."))
}

/// 便捷函数：获取框架适配器
pub async fn get_framework_adapter(framework_type: FrameworkType) -> Result<Arc<dyn FrameworkToolAdapter>> {
    let manager = get_global_adapter_manager()?;
    Ok(manager.get_framework_adapter(framework_type).await)
}

/// 便捷函数：获取引擎适配器 (LLM Compiler兼容)
pub async fn get_engine_adapter() -> Result<Arc<dyn EngineToolAdapter>> {
    let manager = get_global_adapter_manager()?;
    Ok(manager.get_engine_adapter().await)
}

/// 便捷函数：获取全局引擎适配器 (与现有代码兼容)
pub fn get_global_engine_adapter() -> Result<Arc<dyn EngineToolAdapter>> {
    // 提供同步版本以兼容现有代码
    let _manager = get_global_adapter_manager()?;
    // 注意：这是一个简化实现，实际使用时应该使用异步版本
    if let Some(adapter) = GLOBAL_ADAPTER_MANAGER.get() {
        // 使用blocking操作获取适配器（仅用于兼容性）
        Ok(tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                adapter.get_engine_adapter().await
            })
        }))
    } else {
        Err(anyhow!("Global adapter manager not initialized"))
    }
}

/// 检查全局适配器管理器是否已初始化
pub fn is_global_adapter_manager_initialized() -> bool {
    GLOBAL_ADAPTER_MANAGER.get().is_some()
}

// ============================================================================
// 适配器配置构建器
// ============================================================================

/// 适配器配置构建器
pub struct AdapterConfigBuilder {
    config: AdapterConfig,
}

impl AdapterConfigBuilder {
    pub fn new(framework_type: FrameworkType) -> Self {
        Self {
            config: AdapterConfig {
                framework_type,
                ..Default::default()
            },
        }
    }

    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.config.cache_enabled = enabled;
        self
    }

    pub fn max_concurrent_calls(mut self, max: usize) -> Self {
        self.config.max_concurrent_calls = max;
        self
    }

    pub fn default_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = policy;
        self
    }

    pub fn build(self) -> AdapterConfig {
        self.config
    }
}

/// 便捷函数：创建Plan & Execute配置
pub fn create_plan_execute_config() -> AdapterConfig {
    AdapterConfigBuilder::new(FrameworkType::PlanAndExecute)
        .max_concurrent_calls(5)
        .default_timeout(std::time::Duration::from_secs(300))
        .build()
}

/// 便捷函数：创建ReWOO配置
pub fn create_rewoo_config() -> AdapterConfig {
    AdapterConfigBuilder::new(FrameworkType::ReWOO)
        .max_concurrent_calls(3)
        .default_timeout(std::time::Duration::from_secs(180))
        .build()
}

/// 便捷函数：创建LLM Compiler配置
pub fn create_llm_compiler_config() -> AdapterConfig {
    AdapterConfigBuilder::new(FrameworkType::LLMCompiler)
        .max_concurrent_calls(10)
        .default_timeout(std::time::Duration::from_secs(120))
        .build()
}
