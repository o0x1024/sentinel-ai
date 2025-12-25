//! Subdomain brute-force tool using rig-core Tool trait

use rig::tool::Tool;
use rsubdomain::{SubdomainBruteConfig, SubdomainBruteEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Subdomain brute-force arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubdomainBruteArgs {
    /// Target domain(s) to scan, comma-separated for multiple domains
    pub domains: String,
    /// DNS resolvers (comma-separated, e.g., "8.8.8.8,1.1.1.1")
    #[serde(default = "default_resolvers")]
    pub resolvers: String,
    /// Dictionary file path (optional, uses built-in if not provided)
    #[serde(default)]
    pub dictionary_file: Option<String>,
    /// Dictionary words (comma-separated, e.g., "www,mail,api,admin")
    #[serde(default)]
    pub dictionary: Option<String>,
    /// Skip wildcard domains
    #[serde(default = "default_skip_wildcard")]
    pub skip_wildcard: bool,
    /// Bandwidth limit (e.g., "5M", "10M")
    #[serde(default = "default_bandwidth")]
    pub bandwidth_limit: Option<String>,
    /// Enable HTTP/HTTPS verification
    #[serde(default = "default_verify_mode")]
    pub verify_mode: bool,
    /// Enable DNS record resolution
    #[serde(default = "default_resolve_records")]
    pub resolve_records: bool,
}

fn default_resolvers() -> String {
    "8.8.8.8,1.1.1.1,223.5.5.5".to_string()
}
fn default_skip_wildcard() -> bool {
    true
}
fn default_bandwidth() -> Option<String> {
    Some("5M".to_string())
}
fn default_verify_mode() -> bool {
    true
}
fn default_resolve_records() -> bool {
    true
}

/// Single subdomain result
#[derive(Debug, Clone, Serialize)]
pub struct SubdomainInfo {
    pub domain: String,
    pub ip: String,
    pub record_type: String,
    /// HTTP verification result
    pub http_status: Option<u16>,
    /// HTTPS verification result
    pub https_status: Option<u16>,
    /// Page title
    pub title: Option<String>,
    /// DNS records count
    pub dns_records_count: Option<usize>,
}

/// Subdomain brute-force result
#[derive(Debug, Clone, Serialize)]
pub struct SubdomainBruteOutput {
    pub target_domains: Vec<String>,
    pub subdomains: Vec<SubdomainInfo>,
    pub total_found: usize,
    pub scan_duration_ms: u64,
}

/// Subdomain brute errors
#[derive(Debug, thiserror::Error)]
pub enum SubdomainBruteError {
    #[error("Invalid domain: {0}")]
    InvalidDomain(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Scan failed: {0}")]
    ScanFailed(String),
}

/// Subdomain brute-force tool
#[derive(Debug, Clone, Default)]
pub struct SubdomainBruteTool;

impl SubdomainBruteTool {
    /// Parse comma-separated strings
    fn parse_list(input: &str) -> Vec<String> {
        input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl Tool for SubdomainBruteTool {
    const NAME: &'static str = "subdomain_brute";
    type Args = SubdomainBruteArgs;
    type Output = SubdomainBruteOutput;
    type Error = SubdomainBruteError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "High-performance subdomain brute-force scanner. Discovers subdomains using dictionary attack with DNS resolution, HTTP/HTTPS verification, and wildcard detection.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubdomainBruteArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Parse domains
        let domains = Self::parse_list(&args.domains);
        if domains.is_empty() {
            return Err(SubdomainBruteError::InvalidDomain(
                "No valid domains provided".to_string(),
            ));
        }

        // Parse resolvers
        let resolvers = Self::parse_list(&args.resolvers);

        // Parse dictionary if provided
        let dictionary = args.dictionary.map(|d| Self::parse_list(&d));

        // Create config
        let config = SubdomainBruteConfig {
            domains: domains.clone(),
            resolvers,
            dictionary_file: args.dictionary_file,
            dictionary,
            skip_wildcard: args.skip_wildcard,
            bandwidth_limit: args.bandwidth_limit,
            verify_mode: args.verify_mode,
            resolve_records: args.resolve_records,
            silent: true,
            device: None,
        };

        // Run in blocking context because rsubdomain is not Send-safe
        let results = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let engine = SubdomainBruteEngine::new(config)
                    .await
                    .map_err(|e| e.to_string())?;
                engine
                    .run_brute_force()
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await
        .map_err(|e| SubdomainBruteError::ScanFailed(e.to_string()))?
        .map_err(SubdomainBruteError::ScanFailed)?;

        // Convert results
        let subdomains: Vec<SubdomainInfo> = results
            .iter()
            .map(|r| {
                let (http_status, https_status, title) = if let Some(ref verified) = r.verified {
                    (verified.http_status, verified.https_status, verified.title.clone())
                } else {
                    (None, None, None)
                };

                let dns_records_count = r.dns_records.as_ref().map(|d| d.records.len());

                SubdomainInfo {
                    domain: r.domain.clone(),
                    ip: r.ip.clone(),
                    record_type: r.record_type.clone(),
                    http_status,
                    https_status,
                    title,
                    dns_records_count,
                }
            })
            .collect();

        let total_found = subdomains.len();
        let scan_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SubdomainBruteOutput {
            target_domains: domains,
            subdomains,
            total_found,
            scan_duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list() {
        let domains = SubdomainBruteTool::parse_list("example.com, test.com, demo.org");
        assert_eq!(domains.len(), 3);
        assert_eq!(domains[0], "example.com");

        let resolvers = SubdomainBruteTool::parse_list("8.8.8.8, 1.1.1.1");
        assert_eq!(resolvers.len(), 2);
    }
}

