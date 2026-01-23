//! Workflow Orchestrator (P0-4)
//!
//! Integrates artifact protocol, data flow, retry, and auto-sinking into workflow execution.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

use crate::services::workflow_artifact::{
    ArtifactType, ArtifactExtractor, WorkflowArtifact, ArtifactMetadata,
};
use crate::services::data_flow::{DataFlowResolver, ArtifactSinkConfig, ArtifactSinkResult};
use crate::services::retry_executor::{RetryConfig, RateLimiter, RetryExecutor};

/// Workflow step context for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepContext {
    pub step_id: String,
    pub step_name: String,
    pub plugin_id: Option<String>,
    pub tool_name: Option<String>,
    pub config: serde_json::Value,
    pub depends_on: Vec<String>,
    pub retry_config: Option<RetryConfig>,
    pub target_host: Option<String>,
}

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub execution_id: String,
    pub workflow_id: String,
    pub program_id: Option<String>,
    pub scope_id: Option<String>,
    pub initial_inputs: serde_json::Value,
    pub sink_config: ArtifactSinkConfig,
}

/// Orchestrator for bounty workflow execution
pub struct WorkflowOrchestrator {
    data_flow_resolver: DataFlowResolver,
    rate_limiter: Arc<RateLimiter>,
    default_retry_config: RetryConfig,
}

impl Default for WorkflowOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowOrchestrator {
    pub fn new() -> Self {
        Self {
            data_flow_resolver: DataFlowResolver::new(),
            rate_limiter: Arc::new(RateLimiter::default_limits()),
            default_retry_config: RetryConfig::for_network(),
        }
    }
    
    pub fn with_rate_limits(global: usize, per_host: usize, delay_ms: u64) -> Self {
        Self {
            data_flow_resolver: DataFlowResolver::new(),
            rate_limiter: Arc::new(RateLimiter::new(global, per_host, delay_ms)),
            default_retry_config: RetryConfig::for_network(),
        }
    }
    
    /// Resolve step inputs from upstream outputs
    pub fn resolve_step_inputs(
        &self,
        step: &StepContext,
        upstream_results: &HashMap<String, serde_json::Value>,
        initial_inputs: &serde_json::Value,
    ) -> serde_json::Value {
        let plugin_id = step.plugin_id.as_deref()
            .or(step.tool_name.as_deref())
            .unwrap_or("");
        
        // Collect relevant upstream outputs based on dependencies
        let upstream_outputs: Vec<(&str, serde_json::Value)> = step.depends_on.iter()
            .filter_map(|dep_id| {
                upstream_results.get(dep_id).map(|v| (dep_id.as_str(), v.clone()))
            })
            .collect();
        
        // Merge initial inputs into step config
        let mut merged_config = step.config.clone();
        if let (Some(config_obj), Some(initial_obj)) = (merged_config.as_object_mut(), initial_inputs.as_object()) {
            for (key, value) in initial_obj {
                // Only set if not already set in config
                if !config_obj.contains_key(key) || is_empty_value(config_obj.get(key).unwrap()) {
                    config_obj.insert(key.clone(), value.clone());
                }
            }
        }
        
        // Resolve from upstream outputs
        self.data_flow_resolver.resolve_inputs(plugin_id, &merged_config, &upstream_outputs)
    }
    
    /// Process step output into typed artifacts
    pub fn process_step_output(
        &self,
        step: &StepContext,
        execution_id: &str,
        raw_output: &serde_json::Value,
    ) -> Vec<WorkflowArtifact> {
        let mut artifacts = Vec::new();
        
        // Detect artifact type
        let artifact_type = ArtifactExtractor::detect_type(raw_output);
        
        // Extract output data (handle wrapped results)
        let output_data = raw_output.get("output")
            .or_else(|| raw_output.get("result"))
            .unwrap_or(raw_output);
        
        let base_metadata = ArtifactMetadata {
            source: step.plugin_id.clone().or_else(|| step.tool_name.clone()),
            confidence: None,
            duration_ms: None,
            count: None,
            tags: vec![],
            extra: serde_json::Map::new(),
        };
        
        match artifact_type {
            ArtifactType::Finding => {
                // Handle multiple findings
                if let Some(findings_arr) = output_data.get("findings").and_then(|v| v.as_array()) {
                    for finding_data in findings_arr {
                        if let Some(finding) = ArtifactExtractor::extract_finding(finding_data) {
                            artifacts.push(WorkflowArtifact {
                                id: Uuid::new_v4().to_string(),
                                step_id: step.step_id.clone(),
                                execution_id: execution_id.to_string(),
                                artifact_type: ArtifactType::Finding,
                                data: serde_json::to_value(&finding).unwrap_or_default(),
                                metadata: base_metadata.clone(),
                                created_at: Utc::now(),
                            });
                        }
                    }
                } else if let Some(finding) = ArtifactExtractor::extract_finding(output_data) {
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Finding,
                        data: serde_json::to_value(&finding).unwrap_or_default(),
                        metadata: base_metadata.clone(),
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::Subdomains => {
                if let Some(subdomains) = ArtifactExtractor::extract_subdomains(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(subdomains.subdomains.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Subdomains,
                        data: serde_json::to_value(&subdomains).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::LiveHosts => {
                if let Some(hosts) = ArtifactExtractor::extract_live_hosts(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(hosts.hosts.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::LiveHosts,
                        data: serde_json::to_value(&hosts).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::Technologies => {
                if let Some(tech) = ArtifactExtractor::extract_technologies(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(tech.technologies.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Technologies,
                        data: serde_json::to_value(&tech).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::Endpoints => {
                if let Some(endpoints) = ArtifactExtractor::extract_endpoints(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(endpoints.endpoints.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Endpoints,
                        data: serde_json::to_value(&endpoints).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::Secrets => {
                if let Some(secrets) = ArtifactExtractor::extract_secrets(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(secrets.secrets.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Secrets,
                        data: serde_json::to_value(&secrets).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            ArtifactType::Directories => {
                if let Some(dirs) = ArtifactExtractor::extract_directories(output_data) {
                    let mut metadata = base_metadata.clone();
                    metadata.count = Some(dirs.directories.len());
                    artifacts.push(WorkflowArtifact {
                        id: Uuid::new_v4().to_string(),
                        step_id: step.step_id.clone(),
                        execution_id: execution_id.to_string(),
                        artifact_type: ArtifactType::Directories,
                        data: serde_json::to_value(&dirs).unwrap_or_default(),
                        metadata,
                        created_at: Utc::now(),
                    });
                }
            }
            _ => {
                // Store as raw data artifact
                artifacts.push(WorkflowArtifact {
                    id: Uuid::new_v4().to_string(),
                    step_id: step.step_id.clone(),
                    execution_id: execution_id.to_string(),
                    artifact_type: ArtifactType::RawData,
                    data: output_data.clone(),
                    metadata: base_metadata,
                    created_at: Utc::now(),
                });
            }
        }
        
        artifacts
    }
    
    /// Create retry executor for a step
    pub fn create_retry_executor(&self, step: &StepContext) -> RetryExecutor {
        let config = step.retry_config.clone().unwrap_or_else(|| self.default_retry_config.clone());
        RetryExecutor::new(config).with_rate_limiter(self.rate_limiter.clone())
    }
    
    /// Get rate limiter reference
    pub fn rate_limiter(&self) -> Arc<RateLimiter> {
        self.rate_limiter.clone()
    }
    
    /// Prepare artifact sink request from workflow artifacts
    pub fn prepare_sink_request(
        &self,
        context: &WorkflowContext,
        artifacts: &[WorkflowArtifact],
    ) -> Vec<SinkableArtifact> {
        artifacts.iter().map(|artifact| {
            SinkableArtifact {
                artifact_id: artifact.id.clone(),
                step_id: artifact.step_id.clone(),
                execution_id: artifact.execution_id.clone(),
                artifact_type: artifact.artifact_type.clone(),
                data: artifact.data.clone(),
                program_id: context.program_id.clone(),
                scope_id: context.scope_id.clone(),
            }
        }).collect()
    }
}

/// Artifact ready to be sinked to database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkableArtifact {
    pub artifact_id: String,
    pub step_id: String,
    pub execution_id: String,
    pub artifact_type: ArtifactType,
    pub data: serde_json::Value,
    pub program_id: Option<String>,
    pub scope_id: Option<String>,
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

/// Summary of workflow execution with all artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionSummary {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub artifacts: ArtifactSummary,
    pub sink_result: Option<ArtifactSinkResult>,
    pub duration_ms: u64,
    pub errors: Vec<StepError>,
}

/// Summary of artifacts produced
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactSummary {
    pub findings: usize,
    pub evidence: usize,
    pub assets: usize,
    pub subdomains: usize,
    pub live_hosts: usize,
    pub technologies: usize,
    pub endpoints: usize,
    pub secrets: usize,
    pub directories: usize,
    pub raw_data: usize,
}

impl ArtifactSummary {
    pub fn from_artifacts(artifacts: &[WorkflowArtifact]) -> Self {
        let mut summary = Self::default();
        for artifact in artifacts {
            match artifact.artifact_type {
                ArtifactType::Finding => summary.findings += artifact.metadata.count.unwrap_or(1),
                ArtifactType::Evidence => summary.evidence += artifact.metadata.count.unwrap_or(1),
                ArtifactType::Asset => summary.assets += artifact.metadata.count.unwrap_or(1),
                ArtifactType::Subdomains => summary.subdomains += artifact.metadata.count.unwrap_or(0),
                ArtifactType::LiveHosts => summary.live_hosts += artifact.metadata.count.unwrap_or(0),
                ArtifactType::Technologies => summary.technologies += artifact.metadata.count.unwrap_or(0),
                ArtifactType::Endpoints => summary.endpoints += artifact.metadata.count.unwrap_or(0),
                ArtifactType::Secrets => summary.secrets += artifact.metadata.count.unwrap_or(0),
                ArtifactType::Directories => summary.directories += artifact.metadata.count.unwrap_or(0),
                ArtifactType::RawData => summary.raw_data += 1,
            }
        }
        summary
    }
}

/// Step error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepError {
    pub step_id: String,
    pub step_name: String,
    pub error: String,
    pub retries: u32,
    pub is_critical: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_resolve_step_inputs() {
        let orchestrator = WorkflowOrchestrator::new();
        
        let step = StepContext {
            step_id: "step-2".to_string(),
            step_name: "HTTP Prober".to_string(),
            plugin_id: Some("http_prober".to_string()),
            tool_name: None,
            config: json!({
                "targets": [],
                "ports": [80, 443]
            }),
            depends_on: vec!["step-1".to_string()],
            retry_config: None,
            target_host: None,
        };
        
        let mut upstream_results = HashMap::new();
        upstream_results.insert("step-1".to_string(), json!({
            "domain": "example.com",
            "subdomains": [
                {"subdomain": "a.example.com"},
                {"subdomain": "b.example.com"}
            ]
        }));
        
        let initial_inputs = json!({});
        
        let resolved = orchestrator.resolve_step_inputs(&step, &upstream_results, &initial_inputs);
        
        // Should have resolved targets
        assert!(resolved.get("targets").is_some());
        assert!(resolved.get("ports").is_some());
    }
    
    #[test]
    fn test_process_step_output() {
        let orchestrator = WorkflowOrchestrator::new();
        
        let step = StepContext {
            step_id: "step-1".to_string(),
            step_name: "Subdomain Enum".to_string(),
            plugin_id: Some("subdomain_enumerator".to_string()),
            tool_name: None,
            config: json!({}),
            depends_on: vec![],
            retry_config: None,
            target_host: None,
        };
        
        let raw_output = json!({
            "success": true,
            "output": {
                "domain": "example.com",
                "subdomains": [
                    {"subdomain": "a.example.com"},
                    {"subdomain": "b.example.com"}
                ]
            }
        });
        
        let artifacts = orchestrator.process_step_output(&step, "exec-1", &raw_output);
        
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_type, ArtifactType::Subdomains);
        assert_eq!(artifacts[0].metadata.count, Some(2));
    }
}
