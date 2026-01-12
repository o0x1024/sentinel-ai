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

/// WebSocket terminal server
pub struct TerminalServer {
    manager: Arc<TerminalSessionManager>,
    addr: SocketAddr,
    running: Arc<RwLock<bool>>,
}

impl TerminalServer {
    /// Create a new terminal server
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            manager: Arc::new(TerminalSessionManager::new()),
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
                    if self.manager.get_session(&session_id).await.is_some() {
                        info!("Reconnecting to existing session: {}", session_id);
                        // For reconnection, we need to create a new output channel
                        // This is a limitation - we'll create a new session for now
                        return Err("Session reconnection not yet implemented".to_string());
                    } else {
                        return Err(format!("Session not found: {}", session_id));
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
            .send(Message::Text(format!("session:{}", session_id)))
            .await
            .map_err(|e| format!("Failed to send session ID: {}", e))?;

        // Spawn task to forward output to WebSocket
        let session_id_clone = session_id.clone();
        let output_task = tokio::spawn(async move {
            while let Some(data) = output_rx.recv().await {
                if let Err(e) = ws_sender.send(Message::Binary(data)).await {
                    error!("Failed to send output: {}", e);
                    break;
                }
            }
            debug!("Output task ended for session: {}", session_id_clone);
        });

        // Handle input from WebSocket
        let manager = self.manager.clone();
        let session_id_clone = session_id.clone();
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(e) = manager.write_to_session(&session_id_clone, data).await {
                        warn!("Failed to write to session: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    let data = text.into_bytes();
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
                    // Respond with pong
                    // Note: ws_sender is moved, so we can't use it here
                    // The ping/pong is handled automatically by tokio-tungstenite
                    debug!("Received ping: {:?}", data);
                }
                Ok(Message::Pong(_)) => {
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
        let _ = manager.stop_session(&session_id).await;

        Ok(())
    }

    /// Get session manager
    pub fn manager(&self) -> &Arc<TerminalSessionManager> {
        &self.manager
    }
}
