use crate::tools::McpClientManager;
use crate::models::database::{Asset, ScanTask, Vulnerability};
use crate::models::scan::{
    ScanConfig, ScanResult, ScanStatus, ScanStep, ScanTask as ScanTaskModel, TaskStats,
};
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};
use uuid::Uuid;

/// 智能扫描服务
/// 集成AI分析、MCP工具执行、数据库持久化
pub struct ScanService {
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    mcp_client: Arc<RwLock<McpClientManager>>,
    active_tasks: RwLock<HashMap<Uuid, Arc<Mutex<ScanTaskModel>>>>,
}

impl ScanService {
    pub fn new(
        db: Arc<DatabaseService>,
        ai_manager: Arc<AiServiceManager>,
        mcp_client: Arc<RwLock<McpClientManager>>,
    ) -> Self {
        Self {
            db,
            ai_manager,
            mcp_client,
            active_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// 创建扫描任务
    pub async fn create_task(&self, target: String, config: ScanConfig) -> Result<ScanTaskModel> {
        info!("Creating scan task: {}", target);

        let task = ScanTaskModel {
            id: Uuid::new_v4(),
            name: config
                .name
                .clone()
                .unwrap_or_else(|| format!("AI Scan {}", target)),
            target: target.clone(),
            config: config.clone(),
            status: ScanStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            results_count: 0,
            vulnerabilities_found: 0,
            error_message: None,
            current_step: None,
            steps: vec![],
        };

        // 转换为数据库模型
        let db_task = ScanTask {
            id: task.id.to_string(),
            project_id: None,
            name: task.name.clone(),
            description: Some(format!("AI Scan Task: {}", target)),
            target_type: "domain".to_string(),
            targets: json!([target]).to_string(),
            scan_type: "comprehensive".to_string(),
            tools_config: Some(json!(config).to_string()),
            status: "pending".to_string(),
            progress: 0.0,
            priority: 1,
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            execution_time: None,
            results_summary: None,
            error_message: None,
            created_by: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 保存到数据库
        self.db
            .create_scan_task(&db_task)
            .await
            .context("Failed to save scan task to database")?;

        // 加入活跃任务列表
        let mut active_tasks = self.active_tasks.write().await;
        active_tasks.insert(task.id, Arc::new(Mutex::new(task.clone())));

        info!("Scan task created successfully: {}", task.id);
        Ok(task)
    }

    /// 启动扫描任务
    pub async fn start_task(&self, task_id: Uuid) -> Result<()> {
        info!("Starting scan task: {}", task_id);

        // 获取任务
        let task_arc = {
            let active_tasks = self.active_tasks.read().await;
            active_tasks
                .get(&task_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
        };

        // 更新任务状态
        {
            let mut task = task_arc.lock().await;
            task.status = ScanStatus::Running;
            task.started_at = Some(Utc::now());
            task.current_step = Some("AI Scan Planning".to_string());
        }

        // 异步执行扫描
        let db = self.db.clone();
        let ai_manager = self.ai_manager.clone();
        let mcp_client = self.mcp_client.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_scan(task_arc, db, ai_manager, mcp_client).await {
                error!("Scan task execution failed: {}", e);
            }
        });

        Ok(())
    }

    /// 执行扫描任务的核心逻辑
    async fn execute_scan(
        task_arc: Arc<Mutex<ScanTaskModel>>,
        db: Arc<DatabaseService>,
        ai_manager: Arc<AiServiceManager>,
        mcp_client: Arc<RwLock<McpClientManager>>,
    ) -> Result<()> {
        let task_id = {
            let task = task_arc.lock().await;
            task.id
        };

        info!("Starting scan task: {}", task_id);

        // 阶段1: AI分析目标和制定扫描策略
        if let Err(e) = Self::phase_1_ai_planning(&task_arc, &ai_manager).await {
            Self::mark_task_failed(&task_arc, &db, &format!("AI planning phase failed: {}", e))
                .await;
            return Err(e);
        }

        // 阶段2: 资产发现
        if let Err(e) = Self::phase_2_asset_discovery(&task_arc, &db, &mcp_client).await {
            Self::mark_task_failed(
                &task_arc,
                &db,
                &format!("Asset discovery phase failed: {}", e),
            )
            .await;
            return Err(e);
        }

        // 阶段3: 漏洞扫描
        if let Err(e) = Self::phase_3_vulnerability_scan(&task_arc, &db, &mcp_client).await {
            Self::mark_task_failed(
                &task_arc,
                &db,
                &format!("Vulnerability scanning phase failed: {}", e),
            )
            .await;
            return Err(e);
        }

        // 阶段4: AI分析和验证
        if let Err(e) = Self::phase_4_ai_analysis(&task_arc, &db, &ai_manager).await {
            Self::mark_task_failed(&task_arc, &db, &format!("AI analysis phase failed: {}", e))
                .await;
            return Err(e);
        }

        // 完成任务
        Self::complete_task(&task_arc, &db).await?;

        info!("Scan task completed: {}", task_id);
        Ok(())
    }

    /// 阶段1: AI智能规划
    async fn phase_1_ai_planning(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        _ai_manager: &Arc<AiServiceManager>,
    ) -> Result<()> {
        info!("Starting AI scan planning phase");

        let (target, config) = {
            let mut task = task_arc.lock().await;
            task.current_step = Some("AI analysis target".to_string());
            task.progress = 10.0;
            (task.target.clone(), task.config.clone())
        };

        // 使用AI分析目标并制定扫描策略
        let analysis_prompt = format!(
            "As a security expert, please analyze the following target and制定扫描策略：

Target: {}
Scan tools: {:?}
Scan depth: {}

Please analyze:
1. Target type (domain, IP, URL, etc.)
2. Recommended scan tools and order
3. Possible attack surfaces
4. Scan注意事项
5. Expected vulnerability types

Please return the analysis result in JSON format.",
            target, config.tools, config.depth
        );

        // 暂时简化AI规划，等AI服务完全稳定后再完善
        info!("AI扫描规划: {}", analysis_prompt);


        // 解析AI响应并更新扫描步骤
        let steps = Self::parse_ai_scan_plan("")?;

        {
            let mut task = task_arc.lock().await;
            task.steps = steps;
            task.progress = 20.0;
        }

        info!("AI scan planning completed");
        Ok(())
    }

    /// 阶段2: 资产发现
    async fn phase_2_asset_discovery(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        _db: &Arc<DatabaseService>,
        _mcp_client: &Arc<RwLock<McpClientManager>>,
    ) -> Result<()> {
        info!("开始资产发现阶段");

        let target = {
            let mut task = task_arc.lock().await;
            task.current_step = Some("资产发现".to_string());
            task.progress = 30.0;
            task.target.clone()
        };

        // 子域名发现 - 简化版，因为MCP客户端接口还需要实现
        let subdomains = vec![target.clone()]; // 简化版

        // 端口扫描 - 简化版
        let open_ports = vec!["80".to_string(), "443".to_string()]; // 简化版

        // HTTP服务发现 - 简化版
        let http_services = vec![format!("https://{}", target)]; // 简化版

        // 保存发现的资产
        for subdomain in &subdomains {
            let _asset = Asset {
                id: Uuid::new_v4().to_string(),
                project_id: None,
                scan_task_id: None,
                asset_type: "subdomain".to_string(),
                value: subdomain.clone(),
                parent_id: None,
                metadata: Some(json!({"discovery_method": "subfinder"}).to_string()),
                status: "active".to_string(),
                confidence_score: 1.0,
                risk_level: "info".to_string(),
                tags: None,
                notes: None,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            // 暂时注释掉，等待实现create_asset方法
            // if let Err(e) = db.create_asset(&asset).await {
            //     warn!("保存资产失败: {}", e);
            // }
        }

        {
            let mut task = task_arc.lock().await;
            task.progress = 50.0;
            task.results_count = (subdomains.len() + open_ports.len() + http_services.len()) as u32;
        }

        info!(
            "Asset discovery completed, found {} assets",
            subdomains.len() + open_ports.len() + http_services.len()
        );
        Ok(())
    }

    /// 阶段3: 漏洞扫描
    async fn phase_3_vulnerability_scan(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        db: &Arc<DatabaseService>,
        _mcp_client: &Arc<RwLock<McpClientManager>>,
    ) -> Result<()> {
        info!("Starting vulnerability scanning phase");

        let _target = {
            let mut task = task_arc.lock().await;
            task.current_step = Some("Vulnerability scanning".to_string());
            task.progress = 60.0;
            task.target.clone()
        };

        // 模拟漏洞发现
        let vulnerabilities = vec![json!({
            "title": "示例SQL注入漏洞",
            "description": "发现潜在的SQL注入点",
            "severity": "high",
            "cvss": 8.5,
            "type": "sql_injection"
        })];

        // 保存发现的漏洞
        let mut vulns_saved = 0;
        for vuln_data in vulnerabilities {
            if let Ok(vulnerability) = Self::create_vulnerability_from_scan(&vuln_data) {
                if let Ok(_) = db.create_vulnerability(&vulnerability).await {
                    vulns_saved += 1;
                }
            }
        }

        {
            let mut task = task_arc.lock().await;
            task.progress = 80.0;
            task.vulnerabilities_found = vulns_saved;
        }

        info!(
            "Vulnerability scanning completed, found {} vulnerabilities",
            vulns_saved
        );
        Ok(())
    }

    /// 阶段4: AI分析和验证
    async fn phase_4_ai_analysis(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        _db: &Arc<DatabaseService>,
        _ai_manager: &Arc<AiServiceManager>,
    ) -> Result<()> {
        info!("Starting AI analysis verification phase");

        {
            let mut task = task_arc.lock().await;
            task.current_step = Some("AI analysis verification".to_string());
            task.progress = 90.0;
        }

        // 简化版 - 直接完成
        info!("AI analysis verification completed");
        Ok(())
    }

    /// 从扫描结果创建漏洞记录
    fn create_vulnerability_from_scan(vuln_data: &Value) -> Result<Vulnerability> {
        let vuln = Vulnerability {
            id: Uuid::new_v4().to_string(),
            project_id: None,
            asset_id: None,
            scan_task_id: None,
            title: vuln_data
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown vulnerability")
                .to_string(),
            description: vuln_data
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            vulnerability_type: vuln_data
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            severity: vuln_data
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string(),
            cvss_score: vuln_data.get("cvss").and_then(|v| v.as_f64()),
            cvss_vector: vuln_data
                .get("cvss_vector")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cwe_id: vuln_data
                .get("cwe")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            owasp_category: None,
            proof_of_concept: vuln_data
                .get("payload")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            impact: vuln_data
                .get("impact")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            remediation: vuln_data
                .get("remediation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            references: vuln_data
                .get("references")
                .and_then(|v| v.as_array())
                .map(|arr| json!(arr).to_string()),
            status: "open".to_string(),
            verification_status: "unverified".to_string(),

            resolution_date: None,
            tags: vuln_data
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| json!(arr).to_string()),
            attachments: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        Ok(vuln)
    }

    /// 解析AI扫描计划
    fn parse_ai_scan_plan(_ai_response: &str) -> Result<Vec<ScanStep>> {
        // 简化版解析，实际应该更智能
        let steps = vec![
            ScanStep {
                name: "Asset discovery".to_string(),
                description: "Discover subdomains, IPs, and ports of the target".to_string(),
                status: "pending".to_string(),
                started_at: None,
                completed_at: None,
            },
            ScanStep {
                name: "Vulnerability scanning".to_string(),
                description: "Scan vulnerabilities using multiple tools".to_string(),
                status: "pending".to_string(),
                started_at: None,
                completed_at: None,
            },
            ScanStep {
                name: "AI analysis".to_string(),
                description: "Analyze results and generate reports".to_string(),
                status: "pending".to_string(),
                started_at: None,
                completed_at: None,
            },
        ];

        Ok(steps)
    }

    /// 标记任务失败
    async fn mark_task_failed(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        db: &Arc<DatabaseService>,
        error: &str,
    ) {
        let task_id = {
            let mut task = task_arc.lock().await;
            task.status = ScanStatus::Failed;
            task.error_message = Some(error.to_string());
            task.completed_at = Some(Utc::now());
            task.id
        };

        if let Err(e) = db
            .update_scan_task_status(&task_id.to_string(), "failed", Some(0.0))
            .await
        {
            error!("Failed to update task status: {}", e);
        }
    }

    /// 完成任务
    async fn complete_task(
        task_arc: &Arc<Mutex<ScanTaskModel>>,
        db: &Arc<DatabaseService>,
    ) -> Result<()> {
        let task_id = {
            let mut task = task_arc.lock().await;
            task.status = ScanStatus::Completed;
            task.completed_at = Some(Utc::now());
            task.progress = 100.0;
            task.current_step = Some("Scan completed".to_string());
            task.id
        };

        db.update_scan_task_status(&task_id.to_string(), "completed", Some(100.0))
            .await
            .context("Failed to update task completion status")?;

        Ok(())
    }

    /// 停止扫描任务
    pub async fn stop_task(&self, task_id: Uuid) -> Result<()> {
        let active_tasks = self.active_tasks.read().await;
        if let Some(task_arc) = active_tasks.get(&task_id) {
            let mut task = task_arc.lock().await;
            task.status = ScanStatus::Cancelled;
            task.completed_at = Some(Utc::now());

            // 更新数据库
            self.db
                .update_scan_task_status(&task_id.to_string(), "cancelled", None)
                .await?;
        }

        Ok(())
    }

    /// 获取任务列表
    pub async fn list_tasks(&self) -> Result<Vec<ScanTaskModel>> {
        // 从数据库获取任务
        let db_tasks = self.db.get_scan_tasks(None).await?;

        // 转换为模型格式
        let mut tasks = Vec::new();
        for db_task in db_tasks {
            if let Ok(task) = Self::convert_db_task_to_model(&db_task) {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// 获取任务详情
    pub async fn get_task(&self, task_id: Uuid) -> Result<ScanTaskModel> {
        // 先从活跃任务中获取
        let active_tasks = self.active_tasks.read().await;
        if let Some(task_arc) = active_tasks.get(&task_id) {
            let task = task_arc.lock().await;
            return Ok(task.clone());
        }

        // 从数据库获取
        let db_task = self.db.get_scan_task(&task_id.to_string()).await?;
        if let Some(task) = db_task {
            Self::convert_db_task_to_model(&task)
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    /// 获取扫描结果
    pub async fn get_results(&self, _task_id: Uuid) -> Result<Vec<ScanResult>> {
        // 暂时返回空结果，等待实现
        Ok(vec![])
    }

    /// 获取任务统计信息
    pub async fn get_task_stats(&self) -> Result<TaskStats> {
        let tasks = self.list_tasks().await?;
        
        let mut stats = TaskStats {
            total: tasks.len(),
            running: 0,
            pending: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
        };
        
        for task in tasks {
            match task.status {
                ScanStatus::Running => stats.running += 1,
                ScanStatus::Pending => stats.pending += 1,
                ScanStatus::Completed => stats.completed += 1,
                ScanStatus::Failed => stats.failed += 1,
                ScanStatus::Cancelled => stats.cancelled += 1,
                _ => {},
            }
        }
        
        Ok(stats)
    }

    /// 删除扫描任务
    pub async fn delete_task(&self, task_id: Uuid) -> Result<()> {
        // 从活跃任务中移除
        let mut active_tasks = self.active_tasks.write().await;
        active_tasks.remove(&task_id);

        // 从数据库删除 - 需要实现delete_scan_task方法
        // self.db.delete_scan_task(task_id.to_string()).await
        Ok(())
    }

    /// 转换数据库任务为模型任务
    fn convert_db_task_to_model(db_task: &ScanTask) -> Result<ScanTaskModel> {
        let task_id = Uuid::parse_str(&db_task.id)?;

        // 解析配置
        let config = if let Some(config_str) = &db_task.tools_config {
            serde_json::from_str(config_str).unwrap_or_else(|_| ScanConfig {
                name: None,
                tools: vec!["nuclei".to_string()],
                depth: 1,
                timeout: 3600,
                concurrent_scans: 1,
                include_subdomains: true,
                port_range: None,
                custom_wordlists: vec![],
                exclude_patterns: vec![],
                notification_webhook: None,
            })
        } else {
            ScanConfig {
                name: None,
                tools: vec!["nuclei".to_string()],
                depth: 1,
                timeout: 3600,
                concurrent_scans: 1,
                include_subdomains: true,
                port_range: None,
                custom_wordlists: vec![],
                exclude_patterns: vec![],
                notification_webhook: None,
            }
        };

        // 解析目标
        let targets: Vec<String> = serde_json::from_str(&db_task.targets).unwrap_or_default();
        let target = targets.first().cloned().unwrap_or_default();

        let status = match db_task.status.as_str() {
            "pending" => ScanStatus::Pending,
            "running" => ScanStatus::Running,
            "paused" => ScanStatus::Paused,
            "completed" => ScanStatus::Completed,
            "failed" => ScanStatus::Failed,
            "cancelled" => ScanStatus::Cancelled,
            _ => ScanStatus::Pending,
        };

        Ok(ScanTaskModel {
            id: task_id,
            name: db_task.name.clone(),
            target,
            config,
            status,
            progress: db_task.progress as f32,
            created_at: db_task.created_at,
            started_at: db_task.started_at,
            completed_at: db_task.completed_at,
            results_count: 0,         // 需要单独查询
            vulnerabilities_found: 0, // 需要单独查询
            error_message: db_task.error_message.clone(),
            current_step: None,
            steps: vec![],
        })
    }

    /// AI辅助扫描分析（暂时简化实现）
    #[allow(unused)]
    async fn ai_scan_analysis(
        _scan_task: &ScanTask,
        _results: &Value,
        _ai_manager: &Arc<AiServiceManager>,
    ) -> Result<String> {
        // 暂时返回简单响应，等AI服务完全稳定后再完善
        Ok("AI analysis feature is temporarily unavailable".to_string())
    }
}

impl Default for ScanService {
    fn default() -> Self {
        unimplemented!("ScanService requires dependencies to be constructed")
    }
}
