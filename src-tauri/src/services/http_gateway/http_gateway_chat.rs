use super::*;

pub(super) async fn chat(
    State(state): State<GatewayAppState>,
    Json(payload): Json<ChatRequest>,
) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.message.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "message is required",
            &request_id,
        );
    }

    let session_id = payload
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mode = match parse_mode(payload.mode.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return error_response_with_request_id(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                &e,
                &request_id,
            );
        }
    };

    if let Err(e) =
        ensure_conversation_exists(&state, &session_id, payload.service_name.as_deref()).await
    {
        return error_response_with_request_id(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e,
            &request_id,
        );
    }

    if let Err(e) = try_register_active_execution(&state, &session_id, &request_id) {
        return error_response_with_request_id(StatusCode::CONFLICT, "CONFLICT", &e, &request_id);
    }

    let history = match load_history_from_db(&state, &session_id).await {
        Ok(h) => h,
        Err(e) => {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &e,
                &request_id,
            );
        }
    };

    let completion = if mode == GatewayMode::Agent {
        match run_agent_execution(
            &state,
            &session_id,
            &payload.message,
            payload.system_prompt.as_deref(),
            payload.tool_config.clone(),
            payload.max_iterations,
            payload.timeout_secs,
            payload.enable_tenth_man_rule,
            payload.current_browser_shell_direct_write_enabled,
            payload.current_browser_shell_session_id.as_deref(),
            payload.current_terminal_session_id.as_deref(),
            payload.current_terminal_session_fingerprint.as_deref(),
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                release_active_execution(&state, &session_id, &request_id);
                return error_response_with_request_id(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    &e,
                    &request_id,
                );
            }
        }
    } else {
        let completion = match run_chat_completion(
            &state,
            payload.service_name.as_deref(),
            payload.system_prompt.as_deref(),
            &payload.message,
            &history,
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                release_active_execution(&state, &session_id, &request_id);
                return error_response_with_request_id(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    &e,
                    &request_id,
                );
            }
        };

        if let Err(e) = persist_message(&state, &session_id, "user", payload.message.clone()).await
        {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &e,
                &request_id,
            );
        }
        if let Err(e) = persist_message(&state, &session_id, "assistant", completion.clone()).await
        {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &e,
                &request_id,
            );
        }
        completion
    };

    release_active_execution(&state, &session_id, &request_id);

    Json(ChatResponse {
        id: Uuid::new_v4().to_string(),
        session_id,
        message: completion,
        request_id,
        mode: if mode == GatewayMode::Agent {
            "agent".to_string()
        } else {
            "llm".to_string()
        },
    })
    .into_response()
}

pub(super) async fn session_chat(
    Path(session_id): Path<String>,
    State(state): State<GatewayAppState>,
    Json(payload): Json<SessionChatRequest>,
) -> Response {
    if payload.message.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "message is required",
        );
    }

    match state.db.get_ai_conversation(&session_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Session not found"),
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &format!("Failed to query session: {}", e),
            );
        }
    }

    let req = ChatRequest {
        message: payload.message,
        session_id: Some(session_id),
        request_id: payload.request_id,
        since_message_id: payload.since_message_id,
        service_name: payload.service_name,
        system_prompt: payload.system_prompt,
        mode: payload.mode,
        tool_config: payload.tool_config,
        max_iterations: payload.max_iterations,
        timeout_secs: payload.timeout_secs,
        enable_tenth_man_rule: payload.enable_tenth_man_rule,
        current_browser_shell_direct_write_enabled: payload
            .current_browser_shell_direct_write_enabled,
        current_browser_shell_session_id: payload.current_browser_shell_session_id,
        current_terminal_session_fingerprint: payload.current_terminal_session_fingerprint,
        current_terminal_session_id: payload.current_terminal_session_id,
    };
    chat(State(state), Json(req)).await
}

pub(super) async fn chat_stream(
    State(state): State<GatewayAppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.message.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "message is required",
            &request_id,
        );
    }

    let session_id = payload
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mode = match parse_mode(payload.mode.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return error_response_with_request_id(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                &e,
                &request_id,
            );
        }
    };

    if let Err(e) =
        ensure_conversation_exists(&state, &session_id, payload.service_name.as_deref()).await
    {
        return error_response_with_request_id(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e,
            &request_id,
        );
    }

    if let Err(e) = try_register_active_execution(&state, &session_id, &request_id) {
        return error_response_with_request_id(StatusCode::CONFLICT, "CONFLICT", &e, &request_id);
    }

    let history = match load_history_from_db(&state, &session_id).await {
        Ok(h) => h,
        Err(e) => {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &e,
                &request_id,
            );
        }
    };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state_for_task = state.clone();
    let user_message = payload.message.clone();
    let service_name = payload.service_name.clone();
    let system_prompt = payload.system_prompt.clone();
    let tool_config = payload.tool_config.clone();
    let max_iterations = payload.max_iterations;
    let timeout_secs = payload.timeout_secs;
    let enable_tenth_man_rule = payload.enable_tenth_man_rule;
    let since_message_id = resolve_stream_cursor(payload.since_message_id.clone(), &headers);
    let session_id_for_task = session_id.clone();
    let mode_for_task = mode;
    let request_id_for_task = request_id.clone();

    let initial_seen_ids: HashSet<String> =
        match state.db.get_ai_messages_by_conversation(&session_id).await {
            Ok(messages) => {
                if let Some(since_id) = since_message_id.as_deref() {
                    match messages.iter().position(|m| m.id == since_id) {
                        Some(idx) => messages[..=idx].iter().map(|m| m.id.clone()).collect(),
                        None => HashSet::new(),
                    }
                } else {
                    messages.iter().map(|m| m.id.clone()).collect()
                }
            }
            Err(_) => HashSet::new(),
        };

    if mode_for_task != GatewayMode::Agent {
        if let Err(e) = persist_message(&state, &session_id, "user", payload.message.clone()).await
        {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &e,
                &request_id,
            );
        }
    }

    tokio::spawn(async move {
        let _ = tx.send(
            json!({
                "type":"task_status",
                "status":"running",
                "session_id": session_id_for_task,
                "request_id": request_id_for_task,
                "mode": if mode_for_task == GatewayMode::Agent { "agent" } else { "llm" },
            })
            .to_string(),
        );
        if mode_for_task == GatewayMode::Agent {
            let mut seen_ids: HashSet<String> = initial_seen_ids;
            let mut seen_permission_ids: HashSet<String> = HashSet::new();
            let finished = Arc::new(AtomicBool::new(false));
            let poll_state = state_for_task.clone();
            let poll_sid = session_id_for_task.clone();
            let tx_poll = tx.clone();
            let finished_for_poll = finished.clone();
            let request_id_for_poll = request_id_for_task.clone();

            let poller = tokio::spawn(async move {
                while !finished_for_poll.load(Ordering::Relaxed) {
                    if let Ok(messages) = poll_state
                        .db
                        .get_ai_messages_by_conversation(&poll_sid)
                        .await
                    {
                        for m in messages {
                            if !seen_ids.insert(m.id.clone()) {
                                continue;
                            }
                            if m.role == "assistant" {
                                if tx_poll.send(json!({"type":"assistant_delta","role":"assistant","content": m.content, "message_id": m.id, "request_id": request_id_for_poll}).to_string()).is_err() {
                                    let _ = crate::managers::cancellation_manager::cancel_execution(&poll_sid).await;
                                    return;
                                }
                            } else if m.role == "tool" {
                                let payload = m
                                    .metadata
                                    .as_ref()
                                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                                    .unwrap_or(json!({"raw": m.content}));
                                if tx_poll.send(json!({"type":"tool_event","data": payload, "message_id": m.id, "request_id": request_id_for_poll}).to_string()).is_err() {
                                    let _ = crate::managers::cancellation_manager::cancel_execution(&poll_sid).await;
                                    return;
                                }
                            }
                        }
                    }
                    if let Ok(pending_perms) =
                        crate::commands::tool_commands::get_pending_shell_permissions().await
                    {
                        for req in pending_perms {
                            if req.execution_id.as_deref() != Some(&poll_sid) {
                                continue;
                            }
                            if !seen_permission_ids.insert(req.id.clone()) {
                                continue;
                            }
                            if tx_poll
                                .send(
                                    json!({
                                        "type":"permission_required",
                                        "permission": req,
                                        "request_id": request_id_for_poll,
                                    })
                                    .to_string(),
                                )
                                .is_err()
                            {
                                let _ = crate::managers::cancellation_manager::cancel_execution(
                                    &poll_sid,
                                )
                                .await;
                                return;
                            }
                        }
                    }
                    if let Ok(pending_questions) =
                        crate::commands::tool_commands::get_pending_ask_user_questions().await
                    {
                        for req in pending_questions {
                            if req.execution_id.as_deref() != Some(&poll_sid) {
                                continue;
                            }
                            if !seen_permission_ids.insert(format!("question:{}", req.id)) {
                                continue;
                            }
                            if tx_poll
                                .send(
                                    json!({
                                        "type":"ask_user_question_required",
                                        "question": req,
                                        "request_id": request_id_for_poll,
                                    })
                                    .to_string(),
                                )
                                .is_err()
                            {
                                let _ = crate::managers::cancellation_manager::cancel_execution(
                                    &poll_sid,
                                )
                                .await;
                                return;
                            }
                        }
                    }
                    if let Ok(background_tasks) =
                        crate::commands::tool_commands::get_background_shell_tasks(Some(
                            poll_sid.clone(),
                        ))
                        .await
                    {
                        for task in background_tasks {
                            let key = format!("shell-bg:{}:{:?}", task.id, task.status);
                            if !seen_permission_ids.insert(key) {
                                continue;
                            }
                            if tx_poll
                                .send(
                                    json!({
                                        "type":"shell_background_task",
                                        "task": task,
                                        "request_id": request_id_for_poll,
                                    })
                                    .to_string(),
                                )
                                .is_err()
                            {
                                let _ = crate::managers::cancellation_manager::cancel_execution(
                                    &poll_sid,
                                )
                                .await;
                                return;
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            });

            let completion = run_agent_execution(
                &state_for_task,
                &session_id_for_task,
                &user_message,
                system_prompt.as_deref(),
                tool_config.clone(),
                max_iterations,
                timeout_secs,
                enable_tenth_man_rule,
                payload.current_browser_shell_direct_write_enabled,
                payload.current_browser_shell_session_id.as_deref(),
                payload.current_terminal_session_id.as_deref(),
                payload.current_terminal_session_fingerprint.as_deref(),
            )
            .await;
            finished.store(true, Ordering::Relaxed);
            poller.abort();

            match completion {
                Ok(final_text) => {
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"completed",
                            "session_id": session_id_for_task,
                            "message": final_text,
                            "service_name": service_name,
                            "user_message": user_message,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
                Err(e) => {
                    let _ = tx.send(
                        json!({
                            "type":"error",
                            "code":"INTERNAL_ERROR",
                            "message": e,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"failed",
                            "session_id": session_id_for_task,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
            }
        } else {
            let service = match resolve_service(&state_for_task, service_name.as_deref()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(
                        json!({"type":"error","code":"INTERNAL_ERROR","message": e}).to_string(),
                    );
                    release_active_execution(
                        &state_for_task,
                        &session_id_for_task,
                        &request_id_for_task,
                    );
                    return;
                }
            };
            let llm_config = apply_generation_settings_from_db(
                state_for_task.db.as_ref(),
                service.service.to_llm_config(),
            )
            .await;
            let llm_client = sentinel_llm::LlmClient::new(llm_config);
            let completion = llm_client
                .chat(system_prompt.as_deref(), &user_message, &history, None)
                .await;

            match completion {
                Ok(final_text) => {
                    let _ = tx.send(
                        json!({
                            "type":"assistant_delta",
                            "role":"assistant",
                            "content": final_text,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                    let _ = persist_message(
                        &state_for_task,
                        &session_id_for_task,
                        "assistant",
                        final_text.clone(),
                    )
                    .await;
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"completed",
                            "session_id": session_id_for_task,
                            "message": final_text,
                            "service_name": service_name,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
                Err(e) => {
                    let _ = tx.send(
                        json!({
                            "type":"error",
                            "code":"INTERNAL_ERROR",
                            "message": format!("LLM completion error: {}", e),
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"failed",
                            "session_id": session_id_for_task,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
            }
        }
        release_active_execution(&state_for_task, &session_id_for_task, &request_id_for_task);
    });

    let stream = UnboundedReceiverStream::new(rx).map(|msg| {
        let mut event = Event::default().event("message");
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) {
            if let Some(id) = v
                .get("message_id")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                event = event.id(id);
            }
        }
        Ok::<Event, Infallible>(event.data(msg))
    });

    Sse::new(stream)
        .keep_alive(
            KeepAlive::default()
                .interval(Duration::from_secs(10))
                .text("keepalive"),
        )
        .into_response()
}

pub(super) async fn get_pending_permissions(
    Query(query): Query<PendingPermissionsQuery>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    match crate::commands::tool_commands::get_pending_shell_permissions().await {
        Ok(items) => {
            let filtered = if let Some(session_id) = query.session_id.as_deref() {
                items
                    .into_iter()
                    .filter(|v| v.execution_id.as_deref() == Some(session_id))
                    .collect()
            } else {
                items
            };
            Json(PendingPermissionsResponse {
                items: filtered,
                request_id,
            })
            .into_response()
        }
        Err(e) => error_response_with_request_id(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &format!("Failed to query pending permissions: {}", e),
            &request_id,
        ),
    }
}

pub(super) async fn respond_permission(Json(payload): Json<PermissionRespondRequest>) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.id.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "id is required",
            &request_id,
        );
    }

    if let Some(session_id) = payload.session_id.as_deref() {
        match crate::commands::tool_commands::get_pending_shell_permissions().await {
            Ok(items) => {
                let matched = items
                    .iter()
                    .find(|v| v.id == payload.id)
                    .map(|v| v.execution_id.as_deref() == Some(session_id))
                    .unwrap_or(false);
                if !matched {
                    return error_response_with_request_id(
                        StatusCode::FORBIDDEN,
                        "FORBIDDEN",
                        "permission request does not belong to this session",
                        &request_id,
                    );
                }
            }
            Err(e) => {
                return error_response_with_request_id(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    &format!("Failed to validate permission request: {}", e),
                    &request_id,
                );
            }
        }
    }

    match crate::commands::tool_commands::respond_shell_permission(payload.id, payload.allowed)
        .await
    {
        Ok(_) => Json(json!({
            "ok": true,
            "request_id": request_id,
        }))
        .into_response(),
        Err(e) => error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            &format!("Failed to respond permission: {}", e),
            &request_id,
        ),
    }
}
