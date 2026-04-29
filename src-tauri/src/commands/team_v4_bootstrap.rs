use std::sync::Arc;

use chrono::Utc;
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use super::team_v4_api::{
    append_team_v4_event_internal, attach_task_context_snapshot_internal,
    create_team_v4_context_snapshot_internal, create_team_v4_run_internal,
    create_team_v4_task_internal, register_team_v4_agent_internal, start_team_v4_harness_internal,
    update_run_state_internal, TeamV4Agent, TeamV4AppendEventRequest, TeamV4ContextSnapshot,
    TeamV4CreateContextSnapshotRequest, TeamV4CreateRunRequest, TeamV4CreateTaskRequest,
    TeamV4Event, TeamV4HarnessRun, TeamV4RegisterAgentRequest, TeamV4Run,
    TeamV4StartHarnessRequest, TeamV4Task,
};
use super::team_v4_schema::ensure_team_v4_schema;

type DbState<'r> = State<'r, Arc<DatabaseService>>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4StartAssistantRunRequest {
    pub conversation_id: Option<String>,
    pub profile_id: Option<String>,
    pub team_profile_id: Option<String>,
    pub commander_profile_id: Option<String>,
    pub solver_profile_ids: Option<Vec<String>>,
    pub observer_profile_id: Option<String>,
    pub goal: String,
    pub model: Option<String>,
    pub context_mode: Option<String>,
    pub tool_policy_matrix: Option<Value>,
    pub memory_policy: Option<Value>,
    pub harness_policy: Option<Value>,
    pub concurrency_policy: Option<Value>,
    pub safety_policy: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4SolverAssignment {
    pub solver: TeamV4Agent,
    pub task: TeamV4Task,
    pub context_snapshot: TeamV4ContextSnapshot,
    pub harness_run: TeamV4HarnessRun,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4RunBootstrap {
    pub run: TeamV4Run,
    pub commander: TeamV4Agent,
    pub observer: TeamV4Agent,
    pub solver: TeamV4Agent,
    pub solvers: Vec<TeamV4Agent>,
    pub solver_assignments: Vec<TeamV4SolverAssignment>,
    pub root_task: TeamV4Task,
    pub context_snapshot: TeamV4ContextSnapshot,
    pub harness_run: TeamV4HarnessRun,
    pub events: Vec<TeamV4Event>,
}

#[tauri::command]
pub async fn team_v4_start_assistant_run(
    db: DbState<'_>,
    request: TeamV4StartAssistantRunRequest,
) -> Result<TeamV4RunBootstrap, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;

    let commander_profile_id = request
        .commander_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Team v4 requires a commander profile id".to_string())?
        .to_string();
    let observer_profile_id = request
        .observer_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Team v4 requires an observer profile id".to_string())?
        .to_string();
    let mut solver_profile_ids = Vec::new();
    for solver_profile_id in request.solver_profile_ids.clone().unwrap_or_default() {
        let normalized = solver_profile_id.trim().to_string();
        if normalized.is_empty() || solver_profile_ids.contains(&normalized) {
            continue;
        }
        solver_profile_ids.push(normalized);
    }
    if solver_profile_ids.is_empty() {
        return Err("Team v4 requires at least one solver profile id".to_string());
    }

    let tool_policy_matrix = request.tool_policy_matrix.unwrap_or_else(|| json!({}));
    let memory_policy = request.memory_policy.unwrap_or_else(|| json!({}));
    let harness_policy = request
        .harness_policy
        .unwrap_or_else(|| json!({ "leaseSecs": 600 }));
    let concurrency_policy = request
        .concurrency_policy
        .unwrap_or_else(|| json!({ "maxSolvers": 1, "maxTasksPerSolver": 1 }));
    let safety_policy = request.safety_policy.unwrap_or_else(|| json!({}));
    let lease_secs = harness_policy
        .get("leaseSecs")
        .and_then(Value::as_i64)
        .unwrap_or(600)
        .max(30);
    let policy_json = json!({
        "architectureVersion": "team_v4",
        "layers": ["runtime_harness", "commander_scheduler", "solver_observer"],
        "teamProfileId": request.team_profile_id.clone(),
        "commanderProfileId": commander_profile_id,
        "solverProfileIds": solver_profile_ids,
        "observerProfileId": observer_profile_id,
        "toolPolicyMatrix": tool_policy_matrix,
        "memoryPolicy": memory_policy,
        "harnessPolicy": harness_policy,
        "concurrencyPolicy": concurrency_policy,
        "safetyPolicy": safety_policy,
        "contextMode": request.context_mode,
        "model": request.model,
    });

    let mut events = Vec::new();
    let mut run = create_team_v4_run_internal(
        &runtime_pool,
        TeamV4CreateRunRequest {
            conversation_id: request.conversation_id.clone(),
            profile_id: request.profile_id.clone(),
            goal: request.goal.clone(),
            policy_json: Some(policy_json.clone()),
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: None,
                task_id: None,
                event_type: "run_created".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({
                    "goal": run.goal,
                    "conversation_id": run.conversation_id,
                    "profile_id": run.profile_id,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let commander = register_team_v4_agent_internal(
        &runtime_pool,
        &run.id,
        TeamV4RegisterAgentRequest {
            profile_id: Some(commander_profile_id.clone()),
            role_type: "commander".to_string(),
            name: "Commander".to_string(),
            model: request.model.clone(),
            context_mode: request.context_mode.clone(),
            tool_policy_json: Some(policy_json["toolPolicyMatrix"].clone()),
            metadata: Some(json!({
                "responsibility": "global_schedule_task_planning_dynamic_dispatch",
                "safetyBoundary": "plans_and_delegates_before_solver_execution",
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(commander.id.clone()),
                task_id: None,
                event_type: "commander_planned".to_string(),
                visibility: Some("user".to_string()),
                payload: Some(json!({
                    "plan": [
                        "establish_shared_context",
                        "spawn_solver_pool",
                        "attach_observer_noise_filter",
                        "protect_long_task_with_harness"
                    ],
                    "dispatchPolicy": "dynamic_with_commander_approval",
                    "solverCount": solver_profile_ids.len(),
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let observer = register_team_v4_agent_internal(
        &runtime_pool,
        &run.id,
        TeamV4RegisterAgentRequest {
            profile_id: Some(observer_profile_id.clone()),
            role_type: "observer".to_string(),
            name: "Observer".to_string(),
            model: request.model.clone(),
            context_mode: request.context_mode.clone(),
            tool_policy_json: Some(json!({ "mode": "read_filter_summarize" })),
            metadata: Some(json!({
                "responsibility": "filter_noise_promote_high_value_information",
                "memoryGate": "candidate_then_commander_accept",
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(observer.id.clone()),
                task_id: None,
                event_type: "observer_attached".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({
                    "memoryPolicy": policy_json["memoryPolicy"],
                    "sharesOnlyHighValueInformation": true,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let mut solvers = Vec::new();
    for (index, solver_profile_id) in solver_profile_ids.iter().enumerate() {
        let solver = register_team_v4_agent_internal(
            &runtime_pool,
            &run.id,
            TeamV4RegisterAgentRequest {
                profile_id: Some(solver_profile_id.clone()),
                role_type: "solver".to_string(),
                name: format!("Solver {}", index + 1),
                model: request.model.clone(),
                context_mode: request.context_mode.clone(),
                tool_policy_json: Some(policy_json["toolPolicyMatrix"].clone()),
                metadata: Some(json!({
                    "inheritsHistory": true,
                    "spawnSource": "commander",
                    "canReceiveDynamicTasks": true,
                    "solverIndex": index,
                    "profileId": solver_profile_id,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        events.push(
            append_team_v4_event_internal(
                &runtime_pool,
                &run.id,
                TeamV4AppendEventRequest {
                    actor_id: Some(solver.id.clone()),
                    task_id: None,
                    event_type: "solver_spawned".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "solver_id": solver.id,
                        "profile_id": solver_profile_id,
                        "solver_index": index,
                        "inherits_history": true,
                        "context_mode": solver.context_mode,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );
        solvers.push(solver);
    }
    let solver = solvers
        .first()
        .cloned()
        .ok_or_else(|| "Team v4 failed to create a solver".to_string())?;

    let mut root_task = create_team_v4_task_internal(
        &runtime_pool,
        &run.id,
        TeamV4CreateTaskRequest {
            parent_task_id: None,
            task_key: "root".to_string(),
            title: "Root Task".to_string(),
            instruction: request.goal.clone(),
            priority: Some(0),
            assigned_agent_id: Some(solver.id.clone()),
            depends_on: Some(json!([])),
            acceptance_criteria: Some(
                "Commander can explain the result, Observer memory is curated, and Harness has a checkpoint."
                    .to_string(),
            ),
            metadata: Some(json!({
                "createdBy": commander.id.clone(),
                "dispatchMode": "commander_dynamic",
                "solverIndex": 0,
                "profileId": solver.profile_id,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(commander.id.clone()),
                task_id: Some(root_task.id.clone()),
                event_type: "task_created".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({
                    "task_key": root_task.task_key,
                    "title": root_task.title,
                    "assigned_agent_id": root_task.assigned_agent_id,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let context_snapshot = create_team_v4_context_snapshot_internal(
        &runtime_pool,
        &run.id,
        TeamV4CreateContextSnapshotRequest {
            actor_id: Some(solver.id.clone()),
            task_id: Some(root_task.id.clone()),
            role_type: "solver".to_string(),
            source_sequence: events.last().map(|event| event.sequence),
            policy_json: Some(json!({
                "inheritHistory": true,
                "contextMode": request.context_mode,
                "memoryGate": "observer_curated",
                "commanderVisible": true,
            })),
            sections_json: json!([
                {
                    "id": "goal",
                    "title": "User Goal",
                    "content": request.goal,
                },
                {
                    "id": "history_progress",
                    "title": "Inherited History Progress",
                    "content": {
                        "conversation_id": request.conversation_id,
                        "profile_id": request.profile_id,
                        "source": "assistant_conversation_binding"
                    }
                },
                {
                    "id": "shared_memory",
                    "title": "Shared High Value Memory",
                    "content": []
                }
            ]),
            token_estimate: Some(0),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    attach_task_context_snapshot_internal(&runtime_pool, &root_task.id, &context_snapshot.id)
        .await
        .map_err(|e| e.to_string())?;
    root_task.context_snapshot_id = Some(context_snapshot.id.clone());
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(solver.id.clone()),
                task_id: Some(root_task.id.clone()),
                event_type: "context_snapshot_created".to_string(),
                visibility: Some("internal".to_string()),
                payload: Some(json!({
                    "snapshot_id": context_snapshot.id,
                    "inherits_history": true,
                    "source_sequence": context_snapshot.source_sequence,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let harness_run = start_team_v4_harness_internal(
        &runtime_pool,
        &run.id,
        TeamV4StartHarnessRequest {
            actor_id: Some(solver.id.clone()),
            task_id: Some(root_task.id.clone()),
            lease_secs: Some(lease_secs),
            metadata: Some(json!({
                "managedBy": "harness",
                "checkpointPolicy": "event_sequence",
                "recoversFrom": ["lease_expiry", "process_restart", "long_task_pause"]
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(solver.id.clone()),
                task_id: Some(root_task.id.clone()),
                event_type: "harness_started".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({
                    "harness_run_id": harness_run.id,
                    "lease_expires_at": harness_run.lease_expires_at,
                    "checkpoint_sequence": harness_run.checkpoint_sequence,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let mut solver_assignments = vec![TeamV4SolverAssignment {
        solver: solver.clone(),
        task: root_task.clone(),
        context_snapshot: context_snapshot.clone(),
        harness_run: harness_run.clone(),
    }];

    for (index, assignment_solver) in solvers.iter().enumerate().skip(1) {
        let mut solver_task = create_team_v4_task_internal(
            &runtime_pool,
            &run.id,
            TeamV4CreateTaskRequest {
                parent_task_id: Some(root_task.id.clone()),
                task_key: format!("solver-{}", index + 1),
                title: format!("Solver {} Task", index + 1),
                instruction: request.goal.clone(),
                priority: Some(index as i32),
                assigned_agent_id: Some(assignment_solver.id.clone()),
                depends_on: Some(json!([])),
                acceptance_criteria: Some(
                    "Commander receives a structured result, Observer extracts high value evidence, and Harness records a checkpoint."
                        .to_string(),
                ),
                metadata: Some(json!({
                    "createdBy": commander.id.clone(),
                    "dispatchMode": "commander_dynamic",
                    "parentTaskId": root_task.id,
                    "solverIndex": index,
                    "profileId": assignment_solver.profile_id,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        events.push(
            append_team_v4_event_internal(
                &runtime_pool,
                &run.id,
                TeamV4AppendEventRequest {
                    actor_id: Some(commander.id.clone()),
                    task_id: Some(solver_task.id.clone()),
                    event_type: "task_created".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "task_key": solver_task.task_key,
                        "title": solver_task.title,
                        "assigned_agent_id": solver_task.assigned_agent_id,
                        "parent_task_id": solver_task.parent_task_id,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        let solver_context_snapshot = create_team_v4_context_snapshot_internal(
            &runtime_pool,
            &run.id,
            TeamV4CreateContextSnapshotRequest {
                actor_id: Some(assignment_solver.id.clone()),
                task_id: Some(solver_task.id.clone()),
                role_type: "solver".to_string(),
                source_sequence: events.last().map(|event| event.sequence),
                policy_json: Some(json!({
                    "inheritHistory": true,
                    "contextMode": request.context_mode,
                    "memoryGate": "observer_curated",
                    "commanderVisible": true,
                    "solverIndex": index,
                })),
                sections_json: json!([
                    {
                        "id": "goal",
                        "title": "User Goal",
                        "content": request.goal,
                    },
                    {
                        "id": "assignment",
                        "title": "Commander Assignment",
                        "content": {
                            "task_id": solver_task.id,
                            "task_key": solver_task.task_key,
                            "parent_task_id": root_task.id,
                            "solver_id": assignment_solver.id,
                            "solver_profile_id": assignment_solver.profile_id,
                        }
                    },
                    {
                        "id": "shared_memory",
                        "title": "Shared High Value Memory",
                        "content": []
                    }
                ]),
                token_estimate: Some(0),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        attach_task_context_snapshot_internal(
            &runtime_pool,
            &solver_task.id,
            &solver_context_snapshot.id,
        )
        .await
        .map_err(|e| e.to_string())?;
        solver_task.context_snapshot_id = Some(solver_context_snapshot.id.clone());
        events.push(
            append_team_v4_event_internal(
                &runtime_pool,
                &run.id,
                TeamV4AppendEventRequest {
                    actor_id: Some(assignment_solver.id.clone()),
                    task_id: Some(solver_task.id.clone()),
                    event_type: "context_snapshot_created".to_string(),
                    visibility: Some("internal".to_string()),
                    payload: Some(json!({
                        "snapshot_id": solver_context_snapshot.id,
                        "inherits_history": true,
                        "source_sequence": solver_context_snapshot.source_sequence,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        let solver_harness_run = start_team_v4_harness_internal(
            &runtime_pool,
            &run.id,
            TeamV4StartHarnessRequest {
                actor_id: Some(assignment_solver.id.clone()),
                task_id: Some(solver_task.id.clone()),
                lease_secs: Some(lease_secs),
                metadata: Some(json!({
                    "managedBy": "harness",
                    "checkpointPolicy": "event_sequence",
                    "solverIndex": index,
                    "recoversFrom": ["lease_expiry", "process_restart", "long_task_pause"]
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        events.push(
            append_team_v4_event_internal(
                &runtime_pool,
                &run.id,
                TeamV4AppendEventRequest {
                    actor_id: Some(assignment_solver.id.clone()),
                    task_id: Some(solver_task.id.clone()),
                    event_type: "harness_started".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "harness_run_id": solver_harness_run.id,
                        "lease_expires_at": solver_harness_run.lease_expires_at,
                        "checkpoint_sequence": solver_harness_run.checkpoint_sequence,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        solver_assignments.push(TeamV4SolverAssignment {
            solver: assignment_solver.clone(),
            task: solver_task,
            context_snapshot: solver_context_snapshot,
            harness_run: solver_harness_run,
        });
    }

    update_run_state_internal(&runtime_pool, &run.id, "running")
        .await
        .map_err(|e| e.to_string())?;
    run.state = "running".to_string();
    run.updated_at = Utc::now().to_rfc3339();
    events.push(
        append_team_v4_event_internal(
            &runtime_pool,
            &run.id,
            TeamV4AppendEventRequest {
                actor_id: Some(commander.id.clone()),
                task_id: Some(root_task.id.clone()),
                event_type: "run_state_changed".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({ "state": "running" })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    Ok(TeamV4RunBootstrap {
        run,
        commander,
        observer,
        solver,
        solvers,
        solver_assignments,
        root_task,
        context_snapshot,
        harness_run,
        events,
    })
}
