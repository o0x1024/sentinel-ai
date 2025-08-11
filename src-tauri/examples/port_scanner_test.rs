//! 端口扫描器测试示例
//! 
//! 演示如何使用新的高性能端口扫描工具

use sentinel_ai_lib::tools::builtin::PortScanTool;
use sentinel_ai_lib::tools::{ToolExecutionParams, UnifiedTool};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("=== 端口扫描器测试 ===");
    
    // 创建端口扫描工具
    let port_scanner = PortScanTool::new();
    
    // 测试1: 扫描常用端口
    println!("\n测试1: 扫描本地常用端口");
    let mut inputs = HashMap::new();
    inputs.insert("target".to_string(), json!("127.0.0.1"));
    inputs.insert("ports".to_string(), json!("common"));
    inputs.insert("threads".to_string(), json!(50));
    inputs.insert("timeout".to_string(), json!(1));
    
    let params = ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: None,
        execution_id: Some(Uuid::new_v4()),
    };
    
    match port_scanner.execute(params).await {
        Ok(result) => {
            println!("扫描成功!");
            println!("执行时间: {}ms", result.execution_time_ms);
            if let Some(output) = result.output.as_object() {
                if let Some(open_count) = output.get("open_count") {
                    println!("发现开放端口数: {}", open_count);
                }
                if let Some(total_ports) = output.get("total_ports") {
                    println!("总扫描端口数: {}", total_ports);
                }
                if let Some(open_ports) = output.get("open_ports").and_then(|v| v.as_array()) {
                    println!("开放端口详情:");
                    for port in open_ports.iter().take(5) { // 只显示前5个
                        if let Some(port_obj) = port.as_object() {
                            let port_num = port_obj.get("port").and_then(|v| v.as_u64()).unwrap_or(0);
                            let service = port_obj.get("service").and_then(|v| v.as_str()).unwrap_or("Unknown");
                            let response_time = port_obj.get("response_time").and_then(|v| v.as_u64()).unwrap_or(0);
                            println!("  端口 {}: {} (响应时间: {}ms)", port_num, service, response_time);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("扫描失败: {}", e);
        }
    }
    
    // 测试2: 扫描指定端口范围
    println!("\n测试2: 扫描指定端口范围 (80-90)");
    let mut inputs = HashMap::new();
    inputs.insert("target".to_string(), json!("127.0.0.1"));
    inputs.insert("ports".to_string(), json!("80-90"));
    inputs.insert("threads".to_string(), json!(20));
    inputs.insert("timeout".to_string(), json!(1));
    
    let params = ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: None,
        execution_id: Some(Uuid::new_v4()),
    };
    
    match port_scanner.execute(params).await {
        Ok(result) => {
            println!("扫描成功!");
            println!("执行时间: {}ms", result.execution_time_ms);
            if let Some(output) = result.output.as_object() {
                if let Some(open_count) = output.get("open_count") {
                    println!("发现开放端口数: {}", open_count);
                }
            }
        }
        Err(e) => {
            println!("扫描失败: {}", e);
        }
    }
    
    // 测试3: 扫描特定端口列表
    println!("\n测试3: 扫描特定端口列表 (22,80,443,8080)");
    let mut inputs = HashMap::new();
    inputs.insert("target".to_string(), json!("127.0.0.1"));
    inputs.insert("ports".to_string(), json!("22,80,443,8080"));
    inputs.insert("threads".to_string(), json!(10));
    inputs.insert("timeout".to_string(), json!(2));
    
    let params = ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: None,
        execution_id: Some(Uuid::new_v4()),
    };
    
    match port_scanner.execute(params).await {
        Ok(result) => {
            println!("扫描成功!");
            println!("执行时间: {}ms", result.execution_time_ms);
            if let Some(output) = result.output.as_object() {
                if let Some(open_count) = output.get("open_count") {
                    println!("发现开放端口数: {}", open_count);
                }
            }
        }
        Err(e) => {
            println!("扫描失败: {}", e);
        }
    }
    
    println!("\n=== 测试完成 ===");
    Ok(())
}