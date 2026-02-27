//! Agent Team 运行时数据库分发层（PostgreSQL / SQLite）

use anyhow::{anyhow, Result};
use sentinel_db::database_service::connection_manager::DatabasePool;

use super::models::*;
use super::repository as pg_repo;
use super::repository_sqlite as sqlite_repo;

pub async fn create_agent_team_template(
    pool: &DatabasePool,
    req: &CreateAgentTeamTemplateRequest,
    created_by: Option<&str>,
) -> Result<AgentTeamTemplate> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::create_agent_team_template(p, req, created_by).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::create_agent_team_template(p, req, created_by).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn list_agent_team_templates(
    pool: &DatabasePool,
    domain: Option<&str>,
) -> Result<Vec<AgentTeamTemplate>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::list_agent_team_templates(p, domain).await,
        DatabasePool::SQLite(p) => sqlite_repo::list_agent_team_templates(p, domain).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_agent_team_template_detail(
    pool: &DatabasePool,
    id: &str,
) -> Result<Option<AgentTeamTemplate>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_agent_team_template_detail(p, id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_agent_team_template_detail(p, id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn update_agent_team_template(
    pool: &DatabasePool,
    id: &str,
    req: &UpdateAgentTeamTemplateRequest,
) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::update_agent_team_template(p, id, req).await,
        DatabasePool::SQLite(p) => sqlite_repo::update_agent_team_template(p, id, req).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn delete_agent_team_template(pool: &DatabasePool, id: &str) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::delete_agent_team_template(p, id).await,
        DatabasePool::SQLite(p) => sqlite_repo::delete_agent_team_template(p, id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn seed_builtin_templates(pool: &DatabasePool) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::seed_builtin_templates(p).await,
        DatabasePool::SQLite(p) => sqlite_repo::seed_builtin_templates(p).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn create_agent_team_session(
    pool: &DatabasePool,
    req: &CreateAgentTeamSessionRequest,
) -> Result<AgentTeamSession> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::create_agent_team_session(p, req).await,
        DatabasePool::SQLite(p) => sqlite_repo::create_agent_team_session(p, req).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_agent_team_session(
    pool: &DatabasePool,
    id: &str,
) -> Result<Option<AgentTeamSession>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_agent_team_session(p, id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_agent_team_session(p, id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn list_agent_team_sessions(
    pool: &DatabasePool,
    conversation_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentTeamSession>> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::list_agent_team_sessions(p, conversation_id, limit, offset).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::list_agent_team_sessions(p, conversation_id, limit, offset).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn update_agent_team_session(
    pool: &DatabasePool,
    id: &str,
    req: &UpdateAgentTeamSessionRequest,
) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::update_agent_team_session(p, id, req).await,
        DatabasePool::SQLite(p) => sqlite_repo::update_agent_team_session(p, id, req).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn delete_agent_team_session(pool: &DatabasePool, id: &str) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::delete_agent_team_session(p, id).await,
        DatabasePool::SQLite(p) => sqlite_repo::delete_agent_team_session(p, id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn update_session_state(
    pool: &DatabasePool,
    session_id: &str,
    state: &str,
) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::update_session_state(p, session_id, state).await,
        DatabasePool::SQLite(p) => sqlite_repo::update_session_state(p, session_id, state).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn create_round(
    pool: &DatabasePool,
    session_id: &str,
    round_number: i32,
    phase: &str,
) -> Result<AgentTeamRound> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::create_round(p, session_id, round_number, phase).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::create_round(p, session_id, round_number, phase).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn complete_round(
    pool: &DatabasePool,
    round_id: &str,
    divergence_score: Option<f64>,
) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::complete_round(p, round_id, divergence_score).await,
        DatabasePool::SQLite(p) => sqlite_repo::complete_round(p, round_id, divergence_score).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_rounds(pool: &DatabasePool, session_id: &str) -> Result<Vec<AgentTeamRound>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_rounds(p, session_id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_rounds(p, session_id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn create_message(
    pool: &DatabasePool,
    session_id: &str,
    round_id: Option<&str>,
    member_id: Option<&str>,
    member_name: Option<&str>,
    role: &str,
    content: &str,
    token_count: Option<i32>,
) -> Result<AgentTeamMessage> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::create_message(
                p,
                session_id,
                round_id,
                member_id,
                member_name,
                role,
                content,
                token_count,
            )
            .await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::create_message(
                p,
                session_id,
                round_id,
                member_id,
                member_name,
                role,
                content,
                token_count,
            )
            .await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn update_message_tool_calls(
    pool: &DatabasePool,
    message_id: &str,
    tool_calls: &serde_json::Value,
) -> Result<()> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::update_message_tool_calls(p, message_id, tool_calls).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::update_message_tool_calls(p, message_id, tool_calls).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_messages(pool: &DatabasePool, session_id: &str) -> Result<Vec<AgentTeamMessage>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_messages(p, session_id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_messages(p, session_id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn upsert_blackboard_entry(
    pool: &DatabasePool,
    req: &UpdateBlackboardRequest,
) -> Result<AgentTeamBlackboardEntry> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::upsert_blackboard_entry(p, req).await,
        DatabasePool::SQLite(p) => sqlite_repo::upsert_blackboard_entry(p, req).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_blackboard_entries(
    pool: &DatabasePool,
    session_id: &str,
) -> Result<Vec<AgentTeamBlackboardEntry>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_blackboard_entries(p, session_id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_blackboard_entries(p, session_id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn resolve_blackboard_entry(
    pool: &DatabasePool,
    session_id: &str,
    entry_id: &str,
) -> Result<AgentTeamBlackboardEntry> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::resolve_blackboard_entry(p, session_id, entry_id).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::resolve_blackboard_entry(p, session_id, entry_id).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_blackboard_entry_archive(
    pool: &DatabasePool,
    session_id: &str,
    entry_id: &str,
    limit: i64,
) -> Result<AgentTeamBlackboardArchive> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::get_blackboard_entry_archive(p, session_id, entry_id, limit).await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::get_blackboard_entry_archive(p, session_id, entry_id, limit).await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn create_artifact(
    pool: &DatabasePool,
    session_id: &str,
    artifact_type: &str,
    title: &str,
    content: &str,
    created_by: Option<&str>,
    parent_artifact_id: Option<&str>,
    diff_summary: Option<&str>,
) -> Result<AgentTeamArtifact> {
    match pool {
        DatabasePool::PostgreSQL(p) => {
            pg_repo::create_artifact(
                p,
                session_id,
                artifact_type,
                title,
                content,
                created_by,
                parent_artifact_id,
                diff_summary,
            )
            .await
        }
        DatabasePool::SQLite(p) => {
            sqlite_repo::create_artifact(
                p,
                session_id,
                artifact_type,
                title,
                content,
                created_by,
                parent_artifact_id,
                diff_summary,
            )
            .await
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn list_artifacts(
    pool: &DatabasePool,
    session_id: &str,
) -> Result<Vec<AgentTeamArtifact>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::list_artifacts(p, session_id).await,
        DatabasePool::SQLite(p) => sqlite_repo::list_artifacts(p, session_id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}

pub async fn get_artifact_detail(
    pool: &DatabasePool,
    artifact_id: &str,
) -> Result<Option<AgentTeamArtifact>> {
    match pool {
        DatabasePool::PostgreSQL(p) => pg_repo::get_artifact_detail(p, artifact_id).await,
        DatabasePool::SQLite(p) => sqlite_repo::get_artifact_detail(p, artifact_id).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Agent Team 暂不支持 MySQL")),
    }
}
