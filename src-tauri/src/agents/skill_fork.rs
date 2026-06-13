//! Forked skill execution — runs skill instructions in an isolated sub-agent.

use sentinel_tools::buildin_tools::skills::{SkillsForkRequest, SkillsForkResult};

use super::executor::{execute_agent_turn, AgentExecuteParams};
use super::subagent_executor::{
    acquire_subagent_permits, build_subagent_system_prompt_for_role, get_subagent_app_handle,
    get_subagent_parent_context, max_iterations_for_verification_level,
    merge_skill_fork_tool_config, MAX_SUBAGENT_RECURSION_DEPTH,
};
use super::ContextPolicy;

pub async fn execute_forked_skill(request: SkillsForkRequest) -> Result<SkillsForkResult, String> {
    let app_handle = get_subagent_app_handle().map_err(|error| error.to_string())?;
    let parent = get_subagent_parent_context(&request.parent_execution_id)
        .await
        .map_err(|error| error.to_string())?;

    let recursion_depth = parent.recursion_depth + 1;
    if recursion_depth > MAX_SUBAGENT_RECURSION_DEPTH {
        return Err(format!(
            "Skill fork recursion depth exceeded: {recursion_depth} (max {MAX_SUBAGENT_RECURSION_DEPTH})"
        ));
    }

    let _permits = acquire_subagent_permits(&request.parent_execution_id, parent.timeout_secs)
        .await?;

    let fork_execution_id = format!("skill-fork-{}", uuid::Uuid::new_v4());
    let tool_config = merge_skill_fork_tool_config(
        parent.tool_config.clone(),
        request.allowed_tools.as_deref(),
    );
    let model = request
        .model_override
        .clone()
        .unwrap_or_else(|| parent.model.clone());
    let system_prompt = build_subagent_system_prompt_for_role(
        parent.system_prompt.clone(),
        request.agent_role.as_deref(),
    );

    if request.effort.is_some() {
        tracing::info!(
            "Forked skill '{}' requested effort override {:?} (not yet applied)",
            request.skill_name,
            request.effort
        );
    }

    let params = AgentExecuteParams {
        execution_id: fork_execution_id.clone(),
        conversation_id: None,
        cancellation_generation: None,
        model,
        system_prompt,
        task: request.task,
        active_browser_shell_direct_write_enabled: parent.active_browser_shell_direct_write_enabled,
        active_browser_shell_session_id: parent.active_browser_shell_session_id.clone(),
        active_terminal_session_fingerprint: parent.active_terminal_session_fingerprint.clone(),
        active_terminal_session_id: parent.active_terminal_session_id.clone(),
        working_directory: parent.working_directory.clone(),
        provider_config_key: parent.provider_config_key.clone(),
        rig_provider: parent.rig_provider.clone(),
        api_key: parent.api_key.clone(),
        api_base: parent.api_base.clone(),
        max_iterations: max_iterations_for_verification_level(parent.max_iterations),
        timeout_secs: parent.timeout_secs,
        tool_config: Some(tool_config),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        referenced_traffic: None,
        persist_messages: false,
        subagent_run_id: Some(fork_execution_id.clone()),
        harness_run_id: None,
        context_policy: Some(ContextPolicy::subagent()),
        context_engine_mode: None,
        recursion_depth,
    };

    tracing::info!(
        "Executing forked skill '{}' ({}) as sub-agent {}",
        request.skill_name,
        request.skill_id,
        params.execution_id
    );

    let outcome = execute_agent_turn(app_handle, params)
        .await
        .map_err(|error| error.to_string())?;

    Ok(SkillsForkResult {
        success: true,
        result: outcome.final_response,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::ToolConfig;

    #[test]
    fn merge_skill_allowed_tools_into_parent_config() {
        let parent = ToolConfig {
            preselected_tools: vec!["shell".to_string()],
            allowed_tools: vec!["shell".to_string()],
            ..Default::default()
        };

        let merged = merge_skill_fork_tool_config(
            parent,
            Some(&["grep".to_string(), "shell".to_string()]),
        );

        assert!(merged.preselected_tools.contains(&"grep".to_string()));
        assert!(merged.preselected_tools.contains(&"shell".to_string()));
        assert!(merged
            .disabled_tools
            .iter()
            .any(|tool| tool == "spawn_agent"));
    }
}
