use super::{ScanConfig, ScanResult, ScanStatus, ScanTool};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortResult {
    pub port: u16,
    pub status: PortStatus,
    pub service: Option<String>,
    pub banner: Option<String>,
    pub response_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortStatus {
    Open,
    Closed,
    Filtered,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanResults {
    pub target: String,
    pub ports_scanned: Vec<u16>,
    pub open_ports: Vec<PortResult>,
    pub scan_duration: u64,
    pub total_ports: usize,
    pub open_count: usize,
}

pub struct PortScanner {
    common_ports: Vec<u16>,
}

impl PortScanner {
    pub fn new() -> Self {
        Self {
            common_ports: Self::get_common_ports(),
        }
    }

    fn get_common_ports() -> Vec<u16> {
        vec![
            21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 993, 995, 1723, 3306, 3389, 5432,
            5900, 8080,
        ]
    }

    async fn scan_port(&self, target: IpAddr, port: u16, timeout_ms: u64) -> PortResult {
        let start_time = std::time::Instant::now();
        let socket_addr = SocketAddr::new(target, port);

        match timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(socket_addr),
        )
        .await
        {
            Ok(Ok(_stream)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                PortResult {
                    port,
                    status: PortStatus::Open,
                    service: self.identify_service(port),
                    banner: None, // TODO: 实现 banner 抓取
                    response_time,
                }
            }
            Ok(Err(_)) => PortResult {
                port,
                status: PortStatus::Closed,
                service: None,
                banner: None,
                response_time: start_time.elapsed().as_millis() as u64,
            },
            Err(_) => PortResult {
                port,
                status: PortStatus::Timeout,
                service: None,
                banner: None,
                response_time: timeout_ms,
            },
        }
    }

    fn identify_service(&self, port: u16) -> Option<String> {
        match port {
            21 => Some("FTP".to_string()),
            22 => Some("SSH".to_string()),
            23 => Some("Telnet".to_string()),
            25 => Some("SMTP".to_string()),
            53 => Some("DNS".to_string()),
            80 => Some("HTTP".to_string()),
            110 => Some("POP3".to_string()),
            143 => Some("IMAP".to_string()),
            443 => Some("HTTPS".to_string()),
            993 => Some("IMAPS".to_string()),
            995 => Some("POP3S".to_string()),
            3306 => Some("MySQL".to_string()),
            3389 => Some("RDP".to_string()),
            5432 => Some("PostgreSQL".to_string()),
            5900 => Some("VNC".to_string()),
            8080 => Some("HTTP-Alt".to_string()),
            _ => None,
        }
    }

    async fn scan_ports(
        &self,
        target: IpAddr,
        ports: Vec<u16>,
        config: &ScanConfig,
    ) -> anyhow::Result<PortScanResults> {
        let start_time = std::time::Instant::now();
        let semaphore = Arc::new(Semaphore::new(config.threads));
        let mut tasks = Vec::new();

        for port in &ports {
            let semaphore = semaphore.clone();
            let scanner = self.clone();
            let port = *port;
            let timeout_ms = config.timeout * 1000;

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                scanner.scan_port(target, port, timeout_ms).await
            });

            tasks.push(task);
        }

        let mut all_results = Vec::new();
        for task in tasks {
            if let Ok(result) = task.await {
                all_results.push(result);
            }
        }

        let open_ports: Vec<PortResult> = all_results
            .into_iter()
            .filter(|r| matches!(r.status, PortStatus::Open))
            .collect();

        let scan_duration = start_time.elapsed().as_secs();

        let total_ports = ports.len();

        Ok(PortScanResults {
            target: target.to_string(),
            ports_scanned: ports,
            open_count: open_ports.len(),
            total_ports,
            open_ports,
            scan_duration,
        })
    }
}

impl Clone for PortScanner {
    fn clone(&self) -> Self {
        Self {
            common_ports: self.common_ports.clone(),
        }
    }
}

#[async_trait]
impl ScanTool for PortScanner {
    fn name(&self) -> &str {
        "port_scanner"
    }

    fn description(&self) -> &str {
        "高性能TCP端口扫描工具，支持服务识别和并发扫描"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn validate_config(&self, config: &ScanConfig) -> anyhow::Result<()> {
        if config.target.is_empty() {
            return Err(anyhow::anyhow!("目标IP地址不能为空"));
        }

        config
            .target
            .parse::<IpAddr>()
            .map_err(|_| anyhow::anyhow!("无效的IP地址格式"))?;

        if config.threads == 0 || config.threads > 1000 {
            return Err(anyhow::anyhow!("线程数必须在1-1000之间"));
        }

        Ok(())
    }

    async fn scan(&self, config: ScanConfig) -> anyhow::Result<ScanResult> {
        let scan_id = Uuid::new_v4();
        let started_at = chrono::Utc::now();

        let target_ip: IpAddr = config.target.parse()?;

        // 确定要扫描的端口
        let ports = if let Some(ports_value) = config.options.get("ports") {
            serde_json::from_value(ports_value.clone())?
        } else {
            self.common_ports.clone()
        };

        match self.scan_ports(target_ip, ports, &config).await {
            Ok(results) => Ok(ScanResult {
                id: scan_id,
                tool_name: self.name().to_string(),
                target: config.target,
                status: ScanStatus::Completed,
                results: serde_json::to_value(results)?,
                started_at,
                completed_at: Some(chrono::Utc::now()),
                error: None,
            }),
            Err(e) => Ok(ScanResult {
                id: scan_id,
                tool_name: self.name().to_string(),
                target: config.target,
                status: ScanStatus::Failed,
                results: serde_json::Value::Null,
                started_at,
                completed_at: Some(chrono::Utc::now()),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn cancel(&self, _scan_id: Uuid) -> anyhow::Result<()> {
        // TODO: 实现扫描取消逻辑
        Ok(())
    }

    fn default_config(&self) -> ScanConfig {
        ScanConfig {
            target: String::new(),
            timeout: 3,
            threads: 100,
            options: HashMap::new(),
        }
    }

    fn supported_options(&self) -> Vec<String> {
        vec![
            "ports".to_string(),
            "port_range".to_string(),
            "service_detection".to_string(),
            "banner_grab".to_string(),
        ]
    }
}
