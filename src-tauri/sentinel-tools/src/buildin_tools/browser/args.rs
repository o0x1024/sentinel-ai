use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    Launch,
    Goto,
    Snapshot,
    Click,
    Fill,
    Eval,
    Cookies,
    NetworkLog,
    Close,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BrowserToolArgs {
    pub action: BrowserAction,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub headless: Option<bool>,
    #[serde(default)]
    pub network_limit: Option<usize>,
}
