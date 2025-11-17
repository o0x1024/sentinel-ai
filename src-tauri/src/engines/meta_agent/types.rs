//! Meta Agent 类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 子架构类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubArchitecture {
    /// Plan-and-Execute: 适合需要动态重新规划的任务
    PlanAndExecute,
    /// ReWOO: 适合可以预先完整规划的任务
    ReWOO,
    /// LLMCompiler: 适合可以并行执行的任务
    LLMCompiler,
}

impl SubArchitecture {
    /// 获取架构名称
    pub fn name(&self) -> &str {
        match self {
            SubArchitecture::PlanAndExecute => "plan_and_execute",
            SubArchitecture::ReWOO => "rewoo",
            SubArchitecture::LLMCompiler => "llm_compiler",
        }
    }

    /// 获取架构描述
    pub fn description(&self) -> &str {
        match self {
            SubArchitecture::PlanAndExecute => 
                "适合需要动态重新规划的复杂任务，支持根据执行结果调整计划",
            SubArchitecture::ReWOO => 
                "适合可以预先完整规划的任务，一次性生成所有步骤并执行",
            SubArchitecture::LLMCompiler => 
                "适合可以并行执行的任务，自动识别依赖关系并并行化执行",
        }
    }

    /// 获取适用场景
    pub fn scenarios(&self) -> Vec<&str> {
        match self {
            SubArchitecture::PlanAndExecute => vec![
                "安全渗透测试（需要根据结果调整策略）",
                "动态问题解决",
                "需要频繁重新规划的任务",
            ],
            SubArchitecture::ReWOO => vec![
                "信息收集和分析",
                "独立步骤的批处理任务",
                "可以预先规划的工作流",
            ],
            SubArchitecture::LLMCompiler => vec![
                "大规模数据处理",
                "多个独立API调用",
                "可并行的批量操作",
            ],
        }
    }
}

/// Meta Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAgentConfig {
    /// ReAct 主控制器的最大迭代次数
    pub max_iterations: usize,
    /// 是否启用自动架构选择
    pub enable_auto_selection: bool,
    /// 是否允许嵌套调用（子架构再调用子架构）
    pub allow_nested_calls: bool,
    /// 最大嵌套深度
    pub max_nesting_depth: usize,
}

impl Default for MetaAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            enable_auto_selection: true,
            allow_nested_calls: false,
            max_nesting_depth: 2,
        }
    }
}

/// 子架构调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubArchitectureCall {
    /// 调用ID
    pub call_id: String,
    /// 使用的架构
    pub architecture: SubArchitecture,
    /// 任务描述
    pub task_description: String,
    /// 任务参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 开始时间
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// 结束时间
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时长（毫秒）
    pub duration_ms: Option<f64>,
}

/// Meta Agent 执行跟踪
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAgentTrace {
    /// 跟踪ID
    pub trace_id: String,
    /// 原始任务
    pub original_task: String,
    /// ReAct 迭代次数
    pub react_iterations: usize,
    /// 子架构调用记录
    pub sub_calls: Vec<SubArchitectureCall>,
    /// 总执行时长（毫秒）
    pub total_duration_ms: f64,
    /// 最终结果
    pub final_result: Option<String>,
    /// 执行状态
    pub status: MetaAgentStatus,
}

/// Meta Agent 执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetaAgentStatus {
    /// 运行中
    Running,
    /// 成功完成
    Completed,
    /// 失败
    Failed,
    /// 被取消
    Cancelled,
    /// 达到最大迭代次数
    MaxIterationsReached,
}

