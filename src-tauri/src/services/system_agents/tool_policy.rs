use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::BTreeSet;

use crate::agents::{ToolConfig, ToolSelectionStrategy};
use sentinel_db::SystemAgentProfileRecord;

#[derive(Debug, Clone, Default)]
pub struct SystemAgentToolPolicy {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub forbidden: Vec<String>,
}

impl SystemAgentToolPolicy {
    pub fn from_profile(profile: &SystemAgentProfileRecord) -> Self {
        Self {
            required: parse_tool_list(&profile.required_tools_json),
            optional: parse_tool_list(&profile.optional_tools_json),
            forbidden: parse_tool_list(&profile.forbidden_tools_json),
        }
    }

    pub fn validate(&self) -> Result<()> {
        for tool in &self.required {
            if self.forbidden.iter().any(|forbidden| forbidden == tool) {
                return Err(anyhow!(
                    "Tool '{}' cannot be both required and forbidden",
                    tool
                ));
            }
            if self.optional.iter().any(|optional| optional == tool) {
                return Err(anyhow!(
                    "Tool '{}' cannot be both required and optional",
                    tool
                ));
            }
        }

        for tool in &self.optional {
            if self.forbidden.iter().any(|forbidden| forbidden == tool) {
                return Err(anyhow!(
                    "Tool '{}' cannot be both optional and forbidden",
                    tool
                ));
            }
        }

        Ok(())
    }

    pub fn has_explicit_allowlist(&self) -> bool {
        !self.required.is_empty() || !self.optional.is_empty()
    }

    pub fn is_tool_allowed(&self, tool_id: &str) -> bool {
        if self.forbidden.iter().any(|tool| tool == tool_id) {
            return false;
        }

        if self.has_explicit_allowlist() {
            return self.required.iter().any(|tool| tool == tool_id)
                || self.optional.iter().any(|tool| tool == tool_id);
        }

        true
    }

    pub fn ensure_tool_allowed(&self, tool_id: &str) -> Result<()> {
        if !self.is_tool_allowed(tool_id) {
            if self.forbidden.iter().any(|tool| tool == tool_id) {
                return Err(anyhow!(
                    "Tool '{}' is forbidden by system-agent policy",
                    tool_id
                ));
            }
            return Err(anyhow!(
                "Tool '{}' is not allowed by system-agent policy",
                tool_id
            ));
        }

        Ok(())
    }

    pub fn prompt_note(&self) -> Option<String> {
        if self.required.is_empty() && self.optional.is_empty() && self.forbidden.is_empty() {
            return None;
        }

        Some(format!(
            concat!(
                "Tool policy:\n",
                "- required_tools: {}\n",
                "- optional_tools: {}\n",
                "- forbidden_tools: {}\n",
                "- enforcement: required tools are always injected; when required/optional are configured they form the explicit allowlist; forbidden tools are always blocked."
            ),
            render_list(&self.required),
            render_list(&self.optional),
            render_list(&self.forbidden),
        ))
    }

    pub fn declared_tools(&self) -> Vec<String> {
        let mut tools = BTreeSet::new();
        tools.extend(self.required.iter().cloned());
        tools.extend(self.optional.iter().cloned());
        tools.extend(self.forbidden.iter().cloned());
        tools.into_iter().collect()
    }

    pub fn build_runtime_tool_config(&self) -> Option<ToolConfig> {
        let preselected_tools = self
            .required
            .iter()
            .filter(|tool_id| !is_virtual_tool_id(tool_id))
            .cloned()
            .collect::<Vec<_>>();
        let optional_tools = self
            .optional
            .iter()
            .filter(|tool_id| !is_virtual_tool_id(tool_id))
            .cloned()
            .collect::<Vec<_>>();
        let disabled_tools = self
            .forbidden
            .iter()
            .filter(|tool_id| !is_virtual_tool_id(tool_id))
            .cloned()
            .collect::<Vec<_>>();

        let has_runtime_allowlist = !preselected_tools.is_empty() || !optional_tools.is_empty();
        let has_runtime_restrictions = has_runtime_allowlist || !disabled_tools.is_empty();
        if !has_runtime_restrictions {
            return None;
        }

        let allowed_tools = if has_runtime_allowlist {
            let mut allowed = BTreeSet::new();
            allowed.extend(preselected_tools.iter().cloned());
            allowed.extend(optional_tools.iter().cloned());
            allowed.into_iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let max_tools = if allowed_tools.is_empty() {
            5usize
        } else {
            allowed_tools.len().max(preselected_tools.len()).max(1)
        };

        Some(ToolConfig {
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools,
            preselected_tools,
            disabled_tools,
            allowed_tools,
            enabled: true,
        })
    }
}

pub fn validate_profile_tool_policy(profile: &SystemAgentProfileRecord) -> Result<()> {
    SystemAgentToolPolicy::from_profile(profile).validate()
}

fn parse_tool_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
                .filter(|item| !item.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default()
}

fn render_list(items: &[String]) -> String {
    if items.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", items.join(", "))
    }
}

pub fn is_virtual_tool_id(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "traffic_history_reader"
            | "repeater_launcher"
            | "workflow_catalog_reader"
            | "tool_catalog_reader"
            | "plugin_prompt_reader"
            | "plugin_example_reader"
            | "plugin_validator"
            | "plugin_test_result_reader"
            | "traffic_cluster_reader"
            | "auth_context_diff"
            | "workflow_state_reader"
            | "traffic_sequence_reader"
            | "active_replay"
    )
}
