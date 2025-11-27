//! Agent插件工具集成测试
//!
//! 测试AgentPluginProvider的功能

#[cfg(test)]
mod tests {
    use crate::commands::passive_scan_commands::PassiveScanState;
    use crate::tools::{AgentPluginProvider, ToolProvider};
    use sentinel_tools::unified_types::{ToolExecutionParams, UnifiedTool};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_agent_plugin_provider_discovery() {
        // 创建PassiveScanState
        let state = Arc::new(PassiveScanState::new());
        
        // 创建AgentPluginProvider
        let provider = AgentPluginProvider::new(state.clone());
        
        // 测试provider元信息
        assert_eq!(provider.name(), "agent_plugin");
        assert_eq!(provider.description(), "Agent plugin tools for AI-assisted security analysis");
        
        // 获取工具列表
        let tools_result = provider.get_tools().await;
        assert!(tools_result.is_ok(), "Failed to get tools: {:?}", tools_result.err());
        
        let tools = tools_result.unwrap();
        println!("AgentPluginProvider discovered {} tools", tools.len());
        
        // 打印每个工具的信息
        for tool in &tools {
            println!("Tool: {} - {}", tool.name(), tool.description());
            assert!(tool.name().starts_with("plugin::"), "Tool name should start with 'plugin::' prefix");
        }
    }

    #[tokio::test]
    async fn test_agent_plugin_tool_parameters() {
        let state = Arc::new(PassiveScanState::new());
        let provider = AgentPluginProvider::new(state.clone());
        
        let tools = provider.get_tools().await.unwrap();
        
        if tools.is_empty() {
            println!("No agents plugins found, skipping parameter test");
            return;
        }
        
        let tool = &tools[0];
        let params = tool.parameters();
        
        // 验证参数定义
        assert!(!params.parameters.is_empty(), "Tool should have parameters");
        
        // 检查是否有context、target、data参数
        let param_names: Vec<String> = params.parameters.iter()
            .map(|p| p.name.clone())
            .collect();
        
        println!("Tool parameters: {:?}", param_names);
        
        // 这些参数应该都是可选的
        for param in &params.parameters {
            assert!(!param.required, "All parameters should be optional for flexibility");
        }
    }

    #[tokio::test]
    async fn test_get_tool_by_name() {
        let state = Arc::new(PassiveScanState::new());
        let provider = AgentPluginProvider::new(state.clone());
        
        let tools = provider.get_tools().await.unwrap();
        
        if tools.is_empty() {
            println!("No agents plugins found, skipping name lookup test");
            return;
        }
        
        let tool_name = tools[0].name();
        println!("Looking up tool by name: {}", tool_name);
        
        // 测试通过完整名称查找（带plugin::前缀）
        let tool_result = provider.get_tool(tool_name).await;
        assert!(tool_result.is_ok(), "Failed to get tool: {:?}", tool_result.err());
        assert!(tool_result.unwrap().is_some(), "Tool should be found");
        
        // 测试通过插件ID查找（不带前缀）
        if let Some(plugin_id) = tool_name.strip_prefix("plugin::") {
            let tool_result = provider.get_tool(plugin_id).await;
            assert!(tool_result.is_ok());
            assert!(tool_result.unwrap().is_some(), "Tool should be found by plugin_id");
        }
    }
}
