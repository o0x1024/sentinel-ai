//! Workflow Artifact Protocol (P0-1)
//! 
//! Unified output contract for workflow steps, enabling automatic data sinking.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Standard artifact types produced by workflow steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// New vulnerability finding
    Finding,
    /// Evidence supporting a finding
    Evidence,
    /// Discovered or updated asset
    Asset,
    /// Discovered subdomains
    Subdomains,
    /// Live hosts / HTTP probe results
    LiveHosts,
    /// Technology fingerprint
    Technologies,
    /// Discovered endpoints / URLs
    Endpoints,
    /// Secrets / credentials
    Secrets,
    /// Directories / files
    Directories,
    /// Raw data (pass-through to downstream)
    RawData,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::Finding => "finding",
            ArtifactType::Evidence => "evidence",
            ArtifactType::Asset => "asset",
            ArtifactType::Subdomains => "subdomains",
            ArtifactType::LiveHosts => "live_hosts",
            ArtifactType::Technologies => "technologies",
            ArtifactType::Endpoints => "endpoints",
            ArtifactType::Secrets => "secrets",
            ArtifactType::Directories => "directories",
            ArtifactType::RawData => "raw_data",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "finding" | "findings" | "vulnerability" => Some(ArtifactType::Finding),
            "evidence" => Some(ArtifactType::Evidence),
            "asset" | "assets" => Some(ArtifactType::Asset),
            "subdomains" | "subdomain" => Some(ArtifactType::Subdomains),
            "live_hosts" | "livehosts" | "alive" | "hosts" => Some(ArtifactType::LiveHosts),
            "technologies" | "tech" | "techstack" => Some(ArtifactType::Technologies),
            "endpoints" | "endpoint" | "urls" => Some(ArtifactType::Endpoints),
            "secrets" | "secret" | "credentials" => Some(ArtifactType::Secrets),
            "directories" | "directory" | "files" => Some(ArtifactType::Directories),
            "raw" | "raw_data" | "data" => Some(ArtifactType::RawData),
            _ => None,
        }
    }
}

/// Unified workflow step output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowArtifact {
    /// Unique artifact ID
    pub id: String,
    /// Parent step ID
    pub step_id: String,
    /// Workflow execution ID
    pub execution_id: String,
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Primary data payload
    pub data: serde_json::Value,
    /// Metadata (source, confidence, etc.)
    pub metadata: ArtifactMetadata,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Artifact metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactMetadata {
    /// Source plugin/tool name
    pub source: Option<String>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: Option<f64>,
    /// Processing duration in ms
    pub duration_ms: Option<u64>,
    /// Item count (for list-type artifacts)
    pub count: Option<usize>,
    /// Custom tags
    pub tags: Vec<String>,
    /// Additional custom fields
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Finding artifact payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingArtifact {
    pub title: String,
    pub description: String,
    pub finding_type: String,
    pub severity: Option<String>,
    pub confidence: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub cwe_id: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub raw_output: Option<serde_json::Value>,
}

/// Evidence artifact payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceArtifact {
    pub title: String,
    pub evidence_type: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub http_request: Option<serde_json::Value>,
    pub http_response: Option<serde_json::Value>,
    pub screenshot_path: Option<String>,
    pub diff: Option<String>,
}

/// Asset artifact payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetArtifact {
    pub url: String,
    pub hostname: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<u16>,
    pub tech_stack: Vec<String>,
    pub status_code: Option<u16>,
    pub title: Option<String>,
    pub fingerprint: Option<String>,
    pub labels: Vec<String>,
}

/// Subdomain list artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainsArtifact {
    pub domain: String,
    pub subdomains: Vec<SubdomainEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainEntry {
    pub subdomain: String,
    pub source: Option<String>,
}

/// Live hosts artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveHostsArtifact {
    pub hosts: Vec<LiveHostEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveHostEntry {
    pub url: String,
    pub status_code: u16,
    pub title: Option<String>,
    pub content_length: Option<u64>,
    pub technologies: Vec<String>,
    pub headers: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Technology fingerprint artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologiesArtifact {
    pub url: String,
    pub technologies: Vec<TechEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechEntry {
    pub name: String,
    pub version: Option<String>,
    pub category: Option<String>,
    pub confidence: Option<f64>,
}

/// Endpoints artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointsArtifact {
    pub base_url: String,
    pub endpoints: Vec<EndpointEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointEntry {
    pub path: String,
    pub method: Option<String>,
    pub source: Option<String>,
    pub params: Vec<String>,
}

/// Secrets artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsArtifact {
    pub secrets: Vec<SecretEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    pub secret_type: String,
    pub value: String,
    pub source_url: Option<String>,
    pub line: Option<u32>,
    pub context: Option<String>,
}

/// Directories artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoriesArtifact {
    pub base_url: String,
    pub directories: Vec<DirectoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub path: String,
    pub status_code: u16,
    pub content_length: Option<u64>,
    pub redirect_url: Option<String>,
}

/// Artifact extractor - parses raw plugin output into typed artifacts
pub struct ArtifactExtractor;

impl ArtifactExtractor {
    /// Detect artifact type from raw output
    pub fn detect_type(data: &serde_json::Value) -> ArtifactType {
        // Check for explicit type field
        if let Some(t) = data.get("artifact_type").and_then(|v| v.as_str()) {
            if let Some(at) = ArtifactType::from_str(t) {
                return at;
            }
        }
        if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
            if let Some(at) = ArtifactType::from_str(t) {
                return at;
            }
        }
        
        // Heuristic detection
        if data.get("vulnerability").is_some() || data.get("finding").is_some() || data.get("findings").is_some() {
            return ArtifactType::Finding;
        }
        if data.get("subdomains").is_some() {
            return ArtifactType::Subdomains;
        }
        if data.get("hosts").is_some() || data.get("liveHosts").is_some() || data.get("results").is_some() {
            // Check if results contain status_code
            if let Some(arr) = data.get("results").and_then(|v| v.as_array()) {
                if arr.iter().any(|item| item.get("statusCode").is_some() || item.get("status_code").is_some()) {
                    return ArtifactType::LiveHosts;
                }
            }
            if data.get("hosts").is_some() || data.get("liveHosts").is_some() {
                return ArtifactType::LiveHosts;
            }
        }
        if data.get("technologies").is_some() || data.get("techStack").is_some() {
            return ArtifactType::Technologies;
        }
        if data.get("endpoints").is_some() {
            return ArtifactType::Endpoints;
        }
        if data.get("secrets").is_some() {
            return ArtifactType::Secrets;
        }
        if data.get("directories").is_some() || data.get("paths").is_some() {
            return ArtifactType::Directories;
        }
        
        ArtifactType::RawData
    }
    
    /// Extract finding from raw output
    pub fn extract_finding(data: &serde_json::Value) -> Option<FindingArtifact> {
        // Check nested structures
        let finding_data = data.get("finding")
            .or_else(|| data.get("vulnerability"))
            .unwrap_or(data);
        
        // Title is required
        let title = finding_data.get("title")
            .or_else(|| finding_data.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())?;
        
        Some(FindingArtifact {
            title,
            description: finding_data.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            finding_type: finding_data.get("finding_type")
                .or_else(|| finding_data.get("type"))
                .or_else(|| finding_data.get("vulnerabilityType"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            severity: finding_data.get("severity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            confidence: finding_data.get("confidence")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_url: finding_data.get("affected_url")
                .or_else(|| finding_data.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_parameter: finding_data.get("affected_parameter")
                .or_else(|| finding_data.get("parameter"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cwe_id: finding_data.get("cwe_id")
                .or_else(|| finding_data.get("cwe"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            impact: finding_data.get("impact")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            remediation: finding_data.get("remediation")
                .or_else(|| finding_data.get("recommendation"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            reproduction_steps: finding_data.get("reproduction_steps")
                .or_else(|| finding_data.get("steps"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
            raw_output: Some(data.clone()),
        })
    }
    
    /// Extract subdomains from raw output
    pub fn extract_subdomains(data: &serde_json::Value) -> Option<SubdomainsArtifact> {
        let subdomains_arr = data.get("subdomains").and_then(|v| v.as_array())?;
        
        let domain = data.get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let subdomains: Vec<SubdomainEntry> = subdomains_arr.iter().filter_map(|item| {
            if let Some(s) = item.as_str() {
                Some(SubdomainEntry { subdomain: s.to_string(), source: None })
            } else if let Some(obj) = item.as_object() {
                Some(SubdomainEntry {
                    subdomain: obj.get("subdomain").or_else(|| obj.get("host")).and_then(|v| v.as_str())?.to_string(),
                    source: obj.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                })
            } else {
                None
            }
        }).collect();
        
        if subdomains.is_empty() { return None; }
        
        Some(SubdomainsArtifact { domain, subdomains })
    }
    
    /// Extract live hosts from raw output
    pub fn extract_live_hosts(data: &serde_json::Value) -> Option<LiveHostsArtifact> {
        let hosts_arr = data.get("hosts")
            .or_else(|| data.get("liveHosts"))
            .or_else(|| data.get("results"))
            .and_then(|v| v.as_array())?;
        
        let hosts: Vec<LiveHostEntry> = hosts_arr.iter().filter_map(|item| {
            let url = item.get("url").and_then(|v| v.as_str())?.to_string();
            let status_code = item.get("statusCode")
                .or_else(|| item.get("status_code"))
                .or_else(|| item.get("status"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            
            Some(LiveHostEntry {
                url,
                status_code,
                title: item.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                content_length: item.get("contentLength")
                    .or_else(|| item.get("content_length"))
                    .and_then(|v| v.as_u64()),
                technologies: item.get("technologies")
                    .or_else(|| item.get("tech"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
                headers: item.get("headers")
                    .and_then(|v| v.as_object())
                    .cloned(),
            })
        }).collect();
        
        if hosts.is_empty() { return None; }
        
        Some(LiveHostsArtifact { hosts })
    }
    
    /// Extract technologies from raw output
    pub fn extract_technologies(data: &serde_json::Value) -> Option<TechnologiesArtifact> {
        let tech_arr = data.get("technologies")
            .or_else(|| data.get("techStack"))
            .and_then(|v| v.as_array())?;
        
        let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        
        let technologies: Vec<TechEntry> = tech_arr.iter().filter_map(|item| {
            if let Some(name) = item.as_str() {
                Some(TechEntry { name: name.to_string(), version: None, category: None, confidence: None })
            } else if let Some(obj) = item.as_object() {
                let name = obj.get("name").and_then(|v| v.as_str())?.to_string();
                Some(TechEntry {
                    name,
                    version: obj.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    category: obj.get("category").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    confidence: obj.get("confidence").and_then(|v| v.as_f64()),
                })
            } else {
                None
            }
        }).collect();
        
        Some(TechnologiesArtifact { url, technologies })
    }
    
    /// Extract endpoints from raw output
    pub fn extract_endpoints(data: &serde_json::Value) -> Option<EndpointsArtifact> {
        let endpoints_arr = data.get("endpoints")
            .and_then(|v| v.as_array())?;
        
        let base_url = data.get("base_url")
            .or_else(|| data.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let endpoints: Vec<EndpointEntry> = endpoints_arr.iter().filter_map(|item| {
            if let Some(path) = item.as_str() {
                Some(EndpointEntry { path: path.to_string(), method: None, source: None, params: vec![] })
            } else if let Some(obj) = item.as_object() {
                Some(EndpointEntry {
                    path: obj.get("path").or_else(|| obj.get("url")).and_then(|v| v.as_str())?.to_string(),
                    method: obj.get("method").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    source: obj.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    params: obj.get("params")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default(),
                })
            } else {
                None
            }
        }).collect();
        
        if endpoints.is_empty() { return None; }
        
        Some(EndpointsArtifact { base_url, endpoints })
    }
    
    /// Extract secrets from raw output
    pub fn extract_secrets(data: &serde_json::Value) -> Option<SecretsArtifact> {
        let secrets_arr = data.get("secrets").and_then(|v| v.as_array())?;
        
        let secrets: Vec<SecretEntry> = secrets_arr.iter().filter_map(|item| {
            let obj = item.as_object()?;
            Some(SecretEntry {
                secret_type: obj.get("type").or_else(|| obj.get("secret_type")).and_then(|v| v.as_str())?.to_string(),
                value: obj.get("value").or_else(|| obj.get("secret")).and_then(|v| v.as_str())?.to_string(),
                source_url: obj.get("source_url").or_else(|| obj.get("url")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                line: obj.get("line").and_then(|v| v.as_u64()).map(|n| n as u32),
                context: obj.get("context").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
        }).collect();
        
        if secrets.is_empty() { return None; }
        
        Some(SecretsArtifact { secrets })
    }
    
    /// Extract directories from raw output
    pub fn extract_directories(data: &serde_json::Value) -> Option<DirectoriesArtifact> {
        let dirs_arr = data.get("directories")
            .or_else(|| data.get("paths"))
            .or_else(|| data.get("results"))
            .and_then(|v| v.as_array())?;
        
        let base_url = data.get("base_url")
            .or_else(|| data.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let directories: Vec<DirectoryEntry> = dirs_arr.iter().filter_map(|item| {
            if let Some(path) = item.as_str() {
                Some(DirectoryEntry { path: path.to_string(), status_code: 200, content_length: None, redirect_url: None })
            } else if let Some(obj) = item.as_object() {
                Some(DirectoryEntry {
                    path: obj.get("path").or_else(|| obj.get("url")).and_then(|v| v.as_str())?.to_string(),
                    status_code: obj.get("status_code")
                        .or_else(|| obj.get("statusCode"))
                        .or_else(|| obj.get("status"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(200) as u16,
                    content_length: obj.get("content_length")
                        .or_else(|| obj.get("contentLength"))
                        .and_then(|v| v.as_u64()),
                    redirect_url: obj.get("redirect_url")
                        .or_else(|| obj.get("location"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            } else {
                None
            }
        }).collect();
        
        if directories.is_empty() { return None; }
        
        Some(DirectoriesArtifact { base_url, directories })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_detect_type() {
        assert_eq!(ArtifactExtractor::detect_type(&json!({"subdomains": ["a.com"]})), ArtifactType::Subdomains);
        assert_eq!(ArtifactExtractor::detect_type(&json!({"findings": []})), ArtifactType::Finding);
        assert_eq!(ArtifactExtractor::detect_type(&json!({"technologies": []})), ArtifactType::Technologies);
    }
    
    #[test]
    fn test_extract_subdomains() {
        let data = json!({
            "domain": "example.com",
            "subdomains": ["a.example.com", "b.example.com"]
        });
        let result = ArtifactExtractor::extract_subdomains(&data).unwrap();
        assert_eq!(result.domain, "example.com");
        assert_eq!(result.subdomains.len(), 2);
    }
}
