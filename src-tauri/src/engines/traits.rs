//! 执行引擎统一接口定义

use crate::agents::traits::*;
use async_trait::async_trait;
use anyhow::Result;

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

/// Plan-and-Execute引擎特定接口
#[async_trait]
pub trait PlanExecuteEngine: BaseExecutionEngine {
    /// 创建执行计划
    async fn create_plan(&self, task: &AgentTask) -> Result<crate::engines::types::ExecutionPlan>;
    
    /// 执行计划
    async fn execute_plan(
        &self, 
        plan: &crate::engines::types::ExecutionPlan,
        session: &mut dyn AgentSession
    ) -> Result<AgentExecutionResult>;
    
    /// 重新规划
    async fn replan(
        &self,
        original_plan: &crate::engines::types::ExecutionPlan,
        current_state: &crate::engines::types::ExecutionSession,
        error: &crate::engines::types::ExecutionError
    ) -> Result<crate::engines::types::ExecutionPlan>;
}

/// ReWOO引擎特定接口
#[async_trait]
pub trait ReWooEngine: BaseExecutionEngine {
    /// 创建推理计划
    async fn create_reasoning_plan(&self, task: &AgentTask) -> Result<String>;
    
    /// 执行工具调用
    async fn execute_tools(&self, plan: &str, variables: &std::collections::HashMap<String, String>) -> Result<String>;
    
    /// 求解最终答案
    async fn solve(&self, plan: &str, tool_results: &str) -> Result<String>;
}

/// LLMCompiler引擎特定接口
#[async_trait]
pub trait LlmCompilerEngine: BaseExecutionEngine {
    /// 创建DAG任务图
    async fn create_dag(&self, task: &AgentTask) -> Result<TaskDAG>;
    
    /// 并行执行任务
    async fn execute_parallel(&self, dag: &TaskDAG) -> Result<Vec<TaskResult>>;
    
    /// 连接和聚合结果
    async fn join_results(&self, results: &[TaskResult]) -> Result<String>;
}

/// 任务DAG定义
#[derive(Debug, Clone)]
pub struct TaskDAG {
    pub nodes: Vec<TaskNode>,
    pub edges: Vec<TaskEdge>,
}

/// 任务节点
#[derive(Debug, Clone)]
pub struct TaskNode {
    pub id: String,
    pub name: String,
    pub task_type: TaskNodeType,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// 任务节点类型
#[derive(Debug, Clone)]
pub enum TaskNodeType {
    ToolCall,
    LlmCall,
    DataProcessing,
    Aggregation,
}

/// 任务边（依赖关系）
#[derive(Debug, Clone)]
pub struct TaskEdge {
    pub from: String,
    pub to: String,
    pub dependency_type: DependencyType,
}

/// 依赖类型
#[derive(Debug, Clone)]
pub enum DependencyType {
    Sequential,
    DataDependency,
    ResourceDependency,
}

/// 任务结果
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub execution_time_ms: f64,
}
