//! Workflow Artifact Protocol (P0-1)
//!
//! Unified output contract for workflow steps, enabling automatic data sinking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    /// Multi-object surface graph bundle
    SurfaceBundle,
    /// Typed surface graph objects
    SurfaceDomains,
    SurfaceIps,
    SurfaceHosts,
    SurfacePorts,
    SurfaceServices,
    SurfaceWebs,
    SurfaceCertificates,
    SurfaceFingerprints,
    SurfaceRelations,
    SurfaceChanges,
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
            ArtifactType::SurfaceBundle => "surface_bundle",
            ArtifactType::SurfaceDomains => "surface_domains",
            ArtifactType::SurfaceIps => "surface_ips",
            ArtifactType::SurfaceHosts => "surface_hosts",
            ArtifactType::SurfacePorts => "surface_ports",
            ArtifactType::SurfaceServices => "surface_services",
            ArtifactType::SurfaceWebs => "surface_webs",
            ArtifactType::SurfaceCertificates => "surface_certificates",
            ArtifactType::SurfaceFingerprints => "surface_fingerprints",
            ArtifactType::SurfaceRelations => "surface_relations",
            ArtifactType::SurfaceChanges => "surface_changes",
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
            "surface_bundle" | "surfacebundle" => Some(ArtifactType::SurfaceBundle),
            "surface_domains" | "domains" => Some(ArtifactType::SurfaceDomains),
            "surface_ips" | "ips" => Some(ArtifactType::SurfaceIps),
            "surface_hosts" => Some(ArtifactType::SurfaceHosts),
            "surface_ports" | "ports" => Some(ArtifactType::SurfacePorts),
            "surface_services" | "services" => Some(ArtifactType::SurfaceServices),
            "surface_webs" | "webs" | "web_assets" => Some(ArtifactType::SurfaceWebs),
            "surface_certificates" | "certificates" => Some(ArtifactType::SurfaceCertificates),
            "surface_fingerprints" | "fingerprints" => Some(ArtifactType::SurfaceFingerprints),
            "surface_relations" | "relations" => Some(ArtifactType::SurfaceRelations),
            "surface_changes" | "changes" => Some(ArtifactType::SurfaceChanges),
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceEvidencesArtifact {
    pub evidences: Vec<SurfaceEvidenceArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEvidenceArtifact {
    pub asset_type: String,
    pub asset_key: String,
    pub evidence_type: String,
    pub title: String,
    pub content_text: Option<String>,
    pub content_path: Option<String>,
    pub content_json: Option<Value>,
    pub probe_node: Option<String>,
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

/// Multi-object surface graph payload for network asset mapping workflows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceBundleArtifact {
    pub organizations: Vec<Value>,
    pub domains: Vec<Value>,
    pub ips: Vec<Value>,
    pub hosts: Vec<Value>,
    pub ports: Vec<Value>,
    pub services: Vec<Value>,
    pub webs: Vec<SurfaceWebArtifact>,
    pub certificates: Vec<Value>,
    pub fingerprints: Vec<SurfaceFingerprintArtifact>,
    pub relations: Vec<Value>,
    pub changes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceWebsArtifact {
    pub webs: Vec<SurfaceWebArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceFingerprintsArtifact {
    pub fingerprints: Vec<SurfaceFingerprintArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceWebArtifact {
    pub canonical_url: String,
    pub scheme: String,
    pub site_title: Option<String>,
    pub http_status_code: i32,
    pub server_header: Option<String>,
    pub response_headers: Map<String, Value>,
    pub page_fingerprint: Option<String>,
    pub favicon_hash: Option<String>,
    pub framework: Option<String>,
    pub cms: Option<String>,
    pub waf_flag: Option<bool>,
    pub cdn_flag: Option<bool>,
    pub login_flag: Option<bool>,
    pub api_flag: Option<bool>,
    pub openapi_url: Option<String>,
    pub business_type: Option<String>,
    pub language: Option<String>,
    pub filing_info: Option<String>,
    pub content_summary: String,
    pub last_accessed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFingerprintArtifact {
    pub asset_type: String,
    pub asset_key: String,
    pub fingerprint_type: String,
    pub fingerprint_key: Option<String>,
    pub fingerprint_value: String,
    pub rule_id: String,
    pub rule_word: String,
    pub rule_name: String,
    pub normalized_product: String,
    pub normalized_vendor: Option<String>,
    pub normalized_category: String,
    pub normalized_family: Option<String>,
    pub version: Option<String>,
    pub is_primary: Option<bool>,
    pub match_source_part: Option<String>,
    pub confidence: Option<f64>,
    pub evidence: Option<String>,
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
    fn required_string(obj: &Map<String, Value>, field: &str) -> Option<String> {
        obj.get(field)?.as_str().map(str::to_string)
    }

    fn required_i32(obj: &Map<String, Value>, field: &str) -> Option<i32> {
        obj.get(field)?.as_i64().map(|value| value as i32)
    }

    fn required_object(obj: &Map<String, Value>, field: &str) -> Option<Map<String, Value>> {
        obj.get(field)?.as_object().cloned()
    }

    fn parse_surface_web(item: &Value) -> Option<SurfaceWebArtifact> {
        let obj = item.as_object()?;

        Some(SurfaceWebArtifact {
            canonical_url: Self::required_string(obj, "canonical_url")?,
            scheme: Self::required_string(obj, "scheme")?,
            site_title: obj
                .get("site_title")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            http_status_code: Self::required_i32(obj, "http_status_code")?,
            server_header: obj
                .get("server_header")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            response_headers: Self::required_object(obj, "response_headers")?,
            page_fingerprint: obj
                .get("page_fingerprint")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            favicon_hash: obj
                .get("favicon_hash")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            framework: obj
                .get("framework")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            cms: obj
                .get("cms")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            waf_flag: obj.get("waf_flag").and_then(|value| value.as_bool()),
            cdn_flag: obj.get("cdn_flag").and_then(|value| value.as_bool()),
            login_flag: obj.get("login_flag").and_then(|value| value.as_bool()),
            api_flag: obj.get("api_flag").and_then(|value| value.as_bool()),
            openapi_url: obj
                .get("openapi_url")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            business_type: obj
                .get("business_type")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            language: obj
                .get("language")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            filing_info: obj
                .get("filing_info")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_summary: Self::required_string(obj, "content_summary")?,
            last_accessed_at: obj
                .get("last_accessed_at")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        })
    }

    fn parse_surface_fingerprint(item: &Value) -> Option<SurfaceFingerprintArtifact> {
        let obj = item.as_object()?;

        Some(SurfaceFingerprintArtifact {
            asset_type: Self::required_string(obj, "asset_type")?,
            asset_key: Self::required_string(obj, "asset_key")?,
            fingerprint_type: Self::required_string(obj, "fingerprint_type")?,
            fingerprint_key: obj
                .get("fingerprint_key")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            fingerprint_value: Self::required_string(obj, "fingerprint_value")?,
            rule_id: Self::required_string(obj, "rule_id")?,
            rule_word: Self::required_string(obj, "rule_word")?,
            rule_name: Self::required_string(obj, "rule_name")?,
            normalized_product: Self::required_string(obj, "normalized_product")?,
            normalized_vendor: obj
                .get("normalized_vendor")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            normalized_category: Self::required_string(obj, "normalized_category")?,
            normalized_family: obj
                .get("normalized_family")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            version: obj
                .get("version")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            is_primary: obj.get("is_primary").and_then(|value| value.as_bool()),
            match_source_part: obj
                .get("match_source_part")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            confidence: obj.get("confidence").and_then(|value| value.as_f64()),
            evidence: obj
                .get("evidence")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        })
    }

    fn parse_surface_evidence(item: &Value) -> Option<SurfaceEvidenceArtifact> {
        let obj = item.as_object()?;

        let content_text = obj
            .get("content_text")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let content_path = obj
            .get("content_path")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let content_json = obj.get("content_json").cloned();

        if content_text.is_none() && content_path.is_none() && content_json.is_none() {
            return None;
        }

        Some(SurfaceEvidenceArtifact {
            asset_type: Self::required_string(obj, "asset_type")?,
            asset_key: Self::required_string(obj, "asset_key")?,
            evidence_type: Self::required_string(obj, "evidence_type")?,
            title: Self::required_string(obj, "title")?,
            content_text,
            content_path,
            content_json,
            probe_node: obj
                .get("probe_node")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        })
    }

    /// Detect artifact type from raw output
    pub fn detect_type(data: &Value) -> ArtifactType {
        if data.get("surface_artifacts").is_some() || data.get("surface").is_some() {
            return ArtifactType::SurfaceBundle;
        }

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
        if data.get("vulnerability").is_some()
            || data.get("finding").is_some()
            || data.get("findings").is_some()
        {
            return ArtifactType::Finding;
        }
        if data.get("evidences").is_some() || data.get("evidence").is_some() {
            return ArtifactType::Evidence;
        }
        if data.get("subdomains").is_some() {
            return ArtifactType::Subdomains;
        }
        if data.get("domains").is_some() {
            return ArtifactType::SurfaceDomains;
        }
        if data.get("ips").is_some() {
            return ArtifactType::SurfaceIps;
        }
        if data.get("ports").is_some() {
            return ArtifactType::SurfacePorts;
        }
        if data.get("services").is_some() {
            return ArtifactType::SurfaceServices;
        }
        if data.get("webs").is_some() || data.get("web_assets").is_some() {
            return ArtifactType::SurfaceWebs;
        }
        if data.get("certificates").is_some() {
            return ArtifactType::SurfaceCertificates;
        }
        if data.get("fingerprints").is_some() {
            return ArtifactType::SurfaceFingerprints;
        }
        if data.get("relations").is_some() {
            return ArtifactType::SurfaceRelations;
        }
        if data.get("changes").is_some() {
            return ArtifactType::SurfaceChanges;
        }
        if data.get("hosts").is_some()
            || data.get("liveHosts").is_some()
            || data.get("results").is_some()
        {
            // Check if results contain status_code
            if let Some(arr) = data.get("results").and_then(|v| v.as_array()) {
                if arr.iter().any(|item| {
                    item.get("statusCode").is_some() || item.get("status_code").is_some()
                }) {
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
    pub fn extract_finding(data: &Value) -> Option<FindingArtifact> {
        // Check nested structures
        let finding_data = data
            .get("finding")
            .or_else(|| data.get("vulnerability"))
            .unwrap_or(data);

        // Title is required
        let title = finding_data
            .get("title")
            .or_else(|| finding_data.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())?;

        Some(FindingArtifact {
            title,
            description: finding_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            finding_type: finding_data
                .get("finding_type")
                .or_else(|| finding_data.get("type"))
                .or_else(|| finding_data.get("vulnerabilityType"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            severity: finding_data
                .get("severity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            confidence: finding_data
                .get("confidence")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_url: finding_data
                .get("affected_url")
                .or_else(|| finding_data.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_parameter: finding_data
                .get("affected_parameter")
                .or_else(|| finding_data.get("parameter"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cwe_id: finding_data
                .get("cwe_id")
                .or_else(|| finding_data.get("cwe"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            impact: finding_data
                .get("impact")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            remediation: finding_data
                .get("remediation")
                .or_else(|| finding_data.get("recommendation"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            reproduction_steps: finding_data
                .get("reproduction_steps")
                .or_else(|| finding_data.get("steps"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                }),
            raw_output: Some(data.clone()),
        })
    }

    pub fn extract_evidences(data: &Value) -> Option<SurfaceEvidencesArtifact> {
        let evidences = data.get("evidences").and_then(|value| value.as_array())?;
        let evidences = evidences
            .iter()
            .map(Self::parse_surface_evidence)
            .collect::<Option<Vec<_>>>()?;

        Some(SurfaceEvidencesArtifact { evidences })
    }

    /// Extract subdomains from raw output
    pub fn extract_subdomains(data: &Value) -> Option<SubdomainsArtifact> {
        let subdomains_arr = data.get("subdomains").and_then(|v| v.as_array())?;

        let domain = data
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let subdomains: Vec<SubdomainEntry> = subdomains_arr
            .iter()
            .filter_map(|item| {
                if let Some(s) = item.as_str() {
                    Some(SubdomainEntry {
                        subdomain: s.to_string(),
                        source: None,
                    })
                } else if let Some(obj) = item.as_object() {
                    Some(SubdomainEntry {
                        subdomain: obj
                            .get("subdomain")
                            .or_else(|| obj.get("host"))
                            .and_then(|v| v.as_str())?
                            .to_string(),
                        source: obj
                            .get("source")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
                } else {
                    None
                }
            })
            .collect();

        if subdomains.is_empty() {
            return None;
        }

        Some(SubdomainsArtifact { domain, subdomains })
    }

    /// Extract live hosts from raw output
    pub fn extract_live_hosts(data: &Value) -> Option<LiveHostsArtifact> {
        let hosts_arr = data
            .get("hosts")
            .or_else(|| data.get("liveHosts"))
            .or_else(|| data.get("results"))
            .and_then(|v| v.as_array())?;

        let hosts: Vec<LiveHostEntry> = hosts_arr
            .iter()
            .filter_map(|item| {
                let url = item.get("url").and_then(|v| v.as_str())?.to_string();
                let status_code = item
                    .get("statusCode")
                    .or_else(|| item.get("status_code"))
                    .or_else(|| item.get("status"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u16;

                Some(LiveHostEntry {
                    url,
                    status_code,
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    content_length: item
                        .get("contentLength")
                        .or_else(|| item.get("content_length"))
                        .and_then(|v| v.as_u64()),
                    technologies: item
                        .get("technologies")
                        .or_else(|| item.get("tech"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    headers: item.get("headers").and_then(|v| v.as_object()).cloned(),
                })
            })
            .collect();

        if hosts.is_empty() {
            return None;
        }

        Some(LiveHostsArtifact { hosts })
    }

    /// Extract technologies from raw output
    pub fn extract_technologies(data: &Value) -> Option<TechnologiesArtifact> {
        let tech_arr = data
            .get("technologies")
            .or_else(|| data.get("techStack"))
            .and_then(|v| v.as_array())?;

        let url = data
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let technologies: Vec<TechEntry> = tech_arr
            .iter()
            .filter_map(|item| {
                if let Some(name) = item.as_str() {
                    Some(TechEntry {
                        name: name.to_string(),
                        version: None,
                        category: None,
                        confidence: None,
                    })
                } else if let Some(obj) = item.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str())?.to_string();
                    Some(TechEntry {
                        name,
                        version: obj
                            .get("version")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        category: obj
                            .get("category")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        confidence: obj.get("confidence").and_then(|v| v.as_f64()),
                    })
                } else {
                    None
                }
            })
            .collect();

        Some(TechnologiesArtifact { url, technologies })
    }

    /// Extract endpoints from raw output
    pub fn extract_endpoints(data: &Value) -> Option<EndpointsArtifact> {
        let endpoints_arr = data.get("endpoints").and_then(|v| v.as_array())?;

        let base_url = data
            .get("base_url")
            .or_else(|| data.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let endpoints: Vec<EndpointEntry> = endpoints_arr
            .iter()
            .filter_map(|item| {
                if let Some(path) = item.as_str() {
                    Some(EndpointEntry {
                        path: path.to_string(),
                        method: None,
                        source: None,
                        params: vec![],
                    })
                } else if let Some(obj) = item.as_object() {
                    Some(EndpointEntry {
                        path: obj
                            .get("path")
                            .or_else(|| obj.get("url"))
                            .and_then(|v| v.as_str())?
                            .to_string(),
                        method: obj
                            .get("method")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        source: obj
                            .get("source")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        params: obj
                            .get("params")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect();

        if endpoints.is_empty() {
            return None;
        }

        Some(EndpointsArtifact {
            base_url,
            endpoints,
        })
    }

    /// Extract secrets from raw output
    pub fn extract_secrets(data: &Value) -> Option<SecretsArtifact> {
        let secrets_arr = data.get("secrets").and_then(|v| v.as_array())?;

        let secrets: Vec<SecretEntry> = secrets_arr
            .iter()
            .filter_map(|item| {
                let obj = item.as_object()?;
                Some(SecretEntry {
                    secret_type: obj
                        .get("type")
                        .or_else(|| obj.get("secret_type"))
                        .and_then(|v| v.as_str())?
                        .to_string(),
                    value: obj
                        .get("value")
                        .or_else(|| obj.get("secret"))
                        .and_then(|v| v.as_str())?
                        .to_string(),
                    source_url: obj
                        .get("source_url")
                        .or_else(|| obj.get("url"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    line: obj.get("line").and_then(|v| v.as_u64()).map(|n| n as u32),
                    context: obj
                        .get("context")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        if secrets.is_empty() {
            return None;
        }

        Some(SecretsArtifact { secrets })
    }

    /// Extract directories from raw output
    pub fn extract_directories(data: &Value) -> Option<DirectoriesArtifact> {
        let dirs_arr = data
            .get("directories")
            .or_else(|| data.get("paths"))
            .or_else(|| data.get("results"))
            .and_then(|v| v.as_array())?;

        let base_url = data
            .get("base_url")
            .or_else(|| data.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let directories: Vec<DirectoryEntry> = dirs_arr
            .iter()
            .filter_map(|item| {
                if let Some(path) = item.as_str() {
                    Some(DirectoryEntry {
                        path: path.to_string(),
                        status_code: 200,
                        content_length: None,
                        redirect_url: None,
                    })
                } else if let Some(obj) = item.as_object() {
                    Some(DirectoryEntry {
                        path: obj
                            .get("path")
                            .or_else(|| obj.get("url"))
                            .and_then(|v| v.as_str())?
                            .to_string(),
                        status_code: obj
                            .get("status_code")
                            .or_else(|| obj.get("statusCode"))
                            .or_else(|| obj.get("status"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(200) as u16,
                        content_length: obj
                            .get("content_length")
                            .or_else(|| obj.get("contentLength"))
                            .and_then(|v| v.as_u64()),
                        redirect_url: obj
                            .get("redirect_url")
                            .or_else(|| obj.get("location"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
                } else {
                    None
                }
            })
            .collect();

        if directories.is_empty() {
            return None;
        }

        Some(DirectoriesArtifact {
            base_url,
            directories,
        })
    }

    pub fn extract_surface_webs(data: &Value) -> Option<SurfaceWebsArtifact> {
        let webs = data.get("webs").and_then(|value| value.as_array())?;
        let webs = webs
            .iter()
            .map(Self::parse_surface_web)
            .collect::<Option<Vec<_>>>()?;

        Some(SurfaceWebsArtifact { webs })
    }

    pub fn extract_surface_fingerprints(data: &Value) -> Option<SurfaceFingerprintsArtifact> {
        let fingerprints = data
            .get("fingerprints")
            .and_then(|value| value.as_array())?;
        let fingerprints = fingerprints
            .iter()
            .map(Self::parse_surface_fingerprint)
            .collect::<Option<Vec<_>>>()?;

        Some(SurfaceFingerprintsArtifact { fingerprints })
    }

    pub fn extract_surface_bundle(data: &Value) -> Option<SurfaceBundleArtifact> {
        let bundle = data
            .get("surface_artifacts")
            .or_else(|| data.get("surface"))
            .unwrap_or(data);
        let obj = bundle.as_object()?;

        let webs = obj
            .get("webs")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(Self::parse_surface_web)
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .unwrap_or_default();

        let fingerprints = obj
            .get("fingerprints")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(Self::parse_surface_fingerprint)
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .unwrap_or_default();

        Some(SurfaceBundleArtifact {
            organizations: obj
                .get("organizations")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            domains: obj
                .get("domains")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            ips: obj
                .get("ips")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            hosts: obj
                .get("hosts")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            ports: obj
                .get("ports")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            services: obj
                .get("services")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            webs,
            certificates: obj
                .get("certificates")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            fingerprints,
            relations: obj
                .get("relations")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            changes: obj
                .get("changes")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactExtractor;
    use serde_json::json;

    #[test]
    fn extracts_surface_webs_with_strict_schema() {
        let data = json!({
            "webs": [
                {
                    "canonical_url": "https://example.com",
                    "scheme": "https",
                    "site_title": "Example",
                    "http_status_code": 200,
                    "response_headers": {
                        "server": "nginx"
                    },
                    "content_summary": "homepage"
                }
            ]
        });

        let artifact =
            ArtifactExtractor::extract_surface_webs(&data).expect("surface webs should parse");
        assert_eq!(artifact.webs.len(), 1);
        assert_eq!(artifact.webs[0].canonical_url, "https://example.com");
        assert_eq!(artifact.webs[0].http_status_code, 200);
    }

    #[test]
    fn rejects_surface_webs_missing_required_fields() {
        let data = json!({
            "webs": [
                {
                    "canonical_url": "https://example.com",
                    "scheme": "https"
                }
            ]
        });

        assert!(ArtifactExtractor::extract_surface_webs(&data).is_none());
    }

    #[test]
    fn extracts_surface_fingerprints_with_strict_schema() {
        let data = json!({
            "fingerprints": [
                {
                    "asset_type": "web",
                    "asset_key": "https://example.com",
                    "fingerprint_type": "header",
                    "fingerprint_value": "nginx",
                    "rule_id": "rule-nginx",
                    "rule_word": "nginx",
                    "rule_name": "Nginx",
                    "normalized_product": "Nginx",
                    "normalized_category": "web_server",
                    "confidence": 0.95
                }
            ]
        });

        let artifact = ArtifactExtractor::extract_surface_fingerprints(&data)
            .expect("surface fingerprints should parse");
        assert_eq!(artifact.fingerprints.len(), 1);
        assert_eq!(artifact.fingerprints[0].rule_id, "rule-nginx");
        assert_eq!(artifact.fingerprints[0].normalized_category, "web_server");
    }

    #[test]
    fn rejects_surface_fingerprints_missing_normalized_fields() {
        let data = json!({
            "fingerprints": [
                {
                    "asset_type": "web",
                    "asset_key": "https://example.com",
                    "fingerprint_type": "header",
                    "fingerprint_value": "nginx"
                }
            ]
        });

        assert!(ArtifactExtractor::extract_surface_fingerprints(&data).is_none());
    }

    #[test]
    fn extracts_surface_evidences_with_strict_schema() {
        let data = json!({
            "evidences": [
                {
                    "asset_type": "web",
                    "asset_key": "https://example.com",
                    "evidence_type": "http_response_headers",
                    "title": "HTTP Response Headers",
                    "content_json": {
                        "server": "nginx"
                    }
                }
            ]
        });

        let artifact =
            ArtifactExtractor::extract_evidences(&data).expect("surface evidences should parse");
        assert_eq!(artifact.evidences.len(), 1);
        assert_eq!(artifact.evidences[0].evidence_type, "http_response_headers");
    }

    #[test]
    fn rejects_surface_evidences_without_payload_content() {
        let data = json!({
            "evidences": [
                {
                    "asset_type": "web",
                    "asset_key": "https://example.com",
                    "evidence_type": "http_response_headers",
                    "title": "HTTP Response Headers"
                }
            ]
        });

        assert!(ArtifactExtractor::extract_evidences(&data).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_type() {
        assert_eq!(
            ArtifactExtractor::detect_type(&json!({"subdomains": ["a.com"]})),
            ArtifactType::Subdomains
        );
        assert_eq!(
            ArtifactExtractor::detect_type(&json!({"findings": []})),
            ArtifactType::Finding
        );
        assert_eq!(
            ArtifactExtractor::detect_type(&json!({"technologies": []})),
            ArtifactType::Technologies
        );
        assert_eq!(
            ArtifactExtractor::detect_type(&json!({"surface_artifacts": {"domains": []}})),
            ArtifactType::SurfaceBundle
        );
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
