use crate::models::ai::AiRole;
use crate::services::database::{Database, DatabaseService};
use chrono::Utc;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_ai_roles(db: State<'_, Arc<DatabaseService>>) -> Result<Vec<AiRole>, String> {
    db.get_ai_roles().await.map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateAiRolePayload {
    title: String,
    description: String,
    prompt: String,
}

#[tauri::command]
pub async fn create_ai_role(
    payload: CreateAiRolePayload,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<AiRole, String> {
    let now = Utc::now();
    let role = AiRole {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        description: payload.description,
        prompt: payload.prompt,
        is_system: false,
        created_at: now,
        updated_at: now,
    };
    db.create_ai_role(&role).await.map_err(|e| e.to_string())?;
    Ok(role)
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateAiRolePayload {
    id: String,
    title: String,
    description: String,
    prompt: String,
}

#[tauri::command]
pub async fn update_ai_role(
    payload: UpdateAiRolePayload,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 首先获取角色以保留 is_system 属性
    let roles = db.get_ai_roles().await.map_err(|e| {
        let err_msg = format!("Failed to get AI roles: {}", e);
        tracing::error!("{}", err_msg);
        err_msg
    })?;

    let existing_role = roles.iter().find(|r| r.id == payload.id);
    let is_system = existing_role.map(|r| r.is_system).unwrap_or(false);

    tracing::info!("Updating role: {} (is_system: {})", payload.id, is_system);

    let role = AiRole {
        id: payload.id,
        title: payload.title,
        description: payload.description,
        prompt: payload.prompt,
        is_system: is_system, // 保留原有的 is_system 值
        created_at: existing_role.map(|r| r.created_at).unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
    };

    db.update_ai_role(&role).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_ai_role(id: String, db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    tracing::info!("Attempting to delete AI role with ID: {}", id);

    // 首先检查角色是否存在
    let roles = db.get_ai_roles().await.map_err(|e| {
        let err_msg = format!("Failed to get AI roles: {}", e);
        tracing::error!("{}", err_msg);
        err_msg
    })?;

    let role = roles.iter().find(|r| r.id == id);
    if let Some(role) = role {
        tracing::info!("Deleting role: {} ({})", role.title, id);
        match db.delete_ai_role(&id).await {
            Ok(_) => {
                tracing::info!("Successfully deleted role: {}", id);
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to delete AI role: {}", e);
                tracing::error!("{}", err_msg);
                Err(err_msg)
            }
        }
    } else {
        let err_msg = format!("Role not found with ID: {}", id);
        tracing::error!("{}", err_msg);
        Err(err_msg)
    }
}
