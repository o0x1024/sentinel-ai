//! Tool-enabled execution path.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use sentinel_db::Database;
use sentinel_llm::{ChatMessage, StreamContent, StreamingLlmClient, parse_image_from_json};
use sentinel_memory::{get_global_memory, ExecutionRecord, ToolCallSummary};
use sentinel_tools::ToolServer;

use crate::agents::{append_tool_digest, build_context, build_tool_digest, ContextBuildInput};
use crate::agents::executor::message_store::save_assistant_message;
use crate::agents::executor::types::ToolCallRecord;
use crate::agents::executor::utils::{cleanup_container_context_async, truncate_for_memory};
use crate::agents::tenth_man::{InterventionContext, InterventionMode, TenthMan, TriggerReason};
use crate::agents::tool_router::ToolRouter;
use super::AgentExecuteParams;

pub async fn execute_agent_with_tools(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
    tool_server: &ToolServer,
) -> Result<String> {
    let tool_config = params.tool_config.clone().unwrap_or_default();

    // 1. 创建工具路由器（加载所有动态工具：工作流、MCP、插件）
    use tauri::Manager;
    let db_service = app_handle.state::<std::sync::Arc<sentinel_db::DatabaseService>>();

    let tool_router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;

    // 2. 工具选择（传入 LLM 配置用于智能选择）
    let rig_provider = params.rig_provider.to_lowercase();
    let mut llm_config = sentinel_llm::LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider);

    if let Some(ref api_key) = params.api_key {
        llm_config = llm_config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        llm_config = llm_config.with_base_url(api_base);
    }

    // Use plan_tools to support Ability mode with injected context
    let db_pool = db_service.get_pool().ok();
    let selection_plan = tool_router
        .plan_tools(&params.task, &tool_config, Some(&llm_config), db_pool)
        .await?;

    let selected_tool_ids = selection_plan.tool_ids.clone();
    
    tracing::info!(
        "Selected {} tools for execution_id {}: {:?}",
        selected_tool_ids.len(),
        params.execution_id,
        selected_tool_ids
    );

    // Emit ability_selected event if applicable
    if let Some(ref group) = selection_plan.selected_ability_group {
        let _ = app_handle.emit(
            "agent:ability_selected",
            &json!({
                "execution_id": params.execution_id,
                "group_id": group.id,
                "group_name": group.name,
            }),
        );
    }

    // 发送工具选择事件到前端
    let _ = app_handle.emit(
        "agent:tools_selected",
        &json!({
            "execution_id": params.execution_id,
            "tools": selected_tool_ids,
        }),
    );

    // 3. 获取 DynamicTool 实例（用于 rig-core 原生工具调用）
    let dynamic_tools = tool_server.get_dynamic_tools(&selected_tool_ids).await;

    tracing::info!(
        "Got {} dynamic tool instances for rig-core native tool calling",
        dynamic_tools.len()
    );

    // 4. Build context via Context Engineering
    let context_policy = params.context_policy.clone().unwrap_or_default();
    let context_result = build_context(ContextBuildInput {
        app_handle: app_handle.clone(),
        execution_id: params.execution_id.clone(),
        base_system_prompt: params.system_prompt.clone(),
        injected_ability_prompt: selection_plan.injected_system_prompt.clone(),
        task: params.task.clone(),
        rig_provider: rig_provider.clone(),
        llm_config: llm_config.clone(),
        selected_tool_ids: selected_tool_ids.clone(),
        document_attachments: params.document_attachments.clone(),
        policy: context_policy.clone(),
    })
    .await?;

    let final_system_prompt_content = Some(context_result.system_prompt);
    let mut history_chat_messages = context_result.history_messages;

    // 移除历史记录中最后一条用户消息，避免与当前任务重复发送
    // 因为 stream_chat_with_dynamic_tools 会自动将 user_prompt 添加到对话末尾
    if let Some(last) = history_chat_messages.last() {
        if last.role == "user" {
            history_chat_messages.pop();
        }
    }

    // 解析图片附件
    let image_attachment = parse_image_from_json(params.image_attachments.as_ref());
    
    // 6. 使用 rig-core 原生工具调用
    // rig 的 multi_turn() 会自动处理工具调用循环
    let client = StreamingLlmClient::new(llm_config);
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();
    let db_for_stream: Option<std::sync::Arc<sentinel_db::DatabaseService>> = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone());

    // 用于收集工具调用信息
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<std::collections::HashMap<String, (String, String, i64, u32)>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let tool_seq: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
    let tool_call_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let assistant_segment_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let reasoning_content_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let context_policy_for_stream = context_policy.clone();
    
    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();
    let seq_counter = tool_seq.clone();
    let tool_counter = tool_call_counter.clone();
    let segment_buf = assistant_segment_buf.clone();
    let reasoning_buf = reasoning_content_buf.clone();

    // 7. 调用带动态工具的流式方法，增加重试机制以应对模型抖动或解析错误
    let mut retries = 0;
    let max_retries = 2; // 最多重试 2 次
    let mut last_error: Option<anyhow::Error> = None;
    
    // 累积的工具调用记录（跨重试保留）
    let accumulated_tool_calls: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    // 累积的助手输出（跨重试保留）
    let accumulated_assistant_output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let base_history_messages = history_chat_messages.clone();
    let build_retry_history = |attempt: u32| -> Vec<ChatMessage> {
        let mut history = base_history_messages.clone();
        if attempt == 0 {
            return history;
        }

        let tool_calls_snapshot = accumulated_tool_calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default();
        let output_snapshot = accumulated_assistant_output
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();

        let mut unique_calls = std::collections::HashMap::new();
        for call in tool_calls_snapshot {
            unique_calls.entry(call.id.clone()).or_insert(call);
        }
        let mut ordered_calls = unique_calls.into_values().collect::<Vec<_>>();
        ordered_calls.sort_by_key(|c| c.sequence);

        if !ordered_calls.is_empty() {
            let tool_calls_json = serde_json::to_string(
                &ordered_calls
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "type": "function",
                            "function": {
                                "name": c.name,
                                "arguments": c.arguments,
                            }
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default();

            let mut tool_calls_msg = ChatMessage::assistant("");
            tool_calls_msg.tool_calls = Some(tool_calls_json);
            tool_calls_msg.reasoning_content = Some(String::new());
            history.push(tool_calls_msg);

            for call in ordered_calls.iter() {
                if let Some(result) = &call.result {
                    history.push(ChatMessage::tool(result.clone(), call.id.clone()));
                }
            }
        }

        if !output_snapshot.trim().is_empty() {
            history.push(ChatMessage::assistant(output_snapshot));
        }

        history
    };

    while retries <= max_retries {
        if retries > 0 {
            // 保存当前已完成的工具调用到累积记录
            if let Ok(current_calls) = tool_calls_collector.lock() {
                if let Ok(mut acc) = accumulated_tool_calls.lock() {
                    acc.extend(current_calls.clone());
                }
            }
            
            // 保存当前已输出的内容到累积输出
            if let Ok(current_output) = assistant_segment_buf.lock() {
                if !current_output.is_empty() {
                    if let Ok(mut acc) = accumulated_assistant_output.lock() {
                        if !acc.is_empty() {
                            acc.push_str("\n\n");
                        }
                        acc.push_str(current_output.as_str());
                    }
                }
            }
            
            tracing::warn!(
                "Retrying agent execution (attempt {}/{}) due to error: {}. Accumulated {} tool calls and {} chars output.",
                retries,
                max_retries,
                last_error.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0)
            );

            // 发送重试事件给前端（包含已完成的进度信息）
            let _ = app_handle.emit(
                "agent:retry",
                &json!({
                    "execution_id": params.execution_id,
                    "retry_count": retries,
                    "max_retries": max_retries,
                    "error": last_error.as_ref().map(|e| e.to_string()),
                    "accumulated_progress": {
                        "tool_calls": accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                        "output_chars": accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0),
                    }
                }),
            );

            // 重试前稍作延迟
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        let history_for_retry = build_retry_history(retries);
        let result = client
            .stream_chat_with_dynamic_tools(
                final_system_prompt_content.as_deref(),
                &params.task,
                &history_for_retry,
                image_attachment.as_ref(), // 传递图片附件
                dynamic_tools.clone(),
                |content| {
                    if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                        return false;
                    }
                    match content {
                        StreamContent::Text(text) => {
                            // Accumulate assistant text into a segment buffer.
                            let _ = segment_buf.lock().map(|mut buf| buf.push_str(&text));

                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "text",
                                    "content": text,
                                }),
                            );
                        }
                        StreamContent::Reasoning(reasoning) => {
                            // Accumulate reasoning content
                            if let Ok(mut buf) = reasoning_buf.lock() {
                                buf.push_str(&reasoning);
                            }
                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "reasoning",
                                    "content": reasoning,
                                }),
                            );
                        }
                        StreamContent::ToolCallStart { id, name } => {
                            tracing::debug!("Tool call started via rig-core: {} ({})", name, id);
                            
                            // Increment tool call counter
                            tool_counter.fetch_add(1, Ordering::SeqCst);
                            
                            // Tenth Man Intervention Point 1: Before Tool Execution
                            if params.enable_tenth_man_rule {
                                if let Some(ref tm_config) = params.tenth_man_config {
                                    let tenth_man = TenthMan::new(&params);
                                    let current_count = tool_counter.load(Ordering::SeqCst) as usize;
                                    
                                    let context = InterventionContext {
                                        execution_id: execution_id.clone(),
                                        task: params.task.clone(),
                                        tool_call_count: current_count,
                                        current_content: Some(format!("Preparing to call tool: {}", name)),
                                        trigger_reason: TriggerReason::ToolCallThreshold,
                                    };
                                    
                                    if tenth_man.should_trigger(&context) {
                                        let app_clone = app.clone();
                                        let exec_id = execution_id.clone();
                                        let tool_name = name.clone();
                                        let require_confirmation = tm_config.require_user_confirmation;
                                        
                                        tauri::async_runtime::spawn(async move {
                                            match tenth_man.quick_review(&context).await {
                                                Ok(Some(critique)) => {
                                                    tracing::info!("Tenth Man warning before tool call: {}", tool_name);
                                                    let _ = app_clone.emit(
                                                        "agent:tenth_man_warning",
                                                        &json!({
                                                            "execution_id": exec_id,
                                                            "trigger": "before_tool_call",
                                                            "tool_name": tool_name,
                                                            "critique": critique,
                                                            "requires_confirmation": require_confirmation,
                                                        })
                                                    );
                                                }
                                                Ok(None) => {
                                                    tracing::debug!("Tenth Man: No significant risk detected");
                                                }
                                                Err(e) => {
                                                    tracing::warn!("Tenth Man quick review failed: {}", e);
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            
                            let _ = app.emit(
                                "agent:tool_call_start",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                }),
                            );
                        }
                        StreamContent::ToolCallDelta { id, delta } => {
                            let _ = app.emit(
                                "agent:tool_call_delta",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "delta": delta,
                                }),
                            );
                        }
                        StreamContent::ToolCallComplete {
                            id,
                            name,
                            arguments,
                        } => {
                            tracing::debug!("Tool call complete via rig-core: {} ({})", name, id);

                            // 记录 pending 的工具调用，等待结果
                            if let Ok(mut pending_map) = pending.lock() {
                                let seq = seq_counter.fetch_add(1, Ordering::Relaxed);
                                let started_at_ms = chrono::Utc::now().timestamp_millis() + seq as i64;
                                pending_map.insert(
                                    id.clone(),
                                    (name.clone(), arguments.clone(), started_at_ms, seq),
                                );
                            }

                            // Flush assistant segment BEFORE inserting tool call message (preserve ordering on reload).
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let seg = segment_buf
                                    .lock()
                                    .map(|mut g| std::mem::take(&mut *g))
                                    .unwrap_or_default();
                                let seg_trimmed = seg.trim().to_string();
                                if !seg_trimmed.trim().is_empty() {
                                    // Ensure segment timestamp is slightly before tool call timestamp.
                                    let seg_ts_ms = chrono::Utc::now().timestamp_millis() - 1;
                                    let seg_ts = chrono::Utc
                                        .timestamp_millis_opt(seg_ts_ms)
                                        .single()
                                        .unwrap_or_else(chrono::Utc::now);

                                    // Get reasoning content (for deepseek-reasoner with tool calls, always include it)
                                    // 参考：https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls
                                    let reasoning = reasoning_buf
                                        .lock()
                                        .map(|g| {
                                            let r = g.clone();
                                            // 即使为空也返回 Some("")，因为 deepseek-reasoner 要求必须有此字段
                                            Some(if r.trim().is_empty() {
                                                String::new()
                                            } else {
                                                r
                                            })
                                        })
                                        .ok()
                                        .flatten();

                                    let seg_msg = core_db::AiMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        conversation_id: execution_id.clone(),
                                        role: "assistant".to_string(),
                                        content: seg_trimmed.clone(),
                                        metadata: None,
                                        token_count: Some(seg_trimmed.len() as i32),
                                        cost: None,
                                        tool_calls: None,
                                        attachments: None,
                                        reasoning_content: reasoning,
                                        timestamp: seg_ts,
                                        architecture_type: None,
                                        architecture_meta: None,
                                        structured_data: None,
                                    };
                                    tauri::async_runtime::spawn(async move {
                                        if let Err(e) = db.upsert_ai_message_append(&seg_msg).await {
                                            tracing::warn!(
                                                "Failed to persist assistant segment: {}",
                                                e
                                            );
                                        }
                                    });
                                }
                            }

                            // Persist tool call as a standalone message (role=tool) so history ordering is correct.
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let (started_at_ms, seq) = pending
                                    .lock()
                                    .ok()
                                    .and_then(|m| m.get(&id).map(|(_, _, ms, s)| (*ms, *s)))
                                    .unwrap_or((chrono::Utc::now().timestamp_millis(), 0));

                                let started_at = chrono::Utc
                                    .timestamp_millis_opt(started_at_ms)
                                    .single()
                                    .unwrap_or_else(chrono::Utc::now);

                                let tool_args_val: serde_json::Value =
                                    serde_json::from_str(&arguments)
                                        .unwrap_or_else(|_| json!({ "raw": arguments }));
                                let meta = json!({
                                    "kind": "tool_call",
                                    "tool_name": name,
                                    "tool_args": tool_args_val,
                                    "tool_call_id": id,
                                    "status": "running",
                                    "sequence": seq,
                                    "started_at_ms": started_at_ms,
                                });

                                let tool_msg = core_db::AiMessage {
                                    id: id.clone(),
                                    conversation_id: execution_id.clone(),
                                    role: "tool".to_string(),
                                    content: String::new(),
                                    metadata: Some(meta.to_string()),
                                    token_count: None,
                                    cost: None,
                                    tool_calls: None,
                                    attachments: None,
                                    reasoning_content: None,
                                    timestamp: started_at,
                                    architecture_type: None,
                                    architecture_meta: None,
                                    structured_data: None,
                                };
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = db.upsert_ai_message_append(&tool_msg).await {
                                        tracing::warn!("Failed to persist tool call message: {}", e);
                                    }
                                });
                            }

                            let _ = app.emit(
                                "agent:tool_call_complete",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                    "arguments": arguments,
                                }),
                            );
                        }
                        StreamContent::ToolResult { id, result } => {
                            // tracing::info!(
                            //     "Tool result via rig-core: id={}, result_preview={}",
                            //     id,
                            //     &result.chars().take(500).collect::<String>()
                            // );

                            // 将工具调用完整信息添加到收集器
                            if let Ok(mut pending_map) = pending.lock() {
                                if let Some((name, arguments, started_at_ms, seq)) =
                                    pending_map.remove(&id)
                                {
                                    let completed_at_ms = chrono::Utc::now().timestamp_millis();
                                    let duration_ms = completed_at_ms.saturating_sub(started_at_ms);
                                    let name_for_meta = name.clone();
                                    let args_for_meta = arguments.clone();
                                    if let Ok(mut records) = collector.lock() {
                                        records.push(ToolCallRecord {
                                            id: id.clone(),
                                            name,
                                            arguments,
                                            result: Some(result.clone()),
                                            success: !result.to_lowercase().contains("error"),
                                            sequence: seq,
                                            started_at_ms,
                                            completed_at_ms,
                                            duration_ms,
                                        });
                                    }

                                    // Update persisted tool message with result (keep timestamp as started_at to avoid reordering).
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        use chrono::TimeZone;
                                        let started_at = chrono::Utc
                                            .timestamp_millis_opt(started_at_ms)
                                            .single()
                                            .unwrap_or_else(chrono::Utc::now);

                                        let tool_args_val: serde_json::Value =
                                            serde_json::from_str(&args_for_meta)
                                                .unwrap_or_else(|_| json!({ "raw": args_for_meta }));
                                        let meta = json!({
                                            "kind": "tool_call",
                                            "tool_name": name_for_meta,
                                            "tool_args": tool_args_val,
                                            "tool_call_id": id,
                                            "status": "completed",
                                            "sequence": seq,
                                            "started_at_ms": started_at_ms,
                                            "completed_at_ms": completed_at_ms,
                                            "duration_ms": duration_ms,
                                            "tool_result": result,
                                            "success": !result.to_lowercase().contains("error"),
                                        });
                                        let tool_msg = core_db::AiMessage {
                                            id: id.clone(),
                                            conversation_id: execution_id.clone(),
                                            role: "tool".to_string(),
                                            content: String::new(),
                                            metadata: Some(meta.to_string()),
                                            token_count: None,
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: started_at,
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };
                                        tauri::async_runtime::spawn(async move {
                                            if let Err(e) =
                                                db.upsert_ai_message_append(&tool_msg).await
                                            {
                                                tracing::warn!(
                                                    "Failed to persist tool result update: {}",
                                                    e
                                                );
                                            }
                                        });
                                    }

                                    // Update run state with tool digest for resumption context
                                    let app_for_digest = app.clone();
                                    let exec_for_digest = execution_id.clone();
                                    let policy_for_digest = context_policy_for_stream.clone();
                                    let args_value: serde_json::Value = serde_json::from_str(&args_for_meta)
                                        .unwrap_or_else(|_| json!({ "raw": args_for_meta }));
                                    let digest = build_tool_digest(&name_for_meta, &args_value, &result);
                                    tauri::async_runtime::spawn(async move {
                                        if let Err(e) =
                                            append_tool_digest(&app_for_digest, &exec_for_digest, digest, &policy_for_digest)
                                                .await
                                        {
                                            tracing::warn!("Failed to append tool digest: {}", e);
                                        }
                                    });
                                }
                            }

                            let _ = app.emit(
                                "agent:tool_result",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "result": result,
                                }),
                            );
                        }
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    tracing::info!(
                        "Token usage report - execution_id: {}, input: {}, output: {}, total: {}",
                        execution_id,
                        input_tokens,
                        output_tokens,
                        input_tokens + output_tokens
                    );
                    let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "usage",
                                    "input_tokens": input_tokens,
                                    "output_tokens": output_tokens,
                                }),
                            );

                            // 记录 token 使用统计到数据库
                            if input_tokens > 0 || output_tokens > 0 {
                                use tauri::Manager;
                                if let Some(db) =
                                    app.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                                {
                                    let provider = params.rig_provider.clone();
                                    let model = params.model.clone();
                                    let cost = sentinel_llm::calculate_cost(
                                        &provider,
                                        &model,
                                        input_tokens,
                                        output_tokens,
                                    );

                                    let db_clone = db.inner().clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = db_clone
                                            .update_ai_usage(
                                                &provider,
                                                &model,
                                                input_tokens as i32,
                                                output_tokens as i32,
                                                cost,
                                            )
                                            .await
                                        {
                                            tracing::warn!("Failed to update AI usage stats: {}", e);
                                        } else {
                                            tracing::info!(
                                                "Updated AI usage: provider={}, model={}, input={}, output={}, cost=${:.4}",
                                                provider, model, input_tokens, output_tokens, cost
                                            );
                                        }
                                    });
                                }
                            }
                        }
                        StreamContent::Done => {
                            tracing::info!("Stream completed - execution_id: {}", execution_id);
                            // Always clear the segment buffer. Persisting the final segment here will duplicate the
                            // assistant message because we also persist the final response in `save_assistant_message`.
                            // The final response includes tool_calls metadata and is the canonical persisted message.
                            let _ = segment_buf
                                .lock()
                                .map(|mut g| std::mem::take(&mut *g))
                                .unwrap_or_default();
                        }
                    }
                    true
                },
            )
            .await;

        match result {
            Ok(response) => {
                // 合并最终输出和累积的输出
                let final_response = if let Ok(acc) = accumulated_assistant_output.lock() {
                    if !acc.is_empty() && !response.is_empty() {
                        format!("{}\n\n{}", acc, response)
                    } else if !acc.is_empty() {
                        acc.clone()
                    } else {
                        response.clone()
                    }
                } else {
                    response.clone()
                };
                
                tracing::info!(
                    "Agent with tools completed - execution_id: {}, response_length: {}, accumulated_length: {}",
                    params.execution_id,
                    final_response.len(),
                    accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0)
                );

                // 合并所有工具调用记录（包括累积的和当前的）
                let mut all_tool_calls = Vec::new();
                if let Ok(acc_calls) = accumulated_tool_calls.lock() {
                    all_tool_calls.extend(acc_calls.clone());
                }
                if let Ok(current_calls) = tool_calls_collector.lock() {
                    all_tool_calls.extend(current_calls.clone());
                }

                let tool_summaries = all_tool_calls
                    .iter()
                    .map(|call| ToolCallSummary {
                        name: call.name.clone(),
                        success: call.success,
                        duration_ms: Some(call.duration_ms),
                    })
                    .collect::<Vec<_>>();
                    
                tracing::info!(
                    "Total tool calls completed: {} (accumulated: {}, current: {})",
                    tool_summaries.len(),
                    accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                    tool_calls_collector.lock().map(|c| c.len()).unwrap_or(0)
                );

                if let Err(e) = get_global_memory()
                    .record_execution(ExecutionRecord {
                        id: params.execution_id.clone(),
                        task: params.task.clone(),
                        environment: Some(rig_provider.clone()),
                        tool_calls: tool_summaries,
                        success: true,
                        error: None,
                        response_excerpt: Some(truncate_for_memory(&response, 400)),
                        created_at: chrono::Utc::now().timestamp(),
                    })
                    .await
                {
                    tracing::warn!("Failed to store memory record: {}", e);
                }

                let reasoning_content = reasoning_content_buf
                    .lock()
                    .ok()
                    .map(|s| s.clone())
                    .filter(|s| !s.trim().is_empty());

                let tool_calls_slice = if all_tool_calls.is_empty() {
                    None
                } else {
                    Some(all_tool_calls.as_slice())
                };

                save_assistant_message(
                    app_handle,
                    &params.execution_id,
                    &final_response,
                    tool_calls_slice,
                    reasoning_content,
                    params.persist_messages,
                    params.subagent_run_id.as_deref(),
                )
                .await;

                // Tenth Man Rule: Adversarial Review (System-enforced final check)
                if params.enable_tenth_man_rule {
                    let tenth_man = TenthMan::new(&params);
                    
                    // Check if we should run final review based on mode
                    let should_run_final = if let Some(ref config) = params.tenth_man_config {
                        match &config.mode {
                            InterventionMode::SystemOnly => true,
                            InterventionMode::Hybrid { force_final_review, .. } => {
                                *force_final_review
                            }
                            InterventionMode::ToolOnly => false,
                            _ => true, // Legacy modes default to true
                        }
                    } else {
                        true // Default: run final review
                    };
                    
                    if should_run_final {
                        tracing::info!("Running Tenth Man final review with full history for execution_id: {}", params.execution_id);
                        
                        match tenth_man.review_with_history(&params.execution_id).await {
                            Ok(critique) => {
                                tracing::info!("Tenth Man Critique generated ({} chars)", critique.len());
                                
                                if params.persist_messages {
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        let review_msg = core_db::AiMessage {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            conversation_id: params.execution_id.clone(),
                                            role: "system".to_string(),
                                            content: critique.clone(),
                                            metadata: Some(json!({
                                                "kind": "tenth_man_critique",
                                                "trigger": "final_review",
                                                "mode": "system_enforced"
                                            }).to_string()),
                                            token_count: Some(critique.len() as i32),
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: chrono::Utc::now(),
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };
                                        
                                        if let Err(e) = db.create_ai_message(&review_msg).await {
                                            tracing::warn!("Failed to save Tenth Man critique: {}", e);
                                        }
                                        
                                        // Emit event to frontend
                                        let _ = app.emit(
                                            "agent:tenth_man_critique",
                                            &json!({
                                                "execution_id": params.execution_id,
                                                "critique": critique,
                                                "message_id": review_msg.id,
                                                "trigger": "final_review",
                                                "mode": "system_enforced"
                                            })
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Tenth Man Review failed: {}", e);
                            }
                        }
                    } else {
                        tracing::info!("Skipping final Tenth Man review (mode: ToolOnly)");
                    }
                }
                
                // Cleanup Tenth Man execution context
                if params.enable_tenth_man_rule {
                    use crate::agents::tenth_man_executor;
                    tenth_man_executor::clear_tenth_man_execution(&params.execution_id).await;
                }

                // Cleanup container context files (keep history.txt)
                cleanup_container_context_async(&params.execution_id).await;

                return Ok(final_response);
            }
            Err(e) => {
                let err_msg = e.to_string();
                
                // Cleanup container context files even on error
                cleanup_container_context_async(&params.execution_id).await;
                
                // 优化错误消息
                let friendly_err = if err_msg.contains("error decoding response body") {
                    if err_msg.contains("UnexpectedEof") || err_msg.contains("unexpected EOF") {
                        anyhow::anyhow!("LLM provider connection closed unexpectedly. This might be a temporary issue with the provider or proxy. (Original error: {})", err_msg)
                    } else {
                        anyhow::anyhow!("Failed to decode LLM response. The provider may have returned an invalid format. (Original error: {})", err_msg)
                    }
                } else {
                    e
                };

                // 检查是否是可重试的错误（主要是解析错误和网络抖动）
                let is_retryable = is_retryable_error(&err_msg);
                let has_tool_activity = tool_calls_collector
                    .lock()
                    .map(|calls| !calls.is_empty())
                    .unwrap_or(false)
                    || pending_calls
                        .lock()
                        .map(|pending| !pending.is_empty())
                        .unwrap_or(false);

                if is_retryable && !has_tool_activity && retries < max_retries {
                    retries += 1;
                    last_error = Some(friendly_err);

                    // 保存当前工作到累积记录中（在清理之前）
                    if let Ok(current_calls) = tool_calls_collector.lock() {
                        if let Ok(mut acc) = accumulated_tool_calls.lock() {
                            acc.extend(current_calls.clone());
                        }
                    }
                    
                    if let Ok(current_output) = assistant_segment_buf.lock() {
                        if !current_output.is_empty() {
                            if let Ok(mut acc) = accumulated_assistant_output.lock() {
                                if !acc.is_empty() {
                                    acc.push_str("\n\n");
                                }
                                acc.push_str(current_output.as_str());
                            }
                        }
                    }

                    // 清理重试前的临时状态（但不清理累积记录）
                    if let Ok(mut buf) = assistant_segment_buf.lock() {
                        buf.clear();
                    }
                    if let Ok(mut buf) = reasoning_content_buf.lock() {
                        buf.clear();
                    }
                    if let Ok(mut p) = pending_calls.lock() {
                        p.clear();
                    }
                    if let Ok(mut tc) = tool_calls_collector.lock() {
                        tc.clear();
                    }

                    continue;
                } else {
                    // Final failure recording
                    let tool_summaries = tool_calls_collector
                        .lock()
                        .map(|calls| {
                            calls
                                .iter()
                                .map(|call| ToolCallSummary {
                                    name: call.name.clone(),
                                    success: call.success,
                                    duration_ms: Some(call.duration_ms),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    if let Err(err) = get_global_memory()
                        .record_execution(ExecutionRecord {
                            id: params.execution_id.clone(),
                            task: params.task.clone(),
                            environment: Some(rig_provider.clone()),
                            tool_calls: tool_summaries,
                            success: false,
                            error: Some(err_msg),
                            response_excerpt: None,
                            created_at: chrono::Utc::now().timestamp(),
                        })
                        .await
                    {
                        tracing::warn!("Failed to store memory record: {}", err);
                    }
                    return Err(friendly_err);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries reached")))
}

fn is_retryable_error(err_msg: &str) -> bool {
    let err_lower = err_msg.to_lowercase();
    if err_lower.contains("error decoding response body") {
        return true;
    }
    if err_lower.contains("unexpected eof") || err_lower.contains("connection closed") {
        return true;
    }
    if err_lower.contains("timed out") || err_lower.contains("timeout") {
        return true;
    }
    if err_lower.contains("connection reset") || err_lower.contains("network") {
        return true;
    }
    false
}
