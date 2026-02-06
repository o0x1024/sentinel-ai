//! WebSocket terminal server

use super::manager::TerminalSessionManager;
use super::session::TerminalSessionConfig;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use bytes::Bytes;

#[derive(serde::Deserialize)]
struct ResizeMessage {
    #[serde(rename = "type")]
    message_type: String,
    rows: u16,
    cols: u16,
}

/// WebSocket terminal server
pub struct TerminalServer {
    manager: Arc<TerminalSessionManager>,
    addr: SocketAddr,
    running: Arc<RwLock<bool>>,
}


impl TerminalServer {
    pub const NAME: &'static str = "interactive_shell";
    pub const DESCRIPTION: &'static str = "Interactive shell for interactive tools like ssh, msfconsole, sqlmap, etc.";
}

impl TerminalServer {
    /// Create a new terminal server
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            manager: super::TERMINAL_MANAGER.clone(),
            addr,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the server
    pub async fn start(self: Arc<Self>) -> Result<(), String> {
        info!("Starting terminal server on {}", self.addr);

        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("Failed to bind: {}", e))?;

        *self.running.write().await = true;

        while *self.running.read().await {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from: {}", addr);
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }

        info!("Terminal server stopped");
        Ok(())
    }

    /// Stop the server
    pub async fn stop(&self) {
        info!("Stopping terminal server");
        *self.running.write().await = false;
    }

    /// Handle WebSocket connection
    async fn handle_connection(&self, stream: TcpStream) -> Result<(), String> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Wait for initial message with session config or session ID
        let init_msg = ws_receiver
            .next()
            .await
            .ok_or("No initial message")?
            .map_err(|e| format!("Failed to receive init message: {}", e))?;

        let (session_id, mut output_rx) = match init_msg {
            Message::Text(text) => {
                debug!("Received init message: {}", text);
                
                // Try to parse as session ID first
                if text.starts_with("session:") {
                    let session_id = text.strip_prefix("session:").unwrap().to_string();
                    
                    // Check if session exists
                    if let Some(session_lock) = self.manager.get_session(&session_id).await {
                        info!("Reconnecting to existing session: {}", session_id);
                        
                        // Create a new output channel for this WebSocket subscriber
                        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                        {
                            let session = session_lock.read().await;
                            session.add_subscriber(tx).await;
                        }
                        
                        (session_id, rx)
                    } else {
                        // Session not found - client has stale session ID
                        // Create new session with default config instead of returning error
                        warn!("Session not found: {}, creating new session", session_id);
                        self.manager.create_session(TerminalSessionConfig::default()).await?
                    }
                } else {
                    // Parse as config
                    let config: TerminalSessionConfig = serde_json::from_str(&text)
                        .unwrap_or_default();
                    
                    self.manager.create_session(config).await?
                }
            }
            _ => {
                // No config provided, use default
                self.manager.create_session(TerminalSessionConfig::default()).await?
            }
        };

        info!("Terminal session established: {}", session_id);

        // Send session ID to client
        ws_sender
            .send(Message::Text(format!("session:{}", session_id).into()))
            .await
            .map_err(|e| format!("Failed to send session ID: {}", e))?;

        // Spawn task to forward output to WebSocket
        let session_id_clone = session_id.clone();
        let output_task = tokio::spawn(async move {
            info!("[WS Session {}] Output forwarding task started", session_id_clone);
            let mut chunk_count = 0;
            while let Some(data) = output_rx.recv().await {
                chunk_count += 1;
                info!("[WS Session {}] Forwarding chunk #{}: {} bytes", session_id_clone, chunk_count, data.len());
                // Convert Vec<u8> to Bytes
                let bytes_data = Bytes::from(data);
                if let Err(e) = ws_sender.send(Message::Binary(bytes_data)).await {
                    error!("[WS Session {}] Failed to send output: {}", session_id_clone, e);
                    break;
                }
            }
            info!("[WS Session {}] Output task ended, total chunks sent: {}", session_id_clone, chunk_count);
        });

        // Handle input from WebSocket
        let manager = self.manager.clone();
        let session_id_clone = session_id.clone();
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Convert Bytes to Vec<u8>
                    let vec_data = data.to_vec();
                    if let Err(e) = manager.write_to_session(&session_id_clone, vec_data).await {
                        warn!("Failed to write to session: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    if text == "__keepalive__" {
                        let _ = manager.touch_session(&session_id_clone).await;
                        continue;
                    }

                    if let Ok(resize) = serde_json::from_str::<ResizeMessage>(&text) {
                        if resize.message_type == "resize" {
                            if let Err(e) = manager
                                .resize_session(&session_id_clone, resize.rows, resize.cols)
                                .await
                            {
                                warn!("Failed to resize session: {}", e);
                            }
                            continue;
                        }
                    }

                    // Convert Utf8Bytes to Vec<u8>
                    let data = text.as_bytes().to_vec();
                    if let Err(e) = manager.write_to_session(&session_id_clone, data).await {
                        warn!("Failed to write to session: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client closed connection for session: {}", session_id_clone);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let _ = manager.touch_session(&session_id_clone).await;
                    // Respond with pong
                    // Note: ws_sender is moved, so we can't use it here
                    // The ping/pong is handled automatically by tokio-tungstenite
                    debug!("Received ping: {:?}", data);
                }
                Ok(Message::Pong(_)) => {
                    let _ = manager.touch_session(&session_id_clone).await;
                    // Ignore pong
                }
                Ok(Message::Frame(_)) => {
                    // Ignore raw frames
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Cleanup
        output_task.abort();
        // Do not stop session here, let the manager handle it (persistent sessions)
        info!("WebSocket connection ended for session: {}", session_id);

        Ok(())
    }

    /// Get session manager
    pub fn manager(&self) -> &Arc<TerminalSessionManager> {
        &self.manager
    }
}
