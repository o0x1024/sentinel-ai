use super::*;

pub(super) struct InboundWeixinMessage {
    pub(super) message_id: String,
    pub(super) peer_type: String,
    pub(super) peer_id: String,
    pub(super) sender_id: String,
    pub(super) text: String,
    pub(super) context_token: Option<String>,
}

pub(super) fn parse_inbound_message(
    config: &WeixinGatewayConfig,
    message: &Value,
    message_id: String,
) -> Option<InboundWeixinMessage> {
    let text = extract_text(message.get("item_list").and_then(Value::as_array)?)?;
    if text.trim().is_empty() {
        return None;
    }
    let room_id = string_field(message, "room_id")
        .or_else(|| string_field(message, "chat_room_id"))
        .unwrap_or_default();
    let from_user_id = string_field(message, "from_user_id")?;
    let to_user_id = string_field(message, "to_user_id").unwrap_or_default();
    let is_group = !room_id.is_empty()
        || (!to_user_id.is_empty()
            && to_user_id != config.account_id
            && message.get("msg_type").and_then(Value::as_i64) == Some(1));
    let peer_id = if is_group {
        if room_id.is_empty() {
            to_user_id
        } else {
            room_id
        }
    } else {
        from_user_id.clone()
    };

    Some(InboundWeixinMessage {
        message_id,
        peer_type: if is_group { "group" } else { "dm" }.to_string(),
        peer_id,
        sender_id: from_user_id,
        text,
        context_token: string_field(message, "context_token"),
    })
}

fn extract_text(items: &[Value]) -> Option<String> {
    for item in items {
        if item.get("type").and_then(Value::as_i64) == Some(1) {
            return item
                .get("text_item")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }
    for item in items {
        if item.get("type").and_then(Value::as_i64) == Some(3) {
            if let Some(text) = item
                .get("voice_item")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
            {
                return Some(text.to_string());
            }
        }
    }
    None
}

pub(super) fn is_self_message(config: &WeixinGatewayConfig, message: &Value) -> bool {
    string_field(message, "from_user_id").as_deref() == Some(config.account_id.as_str())
}

pub(super) fn is_authorized(config: &WeixinGatewayConfig, message: &InboundWeixinMessage) -> bool {
    if message.peer_type == "group" {
        if config.group_policy == "open" {
            return true;
        }
        return config.group_policy == "allowlist"
            && config
                .group_allowed_users
                .iter()
                .any(|value| value == &message.peer_id || value == &message.sender_id);
    }
    if config.dm_policy == "open" {
        return true;
    }
    config.dm_policy == "allowlist"
        && config
            .allowed_users
            .iter()
            .any(|value| value == &message.sender_id || value == &message.peer_id)
}

pub(super) fn session_id(config: &WeixinGatewayConfig, inbound: &InboundWeixinMessage) -> String {
    session_id_for_peer(config, &inbound.peer_type, &inbound.peer_id)
}

pub(super) fn execution_lock_key_for_inbound(
    config: &WeixinGatewayConfig,
    inbound: &InboundWeixinMessage,
) -> String {
    session_id_for_peer(config, &inbound.peer_type, &inbound.peer_id)
}

pub(super) fn session_id_for_peer(
    config: &WeixinGatewayConfig,
    peer_type: &str,
    peer_id: &str,
) -> String {
    format!("weixin:{}:{}:{}", config.account_id, peer_type, peer_id)
}

pub(super) fn message_identity(message: &Value) -> String {
    string_field(message, "msg_id")
        .or_else(|| string_field(message, "message_id"))
        .unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                string_field(message, "from_user_id").unwrap_or_default(),
                message
                    .get("create_time")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                string_field(message, "client_id").unwrap_or_default()
            )
        })
}

fn string_field(message: &Value, key: &str) -> Option<String> {
    message
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn trim_seen_messages(seen_messages: &mut HashSet<String>) {
    if seen_messages.len() <= 512 {
        return;
    }
    seen_messages.clear();
}

pub(super) fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if current.len() + line.len() + 1 > max_len && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }
        if line.len() > max_len {
            for part in line.as_bytes().chunks(max_len) {
                chunks.push(String::from_utf8_lossy(part).to_string());
            }
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}
