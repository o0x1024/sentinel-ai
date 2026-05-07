use super::config::{
    WeixinQrLoginCredentials, WeixinQrLoginResponse, WeixinQrLoginStatus, DEFAULT_ILINK_BASE_URL,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

const ILINK_APP_ID: &str = "bot";
const CHANNEL_VERSION: &str = "2.2.0";
const ILINK_APP_CLIENT_VERSION: u32 = (2 << 16) | (2 << 8);
const EP_GET_UPDATES: &str = "ilink/bot/getupdates";
const EP_SEND_MESSAGE: &str = "ilink/bot/sendmessage";
const EP_GET_BOT_QR: &str = "ilink/bot/get_bot_qrcode";
const EP_GET_QR_STATUS: &str = "ilink/bot/get_qrcode_status";
const MSG_TYPE_BOT: i64 = 2;
const MSG_STATE_FINISH: i64 = 2;
const ITEM_TEXT: i64 = 1;

#[derive(Clone)]
pub struct WeixinIlinkClient {
    client: Client,
}

impl WeixinIlinkClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| format!("Failed to build Weixin HTTP client: {e}"))?;
        Ok(Self { client })
    }

    pub async fn create_qr_login(&self, bot_type: &str) -> Result<WeixinQrLoginResponse, String> {
        let endpoint = format!("{EP_GET_BOT_QR}?bot_type={}", urlencoding::encode(bot_type));
        let response = self.api_get(DEFAULT_ILINK_BASE_URL, &endpoint).await?;
        let qrcode = value_str(&response, "qrcode");
        if qrcode.is_empty() {
            return Err("Weixin QR response missing qrcode".to_string());
        }
        let qrcode_img_content = value_str(&response, "qrcode_img_content");
        let scan_data = if qrcode_img_content.is_empty() {
            qrcode.clone()
        } else {
            qrcode_img_content.clone()
        };
        Ok(WeixinQrLoginResponse {
            qrcode,
            qrcode_img_content,
            scan_data,
        })
    }

    pub async fn poll_qr_login(
        &self,
        qrcode: &str,
        base_url: Option<&str>,
    ) -> Result<WeixinQrLoginStatus, String> {
        let (status, _) = self
            .poll_qr_login_with_credentials(qrcode, base_url)
            .await?;
        Ok(status)
    }

    pub async fn poll_qr_login_with_credentials(
        &self,
        qrcode: &str,
        base_url: Option<&str>,
    ) -> Result<(WeixinQrLoginStatus, Option<WeixinQrLoginCredentials>), String> {
        let base = base_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_ILINK_BASE_URL);
        let endpoint = format!("{EP_GET_QR_STATUS}?qrcode={}", urlencoding::encode(qrcode));
        let response = self.api_get(base, &endpoint).await?;
        let status = value_str(&response, "status");
        if status == "confirmed" {
            let account_id = value_str(&response, "ilink_bot_id");
            let token = value_str(&response, "bot_token");
            let base_url = value_str(&response, "baseurl");
            let status_response = WeixinQrLoginStatus {
                status: status.clone(),
                account_id: non_empty(account_id.clone()),
                token_configured: !token.is_empty(),
                base_url: non_empty(base_url.clone()),
                user_id: non_empty(value_str(&response, "ilink_user_id")),
                message: None,
            };
            let credentials = if account_id.is_empty() || token.is_empty() {
                None
            } else {
                Some(WeixinQrLoginCredentials {
                    account_id,
                    token,
                    base_url: if base_url.is_empty() {
                        DEFAULT_ILINK_BASE_URL.to_string()
                    } else {
                        base_url
                    },
                    user_id: status_response.user_id.clone(),
                })
            };
            return Ok((status_response, credentials));
        }

        Ok((
            WeixinQrLoginStatus {
                status: if status.is_empty() {
                    "wait".to_string()
                } else {
                    status
                },
                account_id: None,
                token_configured: false,
                base_url: non_empty(value_str(&response, "redirect_host").pipe(|host| {
                    if host.is_empty() {
                        host
                    } else {
                        format!("https://{host}")
                    }
                })),
                user_id: None,
                message: non_empty(value_str(&response, "errmsg")),
            },
            None,
        ))
    }

    pub async fn get_updates(
        &self,
        base_url: &str,
        token: &str,
        sync_buf: &str,
    ) -> Result<Value, String> {
        self.api_post(
            base_url,
            EP_GET_UPDATES,
            json!({ "get_updates_buf": sync_buf }),
            Some(token),
        )
        .await
    }

    pub async fn send_text(
        &self,
        base_url: &str,
        token: &str,
        to_user_id: &str,
        text: &str,
        context_token: Option<&str>,
    ) -> Result<(), String> {
        let mut msg = json!({
            "from_user_id": "",
            "to_user_id": to_user_id,
            "client_id": uuid::Uuid::new_v4().to_string(),
            "message_type": MSG_TYPE_BOT,
            "message_state": MSG_STATE_FINISH,
            "item_list": [{
                "type": ITEM_TEXT,
                "text_item": { "text": text }
            }]
        });
        if let Some(token) = context_token.filter(|value| !value.is_empty()) {
            msg["context_token"] = Value::String(token.to_string());
        }

        let response = self
            .api_post(
                base_url,
                EP_SEND_MESSAGE,
                json!({ "msg": msg }),
                Some(token),
            )
            .await?;
        let errcode = response.get("errcode").and_then(Value::as_i64).unwrap_or(0);
        let ret = response.get("ret").and_then(Value::as_i64).unwrap_or(0);
        if errcode != 0 || ret != 0 {
            return Err(format!(
                "Weixin sendmessage failed: {}",
                compact_json(&response)
            ));
        }
        Ok(())
    }

    async fn api_get(&self, base_url: &str, endpoint: &str) -> Result<Value, String> {
        let url = format!("{}/{}", base_url.trim_end_matches('/'), endpoint);
        let response = self
            .client
            .get(&url)
            .header("iLink-App-Id", ILINK_APP_ID)
            .header(
                "iLink-App-ClientVersion",
                ILINK_APP_CLIENT_VERSION.to_string(),
            )
            .send()
            .await
            .map_err(|e| format!("Weixin GET {endpoint} failed: {e}"))?;
        parse_response(endpoint, response).await
    }

    async fn api_post(
        &self,
        base_url: &str,
        endpoint: &str,
        payload: Value,
        token: Option<&str>,
    ) -> Result<Value, String> {
        let url = format!("{}/{}", base_url.trim_end_matches('/'), endpoint);
        let body = json_with_base_info(payload);
        let body_text = serde_json::to_string(&body)
            .map_err(|e| format!("Failed to encode Weixin request body: {e}"))?;
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("Content-Length", body_text.len().to_string())
            .header("X-WECHAT-UIN", random_wechat_uin())
            .header("iLink-App-Id", ILINK_APP_ID)
            .header(
                "iLink-App-ClientVersion",
                ILINK_APP_CLIENT_VERSION.to_string(),
            )
            .body(body_text);
        if let Some(token) = token.filter(|value| !value.is_empty()) {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("Weixin POST {endpoint} failed: {e}"))?;
        parse_response(endpoint, response).await
    }
}

async fn parse_response(endpoint: &str, response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Weixin {endpoint} response read failed: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "Weixin {endpoint} HTTP {status}: {}",
            truncate(&body, 300)
        ));
    }
    serde_json::from_str(&body).map_err(|e| {
        format!(
            "Weixin {endpoint} returned invalid JSON: {e}; body={}",
            truncate(&body, 300)
        )
    })
}

fn json_with_base_info(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "base_info".to_string(),
            json!({ "channel_version": CHANNEL_VERSION }),
        );
    }
    payload
}

fn random_wechat_uin() -> String {
    let value: u32 = rand::thread_rng().gen();
    BASE64.encode(value.to_string())
}

fn value_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("{}"))
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len])
    }
}

trait Pipe: Sized {
    fn pipe<F, T>(self, f: F) -> T
    where
        F: FnOnce(Self) -> T,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
