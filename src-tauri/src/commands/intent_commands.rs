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
    log::info!("stateless_send_with_service 开始，服务: {}, prompt 长度: {}", service.get_config().model, prompt.len());
    log::debug!("prompt 内容: {}", prompt);
    
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
        options: Some(ChatOptions { 
            temperature: config.temperature, 
            max_tokens: config.max_tokens.or(Some(4096)), // 确保至少有 4096 tokens
            ..Default::default() 
        }),
    };

    log::info!("发送聊天请求，模型: {}, max_tokens: {:?}", chat_req.model, config.max_tokens);

    // 使用流式响应并收集结果
    let mut stream = provider.send_chat_stream(&chat_req).await
        .map_err(|e| anyhow::anyhow!("Chat request failed: {}", e))?;

    let mut content = String::new();
    let mut chunk_count = 0;

    // 收集流式响应
    use futures::StreamExt;
    while let Some(chunk_result) = stream.stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                chunk_count += 1;
                let chunk_len = chunk.content.len();
                content.push_str(&chunk.content);
                log::debug!("chunk {}: 内容长度 {}, 累计长度 {}, finish_reason: {:?}", 
                           chunk_count, chunk_len, content.len(), chunk.finish_reason);
                
                // 如果有结束原因，记录并继续处理其他可能的块
                if let Some(ref reason) = chunk.finish_reason {
                    log::info!("收到结束信号: {}", reason);
                }
            }
            Err(e) => {
                log::error!("Stream error: {}", e);
                return Err(anyhow::anyhow!("Stream error: {}", e));
            }
        }
    }
    
    log::info!("stateless_send_with_service 完成，收到 {} 个块，最终内容长度: {}", chunk_count, content.len());
    log::debug!("最终内容: {}", content);
    
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

    // 优先使用默认Chat模型
    match ai_service_manager.get_default_chat_model().await {
        Ok(Some((provider, model_name))) => {
            log::info!("使用默认Chat模型生成对话响应: {}/{}", provider, model_name);
            // 尝试根据model_name找到对应的服务，model_name已经包含完整的模型ID
            if let Ok(Some(service)) = ai_service_manager.find_service_by_model(&model_name).await {
                match stateless_send_with_service(&service, &prompt).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        log::warn!("默认Chat模型调用失败: {}, 降级到调度器配置", e);
                    }
                }
            } else {
                log::warn!("找不到支持默认Chat模型 {} 的服务", model_name);
            }
        }
        Ok(None) => {
            log::info!("未配置默认Chat模型，使用调度器配置或其他可用服务");
        }
        Err(e) => {
            log::warn!("获取默认Chat模型失败: {}, 使用调度器配置或其他可用服务", e);
        }
    }

    // 降级策略：使用调度器的意图分析模型
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
    log::info!("generate_knowledge_response 开始，用户输入: '{}'", user_input);
    
    let prompt = format!(r#"{}"#, user_input);
    
    log::debug!("生成的 prompt 长度: {}", prompt.len());
    log::debug!("prompt 内容: {}", prompt);

    // 优先使用默认Chat模型
    match ai_service_manager.get_default_chat_model().await {
        Ok(Some((provider, model_name))) => {
            log::info!("使用默认Chat模型生成知识回答: {}/{}", provider, model_name);
            // 尝试根据model_name找到对应的服务，model_name已经包含完整的模型ID
            if let Ok(Some(service)) = ai_service_manager.find_service_by_model(&model_name).await {
                match stateless_send_with_service(&service, &prompt).await {
                    Ok(response) => {
                        log::info!("generate_knowledge_response 成功，使用默认Chat模型，响应长度: {}", response.len());
                        log::debug!("generate_knowledge_response 最终响应: {}", response);
                        return Ok(response);
                    }
                    Err(e) => {
                        log::warn!("默认Chat模型调用失败: {}, 降级到调度器配置", e);
                    }
                }
            } else {
                log::warn!("找不到支持默认Chat模型 {} 的服务", model_name);
            }
        }
        Ok(None) => {
            log::info!("未配置默认Chat模型，使用调度器配置或其他可用服务");
        }
        Err(e) => {
            log::warn!("获取默认Chat模型失败: {}, 使用调度器配置或其他可用服务", e);
        }
    }

    // 降级策略：使用调度器的意图分析模型
    match ai_service_manager.get_service_for_stage(crate::services::ai::SchedulerStage::IntentAnalysis).await {
        Ok(Some(service)) => {
            log::info!("使用调度策略配置的意图分析模型生成知识回答: {}", service.get_config().model);
            match stateless_send_with_service(&service, &prompt).await {
                Ok(response) => {
                    log::info!("generate_knowledge_response 成功，使用调度策略模型，响应长度: {}", response.len());
                    log::debug!("generate_knowledge_response 最终响应: {}", response);
                    Ok(response)
                }
                Err(e) => {
                    log::warn!("调度器配置的意图分析模型调用失败: {}, 降级到其他服务", e);
                    fallback_to_any_service(ai_service_manager, &prompt).await
                }
            }
        }
        Ok(None) => {
            log::info!("调度策略中未配置意图分析模型，使用其他可用服务生成知识回答");
            let result = fallback_to_any_service(ai_service_manager, &prompt).await;
            if let Ok(ref response) = result {
                log::info!("generate_knowledge_response 成功，使用降级服务，响应长度: {}", response.len());
                log::debug!("generate_knowledge_response 最终响应: {}", response);
            }
            result
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
    _ai_service_manager: State<'_, Arc<AiServiceManager>>,
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
            // 对于聊天，返回分类信息，让前端发起流式请求
            Ok(serde_json::json!({
                "type": "chat",
                "classification": classification_result,
                "needs_streaming": true,
                "initial_response": "loading..."
            }))
        }
        UserIntent::Question => {
            // 对于问答，返回分类信息，让前端发起流式请求
            Ok(serde_json::json!({
                "type": "question",
                "classification": classification_result,
                "needs_streaming": true,
                "initial_response": "loading..."
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

/// 流式处理聊天对话
#[command]
pub async fn handle_chat_conversation_stream(
    user_input: String,
    conversation_id: String,
    message_id: String,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use crate::ai_adapter::core::AiAdapterManager;
    use crate::ai_adapter::types::{ChatRequest, Message, ChatOptions};
    use crate::models::ai::MessageRole;
    use futures::StreamExt;
    use tauri::Emitter;
    
    log::info!("handle_chat_conversation_stream 开始，用户输入: '{}'", user_input);
    
    let prompt = format!(r#"
你是一个友好的AI助手。用户和你进行日常对话。

用户说: "{}"

请给出自然、友好的回复。回复要：
1. 简洁明了（不超过200字）
2. 体现你是安全助手的身份
3. 适当引导用户了解你的能力
4. 语气亲切自然

直接返回回复内容，不要JSON格式。
"#, user_input);
    
    // 获取默认服务或适合的服务
    let service = match ai_service_manager.get_default_chat_model().await {
        Ok(Some((provider, model_name))) => {
            log::info!("使用默认Chat模型进行流式聊天: {}/{}", provider, model_name);
            ai_service_manager.find_service_by_model(&model_name).await
                .map_err(|e| format!("Failed to find service: {}", e))?
        }
        _ => {
            log::info!("使用降级服务进行流式聊天");
            None
        }
    };
    
    let service = service.ok_or_else(|| "No suitable AI service available for streaming chat".to_string())?;
    
    // 获取适配器和提供商
    let adapter_manager = AiAdapterManager::global();
    let config = service.get_config();
    let provider = adapter_manager.get_provider_or_default(&config.provider)
        .map_err(|e| format!("Provider not found: {}", e))?;

    let chat_req = ChatRequest {
        model: config.model.clone(),
        messages: vec![Message { role: MessageRole::User, content: prompt, name: None, tool_calls: None, tool_call_id: None }],
        tools: None,
        tool_choice: None,
        user: None,
        extra_params: None,
        options: Some(ChatOptions { 
            temperature: config.temperature, 
            max_tokens: config.max_tokens.or(Some(4096)),
            ..Default::default() 
        }),
    };

    // 在开始流式前通知前端
    if let Err(e) = app_handle.emit(
        "ai_stream_start",
        &serde_json::json!({
            "conversation_id": conversation_id,
            "message_id": message_id,
        }),
    ) {
        log::error!("Failed to emit stream start event: {}", e);
    }

    // 发起流式请求
    let mut stream = provider.send_chat_stream(&chat_req).await
        .map_err(|e| format!("Chat stream request failed: {}", e))?;

    let mut content = String::new();
    
    // 处理流式响应
    while let Some(chunk_result) = stream.stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                content.push_str(&chunk.content);
                
                // 发送增量流式消息到前端
                let stream_message = serde_json::json!({
                    "conversation_id": conversation_id,
                    "message_id": message_id,
                    "content": chunk.content,  // Send only the new chunk content
                    "is_complete": chunk.finish_reason.is_some(),
                    "is_incremental": true,   // Mark as incremental
                    "content_delta": chunk.content,  // The delta content
                    "total_content_length": content.len()  // Total accumulated length
                });
                
                if let Err(e) = app_handle.emit("ai_stream_message", &stream_message) {
                    log::error!("Failed to emit stream message: {}", e);
                }
                
                // 如果有结束原因，退出循环
                if chunk.finish_reason.is_some() {
                    log::info!("Chat stream completed with reason: {:?}", chunk.finish_reason);
                    break;
                }
            }
            Err(e) => {
                log::error!("Stream error: {}", e);
                // 发射错误事件，便于前端统一处理
                let _ = app_handle.emit(
                    "ai_stream_error",
                    &serde_json::json!({
                        "conversation_id": conversation_id,
                        "message_id": message_id,
                        "error": e.to_string(),
                    }),
                );
                return Err(format!("Stream error: {}", e));
            }
        }
    }
    // 通知前端完成（补充完整度事件）
    if let Err(e) = app_handle.emit(
        "ai_stream_complete",
        &serde_json::json!({
            "conversation_id": conversation_id,
            "message_id": message_id,
            "total_content_length": content.len(),
        }),
    ) {
        log::error!("Failed to emit stream complete event: {}", e);
    }

    log::info!("handle_chat_conversation_stream 完成，最终内容长度: {}", content.len());
    Ok(content)
}

/// 流式处理知识问答
#[command]
pub async fn handle_knowledge_question_stream(
    user_input: String,
    conversation_id: String,
    message_id: String,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use crate::ai_adapter::core::AiAdapterManager;
    use crate::ai_adapter::types::{ChatRequest, Message, ChatOptions};
    use crate::models::ai::MessageRole;
    use futures::StreamExt;
    use tauri::Emitter;
    
    log::info!("handle_knowledge_question_stream 开始，用户输入: '{}'", user_input);
    
    let prompt = format!(r#"
你是一个知识渊博、友好的AI助手。用户向你提出了一个知识性问题。

用户问题: "{}"

请提供准确、详细且有帮助的回答。回答要求：
1. 内容准确、全面，充分回答用户的问题
2. 语言清晰易懂，结构化表达
3. 如果是自我介绍类问题，请详细介绍你的身份、能力和用途
4. 保持友好、专业的语调
5. 如果合适，可以提供相关的例子或建议

直接返回回答内容，不要JSON格式。
"#, user_input);
    
    // 获取默认服务或适合的服务
    let service = match ai_service_manager.get_default_chat_model().await {
        Ok(Some((provider, model_name))) => {
            log::info!("使用默认Chat模型进行流式问答: {}/{}", provider, model_name);
            ai_service_manager.find_service_by_model(&model_name).await
                .map_err(|e| format!("Failed to find service: {}", e))?
        }
        _ => {
            log::info!("使用降级服务进行流式问答");
            None
        }
    };
    
    let service = service.ok_or_else(|| "No suitable AI service available for streaming knowledge question".to_string())?;
    
    // 获取适配器和提供商
    let adapter_manager = AiAdapterManager::global();
    let config = service.get_config();
    let provider = adapter_manager.get_provider_or_default(&config.provider)
        .map_err(|e| format!("Provider not found: {}", e))?;

    let chat_req = ChatRequest {
        model: config.model.clone(),
        messages: vec![Message { role: MessageRole::User, content: prompt, name: None, tool_calls: None, tool_call_id: None }],
        tools: None,
        tool_choice: None,
        user: None,
        extra_params: None,
        options: Some(ChatOptions { 
            temperature: config.temperature, 
            max_tokens: config.max_tokens.or(Some(4096)),
            ..Default::default() 
        }),
    };

    // 在开始流式前通知前端
    if let Err(e) = app_handle.emit(
        "ai_stream_start",
        &serde_json::json!({
            "conversation_id": conversation_id,
            "message_id": message_id,
        }),
    ) {
        log::error!("Failed to emit stream start event: {}", e);
    }

    // 发起流式请求
    let mut stream = provider.send_chat_stream(&chat_req).await
        .map_err(|e| format!("Knowledge question stream request failed: {}", e))?;

    let mut content = String::new();
    
    // 处理流式响应
    while let Some(chunk_result) = stream.stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                content.push_str(&chunk.content);
                
                // 发送增量流式消息到前端
                let stream_message = serde_json::json!({
                    "conversation_id": conversation_id,
                    "message_id": message_id,
                    "content": chunk.content,  // Send only the new chunk content
                    "is_complete": chunk.finish_reason.is_some(),
                    "is_incremental": true,   // Mark as incremental
                    "content_delta": chunk.content,  // The delta content
                    "total_content_length": content.len()  // Total accumulated length
                });
                
                if let Err(e) = app_handle.emit("ai_stream_message", &stream_message) {
                    log::error!("Failed to emit stream message: {}", e);
                }
                
                // 如果有结束原因，退出循环
                if chunk.finish_reason.is_some() {
                    log::info!("Knowledge question stream completed with reason: {:?}", chunk.finish_reason);
                    break;
                }
            }
            Err(e) => {
                log::error!("Stream error: {}", e);
                // 发射错误事件，便于前端统一处理
                let _ = app_handle.emit(
                    "ai_stream_error",
                    &serde_json::json!({
                        "conversation_id": conversation_id,
                        "message_id": message_id,
                        "error": e.to_string(),
                    }),
                );
                return Err(format!("Stream error: {}", e));
            }
        }
    }
    // 通知前端完成（补充完整度事件）
    if let Err(e) = app_handle.emit(
        "ai_stream_complete",
        &serde_json::json!({
            "conversation_id": conversation_id,
            "message_id": message_id,
            "total_content_length": content.len(),
        }),
    ) {
        log::error!("Failed to emit stream complete event: {}", e);
    }

    log::info!("handle_knowledge_question_stream 完成，最终内容长度: {}", content.len());
    Ok(content)
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
