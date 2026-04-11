use std::collections::HashMap;

use anyhow::{anyhow, Result};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::agents::{
    clear_parent_context, list_agents, set_parent_context, spawn_agent, wait_agents,
    ControlPlaneSpawnRequest, SubagentParentContext, ToolConfig,
};

#[derive(Debug, Clone)]
pub(crate) struct TeamWaveTaskExecutionRequest {
    pub task_id: String,
    pub task_key: String,
    pub task_title: String,
    pub member_id: String,
    pub prompt: String,
    pub system_prompt: String,
    pub dependency_task_ids: Vec<String>,
    pub dependency_task_keys: Vec<String>,
    pub is_summary_task: bool,
    pub max_iterations: usize,
    pub timeout_secs: u64,
    pub tool_config: ToolConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamWaveTaskExecutionResult {
    pub task_id: String,
    pub task_key: String,
    pub task_title: String,
    pub member_id: String,
    pub dependency_task_ids: Vec<String>,
    pub dependency_task_keys: Vec<String>,
    pub is_summary_task: bool,
    pub execution_result: Result<String, String>,
}

pub(crate) async fn execute_team_wave_tasks(
    _app_handle: &AppHandle,
    session_id: &str,
    rig_provider: &str,
    model: &str,
    api_key: Option<String>,
    api_base: Option<String>,
    cancellation_token: &CancellationToken,
    requests: Vec<TeamWaveTaskExecutionRequest>,
) -> Result<Vec<TeamWaveTaskExecutionResult>> {
    if requests.is_empty() {
        return Ok(Vec::new());
    }

    let parent_execution_id = format!("team-v3-wave:{}", session_id);
    let max_iterations = requests
        .iter()
        .map(|item| item.max_iterations)
        .max()
        .unwrap_or(12);
    let timeout_secs = requests
        .iter()
        .map(|item| item.timeout_secs)
        .max()
        .unwrap_or(180);
    let base_tool_config = requests
        .first()
        .map(|item| item.tool_config.clone())
        .unwrap_or_default();

    set_parent_context(
        parent_execution_id.clone(),
        SubagentParentContext {
            rig_provider: rig_provider.to_string(),
            model: model.to_string(),
            api_key,
            api_base,
            system_prompt: String::new(),
            tool_config: base_tool_config,
            max_iterations,
            timeout_secs,
            task_context: format!("team_v3 session {}", session_id),
            recursion_depth: 0,
        },
    )
    .await;

    let mut request_by_spawned_id = HashMap::new();
    let mut spawned_task_ids = Vec::new();

    for request in requests {
        if cancellation_token.is_cancelled() {
            clear_parent_context(&parent_execution_id).await;
            return Err(anyhow!("Team execution cancelled"));
        }

        let task_id = spawn_agent(ControlPlaneSpawnRequest {
            parent_execution_id: parent_execution_id.clone(),
            task: request.prompt.clone(),
            role: Some("team_worker".to_string()),
            system_prompt: Some(request.system_prompt.clone()),
            tool_config: Some(
                serde_json::to_value(&request.tool_config)
                    .map_err(|error| anyhow!("Failed to serialize team tool config: {}", error))?,
            ),
            max_iterations: request.max_iterations,
            timeout_secs: Some(request.timeout_secs),
            inherit_parent_tools: true,
            depends_on_task_ids: Vec::new(),
        })
        .await
        .map_err(|error| anyhow!("Failed to spawn team worker task: {}", error))?;

        spawned_task_ids.push(task_id.clone());
        request_by_spawned_id.insert(task_id, request);
    }

    let wait_result = tokio::select! {
        _ = cancellation_token.cancelled() => {
            for task_id in &spawned_task_ids {
                let _ = crate::agents::close_agent(&parent_execution_id, task_id).await;
            }
            clear_parent_context(&parent_execution_id).await;
            return Err(anyhow!("Team execution cancelled"));
        }
        result = wait_agents(parent_execution_id.clone(), spawned_task_ids.clone(), timeout_secs) => {
            result.map_err(|error| anyhow!("Failed to wait team worker tasks: {}", error))?
        }
    };

    let mut result_by_id = wait_result
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    for item in list_agents(&parent_execution_id).await {
        result_by_id.insert(item.id.clone(), item);
    }
    clear_parent_context(&parent_execution_id).await;

    let mut results = Vec::new();
    for task_id in spawned_task_ids {
        let Some(request) = request_by_spawned_id.remove(&task_id) else {
            continue;
        };
        let execution_result = match result_by_id.get(&task_id) {
            Some(item) if item.status == "completed" => Ok(item.output.clone().unwrap_or_default()),
            Some(item) => Err(item
                .error
                .clone()
                .unwrap_or_else(|| "Task failed".to_string())),
            None => Err("Task result not found".to_string()),
        };

        results.push(TeamWaveTaskExecutionResult {
            task_id: request.task_id,
            task_key: request.task_key,
            task_title: request.task_title,
            member_id: request.member_id,
            dependency_task_ids: request.dependency_task_ids,
            dependency_task_keys: request.dependency_task_keys,
            is_summary_task: request.is_summary_task,
            execution_result,
        });
    }

    Ok(results)
}
