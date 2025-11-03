//! 架构选择器模块
//! 
//! 负责根据查询分析结果智能选择最适合的Agent架构

use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::services::ai::AiServiceManager;
use super::query_analyzer::QueryAnalysisResult;
use log::info;

/// 架构选择结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSelectionResult {
    /// 选择的架构
    pub selected_architecture: String,
    /// 置信度分数
    pub confidence_score: f32,
    /// 选择理由
    pub selection_reasoning: String,
    /// 架构配置
    pub architecture_config: ArchitectureConfiguration,
    /// 备选架构
    pub fallback_architecture: String,
    /// 性能预测
    pub performance_prediction: PerformancePrediction,
}

/// 架构配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureConfiguration {
    /// 最大并行任务数
    pub max_parallel_tasks: u32,
    /// 每个任务超时时间（秒）
    pub timeout_per_task: u32,
    /// 重试策略
    pub retry_policy: String,
    /// 资源限制
    pub resource_limits: Option<ResourceLimits>,
    /// 优化参数
    pub optimization_params: OptimizationParams,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU核心数
    pub cpu_cores: u32,
    /// 内存限制(GB)
    pub memory_gb: u32,
    /// 网络并发数
    pub network_concurrent: u32,
    /// 最大执行时间（秒）
    pub max_execution_time: u32,
}

/// 优化参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationParams {
    /// 启用流式处理
    pub enable_streaming: bool,
    /// 启用结果缓存
    pub enable_caching: bool,
    /// 启用自适应调度
    pub enable_adaptive_scheduling: bool,
    /// 优先级权重
    pub priority_weight: f32,
}

/// 性能预测
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePrediction {
    /// 预计执行时间（秒）
    pub estimated_execution_time: f64,
    /// 预计成功率
    pub estimated_success_rate: f32,
    /// 预计并行效率
    pub estimated_parallel_efficiency: f32,
    /// 预计资源消耗
    pub estimated_resource_consumption: ResourceConsumption,
}

/// 资源消耗预测
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    /// CPU使用率 (0.0-1.0)
    pub cpu_usage: f32,
    /// 内存使用量 (MB)
    pub memory_usage_mb: u32,
    /// 网络带宽 (Mbps)
    pub network_bandwidth_mbps: f32,
}

/// 架构选择器
pub struct ArchitectureSelector {
    /// AI服务管理器
    ai_service_manager: Arc<AiServiceManager>,
}

impl ArchitectureSelector {
    /// 创建新的架构选择器
    pub fn new(ai_service_manager: Arc<AiServiceManager>) -> Self {
        Self {
            ai_service_manager,
        }
    }

    /// 选择最适合的架构
    pub async fn select_architecture(&self, analysis: &QueryAnalysisResult) -> Result<ArchitectureSelectionResult> {
        info!("Selecting architecture for task_type: {}, complexity: {}", 
              analysis.task_type, analysis.complexity_level);

        // 基于规则的架构选择逻辑
        let (selected_architecture, reasoning, confidence) = self.rule_based_selection(analysis);
        
        // 生成架构配置
        let architecture_config = self.generate_architecture_config(analysis, &selected_architecture);
        
        // 性能预测
        let performance_prediction = self.predict_performance(analysis, &selected_architecture);
        
        // 选择备选架构
        let fallback_architecture = self.select_fallback_architecture(&selected_architecture);

        let result = ArchitectureSelectionResult {
            selected_architecture: selected_architecture.clone(),
            confidence_score: confidence,
            selection_reasoning: reasoning,
            architecture_config,
            fallback_architecture,
            performance_prediction,
        };

        info!("Architecture selected: {} (confidence: {:.2})", 
              selected_architecture, confidence);

        Ok(result)
    }

    /// 基于规则的架构选择
    fn rule_based_selection(&self, analysis: &QueryAnalysisResult) -> (String, String, f32) {
        let task_type = &analysis.task_type;
        let complexity = &analysis.complexity_level;
        let parallelization = &analysis.parallelization_potential;
        let steps = analysis.estimated_steps;

        // 架构选择逻辑
        let (architecture, reasoning, confidence) = match (task_type.as_str(), complexity.as_str(), parallelization.as_str()) {
            // LLMCompiler适用场景
            (_, _, "high") if steps > 5 => {
                ("LlmCompiler".to_string(), 
                 "任务具有高并行化潜力且步骤较多，LLMCompiler的流式DAG执行能充分利用并行性".to_string(), 
                 0.9)
            },
            ("扫描任务", "complex", _) => {
                ("LlmCompiler".to_string(),
                 "复杂扫描任务需要并行执行多个子任务，LLMCompiler的Task Fetching Unit能有效调度".to_string(),
                 0.85)
            },
            
            // ReAct适用场景
            ("调试任务" | "探索任务" | "信息收集", _, _) => {
                ("ReAct".to_string(),
                 "探索性任务需要迭代推理和工具调用，ReAct的Thought-Action-Observation循环能够适应性决策".to_string(),
                 0.85)
            },
            ("分析任务", "medium", _) if steps <= 5 => {
                ("ReAct".to_string(),
                 "中等复杂度的分析任务适合ReAct的交替推理执行模式，提供良好的可解释性".to_string(),
                 0.75)
            },
            (_, "medium", "low") if analysis.task_type.contains("研究") || analysis.task_type.contains("理解") => {
                ("ReAct".to_string(),
                 "研究和理解类任务需要灵活的推理过程，ReAct能够根据观察结果调整策略".to_string(),
                 0.7)
            },
            
            // ReWOO适用场景  
            ("分析任务", "medium" | "complex", _) if steps > 5 => {
                ("ReWoo".to_string(),
                 "复杂分析任务需要结构化推理链，ReWOO的Planner-Worker-Solver架构能提供高效的分析流程".to_string(),
                 0.8)
            },
            ("查询任务", "medium" | "complex", _) => {
                ("ReWoo".to_string(),
                 "复杂查询任务需要工具调用和推理，ReWOO架构能有效整合多个信息源".to_string(),
                 0.75)
            },
            
            // Plan-Execute适用场景
            ("配置任务", _, _) => {
                ("PlanAndExecute".to_string(),
                 "配置任务需要有序执行且可能需要重规划，Plan-Execute提供灵活的规划调整能力".to_string(),
                 0.8)
            },
            ("监控任务", _, _) => {
                ("PlanAndExecute".to_string(),
                 "监控任务通常需要持续执行和动态调整，Plan-Execute的重规划机制很适合".to_string(),
                 0.75)
            },
            (_, "simple", _) => {
                ("PlanAndExecute".to_string(),
                 "简单任务使用Plan-Execute可以快速规划和执行".to_string(),
                 0.7)
            },
            
            // 默认选择
            _ => {
                ("PlanAndExecute".to_string(),
                 "基于任务特征，Plan-Execute提供最稳定的执行保障".to_string(),
                 0.6)
            }
        };

        (architecture, reasoning, confidence)
    }

    /// 生成架构配置
    fn generate_architecture_config(&self, analysis: &QueryAnalysisResult, architecture: &str) -> ArchitectureConfiguration {
        let base_timeout = match analysis.complexity_level.as_str() {
            "simple" => 300,   // 5分钟
            "medium" => 900,   // 15分钟
            "complex" => 1800, // 30分钟
            _ => 600,
        };

        let max_parallel = match (architecture, analysis.parallelization_potential.as_str()) {
            ("LlmCompiler", "high") => 8,
            ("LlmCompiler", "medium") => 4,
            ("LlmCompiler", _) => 2,
            ("ReAct", _) => 1, // ReAct 是顺序迭代的
            ("ReWoo", "high") => 5,
            ("ReWoo", _) => 3,
            ("PlanAndExecute", "high") => 3,
            _ => 1,
        };

        let resource_limits = Some(ResourceLimits {
            cpu_cores: match analysis.resource_requirements.as_str() {
                "light" => 2,
                "medium" => 4,
                "heavy" => 8,
                _ => 4,
            },
            memory_gb: match analysis.resource_requirements.as_str() {
                "light" => 1,
                "medium" => 2,
                "heavy" => 4,
                _ => 2,
            },
            network_concurrent: max_parallel * 2,
            max_execution_time: base_timeout * 2,
        });

        let optimization_params = OptimizationParams {
            enable_streaming: architecture == "LlmCompiler" || architecture == "ReAct",
            enable_caching: analysis.complexity_level != "simple",
            enable_adaptive_scheduling: architecture == "PlanAndExecute" || architecture == "ReAct",
            priority_weight: match analysis.time_sensitivity.as_str() {
                "high" => 1.0,
                "medium" => 0.7,
                "low" => 0.5,
                _ => 0.7,
            },
        };

        ArchitectureConfiguration {
            max_parallel_tasks: max_parallel,
            timeout_per_task: base_timeout,
            retry_policy: if analysis.time_sensitivity == "high" {
                "fast_fail".to_string()
            } else {
                "exponential_backoff".to_string()
            },
            resource_limits,
            optimization_params,
        }
    }

    /// 性能预测
    fn predict_performance(&self, analysis: &QueryAnalysisResult, architecture: &str) -> PerformancePrediction {
        let base_time = analysis.estimated_steps as f64 * match analysis.complexity_level.as_str() {
            "simple" => 30,   // 30秒每步
            "medium" => 90,   // 1.5分钟每步
            "complex" => 180, // 3分钟每步
            _ => 60,
        };

        // 架构效率系数
        let efficiency_factor = match architecture {
            "LlmCompiler" if analysis.parallelization_potential == "high" => 0.4, // 高并行效率
            "LlmCompiler" => 0.7,
            "ReAct" => 0.85, // ReAct 迭代执行，比基准稍快
            "ReWoo" => 0.8,
            "PlanAndExecute" => 1.0, // 基准
            _ => 1.0,
        };

        let estimated_execution_time = (base_time as f32 * efficiency_factor) as f64;

        let estimated_success_rate = match (architecture, analysis.complexity_level.as_str()) {
            ("LlmCompiler", "complex") => 0.85,
            ("LlmCompiler", _) => 0.9,
            ("ReAct", "complex") => 0.75, // 探索性任务成功率略低
            ("ReAct", _) => 0.8,
            ("ReWoo", "complex") => 0.8,
            ("ReWoo", _) => 0.85,
            ("PlanAndExecute", _) => 0.9,
            _ => 0.8,
        };

        let parallel_efficiency = match (architecture, analysis.parallelization_potential.as_str()) {
            ("LlmCompiler", "high") => 0.8,
            ("LlmCompiler", "medium") => 0.6,
            ("ReAct", _) => 0.1, // ReAct 顺序执行，并行效率低
            ("ReWoo", "high") => 0.5,
            ("ReWoo", "medium") => 0.3,
            ("PlanAndExecute", "high") => 0.4,
            _ => 0.2,
        };

        let resource_consumption = ResourceConsumption {
            cpu_usage: match analysis.resource_requirements.as_str() {
                "light" => 0.3,
                "medium" => 0.6,
                "heavy" => 0.9,
                _ => 0.5,
            },
            memory_usage_mb: match analysis.resource_requirements.as_str() {
                "light" => 256,
                "medium" => 512,
                "heavy" => 1024,
                _ => 512,
            },
            network_bandwidth_mbps: if analysis.task_type.contains("扫描") { 10.0 } else { 2.0 },
        };

        PerformancePrediction {
            estimated_execution_time,
            estimated_success_rate,
            estimated_parallel_efficiency: parallel_efficiency,
            estimated_resource_consumption: resource_consumption,
        }
    }

    /// 选择备选架构
    fn select_fallback_architecture(&self, primary: &str) -> String {
        match primary {
            "LlmCompiler" => "PlanAndExecute".to_string(),
            "ReAct" => "PlanAndExecute".to_string(),
            "ReWoo" => "PlanAndExecute".to_string(), 
            "PlanAndExecute" => "ReWoo".to_string(),
            _ => "PlanAndExecute".to_string(),
        }
    }
}
