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

fn read_target_specialist_count(
    concurrency_policy: &Value,
    configured_specialist_count: usize,
) -> Result<usize, String> {
    let requested = concurrency_policy
        .get("maxSpecialists")
        .or_else(|| concurrency_policy.get("max_specialists"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "Team v4 requires concurrencyPolicy.maxSpecialists".to_string())?
        as usize;
    if requested == 0 {
        return Err(
            "Team v4 requires concurrencyPolicy.maxSpecialists to be at least 1".to_string(),
        );
    }
    Ok(requested.max(configured_specialist_count))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4StartAssistantRunRequest {
    pub conversation_id: Option<String>,
    pub profile_id: Option<String>,
    pub team_profile_id: Option<String>,
    pub orchestrator_profile_id: Option<String>,
    pub specialist_profile_ids: Option<Vec<String>>,
    pub monitor_profile_id: Option<String>,
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
pub struct TeamV4SpecialistAssignment {
    pub specialist: TeamV4Agent,
    pub task: TeamV4Task,
    pub context_snapshot: TeamV4ContextSnapshot,
    pub harness_run: TeamV4HarnessRun,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4RunBootstrap {
    pub run: TeamV4Run,
    pub orchestrator: TeamV4Agent,
    pub monitor: TeamV4Agent,
    pub specialist: TeamV4Agent,
    pub specialists: Vec<TeamV4Agent>,
    pub specialist_assignments: Vec<TeamV4SpecialistAssignment>,
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

    let orchestrator_profile_id = request
        .orchestrator_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Team v4 requires a orchestrator profile id".to_string())?
        .to_string();
    let monitor_profile_id = request
        .monitor_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Team v4 requires an monitor profile id".to_string())?
        .to_string();
    let mut specialist_profile_ids = Vec::new();
    for specialist_profile_id in request.specialist_profile_ids.clone().unwrap_or_default() {
        let normalized = specialist_profile_id.trim().to_string();
        if normalized.is_empty() || specialist_profile_ids.contains(&normalized) {
            continue;
        }
        specialist_profile_ids.push(normalized);
    }
    if specialist_profile_ids.is_empty() {
        return Err("Team v4 requires at least one specialist profile id".to_string());
    }

    let tool_policy_matrix = request.tool_policy_matrix.unwrap_or_else(|| json!({}));
    let memory_policy = request.memory_policy.unwrap_or_else(|| json!({}));
    let harness_policy = request
        .harness_policy
        .unwrap_or_else(|| json!({ "leaseSecs": 600 }));
    let concurrency_policy = request
        .concurrency_policy
        .unwrap_or_else(|| json!({ "maxSpecialists": 1, "maxTasksPerSpecialist": 1 }));
    let safety_policy = request.safety_policy.unwrap_or_else(|| json!({}));
    let specialist_instance_count =
        read_target_specialist_count(&concurrency_policy, specialist_profile_ids.len())?;
    let specialist_instance_profile_ids = (0..specialist_instance_count)
        .map(|index| specialist_profile_ids[index % specialist_profile_ids.len()].clone())
        .collect::<Vec<_>>();
    let lease_secs = harness_policy
        .get("leaseSecs")
        .and_then(Value::as_i64)
        .unwrap_or(600)
        .max(30);
    let policy_json = json!({
        "architectureVersion": "team_v4",
        "layers": ["orchestration_layer", "specialist_pool", "monitoring_quality_gate", "runtime_harness"],
        "teamProfileId": request.team_profile_id.clone(),
        "orchestratorProfileId": orchestrator_profile_id,
        "specialistProfileIds": specialist_profile_ids.clone(),
        "specialistInstanceProfileIds": specialist_instance_profile_ids.clone(),
        "specialistInstanceCount": specialist_instance_count,
        "monitorProfileId": monitor_profile_id,
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

    let orchestrator = register_team_v4_agent_internal(
        &runtime_pool,
        &run.id,
        TeamV4RegisterAgentRequest {
            profile_id: Some(orchestrator_profile_id.clone()),
            role_type: "orchestrator".to_string(),
            name: "Orchestrator".to_string(),
            model: request.model.clone(),
            context_mode: request.context_mode.clone(),
            tool_policy_json: Some(policy_json["toolPolicyMatrix"].clone()),
            metadata: Some(json!({
                "responsibility": "dynamic_task_decomposition_dependency_graph_specialist_dispatch",
                "safetyBoundary": "plans_dependencies_and_recovery_before_specialist_execution",
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
                actor_id: Some(orchestrator.id.clone()),
                task_id: None,
                event_type: "orchestrator_planned".to_string(),
                visibility: Some("user".to_string()),
                payload: Some(json!({
                    "plan": [
                    "establish_shared_context",
                    "spawn_specialist_pool",
                    "attach_monitor_quality_gate",
                    "protect_long_task_with_harness"
                ],
                "dispatchPolicy": "dynamic_dependency_graph_with_monitor_feedback",
                    "specialistCount": specialist_instance_count,
                })),
            },
        )
        .await
        .map_err(|e| e.to_string())?,
    );

    let monitor = register_team_v4_agent_internal(
        &runtime_pool,
        &run.id,
        TeamV4RegisterAgentRequest {
            profile_id: Some(monitor_profile_id.clone()),
            role_type: "monitor".to_string(),
            name: "Monitor".to_string(),
            model: request.model.clone(),
            context_mode: request.context_mode.clone(),
            tool_policy_json: Some(json!({ "mode": "metrics_quality_retry_signal" })),
            metadata: Some(json!({
                "responsibility": "collect_runtime_metrics_detect_anomalies_evaluate_quality_trigger_retry_signal",
                "memoryGate": "candidate_then_orchestrator_accept",
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
                actor_id: Some(monitor.id.clone()),
                task_id: None,
                event_type: "monitor_attached".to_string(),
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

    let mut specialists = Vec::new();
    for (index, specialist_profile_id) in specialist_instance_profile_ids.iter().enumerate() {
        let specialist = register_team_v4_agent_internal(
            &runtime_pool,
            &run.id,
            TeamV4RegisterAgentRequest {
                profile_id: Some(specialist_profile_id.clone()),
                role_type: "specialist".to_string(),
                name: format!("Specialist {}", index + 1),
                model: request.model.clone(),
                context_mode: request.context_mode.clone(),
                tool_policy_json: Some(policy_json["toolPolicyMatrix"].clone()),
                metadata: Some(json!({
                    "inheritsHistory": true,
                    "spawnSource": "orchestrator",
                    "canReceiveDynamicTasks": true,
                    "specialistIndex": index,
                    "profileId": specialist_profile_id,
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
                    actor_id: Some(specialist.id.clone()),
                    task_id: None,
                    event_type: "specialist_spawned".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "specialist_id": specialist.id,
                        "profile_id": specialist_profile_id,
                        "specialist_index": index,
                        "specialist_instance_count": specialist_instance_count,
                        "inherits_history": true,
                        "context_mode": specialist.context_mode,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );
        specialists.push(specialist);
    }
    let specialist = specialists
        .first()
        .cloned()
        .ok_or_else(|| "Team v4 failed to create a specialist".to_string())?;

    let mut root_task = create_team_v4_task_internal(
        &runtime_pool,
        &run.id,
        TeamV4CreateTaskRequest {
            parent_task_id: None,
            task_key: "root".to_string(),
            title: "Root Task".to_string(),
            instruction: request.goal.clone(),
            priority: Some(0),
            assigned_agent_id: Some(specialist.id.clone()),
            depends_on: Some(json!([])),
            acceptance_criteria: Some(
                "Orchestrator can explain the result, Monitor memory is curated, and Harness has a checkpoint."
                    .to_string(),
            ),
            metadata: Some(json!({
                "createdBy": orchestrator.id.clone(),
                "dispatchMode": "orchestrator_dynamic",
                "specialistIndex": 0,
                "profileId": specialist.profile_id,
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
                actor_id: Some(orchestrator.id.clone()),
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
            actor_id: Some(specialist.id.clone()),
            task_id: Some(root_task.id.clone()),
            role_type: "specialist".to_string(),
            source_sequence: events.last().map(|event| event.sequence),
            policy_json: Some(json!({
                "inheritHistory": true,
                "contextMode": request.context_mode,
                "memoryGate": "monitor_curated",
                "orchestratorVisible": true,
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
                actor_id: Some(specialist.id.clone()),
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
            actor_id: Some(specialist.id.clone()),
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
                actor_id: Some(specialist.id.clone()),
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

    let mut specialist_assignments = vec![TeamV4SpecialistAssignment {
        specialist: specialist.clone(),
        task: root_task.clone(),
        context_snapshot: context_snapshot.clone(),
        harness_run: harness_run.clone(),
    }];

    for (index, assignment_specialist) in specialists.iter().enumerate().skip(1) {
        let mut specialist_task = create_team_v4_task_internal(
            &runtime_pool,
            &run.id,
            TeamV4CreateTaskRequest {
                parent_task_id: Some(root_task.id.clone()),
                task_key: format!("specialist-{}", index + 1),
                title: format!("Specialist {} Task", index + 1),
                instruction: request.goal.clone(),
                priority: Some(index as i32),
                assigned_agent_id: Some(assignment_specialist.id.clone()),
                depends_on: Some(json!([])),
                acceptance_criteria: Some(
                    "Orchestrator receives a structured result, Monitor extracts high value evidence, and Harness records a checkpoint."
                        .to_string(),
                ),
                metadata: Some(json!({
                    "createdBy": orchestrator.id.clone(),
                    "dispatchMode": "orchestrator_dynamic",
                    "parentTaskId": root_task.id,
                    "specialistIndex": index,
                    "profileId": assignment_specialist.profile_id,
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
                    actor_id: Some(orchestrator.id.clone()),
                    task_id: Some(specialist_task.id.clone()),
                    event_type: "task_created".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "task_key": specialist_task.task_key,
                        "title": specialist_task.title,
                        "assigned_agent_id": specialist_task.assigned_agent_id,
                        "parent_task_id": specialist_task.parent_task_id,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        let specialist_context_snapshot = create_team_v4_context_snapshot_internal(
            &runtime_pool,
            &run.id,
            TeamV4CreateContextSnapshotRequest {
                actor_id: Some(assignment_specialist.id.clone()),
                task_id: Some(specialist_task.id.clone()),
                role_type: "specialist".to_string(),
                source_sequence: events.last().map(|event| event.sequence),
                policy_json: Some(json!({
                    "inheritHistory": true,
                    "contextMode": request.context_mode,
                    "memoryGate": "monitor_curated",
                    "orchestratorVisible": true,
                    "specialistIndex": index,
                })),
                sections_json: json!([
                    {
                        "id": "goal",
                        "title": "User Goal",
                        "content": request.goal,
                    },
                    {
                        "id": "assignment",
                        "title": "Orchestrator Assignment",
                        "content": {
                            "task_id": specialist_task.id,
                            "task_key": specialist_task.task_key,
                            "parent_task_id": root_task.id,
                            "specialist_id": assignment_specialist.id,
                            "specialist_profile_id": assignment_specialist.profile_id,
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
            &specialist_task.id,
            &specialist_context_snapshot.id,
        )
        .await
        .map_err(|e| e.to_string())?;
        specialist_task.context_snapshot_id = Some(specialist_context_snapshot.id.clone());
        events.push(
            append_team_v4_event_internal(
                &runtime_pool,
                &run.id,
                TeamV4AppendEventRequest {
                    actor_id: Some(assignment_specialist.id.clone()),
                    task_id: Some(specialist_task.id.clone()),
                    event_type: "context_snapshot_created".to_string(),
                    visibility: Some("internal".to_string()),
                    payload: Some(json!({
                        "snapshot_id": specialist_context_snapshot.id,
                        "inherits_history": true,
                        "source_sequence": specialist_context_snapshot.source_sequence,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        let specialist_harness_run = start_team_v4_harness_internal(
            &runtime_pool,
            &run.id,
            TeamV4StartHarnessRequest {
                actor_id: Some(assignment_specialist.id.clone()),
                task_id: Some(specialist_task.id.clone()),
                lease_secs: Some(lease_secs),
                metadata: Some(json!({
                    "managedBy": "harness",
                    "checkpointPolicy": "event_sequence",
                    "specialistIndex": index,
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
                    actor_id: Some(assignment_specialist.id.clone()),
                    task_id: Some(specialist_task.id.clone()),
                    event_type: "harness_started".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "harness_run_id": specialist_harness_run.id,
                        "lease_expires_at": specialist_harness_run.lease_expires_at,
                        "checkpoint_sequence": specialist_harness_run.checkpoint_sequence,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?,
        );

        specialist_assignments.push(TeamV4SpecialistAssignment {
            specialist: assignment_specialist.clone(),
            task: specialist_task,
            context_snapshot: specialist_context_snapshot,
            harness_run: specialist_harness_run,
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
                actor_id: Some(orchestrator.id.clone()),
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
        orchestrator,
        monitor,
        specialist,
        specialists,
        specialist_assignments,
        root_task,
        context_snapshot,
        harness_run,
        events,
    })
}
