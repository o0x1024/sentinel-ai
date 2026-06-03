use anyhow::Result;
use sentinel_db::sqlx_compat::PgRow;
use serde_json::Value;
use sqlx::Row;

use super::team_v4_api::{
    TeamV4Agent, TeamV4Event, TeamV4HarnessRun, TeamV4Memory, TeamV4Run, TeamV4Task,
};

fn parse_json_text(raw: String) -> Result<Value> {
    serde_json::from_str::<Value>(&raw).map_err(Into::into)
}

pub(crate) fn map_run(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4Run> {
    let policy_text: String = row.get("policy_json");
    Ok(TeamV4Run {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        profile_id: row.get("profile_id"),
        goal: row.get("goal"),
        state: row.get("state"),
        policy_json: parse_json_text(policy_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_run_pg(row: PgRow) -> Result<TeamV4Run> {
    let policy_text: String = row.get("policy_json");
    Ok(TeamV4Run {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        profile_id: row.get("profile_id"),
        goal: row.get("goal"),
        state: row.get("state"),
        policy_json: parse_json_text(policy_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_agent(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4Agent> {
    let tool_policy_text: String = row.get("tool_policy_json");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Agent {
        id: row.get("id"),
        run_id: row.get("run_id"),
        profile_id: row.get("profile_id"),
        role_type: row.get("role_type"),
        name: row.get("name"),
        status: row.get("status"),
        model: row.get("model"),
        context_mode: row.get("context_mode"),
        tool_policy_json: parse_json_text(tool_policy_text)?,
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_agent_pg(row: PgRow) -> Result<TeamV4Agent> {
    let tool_policy_text: String = row.get("tool_policy_json");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Agent {
        id: row.get("id"),
        run_id: row.get("run_id"),
        profile_id: row.get("profile_id"),
        role_type: row.get("role_type"),
        name: row.get("name"),
        status: row.get("status"),
        model: row.get("model"),
        context_mode: row.get("context_mode"),
        tool_policy_json: parse_json_text(tool_policy_text)?,
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_task(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4Task> {
    let depends_on_text: String = row.get("depends_on");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Task {
        id: row.get("id"),
        run_id: row.get("run_id"),
        parent_task_id: row.get("parent_task_id"),
        task_key: row.get("task_key"),
        title: row.get("title"),
        instruction: row.get("instruction"),
        status: row.get("status"),
        priority: row.get("priority"),
        assigned_agent_id: row.get("assigned_agent_id"),
        depends_on: parse_json_text(depends_on_text)?,
        acceptance_criteria: row.get("acceptance_criteria"),
        context_snapshot_id: row.get("context_snapshot_id"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_task_pg(row: PgRow) -> Result<TeamV4Task> {
    let depends_on_text: String = row.get("depends_on");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Task {
        id: row.get("id"),
        run_id: row.get("run_id"),
        parent_task_id: row.get("parent_task_id"),
        task_key: row.get("task_key"),
        title: row.get("title"),
        instruction: row.get("instruction"),
        status: row.get("status"),
        priority: row.get("priority"),
        assigned_agent_id: row.get("assigned_agent_id"),
        depends_on: parse_json_text(depends_on_text)?,
        acceptance_criteria: row.get("acceptance_criteria"),
        context_snapshot_id: row.get("context_snapshot_id"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_event(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4Event> {
    let payload_text: String = row.get("payload");
    Ok(TeamV4Event {
        id: row.get("id"),
        run_id: row.get("run_id"),
        sequence: row.get("sequence"),
        actor_id: row.get("actor_id"),
        task_id: row.get("task_id"),
        event_type: row.get("event_type"),
        visibility: row.get("visibility"),
        payload: parse_json_text(payload_text)?,
        created_at: row.get("created_at"),
    })
}

pub(crate) fn map_event_pg(row: PgRow) -> Result<TeamV4Event> {
    let payload_text: String = row.get("payload");
    Ok(TeamV4Event {
        id: row.get("id"),
        run_id: row.get("run_id"),
        sequence: row.get("sequence"),
        actor_id: row.get("actor_id"),
        task_id: row.get("task_id"),
        event_type: row.get("event_type"),
        visibility: row.get("visibility"),
        payload: parse_json_text(payload_text)?,
        created_at: row.get("created_at"),
    })
}

pub(crate) fn map_memory(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4Memory> {
    let source_event_ids_text: String = row.get("source_event_ids");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Memory {
        id: row.get("id"),
        run_id: row.get("run_id"),
        task_id: row.get("task_id"),
        kind: row.get("kind"),
        content: row.get("content"),
        confidence: row.get("confidence"),
        source_event_ids: parse_json_text(source_event_ids_text)?,
        accepted_by_orchestrator: row.get("accepted_by_orchestrator"),
        promoted_to_long_term: row.get("promoted_to_long_term"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_memory_pg(row: PgRow) -> Result<TeamV4Memory> {
    let source_event_ids_text: String = row.get("source_event_ids");
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4Memory {
        id: row.get("id"),
        run_id: row.get("run_id"),
        task_id: row.get("task_id"),
        kind: row.get("kind"),
        content: row.get("content"),
        confidence: row.get("confidence"),
        source_event_ids: parse_json_text(source_event_ids_text)?,
        accepted_by_orchestrator: row.get("accepted_by_orchestrator"),
        promoted_to_long_term: row.get("promoted_to_long_term"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_harness_run(row: sqlx::sqlite::SqliteRow) -> Result<TeamV4HarnessRun> {
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4HarnessRun {
        id: row.get("id"),
        run_id: row.get("run_id"),
        actor_id: row.get("actor_id"),
        task_id: row.get("task_id"),
        status: row.get("status"),
        lease_expires_at: row.get("lease_expires_at"),
        last_heartbeat_at: row.get("last_heartbeat_at"),
        checkpoint_sequence: row.get("checkpoint_sequence"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn map_harness_run_pg(row: PgRow) -> Result<TeamV4HarnessRun> {
    let metadata_text: String = row.get("metadata");
    Ok(TeamV4HarnessRun {
        id: row.get("id"),
        run_id: row.get("run_id"),
        actor_id: row.get("actor_id"),
        task_id: row.get("task_id"),
        status: row.get("status"),
        lease_expires_at: row.get("lease_expires_at"),
        last_heartbeat_at: row.get("last_heartbeat_at"),
        checkpoint_sequence: row.get("checkpoint_sequence"),
        metadata: parse_json_text(metadata_text)?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
