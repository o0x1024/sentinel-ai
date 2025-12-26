//! Port scanning tool using rig-core Tool trait

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

/// Port scan arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PortScanArgs {
    /// Target IP address to scan
    pub target: String,
    /// Port range or list (e.g., "1-1000", "80,443,8080", or "common")
    #[serde(default = "default_ports")]
    pub ports: String,
    /// Number of concurrent threads (1-1000)
    #[serde(default = "default_threads")]
    pub threads: usize,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_ports() -> String { "common".to_string() }
fn default_threads() -> usize { 100 }
fn default_timeout() -> u64 { 3 }

/// Port scan result
#[derive(Debug, Clone, Serialize)]
pub struct PortScanOutput {
    pub target: String,
    pub open_ports: Vec<PortInfo>,
    pub total_ports_scanned: usize,
    pub open_count: usize,
    pub scan_duration_ms: u64,
}

/// Information about a single port
#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub status: String,
    pub service: Option<String>,
    pub response_time_ms: u64,
}

/// Port scan errors
#[derive(Debug, thiserror::Error)]
pub enum PortScanError {
    #[error("Invalid target IP: {0}")]
    InvalidTarget(String),
    #[error("Invalid port configuration: {0}")]
    InvalidPorts(String),
    #[error("Scan failed: {0}")]
    ScanFailed(String),
}

/// Port scan tool
#[derive(Debug, Clone, Default)]
pub struct PortScanTool;

impl PortScanTool {
    /// Get common ports list
    fn common_ports() -> Vec<u16> {
        vec![
            21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 993, 995,
            1723, 3306, 3389, 5432, 5900, 8080, 8443,
        ]
    }

    /// Parse port specification
    fn parse_ports(ports_str: &str) -> Result<Vec<u16>, PortScanError> {
        if ports_str == "common" {
            return Ok(Self::common_ports());
        }

        let mut ports = Vec::new();
        for part in ports_str.split(',') {
            let part = part.trim();
            if part.contains('-') {
                let range: Vec<&str> = part.split('-').collect();
                if range.len() == 2 {
                    let start: u16 = range[0].parse()
                        .map_err(|_| PortScanError::InvalidPorts(format!("Invalid range: {}", part)))?;
                    let end: u16 = range[1].parse()
                        .map_err(|_| PortScanError::InvalidPorts(format!("Invalid range: {}", part)))?;
                    for port in start..=end {
                        ports.push(port);
                    }
                }
            } else {
                let port: u16 = part.parse()
                    .map_err(|_| PortScanError::InvalidPorts(format!("Invalid port: {}", part)))?;
                ports.push(port);
            }
        }

        if ports.is_empty() {
            return Err(PortScanError::InvalidPorts("No valid ports specified".to_string()));
        }
        Ok(ports)
    }

    /// Identify service by port number
    fn identify_service(port: u16) -> Option<String> {
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
            8443 => Some("HTTPS-Alt".to_string()),
            _ => None,
        }
    }

    /// Scan a single port
    async fn scan_port(target: IpAddr, port: u16, timeout_ms: u64) -> PortInfo {
        let start = Instant::now();
        let addr = SocketAddr::new(target, port);

        match timeout(Duration::from_millis(timeout_ms), TcpStream::connect(addr)).await {
            Ok(Ok(_)) => PortInfo {
                port,
                status: "open".to_string(),
                service: Self::identify_service(port),
                response_time_ms: start.elapsed().as_millis() as u64,
            },
            Ok(Err(_)) => PortInfo {
                port,
                status: "closed".to_string(),
                service: None,
                response_time_ms: start.elapsed().as_millis() as u64,
            },
            Err(_) => PortInfo {
                port,
                status: "filtered".to_string(),
                service: None,
                response_time_ms: timeout_ms,
            },
        }
    }
}

impl Tool for PortScanTool {
    const NAME: &'static str = "port_scan";
    type Args = PortScanArgs;
    type Output = PortScanOutput;
    type Error = PortScanError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "High-performance TCP port scanner with service identification. Scans target IP for open ports.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(PortScanArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Parse target IP
        let target_ip: IpAddr = args.target.parse()
            .map_err(|_| PortScanError::InvalidTarget(args.target.clone()))?;

        // Parse ports
        let ports = Self::parse_ports(&args.ports)?;

        // Validate threads
        let threads = args.threads.clamp(1, 1000);
        let timeout_ms = args.timeout_secs * 1000;

        // Scan ports concurrently
        let semaphore = Arc::new(Semaphore::new(threads));
        let mut tasks = Vec::new();

        for port in &ports {
            let sem = semaphore.clone();
            let port = *port;

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::scan_port(target_ip, port, timeout_ms).await
            });
            tasks.push(task);
        }

        // Collect results
        let mut open_ports = Vec::new();
        for task in tasks {
            if let Ok(result) = task.await {
                if result.status == "open" {
                    open_ports.push(result);
                }
            }
        }
        
        // Sort by port number
        open_ports.sort_by_key(|p| p.port);
        
        let max_ports = 500; // Hard limit to prevent huge JSON
        if open_ports.len() > max_ports {
             open_ports.truncate(max_ports);
             // Maybe add a warning or indicator in the output struct, but struct is fixed.
             // We'll just truncate for now, the user can scan smaller ranges if needed.
        }

        let scan_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(PortScanOutput {
            target: args.target,
            open_count: open_ports.len(),
            total_ports_scanned: ports.len(),
            open_ports,
            scan_duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ports() {
        assert!(PortScanTool::parse_ports("common").is_ok());
        assert!(PortScanTool::parse_ports("80,443,8080").is_ok());
        assert!(PortScanTool::parse_ports("1-100").is_ok());
        assert!(PortScanTool::parse_ports("invalid").is_err());
    }

    #[test]
    fn test_identify_service() {
        assert_eq!(PortScanTool::identify_service(80), Some("HTTP".to_string()));
        assert_eq!(PortScanTool::identify_service(443), Some("HTTPS".to_string()));
        assert_eq!(PortScanTool::identify_service(12345), None);
    }
}

