//! MessageEmitter implementation using tauri::AppHandle

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;

/// Message emitter trait for sending events
#[async_trait]
pub trait MessageEmitter: Send + Sync {
    /// Emit an event with payload
    async fn emit(&self, event: &str, payload: Value) -> Result<()>;
    
    /// Emit an event to all listeners with payload
    async fn emit_all(&self, event: &str, payload: Value) -> Result<()>;
}

/// Tauri-based message emitter implementation
#[derive(Clone)]
pub struct TauriMessageEmitter {
    app_handle: Arc<AppHandle>,
}

impl TauriMessageEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app_handle),
        }
    }
}

#[async_trait]
impl MessageEmitter for TauriMessageEmitter {
    async fn emit(&self, event: &str, payload: Value) -> Result<()> {
        self.app_handle.emit(event, payload)?;
        Ok(())
    }
    
    async fn emit_all(&self, event: &str, payload: Value) -> Result<()> {
        self.app_handle.emit(event, payload)?;
        Ok(())
    }
}
