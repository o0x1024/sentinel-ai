use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use base64::Engine as _;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationMessage {
    pub title: String,
    pub content: String,
}

fn read_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn read_u64(v: &Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| x.as_u64())
}

fn read_bool(v: &Value, key: &str) -> Option<bool> {
    v.get(key).and_then(|x| x.as_bool())
}

pub async fn send(channel: &str, config: Value, message: NotificationMessage) -> Result<()> {
    match channel {
        "webhook" => send_webhook(config, &message).await,
        "dingtalk" => send_dingtalk(config, &message).await,
        "feishu" => send_feishu(config, &message).await,
        "wecom" => send_wecom(config, &message).await,
        "email" => send_email(config, &message).await,
        other => Err(anyhow!("Unsupported channel: {}", other)),
    }
}

async fn send_webhook(config: Value, message: &NotificationMessage) -> Result<()> {
    let url = read_str(&config, "webhook_url")
        .or_else(|| read_str(&config, "url"))
        .ok_or_else(|| anyhow!("Missing webhook url"))?;
    let method = read_str(&config, "method").unwrap_or_else(|| "POST".to_string());
    // Apply global proxy configuration
    let builder = reqwest::Client::builder();
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());

    let body_template = read_str(&config, "body_template");
    let payload = if let Some(tpl) = body_template {
        match serde_json::from_str::<Value>(&tpl) {
            Ok(mut v) => {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("title".to_string(), Value::String(message.title.clone()));
                    obj.insert("content".to_string(), Value::String(message.content.clone()));
                }
                v
            }
            Err(_) => serde_json::json!({"title": message.title, "content": message.content}),
        }
    } else {
        serde_json::json!({"title": message.title, "content": message.content})
    };

    let headers_json = read_str(&config, "headers_json");
    let mut req = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.post(&url),
    };
    if let Some(h) = headers_json {
        if let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(&h) {
            for (k, v) in map.into_iter() {
                if let Some(s) = v.as_str() {
                    req = req.header(k, s);
                }
            }
        }
    }
    let res = req.json(&payload).send().await?;
    if res.status().is_success() { Ok(()) } else { Err(anyhow!("Webhook status: {}", res.status())) }
}

async fn send_dingtalk(config: Value, message: &NotificationMessage) -> Result<()> {
    let mut url = read_str(&config, "webhook_url").ok_or_else(|| anyhow!("Missing webhook_url"))?;
    if let Some(secret) = read_str(&config, "secret") {
        // DingTalk sign: timestamp + secret
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis().to_string();
        let string_to_sign = format!("{}\n{}", ts, secret);
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
        mac.update(string_to_sign.as_bytes());
        let sign = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        // append query
        let sep = if url.contains('?') { '&' } else { '?' };
        url = format!("{}{}timestamp={}&sign={}", url, sep, ts, urlencoding::encode(&sign));
    }
    // Apply global proxy configuration
    let builder = reqwest::Client::builder();
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
    let msg_type = read_str(&config, "message_type").unwrap_or_else(|| "text".to_string());
    let payload = if msg_type == "markdown" {
        let text = read_str(&config, "markdown_text").unwrap_or_else(|| format!("{}\n{}", message.title, message.content));
        serde_json::json!({
            "msgtype": "markdown",
            "markdown": { "title": message.title, "text": text }
        })
    } else if msg_type == "card" {
        if let Some(card_json) = read_str(&config, "card_payload_json") {
            serde_json::from_str::<Value>(&card_json).unwrap_or_else(|_| serde_json::json!({
                "msgtype": "text",
                "text": { "content": format!("{}\n{}", message.title, message.content) }
            }))
        } else {
            serde_json::json!({
                "msgtype": "text",
                "text": { "content": format!("{}\n{}", message.title, message.content) }
            })
        }
    } else {
        serde_json::json!({
            "msgtype": "text",
            "text": { "content": format!("{}\n{}", message.title, message.content) }
        })
    };
    let res = client.post(&url).json(&payload).send().await?;
    if res.status().is_success() { Ok(()) } else { Err(anyhow!("DingTalk status: {}", res.status())) }
}

async fn send_feishu(config: Value, message: &NotificationMessage) -> Result<()> {
    let url = read_str(&config, "webhook_url").ok_or_else(|| anyhow!("Missing webhook_url"))?;
    // Apply global proxy configuration
    let builder = reqwest::Client::builder();
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
    let msg_type = read_str(&config, "message_type").unwrap_or_else(|| "text".to_string());
    let payload = if msg_type == "markdown" {
        let text = read_str(&config, "markdown_text").unwrap_or_else(|| format!("{}\n{}", message.title, message.content));
        serde_json::json!({
            "msg_type": "post",
            "content": { 
                "post": {
                    "zh_cn": {
                        "title": message.title,
                        "content": [[{ "tag": "text", "text": text }]]
                    }
                }
            }
        })
    } else if msg_type == "card" {
        if let Some(card_json) = read_str(&config, "card_payload_json") {
            serde_json::from_str::<Value>(&card_json).unwrap_or_else(|_| serde_json::json!({
                "msg_type": "text",
                "content": { "text": format!("{}\n{}", message.title, message.content) }
            }))
        } else {
            serde_json::json!({
                "msg_type": "text",
                "content": { "text": format!("{}\n{}", message.title, message.content) }
            })
        }
    } else {
        serde_json::json!({
            "msg_type": "text",
            "content": { "text": format!("{}\n{}", message.title, message.content) }
        })
    };
    let res = client.post(&url).json(&payload).send().await?;
    if res.status().is_success() { Ok(()) } else { Err(anyhow!("Feishu status: {}", res.status())) }
}

async fn send_wecom(config: Value, message: &NotificationMessage) -> Result<()> {
    let url = read_str(&config, "webhook_url").ok_or_else(|| anyhow!("Missing webhook_url"))?;
    // Apply global proxy configuration
    let builder = reqwest::Client::builder();
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
    let msg_type = read_str(&config, "message_type").unwrap_or_else(|| "text".to_string());
    let payload = if msg_type == "markdown" {
        let text = read_str(&config, "markdown_text").unwrap_or_else(|| format!("{}\n{}", message.title, message.content));
        serde_json::json!({
            "msgtype": "markdown",
            "markdown": { "content": text }
        })
    } else if msg_type == "card" {
        if let Some(card_json) = read_str(&config, "card_payload_json") {
            serde_json::from_str::<Value>(&card_json).unwrap_or_else(|_| serde_json::json!({
                "msgtype": "text",
                "text": { "content": format!("{}\n{}", message.title, message.content) }
            }))
        } else {
            serde_json::json!({
                "msgtype": "text",
                "text": { "content": format!("{}\n{}", message.title, message.content) }
            })
        }
    } else {
        serde_json::json!({
            "msgtype": "text",
            "text": { "content": format!("{}\n{}", message.title, message.content) }
        })
    };
    let res = client.post(&url).json(&payload).send().await?;
    if res.status().is_success() { Ok(()) } else { Err(anyhow!("WeCom status: {}", res.status())) }
}

async fn send_email(config: Value, message: &NotificationMessage) -> Result<()> {
    let host = read_str(&config, "smtp_host").ok_or_else(|| anyhow!("Missing smtp_host"))?;
    let port = read_u64(&config, "smtp_port").unwrap_or(25) as u16;
    let enc = read_str(&config, "transport_encryption").unwrap_or_else(|| "TLS".to_string());
    let username = read_str(&config, "email_username");
    let password = read_str(&config, "email_password");
    let from = read_str(&config, "email_from").ok_or_else(|| anyhow!("Missing email_from"))?;
    let to = read_str(&config, "email_to").ok_or_else(|| anyhow!("Missing email_to"))?;

    let recipients: Vec<&str> = to.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if recipients.is_empty() { return Err(anyhow!("No recipients")); }

    let subject = &message.title;
    let body = &message.content;
    let is_html = read_bool(&config, "email_is_html").unwrap_or(false);
    let attachments = config.get("email_attachments").and_then(|v| v.as_array()).cloned();

    // Use blocking send in a spawn_blocking to avoid async transport complexity
    let host_clone = host.clone();
    let enc_clone = enc.clone();
    let username_clone = username.clone();
    let password_clone = password.clone();
    let from_clone = from.clone();
    let recipients_clone: Vec<String> = recipients.into_iter().map(|s| s.to_string()).collect();
    let subject_clone = subject.clone();
    let body_clone = body.clone();
    let is_html_clone = is_html;
    let attachments_clone = attachments.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        use lettre::{Message, SmtpTransport, Transport};
        use lettre::message::{SinglePart, MultiPart};
        use lettre::message::header::{ContentType, ContentDisposition};
        use lettre::transport::smtp::authentication::Credentials;

        let creds_opt = match (username_clone, password_clone) {
            (Some(u), Some(p)) if !u.is_empty() => Some(Credentials::new(u, p)),
            _ => None,
        };

        let mut builder = if enc_clone == "NONE" {
            SmtpTransport::builder_dangerous(&host_clone)
        } else {
            // Treat TLS/SSL uniformly using relay builder (STARTTLS)
            SmtpTransport::relay(&host_clone).map_err(|e| anyhow!("smtp relay error: {}", e))?
        };
        builder = builder.port(port);
        if let Some(creds) = creds_opt { builder = builder.credentials(creds); }
        let mailer = builder.build();

        for rcpt in recipients_clone.iter() {
            let base_part = if is_html_clone { SinglePart::html(body_clone.clone()) } else { SinglePart::plain(body_clone.clone()) };
            let mut mixed = MultiPart::mixed().singlepart(base_part);
            if let Some(atts) = &attachments_clone {
                for att in atts.iter() {
                    if let Some(obj) = att.as_object() {
                        let filename = obj.get("filename").and_then(|x| x.as_str()).unwrap_or("attachment").to_string();
                        let content_b64 = obj.get("content_base64").and_then(|x| x.as_str()).unwrap_or("");
                        let content_type_str = obj.get("content_type").and_then(|x| x.as_str()).unwrap_or("application/octet-stream");
                        if !content_b64.is_empty() {
                            let bytes = base64::engine::general_purpose::STANDARD.decode(content_b64).map_err(|e| anyhow!("attachment base64 decode error: {}", e))?;
                            let ct: ContentType = content_type_str.parse().unwrap_or("application/octet-stream".parse().unwrap());
                            let cd = ContentDisposition::attachment(&filename);
                            let part = SinglePart::builder().header(ct).header(cd).body(bytes);
                            mixed = mixed.singlepart(part);
                        }
                    }
                }
            }
            let email = Message::builder()
                .from(from_clone.parse().map_err(|e| anyhow!("bad from: {}", e))?)
                .to(rcpt.parse().map_err(|e| anyhow!("bad to: {}", e))?)
                .subject(subject_clone.clone())
                .multipart(mixed)
                .map_err(|e| anyhow!("build email error: {}", e))?;
            let response = mailer.send(&email).map_err(|e| anyhow!("smtp send error: {}", e))?;
            if !response.is_positive() {
                return Err(anyhow!("smtp negative response"));
            }
        }
        Ok(())
    }).await??;

    Ok(())
}
