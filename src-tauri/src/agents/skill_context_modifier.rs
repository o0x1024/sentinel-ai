//! Applies skill frontmatter-driven context changes after invocation (Claude Code `contextModifier`).

use std::collections::HashSet;

use crate::agents::ToolConfig;

/// Context modifications requested by an invoked skill's frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillContextModifier {
    pub allowed_tools: Vec<String>,
    pub model_override: Option<String>,
    pub effort: Option<String>,
}

impl SkillContextModifier {
    pub fn is_empty(&self) -> bool {
        self.allowed_tools.is_empty()
            && self.model_override.is_none()
            && self.effort.is_none()
    }
}

/// Merge skill-granted tools into the active tool set, respecting max_tools cap.
pub fn apply_allowed_tools(
    current_tool_ids: &mut Vec<String>,
    tool_config: &ToolConfig,
    allowed_tools: &[String],
    available_tool_ids: &[String],
) {
    if allowed_tools.is_empty() {
        return;
    }

    let available: HashSet<&str> = available_tool_ids.iter().map(String::as_str).collect();
    let mut seen: HashSet<String> = current_tool_ids.iter().cloned().collect();

    for tool_id in allowed_tools {
        let normalized = tool_id.trim();
        if normalized.is_empty() || !available.contains(normalized) || seen.contains(normalized) {
            continue;
        }
        seen.insert(normalized.to_string());
        current_tool_ids.push(normalized.to_string());
    }

    if current_tool_ids.len() > tool_config.max_tools {
        current_tool_ids.truncate(tool_config.max_tools);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_allowed_tools_without_duplicates() {
        let mut current = vec!["shell".to_string(), "file_read".to_string()];
        let available = vec![
            "shell".to_string(),
            "file_read".to_string(),
            "grep".to_string(),
        ];
        let config = ToolConfig {
            max_tools: 10,
            ..Default::default()
        };

        apply_allowed_tools(
            &mut current,
            &config,
            &["grep".to_string(), "shell".to_string()],
            &available,
        );

        assert_eq!(current, vec!["shell", "file_read", "grep"]);
    }

    #[test]
    fn respects_max_tools_cap() {
        let mut current = vec!["shell".to_string()];
        let available = vec![
            "shell".to_string(),
            "grep".to_string(),
            "glob".to_string(),
        ];
        let config = ToolConfig {
            max_tools: 2,
            ..Default::default()
        };

        apply_allowed_tools(
            &mut current,
            &config,
            &["grep".to_string(), "glob".to_string()],
            &available,
        );

        assert_eq!(current.len(), 2);
    }
}
