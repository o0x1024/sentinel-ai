use super::*;

pub(super) async fn bridge_invoke(
    State(state): State<GatewayAppState>,
    Json(req): Json<BridgeInvokeRequest>,
) -> Response {
    let result = match req.command.as_str() {
        "init_shell_permission_handler" => Ok(json!(true)),
        "get_license_info" => Ok(json!({
            "is_licensed": true,
            "needs_activation": false,
            "trial_active": false,
            "trial_started_at": null,
            "trial_expires_at": null,
            "trial_remaining_seconds": null,
            "trial_days_remaining": null
        })),
        "get_pending_shell_permissions" | "get_pending_shell_permissionss" => {
            match crate::commands::tool_commands::get_pending_shell_permissions().await {
                Ok(items) => Ok(json!(items)),
                Err(e) => Err(format!("get_pending_shell_permissions failed: {}", e)),
            }
        }
        "respond_shell_permission" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let allowed = req
                .payload
                .get("allowed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if id.is_empty() {
                Err("respond_shell_permission missing id".to_string())
            } else {
                match crate::commands::tool_commands::respond_shell_permission(id, allowed).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("respond_shell_permission failed: {}", e)),
                }
            }
        }
        "allow_shell_permission_forever" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("allow_shell_permission_forever missing id".to_string())
            } else {
                match crate::commands::tool_commands::allow_shell_permission_forever_with_db(
                    id,
                    state.db.as_ref(),
                )
                .await
                {
                    Ok(result) => Ok(json!(result)),
                    Err(e) => Err(format!("allow_shell_permission_forever failed: {}", e)),
                }
            }
        }
        "get_pending_ask_user_questions" => {
            match crate::commands::tool_commands::get_pending_ask_user_questions().await {
                Ok(items) => Ok(json!(items)),
                Err(e) => Err(format!("get_pending_ask_user_questions failed: {}", e)),
            }
        }
        "respond_ask_user_question" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let answers = req
                .payload
                .get("answers")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if id.is_empty() {
                Err("respond_ask_user_question missing id".to_string())
            } else {
                match serde_json::from_value::<HashMap<String, String>>(answers) {
                    Ok(parsed_answers) => {
                        match crate::commands::tool_commands::respond_ask_user_question(
                            id,
                            parsed_answers,
                        )
                        .await
                        {
                            Ok(_) => Ok(json!(null)),
                            Err(e) => Err(format!("respond_ask_user_question failed: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("respond_ask_user_question invalid answers: {}", e)),
                }
            }
        }
        "reject_ask_user_question" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("reject_ask_user_question missing id".to_string())
            } else {
                match crate::commands::tool_commands::reject_ask_user_question(id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("reject_ask_user_question failed: {}", e)),
                }
            }
        }
        "get_background_shell_tasks" => {
            let execution_id = req
                .payload
                .get("execution_id")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            match crate::commands::tool_commands::get_background_shell_tasks(execution_id).await {
                Ok(items) => Ok(json!(items)),
                Err(e) => Err(format!("get_background_shell_tasks failed: {}", e)),
            }
        }
        "stop_background_shell_task" => {
            let task_id = req
                .payload
                .get("task_id")
                .or_else(|| req.payload.get("taskId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if task_id.is_empty() {
                Err("stop_background_shell_task missing task_id".to_string())
            } else {
                match crate::commands::tool_commands::stop_background_shell_task(task_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("stop_background_shell_task failed: {}", e)),
                }
            }
        }
        "get_ai_conversations" => {
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.list_conversations().await {
                        Ok(items) => Ok(json!(items)),
                        Err(e) => Err(format!("get_ai_conversations failed: {}", e)),
                    }
                } else {
                    Ok(json!([]))
                }
            } else {
                Ok(json!([]))
            }
        }
        "get_ai_conversations_count" => {
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.get_conversations_count().await {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("get_ai_conversations_count failed: {}", e)),
                    }
                } else {
                    Ok(json!(0))
                }
            } else {
                Ok(json!(0))
            }
        }
        "get_ai_conversations_paginated" => {
            let limit = req
                .payload
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(20);
            let offset = req
                .payload
                .get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.list_conversations_paginated(limit, offset).await {
                        Ok(items) => Ok(json!(items)),
                        Err(e) => Err(format!("get_ai_conversations_paginated failed: {}", e)),
                    }
                } else {
                    Ok(json!([]))
                }
            } else {
                Ok(json!([]))
            }
        }
        "create_ai_conversation" => {
            let req_obj = req
                .payload
                .get("request")
                .cloned()
                .ok_or_else(|| "create_ai_conversation missing request".to_string())
                .and_then(|v| {
                    serde_json::from_value::<BridgeCreateConversationRequest>(v)
                        .map_err(|e| e.to_string())
                });
            match req_obj {
                Ok(v) => {
                    let service_name = if v.service_name.trim().is_empty() {
                        "default"
                    } else {
                        v.service_name.as_str()
                    };
                    if let Some(service) = state
                        .ai_manager
                        .get_service(service_name)
                        .or_else(|| state.ai_manager.get_service("default"))
                    {
                        match service.create_conversation(Some(v.title)).await {
                            Ok(id) => Ok(json!(id)),
                            Err(e) => Err(format!("create_ai_conversation failed: {}", e)),
                        }
                    } else {
                        Err("No available AI service".to_string())
                    }
                }
                Err(e) => Err(e),
            }
        }
        "get_ai_messages_by_conversation" => {
            let conversation_id = req
                .payload
                .get("conversation_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if conversation_id.is_empty() {
                Err("get_ai_messages_by_conversation missing conversation_id".to_string())
            } else {
                match state
                    .db
                    .get_ai_messages_by_conversation(&conversation_id)
                    .await
                {
                    Ok(items) => Ok(json!(items)),
                    Err(e) => Err(format!("get_ai_messages_by_conversation failed: {}", e)),
                }
            }
        }
        "save_ai_message" => {
            let req_obj = req
                .payload
                .get("request")
                .cloned()
                .ok_or_else(|| "save_ai_message missing request".to_string())
                .and_then(|v| {
                    serde_json::from_value::<BridgeSaveMessageRequest>(v).map_err(|e| e.to_string())
                });
            match req_obj {
                Ok(v) => {
                    let msg = AiMessage {
                        id: v.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                        conversation_id: v.conversation_id,
                        role: v.role,
                        content: v.content,
                        metadata: v
                            .metadata
                            .as_ref()
                            .and_then(|x| serde_json::to_string(x).ok()),
                        token_count: None,
                        cost: None,
                        tool_calls: None,
                        attachments: None,
                        reasoning_content: None,
                        timestamp: Utc::now(),
                        architecture_type: v.architecture_type,
                        architecture_meta: v
                            .architecture_meta
                            .as_ref()
                            .and_then(|x| serde_json::to_string(x).ok()),
                        structured_data: v
                            .structured_data
                            .as_ref()
                            .and_then(|x| serde_json::to_string(x).ok()),
                    };
                    match state.db.create_ai_message(&msg).await {
                        Ok(_) => Ok(json!(null)),
                        Err(e) => Err(format!("save_ai_message failed: {}", e)),
                    }
                }
                Err(e) => Err(e),
            }
        }
        "update_ai_conversation_title" => {
            let conversation_id = req
                .payload
                .get("conversationId")
                .or_else(|| req.payload.get("conversation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = req
                .payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let service_name = req
                .payload
                .get("serviceName")
                .or_else(|| req.payload.get("service_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            if conversation_id.is_empty() {
                Err("update_ai_conversation_title missing conversation_id".to_string())
            } else if let Some(service) = state
                .ai_manager
                .get_service(&service_name)
                .or_else(|| state.ai_manager.get_service("default"))
            {
                match service
                    .update_conversation_title(&conversation_id, &title)
                    .await
                {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("update_ai_conversation_title failed: {}", e)),
                }
            } else {
                Err("No available AI service".to_string())
            }
        }
        "delete_ai_conversation" => {
            let conversation_id = req
                .payload
                .get("conversationId")
                .or_else(|| req.payload.get("conversation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let service_name = req
                .payload
                .get("serviceName")
                .or_else(|| req.payload.get("service_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            if conversation_id.is_empty() {
                Err("delete_ai_conversation missing conversation_id".to_string())
            } else if let Some(service) = state
                .ai_manager
                .get_service(&service_name)
                .or_else(|| state.ai_manager.get_service("default"))
            {
                match service.delete_conversation(&conversation_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_conversation failed: {}", e)),
                }
            } else {
                Err("No available AI service".to_string())
            }
        }
        "delete_ai_message" => {
            let message_id = req
                .payload
                .get("message_id")
                .or_else(|| req.payload.get("messageId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if message_id.is_empty() {
                Err("delete_ai_message missing message_id".to_string())
            } else {
                match state.db.delete_ai_message(&message_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_message failed: {}", e)),
                }
            }
        }
        "delete_ai_messages_after" => {
            let conversation_id = req
                .payload
                .get("conversation_id")
                .or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let message_id = req
                .payload
                .get("message_id")
                .or_else(|| req.payload.get("messageId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if conversation_id.is_empty() || message_id.is_empty() {
                Err("delete_ai_messages_after missing conversation_id or message_id".to_string())
            } else {
                match state
                    .db
                    .delete_ai_messages_after(&conversation_id, &message_id)
                    .await
                {
                    Ok(n) => Ok(json!(n)),
                    Err(e) => Err(format!("delete_ai_messages_after failed: {}", e)),
                }
            }
        }
        "clear_conversation_messages" => {
            let conversation_id = req
                .payload
                .get("conversation_id")
                .or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if conversation_id.is_empty() {
                Err("clear_conversation_messages missing conversation_id".to_string())
            } else {
                match state
                    .db
                    .delete_ai_messages_by_conversation(&conversation_id)
                    .await
                {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("clear_conversation_messages failed: {}", e)),
                }
            }
        }
        "get_tool_config" => match load_tool_config_from_db(&state).await {
            Some(cfg) => Ok(json!(cfg)),
            None => Ok(json!(null)),
        },
        "save_tool_config" => {
            let tool_config = req
                .payload
                .get("toolConfig")
                .or_else(|| req.payload.get("tool_config"))
                .cloned()
                .ok_or_else(|| "save_tool_config missing tool_config".to_string())
                .and_then(|v| ToolConfig::from_json_value(v).map_err(|e| e.to_string()));
            match tool_config {
                Ok(cfg) => {
                    let raw = serde_json::to_string(&cfg).map_err(|e| e.to_string());
                    match raw {
                        Ok(value) => match state
                            .db
                            .set_config("agent", "tool_config", &value, None)
                            .await
                        {
                            Ok(_) => Ok(json!(null)),
                            Err(e) => Err(format!("save_tool_config failed: {}", e)),
                        },
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        }
        "get_ai_roles" => match state.db.get_ai_roles().await {
            Ok(items) => Ok(json!(items)),
            Err(e) => Err(format!("get_ai_roles failed: {}", e)),
        },
        "get_current_ai_role" => match state.db.get_current_ai_role().await {
            Ok(v) => Ok(json!(v)),
            Err(e) => Err(format!("get_current_ai_role failed: {}", e)),
        },
        "set_current_ai_role" => {
            let role_id = req
                .payload
                .get("roleId")
                .or_else(|| req.payload.get("role_id"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            match state.db.set_current_ai_role(role_id.as_deref()).await {
                Ok(_) => Ok(json!(null)),
                Err(e) => Err(format!("set_current_ai_role failed: {}", e)),
            }
        }
        "create_ai_role" => {
            let payload = req.payload.get("payload").cloned().unwrap_or_default();
            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let description = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let prompt = payload
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let capabilities = payload
                .get("capabilities")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if title.is_empty() {
                Err("create_ai_role missing title".to_string())
            } else {
                let now = Utc::now();
                let role = sentinel_core::models::ai::AiRole {
                    id: Uuid::new_v4().to_string(),
                    title,
                    description,
                    prompt,
                    capabilities,
                    is_system: false,
                    created_at: now,
                    updated_at: now,
                };
                match state.db.create_ai_role(&role).await {
                    Ok(_) => Ok(json!(role)),
                    Err(e) => Err(format!("create_ai_role failed: {}", e)),
                }
            }
        }
        "update_ai_role" => {
            let payload = req.payload.get("payload").cloned().unwrap_or_default();
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let description = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let prompt = payload
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let capabilities =
                payload
                    .get("capabilities")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str())
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty())
                            .collect::<Vec<_>>()
                    });
            if id.is_empty() {
                Err("update_ai_role missing id".to_string())
            } else {
                let roles = state.db.get_ai_roles().await.map_err(|e| e.to_string());
                match roles {
                    Ok(items) => {
                        let existing = items.iter().find(|r| r.id == id);
                        let role = sentinel_core::models::ai::AiRole {
                            id,
                            title,
                            description,
                            prompt,
                            capabilities: capabilities.unwrap_or_else(|| {
                                existing.map(|r| r.capabilities.clone()).unwrap_or_default()
                            }),
                            is_system: existing.map(|r| r.is_system).unwrap_or(false),
                            created_at: existing.map(|r| r.created_at).unwrap_or_else(Utc::now),
                            updated_at: Utc::now(),
                        };
                        match state.db.update_ai_role(&role).await {
                            Ok(_) => Ok(json!(null)),
                            Err(e) => Err(format!("update_ai_role failed: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("update_ai_role failed: {}", e)),
                }
            }
        }
        "delete_ai_role" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("delete_ai_role missing id".to_string())
            } else {
                match state.db.delete_ai_role(&id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_role failed: {}", e)),
                }
            }
        }
        "cancel_ai_stream" => {
            let conversation_id = req
                .payload
                .get("conversation_id")
                .or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if conversation_id.is_empty() {
                Err("cancel_ai_stream missing conversation_id".to_string())
            } else {
                let _ =
                    crate::managers::cancellation_manager::cancel_execution(&conversation_id).await;
                Ok(json!(null))
            }
        }
        "cancel_shell_execution" => {
            let execution_id = req
                .payload
                .get("execution_id")
                .or_else(|| req.payload.get("executionId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if execution_id.is_empty() {
                Err("cancel_shell_execution missing execution_id".to_string())
            } else {
                let _ = sentinel_tools::buildin_tools::shell::cancel_shell_execution(&execution_id)
                    .await;
                Ok(json!(null))
            }
        }
        "get_tool_usage_stats" => {
            let stats = get_tool_usage_statistics().await;
            Ok(json!(stats))
        }
        "get_all_tool_metadata" => {
            let router =
                crate::agents::tool_router::ToolRouter::new_with_all_tools(Some(&state.db)).await;
            let tools = router
                .list_all_tools()
                .into_iter()
                .filter(|t| {
                    t.id != sentinel_tools::buildin_tools::SkillsTool::NAME
                        && t.id != sentinel_tools::buildin_tools::SopsTool::NAME
                })
                .collect::<Vec<_>>();
            Ok(json!(tools))
        }
        "get_tool_statistics" => {
            let router =
                crate::agents::tool_router::ToolRouter::new_with_all_tools(Some(&state.db)).await;
            Ok(json!(router.get_statistics()))
        }
        "clear_tool_usage_stats" => {
            clear_tool_usage_records().await;
            Ok(json!(null))
        }
        "get_config" => {
            let request = req
                .payload
                .get("request")
                .cloned()
                .unwrap_or_else(|| req.payload.clone());
            let category = request
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let key = request
                .get("key")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            if category.is_empty() {
                Err("get_config missing category".to_string())
            } else if let Some(key) = key {
                match state.db.get_config(&category, &key).await {
                    Ok(Some(value)) => Ok(json!([{
                        "id": format!("cfg_{}_{}", category, key),
                        "category": category,
                        "key": key,
                        "value": value,
                        "description": serde_json::Value::Null,
                        "is_encrypted": false
                    }])),
                    Ok(None) => Ok(json!([])),
                    Err(e) => Err(format!("get_config failed: {}", e)),
                }
            } else {
                match state.db.get_configs_by_category(&category).await {
                    Ok(items) => Ok(json!(items
                        .into_iter()
                        .map(|c| json!({
                            "id": c.id,
                            "category": c.category,
                            "key": c.key,
                            "value": c.value.unwrap_or_default(),
                            "description": c.description,
                            "is_encrypted": c.is_encrypted
                        }))
                        .collect::<Vec<_>>())),
                    Err(e) => Err(format!("get_config failed: {}", e)),
                }
            }
        }
        "set_config" => {
            let category = req
                .payload
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let key = req
                .payload
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let value = req
                .payload
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if category.is_empty() || key.is_empty() {
                Err("set_config missing category or key".to_string())
            } else {
                match state.db.set_config(&category, &key, &value, None).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("set_config failed: {}", e)),
                }
            }
        }
        "get_agent_config" => {
            let shell = crate::commands::tool_commands::agent_config::load_shell_config_from_db(
                state.db.as_ref(),
            )
            .await;
            let terminal =
                crate::commands::tool_commands::agent_config::load_terminal_config_from_db(
                    state.db.as_ref(),
                )
                .await;
            let image_attachments =
                crate::commands::tool_commands::agent_config::load_image_attachment_config_from_db(
                    state.db.as_ref(),
                )
                .await;
            let subagent =
                crate::commands::tool_commands::agent_config::load_subagent_config_from_db(
                    state.db.as_ref(),
                )
                .await;
            let completion_guard =
                crate::commands::tool_commands::agent_config::load_completion_guard_config_from_db(
                    state.db.as_ref(),
                )
                .await;
            Ok(json!({
                "shell": shell,
                "terminal": terminal,
                "image_attachments": image_attachments,
                "subagent": subagent,
                "completion_guard": completion_guard
            }))
        }
        "save_agent_config" => {
            let save_result: Result<(), String> = async {
                if let Some(config) = req
                    .payload
                    .get("config")
                    .or_else(|| req.payload.get("payload"))
                {
                    if let Some(terminal) = config.get("terminal") {
                        if let Some(v) = terminal.get("docker_image").and_then(|v| v.as_str()) {
                            state
                                .db
                                .set_config("agent", "terminal_docker_image", v, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal
                            .get("default_execution_mode")
                            .and_then(|v| v.as_str())
                        {
                            state
                                .db
                                .set_config("agent", "default_execution_mode", v, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) =
                            terminal.get("docker_memory_limit").and_then(|v| v.as_str())
                        {
                            state
                                .db
                                .set_config("agent", "docker_memory_limit", v, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal.get("docker_cpu_limit").and_then(|v| v.as_str()) {
                            state
                                .db
                                .set_config("agent", "docker_cpu_limit", v, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal
                            .get("docker_use_host_network")
                            .and_then(|v| v.as_bool())
                        {
                            let val = if v { "true" } else { "false" };
                            state
                                .db
                                .set_config("agent", "docker_use_host_network", val, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                    if let Some(subagent) = config.get("subagent") {
                        if let Some(v) = subagent.get("timeout_secs").and_then(|v| v.as_u64()) {
                            state
                                .db
                                .set_config("agent", "subagent_timeout_secs", &v.to_string(), None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                    if let Some(image_cfg) = config.get("image_attachments") {
                        if let Some(v) = image_cfg.get("mode").and_then(|v| v.as_str()) {
                            state
                                .db
                                .set_config("agent", "image_attachment_mode", v, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = image_cfg
                            .get("allow_upload_to_model")
                            .and_then(|v| v.as_bool())
                        {
                            let val = if v { "true" } else { "false" };
                            state
                                .db
                                .set_config("agent", "allow_image_upload_to_model", val, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                    if let Some(guard_cfg) = config.get("completion_guard") {
                        if let Some(v) = guard_cfg.get("enabled").and_then(|v| v.as_bool()) {
                            let val = if v { "true" } else { "false" };
                            state
                                .db
                                .set_config("agent", "completion_guard_enabled", val, None)
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = guard_cfg
                            .get("tool_heavy_min_tool_calls")
                            .and_then(|v| v.as_u64())
                        {
                            state
                                .db
                                .set_config(
                                    "agent",
                                    "completion_guard_tool_heavy_min_tool_calls",
                                    &v.to_string(),
                                    None,
                                )
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = guard_cfg
                            .get("min_response_chars_tool_heavy")
                            .and_then(|v| v.as_u64())
                        {
                            state
                                .db
                                .set_config(
                                    "agent",
                                    "completion_guard_min_response_chars_tool_heavy",
                                    &v.to_string(),
                                    None,
                                )
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = guard_cfg
                            .get("min_response_chars_after_timeout")
                            .and_then(|v| v.as_u64())
                        {
                            state
                                .db
                                .set_config(
                                    "agent",
                                    "completion_guard_min_response_chars_after_timeout",
                                    &v.to_string(),
                                    None,
                                )
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = guard_cfg
                            .get("unfinished_prefix_max_chars")
                            .and_then(|v| v.as_u64())
                        {
                            state
                                .db
                                .set_config(
                                    "agent",
                                    "completion_guard_unfinished_prefix_max_chars",
                                    &v.to_string(),
                                    None,
                                )
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = guard_cfg
                            .get("enforce_artifact_proof")
                            .and_then(|v| v.as_bool())
                        {
                            let val = if v { "true" } else { "false" };
                            state
                                .db
                                .set_config(
                                    "agent",
                                    "completion_guard_enforce_artifact_proof",
                                    val,
                                    None,
                                )
                                .await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                }
                Ok(())
            }
            .await;
            match save_result {
                Ok(_) => Ok(json!(null)),
                Err(e) => Err(e),
            }
        }
        "get_subagent_runs" => {
            let parent_id = req
                .payload
                .get("parentExecutionId")
                .or_else(|| req.payload.get("parent_execution_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if parent_id.is_empty() {
                Err("get_subagent_runs missing parent_execution_id".to_string())
            } else {
                match state
                    .db
                    .get_subagent_runs_by_parent_internal(&parent_id)
                    .await
                {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("get_subagent_runs failed: {}", e)),
                }
            }
        }
        "start_terminal_server" => {
            let cfg = req.payload.get("config").cloned().and_then(|v| {
                serde_json::from_value::<crate::commands::terminal_commands::TerminalServerConfig>(
                    v,
                )
                .ok()
            });
            match crate::commands::terminal_commands::start_terminal_server(cfg).await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("start_terminal_server failed: {}", e)),
            }
        }
        "stop_terminal_server" => {
            match crate::commands::terminal_commands::stop_terminal_server().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("stop_terminal_server failed: {}", e)),
            }
        }
        "get_terminal_server_status" => {
            match crate::commands::terminal_commands::get_terminal_server_status().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("get_terminal_server_status failed: {}", e)),
            }
        }
        "list_terminal_sessions" => {
            match crate::commands::terminal_commands::list_terminal_sessions().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("list_terminal_sessions failed: {}", e)),
            }
        }
        "stop_terminal_session" => {
            let session_id = req
                .payload
                .get("sessionId")
                .or_else(|| req.payload.get("session_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if session_id.is_empty() {
                Err("stop_terminal_session missing session_id".to_string())
            } else {
                match crate::commands::terminal_commands::stop_terminal_session(session_id).await {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("stop_terminal_session failed: {}", e)),
                }
            }
        }
        "get_terminal_websocket_url" => {
            match crate::commands::terminal_commands::get_terminal_websocket_url().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("get_terminal_websocket_url failed: {}", e)),
            }
        }
        "list_skills" => match state.db.list_skills_summary().await {
            Ok(v) => Ok(json!(v)),
            Err(e) => Err(format!("list_skills failed: {}", e)),
        },
        "list_skills_full" => match state.db.list_all_skills().await {
            Ok(v) => Ok(json!(v)),
            Err(e) => Err(format!("list_skills_full failed: {}", e)),
        },
        "refresh_skills_index" => match scan_and_upsert_skills(state.db.as_ref()).await {
            Ok(v) => Ok(json!(v)),
            Err(e) => Err(format!("refresh_skills_index failed: {}", e)),
        },
        "get_skill_markdown" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("get_skill_markdown missing id".to_string())
            } else {
                match state.db.get_skill(&id).await {
                    Ok(Some(skill)) => {
                        let root = skills_root(state.db.as_ref());
                        let skill_path = if !skill.source_path.is_empty() {
                            root.join(&skill.source_path)
                        } else {
                            root.join(&skill.id).join("SKILL.md")
                        };
                        match std::fs::read_to_string(&skill_path) {
                            Ok(content) => match parse_skill_markdown(&content) {
                                Ok(doc) => Ok(json!(doc.body)),
                                Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                            },
                            Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                        }
                    }
                    Ok(None) => Err("Skill not found".to_string()),
                    Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                }
            }
        }
        "create_skill" => {
            let payload = req
                .payload
                .get("payload")
                .cloned()
                .unwrap_or_else(|| req.payload.clone());
            let name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            match normalize_skill_id(name) {
                Ok(id) => {
                    let description = payload
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if description.is_empty() {
                        Err("Skill description is required".to_string())
                    } else {
                        match state.db.get_skill(&id).await {
                            Ok(Some(_)) => Err("Skill name already exists".to_string()),
                            Ok(None) => {
                                let root = skills_root(state.db.as_ref());
                                let skill_dir = root.join(&id);
                                let skill_md = skill_dir.join("SKILL.md");
                                if let Err(e) = std::fs::create_dir_all(&skill_dir) {
                                    Err(format!("create_skill failed: {}", e))
                                } else {
                                    let content = payload
                                        .get("content")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default();
                                    let markdown = build_skill_markdown(&id, &description, content);
                                    if let Err(e) = std::fs::write(&skill_md, markdown) {
                                        Err(format!("create_skill failed: {}", e))
                                    } else {
                                        let create_payload = sentinel_db::CreateSkill {
                                            id: id.clone(),
                                            name: id.clone(),
                                            description,
                                            source_path: skill_md
                                                .strip_prefix(&root)
                                                .unwrap_or(&skill_md)
                                                .to_string_lossy()
                                                .to_string(),
                                            argument_hint: payload
                                                .get("argument_hint")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            disable_model_invocation: payload
                                                .get("disable_model_invocation")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false),
                                            user_invocable: payload
                                                .get("user_invocable")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(true),
                                            allowed_tools: payload
                                                .get("allowed_tools")
                                                .and_then(|v| {
                                                    serde_json::from_value::<Vec<String>>(v.clone())
                                                        .ok()
                                                })
                                                .unwrap_or_default(),
                                            model: payload
                                                .get("model")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            context: payload
                                                .get("context")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            agent: payload
                                                .get("agent")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            hooks: payload.get("hooks").cloned(),
                                        };
                                        match state.db.create_skill(&create_payload).await {
                                            Ok(skill) => Ok(json!(skill)),
                                            Err(e) => Err(format!("create_skill failed: {}", e)),
                                        }
                                    }
                                }
                            }
                            Err(e) => Err(format!("create_skill failed: {}", e)),
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        "update_skill" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let payload = req
                .payload
                .get("payload")
                .cloned()
                .unwrap_or_else(|| req.payload.clone());
            if id.is_empty() {
                Err("update_skill missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                let mut write_error: Option<String> = None;
                if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
                    let desc = payload
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let desc = if desc.is_empty() {
                        match state.db.get_skill(&id).await {
                            Ok(Some(existing)) => existing.description,
                            _ => String::new(),
                        }
                    } else {
                        desc
                    };
                    if desc.is_empty() {
                        write_error = Some("update_skill missing description".to_string());
                    } else {
                        let _ = std::fs::create_dir_all(&skill_dir);
                        let _ = std::fs::write(
                            skill_dir.join("SKILL.md"),
                            build_skill_markdown(&id, &desc, content),
                        );
                    }
                }

                if let Some(err) = write_error {
                    Err(err)
                } else {
                    let update_payload = sentinel_db::UpdateSkill {
                        name: payload
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        description: payload
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        source_path: Some(
                            skill_dir
                                .join("SKILL.md")
                                .strip_prefix(&root)
                                .unwrap_or(&skill_dir.join("SKILL.md"))
                                .to_string_lossy()
                                .to_string(),
                        ),
                        argument_hint: payload
                            .get("argument_hint")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        disable_model_invocation: payload
                            .get("disable_model_invocation")
                            .and_then(|v| v.as_bool()),
                        user_invocable: payload.get("user_invocable").and_then(|v| v.as_bool()),
                        allowed_tools: payload
                            .get("allowed_tools")
                            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok()),
                        model: payload
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        context: payload
                            .get("context")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        agent: payload
                            .get("agent")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string()),
                        hooks: payload.get("hooks").cloned(),
                    };
                    match state.db.update_skill(&id, &update_payload).await {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("update_skill failed: {}", e)),
                    }
                }
            }
        }
        "delete_skill" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("delete_skill missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let dir = root.join(&id);
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                match state.db.delete_skill(&id).await {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("delete_skill failed: {}", e)),
                }
            }
        }
        "list_skill_files" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                Err("list_skill_files missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                if !skill_dir.exists() {
                    Err("Skill directory not found".to_string())
                } else {
                    let mut out = Vec::new();
                    for entry in walkdir::WalkDir::new(&skill_dir)
                        .max_depth(5)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            let p = entry.path();
                            let rel = p
                                .strip_prefix(&skill_dir)
                                .unwrap_or(p)
                                .to_string_lossy()
                                .to_string();
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            out.push(json!({ "path": rel, "size": size }));
                        }
                    }
                    out.sort_by(|a, b| {
                        a.get("path")
                            .and_then(|v| v.as_str())
                            .cmp(&b.get("path").and_then(|v| v.as_str()))
                    });
                    Ok(json!(out))
                }
            }
        }
        "read_skill_file" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let path = req
                .payload
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() || path.is_empty() {
                Err("read_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                match resolve_skill_file_for_read(&skill_dir, &path) {
                    Ok(p) => match std::fs::read_to_string(&p) {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("read_skill_file failed: {}", e)),
                    },
                    Err(e) => Err(format!("read_skill_file failed: {}", e)),
                }
            }
        }
        "save_skill_file" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let path = req
                .payload
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let content = req
                .payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() || path.is_empty() {
                Err("save_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                if !skill_dir.exists() {
                    Err("Skill directory not found".to_string())
                } else {
                    match resolve_skill_file_for_write(&skill_dir, &path) {
                        Ok(p) => match std::fs::write(&p, content) {
                            Ok(_) => Ok(json!(true)),
                            Err(e) => Err(format!("save_skill_file failed: {}", e)),
                        },
                        Err(e) => Err(format!("save_skill_file failed: {}", e)),
                    }
                }
            }
        }
        "delete_skill_file" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let path = req
                .payload
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() || path.is_empty() {
                Err("delete_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                match resolve_skill_file_for_read(&skill_dir, &path) {
                    Ok(p) => match std::fs::remove_file(&p) {
                        Ok(_) => Ok(json!(true)),
                        Err(e) => Err(format!("delete_skill_file failed: {}", e)),
                    },
                    Err(e) => Err(format!("delete_skill_file failed: {}", e)),
                }
            }
        }
        "import_skill_file" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let source_path = req
                .payload
                .get("sourcePath")
                .or_else(|| req.payload.get("source_path"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let target_path = req
                .payload
                .get("targetPath")
                .or_else(|| req.payload.get("target_path"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            if id.is_empty() || source_path.is_empty() {
                Err("import_skill_file missing id or source_path".to_string())
            } else {
                let source = PathBuf::from(&source_path);
                if !source.is_file() {
                    Err("Source file not found".to_string())
                } else {
                    let filename = source
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or("imported.txt")
                        .to_string();
                    let rel_target = target_path.unwrap_or(filename);
                    let root = skills_root(state.db.as_ref());
                    let skill_dir = root.join(&id);
                    match resolve_skill_file_for_write(&skill_dir, &rel_target) {
                        Ok(p) => match std::fs::copy(&source, &p) {
                            Ok(_) => Ok(json!(rel_target)),
                            Err(e) => Err(format!("import_skill_file failed: {}", e)),
                        },
                        Err(e) => Err(format!("import_skill_file failed: {}", e)),
                    }
                }
            }
        }
        "generate_plugin_stream" => Ok(json!(null)),
        "agent_execute" => {
            let req_obj = serde_json::from_value::<BridgeAgentExecuteRequest>(req.payload.clone())
                .map_err(|e| format!("invalid agent_execute payload: {}", e));
            match req_obj {
                Ok(v) => {
                    let conversation_id = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("conversation_id").and_then(|x| x.as_str()))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    let system_prompt = state
                        .db
                        .get_current_ai_role()
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.prompt);
                    let tool_config = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("tool_config"))
                        .and_then(|x| ToolConfig::from_json_value(x.clone()).ok());
                    let max_iterations = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("max_iterations").and_then(|x| x.as_u64()))
                        .map(|v| v as usize);
                    let timeout_secs = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("timeout_secs").and_then(|x| x.as_u64()));
                    let enable_tenth_man_rule = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("enable_tenth_man_rule").and_then(|x| x.as_bool()));
                    if let Err(e) =
                        ensure_conversation_exists(&state, &conversation_id, Some("default")).await
                    {
                        Err(e)
                    } else {
                        match run_agent_execution(
                            &state,
                            &conversation_id,
                            &v.task,
                            system_prompt.as_deref(),
                            tool_config,
                            max_iterations,
                            timeout_secs,
                            enable_tenth_man_rule,
                            v.config.as_ref().and_then(|c| {
                                c.get("current_browser_shell_direct_write_enabled")
                                    .and_then(|x| x.as_bool())
                            }),
                            v.config.as_ref().and_then(|c| {
                                c.get("current_browser_shell_session_id")
                                    .and_then(|x| x.as_str())
                            }),
                            v.config.as_ref().and_then(|c| {
                                c.get("current_terminal_session_id")
                                    .and_then(|x| x.as_str())
                            }),
                            v.config.as_ref().and_then(|c| {
                                c.get("current_terminal_session_fingerprint")
                                    .and_then(|x| x.as_str())
                            }),
                        )
                        .await
                        {
                            Ok(_) => {
                                let message_id = v
                                    .config
                                    .as_ref()
                                    .and_then(|c| c.get("message_id").and_then(|x| x.as_str()))
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                                Ok(json!(message_id))
                            }
                            Err(e) => Err(format!("agent_execute failed: {}", e)),
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        "http_gateway_get_status" => Ok(json!({
            "running": true,
            "bind_addr": serde_json::Value::Null,
            "started_at": serde_json::Value::Null
        })),
        _ => {
            warn!("http_gateway.bridge.unimplemented command={}", req.command);
            Ok(json!(null))
        }
    };

    match result {
        Ok(data) => Json(BridgeInvokeResponse {
            ok: true,
            data: Some(data),
            error: None,
        })
        .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(BridgeInvokeResponse {
                ok: false,
                data: None,
                error: Some(err),
            }),
        )
            .into_response(),
    }
}
