// 项目管理命令模块
// 暂时简化实现，等架构稳定后再完善

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub status: String,
}

// 暂时提供一个简单的获取项目列表命令
#[tauri::command]
pub async fn get_projects() -> Result<Vec<ProjectInfo>, String> {
    // 暂时返回空列表，等数据库服务完全稳定后再实现
    Ok(vec![])
}

// 其他项目相关命令等架构稳定后再实现... 