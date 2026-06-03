use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Manager};

use super::browser_shell_frame_compactor::summarize_browser_shell_frames;
use crate::commands::traffic::TrafficAnalysisState;
use sentinel_tools::buildin_tools::browser_shell::{
    BrowserShellAction, BrowserShellArgs, BrowserShellError, BrowserShellHandler,
};

#[derive(Clone)]
pub struct AppBrowserShellHandler {
    app_handle: AppHandle,
}

impl AppBrowserShellHandler {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    fn traffic_state(&self) -> Result<tauri::State<'_, TrafficAnalysisState>, BrowserShellError> {
        self.app_handle
            .try_state::<TrafficAnalysisState>()
            .ok_or_else(|| {
                BrowserShellError::RequestFailed("TrafficAnalysisState not initialized".to_string())
            })
    }
}

#[async_trait::async_trait]
impl BrowserShellHandler for AppBrowserShellHandler {
    async fn handle(&self, args: BrowserShellArgs) -> Result<serde_json::Value, BrowserShellError> {
        let traffic_state = self.traffic_state()?;
        let store = traffic_state.get_browser_shell_store();

        match args.action {
            BrowserShellAction::ListSessions => {
                let sessions = store.read().await.list_sessions();
                Ok(json!({ "sessions": sessions }))
            }
            BrowserShellAction::ReadFrames => {
                let session_id = args
                    .session_id
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let frames = store
                    .read()
                    .await
                    .list_frames(&session_id, args.limit.unwrap_or(50));
                let summary = summarize_browser_shell_frames(&session_id, &frames);
                Ok(json!({
                    "session_id": session_id,
                    "inspected_frame_count": summary.inspected_frame_count,
                    "recent_inputs": summary.recent_inputs,
                    "terminal_text": summary.terminal_text,
                    "terminal_text_truncated": summary.terminal_text_truncated,
                    "prompt_detected": summary.prompt_detected,
                    "last_received_at": summary.last_received_at,
                }))
            }
            BrowserShellAction::QueueWrite => {
                let request = store
                    .write()
                    .await
                    .enqueue_write(
                        args.session_id.as_deref().unwrap_or_default(),
                        args.input_text.as_deref().unwrap_or_default(),
                        args.requires_approval,
                    )
                    .map_err(BrowserShellError::RequestFailed)?;
                Ok(json!({ "request": request }))
            }
            BrowserShellAction::ListWriteRequests => {
                let requests = store.read().await.list_write_requests();
                Ok(json!({ "requests": requests }))
            }
            BrowserShellAction::RespondWrite => {
                let request = store
                    .write()
                    .await
                    .respond_write_request(
                        args.request_id.as_deref().unwrap_or_default(),
                        args.allowed.unwrap_or(false),
                    )
                    .map_err(BrowserShellError::RequestFailed)?;
                Ok(json!({ "request": request }))
            }
        }
    }
}

pub fn build_browser_shell_handler(app_handle: AppHandle) -> Arc<dyn BrowserShellHandler> {
    Arc::new(AppBrowserShellHandler::new(app_handle))
}
