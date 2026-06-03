use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficCodecRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub order: i32,
    pub match_rule: CodecMatchRule,
    pub scope: CodecScope,
    pub pipeline: CodecPipeline,
    pub reversible: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecMatchRule {
    pub hosts: Vec<String>,
    pub paths: Vec<String>,
    pub methods: Vec<String>,
    pub content_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecScope {
    pub target: CodecScopeTarget,
    pub fields: Vec<String>,
    pub header_name: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodecScopeTarget {
    JsonField,
    QueryParam,
    FormField,
    FullBody,
    HeaderValue,
    RegexMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecPipeline {
    pub steps: Vec<CodecStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecStep {
    pub id: String,
    #[serde(rename = "type")]
    pub step_type: CodecStepType,
    pub codec: String,
    pub plugin_id: Option<String>,
    pub config: HashMap<String, String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodecStepType {
    Builtin,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
    pub applied_rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecRequestMeta {
    pub host: String,
    pub path: String,
    pub method: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecExportData {
    pub version: String,
    pub format: String,
    pub exported_at: String,
    pub rules: Vec<CodecRuleExport>,
    pub key_placeholders: HashMap<String, KeyPlaceholderInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecRuleExport {
    pub name: String,
    pub match_rule: CodecMatchRule,
    pub scope: CodecScope,
    pub pipeline: CodecPipeline,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPlaceholderInfo {
    pub description: String,
    pub format: String,
    pub length: Option<u32>,
}
