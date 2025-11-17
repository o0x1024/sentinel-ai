//! Website analyzer MCP tools

use async_trait::async_trait;
use sentinel_tools::unified_types::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

use crate::analyzers::{WebsiteAnalyzer, WebsiteAnalysis};
use sentinel_passive::PassiveDatabaseService;

/// Analyze website tool
pub struct AnalyzeWebsiteTool {
    db_service: Arc<PassiveDatabaseService>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl std::fmt::Debug for AnalyzeWebsiteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeWebsiteTool")
            .field("parameters", &self.parameters)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl AnalyzeWebsiteTool {
    pub fn new(db_service: Arc<PassiveDatabaseService>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "domain".to_string(),
                    param_type: ParameterType::String,
                    description: "Domain name to analyze (e.g., 'example.com')".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Domain name to analyze (e.g., 'example.com')"
                    }
                },
                "required": ["domain"]
            }),
            required: vec!["domain".to_string()],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["analyzer".to_string(), "website".to_string(), "plan-b".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            db_service,
            parameters,
            metadata,
        }
    }
}

#[async_trait]
impl UnifiedTool for AnalyzeWebsiteTool {
    fn name(&self) -> &str {
        "analyze_website"
    }

    fn description(&self) -> &str {
        "Analyze a website's structure, API endpoints, parameters, and technology stack based on captured HTTP traffic. This is Plan B advanced feature for intelligent plugin generation."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = Utc::now();

        let domain = params.inputs.get("domain")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: domain"))?;

        log::info!("Analyzing website: {}", domain);

        // Create analyzer
        let analyzer = WebsiteAnalyzer::new(self.db_service.clone());

        // Perform analysis
        let analysis = analyzer.analyze(domain).await?;

        // Build detailed response
        let mut output_parts = Vec::new();

        output_parts.push(format!("🔍 Website Analysis: {}", analysis.domain));
        output_parts.push(format!("Total Requests Analyzed: {}", analysis.total_requests));
        output_parts.push(String::new());

        // API Endpoints
        output_parts.push(format!("📊 API Endpoints Discovered: {}", analysis.endpoints.len()));
        if !analysis.endpoints.is_empty() {
            output_parts.push(String::new());
            for (idx, endpoint) in analysis.endpoints.iter().take(10).enumerate() {
                output_parts.push(format!(
                    "{}. {} {} (pattern: {}, hits: {})",
                    idx + 1,
                    endpoint.method,
                    endpoint.path,
                    endpoint.pattern,
                    endpoint.hit_count
                ));

                if !endpoint.query_params.is_empty() {
                    let params_str: Vec<String> = endpoint.query_params.iter()
                        .map(|p| format!("{}:{:?}", p.name, p.param_type))
                        .collect();
                    output_parts.push(format!("   Query params: {}", params_str.join(", ")));
                }

                if !endpoint.body_params.is_empty() {
                    let params_str: Vec<String> = endpoint.body_params.iter()
                        .map(|p| format!("{}:{:?}", p.name, p.param_type))
                        .collect();
                    output_parts.push(format!("   Body params: {}", params_str.join(", ")));
                }
            }

            if analysis.endpoints.len() > 10 {
                output_parts.push(format!("   ... and {} more endpoints", analysis.endpoints.len() - 10));
            }
        }

        output_parts.push(String::new());

        // Technology Stack
        output_parts.push("🛠️  Technology Stack Detected:".to_string());
        if let Some(ref server) = analysis.tech_stack.server {
            output_parts.push(format!("   Server: {}", server));
        }
        if let Some(ref framework) = analysis.tech_stack.framework {
            output_parts.push(format!("   Framework: {}", framework));
        }
        if let Some(ref db) = analysis.tech_stack.database {
            output_parts.push(format!("   Database: {}", db));
        }
        if let Some(ref lang) = analysis.tech_stack.language {
            output_parts.push(format!("   Language: {}", lang));
        }
        if !analysis.tech_stack.others.is_empty() {
            output_parts.push(format!("   Others: {}", analysis.tech_stack.others.join(", ")));
        }

        output_parts.push(String::new());

        // Parameters Summary
        output_parts.push(format!("📋 Unique Parameters Found: {}", analysis.all_parameters.len()));
        if !analysis.all_parameters.is_empty() {
            let param_names: Vec<String> = analysis.all_parameters.iter()
                .take(20)
                .map(|p| p.name.clone())
                .collect();
            output_parts.push(format!("   {}", param_names.join(", ")));
            if analysis.all_parameters.len() > 20 {
                output_parts.push(format!("   ... and {} more", analysis.all_parameters.len() - 20));
            }
        }

        output_parts.push(String::new());
        output_parts.push(format!("📦 Static Resources: {}", analysis.static_resources_count));
        output_parts.push(format!("🔌 API Endpoints: {}", analysis.api_endpoints_count));

        let output_text = output_parts.join("\n");

        let end_time = Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "analyze_website".to_string(),
            tool_id: "analyzer.analyze_website".to_string(),
            success: true,
            output: json!({
                "analysis": analysis,
                "summary": output_text
            }),
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

/// Website analyzer tool provider
#[derive(Debug)]
pub struct AnalyzerToolProvider {
    passive_state: Arc<crate::commands::passive_scan_commands::PassiveScanState>,
}

impl AnalyzerToolProvider {
    pub fn new(passive_state: Arc<crate::commands::passive_scan_commands::PassiveScanState>) -> Self {
        Self { passive_state }
    }
}

#[async_trait]
impl ToolProvider for AnalyzerToolProvider {
    fn name(&self) -> &str {
        "analyzer"
    }

    fn description(&self) -> &str {
        "Website analysis tools for Plan B advanced plugin generation"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        let db_service = self.passive_state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;
        
        Ok(vec![
            Arc::new(AnalyzeWebsiteTool::new(db_service)),
        ])
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        let db_service = self.passive_state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;
        
        match name {
            "analyze_website" => Ok(Some(Arc::new(AnalyzeWebsiteTool::new(db_service)))),
            _ => Ok(None),
        }
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
