//! Unified multi-agent control plane.

use serde::{Deserialize, Serialize};

use super::subagent_executor::ControlPlaneSpawnRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHandleKind {
    Subagent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHandleSummary {
    pub id: String,
    pub parent_execution_id: String,
    pub kind: AgentHandleKind,
    pub role: Option<String>,
    pub task: String,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub async fn spawn_agent(request: ControlPlaneSpawnRequest) -> Result<String, String> {
    super::subagent_executor::control_plane_spawn_task(request).await
}

pub async fn wait_agents(
    parent_execution_id: String,
    task_ids: Vec<String>,
    timeout_secs: u64,
) -> Result<Vec<AgentHandleSummary>, String> {
    let output = super::subagent_executor::control_plane_wait_tasks(
        parent_execution_id.clone(),
        task_ids.clone(),
        timeout_secs,
    )
    .await?;

    let by_id = list_agents(&parent_execution_id)
        .await
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<std::collections::HashMap<_, _>>();

    Ok(output
        .results
        .into_iter()
        .filter_map(|item| by_id.get(&item.task_id).cloned())
        .collect())
}

pub async fn list_agents(parent_execution_id: &str) -> Vec<AgentHandleSummary> {
    super::subagent_executor::control_plane_list_tasks(parent_execution_id)
        .await
        .into_iter()
        .map(|item| AgentHandleSummary {
            id: item.task_id,
            parent_execution_id: item.parent_execution_id,
            kind: AgentHandleKind::Subagent,
            role: item.role,
            task: item.task,
            status: format!("{:?}", item.status).to_ascii_lowercase(),
            started_at: item.started_at,
            completed_at: item.completed_at,
            output: item.output,
            error: item.error,
        })
        .collect()
}

pub async fn close_agent(parent_execution_id: &str, task_id: &str) -> Result<(), String> {
    super::subagent_executor::control_plane_close_task(parent_execution_id, task_id).await
}
