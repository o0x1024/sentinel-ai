//! 子域名扫描工具

use crate::unified_types::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rsubdomain::{SubdomainBruteConfig, SubdomainBruteEngine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainResult {
    pub subdomain: String,
    pub ip_addresses: Vec<IpAddr>,
    pub record_type: Option<String>,
    pub discovered_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainScanResults {
    pub target_domain: String,
    pub subdomains: Vec<SubdomainResult>,
    pub total_found: usize,
    pub scan_duration: u64,
}

pub struct RSubdomainTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    db_service: Option<Arc<dyn DatabaseProvider>>,
}

impl std::fmt::Debug for RSubdomainTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RSubdomainTool")
            .field("metadata", &self.metadata)
            .field("parameters", &self.parameters)
            .field("db_service", &"<dyn DatabaseProvider>")
            .finish()
    }
}

impl RSubdomainTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            author: "RSubdomain Team".to_string(),
            version: "2.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["subdomain".to_string(), "enumeration".to_string(), "fast".to_string(), "rsubdomain".to_string()],
            install_command: None,
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
                    default_value: Some(json!(false)),
                },
            ],
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "domain": {"type": "string"},
                    "use_database_wordlist": {"type": "boolean"}
                },
                "required": ["domain"]
            }),
            required: vec!["domain".to_string()],
            optional: vec!["use_database_wordlist".to_string()],
        };

        Self { 
            metadata, 
            parameters,
            db_service: None,
        }
    }

    pub fn with_database(mut self, db_service: Arc<dyn DatabaseProvider>) -> Self {
        self.db_service = Some(db_service);
        self
    }

    async fn load_wordlist(&self) -> Result<Vec<String>> {
        if let Some(ref db_service) = self.db_service {
            match db_service.get_subdomain_dictionary().await {
                Ok(dictionary) => {
                    if dictionary.is_empty() {
                        Ok(self.get_buildin_wordlist())
                    } else {
                        Ok(dictionary)
                    }
                }
                Err(_) => {
                    Ok(self.get_buildin_wordlist())
                }
            }
        } else {
            Ok(self.get_buildin_wordlist())
        }
    }

    fn get_buildin_wordlist(&self) -> Vec<String> {
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

    async fn scan_subdomains(
        &self,
        domain: &str,
        use_database_wordlist: bool,
    ) -> Result<SubdomainScanResults> {
        let start_time = std::time::Instant::now();
        let domain_str = domain.to_string();
        let domain_clone = domain_str.clone();

        let wordlist = if use_database_wordlist {
            self.load_wordlist().await?
        } else {
            self.get_buildin_wordlist()
        };

        info!("Starting subdomain scan for {} with {} words", domain, wordlist.len());

        let results = tokio::task::spawn_blocking(move || {
            let mut brute_config = SubdomainBruteConfig::default();
            brute_config.domains = vec![domain_clone];
            brute_config.dictionary = Some(wordlist);

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
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        
        info!("Executing rsubdomain scan with execution_id: {}", execution_id);
        
        let domain = params.inputs.get("domain")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Domain parameter is required"))?;
        
        if domain.is_empty() {
            return Err(anyhow!("目标域名不能为空"));
        }
        
        let use_database_wordlist = params.inputs.get("use_database_wordlist")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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
                        "wordlist_source": if use_database_wordlist { "database" } else { "builtin" }
                    }
                });
                
                info!("Subdomain scan completed for {}: {} subdomains found in {}s", 
                      domain, scan_results.total_found, scan_results.scan_duration);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: true,
                    output: result_json,
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: ExecutionStatus::Completed,
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                error!("RSubdomain execution failed: {}", e);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    status: ExecutionStatus::Failed,
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    async fn get_subdomain_dictionary(&self) -> Result<Vec<String>>;
}

