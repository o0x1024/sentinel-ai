//! 意图分类相关命令

use crate::commands::intent_classifier::{IntentClassifier, IntentClassificationResult, UserIntent};
use crate::services::ai::{AiServiceManager, AiService};
use crate::ai_adapter::core::AiAdapterManager;
use crate::ai_adapter::types::{ChatRequest, Message, ChatOptions, MessageRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::RwLock;

/// 全局意图分类器
pub type GlobalIntentClassifier = Arc<RwLock<Option<IntentClassifier>>>;

/// 意图分类请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassificationRequest {
    /// 用户输入
    pub user_input: String,
    /// 可选的上下文信息
    pub context: Option<String>,
}

/// 聊天响应结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 响应内容
    pub content: String,
    /// 响应类型
    pub response_type: String,
    /// 是否需要后续处理
    pub needs_followup: bool,
}

/// 初始化意图分类器
#[command]
pub async fn initialize_intent_classifier(
    classifier: State<'_, GlobalIntentClassifier>,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    let mut classifier_guard = classifier.write().await;
    
    if classifier_guard.is_some() {
        return Ok("Intent classifier already initialized".to_string());
    }
    
    let intent_classifier = IntentClassifier::new(ai_service_manager.inner().clone());
    *classifier_guard = Some(intent_classifier);
    
    Ok("Intent classifier initialized successfully".to_string())
}

/// 分类用户意图
#[command]
pub async fn classify_user_intent(
    request: IntentClassificationRequest,
    classifier: State<'_, GlobalIntentClassifier>,
) -> Result<IntentClassificationResult, String> {
    let classifier_guard = classifier.read().await;
    
    let intent_classifier = match classifier_guard.as_ref() {
        Some(classifier) => classifier,
        None => return Err("Intent classifier not initialized".to_string())
    };
    
    intent_classifier
        .classify_intent(&request.user_input)
        .await
        .map_err(|e| format!("Failed to classify intent: {}", e))
}

/// 处理普通对话
#[command]
pub async fn handle_chat_conversation(
    user_input: String,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<ChatResponse, String> {
    // 生成友好的对话响应
    let response = generate_chat_response(&user_input, ai_service_manager.inner()).await
        .map_err(|e| format!("Failed to generate chat response: {}", e))?;
    
    Ok(ChatResponse {
        content: response,
        response_type: "chat".to_string(),
        needs_followup: false,
    })
}

/// 处理知识性问答
#[command]
pub async fn handle_knowledge_question(
    user_input: String,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<ChatResponse, String> {
    // 生成知识性回答
    let response = generate_knowledge_response(&user_input, ai_service_manager.inner()).await
        .map_err(|e| format!("Failed to generate knowledge response: {}", e))?;
    
    Ok(ChatResponse {
        content: response,
        response_type: "knowledge".to_string(),
        needs_followup: false,
    })
}

/// 降级到任何可用的服务
async fn stateless_send_with_service(service: &AiService, prompt: &str) -> anyhow::Result<String> {
    // 直接通过适配器发送，不保存到数据库
    let adapter_manager = AiAdapterManager::global();
    let config = service.get_config();
    let provider = adapter_manager.get_provider_or_default(&config.provider)
        .map_err(|e| anyhow::anyhow!("Provider not found: {}", e))?;

    let chat_req = ChatRequest {
        model: config.model.clone(),
        messages: vec![Message { role: MessageRole::User, content: prompt.to_string(), name: None, tool_calls: None, tool_call_id: None }],
        tools: None,
        tool_choice: None,
        user: None,
        extra_params: None,
        options: Some(ChatOptions { temperature: config.temperature, max_tokens: config.max_tokens, ..Default::default() }),
    };

    // 使用流式响应并收集结果
    let mut stream = provider.send_chat_stream(&chat_req).await
        .map_err(|e| anyhow::anyhow!("Chat request failed: {}", e))?;

    let mut content = String::new();

    // 收集流式响应
    use futures::StreamExt;
    while let Some(chunk_result) = stream.stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                content.push_str(&chunk.content);
            }
            Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
        }
    }
    
    Ok(content)
}

async fn fallback_to_any_service(
    ai_service_manager: &AiServiceManager,
    prompt: &str,
) -> anyhow::Result<String> {
    let services = ai_service_manager.list_services();
    for service_name in services {
        if let Some(service) = ai_service_manager.get_service(&service_name) {
            log::info!("尝试使用服务: {}", service_name);
            match stateless_send_with_service(&service, prompt).await {
                Ok(response) => {
                    log::info!("成功使用服务 {} 获得响应", service_name);
                    return Ok(response);
                }
                Err(e) => {
                    log::warn!("服务 {} 调用失败: {}", service_name, e);
                    continue;
                }
            }
        }
    }
    Err(anyhow::anyhow!("所有可用的AI服务都调用失败"))
}

/// 生成对话响应
async fn generate_chat_response(
    user_input: &str,
    ai_service_manager: &AiServiceManager,
) -> anyhow::Result<String> {
    let prompt = format!(r#"
你是一个友好的AI助手。用户和你进行日常对话。

用户说: "{}"

请给出自然、友好的回复。回复要：
1. 简洁明了（不超过100字）
2. 体现你是安全助手的身份
3. 适当引导用户了解你的能力
4. 语气亲切自然

直接返回回复内容，不要JSON格式。
"#, user_input);

    // 优先使用调度策略中配置的意图分析模型，带有降级策略
    match ai_service_manager.get_service_for_stage(crate::services::ai::SchedulerStage::IntentAnalysis).await {
        Ok(Some(service)) => {
            log::info!("使用调度策略配置的意图分析模型生成对话响应: {}", service.get_config().model);
            match stateless_send_with_service(&service, &prompt).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    log::warn!("调度器配置的意图分析模型调用失败: {}, 降级到其他服务", e);
                    fallback_to_any_service(ai_service_manager, &prompt).await
                }
            }
        }
        Ok(None) => {
            log::info!("调度策略中未配置意图分析模型，使用其他可用服务生成对话");
            fallback_to_any_service(ai_service_manager, &prompt).await
        }
        Err(e) => {
            log::warn!("获取调度策略意图分析模型失败: {}, 使用其他可用服务", e);
            fallback_to_any_service(ai_service_manager, &prompt).await
        }
    }
}

/// 生成知识性回答
async fn generate_knowledge_response(
    user_input: &str,
    ai_service_manager: &AiServiceManager,
) -> anyhow::Result<String> {
    let prompt = format!(r#"
你是一个专业的知识专家。用户向你询问相关的知识问题。

用户问题: "{}"

请提供准确、专业的回答：
1. 回答要准确、权威
2. 适当举例说明
3. 如果涉及实际操作，提醒安全和合法性
4. 长度控制在300字左右

直接返回回答内容，不要JSON格式。
"#, user_input);

    // 优先使用调度策略中配置的意图分析模型，带有降级策略
    match ai_service_manager.get_service_for_stage(crate::services::ai::SchedulerStage::IntentAnalysis).await {
        Ok(Some(service)) => {
            log::info!("使用调度策略配置的意图分析模型生成知识回答: {}", service.get_config().model);
            match stateless_send_with_service(&service, &prompt).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    log::warn!("调度器配置的意图分析模型调用失败: {}, 降级到其他服务", e);
                    fallback_to_any_service(ai_service_manager, &prompt).await
                }
            }
        }
        Ok(None) => {
            log::info!("调度策略中未配置意图分析模型，使用其他可用服务生成知识回答");
            fallback_to_any_service(ai_service_manager, &prompt).await
        }
        Err(e) => {
            log::warn!("获取调度策略意图分析模型失败: {}, 使用其他可用服务", e);
            fallback_to_any_service(ai_service_manager, &prompt).await
        }
    }
}

/// 智能路由用户请求（统一入口）
#[command]
pub async fn smart_route_user_request(
    user_input: String,
    classifier: State<'_, GlobalIntentClassifier>,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<serde_json::Value, String> {
    // 1. 分类用户意图
    let classification_request = IntentClassificationRequest {
        user_input: user_input.clone(),
        context: None,
    };
    
    let classification_result = classify_user_intent(classification_request, classifier).await?;
    
    // 2. 根据意图类型路由处理
    match classification_result.intent {
        UserIntent::Chat => {
            let chat_response = handle_chat_conversation(user_input, ai_service_manager).await?;
            Ok(serde_json::json!({
                "type": "chat",
                "classification": classification_result,
                "response": chat_response
            }))
        }
        UserIntent::Question => {
            let knowledge_response = handle_knowledge_question(user_input, ai_service_manager).await?;
            Ok(serde_json::json!({
                "type": "question",
                "classification": classification_result,
                "response": knowledge_response
            }))
        }
        UserIntent::Task => {
            // 返回需要Agent处理的标识，前端会调用原有的Agent执行流程
            Ok(serde_json::json!({
                "type": "task",
                "classification": classification_result,
                "needs_agent_execution": true,
                "extracted_info": classification_result.extracted_info
            }))
        }
    }
}

/// 获取意图分类统计信息
#[command]
pub async fn get_intent_classification_stats() -> Result<serde_json::Value, String> {
    // 这里可以实现统计功能，目前返回模拟数据
    Ok(serde_json::json!({
        "total_classifications": 0,
        "chat_percentage": 0.0,
        "question_percentage": 0.0,
        "task_percentage": 0.0,
        "accuracy_rate": 0.0
    }))
}
