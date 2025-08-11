//! 内置工具提供者
//! 
//! 提供系统内置的安全扫描和分析工具

use super::*;
use crate::services::database::DatabaseService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::process::Command as AsyncCommand;
use tokio::sync::Semaphore;
use tokio::time::timeout as tokio_timeout;
use tracing::{error, info, warn};
use uuid::Uuid;

// ============================================================================
// 内置工具提供者
// ============================================================================

#[derive(Debug)]
pub struct BuiltinToolProvider {
    tools: Vec<Arc<dyn UnifiedTool>>,
}

impl BuiltinToolProvider {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();
        
        // 添加内置工具（只保留完全实现的工具）
        tools.push(Arc::new(PortScanTool::new()));
        tools.push(Arc::new(RSubdomainTool::new(db_service)));
        
        Self { tools }
    }
}

#[async_trait]
impl ToolProvider for BuiltinToolProvider {
    fn name(&self) -> &str {
        "builtin"
    }

    fn description(&self) -> &str {
        "Built-in security scanning and analysis tools"
    }

    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>> {
        Ok(self.tools.clone())
    }

    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>> {
        for tool in &self.tools {
            if tool.name() == name {
                return Ok(Some(tool.clone()));
            }
        }
        Ok(None)
    }

    async fn refresh(&self) -> Result<()> {
        info!("Refreshing builtin tools");
        // 内置工具不需要刷新
        Ok(())
    }
}

// ============================================================================
// 端口扫描相关结构体
// ============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub target: String,
    pub threads: usize,
    pub timeout: u64,
}



// ============================================================================
// 端口扫描工具
// ============================================================================

#[derive(Debug)]
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

        match tokio_timeout(
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

#[derive(Debug)]
pub struct PortScanTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    scanner: PortScanner,
}

impl PortScanTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "2.0.0".to_string(),
            license: "MIT".to_string(),
            tags: vec!["port".to_string(), "scanning".to_string(), "network".to_string(), "tcp".to_string()],
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "target".to_string(),
                    param_type: ParameterType::String,
                    description: "Target IP address to scan".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "ports".to_string(),
                    param_type: ParameterType::String,
                    description: "Port range or list (e.g., '1-1000' or '80,443,8080' or 'common')".to_string(),
                    required: false,
                    default_value: Some(json!("common")),
                },
                ParameterDefinition {
                    name: "threads".to_string(),
                    param_type: ParameterType::Number,
                    description: "Number of concurrent threads (1-1000)".to_string(),
                    required: false,
                    default_value: Some(json!(100)),
                },
                ParameterDefinition {
                    name: "timeout".to_string(),
                    param_type: ParameterType::Number,
                    description: "Connection timeout in seconds".to_string(),
                    required: false,
                    default_value: Some(json!(3)),
                },
            ],
        };

        Self { 
            metadata, 
            parameters,
            scanner: PortScanner::new(),
        }
    }

    fn parse_ports(&self, ports_str: &str) -> Vec<u16> {
        if ports_str == "common" {
            return self.scanner.common_ports.clone();
        }

        let mut ports = Vec::new();
        
        for part in ports_str.split(',') {
            let part = part.trim();
            if part.contains('-') {
                // 处理范围，如 "1-1000"
                let range_parts: Vec<&str> = part.split('-').collect();
                if range_parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (range_parts[0].parse::<u16>(), range_parts[1].parse::<u16>()) {
                        for port in start..=end {
                            ports.push(port);
                        }
                    }
                }
            } else {
                // 处理单个端口
                if let Ok(port) = part.parse::<u16>() {
                    ports.push(port);
                }
            }
        }
        
        ports
    }
}

#[async_trait]
impl UnifiedTool for PortScanTool {
    fn name(&self) -> &str {
        "port_scan"
    }

    fn description(&self) -> &str {
        "高性能TCP端口扫描工具，支持服务识别和并发扫描"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::NetworkScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        // 内置端口扫描器总是可用
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        
        info!("Executing advanced port scan with execution_id: {}", execution_id);
        
        // 验证参数
        let target = params.inputs.get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Target parameter is required"))?;
        
        if target.is_empty() {
            return Err(anyhow!("目标IP地址不能为空"));
        }
        
        let target_ip = target.parse::<IpAddr>()
            .map_err(|_| anyhow!("无效的IP地址格式"))?;
        
        let ports_str = params.inputs.get("ports")
            .and_then(|v| v.as_str())
            .unwrap_or("common");
        
        let threads = params.inputs.get("threads")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;
        
        if threads == 0 || threads > 1000 {
            return Err(anyhow!("线程数必须在1-1000之间"));
        }
        
        let timeout = params.inputs.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(3);
        
        // 解析端口列表
        let ports = self.parse_ports(ports_str);
        if ports.is_empty() {
            return Err(anyhow!("无效的端口配置"));
        }
        
        // 创建扫描配置
        let config = ScanConfig {
            target: target.to_string(),
            threads,
            timeout,
        };
        
        // 执行扫描
        match self.scanner.scan_ports(target_ip, ports, &config).await {
            Ok(scan_results) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                
                let result_json = json!({
                    "target": scan_results.target,
                    "ports_scanned": scan_results.ports_scanned,
                    "open_ports": scan_results.open_ports,
                    "open_count": scan_results.open_count,
                    "total_ports": scan_results.total_ports,
                    "scan_duration": scan_results.scan_duration,
                    "scan_summary": {
                        "target_ip": target,
                        "threads_used": threads,
                        "timeout_seconds": timeout,
                        "ports_config": ports_str
                    }
                });
                
                info!("Port scan completed for {}: {}/{} ports open in {}s", 
                      target, scan_results.open_count, scan_results.total_ports, scan_results.scan_duration);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    success: true,
                    output: result_json,
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                error!("Port scan execution failed: {}", e);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

// ============================================================================
// RSubdomain 工具 - 基于rsubdomain库的高性能子域名扫描器
// ============================================================================

use rsubdomain::{SubdomainBruteConfig, SubdomainBruteEngine};


/// 子域名扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainResult {
    pub subdomain: String,
    pub ip_addresses: Vec<IpAddr>,
    pub record_type: Option<String>,
    pub discovered_at: u64, // Unix timestamp
}

/// 子域名扫描结果集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainScanResults {
    pub target_domain: String,
    pub subdomains: Vec<SubdomainResult>,
    pub total_found: usize,
    pub scan_duration: u64,
}

#[derive(Debug)]
pub struct RSubdomainTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    db_service: Option<Arc<DatabaseService>>,
}

impl RSubdomainTool {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        let metadata = ToolMetadata {
            author: "RSubdomain Team".to_string(),
            version: "2.0.0".to_string(),
            license: "MIT".to_string(),
            tags: vec!["subdomain".to_string(), "enumeration".to_string(), "fast".to_string(), "rsubdomain".to_string()],
            requirements: vec!["rsubdomain".to_string()],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "domain".to_string(),
                    param_type: ParameterType::String,
                    description: "Target domain to enumerate subdomains".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "use_database_wordlist".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Use wordlist from database".to_string(),
                    required: false,
                    default_value: Some(json!(true)),
                },
            ],
        };

        Self { 
            metadata, 
            parameters,
            db_service: Some(db_service),
        }
    }

    /// 设置数据库服务
    pub fn with_database(mut self, db_service: Arc<DatabaseService>) -> Self {
        self.db_service = Some(db_service);
        self
    }

    /// 从数据库加载字典
    async fn load_wordlist(&self) -> Result<Vec<String>> {
        if let Some(ref db_service) = self.db_service {
            match db_service.get_subdomain_dictionary().await {
                Ok(dictionary) => {
                    if dictionary.is_empty() {
                        Ok(self.get_fallback_wordlist())
                    } else {
                        Ok(dictionary)
                    }
                }
                Err(_) => {
                    Ok(self.get_fallback_wordlist())
                }
            }
        } else {
            Ok(self.get_fallback_wordlist())
        }
    }

    /// 获取默认字典
    fn get_fallback_wordlist(&self) -> Vec<String> {
        vec![
            "www".to_string(),
            "mail".to_string(),
            "ftp".to_string(),
            "admin".to_string(),
            "api".to_string(),
            "app".to_string(),
            "blog".to_string(),
            "cdn".to_string(),
            "dev".to_string(),
            "docs".to_string(),
            "forum".to_string(),
            "help".to_string(),
            "m".to_string(),
            "mobile".to_string(),
            "news".to_string(),
            "shop".to_string(),
            "staging".to_string(),
            "support".to_string(),
            "test".to_string(),
            "vpn".to_string(),
            "secure".to_string(),
            "login".to_string(),
            "portal".to_string(),
            "dashboard".to_string(),
            "beta".to_string(),
            "alpha".to_string(),
            "demo".to_string(),
            "old".to_string(),
            "new".to_string(),
            "backup".to_string(),
        ]
    }

    /// 执行子域名扫描
    async fn scan_subdomains(
        &self,
        domain: &str,
        use_database_wordlist: bool,
    ) -> Result<SubdomainScanResults> {
        let start_time = std::time::Instant::now();
        let domain_str = domain.to_string();
        let domain_clone = domain_str.clone();

        // 加载字典
        let wordlist = if use_database_wordlist {
            self.load_wordlist().await?
        } else {
            self.get_fallback_wordlist()
        };

        info!("Starting subdomain scan for {} with {} words", domain, wordlist.len());

        // 在阻塞线程中执行rsubdomain扫描
        let results = tokio::task::spawn_blocking(move || {
            // 使用rsubdomain库进行子域名扫描
            let mut brute_config = SubdomainBruteConfig::default();
            brute_config.domains = vec![domain_clone];
            brute_config.dictionary = Some(wordlist);

            // 创建运行时并执行扫描
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| anyhow!("Failed to create runtime: {}", e))?;
            
            rt.block_on(async {
                let engine = SubdomainBruteEngine::new(brute_config)
                    .await
                    .map_err(|e| anyhow!("Failed to create engine: {}", e))?;
                
                engine
                    .run_brute_force()
                    .await
                    .map_err(|e| anyhow!("Scan failed: {}", e))
            })
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow!("Subdomain scan failed: {}", e))?;

        // 转换结果格式
        let mut found_subdomains = Vec::new();
        for result in results {
            found_subdomains.push(SubdomainResult {
                subdomain: result.domain,
                ip_addresses: vec![result
                    .ip
                    .parse::<IpAddr>()
                    .unwrap_or_else(|_| "0.0.0.0".parse().unwrap())],
                record_type: Some(result.record_type),
                discovered_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        let scan_duration = start_time.elapsed().as_secs();

        Ok(SubdomainScanResults {
            target_domain: domain_str,
            total_found: found_subdomains.len(),
            subdomains: found_subdomains,
            scan_duration,
        })
    }
}

#[async_trait]
impl UnifiedTool for RSubdomainTool {
    fn name(&self) -> &str {
        "rsubdomain"
    }

    fn description(&self) -> &str {
        "高性能子域名扫描工具，使用rsubdomain库进行并发DNS解析和子域名暴破发现"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::ServiceDetection
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        // 检查rsubdomain库是否可用（这里总是返回true，因为我们直接使用库而不是外部命令）
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        
        info!("Executing rsubdomain scan with execution_id: {}", execution_id);
        
        // 验证参数
        let domain = params.inputs.get("domain")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Domain parameter is required"))?;
        
        if domain.is_empty() {
            return Err(anyhow!("目标域名不能为空"));
        }
        
        let threads = params.inputs.get("threads")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);
        
        if threads == 0 || threads > 1000 {
            return Err(anyhow!("线程数必须在1-1000之间"));
        }
        
        let timeout = params.inputs.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);
        
        let use_database_wordlist = params.inputs.get("use_database_wordlist")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // 执行扫描
        match self.scan_subdomains(domain, use_database_wordlist).await {
            Ok(scan_results) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                
                let result_json = json!({
                    "target_domain": scan_results.target_domain,
                    "subdomains": scan_results.subdomains,
                    "total_found": scan_results.total_found,
                    "scan_duration": scan_results.scan_duration,
                    "scan_summary": {
                        "domain": domain,
                        "timeout_seconds": timeout,
                        "wordlist_source": if use_database_wordlist { "database" } else { "builtin" }
                    }
                });
                
                info!("Subdomain scan completed for {}: {} subdomains found in {}s", 
                      domain, scan_results.total_found, scan_results.scan_duration);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    success: true,
                    output: result_json,
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                error!("RSubdomain execution failed: {}", e);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

//写一个域名扫描的单元测试
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rsubdomain_scan() {
        let db_service = Arc::new(DatabaseService::new());
        let tool = RSubdomainTool::new(db_service);
        let result = tool.scan_subdomains("mgtv.com", true).await;
        println!("result: {:?}", result);
    }
}