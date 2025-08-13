//! LLMCompiler Engine - 智能工具编排引擎
//!
//! 基于LangGraph LLMCompiler架构实现的智能工具编排系统
//! 这是主引擎文件，负责协调各个组件的工作

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use chrono::Utc;

use crate::services::ai::AiService;
use crate::tools::ToolSystem;
use crate::services::database::DatabaseService;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};

use super::types::*;
use super::planner::LlmCompilerPlanner;
use super::task_fetcher::TaskFetchingUnit;
use super::executor::ParallelExecutorPool;
use super::joiner::IntelligentJoiner;

/// LLMCompiler引擎 - 主协调器
pub struct LlmCompilerEngine {
    /// Planner组件 - 智能规划器
    planner: Arc<LlmCompilerPlanner>,
    /// Task Fetching Unit - 任务调度单元
    task_fetcher: Arc<TaskFetchingUnit>,
    /// Parallel Executor Pool - 并行执行器池
    executor_pool: Arc<ParallelExecutorPool>,
    /// Intelligent Joiner - 智能决策器
    joiner: Arc<tokio::sync::Mutex<IntelligentJoiner>>,
    /// AI服务 - 用于生成最终响应
    ai_service: Arc<AiService>,
    /// 配置
    config: LlmCompilerConfig,
    /// 动态Prompt仓库
    prompt_repo: Option<PromptRepository>,
}

impl LlmCompilerEngine {
    /// 创建新的LLMCompiler引擎实例
    pub fn new(
        ai_service: Arc<AiService>,
        tool_system: Arc<ToolSystem>,
        config: LlmCompilerConfig,
        db_service: Arc<DatabaseService>,
    ) -> Self {
        // 初始化各个组件
        let pool = db_service.get_pool().expect("DB pool not initialized");
        let planner = Arc::new(LlmCompilerPlanner::new(
            ai_service.clone(),
            tool_system.clone(),
            config.clone(),
            Some(PromptRepository::new(pool.clone())),
        ));
        
        let task_fetcher = Arc::new(TaskFetchingUnit::new(config.clone()));
        
        let executor_pool = Arc::new(ParallelExecutorPool::new(
            tool_system.clone(),
            config.clone(),
        ));
        
        let joiner = Arc::new(tokio::sync::Mutex::new(IntelligentJoiner::new(
            (*ai_service).clone(),
            config.clone(),
            Some(PromptRepository::new(pool.clone())),
        )));
        
        Self {
            planner,
            task_fetcher,
            executor_pool,
            joiner,
            ai_service,
            config,
            prompt_repo: Some(PromptRepository::new(pool.clone())),
        }
    }

    /// 执行工作流 - 主入口点
    pub async fn execute_workflow(
        &self,
        user_query: &str,
        context: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<WorkflowExecutionResult> {
        info!("开始执行LLMCompiler工作流: {}", user_query);
        
        let context = context.unwrap_or_default();
        let mut execution_summary = ExecutionSummary {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_duration_ms: 0,
            replanning_count: 0,
            key_findings: Vec::new(),
            efficiency_metrics: EfficiencyMetrics {
                average_parallelism: 0.0,
                resource_utilization: 0.0,
                task_success_rate: 0.0,
                average_task_duration_ms: 0.0,
            },
        };
        
        let workflow_start = std::time::Instant::now();
        let mut current_plan: Option<DagExecutionPlan> = None;
        let mut all_results: Vec<TaskExecutionResult> = Vec::new();
        
        // 主执行循环
        for round in 1..=self.config.max_iterations {
            info!("开始执行轮次: {}/{}", round, self.config.max_iterations);
            // 更新总轮次（这里用replanning_count来跟踪）
            if round > 1 {
                execution_summary.replanning_count = round - 1;
            }
            
            // 1. 规划阶段
            let execution_plan = if let Some(ref plan) = current_plan {
                // 如果有现有计划，检查是否需要重新规划
                if self.config.enable_replanning && round > 1 {
                    info!("检查是否需要重新规划...");
                    // 这里可以添加重新规划的逻辑
                    plan.clone()
                } else {
                    plan.clone()
                }
            } else {
                // 生成初始计划
                info!("生成初始执行计划...");
                let plan = self.planner.generate_dag_plan(user_query, &context).await?;
                current_plan = Some(plan.clone());
                plan
            };
            
            info!("执行计划包含 {} 个任务", execution_plan.nodes.len());
            
            // 2. 任务调度阶段
            self.task_fetcher.initialize_plan(&execution_plan).await?;
            
            // 3. 并行执行阶段
            let round_results = self.execute_parallel_round().await?;
            
            if round_results.is_empty() {
                warn!("轮次 {} 没有执行任何任务", round);
                break;
            }
            
            // 更新统计信息
            let completed_count = round_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
            let failed_count = round_results.iter().filter(|r| r.status == TaskStatus::Failed).count();
            
            execution_summary.successful_tasks += completed_count;
            execution_summary.failed_tasks += failed_count;
            execution_summary.total_tasks += round_results.len();
            
            let round_duration: u64 = round_results.iter().map(|r| r.duration_ms).sum();
            execution_summary.total_duration_ms += round_duration;
            
            all_results.extend(round_results.clone());
            
            info!(
                "轮次 {} 完成: 成功 {} 个任务, 失败 {} 个任务, 耗时 {}ms",
                round, completed_count, failed_count, round_duration
            );
            
            // 4. 智能决策阶段
            let mut joiner = self.joiner.lock().await;
            let decision = joiner.analyze_and_decide(
                user_query,
                &execution_plan,
                &round_results,
                round,
            ).await?;
            
            // 记录决策信息（可以添加到key_findings中）
            match &decision {
                JoinerDecision::Complete { response, .. } => {
                    execution_summary.key_findings.push(format!("决策: 完成执行 - {}", response));
                }
                JoinerDecision::Continue { feedback, .. } => {
                    execution_summary.key_findings.push(format!("决策: 继续执行 - {}", feedback));
                }
            }
            
            match decision {
                JoinerDecision::Complete { response, .. } => {
                    info!("Joiner决定完成执行: {}", response);
                    break;
                }
                JoinerDecision::Continue { feedback, .. } => {
                    info!("Joiner决定继续执行: {}", feedback);
                    // 继续下一轮
                }
            }
            
            // 检查是否还有待执行的任务
            if !self.task_fetcher.has_pending_tasks().await {
                info!("所有任务已完成，结束执行");
                break;
            }
        }
        
        let total_duration = workflow_start.elapsed();
        execution_summary.total_duration_ms = total_duration.as_millis() as u64;
        
        // 计算最终效率指标
        execution_summary.efficiency_metrics = self.calculate_efficiency_metrics(&execution_summary);
        
        info!(
            "工作流执行完成: {} 个任务, 成功 {}, 失败 {}, 耗时 {}ms",
            execution_summary.total_tasks,
            execution_summary.successful_tasks,
            execution_summary.failed_tasks,
            execution_summary.total_duration_ms
        );
        
        // 生成最终结果
        let final_response = self.generate_final_response(
            user_query,
            &all_results,
            &execution_summary,
        ).await?;
        
        Ok(WorkflowExecutionResult {
            success: true,
            response: final_response,
            execution_summary: execution_summary.clone(),
            task_results: all_results,
            efficiency_metrics: execution_summary.efficiency_metrics,
            error: None,
        })
    }

    /// 执行一轮并行任务
    async fn execute_parallel_round(&self) -> Result<Vec<TaskExecutionResult>> {
        let mut results = Vec::new();
        
        // 获取所有就绪的任务
        let ready_tasks = self.task_fetcher.fetch_ready_tasks(self.config.max_concurrency).await;
        
        if ready_tasks.is_empty() {
            return Ok(results);
        }
        
        info!("开始并行执行 {} 个就绪任务", ready_tasks.len());
        
        // 标记任务为执行中并启动执行
        let mut handles = Vec::new();
        for task in ready_tasks {
            let executor = self.executor_pool.clone();
            let handle = tokio::spawn(async move {
                executor.execute_task(task).await
            });
            handles.push(handle);
        }
        
        // 等待所有任务完成
        for handle in handles {
            match handle.await {
                Ok(result) => {
                    // 更新任务状态
                    self.task_fetcher.complete_task(result.clone()).await?;
                    results.push(result);
                }
                Err(e) => {
                    error!("任务执行句柄错误: {}", e);
                }
            }
        }
        
        Ok(results)
    }

    /// 生成最终响应
    async fn generate_final_response(
        &self,
        user_query: &str,
        task_results: &[TaskExecutionResult],
        execution_summary: &ExecutionSummary,
    ) -> Result<String> {
        // 收集所有成功任务的输出
        let successful_outputs: Vec<&std::collections::HashMap<String, serde_json::Value>> = task_results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .map(|r| &r.outputs)
            .collect();
        
        if successful_outputs.is_empty() {
            return Ok("抱歉，没有成功执行任何任务，无法提供有效结果。".to_string());
        }
        
        // 构建响应生成提示
        // 报告生成 Prompt：优先从动态模板(LLMCompiler/Report or Execution)获取
        let response_prompt = if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(dynamic)) = repo.get_active_prompt(ArchitectureType::LLMCompiler, StageType::Execution).await {
                dynamic
            } else {
                self.build_default_response_prompt(user_query, &successful_outputs, execution_summary)
            }
        } else {
            self.build_default_response_prompt(user_query, &successful_outputs, execution_summary)
        };
        
        // 调用AI生成最终响应
        info!("开始调用AI生成最终响应，提示词长度: {} 字符", response_prompt.len());
        debug!("AI响应生成提示词: {}", response_prompt);
        
        match self.ai_service.send_message_stream_with_prompt(&response_prompt, None, None).await {
            Ok(ai_response) => {
                if ai_response.trim().is_empty() {
                    warn!("AI返回了空响应，使用默认响应");
                    Ok(self.generate_default_response(task_results, execution_summary))
                } else {
                    info!("AI响应生成成功，响应长度: {} 字符", ai_response.len());
                    Ok(ai_response)
                }
            }
            Err(e) => {
                error!("AI响应生成失败: {}, 使用默认响应", e);
                Ok(self.generate_default_response(task_results, execution_summary))
            }
        }
    }

    fn build_default_response_prompt(
        &self,
        user_query: &str,
        successful_outputs: &[&std::collections::HashMap<String, serde_json::Value>],
        execution_summary: &ExecutionSummary,
    ) -> String {
        format!(
            r#"你是一个专业的安全分析专家，请基于以下LLMCompiler工作流执行结果，为用户查询生成一个完整、准确、专业的分析报告。

**用户查询**: {}

**执行统计**:
- 总任务数: {}
- 成功任务: {}
- 失败任务: {}
- 总耗时: {}ms
- 任务成功率: {:.1}%

**执行结果详情**:
{}

{}"#,
            user_query,
            execution_summary.total_tasks,
            execution_summary.successful_tasks,
            execution_summary.failed_tasks,
            execution_summary.total_duration_ms,
            if execution_summary.total_tasks > 0 {
                (execution_summary.successful_tasks as f32 / execution_summary.total_tasks as f32) * 100.0
            } else {
                0.0
            },
            self.format_outputs_for_response(&successful_outputs),
            if execution_summary.failed_tasks > 0 {
                format!("\n## ❌ 执行问题\n本次执行中有 {} 个任务失败，请在报告中说明可能的原因和影响。", execution_summary.failed_tasks)
            } else {
                String::new()
            }
        )
    }

    /// 格式化输出用于响应生成
    fn format_outputs_for_response(
        &self,
        outputs: &[&std::collections::HashMap<String, serde_json::Value>],
    ) -> String {
        if outputs.is_empty() {
            return "暂无成功执行的任务输出".to_string();
        }

        outputs
            .iter()
            .enumerate()
            .map(|(i, output)| {
                let mut formatted_output = Vec::new();
                
                // 格式化每个输出字段
                for (key, value) in output.iter() {
                    let formatted_value = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Array(arr) => {
                            if arr.len() <= 5 {
                                format!("[{}]", arr.iter()
                                    .map(|v| match v {
                                        serde_json::Value::String(s) => s.clone(),
                                        _ => v.to_string()
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", "))
                            } else {
                                format!("[{} 个项目]", arr.len())
                            }
                        },
                        serde_json::Value::Object(_) => "[对象数据]".to_string(),
                        serde_json::Value::Null => "null".to_string(),
                    };
                    
                    formatted_output.push(format!("  - {}: {}", key, formatted_value));
                }
                
                format!(
                    "### 任务 {}\n{}",
                    i + 1,
                    formatted_output.join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// 生成默认响应（当AI响应失败时）
    fn generate_default_response(
        &self,
        task_results: &[TaskExecutionResult],
        execution_summary: &ExecutionSummary,
    ) -> String {
        let successful_tasks = task_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let failed_tasks = task_results.iter().filter(|r| r.status == TaskStatus::Failed).count();
        
        format!(
            "执行完成！\n\n\
            📊 执行统计:\n\
            - 总任务: {}\n\
            - 成功任务: {}\n\
            - 失败任务: {}\n\
            - 总耗时: {}ms\n\n\
            📋 主要结果:\n\
            {}\n\n\
            {}",
            execution_summary.total_tasks,
            successful_tasks,
            failed_tasks,
            execution_summary.total_duration_ms,
            self.summarize_task_outputs(task_results),
            if failed_tasks > 0 {
                "⚠️ 部分任务执行失败，可能需要检查输入参数或网络连接。"
            } else {
                "✅ 所有任务执行成功！"
            }
        )
    }

    /// 总结任务输出
    fn summarize_task_outputs(&self, task_results: &[TaskExecutionResult]) -> String {
        let mut summaries = Vec::new();
        
        for result in task_results {
            if result.status == TaskStatus::Completed {
                let summary = if let Some(result_value) = result.outputs.get("result") {
                    format!("- {}: {}", result.task_id, result_value)
                } else {
                    format!("- {}: 执行成功", result.task_id)
                };
                summaries.push(summary);
            } else if result.status == TaskStatus::Failed {
                let error_msg = result.error.as_deref().unwrap_or("未知错误");
                summaries.push(format!("- {}: 执行失败 - {}", result.task_id, error_msg));
            }
        }
        
        if summaries.is_empty() {
            "无可用结果".to_string()
        } else {
            summaries.join("\n")
        }
    }

    /// 计算效率指标
    fn calculate_efficiency_metrics(&self, execution_summary: &ExecutionSummary) -> EfficiencyMetrics {
        let total_tasks = execution_summary.total_tasks;
        
        let task_success_rate = if total_tasks > 0 {
            execution_summary.successful_tasks as f32 / total_tasks as f32
        } else {
            0.0
        };
        
        let average_task_duration_ms = if execution_summary.successful_tasks > 0 {
            execution_summary.total_duration_ms as f32 / execution_summary.successful_tasks as f32
        } else {
            0.0
        };
        
        // 简化的并行效率计算（基于重规划次数+1作为轮次）
        let total_rounds = execution_summary.replanning_count + 1;
        let average_parallelism = if total_rounds > 0 {
            total_tasks as f32 / total_rounds as f32
        } else {
            0.0
        }.min(self.config.max_concurrency as f32);
        
        let resource_utilization = if self.config.max_concurrency > 0 {
            average_parallelism / self.config.max_concurrency as f32
        } else {
            0.0
        }.min(1.0);
        
        EfficiencyMetrics {
            average_parallelism,
            resource_utilization,
            task_success_rate,
            average_task_duration_ms,
        }
    }

    /// 获取引擎状态
    pub async fn get_engine_status(&self) -> EngineStatus {
        let task_stats = self.task_fetcher.get_execution_stats().await;
        // 暂时注释掉私有类型的调用
        // let executor_metrics = self.executor_pool.get_execution_metrics().await;
        
        EngineStatus {
            is_running: task_stats.executing_tasks > 0,
            pending_tasks: task_stats.waiting_tasks + task_stats.ready_tasks,
            executing_tasks: task_stats.executing_tasks,
            completed_tasks: task_stats.completed_tasks,
            failed_tasks: task_stats.failed_tasks,
            available_capacity: self.executor_pool.available_permits(),
            total_capacity: self.config.max_concurrency,
        }
    }

    /// 取消当前执行
    pub async fn cancel_execution(&self) -> Result<()> {
        info!("取消当前执行");
        self.task_fetcher.cancel_pending_tasks().await?;
        Ok(())
    }

    /// 重置引擎状态
    pub async fn reset(&self) -> Result<()> {
        info!("重置引擎状态");
        // 清空任务队列
        self.task_fetcher.cancel_pending_tasks().await?;
        // 注意：joiner没有reset方法，这里可以重新创建或者添加reset方法
        Ok(())
    }
}

/// 工作流执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 最终响应
    pub response: String,
    /// 执行摘要
    pub execution_summary: ExecutionSummary,
    /// 任务结果列表
    pub task_results: Vec<TaskExecutionResult>,
    /// 效率指标
    pub efficiency_metrics: EfficiencyMetrics,
    /// 错误信息
    pub error: Option<String>,
}

/// 引擎状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineStatus {
    /// 是否正在运行
    pub is_running: bool,
    /// 待执行任务数
    pub pending_tasks: usize,
    /// 执行中任务数
    pub executing_tasks: usize,
    /// 已完成任务数
    pub completed_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 可用执行容量
    pub available_capacity: usize,
    /// 总执行容量
    pub total_capacity: usize,
}