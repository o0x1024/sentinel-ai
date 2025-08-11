//! Task Fetching Unit - 智能任务调度单元
//!
//! 负责DAG任务的智能调度和依赖管理
//! 核心功能：
//! - 依赖解析：分析任务间的依赖关系
//! - 并行调度：将无依赖的任务并行执行
//! - 阻塞管理：等待依赖完成后再执行
//! - 变量替换：将$1等占位符替换为实际结果

use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info, warn, error, debug};

use super::types::*;

/// Task Fetching Unit - 智能任务调度单元
pub struct TaskFetchingUnit {
    /// 等待队列（等待依赖的任务）
    waiting_queue: Arc<RwLock<VecDeque<DagTaskNode>>>,
    /// 就绪队列（可立即执行的任务）
    ready_queue: Arc<RwLock<VecDeque<DagTaskNode>>>,
    /// 正在执行的任务
    executing_tasks: Arc<RwLock<HashMap<String, JoinHandle<TaskExecutionResult>>>>,
    /// 已完成的任务结果
    completed_tasks: Arc<RwLock<HashMap<String, TaskExecutionResult>>>,
    /// 失败的任务
    failed_tasks: Arc<RwLock<HashMap<String, TaskExecutionResult>>>,
    /// 依赖关系图
    dependency_graph: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 变量解析上下文
    variable_context: Arc<RwLock<VariableResolutionContext>>,
    /// 配置
    config: LlmCompilerConfig,
}

impl TaskFetchingUnit {
    pub fn new(config: LlmCompilerConfig) -> Self {
        Self {
            waiting_queue: Arc::new(RwLock::new(VecDeque::new())),
            ready_queue: Arc::new(RwLock::new(VecDeque::new())),
            executing_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            failed_tasks: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            variable_context: Arc::new(RwLock::new(VariableResolutionContext::new())),
            config,
        }
    }

    /// 初始化DAG计划
    pub async fn initialize_plan(&self, plan: &DagExecutionPlan) -> Result<()> {
        info!("初始化DAG计划: {} (版本: {})", plan.name, plan.version);

        // 清空所有队列
        self.clear_all_queues().await;

        // 设置依赖关系图
        {
            let mut deps = self.dependency_graph.write().await;
            *deps = plan.dependency_graph.clone();
        }

        // 设置变量映射
        {
            let mut context = self.variable_context.write().await;
            context.variable_mappings = plan.variable_mappings.clone();
            // 设置全局变量
            for (key, value) in &plan.global_config {
                context.global_variables.insert(key.clone(), value.clone());
            }
        }

        // 分析任务依赖并分配到相应队列
        let mut waiting_queue = self.waiting_queue.write().await;
        let mut ready_queue = self.ready_queue.write().await;

        for mut node in plan.nodes.clone() {
            if node.dependencies.is_empty() {
                // 无依赖任务直接进入就绪队列
                node.status = TaskStatus::Ready;
                ready_queue.push_back(node);
            } else {
                // 有依赖任务进入等待队列
                node.status = TaskStatus::Pending;
                waiting_queue.push_back(node);
            }
        }

        // 按优先级排序就绪队列
        let mut ready_tasks: Vec<_> = ready_queue.drain(..).collect();
        ready_tasks.sort_by_key(|task| task.priority);
        ready_queue.extend(ready_tasks);

        info!(
            "DAG初始化完成: {} 个就绪任务, {} 个等待任务",
            ready_queue.len(),
            waiting_queue.len()
        );

        Ok(())
    }

    /// 清空所有队列
    async fn clear_all_queues(&self) {
        self.waiting_queue.write().await.clear();
        self.ready_queue.write().await.clear();
        self.executing_tasks.write().await.clear();
        self.completed_tasks.write().await.clear();
        self.failed_tasks.write().await.clear();
    }

    /// 获取下一个可执行的任务
    pub async fn fetch_next_task(&self) -> Option<DagTaskNode> {
        let mut ready_queue = self.ready_queue.write().await;
        
        // 优先获取高优先级任务
        if let Some(task) = ready_queue.pop_front() {
            debug!("获取就绪任务: {} (优先级: {})", task.name, task.priority);
            Some(task)
        } else {
            None
        }
    }

    /// 获取多个可执行任务（用于批量并行执行）
    pub async fn fetch_ready_tasks(&self, max_count: usize) -> Vec<DagTaskNode> {
        let mut ready_queue = self.ready_queue.write().await;
        let mut tasks = Vec::new();
        
        for _ in 0..max_count {
            if let Some(task) = ready_queue.pop_front() {
                tasks.push(task);
            } else {
                break;
            }
        }
        
        if !tasks.is_empty() {
            info!("批量获取 {} 个就绪任务", tasks.len());
        }
        
        tasks
    }

    /// 标记任务开始执行
    pub async fn mark_task_executing(&self, task_id: String, handle: JoinHandle<TaskExecutionResult>) {
        let mut executing = self.executing_tasks.write().await;
        executing.insert(task_id.clone(), handle);
        debug!("任务开始执行: {}", task_id);
    }

    /// 标记任务完成并更新依赖
    pub async fn complete_task(&self, result: TaskExecutionResult) -> Result<()> {
        info!("任务完成: {} (状态: {:?})", result.task_id, result.status);

        // 从执行队列中移除
        {
            let mut executing = self.executing_tasks.write().await;
            executing.remove(&result.task_id);
        }

        // 根据执行状态保存结果
        match result.status {
            TaskStatus::Completed => {
                // 保存成功结果
                {
                    let mut completed = self.completed_tasks.write().await;
                    completed.insert(result.task_id.clone(), result.clone());
                }
                
                // 更新变量解析上下文
                {
                    let mut context = self.variable_context.write().await;
                    context.add_task_result(result.clone());
                }
                
                // 检查等待队列中的任务是否可以执行
                self.update_waiting_tasks().await?;
            }
            TaskStatus::Failed => {
                // 保存失败结果
                {
                    let mut failed = self.failed_tasks.write().await;
                    failed.insert(result.task_id.clone(), result.clone());
                }
                
                // 检查是否可以重试
                if result.retry_count < self.config.max_task_retries {
                    warn!("任务失败，准备重试: {} (重试次数: {})", result.task_id, result.retry_count + 1);
                    // 这里可以实现重试逻辑
                } else {
                    error!("任务最终失败: {} (已重试 {} 次)", result.task_id, result.retry_count);
                    // 可以选择取消依赖此任务的其他任务，或者继续执行
                }
            }
            _ => {
                warn!("收到非终态任务结果: {} (状态: {:?})", result.task_id, result.status);
            }
        }

        Ok(())
    }

    /// 更新等待队列，将满足依赖的任务移到就绪队列
    async fn update_waiting_tasks(&self) -> Result<()> {
        let mut waiting_queue = self.waiting_queue.write().await;
        let mut ready_queue = self.ready_queue.write().await;
        let completed = self.completed_tasks.read().await;

        let mut newly_ready = Vec::new();
        let mut still_waiting = VecDeque::new();

        while let Some(mut task) = waiting_queue.pop_front() {
            // 检查所有依赖是否已完成
            let all_deps_completed = task.dependencies.iter()
                .all(|dep_id| completed.contains_key(dep_id));

            if all_deps_completed {
                // 解析变量引用
                self.resolve_task_variables(&mut task).await?;
                task.status = TaskStatus::Ready;
                newly_ready.push(task);
            } else {
                still_waiting.push_back(task);
            }
        }

        // 更新队列
        *waiting_queue = still_waiting;
        
        // 按优先级排序新就绪的任务
        newly_ready.sort_by_key(|task| task.priority);
        
        for task in newly_ready {
            ready_queue.push_back(task);
        }

        if !ready_queue.is_empty() {
            debug!("更新后就绪队列有 {} 个任务", ready_queue.len());
        }

        Ok(())
    }

    /// 解析任务的变量引用
    async fn resolve_task_variables(&self, task: &mut DagTaskNode) -> Result<()> {
        let context = self.variable_context.read().await;
        
        // 解析输入参数中的变量引用
        for (key, value) in task.inputs.iter_mut() {
            if let Value::String(s) = value.clone() {
                if s.starts_with('$') {
                    if let Some(resolved_value) = context.resolve_variable(&s) {
                        *value = resolved_value;
                        debug!("解析变量 {} -> {:?}", s, value);
                    } else {
                        warn!("无法解析变量引用: {}", s);
                    }
                }
            }
        }
        
        // 清空变量引用列表（已解析）
        task.variable_refs.clear();
        
        Ok(())
    }

    /// 获取执行统计
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        let waiting_count = self.waiting_queue.read().await.len();
        let ready_count = self.ready_queue.read().await.len();
        let executing_count = self.executing_tasks.read().await.len();
        let completed_count = self.completed_tasks.read().await.len();
        let failed_count = self.failed_tasks.read().await.len();
        let total_count = waiting_count + ready_count + executing_count + completed_count + failed_count;

        ExecutionStats {
            waiting_tasks: waiting_count,
            ready_tasks: ready_count,
            executing_tasks: executing_count,
            completed_tasks: completed_count,
            failed_tasks: failed_count,
            total_tasks: total_count,
        }
    }

    /// 获取所有已完成的任务结果
    pub async fn get_completed_results(&self) -> Vec<TaskExecutionResult> {
        self.completed_tasks.read().await.values().cloned().collect()
    }

    /// 获取所有失败的任务结果
    pub async fn get_failed_results(&self) -> Vec<TaskExecutionResult> {
        self.failed_tasks.read().await.values().cloned().collect()
    }

    /// 检查是否还有任务需要执行
    pub async fn has_pending_tasks(&self) -> bool {
        let stats = self.get_execution_stats().await;
        stats.waiting_tasks > 0 || stats.ready_tasks > 0 || stats.executing_tasks > 0
    }

    /// 取消所有等待和就绪的任务
    pub async fn cancel_pending_tasks(&self) -> Result<()> {
        info!("取消所有待执行任务");
        
        // 清空等待队列和就绪队列
        self.waiting_queue.write().await.clear();
        self.ready_queue.write().await.clear();
        
        // 取消正在执行的任务
        let executing_tasks = self.executing_tasks.write().await;
        for (task_id, handle) in executing_tasks.iter() {
            warn!("取消执行中的任务: {}", task_id);
            handle.abort();
        }
        
        Ok(())
    }

    /// 获取任务依赖关系图的拓扑排序
    pub async fn get_topological_order(&self) -> Result<Vec<String>> {
        let dependency_graph = self.dependency_graph.read().await;
        self.topological_sort(&dependency_graph)
    }

    /// 拓扑排序算法
    fn topological_sort(&self, graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        
        // 构建邻接表和入度表
        for (node, deps) in graph {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                adj_list.entry(dep.clone()).or_insert_with(Vec::new).push(node.clone());
                *in_degree.entry(node.clone()).or_insert(0) += 1;
            }
        }
        
        // Kahn算法
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result: Vec<String> = Vec::new();
        
        // 找到所有入度为0的节点
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }
        
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            
            // 减少相邻节点的入度
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
        
        // 检查是否有环
        if result.len() != in_degree.len() {
            return Err(anyhow::anyhow!("检测到循环依赖"));
        }
        
        Ok(result)
    }

    /// 获取可以并行执行的任务组
    pub async fn get_parallel_task_groups(&self) -> Result<Vec<Vec<String>>> {
        let dependency_graph = self.dependency_graph.read().await;
        let topo_order = self.topological_sort(&dependency_graph)?;
        
        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut processed: HashSet<String> = HashSet::new();
        
        for task_id in topo_order.clone() {
            if processed.contains(&task_id) {
                continue;
            }
            
            // 找到所有可以与当前任务并行执行的任务
            let mut parallel_group = vec![task_id.clone()];
            processed.insert(task_id.clone());
            
            // 检查其他未处理的任务是否可以并行
            for other_task in &topo_order {
                if processed.contains(other_task) {
                    continue;
                }
                
                // 检查是否有依赖关系
                if !self.has_dependency_path(&dependency_graph, &task_id, other_task) &&
                   !self.has_dependency_path(&dependency_graph, other_task, &task_id) {
                    parallel_group.push(other_task.clone());
                    processed.insert(other_task.clone());
                }
            }
            
            groups.push(parallel_group);
        }
        
        Ok(groups)
    }

    /// 检查两个任务之间是否存在依赖路径
    fn has_dependency_path(
        &self,
        graph: &HashMap<String, Vec<String>>,
        from: &str,
        to: &str,
    ) -> bool {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = vec![from.to_string()];
        
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            if current == to {
                return true;
            }
            
            if let Some(deps) = graph.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        
        false
    }

    /// 获取任务的详细状态信息
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        // 检查各个队列中的任务状态
        
        // 检查等待队列
        let waiting_queue = self.waiting_queue.read().await;
        if waiting_queue.iter().any(|task| task.id == task_id) {
            return Some(TaskStatus::Pending);
        }
        
        // 检查就绪队列
        let ready_queue = self.ready_queue.read().await;
        if ready_queue.iter().any(|task| task.id == task_id) {
            return Some(TaskStatus::Ready);
        }
        
        // 检查执行中任务
        let executing_tasks = self.executing_tasks.read().await;
        if executing_tasks.contains_key(task_id) {
            return Some(TaskStatus::Running);
        }
        
        // 检查已完成任务
        let completed_tasks = self.completed_tasks.read().await;
        if completed_tasks.contains_key(task_id) {
            return Some(TaskStatus::Completed);
        }
        
        // 检查失败任务
        let failed_tasks = self.failed_tasks.read().await;
        if failed_tasks.contains_key(task_id) {
            return Some(TaskStatus::Failed);
        }
        
        None
    }
}