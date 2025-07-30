use crate::models::scan_session::*;
use crate::services::scan_session::ScanSessionService;
use crate::tools::tool_manager::ToolManager;
use crate::tools::{ScanConfig, ScanResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 扫描引擎配置
#[derive(Debug, Clone)]
pub struct ScanEngineConfig {
    /// 最大并发扫描数
    pub max_concurrent_scans: usize,
    /// 扫描超时时间（秒）
    pub scan_timeout_seconds: u64,
    /// 阶段间延迟（毫秒）
    pub stage_delay_ms: u64,
    /// 是否启用智能优化
    pub enable_smart_optimization: bool,
}

impl Default for ScanEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_scans: 3,
            scan_timeout_seconds: 3600, // 1小时
            stage_delay_ms: 1000,       // 1秒
            enable_smart_optimization: true,
        }
    }
}

/// 扫描引擎状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEngineStatus {
    pub running_scans: usize,
    pub completed_scans: usize,
    pub failed_scans: usize,
    pub total_scans: usize,
}

/// 扫描引擎事件
#[derive(Debug, Clone)]
pub enum ScanEngineEvent {
    ScanStarted {
        session_id: Uuid,
    },
    StageStarted {
        session_id: Uuid,
        stage_name: String,
    },
    StageCompleted {
        session_id: Uuid,
        stage_name: String,
    },
    ScanCompleted {
        session_id: Uuid,
    },
    ScanFailed {
        session_id: Uuid,
        error: String,
    },
    ProgressUpdated {
        session_id: Uuid,
        progress: f64,
    },
}

/// 智能扫描引擎
#[derive(Clone)]
pub struct ScanEngine {
    config: ScanEngineConfig,
    tool_manager: Arc<ToolManager>,
    scan_session_service: Arc<ScanSessionService>,
    running_scans: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
    event_sender: mpsc::UnboundedSender<ScanEngineEvent>,
    status: Arc<RwLock<ScanEngineStatus>>,
}

impl ScanEngine {
    /// 创建新的扫描引擎
    pub fn new(
        config: ScanEngineConfig,
        tool_manager: Arc<ToolManager>,
        scan_session_service: Arc<ScanSessionService>,
    ) -> (Self, mpsc::UnboundedReceiver<ScanEngineEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let engine = Self {
            config,
            tool_manager,
            scan_session_service,
            running_scans: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            status: Arc::new(RwLock::new(ScanEngineStatus {
                running_scans: 0,
                completed_scans: 0,
                failed_scans: 0,
                total_scans: 0,
            })),
        };

        (engine, event_receiver)
    }

    /// 启动扫描
    pub async fn start_scan(&self, session_id: Uuid) -> Result<()> {
        // 检查并发限制
        let running_count = self.running_scans.read().await.len();
        if running_count >= self.config.max_concurrent_scans {
            return Err(anyhow!("已达到最大并发扫描数限制"));
        }

        // 获取扫描会话
        let session = self
            .scan_session_service
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow!("扫描会话不存在"))?;

        // 更新状态
        {
            let mut status = self.status.write().await;
            status.running_scans += 1;
            status.total_scans += 1;
        }

        // 发送事件
        let _ = self
            .event_sender
            .send(ScanEngineEvent::ScanStarted { session_id });

        // 启动扫描任务
        let scan_task = self.create_scan_task(session).await?;
        self.running_scans
            .write()
            .await
            .insert(session_id, scan_task);

        Ok(())
    }

    /// 停止扫描
    pub async fn stop_scan(&self, session_id: Uuid) -> Result<()> {
        if let Some(task) = self.running_scans.write().await.remove(&session_id) {
            task.abort();

            // 更新会话状态
            let update_request = UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Cancelled),
                error_message: Some("扫描被用户取消".to_string()),
                ..Default::default()
            };

            self.scan_session_service
                .update_session(session_id, update_request)
                .await?;

            // 更新状态
            {
                let mut status = self.status.write().await;
                status.running_scans = status.running_scans.saturating_sub(1);
            }

            info!("扫描 {} 已被停止", session_id);
        }

        Ok(())
    }

    /// 获取引擎状态
    pub async fn get_status(&self) -> ScanEngineStatus {
        self.status.read().await.clone()
    }

    /// 创建扫描任务
    async fn create_scan_task(&self, session: ScanSession) -> Result<tokio::task::JoinHandle<()>> {
        let session_id = session.id;
        let tool_manager = self.tool_manager.clone();
        let scan_session_service = self.scan_session_service.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        let running_scans = self.running_scans.clone();
        let status = self.status.clone();

        let task = tokio::spawn(async move {
            let start_time = Instant::now();

            // 更新会话状态为运行中
            let update_request = UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Running),
                ..Default::default()
            };

            if let Err(e) = scan_session_service
                .update_session(session_id, update_request)
                .await
            {
                error!("更新扫描会话状态失败: {}", e);
                return;
            }

            // 执行扫描流程
            let result = Self::execute_scan_workflow(
                session,
                tool_manager,
                scan_session_service.clone(),
                event_sender.clone(),
                config,
            )
            .await;

            // 处理扫描结果
            let final_status = match result {
                Ok(_) => {
                    let _ = event_sender.send(ScanEngineEvent::ScanCompleted { session_id });
                    {
                        let mut s = status.write().await;
                        s.completed_scans += 1;
                        s.running_scans = s.running_scans.saturating_sub(1);
                    }
                    ScanSessionStatus::Completed
                }
                Err(e) => {
                    error!("扫描失败: {}", e);
                    let _ = event_sender.send(ScanEngineEvent::ScanFailed {
                        session_id,
                        error: e.to_string(),
                    });
                    {
                        let mut s = status.write().await;
                        s.failed_scans += 1;
                        s.running_scans = s.running_scans.saturating_sub(1);
                    }
                    ScanSessionStatus::Failed
                }
            };

            // 更新最终状态
            let duration = start_time.elapsed();
            let update_request = UpdateScanSessionRequest {
                status: Some(final_status),
                progress: Some(100.0),
                ..Default::default()
            };

            if let Err(e) = scan_session_service
                .update_session(session_id, update_request)
                .await
            {
                error!("更新扫描会话最终状态失败: {}", e);
            }

            // 从运行列表中移除
            running_scans.write().await.remove(&session_id);

            info!("扫描 {} 完成，耗时: {:?}", session_id, duration);
        });

        Ok(task)
    }

    /// 执行扫描工作流
    async fn execute_scan_workflow(
        session: ScanSession,
        tool_manager: Arc<ToolManager>,
        scan_session_service: Arc<ScanSessionService>,
        event_sender: mpsc::UnboundedSender<ScanEngineEvent>,
        config: ScanEngineConfig,
    ) -> Result<()> {
        let session_id = session.id;
        let target = session.target.clone();
        let scan_config: Value = session.config;

        // 定义扫描阶段
        let stages = Self::define_scan_stages(&session.scan_type, &target, &scan_config)?;

        // 更新总阶段数
        let update_request = UpdateScanSessionRequest {
            total_stages: Some(stages.len() as i32),
            ..Default::default()
        };
        scan_session_service
            .update_session(session_id, update_request)
            .await?;

        let mut completed_stages = 0;
        let mut all_results = HashMap::new();

        // 执行每个阶段
        for (index, stage_config) in stages.iter().enumerate() {
            let stage_name = stage_config.stage_name.clone();

            // 发送阶段开始事件
            let _ = event_sender.send(ScanEngineEvent::StageStarted {
                session_id,
                stage_name: stage_name.clone(),
            });

            // 更新当前阶段
            let update_request = UpdateScanSessionRequest {
                current_stage: Some(stage_name.clone()),
                ..Default::default()
            };
            scan_session_service
                .update_session(session_id, update_request)
                .await?;

            // 创建扫描阶段记录
            let stage = ScanStage {
                id: Uuid::new_v4(),
                session_id,
                stage_name: stage_name.clone(),
                stage_order: index as i32,
                status: ScanStageStatus::Running,
                tool_name: stage_config.tool_name.clone(),
                config: stage_config.config.clone(),
                results: None,
                error_message: None,
                started_at: Some(chrono::Utc::now()),
                completed_at: None,
                duration_ms: None,
            };

            scan_session_service.create_stage(stage.clone()).await?;

            // 执行阶段
            let stage_start = Instant::now();
            let stage_result =
                Self::execute_stage(&stage_config, &target, &all_results, &tool_manager).await;

            let stage_duration = stage_start.elapsed();

            // 更新阶段结果
            let mut updated_stage = stage;
            match stage_result {
                Ok(result) => {
                    updated_stage.status = ScanStageStatus::Completed;
                    updated_stage.results = Some(result.clone());
                    all_results.insert(stage_name.clone(), result);
                    completed_stages += 1;

                    // 发送阶段完成事件
                    let _ = event_sender.send(ScanEngineEvent::StageCompleted {
                        session_id,
                        stage_name: stage_name.clone(),
                    });
                }
                Err(e) => {
                    updated_stage.status = ScanStageStatus::Failed;
                    updated_stage.error_message = Some(e.to_string());
                    warn!("阶段 {} 执行失败: {}", stage_name, e);
                }
            }

            updated_stage.completed_at = Some(chrono::Utc::now());
            updated_stage.duration_ms = Some(stage_duration.as_millis() as i64);

            scan_session_service.update_stage(&updated_stage).await?;

            // 更新整体进度
            let progress = (completed_stages as f64 / stages.len() as f64) * 100.0;
            let update_request = UpdateScanSessionRequest {
                progress: Some(progress as f32),
                completed_stages: Some(completed_stages),
                ..Default::default()
            };
            scan_session_service
                .update_session(session_id, update_request)
                .await?;

            // 发送进度更新事件
            let _ = event_sender.send(ScanEngineEvent::ProgressUpdated {
                session_id,
                progress,
            });

            // 阶段间延迟
            if index < stages.len() - 1 {
                sleep(Duration::from_millis(config.stage_delay_ms)).await;
            }
        }

        // 生成最终结果摘要
        let results_summary = Self::generate_results_summary(&all_results)?;
        let update_request = UpdateScanSessionRequest {
            results_summary: Some(results_summary),
            ..Default::default()
        };
        scan_session_service
            .update_session(session_id, update_request)
            .await?;

        Ok(())
    }

    /// 定义扫描阶段
    fn define_scan_stages(
        scan_type: &str,
        target: &str,
        config: &Value,
    ) -> Result<Vec<StageConfig>> {
        let mut stages = Vec::new();

        match scan_type {
            "comprehensive" => {
                // 综合扫描：子域名 -> 端口扫描 -> 漏洞扫描
                stages.push(StageConfig {
                    stage_name: "子域名发现".to_string(),
                    tool_name: "subdomain_scanner".to_string(),
                    config: serde_json::json!({
                        "target": target,
                        "wordlist_size": config.get("subdomain_wordlist_size").unwrap_or(&serde_json::json!(1000)),
                        "threads": config.get("subdomain_threads").unwrap_or(&serde_json::json!(50))
                    }),
                });

                stages.push(StageConfig {
                    stage_name: "端口扫描".to_string(),
                    tool_name: "port_scanner".to_string(),
                    config: serde_json::json!({
                        "target": target,
                        "ports": config.get("ports").unwrap_or(&serde_json::json!("1-1000")),
                        "threads": config.get("port_threads").unwrap_or(&serde_json::json!(100))
                    }),
                });
            }
            "subdomain" => {
                // 仅子域名扫描
                stages.push(StageConfig {
                    stage_name: "子域名发现".to_string(),
                    tool_name: "subdomain_scanner".to_string(),
                    config: serde_json::json!({
                        "target": target,
                        "wordlist_size": config.get("wordlist_size").unwrap_or(&serde_json::json!(1000)),
                        "threads": config.get("threads").unwrap_or(&serde_json::json!(50))
                    }),
                });
            }
            "port" => {
                // 仅端口扫描
                stages.push(StageConfig {
                    stage_name: "端口扫描".to_string(),
                    tool_name: "port_scanner".to_string(),
                    config: serde_json::json!({
                        "target": target,
                        "ports": config.get("ports").unwrap_or(&serde_json::json!("1-1000")),
                        "threads": config.get("threads").unwrap_or(&serde_json::json!(100))
                    }),
                });
            }
            _ => {
                return Err(anyhow!("不支持的扫描类型: {}", scan_type));
            }
        }

        Ok(stages)
    }

    /// 执行单个阶段
    async fn execute_stage(
        stage_config: &StageConfig,
        target: &str,
        previous_results: &HashMap<String, Value>,
        tool_manager: &ToolManager,
    ) -> Result<Value> {
        // 获取工具
        let tool = tool_manager
            .get_tool(&stage_config.tool_name)
            .await
            .ok_or_else(|| anyhow!("工具不存在: {}", stage_config.tool_name))?;

        // 准备扫描配置
        let mut scan_config = ScanConfig::new(target.to_string());

        // 合并配置
        if let Some(config_obj) = stage_config.config.as_object() {
            for (key, value) in config_obj {
                scan_config.set_parameter(key.clone(), value.clone());
            }
        }

        // 如果有前置结果，可以用于优化当前阶段
        if stage_config.stage_name == "端口扫描" {
            if let Some(subdomain_results) = previous_results.get("子域名发现") {
                // 使用子域名结果优化端口扫描目标
                if let Some(subdomains) = subdomain_results.get("subdomains") {
                    scan_config.set_parameter("targets".to_string(), subdomains.clone());
                }
            }
        }

        // 执行扫描
        let scan_result = tool.scan(scan_config).await?;

        // 转换结果为JSON
        let json_result = serde_json::to_value(scan_result)?;

        Ok(json_result)
    }

    /// 生成结果摘要
    fn generate_results_summary(results: &HashMap<String, Value>) -> Result<Value> {
        let mut summary = serde_json::Map::new();

        for (stage_name, result) in results {
            let stage_summary = match stage_name.as_str() {
                "子域名发现" => {
                    let subdomain_count = result
                        .get("subdomains")
                        .and_then(|s| s.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0);

                    serde_json::json!({
                        "found_subdomains": subdomain_count,
                        "status": "completed"
                    })
                }
                "端口扫描" => {
                    let open_ports = result
                        .get("open_ports")
                        .and_then(|p| p.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0);

                    serde_json::json!({
                        "open_ports_count": open_ports,
                        "status": "completed"
                    })
                }
                _ => {
                    serde_json::json!({
                        "status": "completed"
                    })
                }
            };

            summary.insert(stage_name.clone(), stage_summary);
        }

        Ok(Value::Object(summary))
    }
}

/// 阶段配置
#[derive(Debug, Clone)]
struct StageConfig {
    stage_name: String,
    tool_name: String,
    config: Value,
}
