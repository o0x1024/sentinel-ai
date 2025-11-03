//! MessageEmitter implementation using tauri::AppHandle

use sentinel_engines::MessageEmitter;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;

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

