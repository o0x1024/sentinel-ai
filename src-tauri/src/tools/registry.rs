//! 工具注册系统
//!
//! 统一管理所有工具的注册、查询和执行

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Agent 工具类别（避免与 sentinel-tools 冲突）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolCategory {
    Reconnaissance,    // 信息收集
    Scanning,          // 扫描探测
    Exploitation,      // 漏洞利用
    PostExploitation,  // 后渗透
    CodeAnalysis,      // 代码分析
    BaselineCheck,     // 基线检查
    Remediation,       // 修复建议
    Utility,           // 通用工具
}

impl std::fmt::Display for AgentToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentToolCategory::Reconnaissance => write!(f, "Reconnaissance"),
            AgentToolCategory::Scanning => write!(f, "Scanning"),
            AgentToolCategory::Exploitation => write!(f, "Exploitation"),
            AgentToolCategory::PostExploitation => write!(f, "Post-Exploitation"),
            AgentToolCategory::CodeAnalysis => write!(f, "Code Analysis"),
            AgentToolCategory::BaselineCheck => write!(f, "Baseline Check"),
            AgentToolCategory::Remediation => write!(f, "Remediation"),
            AgentToolCategory::Utility => write!(f, "Utility"),
        }
    }
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// 工具返回值定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReturn {
    #[serde(rename = "type")]
    pub return_type: String,
    pub description: String,
}

/// 工具使用示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub args: serde_json::Value,
    pub expected_output: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: AgentToolCategory,
    pub parameters: Vec<ToolParameter>,
    pub returns: ToolReturn,
    #[serde(default)]
    pub examples: Vec<ToolExample>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_timeout() -> u64 {
    60
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, category: AgentToolCategory) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            parameters: vec![],
            returns: ToolReturn {
                return_type: "object".to_string(),
                description: "Tool execution result".to_string(),
            },
            examples: vec![],
            timeout_secs: 60,
            requires_confirmation: false,
            tags: vec![],
        }
    }

    pub fn with_parameter(mut self, param: ToolParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn requires_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Agent 工具信息（用于前端展示，避免与 sentinel-tools 冲突）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: Vec<ToolParameter>,
    pub requires_confirmation: bool,
    pub tags: Vec<String>,
}

impl From<&ToolDefinition> for AgentToolInfo {
    fn from(def: &ToolDefinition) -> Self {
        Self {
            name: def.name.clone(),
            description: def.description.clone(),
            category: def.category.to_string(),
            parameters: def.parameters.clone(),
            requires_confirmation: def.requires_confirmation,
            tags: def.tags.clone(),
        }
    }
}

/// 工具执行器 trait
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDefinition;
    
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value>;
    
    fn validate_args(&self, args: &serde_json::Value) -> Result<()> {
        let def = self.definition();
        for param in &def.parameters {
            if param.required && args.get(&param.name).is_none() {
                return Err(anyhow!("Missing required parameter: {}", param.name));
            }
        }
        Ok(())
    }
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    categories: HashMap<AgentToolCategory, Vec<String>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// 注册工具
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let def = tool.definition();
        let name = def.name.clone();
        let category = def.category.clone();
        
        self.tools.insert(name.clone(), Arc::new(tool));
        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(name);
    }

    /// 注册动态工具
    pub fn register_dynamic(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        let name = def.name.clone();
        let category = def.category.clone();
        
        self.tools.insert(name.clone(), tool);
        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(name);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// 检查工具是否存在
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// 列出所有工具名称
    pub fn list_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 列出所有工具信息
    pub fn list_all(&self) -> Vec<AgentToolInfo> {
        self.tools.values()
            .map(|t| AgentToolInfo::from(t.definition()))
            .collect()
    }

    /// 按类别列出工具
    pub fn list_by_category(&self, category: &AgentToolCategory) -> Vec<AgentToolInfo> {
        self.categories
            .get(category)
            .map(|names| {
                names.iter()
                    .filter_map(|name| self.tools.get(name))
                    .map(|t| AgentToolInfo::from(t.definition()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 搜索工具
    pub fn search(&self, query: &str) -> Vec<AgentToolInfo> {
        let query_lower = query.to_lowercase();
        self.tools.values()
            .filter(|t| {
                let def = t.definition();
                def.name.to_lowercase().contains(&query_lower) ||
                def.description.to_lowercase().contains(&query_lower) ||
                def.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|t| AgentToolInfo::from(t.definition()))
            .collect()
    }

    /// 执行工具
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let tool = self.get(name)
            .ok_or_else(|| anyhow!("Tool not found: {}", name))?;
        
        tool.validate_args(&args)?;
        tool.execute(args).await
    }

    /// 获取工具数量
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// 获取类别统计
    pub fn category_stats(&self) -> HashMap<String, usize> {
        self.categories.iter()
            .map(|(cat, tools)| (cat.to_string(), tools.len()))
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 参数构建器 ============

impl ToolParameter {
    pub fn string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    pub fn number(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "number".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    pub fn boolean(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "boolean".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default = Some(value);
        self.required = false;
        self
    }

    pub fn with_enum(mut self, values: Vec<&str>) -> Self {
        self.enum_values = Some(values.into_iter().map(String::from).collect());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool {
        definition: ToolDefinition,
    }

    #[async_trait::async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "status": "ok",
                "args": args
            }))
        }
    }

    #[test]
    fn test_tool_registration() {
        let mut registry = ToolRegistry::new();
        
        let tool = MockTool {
            definition: ToolDefinition::new("test_tool", "A test tool", AgentToolCategory::Utility),
        };
        
        registry.register(tool);
        
        assert!(registry.has("test_tool"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_tool_search() {
        let mut registry = ToolRegistry::new();
        
        let tool1 = MockTool {
            definition: ToolDefinition::new("port_scan", "Scan ports", AgentToolCategory::Scanning)
                .with_tag("network"),
        };
        let tool2 = MockTool {
            definition: ToolDefinition::new("whois", "WHOIS lookup", AgentToolCategory::Reconnaissance),
        };
        
        registry.register(tool1);
        registry.register(tool2);
        
        let results = registry.search("scan");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "port_scan");
    }

    #[test]
    fn test_parameter_builder() {
        let param = ToolParameter::string("target", "Target host")
            .optional()
            .with_default(serde_json::json!("localhost"));
        
        assert!(!param.required);
        assert!(param.default.is_some());
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let mut registry = ToolRegistry::new();
        
        let tool = MockTool {
            definition: ToolDefinition::new("test", "Test", AgentToolCategory::Utility)
                .with_parameter(ToolParameter::string("input", "Input value")),
        };
        
        registry.register(tool);
        
        let result = registry.execute("test", serde_json::json!({"input": "hello"})).await;
        assert!(result.is_ok());
    }
}

