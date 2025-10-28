//! Agent系统的核心抽象接口
//! 
//! 定义了Agent、ExecutionEngine、Workflow等核心概念的统一接口

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use uuid::Uuid;
use tauri::AppHandle;

/// Agent能力枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// 网络扫描
    NetworkScanning,
    /// 漏洞检测  
    VulnerabilityDetection,
    /// 数据分析
    DataAnalysis,
    /// 并行处理
    ParallelProcessing,
    /// 工具集成
    ToolIntegration,
    /// 自然语言处理
    NaturalLanguageProcessing,
}

/// Agent任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// 任务ID
    pub id: String,
    /// 任务描述
    pub description: String,
    /// 目标信息
    pub target: Option<String>,
    /// 任务参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 用户ID
    pub user_id: String,
    /// 优先级
    pub priority: TaskPriority,
    /// 超时时间(秒)
    pub timeout: Option<u64>,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Agent会话状态
#[derive(Debug, Clone, Serialize, Deserialize,PartialEq)]
pub enum AgentSessionStatus {
    /// 已创建
    Created,
    /// 规划中
    Planning,
    /// 执行中
    Executing,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// Agent执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResult {
    /// 结果ID
    pub id: String,
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间(毫秒)
    pub execution_time_ms: u64,
    /// 使用的资源
    pub resources_used: HashMap<String, f64>,
    /// 生成的工作产品
    pub artifacts: Vec<ExecutionArtifact>,
}

/// 执行工作产品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    /// 产品类型
    pub artifact_type: ArtifactType,
    /// 产品名称
    pub name: String,
    /// 产品数据
    pub data: serde_json::Value,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 工作产品类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    /// 扫描报告
    ScanReport,
    /// 漏洞列表
    VulnerabilityList,
    /// 分析结果
    AnalysisResult,
    /// 日志文件
    LogFile,
    /// 配置文件
    ConfigFile,
}

/// Agent会话接口
#[async_trait]
pub trait AgentSession: Send + Sync {
    /// 获取会话ID
    fn get_session_id(&self) -> &str;
    
    /// 获取关联的任务
    fn get_task(&self) -> &AgentTask;
    
    /// 获取当前状态
    fn get_status(&self) -> AgentSessionStatus;
    
    /// 更新状态
    async fn update_status(&mut self, status: AgentSessionStatus) -> Result<()>;
    
    /// 添加执行日志
    async fn add_log(&mut self, level: LogLevel, message: String) -> Result<()>;
    
    /// 获取执行日志
    fn get_logs(&self) -> &[SessionLog];
    
    /// 设置执行结果
    async fn set_result(&mut self, result: AgentExecutionResult) -> Result<()>;
    
    /// 获取执行结果
    fn get_result(&self) -> Option<&AgentExecutionResult>;
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize,PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 会话日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

/// Agent核心接口
#[async_trait]
pub trait Agent: Send + Sync {
    /// 获取Agent名称
    fn get_name(&self) -> &str;
    
    /// 获取Agent描述
    fn get_description(&self) -> &str;
    
    /// 获取Agent能力
    fn get_capabilities(&self) -> &[Capability];
    
    /// 检查是否可以处理指定任务
    fn can_handle_task(&self, task: &AgentTask) -> bool;
    
    /// 创建执行会话
    async fn create_session(&'_ self, task: AgentTask) -> Result<Box<dyn AgentSession>>;
    
    /// 执行任务
    async fn execute(&'_ self, session: &mut dyn AgentSession) -> Result<()>;
    
    /// 取消执行
    async fn cancel(&'_ self, session_id: &str) -> Result<()>;
    
    /// 获取执行统计
    async fn get_statistics(&'_ self) -> Result<AgentStatistics>;
}

/// Agent统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功次数
    pub successful_executions: u64,
    /// 失败次数
    pub failed_executions: u64,
    /// 平均执行时间(毫秒)
    pub average_execution_time_ms: f64,
    /// 最后执行时间
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// 执行引擎信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    /// 引擎名称
    pub name: String,
    /// 引擎版本
    pub version: String,
    /// 引擎描述
    pub description: String,
    /// 支持的场景
    pub supported_scenarios: Vec<String>,
    /// 性能特征
    pub performance_characteristics: PerformanceCharacteristics,
}

/// 性能特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    /// Token效率(0-100)
    pub token_efficiency: u8,
    /// 执行速度(0-100)
    pub execution_speed: u8,
    /// 资源消耗(0-100, 越低越好)
    pub resource_usage: u8,
    /// 并发能力(0-100)
    pub concurrency_capability: u8,
    /// 复杂度处理能力(0-100)
    pub complexity_handling: u8,
}

/// 执行引擎接口
#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    /// 获取引擎信息
    fn get_engine_info(&self) -> &EngineInfo;
    
    /// 检查是否支持任务
    fn supports_task(&self, task: &AgentTask) -> bool;
    
    /// 创建执行计划
    async fn create_plan(&self, task: &AgentTask) -> Result<ExecutionPlan>;
    
    /// 执行计划
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<AgentExecutionResult>;
    
    /// 获取执行进度
    async fn get_progress(&self, session_id: &str) -> Result<ExecutionProgress>;
    
    /// 取消执行
    async fn cancel_execution(&self, session_id: &str) -> Result<()> {
        // 默认实现，引擎可以选择重写
        log::warn!("Cancellation not implemented for session: {}", session_id);
        Ok(())
    }
}

/// 执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 计划ID
    pub id: String,
    /// 计划名称
    pub name: String,
    /// 执行步骤
    pub steps: Vec<ExecutionStep>,
    /// 预估执行时间(秒)
    pub estimated_duration: u64,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// 步骤ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: StepType,
    /// 依赖的步骤
    pub dependencies: Vec<String>,
    /// 步骤参数
    pub parameters: HashMap<String, serde_json::Value>,
}

/// 步骤类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// 工具调用
    ToolCall,
    /// 数据处理
    DataProcessing,
    /// 条件判断
    Conditional,
    /// 并行执行
    Parallel,
    /// 等待
    Wait,
    /// LLM调用
    LlmCall,
}

/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU核心数
    pub cpu_cores: Option<u32>,
    /// 内存(MB)
    pub memory_mb: Option<u32>,
    /// 网络并发数
    pub network_concurrency: Option<u32>,
    /// 磁盘空间(MB)
    pub disk_space_mb: Option<u32>,
}

/// 执行进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    /// 总步骤数
    pub total_steps: u32,
    /// 已完成步骤数
    pub completed_steps: u32,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 进度百分比(0-100)
    pub progress_percentage: f32,
    /// 预计剩余时间(秒)
    pub estimated_remaining_seconds: Option<u64>,
}

// 默认实现

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

impl Default for AgentTask {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: String::new(),
            target: None,
            parameters: HashMap::new(),
            user_id: "system".to_string(),
            priority: TaskPriority::Normal,
            timeout: Some(3600), // 1小时默认超时
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_cores: Some(2),
            memory_mb: Some(512),
            network_concurrency: Some(10),
            disk_space_mb: Some(100),
        }
    }
}

impl AgentTask {
    /// 创建新任务
    pub fn new(description: String, user_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description,
            user_id,
            ..Default::default()
        }
    }
    
    /// 设置目标
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// 添加参数
    pub fn with_parameter(mut self, key: String, value: serde_json::Value) -> Self {
        self.parameters.insert(key, value);
        self
    }
}
