use crate::service_probe_runtime::ServiceProbeRule;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceProbeRuleMetadata {
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub probe_name: Option<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub ssl_ports: Vec<u16>,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub softmatch: Option<bool>,
    #[serde(default)]
    pub matchers: Vec<ServiceProbeRuleMatcher>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceProbeRuleMatcher {
    #[serde(default)]
    pub part: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
}

pub(crate) fn parse_rule_metadata(rule: &ServiceProbeRule) -> ServiceProbeRuleMetadata {
    if rule.metadata.is_null() {
        return ServiceProbeRuleMetadata::default();
    }

    serde_json::from_value::<ServiceProbeRuleMetadata>(rule.metadata.clone()).unwrap_or_default()
}

pub(crate) fn metadata_string_field(value: Option<&String>) -> Option<String> {
    value
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

pub(crate) fn metadata_object_field<'a>(
    metadata: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Option<&'a Value> {
    metadata.get(key)
}
