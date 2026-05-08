use crate::agents::ToolConfig;

const TASKS_TOOL_ID: &str = "tasks";

fn contains_tool_id(items: &[String], tool_id: &str) -> bool {
    items.iter().any(|item| item == tool_id)
}

pub(crate) fn append_task_plan_contract(system_prompt: &str, force_tasks: bool) -> String {
    const TASK_PLAN_CONTRACT: &str = "[TaskPlanContract]
- For simple single-step tasks, complete the work directly without creating a UI plan and without calling `tasks`.
- For multi-step work, publish and maintain a visible plan with the `tasks` tool.
- Each `tasks` call submits the complete desired `plan`; omitted old steps are removed from the displayed plan.
- Keep at most one step `in_progress`, and update the plan when meaningful progress changes.
- The task plan is a UI progress/event stream only. Normal model execution success, cancellation, or errors determine whether the run is finished.";

    if !force_tasks || system_prompt.contains("[TaskPlanContract]") {
        return system_prompt.to_string();
    }

    let trimmed = system_prompt.trim();
    if trimmed.is_empty() {
        TASK_PLAN_CONTRACT.to_string()
    } else {
        format!("{}\n\n{}", trimmed, TASK_PLAN_CONTRACT)
    }
}

/// When the runtime requires plan publishing, the task tool must survive
/// manual tool selection, disabled tools, and explicit allow-lists.
pub(crate) fn require_tasks_tool(mut config: Option<ToolConfig>) -> Option<ToolConfig> {
    let mut tool_config = config.take().unwrap_or_default();
    tool_config.enabled = true;
    tool_config
        .disabled_tools
        .retain(|tool_id| tool_id != TASKS_TOOL_ID);

    if !contains_tool_id(&tool_config.preselected_tools, TASKS_TOOL_ID) {
        tool_config
            .preselected_tools
            .insert(0, TASKS_TOOL_ID.to_string());
    }

    if !tool_config.allowed_tools.is_empty()
        && !contains_tool_id(&tool_config.allowed_tools, TASKS_TOOL_ID)
    {
        tool_config.allowed_tools.push(TASKS_TOOL_ID.to_string());
    }

    if tool_config.max_tools < tool_config.preselected_tools.len() {
        tool_config.max_tools = tool_config.preselected_tools.len();
    }

    Some(tool_config)
}

#[cfg(test)]
mod tests {
    use super::require_tasks_tool;
    use crate::agents::{ToolConfig, ToolSelectionStrategy};

    #[test]
    fn require_tasks_tool_enables_and_preselects_tasks() {
        let config = ToolConfig {
            enabled: false,
            selection_strategy: ToolSelectionStrategy::Manual(vec!["shell".to_string()]),
            max_tools: 1,
            preselected_tools: vec!["shell".to_string()],
            disabled_tools: vec!["tasks".to_string(), "grep".to_string()],
            allowed_tools: vec!["shell".to_string()],
        };

        let patched = require_tasks_tool(Some(config)).expect("config");

        assert!(patched.enabled);
        assert_eq!(
            patched.preselected_tools.first().map(String::as_str),
            Some("tasks")
        );
        assert!(patched.allowed_tools.iter().any(|id| id == "tasks"));
        assert!(!patched.disabled_tools.iter().any(|id| id == "tasks"));
        assert!(patched.max_tools >= patched.preselected_tools.len());
    }

    #[test]
    fn require_tasks_tool_builds_default_tool_config() {
        let patched = require_tasks_tool(None).expect("config");

        assert!(patched.enabled);
        assert!(patched.preselected_tools.iter().any(|id| id == "tasks"));
        assert!(!patched.disabled_tools.iter().any(|id| id == "tasks"));
    }
}
