// 数据库管理命令模块

use crate::services::database::DatabaseService;
use sentinel_db::database_service::{
    load_db_config_from_disk, save_db_config_to_disk, DatabaseConfig, DatabaseMigration,
    DatabasePool, DatabaseType,
};
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

// 临时定义QueryHistory结构体，等待数据库模型完善
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryHistory {
    pub id: String,
    pub query: String,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: i64,
    pub result_count: i32,
}

/// 数据库状态信息
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub connected: bool,
    #[serde(rename = "type")]
    pub db_type: String,
    pub size: u64,
    pub tables: i32,
    pub path: String,
    pub last_backup: Option<String>,
}

/// 备份信息
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub path: String,
    pub size: u64,
    pub created_at: String,
}

/// 执行自定义SQL查询
#[tauri::command]
pub async fn execute_query(
    query: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<Value>, String> {
    db_service
        .execute_query(&query)
        .await
        .map_err(|e| e.to_string())
}

/// 获取查询历史（临时简化实现）
#[tauri::command]
pub async fn get_query_history(
    _db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<QueryHistory>, String> {
    // 暂时返回空数组，等数据库模型完善后再实现
    Ok(vec![])
}

/// 清除查询历史（临时简化实现）
#[tauri::command]
pub async fn clear_query_history(
    _db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 暂时返回成功，等数据库模型完善后再实现
    Ok(())
}

/// 获取数据库状态
#[tauri::command]
pub async fn get_database_status(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<DatabaseStatus, String> {
    // 获取数据库统计信息
    let stats = db_service
        .get_stats()
        .await
        .map_err(|e| format!("获取数据库统计信息失败: {}", e))?;

    // 获取数据库路径
    let db_path = db_service.get_db_path();
    tracing::info!("Database path: {}", db_path.display());

    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    let table_count_sql = match db_kind {
        DatabaseType::PostgreSQL => {
            "SELECT COUNT(*) as table_count FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        }
        DatabaseType::MySQL => {
            "SELECT COUNT(*) as table_count FROM information_schema.tables WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'"
        }
        DatabaseType::SQLite => {
            "SELECT COUNT(*) as table_count FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
        }
    };

    let table_count = match db_service.execute_query(table_count_sql).await {
        Ok(rows) => {
            tracing::info!("Table count query result: {:?}", rows);
            if let Some(first) = rows.first() {
                let count = first
                    .get("table_count")
                    .and_then(|v| {
                        v.as_i64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                    })
                    .unwrap_or(0) as i32;
                tracing::info!("Table count: {}", count);
                count
            } else {
                tracing::warn!("No rows returned from table count query");
                0
            }
        }
        Err(e) => {
            tracing::error!("Failed to get table count: {}", e);
            0
        }
    };

    // 检查最后备份时间
    let last_backup = get_last_backup_info(&db_path).map(|info| info.created_at);

    // Get DB type from config
    let (db_type, connection_info) = if let Some(config) = db_service.get_db_config() {
        let type_str = match config.db_type {
            sentinel_db::database_service::DatabaseType::PostgreSQL => "PostgreSQL",
            sentinel_db::database_service::DatabaseType::MySQL => "MySQL",
            sentinel_db::database_service::DatabaseType::SQLite => "SQLite",
        };

        let conn_info = match config.db_type {
            sentinel_db::database_service::DatabaseType::SQLite => config
                .path
                .clone()
                .unwrap_or_else(|| db_path.to_string_lossy().to_string()),
            _ => {
                let default_port = match config.db_type {
                    sentinel_db::database_service::DatabaseType::PostgreSQL => 5432,
                    sentinel_db::database_service::DatabaseType::MySQL => 3306,
                    sentinel_db::database_service::DatabaseType::SQLite => 0,
                };
                format!(
                    "{}:{}",
                    config.host.as_deref().unwrap_or("localhost"),
                    config.port.unwrap_or(default_port)
                )
            }
        };
        (type_str.to_string(), conn_info)
    } else {
        ("SQLite".to_string(), db_path.to_string_lossy().to_string())
    };

    let status = DatabaseStatus {
        connected: true,
        db_type,
        size: stats.db_size_bytes,
        tables: table_count,
        path: connection_info,
        last_backup,
    };

    tracing::info!(
        "Returning database status: path={}, tables={}, size={}",
        status.path,
        status.tables,
        status.size
    );

    Ok(status)
}

/// 获取数据库路径
#[tauri::command]
pub async fn get_database_path(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let path = db_service.get_db_path();
    Ok(path.to_string_lossy().to_string())
}

/// 测试数据库连接
#[tauri::command]
pub async fn test_database_connection(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<bool, String> {
    // 尝试执行简单查询来验证连接
    match db_service.execute_query("SELECT 1").await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("数据库连接测试失败: {}", e)),
    }
}

/// 创建数据库备份
#[tauri::command]
pub async fn create_database_backup(
    backup_path: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let path = backup_path.map(PathBuf::from);

    let result_path = db_service
        .backup(path)
        .await
        .map_err(|e| format!("创建备份失败: {}", e))?;

    Ok(result_path.to_string_lossy().to_string())
}

/// 恢复数据库备份
#[tauri::command]
pub async fn restore_database_backup(
    backup_path: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let path = PathBuf::from(backup_path);

    if !path.exists() {
        return Err("备份文件不存在".to_string());
    }

    db_service
        .restore(path)
        .await
        .map_err(|e| format!("恢复备份失败: {}", e))
}

/// 优化数据库（VACUUM）
#[tauri::command]
pub async fn optimize_database(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    match db_kind {
        DatabaseType::PostgreSQL | DatabaseType::SQLite => {
            db_service
                .execute_query("VACUUM")
                .await
                .map_err(|e| format!("优化数据库失败: {}", e))?;

            db_service
                .execute_query("ANALYZE")
                .await
                .map_err(|e| format!("更新统计信息失败: {}", e))?;
        }
        DatabaseType::MySQL => {
            let tables = db_service
                .execute_query("SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE()")
                .await
                .map_err(|e| format!("获取 MySQL 表列表失败: {}", e))?;

            for table in tables {
                if let Some(name) = table.get("table_name").and_then(|v| v.as_str()) {
                    let sql = format!("OPTIMIZE TABLE `{}`", name);
                    let _ = db_service.execute_query(&sql).await;
                }
            }
        }
    }

    Ok("数据库优化完成".to_string())
}

/// 重建数据库索引
#[tauri::command]
pub async fn rebuild_database_indexes(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    let index_sql = match db_kind {
        DatabaseType::PostgreSQL => {
            "SELECT indexname as name FROM pg_indexes WHERE schemaname = 'public'"
        }
        DatabaseType::MySQL => {
            "SELECT DISTINCT index_name as name FROM information_schema.statistics WHERE table_schema = DATABASE() AND index_name <> 'PRIMARY'"
        }
        DatabaseType::SQLite => {
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name NOT LIKE 'sqlite_%'"
        }
    };

    let indexes = db_service
        .execute_query(index_sql)
        .await
        .map_err(|e| format!("获取索引列表失败: {}", e))?;

    let mut rebuilt_count = 0;

    for index in indexes {
        if let Some(name) = index.get("name").and_then(|v| v.as_str()) {
            let query = match db_kind {
                DatabaseType::PostgreSQL => format!("REINDEX INDEX \"{}\"", name),
                DatabaseType::SQLite => format!("REINDEX \"{}\"", name),
                DatabaseType::MySQL => {
                    // MySQL has no direct REINDEX command; handled by OPTIMIZE TABLE in optimize_database.
                    continue;
                }
            };
            if db_service.execute_query(&query).await.is_ok() {
                rebuilt_count += 1;
            }
        }
    }

    Ok(format!("已重建 {} 个索引", rebuilt_count))
}

/// 清理旧数据
#[tauri::command]
pub async fn cleanup_database(
    retention_days: i32,
    cleanup_logs: bool,
    cleanup_temp_files: bool,
    cleanup_old_sessions: bool,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let mut deleted_count = 0;
    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    let (logs_query, sessions_query) = match db_kind {
        DatabaseType::PostgreSQL => (
            format!(
                "DELETE FROM agent_session_logs WHERE created_at < NOW() - INTERVAL '{} days'",
                retention_days
            ),
            format!(
                "DELETE FROM agent_sessions WHERE created_at < NOW() - INTERVAL '{} days'",
                retention_days
            ),
        ),
        DatabaseType::MySQL => (
            format!(
                "DELETE FROM agent_session_logs WHERE created_at < DATE_SUB(NOW(), INTERVAL {} DAY)",
                retention_days
            ),
            format!(
                "DELETE FROM agent_sessions WHERE created_at < DATE_SUB(NOW(), INTERVAL {} DAY)",
                retention_days
            ),
        ),
        DatabaseType::SQLite => (
            format!(
                "DELETE FROM agent_session_logs WHERE created_at < datetime('now', '-{} days')",
                retention_days
            ),
            format!(
                "DELETE FROM agent_sessions WHERE created_at < datetime('now', '-{} days')",
                retention_days
            ),
        ),
    };

    // 清理旧日志
    if cleanup_logs {
        if let Ok(result) = db_service.execute_query(&logs_query).await {
            // 统计删除数量
            deleted_count += result.len();
        }
    }

    // 清理旧会话
    if cleanup_old_sessions {
        if let Ok(result) = db_service.execute_query(&sessions_query).await {
            deleted_count += result.len();
        }
    }

    // 清理临时文件记录
    if cleanup_temp_files {
        // 可以在这里添加清理临时文件的逻辑
        tracing::info!("Cleaning up temp files...");
    }

    // 最后执行 VACUUM 回收空间
    let _ = db_service.execute_query("VACUUM").await;

    Ok(format!("清理完成，共清理 {} 条记录", deleted_count))
}

/// 列出所有备份文件
#[tauri::command]
pub async fn list_database_backups(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BackupInfo>, String> {
    let db_path = db_service.get_db_path();
    let default_path = PathBuf::from(".");
    let backup_dir = db_path.parent().unwrap_or(&default_path);

    let mut backups = Vec::new();

    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("backup_") && name.ends_with(".db") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        let created = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .ok()
                            .map(|t| {
                                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                            .unwrap_or_else(|| "Unknown".to_string());

                        backups.push(BackupInfo {
                            path: path.to_string_lossy().to_string(),
                            size: metadata.len(),
                            created_at: created,
                        });
                    }
                } else if name.starts_with("backup_") && name.ends_with(".sql") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        let created = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .ok()
                            .map(|t| {
                                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                            .unwrap_or_else(|| "Unknown".to_string());

                        backups.push(BackupInfo {
                            path: path.to_string_lossy().to_string(),
                            size: metadata.len(),
                            created_at: created,
                        });
                    }
                }
            }
        }
    }

    // 按创建时间倒序排序
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backups)
}

/// 删除备份文件
#[tauri::command]
pub async fn delete_database_backup(backup_path: String) -> Result<(), String> {
    let path = PathBuf::from(&backup_path);

    if !path.exists() {
        return Err("备份文件不存在".to_string());
    }

    // 确保只能删除备份文件
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let valid_ext = name.ends_with(".db") || name.ends_with(".sql");
        if !name.starts_with("backup_") || !valid_ext {
            return Err("只能删除备份文件".to_string());
        }
    } else {
        return Err("无效的文件路径".to_string());
    }

    std::fs::remove_file(&path).map_err(|e| format!("删除备份失败: {}", e))
}

/// 导出数据为 JSON
#[tauri::command]
pub async fn export_database_json(
    tables: Vec<String>,
    output_path: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let mut export_data = serde_json::Map::new();
    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    for table in tables {
        let query = match db_kind {
            DatabaseType::MySQL => format!("SELECT * FROM `{}`", table),
            _ => format!("SELECT * FROM \"{}\"", table),
        };
        match db_service.execute_query(&query).await {
            Ok(rows) => {
                export_data.insert(table, serde_json::Value::Array(rows));
            }
            Err(e) => {
                tracing::warn!("Failed to export table {}: {}", table, e);
            }
        }
    }

    let json =
        serde_json::to_string_pretty(&export_data).map_err(|e| format!("序列化失败: {}", e))?;

    std::fs::write(&output_path, json).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(output_path)
}

/// 导入 JSON 数据（谨慎使用）
#[tauri::command]
pub async fn import_database_json(
    input_path: String,
    _db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let content =
        std::fs::read_to_string(&input_path).map_err(|e| format!("读取文件失败: {}", e))?;

    let data: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let mut imported_count = 0;

    if let Some(obj) = data.as_object() {
        for (table, rows) in obj {
            if let Some(arr) = rows.as_array() {
                for row in arr {
                    // 这里仅记录导入信息，实际导入逻辑需要根据表结构具体实现
                    tracing::info!("Would import row to table {}: {:?}", table, row);
                    imported_count += 1;
                }
            }
        }
    }

    // 注意：实际导入逻辑需要根据具体表结构实现
    // 这里仅返回提示信息
    Ok(format!(
        "解析完成，共 {} 条记录待导入（实际导入功能待实现）",
        imported_count
    ))
}

/// 获取数据库统计详情
#[tauri::command]
pub async fn get_database_statistics(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Value, String> {
    let stats = db_service
        .get_stats()
        .await
        .map_err(|e| format!("获取统计信息失败: {}", e))?;

    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    let table_info_sql = match db_kind {
        DatabaseType::PostgreSQL => {
            "SELECT table_name, 0 as column_count FROM information_schema.tables WHERE table_schema = 'public'"
        }
        DatabaseType::MySQL => {
            "SELECT table_name, 0 as column_count FROM information_schema.tables WHERE table_schema = DATABASE()"
        }
        DatabaseType::SQLite => {
            "SELECT name as table_name, 0 as column_count FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        }
    };

    let table_info = db_service
        .execute_query(table_info_sql)
        .await
        .unwrap_or_default();

    Ok(serde_json::json!({
        "scan_tasks_count": stats.scan_tasks_count,
        "vulnerabilities_count": stats.vulnerabilities_count,
        "assets_count": stats.assets_count,
        "conversations_count": stats.conversations_count,
        "db_size_bytes": stats.db_size_bytes,
        "db_size_formatted": format_file_size(stats.db_size_bytes),
        "tables": table_info,
        "last_backup": stats.last_backup
    }))
}

/// 重置数据库（危险操作）
#[tauri::command]
pub async fn reset_database(
    confirm_text: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    // 需要输入确认文本
    if confirm_text != "CONFIRM_RESET" {
        return Err("确认文本不正确，请输入 'CONFIRM_RESET'".to_string());
    }

    // 首先创建备份
    let backup_path = db_service
        .backup(None)
        .await
        .map_err(|e| format!("创建备份失败: {}", e))?;

    tracing::warn!(
        "Database reset initiated. Backup created at: {:?}",
        backup_path
    );
    let db_kind = db_service
        .get_db_config()
        .map(|c| c.db_type.clone())
        .unwrap_or(DatabaseType::SQLite);

    // 删除所有用户数据表的内容
    let tables_to_clear = vec![
        "scan_tasks",
        "vulnerabilities",
        "assets",
        "ai_conversations",
        "ai_messages",
        "agent_tasks",
        "agent_sessions",
        "agent_session_logs",
        "agent_execution_results",
        "agent_execution_steps",
    ];

    for table in &tables_to_clear {
        let query = match db_kind {
            DatabaseType::MySQL => format!("DELETE FROM `{}`", table),
            _ => format!("DELETE FROM \"{}\"", table),
        };
        let _ = db_service.execute_query(&query).await;
    }

    // 执行 VACUUM 回收空间
    let _ = db_service.execute_query("VACUUM").await;

    Ok(format!(
        "数据库已重置。备份已保存到: {}",
        backup_path.to_string_lossy()
    ))
}

// 辅助函数

fn get_last_backup_info(db_path: &PathBuf) -> Option<BackupInfo> {
    let backup_dir = db_path.parent()?;

    let mut latest_backup: Option<BackupInfo> = None;

    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("backup_") && (name.ends_with(".db") || name.ends_with(".sql"))
                {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        let created = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .ok()
                            .map(|t| {
                                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                            .unwrap_or_else(|| "Unknown".to_string());

                        let info = BackupInfo {
                            path: path.to_string_lossy().to_string(),
                            size: metadata.len(),
                            created_at: created.clone(),
                        };

                        if let Some(ref current) = latest_backup {
                            if created > current.created_at {
                                latest_backup = Some(info);
                            }
                        } else {
                            latest_backup = Some(info);
                        }
                    }
                }
            }
        }
    }

    latest_backup
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// New database migration commands

/// Test database connection with given config
#[tauri::command]
pub async fn test_db_connection(config: DatabaseConfig) -> Result<bool, String> {
    let pool = DatabasePool::connect(&config)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    pool.test_connection()
        .await
        .map_err(|e| format!("Connection test failed: {}", e))?;

    Ok(true)
}

/// Export database to JSON file
#[tauri::command]
pub async fn export_db_to_json(
    output_path: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let source_config = db_service
        .get_db_config()
        .cloned()
        .ok_or_else(|| "Failed to read current database config".to_string())?;
    let db_pool = DatabasePool::connect(&source_config)
        .await
        .map_err(|e| format!("Failed to connect source database: {}", e))?;
    let migration = DatabaseMigration::new(db_pool);

    migration
        .export_to_json(&output_path)
        .await
        .map_err(|e| format!("Export failed: {}", e))?;

    Ok(output_path)
}

/// Export database to SQL file
#[tauri::command]
pub async fn export_db_to_sql(
    output_path: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let source_config = db_service
        .get_db_config()
        .cloned()
        .ok_or_else(|| "Failed to read current database config".to_string())?;
    let db_pool = DatabasePool::connect(&source_config)
        .await
        .map_err(|e| format!("Failed to connect source database: {}", e))?;
    let migration = DatabaseMigration::new(db_pool);

    migration
        .export_to_sql(&output_path)
        .await
        .map_err(|e| format!("Export failed: {}", e))?;

    Ok(output_path)
}

/// Import database from JSON file
#[tauri::command]
pub async fn import_db_from_json(
    input_path: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let source_config = db_service
        .get_db_config()
        .cloned()
        .ok_or_else(|| "Failed to read current database config".to_string())?;
    let db_pool = DatabasePool::connect(&source_config)
        .await
        .map_err(|e| format!("Failed to connect source database: {}", e))?;
    let migration = DatabaseMigration::new(db_pool);

    migration
        .import_from_json(&input_path)
        .await
        .map_err(|e| format!("Import failed: {}", e))?;

    Ok("Import completed successfully".to_string())
}

/// Migrate database to another database type
#[tauri::command]
pub async fn migrate_database(
    target_config: DatabaseConfig,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let source_config = db_service
        .get_db_config()
        .cloned()
        .ok_or_else(|| "Failed to read current database config".to_string())?;
    let source_pool = DatabasePool::connect(&source_config)
        .await
        .map_err(|e| format!("Failed to connect source database: {}", e))?;

    // Connect to target database
    let target_pool = DatabasePool::connect(&target_config)
        .await
        .map_err(|e| format!("Failed to connect to target database: {}", e))?;

    let migration = DatabaseMigration::new(source_pool).with_target(target_pool);

    migration
        .migrate()
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;

    Ok(format!(
        "Migration to {:?} completed successfully",
        target_config.db_type
    ))
}

/// Save database configuration to a persistent file
#[tauri::command]
pub async fn save_db_config(config: DatabaseConfig) -> Result<String, String> {
    let config_path =
        save_db_config_to_disk(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    Ok(format!("Database config saved to {:?}", config_path))
}

/// Load database configuration from the persistent file
#[tauri::command]
pub async fn load_db_config() -> Result<Option<DatabaseConfig>, String> {
    load_db_config_from_disk().map_err(|e| format!("Failed to load config: {}", e))
}
