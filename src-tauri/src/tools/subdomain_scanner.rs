use super::{ScanConfig, ScanResult, ScanStatus, ScanTool};
use crate::services::database::DatabaseService;
use async_trait::async_trait;
use rsubdomain::{SubdomainBruteConfig, SubdomainBruteEngine};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainResult {
    pub subdomain: String,
    pub ip_addresses: Vec<IpAddr>,
    pub record_type: Option<String>,
    pub discovered_at: u64, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainScanResults {
    pub target_domain: String,
    pub subdomains: Vec<SubdomainResult>,
    pub total_found: usize,
    pub scan_duration: u64,
}

pub struct SubdomainScanner {
    db_service: Arc<DatabaseService>,
}

impl SubdomainScanner {
    pub fn new(db_service: Arc<DatabaseService>) -> anyhow::Result<Self> {
        Ok(Self { db_service })
    }

    pub async fn load_wordlist(&self) -> anyhow::Result<Vec<String>> {
        match self.db_service.get_subdomain_dictionary().await {
            Ok(dictionary) => {
                if dictionary.is_empty() {
                    // 如果数据库中没有字典，返回默认字典
                    Ok(self.get_fallback_wordlist())
                } else {
                    Ok(dictionary)
                }
            }
            Err(_) => {
                // 数据库查询失败时使用默认字典
                Ok(self.get_fallback_wordlist())
            }
        }
    }

    fn get_fallback_wordlist(&self) -> Vec<String> {
        // 扩展的子域名字典，基于rsubdomain的常用子域名
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
        config: &ScanConfig,
    ) -> anyhow::Result<SubdomainScanResults> {
        let start_time = std::time::Instant::now();
        let domain_str = domain.to_string();
        let domain_clone = domain_str.clone();

        // 从数据库加载字典
        let _wordlist = self.load_wordlist().await?;

        // 在阻塞线程中执行rsubdomain扫描，因为它不是Send安全的
        let results = tokio::task::spawn_blocking(move || {
            // 使用rsubdomain库进行子域名扫描
            let mut brute_config = SubdomainBruteConfig::default();

            // 设置目标域名
            brute_config.domains = vec![domain_clone];

            // 创建引擎并执行扫描
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let engine = SubdomainBruteEngine::new(brute_config)
                    .await
                    .map_err(|e| anyhow::anyhow!("创建引擎失败: {}", e))?;
                engine
                    .run_brute_force()
                    .await
                    .map_err(|e| anyhow::anyhow!("扫描失败: {}", e))
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Subdomain scan failed: {}", e))?;

        // 转换结果格式
        let mut found_subdomains = Vec::new();
        // TODO: 根据实际的results类型来处理结果
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

impl Clone for SubdomainScanner {
    fn clone(&self) -> Self {
        Self {
            db_service: Arc::clone(&self.db_service),
        }
    }
}

#[async_trait]
impl ScanTool for SubdomainScanner {
    fn name(&self) -> &str {
        "subdomain_scanner"
    }

    fn description(&self) -> &str {
        "高性能子域名扫描工具，使用并发DNS解析进行子域名暴破，支持自定义字典和线程控制"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn validate_config(&self, config: &ScanConfig) -> anyhow::Result<()> {
        if config.target.is_empty() {
            return Err(anyhow::anyhow!("目标域名不能为空"));
        }

        if config.threads == 0 || config.threads > 1000 {
            return Err(anyhow::anyhow!("线程数必须在1-1000之间"));
        }

        Ok(())
    }

    async fn scan(&self, config: ScanConfig) -> anyhow::Result<ScanResult> {
        // 使用线程安全的UUID生成
        let scan_id = {
            use uuid::Uuid;
            Uuid::new_v4()
        };
        let started_at = chrono::Utc::now();

        match self.scan_subdomains(&config.target, &config).await {
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
            timeout: 30,
            threads: 50,
            options: HashMap::new(),
        }
    }

    fn supported_options(&self) -> Vec<String> {
        vec![
            "wordlist_name".to_string(),
            "dns_servers".to_string(),
            "recursive".to_string(),
        ]
    }
}
