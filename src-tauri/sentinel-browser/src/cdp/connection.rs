use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, warn};

use super::events::CdpEvent;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Serialize)]
struct CdpRequest {
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CdpRawResponse {
    id: Option<u64>,
    result: Option<Value>,
    error: Option<CdpResponseError>,
    method: Option<String>,
    params: Option<Value>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CdpResponseError {
    pub code: i64,
    pub message: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CdpResponse {
    pub result: Value,
}

type PendingRequest = oneshot::Sender<Result<CdpResponse, CdpResponseError>>;

/// Manages a WebSocket connection to Chrome's CDP endpoint.
/// Multiplexes commands/responses across multiple sessions (tabs).
pub struct CdpConnection {
    next_id: AtomicU64,
    command_tx: mpsc::Sender<CdpRequest>,
    pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    event_tx: broadcast::Sender<CdpEvent>,
    _reader_handle: tokio::task::JoinHandle<()>,
    _writer_handle: tokio::task::JoinHandle<()>,
}

impl CdpConnection {
    /// Connect to a CDP WebSocket endpoint (e.g. ws://127.0.0.1:9222/devtools/browser/...)
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("WebSocket connection failed to {}: {}", ws_url, e))?;

        debug!("CDP connected to {}", ws_url);

        let (write_half, read_half) = ws_stream.split();
        let pending: Arc<Mutex<HashMap<u64, PendingRequest>>> = Arc::new(Mutex::new(HashMap::new()));
        let (event_tx, _) = broadcast::channel(256);
        let (command_tx, command_rx) = mpsc::channel::<CdpRequest>(64);

        let writer_handle = tokio::spawn(Self::writer_loop(command_rx, write_half));
        let reader_handle = tokio::spawn(Self::reader_loop(
            read_half,
            pending.clone(),
            event_tx.clone(),
        ));

        Ok(Self {
            next_id: AtomicU64::new(1),
            command_tx,
            pending,
            event_tx,
            _reader_handle: reader_handle,
            _writer_handle: writer_handle,
        })
    }

    /// Send a CDP command and await its response
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<CdpResponse, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        let request = CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id: session_id.map(|s| s.to_string()),
        };

        self.command_tx
            .send(request)
            .await
            .map_err(|e| format!("Failed to send command: {}", e))?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(err))) => Err(format!("CDP error {}: {}", err.code, err.message)),
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(format!("Command '{}' timed out after 30s", method))
            }
        }
    }

    /// Subscribe to CDP events
    pub fn subscribe_events(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    async fn writer_loop(
        mut command_rx: mpsc::Receiver<CdpRequest>,
        mut write_half: futures_util::stream::SplitSink<WsStream, Message>,
    ) {
        while let Some(request) = command_rx.recv().await {
            let json = match serde_json::to_string(&request) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize CDP request: {}", e);
                    continue;
                }
            };
            debug!("CDP >> {}", json);
            if let Err(e) = write_half.send(Message::Text(json)).await {
                error!("WebSocket write error: {}", e);
                break;
            }
        }
    }

    async fn reader_loop(
        mut read_half: futures_util::stream::SplitStream<WsStream>,
        pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
        event_tx: broadcast::Sender<CdpEvent>,
    ) {
        while let Some(msg) = read_half.next().await {
            let text = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) => {
                    debug!("CDP WebSocket closed by server");
                    break;
                }
                Ok(_) => continue,
                Err(e) => {
                    error!("WebSocket read error: {}", e);
                    break;
                }
            };

            debug!("CDP << {}", &text[..text.len().min(200)]);

            let raw: CdpRawResponse = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to parse CDP message: {}", e);
                    continue;
                }
            };

            // Command response (has `id`)
            if let Some(id) = raw.id {
                let mut pending = pending.lock().await;
                if let Some(tx) = pending.remove(&id) {
                    let result = match raw.error {
                        Some(err) => Err(err),
                        None => Ok(CdpResponse {
                            result: raw.result.unwrap_or(Value::Null),
                        }),
                    };
                    let _ = tx.send(result);
                }
            }
            // Event (has `method` but no `id`)
            else if let Some(method) = raw.method {
                let event = CdpEvent {
                    method,
                    params: raw.params.unwrap_or(Value::Null),
                    session_id: raw.session_id,
                };
                let _ = event_tx.send(event);
            }
        }
    }
}
