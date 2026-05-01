use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FetchBody {
    Text { text: String },
    Bytes { bytes: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOptions {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<FetchBody>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub max_redirects: Option<usize>,
    #[serde(default)]
    pub max_body_bytes: Option<usize>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub active_probe: Option<ActiveProbeFetchOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub success: bool,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub ok: bool,
    pub redirected: bool,
    pub final_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeFetchOptions {
    #[serde(default)]
    pub probe_label: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub target_location: Option<String>,
    #[serde(default)]
    pub probe_value: Option<String>,
    #[serde(default)]
    pub technique: Option<String>,
}
