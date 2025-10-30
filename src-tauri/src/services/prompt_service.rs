//! Prompt服务层
//! 
//! 提供统一的prompt管理和优化服务，整合：
//! - 配置管理
//! - 模板管理
//! - A/B测试
//! - 自动优化
//! - 性能监控

use sentinel_prompt::prompt_template_manager::ValidationRules;
use sentinel_prompt::{
    CreateTestRequest, OptimizationResult, OptimizationSuggestion, OptimizerConfig, PerformanceRecord, PromptBuildContext, PromptBuildResult, PromptBuilder, PromptConfig, PromptConfigManager, PromptOptimizer, PromptTemplateManager, SystemMetrics, TemplateManagerConfig, TokenUsage
};
use sentinel_prompt::prompt_ab_test_manager::{PromptABTestManager, ABTest, ABTestResults, ABTestConfig, UserContext};
use sentinel_prompt::prompt_optimizer::{BatchTestResult, ConfigReport, ExecutionContext, PerformanceAnalysis, ReportType, TestScenario, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;

/// Prompt服务
#[derive(Debug)]
pub struct PromptService {
    /// 配置管理器
    config_manager: Arc<PromptConfigManager>,
    /// 模板管理器
    template_manager: Arc<PromptTemplateManager>,
    /// A/B测试管理器
    ab_test_manager: Arc<PromptABTestManager>,
    /// 优化器
    optimizer: Arc<PromptOptimizer>,
    /// Prompt构建器
    builder: Arc<PromptBuilder>,
    /// 服务配置
    config: PromptServiceConfig,
    /// 活跃会话
    active_sessions: Arc<RwLock<HashMap<String, PromptSession>>>,
}

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptServiceConfig {
    /// 数据存储路径
    pub data_path: PathBuf,
    /// 启用自动优化
    pub enable_auto_optimization: bool,
    /// 启用A/B测试
    pub enable_ab_testing: bool,
    /// 启用性能监控
    pub enable_performance_monitoring: bool,
    /// 缓存大小
    pub cache_size: usize,
    /// 会话超时时间（分钟）
    pub session_timeout_minutes: u64,
    /// 最大并发会话数
    pub max_concurrent_sessions: usize,
}

/// Prompt会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSession {
    /// 会话ID
    pub session_id: String,
    /// 用户ID
    pub user_id: Option<String>,
    /// 当前配置ID
    pub config_id: String,
    /// 会话开始时间
    pub started_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 会话上下文
    pub context: HashMap<String, serde_json::Value>,
    /// 执行历史
    pub execution_history: Vec<ExecutionRecord>,
    /// 性能统计
    pub performance_stats: SessionPerformanceStats,
}

/// 执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// 记录ID
    pub record_id: String,
    /// 执行时间
    pub executed_at: DateTime<Utc>,
    /// 请求类型
    pub request_type: String,
    /// 输入参数
    pub input_params: HashMap<String, serde_json::Value>,
    /// 构建结果
    pub build_result: Option<PromptBuildResult>,
    /// 执行时长（毫秒）
    pub duration_ms: f64,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 会话性能统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPerformanceStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 总Token使用量
    pub total_tokens_used: u64,
    /// 总成本（美元）
    pub total_cost_usd: f64,
    /// 用户满意度评分
    pub user_satisfaction_score: Option<f32>,
}

/// Prompt构建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBuildRequest {
    /// 会话ID
    pub session_id: String,
    /// 构建类型
    pub build_type: PromptBuildType,
    /// 构建上下文
    pub context: PromptBuildContext,
    /// 配置覆盖
    pub config_override: Option<HashMap<String, serde_json::Value>>,
    /// 是否记录性能
    pub record_performance: bool,
}

/// Prompt构建类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptBuildType {
    /// 规划器
    Planner,
    /// 执行器
    Executor,
    /// 重规划器
    Replanner,
    /// 报告生成器
    ReportGenerator,
}

/// Prompt构建响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBuildResponse {
    /// 请求ID
    pub request_id: String,
    /// 构建结果
    pub result: PromptBuildResult,
    /// 使用的配置ID
    pub config_id: String,
    /// A/B测试变体ID
    pub ab_test_variant_id: Option<String>,
    /// 构建时间
    pub built_at: DateTime<Utc>,
    /// 性能指标
    pub performance_metrics: Option<HashMap<String, f64>>,
}

/// 优化请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequest {
    /// 配置ID
    pub config_id: String,
    /// 优化目标
    pub optimization_targets: Option<Vec<String>>,
    /// 是否自动应用
    pub auto_apply: bool,
    /// 验证设置
    pub validation_settings: Option<ValidationSettings>,
}

/// 验证设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettings {
    /// 启用A/B测试验证
    pub enable_ab_test: bool,
    /// 测试持续时间（小时）
    pub test_duration_hours: Option<f64>,
    /// 最小样本大小
    pub min_sample_size: Option<usize>,
    /// 置信水平
    pub confidence_level: Option<f64>,
}

/// 服务统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    /// 活跃会话数
    pub active_sessions: usize,
    /// 总配置数
    pub total_configs: usize,
    /// 总模板数
    pub total_templates: usize,
    /// 活跃A/B测试数
    pub active_ab_tests: usize,
    /// 今日请求数
    pub requests_today: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 系统健康状态
    pub health_status: HealthStatus,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重
    Critical,
}

impl Default for PromptServiceConfig {
    fn default() -> Self {
        Self {
            data_path: PathBuf::from("./data/prompt_service"),
            enable_auto_optimization: true,
            enable_ab_testing: true,
            enable_performance_monitoring: true,
            cache_size: 1000,
            session_timeout_minutes: 60,
            max_concurrent_sessions: 100,
        }
    }
}

impl Default for SessionPerformanceStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            avg_response_time_ms: 0.0,
            total_tokens_used: 0,
            total_cost_usd: 0.0,
            user_satisfaction_score: None,
        }
    }
}

impl PromptService {
    /// 创建新的Prompt服务
    pub async fn new(config: PromptServiceConfig) -> Result<Self> {
        // 确保数据目录存在
        tokio::fs::create_dir_all(&config.data_path).await?;
        
        // 创建子组件
        let config_manager = Arc::new(PromptConfigManager::new());
        
        let template_manager = Arc::new(PromptTemplateManager::new(
            config.data_path.join("templates"),
            TemplateManagerConfig {
                enable_cache: true,
                cache_ttl_seconds: 3600,
                enable_hot_reload: true,
                max_cache_size: config.cache_size,
                enable_versioning: true,
                max_versions: 10,
                validation_rules: ValidationRules {
                    max_template_length: 10000,
                    min_template_length: 50,
                    required_variables: vec![],
                    forbidden_content: vec![],
                    required_structure_markers: vec![],
                    max_length: Some(10000),
                },
                template_dir: config.data_path.join("templates"),
                validation_enabled: true,
                auto_backup: true,
            },
        ).await?);
        
        let ab_test_manager = Arc::new(PromptABTestManager::new(
            config.data_path.join("ab_tests"),
            ABTestConfig::default(),
        ));
        
        let optimizer = Arc::new(PromptOptimizer::new(
            OptimizerConfig::default(),
            ab_test_manager.clone(),
        ));
        
        let builder = Arc::new(PromptBuilder::new((*config_manager).clone()));
        
        Ok(Self {
            config_manager,
            template_manager,
            ab_test_manager,
            optimizer,
            builder,
            config,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 创建新会话
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        config_id: Option<String>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        // 使用默认配置或指定配置
        let config_id = match config_id {
            Some(id) => {
                // 验证配置是否存在
                if !self.config_manager.config_exists(&id).await? {
                    return Err(anyhow!("Configuration not found: {}", id));
                }
                id
            },
            None => {
                // 使用默认配置
                "default".to_string()
            },
        };
        
        let session = PromptSession {
            session_id: session_id.clone(),
            user_id,
            config_id,
            started_at: now,
            last_active_at: now,
            context: HashMap::new(),
            execution_history: Vec::new(),
            performance_stats: SessionPerformanceStats::default(),
        };
        
        // 检查并发会话限制
        let mut sessions = self.active_sessions.write().await;
        if sessions.len() >= self.config.max_concurrent_sessions {
            // 清理过期会话
            self.cleanup_expired_sessions(&mut sessions).await;
            
            if sessions.len() >= self.config.max_concurrent_sessions {
                return Err(anyhow!("Maximum concurrent sessions reached"));
            }
        }
        
        sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// 构建prompt
    pub async fn build_prompt(
        &self,
        request: PromptBuildRequest,
    ) -> Result<PromptBuildResponse> {
        let request_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        
        // 获取会话
        let mut sessions = self.active_sessions.write().await;
        let session = sessions.get_mut(&request.session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", request.session_id))?;
        
        // 更新会话活跃时间
        session.last_active_at = Utc::now();
        
        // 获取配置
        let mut config = self.config_manager.get_config(&session.config_id).await?;
        
        // 应用配置覆盖
        if let Some(overrides) = &request.config_override {
            self.apply_config_overrides(&mut config, overrides)?;
        }
        
        // 检查A/B测试
        let ab_test_variant_id = if self.config.enable_ab_testing {
            self.get_ab_test_variant(&session.config_id, &session.user_id).await?
        } else {
            None
        };
        
        // 构建prompt
        let result = match request.build_type {
            PromptBuildType::Planner => {
                self.builder.build_planner_prompt(&request.context).await?
            },
            PromptBuildType::Executor => {
                self.builder.build_executor_prompt(&request.context, "").await?
            },
            PromptBuildType::Replanner => {
                self.builder.build_replanner_prompt(&request.context, "", "").await?
            },
            PromptBuildType::ReportGenerator => {
                self.builder.build_report_prompt(&request.context, "", "").await?
            },
        };
        
        let duration = start_time.elapsed();
        
        // 记录执行历史
        let execution_record = ExecutionRecord {
            record_id: request_id.clone(),
            executed_at: Utc::now(),
            request_type: format!("{:?}", request.build_type),
            input_params: serde_json::to_value(&request.context)?
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            build_result: Some(result.clone()),
            duration_ms: duration.as_millis() as f64,
            success: true,
            error_message: None,
        };
        
        session.execution_history.push(execution_record);
        
        // 更新性能统计
        session.performance_stats.total_requests += 1;
        session.performance_stats.successful_requests += 1;
        session.performance_stats.avg_response_time_ms = 
            (session.performance_stats.avg_response_time_ms * (session.performance_stats.total_requests - 1) as f64 + duration.as_millis() as f64) / session.performance_stats.total_requests as f64;
        
        // 记录性能数据用于优化
        if request.record_performance && self.config.enable_performance_monitoring {
            let performance_record = PerformanceRecord {
                record_id: request_id.clone(),
                config_id: session.config_id.clone(),
                timestamp: Utc::now(),
                metrics: [
                    ("response_time_ms".to_string(), duration.as_millis() as f64),
                    ("token_count".to_string(), result.build_stats.tool_count as f64),
                    ("template_complexity".to_string(), result.build_stats.template_length as f64),
                ].into_iter().collect(),
                context: ExecutionContext {
                    task_type: "prompt_generation".to_string(),
                    input_complexity: 0.5,
                    target_info: request.context.target_info.as_ref().map(|t| t.host.clone()),
                    environment: HashMap::new(),
                },
                user_feedback: None,
                system_metrics: SystemMetrics {
                    response_time_ms: duration.as_millis() as f64,
                    token_usage: TokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        total_tokens: 0,
                        cost_usd: 0.0,
                    },
                    memory_usage_mb: 0.0,
                    cpu_usage_percent: 0.0,
                    error_rate: 0.0,
                },
            };
            
            if let Err(e) = self.optimizer.record_performance(performance_record).await {
                eprintln!("Failed to record performance data: {}", e);
            }
        }
        
        Ok(PromptBuildResponse {
            request_id,
            result,
            config_id: session.config_id.clone(),
            ab_test_variant_id,
            built_at: Utc::now(),
            performance_metrics: Some([
                ("response_time_ms".to_string(), duration.as_millis() as f64),
            ].into_iter().collect()),
        })
    }

    /// 优化配置
    pub async fn optimize_config(
        &self,
        request: OptimizationRequest,
    ) -> Result<OptimizationResult> {
        if !self.config.enable_auto_optimization {
            return Err(anyhow!("Auto optimization is disabled"));
        }
        
        let config = self.config_manager.get_config(&request.config_id).await?;
        let result = self.optimizer.optimize_config(&request.config_id, &config).await?;
        
        // 如果启用自动应用且优化结果足够好
        if request.auto_apply && self.should_auto_apply(&result) {
            self.config_manager.save_config(&request.config_id, &result.optimized_config).await?;
        }
        
        Ok(result)
    }

    /// 获取优化建议
    pub async fn get_optimization_suggestions(
        &self,
        config_id: &str,
    ) -> Result<Vec<OptimizationSuggestion>> {
        let config = self.config_manager.get_config(config_id).await?;
        self.optimizer.get_suggestions(config_id, &config).await
    }

    /// 创建A/B测试
    pub async fn create_ab_test(
        &self,
        request: CreateTestRequest,
    ) -> Result<ABTest> {
        if !self.config.enable_ab_testing {
            return Err(anyhow!("A/B testing is disabled"));
        }
        
        self.ab_test_manager.create_test(request).await
    }

    /// 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Result<PromptSession> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))
    }

    /// 关闭会话
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        Ok(())
    }

    /// 获取服务统计信息
    pub async fn get_service_stats(&self) -> Result<ServiceStats> {
        let sessions = self.active_sessions.read().await;
        let active_sessions = sessions.len();
        
        let total_configs = self.config_manager.list_configs().await?.len();
        let total_templates = self.template_manager.list_templates().await?.len();
        let active_ab_tests = self.ab_test_manager.list_active_tests().await?.len();
        
        // 计算今日请求数和平均响应时间
        let (requests_today, avg_response_time_ms) = self.calculate_daily_stats(&sessions).await;
        
        Ok(ServiceStats {
            active_sessions,
            total_configs,
            total_templates,
            active_ab_tests,
            requests_today,
            avg_response_time_ms,
            health_status: self.assess_health_status().await,
            last_updated: Utc::now(),
        })
    }

    /// 清理过期会话
    async fn cleanup_expired_sessions(
        &self,
        sessions: &mut HashMap<String, PromptSession>,
    ) {
        let timeout_duration = chrono::Duration::minutes(self.config.session_timeout_minutes as i64);
        let cutoff_time = Utc::now() - timeout_duration;
        
        sessions.retain(|_, session| session.last_active_at > cutoff_time);
    }

    /// 应用配置覆盖
    fn apply_config_overrides(
        &self,
        config: &mut PromptConfig,
        overrides: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for (key, value) in overrides {
            match key.as_str() {
                "agent_profile.planner_domain" => {
                    if let serde_json::Value::String(domain) = value {
                        config.agent_profile.planner_domain = domain.clone();
                    }
                },
                "agent_profile.report_template" => {
                    if let serde_json::Value::String(template) = value {
                        config.agent_profile.report_template = template.clone();
                    }
                },
                "domain_template.domain_instructions" => {
                    if let serde_json::Value::String(instructions) = value {
                        config.domain_template.domain_instructions = instructions.clone();
                    }
                },
                _ => {
                    // 忽略未知的覆盖键
                }
            }
        }
        Ok(())
    }

    /// 获取A/B测试变体
    async fn get_ab_test_variant(
        &self,
        _config_id: &str,
        user_id: &Option<String>,
    ) -> Result<Option<String>> {
        // 查找针对此配置的活跃A/B测试
        let active_tests = self.ab_test_manager.list_active_tests().await?;
        
        for test in active_tests {
            // 检查测试是否适用于此配置
            // 简化匹配逻辑，假设所有测试都适用于当前配置
            if !test.variants.is_empty() {
                let _user_key = user_id.as_deref().unwrap_or("anonymous");
                let user_context = UserContext {
                    user_id: user_id.clone(),
                    session_id: "default".to_string(),
                    attributes: std::collections::HashMap::new(),
                };
                if let Ok(variant_id) = self.ab_test_manager.assign_variant(&test.test_id, &user_context).await {
                    return Ok(Some(variant_id));
                }
            }
        }
        
        Ok(None)
    }

    /// 判断是否应该自动应用优化
    fn should_auto_apply(&self, result: &OptimizationResult) -> bool {
        // 检查性能改进是否达到阈值
        for (_, improvement) in &result.performance_improvement {
            if improvement.abs() < 0.05 { // 5%的改进阈值
                return false;
            }
        }
        
        // 检查应用的建议是否都是低风险的
        for suggestion in &result.applied_suggestions {
            if matches!(suggestion.risk_assessment.risk_level, RiskLevel::High | RiskLevel::Critical) {
                return false;
            }
        }
        
        true
    }

    /// 计算每日统计
    async fn calculate_daily_stats(
        &self,
        sessions: &HashMap<String, PromptSession>,
    ) -> (u64, f64) {
        let today = Utc::now().date_naive();
        let mut total_requests = 0u64;
        let mut total_response_time = 0.0;
        let mut response_count = 0u64;
        
        for session in sessions.values() {
            for record in &session.execution_history {
                if record.executed_at.date_naive() == today {
                    total_requests += 1;
                    if record.success {
                        total_response_time += record.duration_ms as f64;
                        response_count += 1;
                    }
                }
            }
        }
        
        let avg_response_time = if response_count > 0 {
            total_response_time / response_count as f64
        } else {
            0.0
        };
        
        (total_requests, avg_response_time)
    }

    /// 评估健康状态
    async fn assess_health_status(&self) -> HealthStatus {
        let sessions = self.active_sessions.read().await;
        
        // 简单的健康检查逻辑
        if sessions.len() > self.config.max_concurrent_sessions * 9 / 10 {
            HealthStatus::Warning
        } else if sessions.len() >= self.config.max_concurrent_sessions {
            HealthStatus::Critical
        } else {
            HealthStatus::Healthy
        }
    }

    // 公共访问器方法，用于访问内部组件
    
    /// 获取配置管理器的引用
    pub fn config_manager(&self) -> &PromptConfigManager {
        &self.config_manager
    }
    
    /// 获取模板管理器的引用
    pub fn template_manager(&self) -> &PromptTemplateManager {
        &self.template_manager
    }
    
    /// 获取模板管理器的可变引用
    pub fn template_manager_mut(&mut self) -> &mut PromptTemplateManager {
        Arc::get_mut(&mut self.template_manager).unwrap()
    }
    
    /// 获取A/B测试管理器的引用
    pub fn ab_test_manager(&self) -> &PromptABTestManager {
        &self.ab_test_manager
    }
    
    /// 获取优化器的引用
    pub fn optimizer(&self) -> &PromptOptimizer {
        &self.optimizer
    }
    
    /// 记录性能数据
    pub async fn record_performance_data(
        &self,
        record: PerformanceRecord,
    ) -> Result<()> {
        self.optimizer.record_performance(record).await
    }
    
    /// 获取性能分析
    pub async fn get_performance_analysis(
        &self,
        config_id: &str,
        time_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    ) -> Result<PerformanceAnalysis> {
        self.optimizer.get_performance_analysis(config_id, time_range).await
    }
    
    /// 批量测试配置
    pub async fn batch_test_configs(
        &self,
        config_ids: Vec<String>,
        test_scenarios: Vec<TestScenario>,
    ) -> Result<BatchTestResult> {
        self.optimizer.batch_test_configs(config_ids, test_scenarios).await
    }
    
    /// 生成配置报告
    pub async fn generate_config_report(
        &self,
        config_id: &str,
        report_type: ReportType,
    ) -> Result<ConfigReport> {
        self.optimizer.generate_config_report(config_id, report_type).await
    }
    
    /// 列出A/B测试
    pub async fn list_ab_tests(&self) -> Result<Vec<ABTest>> {
        self.ab_test_manager.list_tests().await
    }
    
    /// 获取A/B测试结果
    pub async fn get_ab_test_results(&self, test_id: &str) -> Result<ABTestResults> {
        self.ab_test_manager.get_test_results(test_id).await
    }
    
    /// 停止A/B测试
    pub async fn stop_ab_test(&self, test_id: &str) -> Result<()> {
        self.ab_test_manager.stop_test(test_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_prompt_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = PromptServiceConfig {
            data_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let service = PromptService::new(config).await.unwrap();
        let stats = service.get_service_stats().await.unwrap();
        
        assert_eq!(stats.active_sessions, 0);
        assert!(matches!(stats.health_status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_session_management() {
        let temp_dir = TempDir::new().unwrap();
        let config = PromptServiceConfig {
            data_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let service = PromptService::new(config).await.unwrap();
        
        // 创建会话
        let session_id = service.create_session(
            Some("test_user".to_string()),
            None,
        ).await.unwrap();
        
        // 获取会话
        let session = service.get_session(&session_id).await.unwrap();
        assert_eq!(session.user_id, Some("test_user".to_string()));
        
        // 关闭会话
        service.close_session(&session_id).await.unwrap();
        
        // 验证会话已关闭
        assert!(service.get_session(&session_id).await.is_err());
    }
}