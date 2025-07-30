use super::types::{
    ToolDefinition, 
    ToolInput, 
    CallToolError, 
    CallToolResult, 
    McpTool,
    McpToolInfo,
    ToolCategory,
    ToolParameters,
    ToolMetadata
};

use std::collections::HashMap;
use anyhow;
use uuid::Uuid;


/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn McpTool>>,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry
    }
    
    /// 注册工具
    pub fn register_tool(&mut self, tool: Box<dyn McpTool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }
    
    /// 获取工具
    pub fn get_tool(&self, name: &str) -> anyhow::Result<ToolDefinition> {
        self.tools.get(name)
            .map(|t| t.definition())
            .ok_or_else(|| anyhow::anyhow!("Tool does not exist: {}", name))
    }

    /// 检查工具是否存在
    pub fn tool_exists(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
    
    /// 列出所有工具名称
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 列出所有工具名称（包含详情）
    pub fn list_tools_with_details(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
    
    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
    
    /// 执行工具
    pub async fn execute_tool(&self, name: &str, args: serde_json::Value) -> CallToolResult {
        let tool = self.tools.get(name).ok_or_else(|| CallToolError::msg(format!("工具不存在: {}", name)))?;
        
        let input = ToolInput { arguments: args };
        tool.call(input).await
    }
    
    /// 获取工具详情
    pub fn get_tool_details(&self) -> anyhow::Result<Vec<McpToolInfo>> {
        let mut tools = Vec::new();
        
        for (name, tool) in &self.tools {
            let definition = tool.definition();
            
            // 提取所需参数
            let schema = definition.input_schema.clone();
            let mut required = Vec::new();
            let mut optional = Vec::new();
            
            if let Some(obj) = schema.as_object() {
                if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
                    for (prop_name, _) in props {
                        if let Some(req) = obj.get("required").and_then(|r| r.as_array()) {
                            if req.iter().any(|v| v.as_str() == Some(prop_name)) {
                                required.push(prop_name.clone());
                            } else {
                                optional.push(prop_name.clone());
                            }
                        } else {
                            // 如果没有required字段，则所有属性都是可选的
                            optional.push(prop_name.clone());
                        }
                    }
                }
            }
            
            tools.push(McpToolInfo {
                id: Uuid::new_v4().to_string(),
                name: name.clone(),
                description: definition.description.clone(),
                version: "1.0.0".to_string(),
                category: ToolCategory::Utility, // 默认分类
                parameters: ToolParameters {
                    schema,
                    required,
                    optional,
                },
                metadata: ToolMetadata {
                    author: "Sentinel AI".to_string(),
                    license: "MIT".to_string(),
                    homepage: None,
                    repository: None,
                    tags: Vec::new(),
                    install_command: None,
                    requirements: Vec::new(),
                },
            });
        }
        
        Ok(tools)
    }
    
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
} 