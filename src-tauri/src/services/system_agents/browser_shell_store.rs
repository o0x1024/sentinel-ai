use std::collections::{HashMap, VecDeque};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};

use super::browser_shell_models::{
    BrowserShellCommandAckInput, BrowserShellFrame, BrowserShellFrameInput, BrowserShellSession,
    BrowserShellSessionRemoveInput, BrowserShellSessionUpsertInput, BrowserShellWriteRequest,
    BrowserShellWriteStatus,
};

const MAX_FRAMES_PER_SESSION: usize = 256;
const MAX_WRITE_REQUESTS: usize = 256;

#[derive(Debug, Clone, Default)]
pub struct BrowserShellStore {
    sessions: HashMap<String, BrowserShellSession>,
    frames: HashMap<String, VecDeque<BrowserShellFrame>>,
    write_requests: HashMap<String, BrowserShellWriteRequest>,
}

impl BrowserShellStore {
    pub fn upsert_session(
        &mut self,
        input: BrowserShellSessionUpsertInput,
    ) -> Result<BrowserShellSession, String> {
        let session_id = input.id.trim();
        if session_id.is_empty() {
            return Err("Browser shell session id is required".to_string());
        }
        let page_url = input.page_url.trim();
        if page_url.is_empty() {
            return Err("Browser shell page_url is required".to_string());
        }
        let ws_url = input.ws_url.trim();
        if ws_url.is_empty() {
            return Err("Browser shell ws_url is required".to_string());
        }

        let seen_at =
            parse_optional_timestamp(input.occurred_at.as_deref()).unwrap_or_else(Utc::now);
        let current = self.sessions.get(session_id).cloned();
        let session = BrowserShellSession {
            id: session_id.to_string(),
            tab_id: input
                .tab_id
                .or_else(|| current.as_ref().and_then(|item| item.tab_id)),
            frame_id: input
                .frame_id
                .or_else(|| current.as_ref().and_then(|item| item.frame_id)),
            page_url: page_url.to_string(),
            page_title: input
                .page_title
                .or_else(|| current.as_ref().and_then(|item| item.page_title.clone())),
            ws_url: ws_url.to_string(),
            protocol: input
                .protocol
                .or_else(|| current.as_ref().and_then(|item| item.protocol.clone())),
            terminal_kind: input
                .terminal_kind
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    current
                        .as_ref()
                        .map(|item| item.terminal_kind.clone())
                        .unwrap_or_else(|| "websocket_shell".to_string())
                }),
            writable: input
                .writable
                .unwrap_or_else(|| current.as_ref().map(|item| item.writable).unwrap_or(true)),
            connected: input
                .connected
                .unwrap_or_else(|| current.as_ref().map(|item| item.connected).unwrap_or(true)),
            last_seen_at: seen_at,
        };
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    pub fn record_frame(
        &mut self,
        input: BrowserShellFrameInput,
    ) -> Result<BrowserShellFrame, String> {
        let session_id = input.session_id.trim();
        if session_id.is_empty() {
            return Err("Browser shell frame session_id is required".to_string());
        }
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Browser shell session '{}' not found", session_id))?;

        let received_at =
            parse_optional_timestamp(input.occurred_at.as_deref()).unwrap_or_else(Utc::now);
        session.last_seen_at = received_at;
        session.connected = true;

        let frame = BrowserShellFrame {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            direction: normalize_small_text(&input.direction),
            frame_type: normalize_small_text(&input.frame_type),
            text_preview: input.text_preview.map(|value| trim_preview(&value)),
            payload_base64: input.payload_base64.map(|value| value.trim().to_string()),
            received_at,
        };

        let entry = self.frames.entry(session_id.to_string()).or_default();
        entry.push_back(frame.clone());
        if entry.len() > MAX_FRAMES_PER_SESSION {
            let overflow = entry.len() - MAX_FRAMES_PER_SESSION;
            entry.drain(0..overflow);
        }
        Ok(frame)
    }

    pub fn remove_session(
        &mut self,
        input: BrowserShellSessionRemoveInput,
    ) -> Result<bool, String> {
        let session_id = input.id.trim();
        if session_id.is_empty() {
            return Err("Browser shell session id is required".to_string());
        }

        let removed = self.sessions.remove(session_id).is_some();
        self.frames.remove(session_id);
        self.write_requests
            .retain(|_, request| request.session_id != session_id);
        Ok(removed)
    }

    pub fn list_sessions(&self) -> Vec<BrowserShellSession> {
        let mut sessions = self.sessions.values().cloned().collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.last_seen_at.cmp(&left.last_seen_at));
        sessions
    }

    pub fn list_frames(&self, session_id: &str, limit: usize) -> Vec<BrowserShellFrame> {
        let normalized_limit = limit.max(1).min(MAX_FRAMES_PER_SESSION);
        let Some(frames) = self.frames.get(session_id.trim()) else {
            return Vec::new();
        };
        let mut items = frames
            .iter()
            .rev()
            .take(normalized_limit)
            .cloned()
            .collect::<Vec<_>>();
        items.reverse();
        items
    }

    pub fn list_write_requests(&self) -> Vec<BrowserShellWriteRequest> {
        let mut items = self.write_requests.values().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        items
    }

    pub fn enqueue_write(
        &mut self,
        session_id: &str,
        input_text: &str,
        requires_approval: bool,
    ) -> Result<BrowserShellWriteRequest, String> {
        let normalized_session_id = session_id.trim();
        if normalized_session_id.is_empty() {
            return Err("Browser shell write session_id is required".to_string());
        }
        if !self.sessions.contains_key(normalized_session_id) {
            return Err(format!(
                "Browser shell session '{}' not found",
                normalized_session_id
            ));
        }

        let normalized_input = input_text.to_string();
        if normalized_input.is_empty() {
            return Err("Browser shell write input_text is required".to_string());
        }

        let now = Utc::now();
        let write = BrowserShellWriteRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id: normalized_session_id.to_string(),
            input_base64: BASE64_STANDARD.encode(normalized_input.as_bytes()),
            input_text: normalized_input,
            requires_approval,
            status: if requires_approval {
                BrowserShellWriteStatus::PendingApproval
            } else {
                BrowserShellWriteStatus::Queued
            },
            error: None,
            created_at: now,
            updated_at: now,
        };
        self.write_requests
            .insert(write.request_id.clone(), write.clone());
        self.trim_write_requests();
        Ok(write)
    }

    pub fn respond_write_request(
        &mut self,
        request_id: &str,
        allowed: bool,
    ) -> Result<BrowserShellWriteRequest, String> {
        let request = self
            .write_requests
            .get_mut(request_id.trim())
            .ok_or_else(|| format!("Browser shell write request '{}' not found", request_id))?;
        if request.status != BrowserShellWriteStatus::PendingApproval {
            return Err(format!(
                "Browser shell write request '{}' is not awaiting approval",
                request_id
            ));
        }

        request.status = if allowed {
            BrowserShellWriteStatus::Queued
        } else {
            BrowserShellWriteStatus::Rejected
        };
        request.updated_at = Utc::now();
        if !allowed {
            request.error = Some("Write request rejected".to_string());
        }
        Ok(request.clone())
    }

    pub fn poll_queued_writes(
        &mut self,
        session_ids: &[String],
        limit: usize,
    ) -> Vec<BrowserShellWriteRequest> {
        let normalized_limit = limit.max(1).min(MAX_WRITE_REQUESTS);
        let session_lookup = session_ids
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        let mut request_ids = self
            .write_requests
            .values()
            .filter(|item| {
                item.status == BrowserShellWriteStatus::Queued
                    && session_lookup
                        .iter()
                        .any(|session_id| *session_id == item.session_id.as_str())
            })
            .map(|item| (item.request_id.clone(), item.created_at))
            .collect::<Vec<_>>();
        request_ids.sort_by(|left, right| left.1.cmp(&right.1));

        let dispatch_started_at = Utc::now();
        request_ids
            .into_iter()
            .take(normalized_limit)
            .filter_map(|(request_id, _)| {
                let request = self.write_requests.get_mut(&request_id)?;
                request.status = BrowserShellWriteStatus::Dispatching;
                request.updated_at = dispatch_started_at;
                request.error = None;
                Some(request.clone())
            })
            .collect()
    }

    pub fn ack_write(
        &mut self,
        input: BrowserShellCommandAckInput,
    ) -> Result<BrowserShellWriteRequest, String> {
        let request = self
            .write_requests
            .get_mut(input.request_id.trim())
            .ok_or_else(|| {
                format!(
                    "Browser shell write request '{}' not found",
                    input.request_id.trim()
                )
            })?;
        if request.session_id != input.session_id.trim() {
            return Err("Browser shell write request session mismatch".to_string());
        }
        if request.status != BrowserShellWriteStatus::Dispatching {
            return Err(format!(
                "Browser shell write request '{}' is not currently dispatching",
                request.request_id
            ));
        }
        request.status = if input.success {
            BrowserShellWriteStatus::Delivered
        } else {
            BrowserShellWriteStatus::Failed
        };
        request.error = input.error.map(|value| value.trim().to_string());
        request.updated_at = Utc::now();
        Ok(request.clone())
    }

    fn trim_write_requests(&mut self) {
        if self.write_requests.len() <= MAX_WRITE_REQUESTS {
            return;
        }
        let mut items = self
            .write_requests
            .values()
            .map(|item| (item.request_id.clone(), item.updated_at))
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.1.cmp(&right.1));
        let overflow = self.write_requests.len() - MAX_WRITE_REQUESTS;
        for (request_id, _) in items.into_iter().take(overflow) {
            self.write_requests.remove(&request_id);
        }
    }
}

fn parse_optional_timestamp(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn normalize_small_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    trimmed.to_ascii_lowercase()
}

fn trim_preview(raw: &str) -> String {
    let normalized = raw
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "");
    if normalized.chars().count() > 4096 {
        normalized.chars().take(4096).collect()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::system_agents::BrowserShellWriteStatus;

    fn build_session_input() -> BrowserShellSessionUpsertInput {
        BrowserShellSessionUpsertInput {
            id: "session-1".to_string(),
            tab_id: Some(1),
            frame_id: Some(0),
            page_url: "https://example.com/terminal".to_string(),
            page_title: Some("Terminal".to_string()),
            ws_url: "wss://example.com/terminal".to_string(),
            protocol: Some("terminal".to_string()),
            terminal_kind: Some("websocket_shell".to_string()),
            writable: Some(true),
            connected: Some(true),
            occurred_at: None,
        }
    }

    #[test]
    fn queue_write_requires_existing_session() {
        let mut store = BrowserShellStore::default();
        let error = store
            .enqueue_write("missing", "ls\n", true)
            .expect_err("write should require an existing session");
        assert!(error.contains("not found"));
    }

    #[test]
    fn browser_shell_lifecycle_flows_from_pending_to_delivered() {
        let mut store = BrowserShellStore::default();
        store
            .upsert_session(build_session_input())
            .expect("session should be created");

        let frame = store
            .record_frame(BrowserShellFrameInput {
                session_id: "session-1".to_string(),
                direction: "in".to_string(),
                frame_type: "text".to_string(),
                text_preview: Some("$ ".to_string()),
                payload_base64: None,
                occurred_at: None,
            })
            .expect("frame should record");
        assert_eq!(frame.session_id, "session-1");

        let queued = store
            .enqueue_write("session-1", "whoami\n", true)
            .expect("write should queue");
        assert_eq!(queued.status, BrowserShellWriteStatus::PendingApproval);

        let approved = store
            .respond_write_request(&queued.request_id, true)
            .expect("approval should succeed");
        assert_eq!(approved.status, BrowserShellWriteStatus::Queued);

        let pending = store.poll_queued_writes(&["session-1".to_string()], 10);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, queued.request_id);
        assert_eq!(pending[0].status, BrowserShellWriteStatus::Dispatching);

        let delivered = store
            .ack_write(BrowserShellCommandAckInput {
                request_id: queued.request_id.clone(),
                session_id: "session-1".to_string(),
                success: true,
                error: None,
            })
            .expect("ack should succeed");
        assert_eq!(delivered.status, BrowserShellWriteStatus::Delivered);
    }

    #[test]
    fn trim_preview_preserves_multiline_terminal_output() {
        let preview = trim_preview("line-1\r\nline-2\rprompt$ ");
        assert_eq!(preview, "line-1\nline-2\nprompt$ ");
    }

    #[test]
    fn remove_session_purges_frames_and_write_requests() {
        let mut store = BrowserShellStore::default();
        let session = store.upsert_session(build_session_input()).unwrap();

        store
            .record_frame(BrowserShellFrameInput {
                session_id: session.id.clone(),
                direction: "in".to_string(),
                frame_type: "text".to_string(),
                text_preview: Some("prompt$".to_string()),
                payload_base64: None,
                occurred_at: None,
            })
            .unwrap();
        store.enqueue_write(&session.id, "pwd", false).unwrap();

        let removed = store
            .remove_session(BrowserShellSessionRemoveInput {
                id: session.id.clone(),
            })
            .unwrap();

        assert!(removed);
        assert!(store.list_sessions().is_empty());
        assert!(store.list_frames(&session.id, 10).is_empty());
        assert!(store.list_write_requests().is_empty());
    }
}
