use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{broadcast, Mutex};
use tracing::info;

use super::commands::Cdp;
use super::connection::CdpConnection;
use super::events::CdpEvent;

/// Information about a browser tab/target
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_id: String,
    pub session_id: Option<String>,
    pub url: String,
    pub title: String,
    pub target_type: String,
    pub attached: bool,
}

/// Manages multiple tab sessions over a single CDP connection
pub struct CdpSessionManager {
    connection: Arc<CdpConnection>,
    targets: Arc<Mutex<HashMap<String, TargetInfo>>>,
}

impl CdpSessionManager {
    pub async fn new(ws_url: &str) -> Result<Self, String> {
        let connection = CdpConnection::connect(ws_url).await?;
        let connection = Arc::new(connection);

        let manager = Self {
            connection,
            targets: Arc::new(Mutex::new(HashMap::new())),
        };

        manager.init().await?;
        Ok(manager)
    }

    /// Initialize: discover existing targets and enable target tracking
    async fn init(&self) -> Result<(), String> {
        let (method, params) = Cdp::target_set_discover_targets(true);
        self.connection.send_command(method, Some(params), None).await?;

        self.refresh_targets().await?;
        Ok(())
    }

    /// Refresh the target list from the browser
    pub async fn refresh_targets(&self) -> Result<(), String> {
        let (method, params) = Cdp::target_get_targets();
        let resp = self.connection.send_command(method, Some(params), None).await?;

        let target_infos: Vec<RawTargetInfo> = serde_json::from_value(
            resp.result.get("targetInfos").cloned().unwrap_or(Value::Array(vec![])),
        )
        .unwrap_or_default();

        let mut targets = self.targets.lock().await;
        for info in target_infos {
            let existing_session = targets.get(&info.target_id).and_then(|t| t.session_id.clone());
            targets.insert(
                info.target_id.clone(),
                TargetInfo {
                    target_id: info.target_id,
                    session_id: existing_session,
                    url: info.url,
                    title: info.title,
                    target_type: info.r#type,
                    attached: info.attached,
                },
            );
        }

        Ok(())
    }

    /// Attach to a target and get a session ID for sending commands to that tab
    pub async fn attach_to_target(&self, target_id: &str) -> Result<String, String> {
        let (method, params) = Cdp::target_attach_to_target(target_id, true);
        let resp = self.connection.send_command(method, Some(params), None).await?;

        let session_id = resp
            .result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or("No sessionId in attach response")?
            .to_string();

        let mut targets = self.targets.lock().await;
        if let Some(target) = targets.get_mut(target_id) {
            target.session_id = Some(session_id.clone());
            target.attached = true;
        }

        // Enable required domains on this session
        self.enable_domains(&session_id).await?;

        info!("Attached to target {} with session {}", target_id, session_id);
        Ok(session_id)
    }

    /// Enable CDP domains needed for automation on a session
    async fn enable_domains(&self, session_id: &str) -> Result<(), String> {
        let domains = [
            Cdp::page_enable(),
            Cdp::dom_enable(),
            Cdp::runtime_enable(),
            Cdp::network_enable(),
            Cdp::accessibility_enable(),
        ];

        for (method, params) in domains {
            self.connection
                .send_command(method, Some(params), Some(session_id))
                .await?;
        }

        Ok(())
    }

    /// Create a new tab and attach to it
    pub async fn create_tab(&self, url: &str) -> Result<TargetInfo, String> {
        let (method, params) = Cdp::target_create_target(url);
        let resp = self.connection.send_command(method, Some(params), None).await?;

        let target_id = resp
            .result
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or("No targetId in create response")?
            .to_string();

        let session_id = self.attach_to_target(&target_id).await?;

        let info = TargetInfo {
            target_id: target_id.clone(),
            session_id: Some(session_id),
            url: url.to_string(),
            title: String::new(),
            target_type: "page".to_string(),
            attached: true,
        };

        let mut targets = self.targets.lock().await;
        targets.insert(target_id, info.clone());

        Ok(info)
    }

    /// Close a tab
    pub async fn close_tab(&self, target_id: &str) -> Result<(), String> {
        let (method, params) = Cdp::target_close_target(target_id);
        self.connection.send_command(method, Some(params), None).await?;

        let mut targets = self.targets.lock().await;
        targets.remove(target_id);

        Ok(())
    }

    /// Activate (bring to front) a tab
    pub async fn activate_tab(&self, target_id: &str) -> Result<(), String> {
        let (method, params) = Cdp::target_activate_target(target_id);
        self.connection.send_command(method, Some(params), None).await?;
        Ok(())
    }

    /// List all page-type targets
    pub async fn list_tabs(&self) -> Vec<TargetInfo> {
        let targets = self.targets.lock().await;
        targets
            .values()
            .filter(|t| t.target_type == "page")
            .cloned()
            .collect()
    }

    /// Get the session ID for a target (attaching if needed)
    pub async fn get_session_id(&self, target_id: &str) -> Result<String, String> {
        let targets = self.targets.lock().await;
        if let Some(target) = targets.get(target_id) {
            if let Some(ref sid) = target.session_id {
                return Ok(sid.clone());
            }
        }
        drop(targets);

        self.attach_to_target(target_id).await
    }

    /// Send a command to a specific tab by target_id
    pub async fn send_to_target(
        &self,
        target_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let session_id = self.get_session_id(target_id).await?;
        let resp = self
            .connection
            .send_command(method, params, Some(&session_id))
            .await?;
        Ok(resp.result)
    }

    /// Get the underlying connection for subscribing to events
    pub fn connection(&self) -> &Arc<CdpConnection> {
        &self.connection
    }

    /// Subscribe to events from the connection
    pub fn subscribe_events(&self) -> broadcast::Receiver<CdpEvent> {
        self.connection.subscribe_events()
    }
}

#[derive(Debug, Deserialize)]
struct RawTargetInfo {
    #[serde(rename = "targetId")]
    target_id: String,
    url: String,
    title: String,
    #[serde(rename = "type")]
    r#type: String,
    attached: bool,
}
