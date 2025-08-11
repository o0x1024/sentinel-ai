//! AI模块集成示例
//! 
//! 本示例展示如何在Sentinel AI项目中集成和使用AI模块
//! 包括基础对话、流式响应、工具调用等功能

use anyhow::Result;
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// 导入AI相关模块
use crate::ai_adapter::{
    global_client,
    types::*,
    providers::ProviderFactory,
    error::AiAdapterError,
};
use crate::services::ai::{AiServiceManager, AiConfig};
use crate::services::database::Database;

/// AI集成示例主结构
pub struct AiIntegrationExample {
    ai_manager: AiServiceManager,
    conversation_id: String,
}

impl AiIntegrationExample {
    /// 创建新的AI集成示例实例
    pub async fn new(db: Arc<dyn Database + Send + Sync>) -> Result<Self> {
        let mut ai_manager = AiServiceManager::new(db);
        
        // 初始化默认服务
        ai_manager.init_default_services().await?;
        
        // 添加自定义DeepSeek服务
        let deepseek_config = AiConfig {
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            api_base: Some("https://api.deepseek.com".to_string()),
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(4000),
        };
        
        ai_manager.add_service("deepseek_custom".to_string(), deepseek_config).await?;
        
        Ok(Self {
            ai_manager,
            conversation_id: Uuid::new_v4().to_string(),
        })
    }
    
    /// 示例1: 基础对话功能
    pub async fn example_basic_chat(&self) -> Result<()> {
        println!("=== 示例1: 基础对话 ===");
        
        let service = self.ai_manager.get_service("deepseek_custom")
            .ok_or_else(|| anyhow::anyhow!("DeepSeek服务未找到"))?;
        
        let questions = vec![
            "你好，请介绍一下自己",
            "什么是Rust编程语言？",
            "Rust有哪些主要特点？",
        ];
        
        for question in questions {
            println!("\n用户: {}", question);
            
            match service.send_message(&self.conversation_id, question, None, None).await {
                Ok(response) => {
                    println!("AI: {}", response);
                }
                Err(e) => {
                    eprintln!("错误: {}", e);
                }
            }
            
            // 添加延迟避免请求过快
            sleep(Duration::from_millis(500)).await;
        }
        
        Ok(())
    }
    
    /// 示例2: 流式对话
    pub async fn example_stream_chat(&self) -> Result<()> {
        println!("\n=== 示例2: 流式对话 ===");
        
        let client = global_client();
        
        // 确保提供商已注册
        self.ensure_provider_registered().await?;
        
        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![
                Message::system("你是一个有用的AI助手，请用简洁明了的语言回答问题。"),
                Message::user("请详细解释什么是异步编程，并举例说明")
            ],
            tools: None,
            tool_choice: None,
            user: Some("example_user".to_string()),
            extra_params: None,
            options: Some(ChatOptions {
                stream: Some(true),
                temperature: Some(0.8),
                max_tokens: Some(2000),
                ..Default::default()
            }),
        };
        
        println!("\n用户: 请详细解释什么是异步编程，并举例说明");
        print!("AI: ");
        
        match client.chat_stream(Some("deepseek"), request).await {
            Ok(mut stream) => {
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(chunk) => {
                            if let Some(content) = chunk.delta {
                                print!("{}", content);
                                tokio::io::AsyncWriteExt::flush(&mut tokio::io::stdout()).await?;
                            }
                        }
                        Err(e) => {
                            eprintln!("\n流式响应错误: {}", e);
                            break;
                        }
                    }
                }
                println!();
            }
            Err(e) => {
                eprintln!("流式请求失败: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 示例3: 工具调用
    pub async fn example_tool_calling(&self) -> Result<()> {
        println!("\n=== 示例3: 工具调用 ===");
        
        let client = global_client();
        
        // 定义可用工具
        let tools = vec![
            Tool {
                name: "calculator".to_string(),
                description: "执行基本数学计算".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "要计算的数学表达式，如 '2 + 3 * 4'"
                        }
                    },
                    "required": ["expression"]
                }),
            },
            Tool {
                name: "get_time".to_string(),
                description: "获取当前时间".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "description": "时区，如 'UTC', 'Asia/Shanghai'",
                            "default": "UTC"
                        }
                    }
                }),
            },
            Tool {
                name: "search_info".to_string(),
                description: "搜索相关信息".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询"
                        }
                    },
                    "required": ["query"]
                }),
            }
        ];
        
        let test_questions = vec![
            "请帮我计算 (123 + 456) * 2",
            "现在几点了？",
            "搜索一下Rust语言的最新特性",
        ];
        
        for question in test_questions {
            println!("\n用户: {}", question);
            
            let request = ChatRequest {
                model: "deepseek-chat".to_string(),
                messages: vec![
                    Message::system("你是一个有用的AI助手。当需要执行计算、获取时间或搜索信息时，请使用相应的工具。"),
                    Message::user(question)
                ],
                tools: Some(tools.clone()),
                tool_choice: Some("auto".to_string()),
                user: Some("example_user".to_string()),
                extra_params: None,
                options: Some(ChatOptions::default()),
            };
            
            match client.chat(Some("deepseek"), request).await {
                Ok(response) => {
                    // 检查是否有工具调用
                    if let Some(tool_calls) = &response.message.tool_calls {
                        for tool_call in tool_calls {
                            println!("🔧 工具调用: {} - {}", tool_call.name, tool_call.arguments);
                            
                            // 执行工具调用
                            let tool_result = self.execute_tool(&tool_call.name, &tool_call.arguments).await?;
                            println!("🔧 工具结果: {}", tool_result);
                            
                            // 将工具结果发送回AI
                            let follow_up_messages = vec![
                                Message::system("你是一个有用的AI助手。"),
                                Message::user(question),
                                response.message.clone(),
                                Message::tool(&tool_result, &tool_call.id)
                            ];
                            
                            let follow_up_request = ChatRequest {
                                model: "deepseek-chat".to_string(),
                                messages: follow_up_messages,
                                tools: None,
                                tool_choice: None,
                                user: Some("example_user".to_string()),
                                extra_params: None,
                                options: Some(ChatOptions::default()),
                            };
                            
                            match client.chat(Some("deepseek"), follow_up_request).await {
                                Ok(final_response) => {
                                    if let MessageContent::Text(content) = &final_response.message.content {
                                        println!("AI: {}", content);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("工具调用后续请求失败: {}", e);
                                }
                            }
                        }
                    } else {
                        // 没有工具调用，直接显示回复
                        if let MessageContent::Text(content) = &response.message.content {
                            println!("AI: {}", content);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("请求失败: {}", e);
                }
            }
            
            sleep(Duration::from_millis(1000)).await;
        }
        
        Ok(())
    }
    
    /// 示例4: 错误处理和重试机制
    pub async fn example_error_handling(&self) -> Result<()> {
        println!("\n=== 示例4: 错误处理和重试 ===");
        
        let client = global_client();
        
        // 测试不同类型的错误
        let test_cases = vec![
            ("invalid_provider", "测试无效提供商"),
            ("deepseek", "测试正常请求"),
        ];
        
        for (provider, description) in test_cases {
            println!("\n测试: {}", description);
            
            let request = ChatRequest {
                model: "deepseek-chat".to_string(),
                messages: vec![Message::user("Hello")],
                tools: None,
                tool_choice: None,
                user: None,
                extra_params: None,
                options: Some(ChatOptions::default()),
            };
            
            // 带重试的请求
            match self.chat_with_retry(client, Some(provider), request, 3).await {
                Ok(response) => {
                    if let MessageContent::Text(content) = &response.message.content {
                        println!("✅ 成功: {}", content);
                    }
                }
                Err(e) => {
                    println!("❌ 失败: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// 示例5: 对话历史管理
    pub async fn example_conversation_history(&self) -> Result<()> {
        println!("\n=== 示例5: 对话历史管理 ===");
        
        let service = self.ai_manager.get_service("deepseek_custom")
            .ok_or_else(|| anyhow::anyhow!("DeepSeek服务未找到"))?;
        
        let conversation_id = Uuid::new_v4().to_string();
        
        // 多轮对话
        let conversation = vec![
            "我叫张三，我是一名程序员",
            "我喜欢使用Rust编程",
            "你还记得我的名字吗？",
            "我喜欢什么编程语言？",
        ];
        
        for message in conversation {
            println!("\n用户: {}", message);
            
            match service.send_message(&conversation_id, message, None, None).await {
                Ok(response) => {
                    println!("AI: {}", response);
                }
                Err(e) => {
                    eprintln!("错误: {}", e);
                }
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        // 获取对话历史
        match service.get_conversation_history(&conversation_id).await {
            Ok(history) => {
                println!("\n📚 对话历史:");
                for (i, msg) in history.iter().enumerate() {
                    println!("{}: {}", i + 1, msg);
                }
            }
            Err(e) => {
                eprintln!("获取对话历史失败: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 确保提供商已注册
    async fn ensure_provider_registered(&self) -> Result<()> {
        let client = global_client();
        
        // 检查提供商是否已存在
        if client.list_providers()?.contains(&"deepseek".to_string()) {
            return Ok(());
        }
        
        // 注册DeepSeek提供商
        let config = ProviderConfig {
            name: "deepseek".to_string(),
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            api_base: Some("https://api.deepseek.com".to_string()),
            api_version: None,
            timeout: None,
            max_retries: Some(3),
            extra_headers: None,
        };
        
        let provider = ProviderFactory::create(config)?;
        client.register_provider(provider)?;
        
        Ok(())
    }
    
    /// 执行工具调用
    async fn execute_tool(&self, tool_name: &str, arguments: &str) -> Result<String> {
        match tool_name {
            "calculator" => {
                let args: serde_json::Value = serde_json::from_str(arguments)?;
                let expression = args["expression"].as_str().unwrap_or("");
                Ok(self.calculate(expression))
            }
            "get_time" => {
                let args: serde_json::Value = serde_json::from_str(arguments)?;
                let timezone = args["timezone"].as_str().unwrap_or("UTC");
                Ok(self.get_current_time(timezone))
            }
            "search_info" => {
                let args: serde_json::Value = serde_json::from_str(arguments)?;
                let query = args["query"].as_str().unwrap_or("");
                Ok(self.search_information(query).await)
            }
            _ => Ok(format!("未知工具: {}", tool_name))
        }
    }
    
    /// 简单计算器
    fn calculate(&self, expression: &str) -> String {
        // 这里使用简单的字符串匹配，实际项目中可以使用表达式解析器
        match expression {
            expr if expr.contains("(123 + 456) * 2") => "1158".to_string(),
            expr if expr.contains("123 + 456") => "579".to_string(),
            expr if expr.contains("2 + 3 * 4") => "14".to_string(),
            _ => format!("无法计算表达式: {}", expression)
        }
    }
    
    /// 获取当前时间
    fn get_current_time(&self, timezone: &str) -> String {
        use chrono::{Utc, Local};
        
        match timezone {
            "UTC" => format!("UTC时间: {}", Utc::now().format("%Y-%m-%d %H:%M:%S")),
            "Asia/Shanghai" | "local" => format!("本地时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
            _ => format!("不支持的时区: {}", timezone)
        }
    }
    
    /// 模拟搜索信息
    async fn search_information(&self, query: &str) -> String {
        // 模拟搜索延迟
        sleep(Duration::from_millis(100)).await;
        
        match query.to_lowercase().as_str() {
            q if q.contains("rust") => {
                "Rust是一种系统编程语言，注重安全、速度和并发。最新特性包括异步编程改进、更好的错误处理等。".to_string()
            }
            q if q.contains("ai") || q.contains("人工智能") => {
                "人工智能是计算机科学的一个分支，致力于创建能够执行通常需要人类智能的任务的系统。".to_string()
            }
            _ => format!("搜索结果: 关于'{}'的信息暂时无法获取，请尝试其他关键词。", query)
        }
    }
    
    /// 带重试机制的聊天请求
    async fn chat_with_retry(
        &self,
        client: &'static crate::ai_adapter::core::AiClient,
        provider: Option<&str>,
        request: ChatRequest,
        max_retries: u32,
    ) -> Result<ChatResponse> {
        for attempt in 1..=max_retries {
            match client.chat(provider, request.clone()).await {
                Ok(response) => return Ok(response),
                Err(AiAdapterError::RateLimitError(_)) if attempt < max_retries => {
                    let delay = Duration::from_secs(2_u64.pow(attempt));
                    println!("⏳ 速率限制，等待 {:?} 后重试...", delay);
                    sleep(delay).await;
                    continue;
                }
                Err(AiAdapterError::NetworkError(_)) if attempt < max_retries => {
                    let delay = Duration::from_secs(attempt as u64);
                    println!("🌐 网络错误，等待 {:?} 后重试...", delay);
                    sleep(delay).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        Err(anyhow::anyhow!("重试次数已用完"))
    }
}

/// 运行所有示例
pub async fn run_all_examples(db: Arc<dyn Database + Send + Sync>) -> Result<()> {
    println!("🚀 开始运行AI模块集成示例...");
    
    let example = AiIntegrationExample::new(db).await?;
    
    // 运行各个示例
    example.example_basic_chat().await?;
    example.example_stream_chat().await?;
    example.example_tool_calling().await?;
    example.example_error_handling().await?;
    example.example_conversation_history().await?;
    
    println!("\n✅ 所有示例运行完成！");
    
    Ok(())
}

/// 主函数示例
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::init();
    
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 创建数据库实例（这里需要根据实际项目调整）
    // let db = create_database_instance().await?;
    
    // 运行示例
    // run_all_examples(db).await?;
    
    println!("请在实际项目中提供数据库实例后运行此示例");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ai_integration_example() {
        // 这里可以添加单元测试
        // 需要模拟数据库和API响应
    }
}