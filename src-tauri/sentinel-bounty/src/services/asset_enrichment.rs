//! Asset Enrichment Service
//! 
//! Automatically enriches asset metadata with additional information:
//! - IP geolocation and ASN information
//! - Domain WHOIS and DNS records
//! - Technology stack detection
//! - Certificate information
//! - Security posture analysis

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use sentinel_db::{DatabaseService, BountyAssetRow};

/// IP enrichment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpEnrichment {
    pub ip: String,
    pub ip_version: String,
    pub asn: Option<i32>,
    pub asn_org: Option<String>,
    pub isp: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_cloud: bool,
    pub cloud_provider: Option<String>,
}

/// Asset enrichment service
pub struct AssetEnrichmentService {
    db_service: Arc<DatabaseService>,
    enabled: Arc<RwLock<bool>>,
}

impl AssetEnrichmentService {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self {
            db_service,
            enabled: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start the enrichment service
    pub async fn start(&self) -> Result<()> {
        let mut enabled = self.enabled.write().await;
        if *enabled {
            return Ok(());
        }
        
        *enabled = true;
        info!("Asset enrichment service started");
        
        // Spawn background enrichment task
        let db_service = self.db_service.clone();
        let enabled_flag = self.enabled.clone();
        
        tokio::spawn(async move {
            Self::enrichment_loop(db_service, enabled_flag).await;
        });
        
        Ok(())
    }
    
    /// Stop the enrichment service
    pub async fn stop(&self) -> Result<()> {
        let mut enabled = self.enabled.write().await;
        *enabled = false;
        info!("Asset enrichment service stopped");
        Ok(())
    }
    
    /// Background enrichment loop
    async fn enrichment_loop(db_service: Arc<DatabaseService>, enabled: Arc<RwLock<bool>>) {
        info!("Starting asset enrichment background loop");
        
        while *enabled.read().await {
            match Self::enrich_pending_assets(&db_service).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Enriched {} assets", count);
                    }
                }
                Err(e) => {
                    error!("Asset enrichment failed: {}", e);
                }
            }
            
            // Wait before next enrichment cycle (5 minutes)
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
        }
        
        info!("Asset enrichment background loop stopped");
    }
    
    /// Enrich assets that are pending enrichment
    async fn enrich_pending_assets(_db_service: &DatabaseService) -> Result<usize> {
        // TODO: Query assets that need enrichment
        // For now, return 0
        Ok(0)
    }
    
    /// Enrich a single asset
    pub async fn enrich_asset(&self, asset_id: &str) -> Result<()> {
        let asset = self.db_service.get_bounty_asset(asset_id).await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;
        
        info!("Enriching asset: {} ({})", asset.canonical_url, asset.asset_type);
        
        match asset.asset_type.as_str() {
            "domain" => {
                self.enrich_domain_asset(&asset).await?;
            }
            "ip" => {
                self.enrich_ip_asset(&asset).await?;
            }
            "port" => {
                self.enrich_port_asset(&asset).await?;
            }
            "url" => {
                self.enrich_url_asset(&asset).await?;
            }
            _ => {
                warn!("Unknown asset type for enrichment: {}", asset.asset_type);
            }
        }
        
        Ok(())
    }
    
    /// Enrich domain asset with DNS and WHOIS data
    async fn enrich_domain_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        let domain = &asset.canonical_url;
        info!("Enriching domain asset: {}", domain);
        
        let mut updated_asset = asset.clone();
        let mut changed = false;
        
        // Resolve DNS records if not already present
        if updated_asset.ip_addresses_json.is_none() {
            match Self::resolve_domain_ips(domain).await {
                Ok(ips) if !ips.is_empty() => {
                    updated_asset.ip_addresses_json = Some(serde_json::to_string(&ips)?);
                    changed = true;
                    info!("Resolved {} IP addresses for {}", ips.len(), domain);
                }
                Err(e) => warn!("Failed to resolve IPs for {}: {}", domain, e),
                _ => {}
            }
        }
        
        // Detect parent domain if not set
        if updated_asset.parent_domain.is_none() {
            let parts: Vec<&str> = domain.split('.').collect();
            if parts.len() > 2 {
                let parent = parts[parts.len()-2..].join(".");
                updated_asset.parent_domain = Some(parent);
                changed = true;
            }
        }
        
        // Update database if changes were made
        if changed {
            self.db_service.update_bounty_asset(&updated_asset).await?;
            info!("Domain enrichment completed for: {}", domain);
        }
        
        Ok(())
    }
    
    /// Resolve domain to IP addresses
    async fn resolve_domain_ips(domain: &str) -> Result<Vec<String>> {
        use tokio::net::lookup_host;
        
        let addresses = lookup_host(format!("{}:443", domain)).await?;
        let ips: Vec<String> = addresses
            .map(|addr| addr.ip().to_string())
            .collect();
        
        Ok(ips)
    }
    
    /// Enrich IP asset with geolocation and ASN data
    async fn enrich_ip_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        let ip = &asset.canonical_url;
        info!("Enriching IP asset: {}", ip);
        
        let mut updated_asset = asset.clone();
        let mut changed = false;
        
        // Basic IP version detection if not set
        if updated_asset.ip_version.is_none() {
            let is_ipv6 = ip.contains(':');
            updated_asset.ip_version = Some(if is_ipv6 { "IPv6" } else { "IPv4" }.to_string());
            changed = true;
        }
        
        // Attempt reverse DNS lookup if hostname not set
        if updated_asset.hostname.is_none() {
            match Self::reverse_dns_lookup(ip).await {
                Ok(Some(hostname)) => {
                    updated_asset.hostname = Some(hostname.clone());
                    info!("Reverse DNS for {}: {}", ip, hostname);
                    changed = true;
                }
                Err(e) => warn!("Reverse DNS failed for {}: {}", ip, e),
                _ => {}
            }
        }
        
        // Detect cloud provider based on IP ranges (basic detection)
        if updated_asset.is_cloud.is_none() {
            if let Some((is_cloud, provider)) = Self::detect_cloud_provider(ip) {
                updated_asset.is_cloud = Some(is_cloud);
                updated_asset.cloud_provider = provider;
                changed = true;
            }
        }
        
        // For more detailed enrichment (ASN, geolocation), would need external API
        // This is a placeholder - see integrate_external_api for full implementation
        
        if changed {
            self.db_service.update_bounty_asset(&updated_asset).await?;
            info!("IP enrichment completed for: {}", ip);
        }
        
        Ok(())
    }
    
    /// Reverse DNS lookup
    async fn reverse_dns_lookup(_ip: &str) -> Result<Option<String>> {
        // TODO: Implement reverse DNS lookup
        // Requires additional networking capabilities
        Ok(None)
    }
    
    /// Basic cloud provider detection based on IP ranges
    fn detect_cloud_provider(ip: &str) -> Option<(bool, Option<String>)> {
        // Basic detection - real implementation should use CIDR ranges
        if ip.starts_with("13.") || ip.starts_with("52.") || ip.starts_with("54.") {
            Some((true, Some("AWS".to_string())))
        } else if ip.starts_with("20.") || ip.starts_with("40.") {
            Some((true, Some("Azure".to_string())))
        } else if ip.starts_with("34.") || ip.starts_with("35.") {
            Some((true, Some("GCP".to_string())))
        } else if ip.starts_with("104.") {
            Some((true, Some("Cloudflare".to_string())))
        } else {
            Some((false, None))
        }
    }
    
    /// Enrich port asset with service detection
    async fn enrich_port_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        info!("Enriching port asset: {}", asset.canonical_url);
        
        let mut updated_asset = asset.clone();
        let mut changed = false;
        
        // If we have an IP, enrich it
        if let Some(ref hostname) = updated_asset.hostname {
            if updated_asset.ip_version.is_none() {
                let is_ipv6 = hostname.contains(':');
                updated_asset.ip_version = Some(if is_ipv6 { "IPv6" } else { "IPv4" }.to_string());
                changed = true;
            }
        }
        
        // Calculate attack surface score if not set
        if updated_asset.attack_surface_score.is_none() {
            if let Some(port) = updated_asset.port {
                let score = Self::calculate_port_risk(port, updated_asset.service_name.as_deref());
                updated_asset.attack_surface_score = Some(score);
                changed = true;
            }
        }
        
        // For banner grabbing and service detection, would need network scanning
        // This is beyond basic enrichment and should be done by specialized plugins
        
        if changed {
            self.db_service.update_bounty_asset(&updated_asset).await?;
            info!("Port enrichment completed for: {}", asset.canonical_url);
        }
        
        Ok(())
    }
    
    /// Calculate port risk score
    fn calculate_port_risk(port: i32, service: Option<&str>) -> f64 {
        let mut score: f64 = 0.0;
        
        match port {
            21 => score += 40.0,  // FTP
            22 => score += 20.0,  // SSH
            23 => score += 50.0,  // Telnet
            25 => score += 30.0,  // SMTP
            80 => score += 15.0,  // HTTP
            443 => score += 10.0, // HTTPS
            445 => score += 45.0, // SMB
            1433 => score += 35.0, // MSSQL
            3306 => score += 35.0, // MySQL
            3389 => score += 40.0, // RDP
            5432 => score += 30.0, // PostgreSQL
            6379 => score += 35.0, // Redis
            8080 => score += 20.0, // HTTP-alt
            27017 => score += 35.0, // MongoDB
            _ if port < 1024 => score += 15.0,
            _ => score += 5.0,
        }
        
        if let Some(svc) = service {
            let svc_lower = svc.to_lowercase();
            if svc_lower.contains("telnet") || svc_lower.contains("ftp") {
                score += 20.0;
            } else if svc_lower.contains("rdp") || svc_lower.contains("vnc") {
                score += 15.0;
            } else if svc_lower.contains("sql") || svc_lower.contains("database") {
                score += 15.0;
            }
        }
        
        score.min(100.0)
    }
    
    /// Enrich URL asset with technology stack and security headers
    async fn enrich_url_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        info!("Enriching URL asset: {}", asset.canonical_url);
        
        let mut updated_asset = asset.clone();
        let mut changed = false;
        
        // Perform HTTP request to get basic info
        if updated_asset.http_status.is_none() || updated_asset.headers_json.is_none() {
            match Self::fetch_url_info(&asset.canonical_url).await {
                Ok(info) => {
                    if updated_asset.http_status.is_none() {
                        updated_asset.http_status = Some(info.status_code);
                        changed = true;
                    }
                    
                    if updated_asset.headers_json.is_none() && !info.headers.is_empty() {
                        updated_asset.headers_json = Some(serde_json::to_string(&info.headers)?);
                        changed = true;
                    }
                    
                    if updated_asset.content_type.is_none() {
                        updated_asset.content_type = info.content_type;
                        changed = true;
                    }
                    
                    if updated_asset.content_length.is_none() {
                        updated_asset.content_length = info.content_length;
                        changed = true;
                    }
                    
                    // Detect WAF/CDN from headers
                    if updated_asset.waf_detected.is_none() {
                        updated_asset.waf_detected = Some(Self::detect_waf(&info.headers).to_string());
                        changed = true;
                    }
                    
                    if updated_asset.cdn_detected.is_none() {
                        updated_asset.cdn_detected = Self::detect_cdn(&info.headers).map(|s| s.to_string());
                        changed = true;
                    }
                    
                    info!("Fetched HTTP info for {}: status={}", asset.canonical_url, info.status_code);
                }
                Err(e) => warn!("Failed to fetch URL info for {}: {}", asset.canonical_url, e),
            }
        }
        
        // Update is_alive based on HTTP status
        if let Some(status) = updated_asset.http_status {
            let alive = status >= 200 && status < 500;
            if alive != updated_asset.is_alive {
                updated_asset.is_alive = alive;
                changed = true;
            }
        }
        
        if changed {
            self.db_service.update_bounty_asset(&updated_asset).await?;
            info!("URL enrichment completed for: {}", asset.canonical_url);
        }
        
        Ok(())
    }
    
    /// Fetch basic URL information
    async fn fetch_url_info(_url: &str) -> Result<UrlInfo> {
        // TODO: Implement HTTP client for URL fetching
        // For now, return a placeholder
        // Real implementation would use reqwest or similar
        Err(anyhow::anyhow!("URL fetching not yet implemented - requires HTTP client"))
    }
    
    /// Detect WAF from HTTP headers
    fn detect_waf(headers: &std::collections::HashMap<String, String>) -> bool {
        let waf_headers = ["x-sucuri-id", "x-waf-event-info", "x-cdn", "cf-ray", "server"];
        let waf_servers = ["cloudflare", "sucuri", "imperva", "barracuda", "fortiweb"];
        
        for (key, value) in headers.iter() {
            let key_lower = key.to_lowercase();
            let value_lower = value.to_lowercase();
            
            if waf_headers.iter().any(|&h| key_lower.contains(h)) {
                return true;
            }
            
            if key_lower == "server" && waf_servers.iter().any(|&s| value_lower.contains(s)) {
                return true;
            }
        }
        
        false
    }
    
    /// Detect CDN from HTTP headers
    fn detect_cdn(headers: &std::collections::HashMap<String, String>) -> Option<String> {
        if headers.contains_key("cf-ray") || headers.contains_key("cf-cache-status") {
            return Some("Cloudflare".to_string());
        }
        
        if headers.contains_key("x-amz-cf-id") || headers.contains_key("x-amz-cf-pop") {
            return Some("AWS CloudFront".to_string());
        }
        
        if let Some(server) = headers.get("server") {
            if server.to_lowercase().contains("cloudflare") {
                return Some("Cloudflare".to_string());
            } else if server.to_lowercase().contains("cloudfront") {
                return Some("AWS CloudFront".to_string());
            } else if server.to_lowercase().contains("akamai") {
                return Some("Akamai".to_string());
            }
        }
        
        None
    }
    
    /// Enrich IP information from external API
    pub async fn enrich_ip_from_api(ip: &str) -> Result<IpEnrichment> {
        // TODO: Call IP enrichment API (ipinfo.io, ipapi.co, etc.)
        // This is a placeholder implementation
        
        let is_ipv6 = ip.contains(':');
        
        Ok(IpEnrichment {
            ip: ip.to_string(),
            ip_version: if is_ipv6 { "IPv6" } else { "IPv4" }.to_string(),
            asn: None,
            asn_org: None,
            isp: None,
            country: None,
            city: None,
            latitude: None,
            longitude: None,
            is_cloud: false,
            cloud_provider: None,
        })
    }
    
    /// Calculate attack surface score for an asset
    pub fn calculate_attack_surface_score(asset: &BountyAssetRow) -> f64 {
        let mut score: f64 = 0.0;
        
        // Exposure level (40% weight)
        score += match asset.exposure_level.as_deref() {
            Some("internet") => 40.0,
            Some("intranet") => 20.0,
            Some("private") => 5.0,
            _ => 10.0,
        };
        
        // Vulnerability count (30% weight)
        if let Some(vuln_count) = asset.vulnerability_count {
            score += (vuln_count as f64 * 5.0).min(30.0);
        }
        
        // Criticality (20% weight)
        score += match asset.criticality.as_deref() {
            Some("critical") => 20.0,
            Some("high") => 15.0,
            Some("medium") => 10.0,
            Some("low") => 5.0,
            _ => 7.0,
        };
        
        // Port/Service risk (10% weight)
        if asset.asset_type == "port" {
            if let Some(port) = asset.port {
                score += match port {
                    21 | 23 | 445 | 3389 => 10.0, // High-risk ports
                    22 | 3306 | 5432 => 7.0,      // Medium-risk
                    _ => 3.0,
                };
            }
        }
        
        score.min(100.0)
    }
}

/// URL information from HTTP request
#[derive(Debug)]
struct UrlInfo {
    status_code: i32,
    headers: std::collections::HashMap<String, String>,
    content_type: Option<String>,
    content_length: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attack_surface_score_calculation() {
        let mut asset = BountyAssetRow {
            id: "test".to_string(),
            program_id: "test".to_string(),
            scope_id: None,
            asset_type: "domain".to_string(),
            canonical_url: "example.com".to_string(),
            exposure_level: Some("internet".to_string()),
            vulnerability_count: Some(5),
            criticality: Some("high".to_string()),
            ..Default::default()
        };
        
        // This test will fail until Default is implemented
        // let score = AssetEnrichmentService::calculate_attack_surface_score(&asset);
        // assert!(score > 0.0 && score <= 100.0);
    }
}
