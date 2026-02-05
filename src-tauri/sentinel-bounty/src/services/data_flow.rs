//! Workflow Data Flow Orchestration (P0-2)
//!
//! Handles automatic data passing between workflow steps based on port connections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::services::workflow_artifact::{ArtifactType, ArtifactExtractor, SubdomainsArtifact, LiveHostsArtifact};

/// Step output port mapping - defines what a step produces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutputMapping {
    pub step_id: String,
    pub plugin_id: String,
    /// Maps output port name -> artifact type
    pub output_ports: HashMap<String, ArtifactType>,
}

/// Step input port mapping - defines what a step consumes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInputMapping {
    pub step_id: String,
    pub plugin_id: String,
    /// Maps input param name -> expected artifact type
    pub input_params: HashMap<String, ParamBindingSpec>,
}

/// How to bind input param to upstream output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamBindingSpec {
    /// Expected artifact type
    pub artifact_type: ArtifactType,
    /// JSON path to extract (e.g., "subdomains[*].subdomain")
    pub extract_path: Option<String>,
    /// Whether this is required
    pub required: bool,
}

/// Plugin port registry - known plugin input/output contracts
pub struct PluginPortRegistry {
    /// plugin_id -> output ports
    output_specs: HashMap<String, Vec<(String, ArtifactType)>>,
    /// plugin_id -> input binding specs
    input_specs: HashMap<String, Vec<(String, ParamBindingSpec)>>,
}

impl Default for PluginPortRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginPortRegistry {
    pub fn new() -> Self {
        let registry = Self {
            output_specs: HashMap::new(),
            input_specs: HashMap::new(),
        };
        // registry.register_builtin_plugins();
        registry
    }
    
    #[allow(dead_code)]
    fn register_builtin_plugins(&mut self) {
        // Subdomain Enumerator
        self.output_specs.insert("subdomain_enumerator".to_string(), vec![
            ("subdomains".to_string(), ArtifactType::Subdomains),
        ]);
        self.input_specs.insert("subdomain_enumerator".to_string(), vec![
            ("domain".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::RawData,
                extract_path: None,
                required: true,
            }),
        ]);
        
        // HTTP Prober
        self.output_specs.insert("http_prober".to_string(), vec![
            ("live_hosts".to_string(), ArtifactType::LiveHosts),
        ]);
        self.input_specs.insert("http_prober".to_string(), vec![
            ("targets".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::Subdomains,
                extract_path: Some("subdomains[*].subdomain".to_string()),
                required: true,
            }),
        ]);
        
        // Tech Fingerprinter
        self.output_specs.insert("tech_fingerprinter".to_string(), vec![
            ("technologies".to_string(), ArtifactType::Technologies),
        ]);
        self.input_specs.insert("tech_fingerprinter".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // Directory Bruteforcer
        self.output_specs.insert("directory_bruteforcer".to_string(), vec![
            ("directories".to_string(), ArtifactType::Directories),
        ]);
        self.input_specs.insert("directory_bruteforcer".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // JS Analyzer
        self.output_specs.insert("js_analyzer".to_string(), vec![
            ("endpoints".to_string(), ArtifactType::Endpoints),
            ("secrets".to_string(), ArtifactType::Secrets),
        ]);
        self.input_specs.insert("js_analyzer".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // SSRF Detector
        self.output_specs.insert("ssrf_detector".to_string(), vec![
            ("findings".to_string(), ArtifactType::Finding),
        ]);
        self.input_specs.insert("ssrf_detector".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // CORS Misconfiguration
        self.output_specs.insert("cors_misconfiguration".to_string(), vec![
            ("findings".to_string(), ArtifactType::Finding),
        ]);
        self.input_specs.insert("cors_misconfiguration".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // Open Redirect Detector
        self.output_specs.insert("open_redirect_detector".to_string(), vec![
            ("findings".to_string(), ArtifactType::Finding),
        ]);
        self.input_specs.insert("open_redirect_detector".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // Next.js RCE Scanner
        self.output_specs.insert("nextjs_rce_scanner".to_string(), vec![
            ("findings".to_string(), ArtifactType::Finding),
        ]);
        self.input_specs.insert("nextjs_rce_scanner".to_string(), vec![
            ("url".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::LiveHosts,
                extract_path: Some("hosts[0].url".to_string()),
                required: true,
            }),
        ]);
        
        // Subdomain Takeover
        self.output_specs.insert("subdomain_takeover".to_string(), vec![
            ("findings".to_string(), ArtifactType::Finding),
        ]);
        self.input_specs.insert("subdomain_takeover".to_string(), vec![
            ("domain".to_string(), ParamBindingSpec {
                artifact_type: ArtifactType::Subdomains,
                extract_path: Some("subdomains[*].subdomain".to_string()),
                required: true,
            }),
        ]);
    }
    
    /// Get output ports for a plugin
    pub fn get_output_ports(&self, plugin_id: &str) -> Option<&Vec<(String, ArtifactType)>> {
        self.output_specs.get(plugin_id)
    }
    
    /// Get input specs for a plugin
    pub fn get_input_specs(&self, plugin_id: &str) -> Option<&Vec<(String, ParamBindingSpec)>> {
        self.input_specs.get(plugin_id)
    }
}

/// Data flow resolver - resolves upstream data to downstream input params
pub struct DataFlowResolver {
    registry: PluginPortRegistry,
}

impl Default for DataFlowResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DataFlowResolver {
    pub fn new() -> Self {
        Self {
            registry: PluginPortRegistry::new(),
        }
    }
    
    /// Resolve input params for a step based on upstream outputs
    pub fn resolve_inputs(
        &self,
        step_plugin_id: &str,
        step_config: &serde_json::Value,
        upstream_outputs: &[(&str, serde_json::Value)], // (step_id, output_data)
    ) -> serde_json::Value {
        let mut resolved = step_config.clone();
        let resolved_obj = match resolved.as_object_mut() {
            Some(obj) => obj,
            None => return resolved,
        };
        
        // Get input specs for this plugin
        let input_specs = match self.registry.get_input_specs(step_plugin_id) {
            Some(specs) => specs,
            None => return resolved,
        };
        
        for (param_name, binding_spec) in input_specs {
            // Skip if already has a non-empty value
            if let Some(existing) = resolved_obj.get(param_name) {
                if !is_empty_value(existing) {
                    continue;
                }
            }
            
            // Try to find matching upstream output
            for (_upstream_step_id, upstream_data) in upstream_outputs {
                if let Some(extracted) = self.extract_value(upstream_data, binding_spec) {
                    resolved_obj.insert(param_name.clone(), extracted);
                    break;
                }
            }
        }
        
        resolved
    }
    
    /// Extract value from upstream data based on binding spec
    fn extract_value(
        &self,
        upstream_data: &serde_json::Value,
        binding_spec: &ParamBindingSpec,
    ) -> Option<serde_json::Value> {
        // Check if upstream data matches expected artifact type
        let detected_type = ArtifactExtractor::detect_type(upstream_data);
        if detected_type != binding_spec.artifact_type && detected_type != ArtifactType::RawData {
            return None;
        }
        
        // Extract based on path
        match &binding_spec.extract_path {
            Some(path) => self.extract_by_path(upstream_data, path),
            None => Some(upstream_data.clone()),
        }
    }
    
    /// Extract value by JSON path (simplified implementation)
    fn extract_by_path(&self, data: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        self.extract_recursive(data, &parts)
    }
    
    fn extract_recursive(&self, data: &serde_json::Value, path_parts: &[&str]) -> Option<serde_json::Value> {
        if path_parts.is_empty() {
            return Some(data.clone());
        }
        
        let part = path_parts[0];
        let remaining = &path_parts[1..];
        
        // Handle array wildcard: "field[*].subfield"
        if part.ends_with("[*]") {
            let field_name = &part[..part.len() - 3];
            let arr = data.get(field_name)?.as_array()?;
            
            if remaining.is_empty() {
                // Return the array as-is
                return Some(serde_json::Value::Array(arr.clone()));
            }
            
            // Map over array items and extract subfield
            let extracted: Vec<serde_json::Value> = arr.iter()
                .filter_map(|item| self.extract_recursive(item, remaining))
                .collect();
            
            if extracted.is_empty() {
                return None;
            }
            return Some(serde_json::Value::Array(extracted));
        }
        
        // Handle array index: "field[0]"
        if part.contains('[') && part.ends_with(']') {
            let bracket_pos = part.find('[')?;
            let field_name = &part[..bracket_pos];
            let index_str = &part[bracket_pos + 1..part.len() - 1];
            let index: usize = index_str.parse().ok()?;
            
            let arr = data.get(field_name)?.as_array()?;
            let item = arr.get(index)?;
            
            return self.extract_recursive(item, remaining);
        }
        
        // Regular field access
        let next = data.get(part)?;
        self.extract_recursive(next, remaining)
    }
    
    /// Transform subdomains artifact to targets array for http_prober
    pub fn subdomains_to_targets(artifact: &SubdomainsArtifact) -> serde_json::Value {
        let targets: Vec<String> = artifact.subdomains.iter()
            .map(|s| s.subdomain.clone())
            .collect();
        serde_json::json!(targets)
    }
    
    /// Transform live hosts to single URL for downstream plugins
    pub fn live_hosts_to_url(artifact: &LiveHostsArtifact) -> Option<serde_json::Value> {
        artifact.hosts.first().map(|h| serde_json::json!(h.url))
    }
    
    /// Transform live hosts to URL list for batch scanning
    pub fn live_hosts_to_urls(artifact: &LiveHostsArtifact) -> serde_json::Value {
        let urls: Vec<&str> = artifact.hosts.iter().map(|h| h.url.as_str()).collect();
        serde_json::json!(urls)
    }
}

fn is_empty_value(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Artifact sink controller - handles automatic sinking of artifacts to database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSinkConfig {
    /// Whether to auto-create findings
    pub auto_create_findings: bool,
    /// Whether to auto-create evidence
    pub auto_create_evidence: bool,
    /// Whether to auto-update assets
    pub auto_update_assets: bool,
    /// Minimum confidence to create finding (0.0 - 1.0)
    pub min_confidence: f64,
    /// Severity threshold (only sink findings >= this severity)
    pub min_severity: Option<String>,
    /// Whether to deduplicate by fingerprint
    pub deduplicate: bool,
}

impl Default for ArtifactSinkConfig {
    fn default() -> Self {
        Self {
            auto_create_findings: true,
            auto_create_evidence: true,
            auto_update_assets: true,
            min_confidence: 0.5,
            min_severity: None,
            deduplicate: true,
        }
    }
}

/// Artifact sink result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSinkResult {
    pub findings_created: Vec<String>,
    pub evidence_created: Vec<String>,
    pub assets_created: Vec<String>,
    pub assets_updated: Vec<String>,
    pub subdomains_imported: usize,
    pub live_hosts_imported: usize,
    pub skipped_duplicates: usize,
    pub errors: Vec<String>,
}

impl Default for ArtifactSinkResult {
    fn default() -> Self {
        Self {
            findings_created: vec![],
            evidence_created: vec![],
            assets_created: vec![],
            assets_updated: vec![],
            subdomains_imported: 0,
            live_hosts_imported: 0,
            skipped_duplicates: 0,
            errors: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_extract_by_path() {
        let resolver = DataFlowResolver::new();
        
        let data = json!({
            "subdomains": [
                {"subdomain": "a.com"},
                {"subdomain": "b.com"}
            ]
        });
        
        let result = resolver.extract_by_path(&data, "subdomains[*].subdomain").unwrap();
        assert_eq!(result, json!(["a.com", "b.com"]));
    }
    
    #[test]
    fn test_extract_first_element() {
        let resolver = DataFlowResolver::new();
        
        let data = json!({
            "hosts": [
                {"url": "https://a.com"},
                {"url": "https://b.com"}
            ]
        });
        
        let result = resolver.extract_by_path(&data, "hosts[0].url").unwrap();
        assert_eq!(result, json!("https://a.com"));
    }
    
    #[test]
    fn test_resolve_inputs() {
        let resolver = DataFlowResolver::new();
        
        let step_config = json!({
            "targets": [],
            "ports": [80, 443]
        });
        
        let upstream_output = json!({
            "domain": "example.com",
            "subdomains": [
                {"subdomain": "a.example.com"},
                {"subdomain": "b.example.com"}
            ]
        });
        
        let resolved = resolver.resolve_inputs(
            "http_prober",
            &step_config,
            &[("step-1", upstream_output)],
        );
        
        // Should have resolved targets from subdomains
        let targets = resolved.get("targets").unwrap();
        assert!(targets.is_array());
    }
}
