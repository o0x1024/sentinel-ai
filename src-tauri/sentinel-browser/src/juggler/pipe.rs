use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::HashMap;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::commands::{JugglerCommand, JugglerResponse};
use crate::adapter::traits::BrowserError;

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<JugglerResponse>>>>;

/// Pipe-based transport for Juggler protocol (fd3/fd4 or stdin/stdout).
/// Camoufox exposes automation over a JSON line protocol on stdio pipes.
pub struct JugglerPipe {
    sender: mpsc::Sender<JugglerCommand>,
    pending: PendingMap,
    _event_tx: mpsc::UnboundedSender<JugglerResponse>,
    next_id: AtomicU64,
    connected: AtomicBool,
}

impl JugglerPipe {
    /// Create a new JugglerPipe from a child process's stdin/stdout.
    /// `reader` should be the stdout of the browser process.
    /// `writer` should be the stdin of the browser process.
    pub fn new(
        reader: tokio::process::ChildStdout,
        writer: ChildStdin,
        event_tx: mpsc::UnboundedSender<JugglerResponse>,
    ) -> Self {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (sender, mut cmd_rx) = mpsc::channel::<JugglerCommand>(256);

        // Writer task: serialize commands and write to pipe
        let writer = Arc::new(Mutex::new(writer));
        let writer_clone = writer.clone();
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let mut line = serde_json::to_string(&cmd).unwrap_or_default();
                line.push('\n');
                let mut w = writer_clone.lock().await;
                if w.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                let _ = w.flush().await;
            }
        });

        // Reader task: read lines from pipe and dispatch responses
        let pending_clone = pending.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match buf_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(resp) = serde_json::from_str::<JugglerResponse>(trimmed) {
                            if resp.is_event() {
                                let _ = event_tx_clone.send(resp);
                            } else if let Some(id) = resp.id {
                                let mut pending = pending_clone.lock().await;
                                if let Some(tx) = pending.remove(&id) {
                                    let _ = tx.send(resp);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            sender,
            pending,
            _event_tx: event_tx,
            next_id: AtomicU64::new(1),
            connected: AtomicBool::new(true),
        }
    }

    /// Send a command and wait for its response
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<String>,
    ) -> Result<Value, BrowserError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(BrowserError::NotConnected);
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        let cmd = JugglerCommand {
            id,
            method: method.to_string(),
            params,
            session_id,
        };

        self.sender
            .send(cmd)
            .await
            .map_err(|_| BrowserError::ConnectionFailed("Pipe closed".to_string()))?;

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx,
        )
        .await
        .map_err(|_| BrowserError::Timeout(format!("Command '{}' timed out", method)))?
        .map_err(|_| BrowserError::ConnectionFailed("Response channel dropped".to_string()))?;

        if let Some(err) = resp.error {
            return Err(BrowserError::ProtocolError(err.message));
        }

        Ok(resp.result.unwrap_or(Value::Null))
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
    }
}
