use serde_json::{json, Value};

use super::commands::methods;
use super::pipe::JugglerPipe;
use crate::adapter::traits::{BrowserError, TabId, TabInfo};

/// Manages Juggler sessions (one per page/tab)
pub struct JugglerSession {
    pipe: JugglerPipe,
    sessions: std::collections::HashMap<String, String>, // tab_id -> session_id
}

impl JugglerSession {
    pub fn new(pipe: JugglerPipe) -> Self {
        Self {
            pipe,
            sessions: std::collections::HashMap::new(),
        }
    }

    pub fn pipe(&self) -> &JugglerPipe {
        &self.pipe
    }

    pub fn is_connected(&self) -> bool {
        self.pipe.is_connected()
    }

    pub fn disconnect(&self) {
        self.pipe.disconnect();
    }

    /// Attach to a target and store the session ID
    pub async fn attach(&mut self, target_id: &str) -> Result<String, BrowserError> {
        let result = self
            .pipe
            .send_command(
                methods::TARGET_ATTACH,
                Some(json!({ "targetId": target_id })),
                None,
            )
            .await?;

        let session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or(target_id)
            .to_string();

        self.sessions.insert(target_id.to_string(), session_id.clone());
        Ok(session_id)
    }

    /// Get session ID for a tab, attaching if necessary
    pub async fn get_session(&mut self, tab_id: &str) -> Result<String, BrowserError> {
        if let Some(session) = self.sessions.get(tab_id) {
            return Ok(session.clone());
        }
        self.attach(tab_id).await
    }

    /// Send a command to a specific tab's session
    pub async fn send_to_tab(
        &mut self,
        tab_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, BrowserError> {
        let session_id = self.get_session(tab_id).await?;
        self.pipe.send_command(method, params, Some(session_id)).await
    }

    /// List all browser targets (pages)
    pub async fn list_targets(&self) -> Result<Vec<TabInfo>, BrowserError> {
        let result = self
            .pipe
            .send_command(methods::TARGET_GET_TARGETS, None, None)
            .await?;

        let targets = result
            .get("targets")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let tabs = targets
            .iter()
            .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
            .map(|t| TabInfo {
                id: TabId(
                    t.get("targetId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                ),
                url: t.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                title: t.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                is_active: false,
            })
            .collect();

        Ok(tabs)
    }

    /// Create a new page/tab
    pub async fn new_page(&mut self, url: Option<&str>) -> Result<TabId, BrowserError> {
        let params = url.map(|u| json!({ "url": u }));
        let result = self
            .pipe
            .send_command(methods::TARGET_NEW_PAGE, params, None)
            .await?;

        let target_id = result
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or(BrowserError::ProtocolError("No targetId in response".to_string()))?
            .to_string();

        Ok(TabId(target_id))
    }

    /// Close a page/tab
    pub async fn close_page(&mut self, tab_id: &str) -> Result<(), BrowserError> {
        self.pipe
            .send_command(
                methods::TARGET_CLOSE_PAGE,
                Some(json!({ "targetId": tab_id })),
                None,
            )
            .await?;
        self.sessions.remove(tab_id);
        Ok(())
    }
}
