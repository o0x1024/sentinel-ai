//! Prompt A/B测试管理器
//! 
//! 实现prompt效果的对比测试和优化功能，支持：
//! - A/B测试设计和执行
//! - 效果评估和统计分析
//! - 自动优化建议
//! - 测试结果持久化

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A/B测试管理器
#[derive(Debug)]
pub struct PromptABTestManager {
    /// 活跃测试
    active_tests: Arc<RwLock<HashMap<String, ABTest>>>,
    /// 测试结果存储
    results_storage: TestResultsStorage,
    /// 统计分析器
    analyzer: StatisticalAnalyzer,
    /// 配置
    config: ABTestConfig,
}

/// A/B测试
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    /// 测试ID
    pub test_id: String,
    /// 测试名称
    pub name: String,
    /// 测试描述
    pub description: String,
    /// 测试状态
    pub status: TestStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 测试变体
    pub variants: Vec<TestVariant>,
    /// 流量分配
    pub traffic_allocation: TrafficAllocation,
    /// 评估指标
    pub metrics: Vec<EvaluationMetric>,
    /// 测试条件
    pub conditions: TestConditions,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 测试状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    /// 草稿
    Draft,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
}

/// 测试变体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestVariant {
    /// 变体ID
    pub variant_id: String,
    /// 变体名称
    pub name: String,
    /// 变体描述
    pub description: String,
    /// 是否为控制组
    pub is_control: bool,
    /// Prompt配置
    pub prompt_config: PromptConfig,
    /// 流量权重
    pub traffic_weight: f32,
    /// 变体特定配置
    pub variant_config: HashMap<String, serde_json::Value>,
}

/// 流量分配
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficAllocation {
    /// 分配策略
    pub strategy: AllocationStrategy,
    /// 总流量百分比
    pub total_traffic_percent: f32,
    /// 变体权重
    pub variant_weights: HashMap<String, f32>,
    /// 用户分组规则
    pub user_segmentation: Option<UserSegmentation>,
}

/// 分配策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// 随机分配
    Random,
    /// 基于用户ID哈希
    UserIdHash,
    /// 基于会话ID
    SessionId,
    /// 自定义规则
    Custom(String),
}

/// 用户分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSegmentation {
    /// 分组规则
    pub rules: Vec<SegmentationRule>,
    /// 默认分组
    pub default_segment: String,
}

/// 分组规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationRule {
    /// 规则名称
    pub name: String,
    /// 条件
    pub condition: String,
    /// 目标分组
    pub target_segment: String,
    /// 优先级
    pub priority: i32,
}

/// 评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 目标值
    pub target_value: Option<f64>,
    /// 权重
    pub weight: f32,
    /// 是否为主要指标
    pub is_primary: bool,
    /// 计算方法
    pub calculation_method: CalculationMethod,
}

/// 指标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// 成功率
    SuccessRate,
    /// 响应时间
    ResponseTime,
    /// 准确性
    Accuracy,
    /// 完整性
    Completeness,
    /// 用户满意度
    UserSatisfaction,
    /// 转化率
    ConversionRate,
    /// 自定义指标
    Custom(String),
}

/// 计算方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalculationMethod {
    /// 平均值
    Average,
    /// 中位数
    Median,
    /// 百分位数
    Percentile(f32),
    /// 总和
    Sum,
    /// 计数
    Count,
    /// 比率
    Ratio,
}

/// 测试条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConditions {
    /// 最小样本大小
    pub min_sample_size: usize,
    /// 最大运行时间（小时）
    pub max_duration_hours: Option<u64>,
    /// 置信水平
    pub confidence_level: f32,
    /// 最小检测效应
    pub minimum_detectable_effect: f32,
    /// 早停规则
    pub early_stopping_rules: Vec<EarlyStoppingRule>,
}

/// 早停规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlyStoppingRule {
    /// 规则名称
    pub name: String,
    /// 条件
    pub condition: String,
    /// 动作
    pub action: EarlyStoppingAction,
}

/// 早停动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EarlyStoppingAction {
    /// 停止测试
    StopTest,
    /// 暂停测试
    PauseTest,
    /// 调整流量
    AdjustTraffic(f32),
    /// 发送警告
    SendAlert,
}

/// 测试执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecution {
    /// 执行ID
    pub execution_id: String,
    /// 测试ID
    pub test_id: String,
    /// 变体ID
    pub variant_id: String,
    /// 用户ID
    pub user_id: Option<String>,
    /// 会话ID
    pub session_id: String,
    /// 执行时间
    pub executed_at: DateTime<Utc>,
    /// 输入数据
    pub input_data: serde_json::Value,
    /// 输出结果
    pub output_result: serde_json::Value,
    /// 指标值
    pub metric_values: HashMap<String, f64>,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 错误信息
    pub error_message: Option<String>,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 取消
    Cancelled,
}

/// 测试结果分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAnalysis {
    /// 测试ID
    pub test_id: String,
    /// 分析时间
    pub analyzed_at: DateTime<Utc>,
    /// 变体结果
    pub variant_results: HashMap<String, VariantResult>,
    /// 统计显著性
    pub statistical_significance: StatisticalSignificance,
    /// 推荐动作
    pub recommendations: Vec<Recommendation>,
    /// 置信区间
    pub confidence_intervals: HashMap<String, ConfidenceInterval>,
    /// 效应大小
    pub effect_sizes: HashMap<String, f64>,
}

/// 变体结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResult {
    /// 变体ID
    pub variant_id: String,
    /// 样本大小
    pub sample_size: usize,
    /// 指标结果
    pub metric_results: HashMap<String, MetricResult>,
    /// 总体评分
    pub overall_score: f64,
    /// 排名
    pub rank: usize,
}

/// 指标结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricResult {
    /// 指标名称
    pub metric_name: String,
    /// 平均值
    pub mean: f64,
    /// 标准差
    pub std_dev: f64,
    /// 中位数
    pub median: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 百分位数
    pub percentiles: HashMap<String, f64>,
}

/// 统计显著性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSignificance {
    /// 是否显著
    pub is_significant: bool,
    /// p值
    pub p_value: f64,
    /// 置信水平
    pub confidence_level: f32,
    /// 检验方法
    pub test_method: String,
}

/// 推荐动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// 推荐类型
    pub recommendation_type: RecommendationType,
    /// 描述
    pub description: String,
    /// 置信度
    pub confidence: f32,
    /// 预期影响
    pub expected_impact: f64,
    /// 实施优先级
    pub priority: Priority,
}

/// 推荐类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// 选择获胜变体
    SelectWinner,
    /// 继续测试
    ContinueTesting,
    /// 调整流量分配
    AdjustTraffic,
    /// 创建新变体
    CreateNewVariant,
    /// 停止测试
    StopTest,
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    /// 低
    Low,
    /// 中
    Medium,
    /// 高
    High,
    /// 紧急
    Critical,
}

/// 置信区间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// 下界
    pub lower_bound: f64,
    /// 上界
    pub upper_bound: f64,
    /// 置信水平
    pub confidence_level: f32,
}

/// 测试结果存储
#[derive(Debug)]
pub struct TestResultsStorage {
    /// 存储路径
    storage_path: std::path::PathBuf,
    /// 内存缓存
    cache: Arc<RwLock<HashMap<String, Vec<TestExecution>>>>,
}

/// 统计分析器
#[derive(Debug)]
pub struct StatisticalAnalyzer {
    /// 分析配置
    config: AnalysisConfig,
}

/// 分析配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// 默认置信水平
    pub default_confidence_level: f32,
    /// 最小样本大小
    pub min_sample_size: usize,
    /// 使用的统计检验方法
    pub statistical_tests: Vec<String>,
    /// 多重比较校正
    pub multiple_comparison_correction: bool,
}

/// A/B测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    /// 最大并发测试数
    pub max_concurrent_tests: usize,
    /// 默认测试持续时间（小时）
    pub default_test_duration_hours: u64,
    /// 自动分析间隔（分钟）
    pub auto_analysis_interval_minutes: u64,
    /// 启用早停
    pub enable_early_stopping: bool,
}

impl Default for ABTestConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 10,
            default_test_duration_hours: 168, // 1 week
            auto_analysis_interval_minutes: 60,
            enable_early_stopping: true,
        }
    }
}

/// A/B测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResults {
    /// 测试ID
    pub test_id: String,
    /// 测试名称
    pub test_name: String,
    /// 测试状态
    pub status: TestStatus,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 变体结果
    pub variant_results: HashMap<String, VariantResult>,
    /// 获胜变体
    pub winning_variant: Option<String>,
    /// 统计显著性
    pub statistical_significance: Option<StatisticalSignificance>,
    /// 总样本数
    pub total_samples: usize,
    /// 置信水平
    pub confidence_level: f32,
    /// 效应大小
    pub effect_size: Option<f64>,
    /// 推荐动作
    pub recommendations: Vec<Recommendation>,
    /// 测试摘要
    pub summary: String,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            default_confidence_level: 0.95,
            min_sample_size: 100,
            statistical_tests: vec![
                "t_test".to_string(),
                "mann_whitney".to_string(),
                "chi_square".to_string(),
            ],
            multiple_comparison_correction: true,
        }
    }
}

impl PromptABTestManager {
    /// 创建新的A/B测试管理器
    pub fn new(storage_path: std::path::PathBuf, config: ABTestConfig) -> Self {
        Self {
            active_tests: Arc::new(RwLock::new(HashMap::new())),
            results_storage: TestResultsStorage::new(storage_path),
            analyzer: StatisticalAnalyzer::new(AnalysisConfig::default()),
            config,
        }
    }

    /// 创建新的A/B测试
    pub async fn create_test(&self, test_config: CreateTestRequest) -> Result<ABTest> {
        let test_id = Uuid::new_v4().to_string();
        
        let test = ABTest {
            test_id: test_id.clone(),
            name: test_config.name,
            description: test_config.description,
            status: TestStatus::Draft,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            variants: test_config.variants,
            traffic_allocation: test_config.traffic_allocation,
            metrics: test_config.metrics,
            conditions: test_config.conditions,
            metadata: test_config.metadata,
        };

        // 验证测试配置
        self.validate_test(&test)?;

        // 保存测试
        let mut active_tests = self.active_tests.write().await;
        active_tests.insert(test_id.clone(), test.clone());

        Ok(test)
    }

    /// 开始测试
    pub async fn start_test(&self, test_id: &str) -> Result<()> {
        let mut active_tests = self.active_tests.write().await;
        
        if let Some(test) = active_tests.get_mut(test_id) {
            if test.status != TestStatus::Draft {
                return Err(anyhow!("Test is not in draft status"));
            }
            
            test.status = TestStatus::Running;
            test.started_at = Some(Utc::now());
            
            Ok(())
        } else {
            Err(anyhow!("Test not found: {}", test_id))
        }
    }

    /// 暂停测试
    pub async fn pause_test(&self, test_id: &str) -> Result<()> {
        let mut active_tests = self.active_tests.write().await;
        
        if let Some(test) = active_tests.get_mut(test_id) {
            if test.status != TestStatus::Running {
                return Err(anyhow!("Test is not running"));
            }
            
            test.status = TestStatus::Paused;
            Ok(())
        } else {
            Err(anyhow!("Test not found: {}", test_id))
        }
    }

    /// 停止测试
    pub async fn stop_test(&self, test_id: &str) -> Result<()> {
        let mut active_tests = self.active_tests.write().await;
        
        if let Some(test) = active_tests.get_mut(test_id) {
            test.status = TestStatus::Completed;
            test.ended_at = Some(Utc::now());
            Ok(())
        } else {
            Err(anyhow!("Test not found: {}", test_id))
        }
    }

    /// 分配用户到变体
    pub async fn assign_variant(&self, test_id: &str, user_context: &UserContext) -> Result<String> {
        let active_tests = self.active_tests.read().await;
        
        if let Some(test) = active_tests.get(test_id) {
            if test.status != TestStatus::Running {
                return Err(anyhow!("Test is not running"));
            }
            
            self.allocate_variant(test, user_context)
        } else {
            Err(anyhow!("Test not found: {}", test_id))
        }
    }

    /// 记录测试执行结果
    pub async fn record_execution(&self, execution: TestExecution) -> Result<()> {
        self.results_storage.store_execution(execution).await
    }

    /// 分析测试结果
    pub async fn analyze_test(&self, test_id: &str) -> Result<TestAnalysis> {
        let active_tests = self.active_tests.read().await;
        
        if let Some(test) = active_tests.get(test_id) {
            let executions = self.results_storage.get_executions(test_id).await?;
            self.analyzer.analyze_test_results(test, &executions).await
        } else {
            Err(anyhow!("Test not found: {}", test_id))
        }
    }

    /// 获取测试列表
    pub async fn list_tests(&self) -> Result<Vec<ABTest>> {
        let active_tests = self.active_tests.read().await;
        Ok(active_tests.values().cloned().collect())
    }

    /// 获取活跃测试列表
    pub async fn list_active_tests(&self) -> Result<Vec<ABTest>> {
        let active_tests = self.active_tests.read().await;
        Ok(active_tests.values()
            .filter(|test| test.status == TestStatus::Running)
            .cloned()
            .collect())
    }

    /// 获取测试详情
    pub async fn get_test(&self, test_id: &str) -> Result<ABTest> {
        let active_tests = self.active_tests.read().await;
        
        active_tests.get(test_id)
            .cloned()
            .ok_or_else(|| anyhow!("Test not found: {}", test_id))
    }

    /// 获取测试结果
    pub async fn get_test_results(&self, test_id: &str) -> Result<ABTestResults> {
        let test = self.get_test(test_id).await?;
        let analysis = self.analyze_test(test_id).await?;
        
        // 找到获胜变体
        let winning_variant = analysis.variant_results.iter()
            .max_by(|a, b| a.1.overall_score.partial_cmp(&b.1.overall_score).unwrap())
            .map(|(variant_id, _)| variant_id.clone());
        
        // 计算总样本数
        let total_samples = analysis.variant_results.values()
            .map(|r| r.sample_size)
            .sum();
        
        // 生成摘要
        let summary = self.generate_test_summary(&test, &analysis);
        
        Ok(ABTestResults {
            test_id: test.test_id,
            test_name: test.name,
            status: test.status,
            started_at: test.started_at,
            ended_at: test.ended_at,
            variant_results: analysis.variant_results,
            winning_variant,
            statistical_significance: Some(analysis.statistical_significance),
            total_samples,
            confidence_level: test.conditions.confidence_level,
            effect_size: analysis.effect_sizes.values().next().copied(),
            recommendations: analysis.recommendations,
            summary,
        })
    }

    /// 生成测试摘要
    fn generate_test_summary(&self, test: &ABTest, analysis: &TestAnalysis) -> String {
        let total_samples: usize = analysis.variant_results.values()
            .map(|r| r.sample_size)
            .sum();
        
        let best_variant = analysis.variant_results.iter()
            .max_by(|a, b| a.1.overall_score.partial_cmp(&b.1.overall_score).unwrap())
            .map(|(variant_id, result)| (variant_id, result.overall_score));
        
        match best_variant {
            Some((variant_id, score)) => {
                format!(
                    "测试 '{}' 已收集 {} 个样本。变体 '{}' 表现最佳，总体评分为 {:.3}。{}",
                    test.name,
                    total_samples,
                    variant_id,
                    score,
                    if analysis.statistical_significance.is_significant {
                        "结果具有统计显著性。"
                    } else {
                        "结果尚未达到统计显著性。"
                    }
                )
            },
            None => format!("测试 '{}' 尚无足够数据进行分析。", test.name)
        }
    }

    /// 验证测试配置
    fn validate_test(&self, test: &ABTest) -> Result<()> {
        // 检查变体数量
        if test.variants.len() < 2 {
            return Err(anyhow!("At least 2 variants are required"));
        }

        // 检查控制组
        let control_count = test.variants.iter().filter(|v| v.is_control).count();
        if control_count != 1 {
            return Err(anyhow!("Exactly one control variant is required"));
        }

        // 检查流量分配
        let total_weight: f32 = test.traffic_allocation.variant_weights.values().sum();
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(anyhow!("Variant weights must sum to 1.0"));
        }

        // 检查指标
        if test.metrics.is_empty() {
            return Err(anyhow!("At least one metric is required"));
        }

        let primary_metrics = test.metrics.iter().filter(|m| m.is_primary).count();
        if primary_metrics == 0 {
            return Err(anyhow!("At least one primary metric is required"));
        }

        Ok(())
    }

    /// 分配变体
    fn allocate_variant(&self, test: &ABTest, user_context: &UserContext) -> Result<String> {
        match test.traffic_allocation.strategy {
            AllocationStrategy::Random => {
                self.random_allocation(&test.variants)
            },
            AllocationStrategy::UserIdHash => {
                if let Some(user_id) = &user_context.user_id {
                    self.hash_based_allocation(&test.variants, user_id)
                } else {
                    self.random_allocation(&test.variants)
                }
            },
            AllocationStrategy::SessionId => {
                self.hash_based_allocation(&test.variants, &user_context.session_id)
            },
            AllocationStrategy::Custom(_) => {
                // 实现自定义分配逻辑
                self.random_allocation(&test.variants)
            },
        }
    }

    /// 随机分配
    fn random_allocation(&self, variants: &[TestVariant]) -> Result<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_value: f32 = rand::random();
        
        let mut cumulative_weight = 0.0;
        for variant in variants {
            cumulative_weight += variant.traffic_weight;
            if random_value <= cumulative_weight {
                return Ok(variant.variant_id.clone());
            }
        }
        
        // 默认返回第一个变体
        Ok(variants[0].variant_id.clone())
    }

    /// 基于哈希的分配
    fn hash_based_allocation(&self, variants: &[TestVariant], key: &str) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash_value = hasher.finish();
        
        let normalized_hash = (hash_value as f64) / (u64::MAX as f64);
        
        let mut cumulative_weight = 0.0;
        for variant in variants {
            cumulative_weight += variant.traffic_weight as f64;
            if normalized_hash <= cumulative_weight {
                return Ok(variant.variant_id.clone());
            }
        }
        
        Ok(variants[0].variant_id.clone())
    }
}

/// 用户上下文
#[derive(Debug, Clone)]
pub struct UserContext {
    /// 用户ID
    pub user_id: Option<String>,
    /// 会话ID
    pub session_id: String,
    /// 用户属性
    pub attributes: HashMap<String, serde_json::Value>,
}

/// 创建测试请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTestRequest {
    /// 测试名称
    pub name: String,
    /// 测试描述
    pub description: String,
    /// 测试变体
    pub variants: Vec<TestVariant>,
    /// 流量分配
    pub traffic_allocation: TrafficAllocation,
    /// 评估指标
    pub metrics: Vec<EvaluationMetric>,
    /// 测试条件
    pub conditions: TestConditions,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TestResultsStorage {
    /// 创建新的结果存储
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            storage_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 存储执行结果
    pub async fn store_execution(&self, execution: TestExecution) -> Result<()> {
        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.entry(execution.test_id.clone())
            .or_insert_with(Vec::new)
            .push(execution.clone());

        // 持久化到文件
        let file_path = self.storage_path.join(format!("{}.json", execution.test_id));
        let executions = cache.get(&execution.test_id).unwrap();
        let content = serde_json::to_string_pretty(executions)?;
        tokio::fs::write(file_path, content).await?;

        Ok(())
    }

    /// 获取执行结果
    pub async fn get_executions(&self, test_id: &str) -> Result<Vec<TestExecution>> {
        let cache = self.cache.read().await;
        
        if let Some(executions) = cache.get(test_id) {
            Ok(executions.clone())
        } else {
            // 从文件加载
            self.load_executions_from_file(test_id).await
        }
    }

    /// 从文件加载执行结果
    async fn load_executions_from_file(&self, test_id: &str) -> Result<Vec<TestExecution>> {
        let file_path = self.storage_path.join(format!("{}.json", test_id));
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(file_path).await?;
        let executions: Vec<TestExecution> = serde_json::from_str(&content)?;
        
        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.insert(test_id.to_string(), executions.clone());
        
        Ok(executions)
    }
}

impl StatisticalAnalyzer {
    /// 创建新的统计分析器
    pub fn new(config: AnalysisConfig) -> Self {
        Self { config }
    }

    /// 分析测试结果
    pub async fn analyze_test_results(
        &self,
        test: &ABTest,
        executions: &[TestExecution],
    ) -> Result<TestAnalysis> {
        let mut variant_results = HashMap::new();
        
        // 按变体分组执行结果
        let mut variant_executions: HashMap<String, Vec<&TestExecution>> = HashMap::new();
        for execution in executions {
            variant_executions.entry(execution.variant_id.clone())
                .or_insert_with(Vec::new)
                .push(execution);
        }

        // 计算每个变体的结果
        for (variant_id, variant_executions) in &variant_executions {
            let result = self.calculate_variant_result(variant_id, variant_executions, &test.metrics)?;
            variant_results.insert(variant_id.clone(), result);
        }

        // 计算统计显著性
        let statistical_significance = self.calculate_statistical_significance(&variant_results)?;

        // 生成推荐
        let recommendations = self.generate_recommendations(test, &variant_results, &statistical_significance)?;

        // 计算置信区间
        let confidence_intervals = self.calculate_confidence_intervals(&variant_results)?;

        // 计算效应大小
        let effect_sizes = self.calculate_effect_sizes(&variant_results)?;

        Ok(TestAnalysis {
            test_id: test.test_id.clone(),
            analyzed_at: Utc::now(),
            variant_results,
            statistical_significance,
            recommendations,
            confidence_intervals,
            effect_sizes,
        })
    }

    /// 计算变体结果
    fn calculate_variant_result(
        &self,
        variant_id: &str,
        executions: &[&TestExecution],
        metrics: &[EvaluationMetric],
    ) -> Result<VariantResult> {
        let mut metric_results = HashMap::new();
        
        for metric in metrics {
            let values: Vec<f64> = executions.iter()
                .filter_map(|e| e.metric_values.get(&metric.name))
                .copied()
                .collect();
            
            if !values.is_empty() {
                let result = self.calculate_metric_statistics(&values);
                metric_results.insert(metric.name.clone(), result);
            }
        }

        // 计算总体评分（简化实现）
        let overall_score = metric_results.values()
            .map(|r| r.mean)
            .sum::<f64>() / metric_results.len() as f64;

        Ok(VariantResult {
            variant_id: variant_id.to_string(),
            sample_size: executions.len(),
            metric_results,
            overall_score,
            rank: 0, // 将在后续排序中设置
        })
    }

    /// 计算指标统计
    fn calculate_metric_statistics(&self, values: &[f64]) -> MetricResult {
        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        let median = if sorted_values.len() % 2 == 0 {
            (sorted_values[sorted_values.len() / 2 - 1] + sorted_values[sorted_values.len() / 2]) / 2.0
        } else {
            sorted_values[sorted_values.len() / 2]
        };
        
        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        
        let mut percentiles = HashMap::new();
        percentiles.insert("p25".to_string(), self.percentile(&sorted_values, 0.25));
        percentiles.insert("p75".to_string(), self.percentile(&sorted_values, 0.75));
        percentiles.insert("p90".to_string(), self.percentile(&sorted_values, 0.90));
        percentiles.insert("p95".to_string(), self.percentile(&sorted_values, 0.95));
        
        MetricResult {
            metric_name: "unknown".to_string(), // 将在调用处设置
            mean,
            std_dev,
            median,
            min,
            max,
            percentiles,
        }
    }

    /// 计算百分位数
    fn percentile(&self, sorted_values: &[f64], p: f64) -> f64 {
        let index = (p * (sorted_values.len() - 1) as f64) as usize;
        sorted_values[index]
    }

    /// 计算统计显著性（简化实现）
    fn calculate_statistical_significance(
        &self,
        _variant_results: &HashMap<String, VariantResult>,
    ) -> Result<StatisticalSignificance> {
        // 简化实现，实际应该进行适当的统计检验
        Ok(StatisticalSignificance {
            is_significant: false,
            p_value: 0.05,
            confidence_level: self.config.default_confidence_level,
            test_method: "t_test".to_string(),
        })
    }

    /// 生成推荐
    fn generate_recommendations(
        &self,
        _test: &ABTest,
        variant_results: &HashMap<String, VariantResult>,
        _statistical_significance: &StatisticalSignificance,
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();
        
        // 找到最佳变体
        if let Some((best_variant, _)) = variant_results.iter()
            .max_by(|a, b| a.1.overall_score.partial_cmp(&b.1.overall_score).unwrap()) {
            
            recommendations.push(Recommendation {
                recommendation_type: RecommendationType::SelectWinner,
                description: format!("建议选择变体 {} 作为获胜者", best_variant),
                confidence: 0.8,
                expected_impact: 0.1,
                priority: Priority::High,
            });
        }
        
        Ok(recommendations)
    }

    /// 计算置信区间（简化实现）
    fn calculate_confidence_intervals(
        &self,
        _variant_results: &HashMap<String, VariantResult>,
    ) -> Result<HashMap<String, ConfidenceInterval>> {
        Ok(HashMap::new())
    }

    /// 计算效应大小（简化实现）
    fn calculate_effect_sizes(
        &self,
        _variant_results: &HashMap<String, VariantResult>,
    ) -> Result<HashMap<String, f64>> {
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ab_test_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ABTestConfig::default();
        let manager = PromptABTestManager::new(temp_dir.path().to_path_buf(), config);
        
        let request = CreateTestRequest {
            name: "Test Prompt".to_string(),
            description: "Testing prompt effectiveness".to_string(),
            variants: vec![
                TestVariant {
                    variant_id: "control".to_string(),
                    name: "Control".to_string(),
                    description: "Original prompt".to_string(),
                    is_control: true,
                    prompt_config: PromptConfig::default(),
                    traffic_weight: 0.5,
                    variant_config: HashMap::new(),
                },
                TestVariant {
                    variant_id: "treatment".to_string(),
                    name: "Treatment".to_string(),
                    description: "New prompt".to_string(),
                    is_control: false,
                    prompt_config: PromptConfig::default(),
                    traffic_weight: 0.5,
                    variant_config: HashMap::new(),
                },
            ],
            traffic_allocation: TrafficAllocation {
                strategy: AllocationStrategy::Random,
                total_traffic_percent: 100.0,
                variant_weights: [
                    ("control".to_string(), 0.5),
                    ("treatment".to_string(), 0.5),
                ].into_iter().collect(),
                user_segmentation: None,
            },
            metrics: vec![
                EvaluationMetric {
                    name: "success_rate".to_string(),
                    metric_type: MetricType::SuccessRate,
                    description: "Task success rate".to_string(),
                    target_value: Some(0.8),
                    weight: 1.0,
                    is_primary: true,
                    calculation_method: CalculationMethod::Average,
                },
            ],
            conditions: TestConditions {
                min_sample_size: 100,
                max_duration_hours: Some(168), // 1 week
                confidence_level: 0.95,
                minimum_detectable_effect: 0.05,
                early_stopping_rules: vec![],
            },
            metadata: HashMap::new(),
        };
        
        let test = manager.create_test(request).await.unwrap();
        assert_eq!(test.variants.len(), 2);
        assert_eq!(test.status, TestStatus::Draft);
    }

    #[test]
    fn test_statistical_analyzer() {
        let analyzer = StatisticalAnalyzer::new(AnalysisConfig::default());
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = analyzer.calculate_metric_statistics(&values);
        
        assert_eq!(result.mean, 3.0);
        assert_eq!(result.median, 3.0);
        assert_eq!(result.min, 1.0);
        assert_eq!(result.max, 5.0);
    }
}