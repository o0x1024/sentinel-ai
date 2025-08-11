//! MCP工具提供者
//! 
//! 提供MCP (Model Context Protocol) 工具的统一管理和调用接口

use super::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

// ============================================================================
// MCP工具提供者
// ============================================================================

#[derive(Debug)]
pub struct McpToolProvider {
    tools: Arc<RwLock<HashMap<String, Arc<dyn UnifiedTool>>>>,
    mcp_client: Option<McpClient>,
}

impl McpToolProvider {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            mcp_client: None,
        }
    }

    pub async fn initialize(&mut self, config: McpConfig) -> Result<()> {
        info!("Initializing MCP tool provider with config: {:?}", config);
        
        let client = McpClient::new(config).await?;
        self.mcp_client = Some(client);
        
        self.refresh_tools().await?;
        Ok(())
    }

    async fn refresh_tools(&self) -> Result<()> {
        let client = self.mcp_client.as_ref()
            .ok_or_else(|| anyhow!("MCP client not initialized"))?;
        
        let mcp_tools = client.list_tools().await?;
        let mut tools = self.tools.write().await;
        tools.clear();
        
        for mcp_tool in mcp_tools {
            let wrapper = Arc::new(McpToolWrapper::new(mcp_tool, client.clone()));
            tools.insert(wrapper.name().to_string(), wrapper);
        }
        
        info!("Refreshed {} MCP tools", tools.len());
        Ok(())
    }
}

#[async_trait]
impl ToolProvider for McpToolProvider {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Model Context Protocol (MCP) tools provider"
    }

    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>> {
        let tools = self.tools.read().await;
        Ok(tools.values().cloned().collect())
    }

    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>> {
        let tools = self.tools.read().await;
        Ok(tools.get(name).cloned())
    }

    async fn refresh(&self) -> Result<()> {
        self.refresh_tools().await
    }

    async fn is_available(&self) -> bool {
        self.mcp_client.is_some()
    }
}

// ============================================================================
// MCP配置
// ============================================================================

#[derive(Debug, Clone)]
pub struct McpConfig {
    pub server_url: String,
    pub timeout: std::time::Duration,
    pub max_retries: u32,
    pub auth_token: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            timeout: std::time::Duration::from_secs(30),
            max_retries: 3,
            auth_token: None,
        }
    }
}

// ============================================================================
// MCP客户端
// ============================================================================

#[derive(Clone)]
#[derive(Debug)]
pub struct McpClient {
    config: McpConfig,
    client: reqwest::Client,
}

impl McpClient {
    pub async fn new(config: McpConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        
        Ok(Self { config, client })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let url = format!("{}/tools", self.config.server_url);
        
        let mut request = self.client.get(&url);
        
        if let Some(ref token) = self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await
            .map_err(|e| anyhow!("Failed to list MCP tools: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("MCP server returned error: {}", response.status()));
        }
        
        let tools: Vec<McpTool> = response.json().await
            .map_err(|e| anyhow!("Failed to parse MCP tools response: {}", e))?;
        
        Ok(tools)
    }

    pub async fn call_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        let url = format!("{}/tools/{}/call", self.config.server_url, tool_name);
        
        let mut request = self.client.post(&url).json(&params);
        
        if let Some(ref token) = self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await
            .map_err(|e| anyhow!("Failed to call MCP tool: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("MCP tool call failed: {}", response.status()));
        }
        
        let result: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse MCP tool response: {}", e))?;
        
        Ok(result)
    }

    pub async fn get_tool_info(&self, tool_name: &str) -> Result<McpTool> {
        let url = format!("{}/tools/{}", self.config.server_url, tool_name);
        
        let mut request = self.client.get(&url);
        
        if let Some(ref token) = self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await
            .map_err(|e| anyhow!("Failed to get MCP tool info: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("MCP server returned error: {}", response.status()));
        }
        
        let tool: McpTool = response.json().await
            .map_err(|e| anyhow!("Failed to parse MCP tool info: {}", e))?;
        
        Ok(tool)
    }
}

// ============================================================================
// MCP工具定义
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub parameters: McpToolParameters,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolParameters {
    pub properties: HashMap<String, McpParameterDefinition>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpParameterDefinition {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: Option<String>,
    pub default: Option<Value>,
    pub enum_values: Option<Vec<Value>>,
}

// ============================================================================
// MCP工具包装器
// ============================================================================

#[derive(Debug)]
pub struct McpToolWrapper {
    mcp_tool: McpTool,
    client: McpClient,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl McpToolWrapper {
    pub fn new(mcp_tool: McpTool, client: McpClient) -> Self {
        let parameters = Self::convert_parameters(&mcp_tool.parameters);
        let metadata = Self::convert_metadata(&mcp_tool);
        
        Self {
            mcp_tool,
            client,
            parameters,
            metadata,
        }
    }

    fn convert_parameters(mcp_params: &McpToolParameters) -> ToolParameters {
        let mut parameters = Vec::new();
        
        for (name, param_def) in &mcp_params.properties {
            let param_type = match param_def.param_type.as_str() {
                "string" => ParameterType::String,
                "number" | "integer" => ParameterType::Number,
                "boolean" => ParameterType::Boolean,
                "array" => ParameterType::Array,
                "object" => ParameterType::Object,
                _ => ParameterType::String,
            };
            
            let parameter = ParameterDefinition {
                name: name.clone(),
                param_type,
                description: param_def.description.clone().unwrap_or_default(),
                required: mcp_params.required.contains(name),
                default_value: param_def.default.clone(),
            };
            
            parameters.push(parameter);
        }
        
        ToolParameters { parameters }
    }

    fn convert_metadata(mcp_tool: &McpTool) -> ToolMetadata {
        let mut tags = vec!["mcp".to_string()];
        
        if let Some(category) = &mcp_tool.category {
            tags.push(category.clone());
        }
        
        ToolMetadata {
            author: "MCP Provider".to_string(),
            version: "1.0.0".to_string(),
            license: "Unknown".to_string(),
            tags,
            requirements: vec!["mcp".to_string()],
        }
    }

    fn convert_category(category: Option<&String>) -> ToolCategory {
        match category.map(|s| s.as_str()) {
            Some("network") | Some("scanning") => ToolCategory::NetworkScanning,
            Some("vulnerability") | Some("security") => ToolCategory::VulnerabilityScanning,
            Some("service") | Some("detection") => ToolCategory::ServiceDetection,
            Some("code") | Some("analysis") => ToolCategory::CodeAnalysis,
            Some("data") | Some("processing") => ToolCategory::DataProcessing,
            Some("system") | Some("utility") => ToolCategory::SystemUtility,
            Some(other) => ToolCategory::Custom(other.to_string()),
            None => ToolCategory::Custom("mcp".to_string()),
        }
    }
}

#[async_trait]
impl UnifiedTool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.mcp_tool.name
    }

    fn description(&self) -> &str {
        &self.mcp_tool.description
    }

    fn category(&self) -> ToolCategory {
        Self::convert_category(self.mcp_tool.category.as_ref())
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        // 简单的健康检查
        match self.client.get_tool_info(&self.mcp_tool.name).await {
            Ok(_) => true,
            Err(e) => {
                warn!("MCP tool {} is not available: {}", self.mcp_tool.name, e);
                false
            }
        }
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        
        info!("Executing MCP tool: {} with execution_id: {}", self.mcp_tool.name, execution_id);
        
        // 转换参数格式
        let mcp_params = json!(params.inputs);
        
        // 调用MCP工具
        match self.client.call_tool(&self.mcp_tool.name, mcp_params).await {
            Ok(output) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.mcp_tool.name.clone(),
                    success: true,
                    output,
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                error!("MCP tool execution failed: {}", e);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.mcp_tool.name.clone(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

// ============================================================================
// MCP工具管理器
// ============================================================================

pub struct McpToolManager {
    providers: HashMap<String, McpToolProvider>,
    default_config: McpConfig,
}

impl McpToolManager {
    pub fn new(default_config: McpConfig) -> Self {
        Self {
            providers: HashMap::new(),
            default_config,
        }
    }

    pub async fn add_provider(&mut self, name: String, config: Option<McpConfig>) -> Result<()> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        
        let mut provider = McpToolProvider::new();
        provider.initialize(config).await?;
        
        self.providers.insert(name.clone(), provider);
        info!("Added MCP provider: {}", name);
        
        Ok(())
    }

    pub async fn remove_provider(&mut self, name: &str) -> Result<()> {
        if self.providers.remove(name).is_some() {
            info!("Removed MCP provider: {}", name);
            Ok(())
        } else {
            Err(anyhow!("MCP provider '{}' not found", name))
        }
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub async fn get_all_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>> {
        let mut all_tools = Vec::new();
        
        for provider in self.providers.values() {
            match provider.get_tools().await {
                Ok(tools) => all_tools.extend(tools),
                Err(e) => error!("Failed to get tools from MCP provider: {}", e),
            }
        }
        
        Ok(all_tools)
    }

    pub async fn refresh_all(&self) -> Result<()> {
        for (name, provider) in &self.providers {
            if let Err(e) = provider.refresh().await {
                error!("Failed to refresh MCP provider {}: {}", name, e);
            }
        }
        Ok(())
    }
}