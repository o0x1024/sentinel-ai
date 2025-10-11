//! 负载均衡模块
//! 
//! 负责智能调度器的负载均衡，包括资源监控、任务分配策略等

use std::collections::HashMap;

use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;
use log::{info, warn};

use super::task_queue::{TaskItem, ResourceRequirements};

/// 执行节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionNode {
    /// 节点ID
    pub id: String,
    /// 节点名称
    pub name: String,
    /// 节点状态
    pub status: NodeStatus,
    /// 资源容量
    pub capacity: NodeCapacity,
    /// 当前资源使用情况
    pub current_usage: ResourceUsage,
    /// 运行中的任务
    pub running_tasks: Vec<String>,
    /// 节点性能指标
    pub performance_metrics: PerformanceMetrics,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 维护中
    Maintenance,
    /// 过载
    Overloaded,
    /// 故障
    Faulty,
}

/// 节点容量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    /// CPU核心数
    pub cpu_cores: u32,
    /// 内存容量 (GB)
    pub memory_gb: u32,
    /// 网络带宽 (Mbps)
    pub network_mbps: f32,
    /// 存储空间 (GB)
    pub storage_gb: u32,
    /// 最大并发任务数
    pub max_concurrent_tasks: u32,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU使用率 (0.0-1.0)
    pub cpu_usage: f32,
    /// 内存使用量 (GB)
    pub memory_used_gb: f32,
    /// 网络使用率 (0.0-1.0)
    pub network_usage: f32,
    /// 存储使用量 (GB)
    pub storage_used_gb: f32,
    /// 当前并发任务数
    pub concurrent_tasks: u32,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 平均响应时间 (ms)
    pub avg_response_time_ms: f64,
    /// 任务完成率
    pub task_completion_rate: f32,
    /// 错误率
    pub error_rate: f32,
    /// 吞吐量 (任务/小时)
    pub throughput: f64,
    /// 可用性 (0.0-1.0)
    pub availability: f32,
}

/// 负载均衡策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// 轮询
    RoundRobin,
    /// 最少连接
    LeastConnections,
    /// 加权轮询
    WeightedRoundRobin,
    /// 资源优先
    ResourceBased,
    /// 性能优先
    PerformanceBased,
    /// 智能调度
    Intelligent,
}

/// 负载均衡配置
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// 负载均衡策略
    pub strategy: LoadBalancingStrategy,
    /// 健康检查间隔 (秒)
    pub health_check_interval: f64,
    /// 节点超载阈值
    pub overload_threshold: f32,
    /// 启用自动故障转移
    pub enable_failover: bool,
    /// 启用预测性调度
    pub enable_predictive_scheduling: bool,
    /// 权重配置
    pub node_weights: HashMap<String, f32>,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::Intelligent,
            health_check_interval: 30,
            overload_threshold: 0.8,
            enable_failover: true,
            enable_predictive_scheduling: true,
            node_weights: HashMap::new(),
        }
    }
}

/// 负载均衡器
pub struct LoadBalancer {
    /// 执行节点
    nodes: RwLock<HashMap<String, ExecutionNode>>,
    /// 配置
    config: LoadBalancerConfig,
    /// 轮询计数器
    round_robin_counter: RwLock<usize>,
    /// 负载均衡统计
    statistics: RwLock<LoadBalancerStatistics>,
}

/// 负载均衡统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStatistics {
    /// 总任务分配数
    pub total_assignments: f64,
    /// 成功分配数
    pub successful_assignments: f64,
    /// 失败分配数
    pub failed_assignments: f64,
    /// 节点故障转移次数
    pub failover_count: f64,
    /// 平均负载分布
    pub load_distribution: HashMap<String, f32>,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    pub fn new(config: Option<LoadBalancerConfig>) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            config: config.unwrap_or_default(),
            round_robin_counter: RwLock::new(0),
            statistics: RwLock::new(LoadBalancerStatistics {
                total_assignments: 0,
                successful_assignments: 0,
                failed_assignments: 0,
                failover_count: 0,
                load_distribution: HashMap::new(),
                last_updated: Utc::now(),
            }),
        }
    }

    /// 注册执行节点
    pub async fn register_node(&self, node: ExecutionNode) -> Result<()> {
        info!("Registering execution node: {} ({})", node.name, node.id);
        
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.id.clone(), node);
        
        Ok(())
    }

    /// 注销执行节点
    pub async fn unregister_node(&self, node_id: &str) -> Result<()> {
        info!("Unregistering execution node: {}", node_id);
        
        let mut nodes = self.nodes.write().await;
        if nodes.remove(node_id).is_some() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found: {}", node_id))
        }
    }

    /// 选择最适合的执行节点
    pub async fn select_node(&self, task: &TaskItem) -> Result<String> {
        let nodes = self.nodes.read().await;
        
        if nodes.is_empty() {
            return Err(anyhow::anyhow!("No execution nodes available"));
        }

        let available_nodes: Vec<&ExecutionNode> = nodes.values()
            .filter(|node| self.is_node_available(node, &task.resource_requirements))
            .collect();

        if available_nodes.is_empty() {
            return Err(anyhow::anyhow!("No suitable nodes available for task"));
        }

        let selected_node = match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(&available_nodes).await
            },
            LoadBalancingStrategy::LeastConnections => {
                self.select_least_connections(&available_nodes)
            },
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(&available_nodes).await
            },
            LoadBalancingStrategy::ResourceBased => {
                self.select_resource_based(&available_nodes, &task.resource_requirements)
            },
            LoadBalancingStrategy::PerformanceBased => {
                self.select_performance_based(&available_nodes)
            },
            LoadBalancingStrategy::Intelligent => {
                self.select_intelligent(&available_nodes, task)
            },
        };

        // 更新统计信息
        self.update_assignment_statistics(&selected_node.id, true).await;

        info!("Selected node {} for task {}", selected_node.name, task.id);
        Ok(selected_node.id.clone())
    }

    /// 更新节点资源使用情况
    pub async fn update_node_usage(&self, node_id: &str, usage: ResourceUsage) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            node.current_usage = usage;
            node.last_updated = Utc::now();
            
            // 检查是否过载
            if self.is_node_overloaded(node) {
                node.status = NodeStatus::Overloaded;
                warn!("Node {} is overloaded", node.name);
            } else if node.status == NodeStatus::Overloaded {
                node.status = NodeStatus::Online;
                info!("Node {} is no longer overloaded", node.name);
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found: {}", node_id))
        }
    }

    /// 检查节点是否可用
    fn is_node_available(&self, node: &ExecutionNode, requirements: &ResourceRequirements) -> bool {
        if node.status != NodeStatus::Online {
            return false;
        }

        // 检查并发任务限制
        if node.current_usage.concurrent_tasks >= node.capacity.max_concurrent_tasks {
            return false;
        }

        // 检查资源容量
        let cpu_available = 1.0 - node.current_usage.cpu_usage;
        let memory_available = node.capacity.memory_gb as f32 - node.current_usage.memory_used_gb;
        let network_available = 1.0 - node.current_usage.network_usage;

        cpu_available >= requirements.cpu &&
        memory_available >= requirements.memory_mb as f32 / 1024.0 &&
        network_available >= requirements.network_mbps / 100.0 // 假设100Mbps为满载
    }

    /// 检查节点是否过载
    fn is_node_overloaded(&self, node: &ExecutionNode) -> bool {
        node.current_usage.cpu_usage > self.config.overload_threshold ||
        node.current_usage.memory_used_gb / node.capacity.memory_gb as f32 > self.config.overload_threshold ||
        node.current_usage.network_usage > self.config.overload_threshold
    }

    /// 轮询选择
    async fn select_round_robin<'a>(&self, nodes: &[&'a ExecutionNode]) -> &'a ExecutionNode {
        let mut counter = self.round_robin_counter.write().await;
        let index = *counter % nodes.len();
        *counter += 1;
        nodes[index]
    }

    /// 最少连接选择
    fn select_least_connections<'a>(&self, nodes: &[&'a ExecutionNode]) -> &'a ExecutionNode {
        nodes.iter()
            .min_by_key(|node| node.current_usage.concurrent_tasks)
            .unwrap()
    }

    /// 加权轮询选择
    async fn select_weighted_round_robin<'a>(&self, nodes: &[&'a ExecutionNode]) -> &'a ExecutionNode {
        // 简化实现：基于CPU容量权重
        let total_weight: u32 = nodes.iter().map(|node| node.capacity.cpu_cores).sum();
        let mut counter = self.round_robin_counter.write().await;
        let target = *counter % total_weight as usize;
        *counter += 1;

        let mut current_weight = 0;
        for node in nodes {
            current_weight += node.capacity.cpu_cores;
            if current_weight as usize > target {
                return node;
            }
        }

        nodes[0] // 回退
    }

    /// 基于资源选择
    fn select_resource_based<'a>(&self, nodes: &[&'a ExecutionNode], requirements: &ResourceRequirements) -> &'a ExecutionNode {
        nodes.iter()
            .max_by(|a, b| {
                let score_a = self.calculate_resource_score(a, requirements);
                let score_b = self.calculate_resource_score(b, requirements);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
    }

    /// 基于性能选择
    fn select_performance_based<'a>(&self, nodes: &[&'a ExecutionNode]) -> &'a ExecutionNode {
        nodes.iter()
            .max_by(|a, b| {
                let score_a = self.calculate_performance_score(a);
                let score_b = self.calculate_performance_score(b);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
    }

    /// 智能选择
    fn select_intelligent<'a>(&self, nodes: &[&'a ExecutionNode], task: &TaskItem) -> &'a ExecutionNode {
        nodes.iter()
            .max_by(|a, b| {
                let score_a = self.calculate_intelligent_score(a, task);
                let score_b = self.calculate_intelligent_score(b, task);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
    }

    /// 计算资源匹配分数
    fn calculate_resource_score(&self, node: &ExecutionNode, requirements: &ResourceRequirements) -> f32 {
        let cpu_score = (1.0 - node.current_usage.cpu_usage) / requirements.cpu;
        let memory_available = node.capacity.memory_gb as f32 - node.current_usage.memory_used_gb;
        let memory_score = memory_available / (requirements.memory_mb as f32 / 1024.0);
        let network_score = (1.0 - node.current_usage.network_usage) / (requirements.network_mbps / 100.0);

        (cpu_score + memory_score + network_score) / 3.0
    }

    /// 计算性能分数
    fn calculate_performance_score(&self, node: &ExecutionNode) -> f32 {
        let response_time_score = 1.0 / (node.performance_metrics.avg_response_time_ms / 1000.0 + 1.0) as f32;
        let completion_rate_score = node.performance_metrics.task_completion_rate;
        let error_rate_score = 1.0 - node.performance_metrics.error_rate;
        let availability_score = node.performance_metrics.availability;

        (response_time_score + completion_rate_score + error_rate_score + availability_score) / 4.0
    }

    /// 计算智能分数
    fn calculate_intelligent_score(&self, node: &ExecutionNode, task: &TaskItem) -> f32 {
        let resource_score = self.calculate_resource_score(node, &task.resource_requirements);
        let performance_score = self.calculate_performance_score(node);
        let load_score = 1.0 - (node.current_usage.concurrent_tasks as f32 / node.capacity.max_concurrent_tasks as f32);
        
        // 根据任务优先级调整权重
        let priority_weight = match task.priority {
            super::task_queue::TaskPriority::Critical => 1.0,
            super::task_queue::TaskPriority::High => 0.8,
            super::task_queue::TaskPriority::Normal => 0.6,
            super::task_queue::TaskPriority::Low => 0.4,
        };

        (resource_score * 0.4 + performance_score * 0.4 + load_score * 0.2) * priority_weight
    }

    /// 更新分配统计
    async fn update_assignment_statistics(&self, node_id: &str, success: bool) {
        let mut stats = self.statistics.write().await;
        stats.total_assignments += 1;
        
        if success {
            stats.successful_assignments += 1;
        } else {
            stats.failed_assignments += 1;
        }

        *stats.load_distribution.entry(node_id.to_string()).or_insert(0.0) += 1.0;
        stats.last_updated = Utc::now();
    }

    /// 获取负载均衡统计
    pub async fn get_statistics(&self) -> LoadBalancerStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    /// 获取所有节点状态
    pub async fn get_all_nodes(&self) -> Vec<ExecutionNode> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// 健康检查
    pub async fn health_check(&self) -> u32 {
        let mut unhealthy_count = 0;
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(self.config.health_check_interval as i64 * 2);

        let mut nodes = self.nodes.write().await;
        for node in nodes.values_mut() {
            // 检查节点是否长时间未更新
            if now.signed_duration_since(node.last_updated) > timeout {
                if node.status == NodeStatus::Online {
                    node.status = NodeStatus::Offline;
                    unhealthy_count += 1;
                    warn!("Node {} marked as offline due to timeout", node.name);
                }
            }
        }

        unhealthy_count
    }
}
