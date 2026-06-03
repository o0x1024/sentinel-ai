use crate::proxy::{
    InterceptAction, PendingInterceptWebSocketMessage, ScanTask, TrafficProxyHandler,
    WebSocketDirection, WebSocketMessageContext,
};
use hudsucker::tokio_tungstenite::tungstenite::Message;
use hudsucker::{WebSocketContext, WebSocketHandler};
use tracing::{debug, info, warn};

/// WebSocket 处理器实现
///
/// 处理 WebSocket 消息的转发，记录消息到扫描器，
/// 优雅地处理连接关闭等情况，避免产生大量无意义的错误日志。
impl WebSocketHandler for TrafficProxyHandler {
    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // 获取消息类型和内容 (用于记录)
        let (message_type, content, content_length) = match &msg {
            Message::Text(text) => {
                // Utf8Bytes 实现 Deref<str>，需要转换为 String
                let text_str = text.to_string();
                let len = text_str.len();
                ("text".to_string(), Some(text_str), len)
            }
            Message::Binary(data) => {
                // 对于二进制数据，尝试转换为 base64
                use base64::{engine::general_purpose, Engine as _};
                let base64_content = format!("[BASE64]{}", general_purpose::STANDARD.encode(data));
                let len = data.len();
                ("binary".to_string(), Some(base64_content), len)
            }
            Message::Ping(data) => ("ping".to_string(), None, data.len()),
            Message::Pong(data) => ("pong".to_string(), None, data.len()),
            Message::Close(reason) => {
                let reason_str = reason
                    .as_ref()
                    .map(|r| format!("code={}, reason={}", r.code, r.reason));
                ("close".to_string(), reason_str, 0)
            }
            _ => ("unknown".to_string(), None, 0),
        };

        // 发送消息到扫描器 (用于记录到历史缓存)
        if let Some(tx) = &self.scan_tx {
            // 生成唯一的消息 ID
            let message_id = uuid::Uuid::new_v4().to_string();

            // 从连接映射中获取 WebSocket 连接 ID
            let conn_key = Self::generate_ws_connection_key(ctx);
            let connection_id = {
                let ws_map = self.conn_to_ws_id.read().await;
                ws_map.get(&conn_key).cloned()
            };

            // 如果找不到连接 ID，使用连接键作为备用
            let connection_id = connection_id.unwrap_or_else(|| {
                warn!(
                    "WebSocket connection ID not found for conn_key: {}, using fallback",
                    conn_key
                );
                format!("ws-unknown-{}", uuid::Uuid::new_v4().simple())
            });

            // 判断消息方向
            // Hudsucker 的 handle_message 对于 WebSocket 会被调用两次：
            // 1. 客户端 -> 服务器方向的消息
            // 2. 服务器 -> 客户端方向的消息
            // 通过交替计数来判断方向（简单但有效的方法）
            // 更精确的方法需要 Hudsucker 提供更多上下文信息

            // 获取或创建此连接的消息计数器
            let conn_key_for_counter = conn_key.clone();
            let direction = {
                let mut ws_counters = self.ws_message_counters.write().await;
                let counter = ws_counters.entry(conn_key_for_counter).or_insert(0);
                *counter += 1;

                // 假设消息交替出现：奇数为客户端->服务器，偶数为服务器->客户端
                // 这是一个简化假设，可能不完全准确，但对大多数情况有效
                if *counter % 2 == 1 {
                    WebSocketDirection::ClientToServer
                } else {
                    WebSocketDirection::ServerToClient
                }
            };

            // 拦截逻辑
            let mut intercepted = false;
            if let Some(intercept_state) = &self.intercept_state {
                let websocket_enabled = *intercept_state.websocket_enabled.read().await;

                // 只拦截文本和二进制消息
                let should_intercept = websocket_enabled
                    && match &msg {
                        Message::Text(_) | Message::Binary(_) => true,
                        _ => false,
                    };

                if should_intercept {
                    if let Some(pending_tx) = &intercept_state.pending_websocket_tx {
                        intercepted = true;

                        // 创建 oneshot channel
                        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

                        let pending_msg = PendingInterceptWebSocketMessage {
                            id: message_id.clone(),
                            connection_id: connection_id.clone(),
                            direction,
                            message_type: message_type.clone(),
                            content: content.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            response_tx,
                        };

                        info!("Intercepting WebSocket message: {}", message_id);

                        // 发送到待处理队列
                        if let Err(e) = pending_tx.send(pending_msg) {
                            warn!("Failed to send intercept websocket message: {}", e);
                            intercepted = false; // 发送失败，回退到正常记录
                        } else {
                            // 等待用户操作
                            match response_rx.await {
                                Ok(action) => match action {
                                    InterceptAction::Forward(modified_content) => {
                                        // 获取最终要发送和记录的内容
                                        let (final_content, final_length) =
                                            if let Some(ref new_content) = modified_content {
                                                (Some(new_content.clone()), new_content.len())
                                            } else {
                                                (content.clone(), content_length)
                                            };

                                        // 记录最终发送的消息到历史
                                        let final_msg_ctx = WebSocketMessageContext {
                                            id: message_id.clone(),
                                            connection_id: connection_id.clone(),
                                            direction,
                                            message_type: message_type.clone(),
                                            content: final_content,
                                            content_length: final_length,
                                            timestamp: chrono::Utc::now(),
                                        };

                                        if let Err(e) =
                                            tx.send(ScanTask::WebSocketMessage(final_msg_ctx))
                                        {
                                            warn!(
                                                "Failed to send WebSocket message to scanner: {}",
                                                e
                                            );
                                        } else {
                                            info!("WebSocket message recorded after intercept: conn_id={}, type={}, modified={}", 
                                                connection_id, message_type, modified_content.is_some());
                                        }

                                        // 如果有修改，发送修改后的消息
                                        if let Some(new_content) = modified_content {
                                            if message_type == "text" {
                                                return Some(Message::Text(new_content.into()));
                                            } else if message_type == "binary" {
                                                // 尝试从 base64 解码
                                                let clean_content =
                                                    if new_content.starts_with("[BASE64]") {
                                                        &new_content[8..]
                                                    } else {
                                                        &new_content
                                                    };

                                                use base64::{
                                                    engine::general_purpose, Engine as _,
                                                };
                                                if let Ok(decoded) =
                                                    general_purpose::STANDARD.decode(clean_content)
                                                {
                                                    return Some(Message::Binary(decoded.into()));
                                                } else {
                                                    warn!("Failed to decode base64 content for modified WebSocket message");
                                                }
                                            }
                                        }
                                        // 无修改，继续处理（走到下面的 match）
                                    }
                                    InterceptAction::Drop => {
                                        info!("Dropped WebSocket message: {}", message_id);
                                        return None;
                                    }
                                },
                                Err(_) => {
                                    warn!(
                                        "Intercept response channel closed for WebSocket message"
                                    );
                                    intercepted = false; // channel 关闭，回退到正常记录
                                }
                            }
                        }
                    }
                }
            }

            // 如果没有被拦截，正常记录消息到历史
            if !intercepted {
                let msg_ctx = WebSocketMessageContext {
                    id: message_id.clone(),
                    connection_id: connection_id.clone(),
                    direction,
                    message_type: message_type.clone(),
                    content: content.clone(),
                    content_length,
                    timestamp: chrono::Utc::now(),
                };

                if let Err(e) = tx.send(ScanTask::WebSocketMessage(msg_ctx)) {
                    warn!("Failed to send WebSocket message to scanner: {}", e);
                } else {
                    debug!(
                        "WebSocket message recorded: conn_id={}, type={}, length={}",
                        connection_id, message_type, content_length
                    );
                }
            }
        }

        // 原有的消息处理逻辑
        match &msg {
            Message::Close(_) => {
                debug!("WebSocket close message received, closing connection gracefully");
                None // 返回 None 优雅关闭，不转发 close 帧
            }
            Message::Ping(data) => {
                // 自动响应 Ping 为 Pong
                Some(Message::Pong(data.clone()))
            }
            _ => {
                // 转发其他所有消息（Text, Binary, Pong 等）
                Some(msg)
            }
        }
    }
}
