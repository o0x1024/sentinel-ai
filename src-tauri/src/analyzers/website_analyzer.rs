//! Website Analyzer - Extract API endpoints and analyze website structure from proxy logs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use url::Url;

use sentinel_passive::PassiveDatabaseService;
use super::param_extractor::{ParamExtractor, Parameter};
use super::tech_stack_detector::{TechStack, TechStackDetector};

/// API endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// Original path
    pub path: String,
    /// Normalized pattern (e.g., /user/:id)
    pub pattern: String,
    /// HTTP method
    pub method: String,
    /// Request content type
    pub content_type: Option<String>,
    /// Response content type
    pub response_content_type: Option<String>,
    /// Query parameters found
    pub query_params: Vec<Parameter>,
    /// Body parameters found (for POST/PUT)
    pub body_params: Vec<Parameter>,
    /// Number of times this endpoint was accessed
    pub hit_count: usize,
    /// Example URLs
    pub examples: Vec<String>,
}

/// Complete website analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteAnalysis {
    /// Target domain
    pub domain: String,
    /// Total requests analyzed
    pub total_requests: usize,
    /// Unique API endpoints
    pub endpoints: Vec<ApiEndpoint>,
    /// Technology stack detected
    pub tech_stack: TechStack,
    /// All unique parameters found
    pub all_parameters: Vec<Parameter>,
    /// Static resources count
    pub static_resources_count: usize,
    /// API endpoints count
    pub api_endpoints_count: usize,
}

/// Website Analyzer
pub struct WebsiteAnalyzer {
    db_service: Arc<PassiveDatabaseService>,
    param_extractor: ParamExtractor,
    tech_detector: TechStackDetector,
}

impl WebsiteAnalyzer {
    pub fn new(db_service: Arc<PassiveDatabaseService>) -> Self {
        Self {
            db_service,
            param_extractor: ParamExtractor::new(),
            tech_detector: TechStackDetector::new(),
        }
    }

    /// Analyze a website by domain
    pub async fn analyze(&self, domain: &str) -> Result<WebsiteAnalysis> {
        log::info!("Starting website analysis for domain: {}", domain);

        // 1. Fetch all requests for this domain from database
        let requests = self.fetch_requests_by_domain(domain).await
            .context("Failed to fetch requests from database")?;

        if requests.is_empty() {
            log::warn!("No requests found for domain: {}", domain);
            return Ok(WebsiteAnalysis {
                domain: domain.to_string(),
                total_requests: 0,
                endpoints: vec![],
                tech_stack: TechStack::default(),
                all_parameters: vec![],
                static_resources_count: 0,
                api_endpoints_count: 0,
            });
        }

        log::info!("Found {} requests for domain {}", requests.len(), domain);

        // 2. Extract endpoints
        let endpoints = self.extract_endpoints(&requests)
            .context("Failed to extract endpoints")?;

        // 3. Detect technology stack
        let tech_stack = self.tech_detector.detect(&requests);

        // 4. Collect all unique parameters
        let all_parameters = self.collect_all_parameters(&endpoints);

        // 5. Count static vs API resources
        let static_resources_count = self.count_static_resources(&requests);
        let api_endpoints_count = endpoints.len();

        log::info!(
            "Analysis complete: {} endpoints, {} params, tech: {:?}",
            api_endpoints_count,
            all_parameters.len(),
            tech_stack
        );

        Ok(WebsiteAnalysis {
            domain: domain.to_string(),
            total_requests: requests.len(),
            endpoints,
            tech_stack,
            all_parameters,
            static_resources_count,
            api_endpoints_count,
        })
    }

    /// Fetch requests from database by domain
    async fn fetch_requests_by_domain(&self, domain: &str) -> Result<Vec<ProxyRequest>> {
        // Query database for requests matching domain
        let records = self.db_service.list_proxy_requests_by_host(domain, 1000).await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

        // Convert to internal format
        Ok(records.into_iter().map(|r| ProxyRequest {
            url: r.url,
            method: r.method,
            status_code: r.status_code,
            request_headers: r.request_headers,
            request_body: r.request_body,
            response_headers: r.response_headers,
            response_body: r.response_body,
        }).collect())
    }

    /// Extract and normalize API endpoints
    fn extract_endpoints(&self, requests: &[ProxyRequest]) -> Result<Vec<ApiEndpoint>> {
        let mut endpoint_map: HashMap<String, EndpointBuilder> = HashMap::new();

        for req in requests {
            // Parse URL
            let url = Url::parse(&req.url).context("Failed to parse URL")?;
            let path = url.path();

            // Skip static resources
            if self.is_static_resource(path) {
                continue;
            }

            // Normalize path pattern (e.g., /user/123 -> /user/:id)
            let pattern = self.normalize_path(path);
            let key = format!("{}:{}", req.method, pattern);

            // Get or create endpoint builder
            let builder = endpoint_map.entry(key.clone()).or_insert_with(|| EndpointBuilder {
                path: path.to_string(),
                pattern: pattern.clone(),
                method: req.method.clone(),
                examples: Vec::new(),
                query_params_set: HashSet::new(),
                body_params_set: HashSet::new(),
                content_types: HashSet::new(),
                response_content_types: HashSet::new(),
                hit_count: 0,
            });

            // Update builder
            builder.hit_count += 1;
            if builder.examples.len() < 3 {
                builder.examples.push(req.url.clone());
            }

            // Extract parameters
            let query_params = self.param_extractor.extract_query_params(url.query().unwrap_or(""));
            for param in query_params {
                builder.query_params_set.insert(param);
            }

            // Extract body parameters (for POST/PUT/PATCH)
            if matches!(req.method.as_str(), "POST" | "PUT" | "PATCH") {
                if let Some(ref body) = req.request_body {
                    let content_type = self.extract_content_type(&req.request_headers);
                    let body_params = self.param_extractor.extract_body_params(body, &content_type);
                    for param in body_params {
                        builder.body_params_set.insert(param);
                    }
                    if let Some(ct) = content_type {
                        builder.content_types.insert(ct);
                    }
                }
            }

            // Extract response content type
            if let Some(ref resp_headers) = req.response_headers {
                if let Some(ct) = self.extract_content_type(&Some(resp_headers.clone())) {
                    builder.response_content_types.insert(ct);
                }
            }
        }

        // Convert builders to endpoints
        let mut endpoints: Vec<ApiEndpoint> = endpoint_map.into_iter()
            .map(|(_, builder)| builder.build())
            .collect();

        // Sort by hit count (descending)
        endpoints.sort_by(|a, b| b.hit_count.cmp(&a.hit_count));

        Ok(endpoints)
    }

    /// Normalize path to pattern (e.g., /user/123 -> /user/:id)
    fn normalize_path(&self, path: &str) -> String {
        let segments: Vec<&str> = path.split('/').collect();
        let normalized: Vec<String> = segments.iter().map(|seg| {
            if seg.is_empty() {
                return String::new();
            }
            
            // Check if segment is numeric ID
            if seg.parse::<i64>().is_ok() {
                return ":id".to_string();
            }
            
            // Check if segment is UUID
            if self.is_uuid(seg) {
                return ":uuid".to_string();
            }
            
            // Check if segment is hash
            if seg.len() >= 16 && seg.chars().all(|c| c.is_ascii_hexdigit()) {
                return ":hash".to_string();
            }
            
            seg.to_string()
        }).collect();

        normalized.join("/")
    }

    /// Check if string is UUID format
    fn is_uuid(&self, s: &str) -> bool {
        if s.len() != 36 {
            return false;
        }
        let parts: Vec<&str> = s.split('-').collect();
        parts.len() == 5 && 
            parts[0].len() == 8 && 
            parts[1].len() == 4 &&
            parts[2].len() == 4 &&
            parts[3].len() == 4 &&
            parts[4].len() == 12
    }

    /// Check if path is a static resource
    fn is_static_resource(&self, path: &str) -> bool {
        let static_extensions = [
            ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico",
            ".woff", ".woff2", ".ttf", ".eot", ".map", ".webp", ".mp4", ".mp3",
            ".pdf", ".zip", ".rar", ".tar", ".gz", ".xml", ".json",
        ];
        
        static_extensions.iter().any(|ext| path.to_lowercase().ends_with(ext))
    }

    /// Count static resources
    fn count_static_resources(&self, requests: &[ProxyRequest]) -> usize {
        requests.iter().filter(|req| {
            if let Ok(url) = Url::parse(&req.url) {
                self.is_static_resource(url.path())
            } else {
                false
            }
        }).count()
    }

    /// Extract content type from headers
    fn extract_content_type(&self, headers: &Option<String>) -> Option<String> {
        let headers_str = headers.as_ref()?;
        
        // Try to parse as JSON (HashMap<String, String>)
        if let Ok(headers_map) = serde_json::from_str::<HashMap<String, String>>(headers_str) {
            // Look for Content-Type header (case insensitive)
            for (key, value) in headers_map.iter() {
                if key.to_lowercase() == "content-type" {
                    // Extract main type (before semicolon)
                    return Some(value.split(';').next()?.trim().to_string());
                }
            }
        }
        
        None
    }

    /// Collect all unique parameters from endpoints
    fn collect_all_parameters(&self, endpoints: &[ApiEndpoint]) -> Vec<Parameter> {
        let mut param_map: HashMap<String, Parameter> = HashMap::new();
        
        for endpoint in endpoints {
            for param in &endpoint.query_params {
                param_map.entry(param.name.clone())
                    .or_insert_with(|| param.clone());
            }
            for param in &endpoint.body_params {
                param_map.entry(param.name.clone())
                    .or_insert_with(|| param.clone());
            }
        }
        
        param_map.into_values().collect()
    }
}

/// Internal request representation
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    pub status_code: i32,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
}

/// Endpoint builder (internal)
struct EndpointBuilder {
    path: String,
    pattern: String,
    method: String,
    examples: Vec<String>,
    query_params_set: HashSet<Parameter>,
    body_params_set: HashSet<Parameter>,
    content_types: HashSet<String>,
    response_content_types: HashSet<String>,
    hit_count: usize,
}

impl EndpointBuilder {
    fn build(self) -> ApiEndpoint {
        ApiEndpoint {
            path: self.path,
            pattern: self.pattern,
            method: self.method,
            content_type: self.content_types.into_iter().next(),
            response_content_type: self.response_content_types.into_iter().next(),
            query_params: self.query_params_set.into_iter().collect(),
            body_params: self.body_params_set.into_iter().collect(),
            hit_count: self.hit_count,
            examples: self.examples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        let analyzer = WebsiteAnalyzer::new(Arc::new(
            PassiveDatabaseService::new(":memory:").unwrap()
        ));

        assert_eq!(
            analyzer.normalize_path("/user/123/profile"),
            "/user/:id/profile"
        );
        assert_eq!(
            analyzer.normalize_path("/api/v1/resource/abc123def456"),
            "/api/v1/resource/:hash"
        );
    }

    #[test]
    fn test_is_static_resource() {
        let analyzer = WebsiteAnalyzer::new(Arc::new(
            PassiveDatabaseService::new(":memory:").unwrap()
        ));

        assert!(analyzer.is_static_resource("/assets/main.js"));
        assert!(analyzer.is_static_resource("/images/logo.png"));
        assert!(!analyzer.is_static_resource("/api/users"));
    }
}

