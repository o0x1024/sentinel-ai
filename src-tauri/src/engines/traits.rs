//! 执行引擎统一接口定义
//!
//! 泛化后的引擎架构，所有执行模式都内嵌在 ReAct 引擎中

use crate::agents::traits::*;
use async_trait::async_trait;

/// 执行引擎基础trait
#[async_trait]
pub trait BaseExecutionEngine: Send + Sync {
    /// 获取引擎名称
    fn get_name(&self) -> &str;
    
    /// 获取引擎描述
    fn get_description(&self) -> &str;
    
    /// 获取引擎版本
    fn get_version(&self) -> &str;
    
    /// 获取支持的场景
    fn get_supported_scenarios(&self) -> Vec<String>;
    
    /// 获取性能特征
    fn get_performance_characteristics(&self) -> PerformanceCharacteristics;
}
