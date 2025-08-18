use crate::services::mcp::McpService;
use crate::tools::ToolSystem;
use serde_json::Value;
use tauri::State;
use std::sync::Arc;

/// 测试MCP工具注册和获取功能
#[tauri::command]
pub async fn test_mcp_tools_registration(
    mcp_service: State<'_, Arc<McpService>>,
    tool_system: State<'_, ToolSystem>,
    tool_id: Option<String>,
) -> Result<Value, String> {
    println!("🔧 开始测试MCP工具注册功能...");
    
    let mut results = serde_json::json!({
        "status": "success",
        "tests": []
    });
    
    let tests = results["tests"].as_array_mut().unwrap();
    
    // 测试1: 检查工具管理器中的工具
    let tools = tool_system.list_tools().await;
    let subdomain_tool_exists = tools.iter().any(|t| t.name == "subdomain_scanner");
    let port_tool_exists = tools.iter().any(|t| t.name == "port_scanner");
    
    tests.push(serde_json::json!({
        "name": "工具管理器工具检查",
        "subdomain_scanner_exists": subdomain_tool_exists,
        "port_scanner_exists": port_tool_exists,
        "status": if subdomain_tool_exists && port_tool_exists { "pass" } else { "fail" }
    }));
    
    // 测试2: 检查MCP服务中的可用工具
    match mcp_service.get_available_tools().await {
        Ok(available_tools) => {
            let tool_names: Vec<String> = available_tools.iter().map(|t| t.name.clone()).collect();
            let has_subdomain = tool_names.contains(&"subdomain_scanner".to_string());
            let has_port = tool_names.contains(&"port_scanner".to_string());
            
            tests.push(serde_json::json!({
                "name": "MCP服务工具检查",
                "available_tools": tool_names,
                "has_subdomain_scanner": has_subdomain,
                "has_port_scanner": has_port,
                "total_tools": tool_names.len(),
                "status": if has_subdomain && has_port { "pass" } else { "fail" }
            }));
            
            // 测试3: 检查工具定义
            for tool in &available_tools {
                if tool.name == "subdomain_scanner" || tool.name == "port_scanner" {
                    tests.push(serde_json::json!({
                        "name": format!("工具定义检查 - {}", tool.name),
                        "tool_name": tool.name,
                        "description": tool.description,
                        "has_schema": !tool.parameters.schema.is_null(),
                        "schema": tool.parameters.schema,
                        "status": "pass"
                    }));
                }
            }
        }
        Err(e) => {
            tests.push(serde_json::json!({
                "name": "MCP服务工具检查",
                "error": e.to_string(),
                "status": "fail"
            }));
        }
    }
    
    // 测试4: 测试工具执行（根据传入的tool_id）
    if let Some(tool_name) = tool_id {
        let (test_params, test_name) = match tool_name.as_str() {
            "subdomain_scanner" => (
                serde_json::json!({
                    "domain": "example.com",
                    "threads": 10,
                    "timeout": 5
                }),
                "子域名扫描器执行测试"
            ),
            "port_scanner" => (
                serde_json::json!({
                    "target": "127.0.0.1",
                    "ports": "80,443,8080",
                    "timeout": 5
                }),
                "端口扫描器执行测试"
            ),
            _ => {
                tests.push(serde_json::json!({
                    "name": format!("未知工具测试 - {}", tool_name),
                    "error": "不支持的工具类型",
                    "status": "fail"
                }));
                return Ok(results);
            }
        };
        
        match mcp_service.execute_tool(&tool_name, test_params.clone()).await {
            Ok(result) => {
                tests.push(serde_json::json!({
                    "name": test_name,
                    "tool_name": tool_name,
                    "parameters": test_params,
                    "result_type": if result.is_object() { "object" } else if result.is_array() { "array" } else { "other" },
                    "status": "pass"
                }));
            }
            Err(e) => {
                tests.push(serde_json::json!({
                    "name": test_name,
                    "tool_name": tool_name,
                    "parameters": test_params,
                    "error": e.to_string(),
                    "status": "fail"
                }));
            }
        }
    } else {
        // 如果没有指定tool_id，则测试所有工具
        let test_cases = vec![
            ("subdomain_scanner", serde_json::json!({
                "domain": "example.com",
                "threads": 10,
                "timeout": 5
            }), "子域名扫描器执行测试"),
            ("port_scanner", serde_json::json!({
                "target": "127.0.0.1",
                "ports": "80,443,8080,10809,7890",
                "timeout": 5
            }), "端口扫描器执行测试")
        ];
        
        for (tool_name, test_params, test_name) in test_cases {
            match mcp_service.execute_tool(tool_name, test_params.clone()).await {
                Ok(result) => {
                    tests.push(serde_json::json!({
                        "name": test_name,
                        "tool_name": tool_name,
                        "parameters": test_params,
                        "result_type": if result.is_object() { "object" } else if result.is_array() { "array" } else { "other" },
                        "status": "pass"
                    }));
                }
                Err(e) => {
                    tests.push(serde_json::json!({
                        "name": test_name,
                        "tool_name": tool_name,
                        "parameters": test_params,
                        "error": e.to_string(),
                        "status": "fail"
                    }));
                }
            }
        }
    }
    
    println!("✅ MCP工具注册测试完成");
    Ok(results)
}

/// 测试AI服务工具获取功能
#[tauri::command]
pub async fn test_ai_service_tools(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Value, String> {
    println!("🤖 开始测试AI服务工具获取功能...");
    
    let mut results = serde_json::json!({
        "status": "success",
        "ai_tools_simulation": []
    });
    
    // 模拟AI服务获取工具的过程
    match mcp_service.get_available_tools().await {
        Ok(available_tools) => {
            let mut ai_tools = Vec::new();
            
            for tool in available_tools {
                // 模拟转换为AI function calling格式
                let ai_tool = serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "schema": tool.parameters.schema,
                    "function_calling_ready": true
                });
                ai_tools.push(ai_tool);
            }
            
            results["ai_tools_simulation"] = serde_json::Value::Array(ai_tools);
            results["total_tools_for_ai"] = serde_json::Value::Number(serde_json::Number::from(results["ai_tools_simulation"].as_array().unwrap().len()));
        }
        Err(e) => {
            results["error"] = serde_json::Value::String(e.to_string());
            results["status"] = serde_json::Value::String("fail".to_string());
        }
    }
    
    println!("✅ AI服务工具获取测试完成");
    Ok(results)
}

/// 获取当前MCP工具状态
#[tauri::command]
pub async fn get_mcp_tools_status(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Value, String> {
    let mut status = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "mcp_service_available": true,
        "tools": []
    });
    
    match mcp_service.get_available_tools().await {
        Ok(tools) => {
            let tools_info: Vec<Value> = tools.iter().map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "category": format!("{:?}", tool.category),
                    "version": tool.version,
                    "author": tool.metadata.author,
                    "has_schema": !tool.parameters.schema.is_null()
                })
            }).collect();
            
            status["tools"] = serde_json::Value::Array(tools_info);
            status["total_tools"] = serde_json::Value::Number(serde_json::Number::from(tools.len()));
        }
        Err(e) => {
            status["error"] = serde_json::Value::String(e.to_string());
            status["mcp_service_available"] = serde_json::Value::Bool(false);
        }
    }
    
    Ok(status)
}