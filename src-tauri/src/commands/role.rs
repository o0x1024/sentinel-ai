use sentinel_core::models::ai::AiRole;
use crate::services::database::{Database, DatabaseService};
use chrono::Utc;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_ai_roles(db: State<'_, Arc<DatabaseService>>) -> Result<Vec<AiRole>, String> {
    db.inner().get_ai_roles().await.map_err(|e| e.to_string())
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
    db.inner().create_ai_role(&role).await.map_err(|e| e.to_string())?;
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
    let roles = db.inner().get_ai_roles().await.map_err(|e| {
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
        is_system, // 保留原有的 is_system 值
        created_at: existing_role.map(|r| r.created_at).unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
    };

    db.inner().update_ai_role(&role).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_ai_role(id: String, db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    tracing::info!("Attempting to delete AI role with ID: {}", id);

    // 首先检查角色是否存在
    let roles = db.inner().get_ai_roles().await.map_err(|e| {
        let err_msg = format!("Failed to get AI roles: {}", e);
        tracing::error!("{}", err_msg);
        err_msg
    })?;

    let role = roles.iter().find(|r| r.id == id);
    if let Some(role) = role {
        tracing::info!("Deleting role: {} ({})", role.title, id);
        
        // 如果删除的是当前选中的角色，清除选择
        if let Ok(Some(current_role)) = db.inner().get_current_ai_role().await {
            if current_role.id == id {
                let _ = db.inner().set_current_ai_role(None).await;
            }
        }
        
        match db.inner().delete_ai_role(&id).await {
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

#[tauri::command]
pub async fn set_current_ai_role(
    role_id: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    tracing::info!("Setting current AI role to: {:?}", role_id);
    
    match role_id.as_deref() {
        Some(id) => {
            // 验证角色是否存在
            let roles = db.inner().get_ai_roles().await.map_err(|e| {
                let err_msg = format!("Failed to get AI roles: {}", e);
                tracing::error!("{}", err_msg);
                err_msg
            })?;
            
            if !roles.iter().any(|r| r.id == id) {
                let err_msg = format!("Role not found with ID: {}", id);
                tracing::error!("{}", err_msg);
                return Err(err_msg);
            }
            
            db.inner().set_current_ai_role(Some(id)).await.map_err(|e| {
                let err_msg = format!("Failed to set current AI role: {}", e);
                tracing::error!("{}", err_msg);
                err_msg
            })
        }
        None => {
            db.inner().set_current_ai_role(None).await.map_err(|e| {
                let err_msg = format!("Failed to clear current AI role: {}", e);
                tracing::error!("{}", err_msg);
                err_msg
            })
        }
    }
}

#[tauri::command]
pub async fn get_current_ai_role(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Option<AiRole>, String> {
    db.inner().get_current_ai_role().await.map_err(|e| {
        let err_msg = format!("Failed to get current AI role: {}", e);
        tracing::error!("{}", err_msg);
        err_msg
    })
}
