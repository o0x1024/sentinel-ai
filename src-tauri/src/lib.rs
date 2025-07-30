// 模块声明
pub mod commands;
pub mod models;
pub mod services;
pub mod mcp;

// 导入依赖
use tauri::{generate_handler, Manager};
use tracing_subscriber;
use std::sync::Arc;
use crate::mcp::{McpClientManager, McpServerManager};
use tokio::sync::Mutex;

// 导入服务
use services::{database::DatabaseService, mcp::McpService, ai::AiServiceManager};

// 导入命令
use commands::{ai, database as db_commands, mcp as mcp_commands, config, vulnerability};

/// 应用程序主入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sentinel_ai=debug".parse().unwrap())
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let mut db_service = DatabaseService::new();
                db_service.initialize().await.expect("Failed to initialize database");
                let db_service = Arc::new(db_service);

                let client_manager = Arc::new(McpClientManager::new(db_service.clone()));
                let server_manager = Arc::new(McpServerManager::new());
                let mcp_service = Arc::new(McpService::new(client_manager.clone()));

                let mut ai_manager = AiServiceManager::new(db_service.clone());
                ai_manager.set_mcp_service(mcp_service.clone());
                ai_manager.set_app_handle(handle.clone());

                if let Err(e) = ai_manager.init_default_services().await {
                    tracing::error!("Failed to initialize AI services: {}", e);
                } else {
                    tracing::info!("AI services initialized successfully");
                }
                
                let ai_manager = Arc::new(ai_manager);
                
                handle.manage(db_service);
                handle.manage(client_manager.clone());
                handle.manage(server_manager);
                handle.manage(mcp_service);
                handle.manage(ai_manager);

                tauri::async_runtime::spawn(async move {
                    if let Err(e) = client_manager.initialize().await {
                        tracing::error!("Failed to initialize MCP client: {}", e);
                    } else {
                        tracing::info!("MCP client initialized successfully");
                    }
                });
            });

            Ok(())
        })
        .invoke_handler(generate_handler![
            // AI 相关命令
            ai::list_ai_services,
            ai::add_ai_service,
            ai::remove_ai_service,
            ai::create_ai_conversation,
            ai::send_ai_message,
            ai::send_ai_message_stream,
            ai::get_ai_conversations,
            ai::get_ai_conversation_history,
            ai::delete_ai_conversation,
            ai::update_ai_conversation_title,
            ai::archive_ai_conversation,
            ai::get_ai_service_info,
            ai::configure_ai_service,
            ai::execute_ai_tool_call,
            ai::get_available_ai_models,
            ai::update_ai_models,
            ai::test_ai_connection,
            ai::save_ai_config,
            ai::get_ai_usage_stats,
            ai::reload_ai_services,
            ai::get_ai_service_status,
            ai::print_ai_conversations,
            ai::get_ai_chat_models,
            ai::get_ai_embedding_models,
            ai::get_default_ai_model,
            ai::set_default_ai_model,
            ai::get_ai_model_config,
            ai::update_ai_model_config,
            ai::save_ai_providers_config,

            // 数据库相关命令
            db_commands::execute_query,
            db_commands::get_query_history,
            db_commands::clear_query_history,
            
            // 漏洞相关命令
            vulnerability::list_vulnerabilities,
            vulnerability::get_vulnerability,
            vulnerability::update_vulnerability_status,
            vulnerability::generate_vulnerability_report,
            vulnerability::verify_vulnerability,
            vulnerability::delete_vulnerability,
            vulnerability::get_vulnerability_stats,
            
            // 配置相关命令
            config::save_config,
            config::get_config,
            config::delete_config,
            config::get_config_categories,
            config::save_config_batch,
            config::set_config,
            config::get_theme,
            config::set_theme,
            config::get_language,
            config::set_language,
            
            // MCP相关命令
            mcp_commands::list_tools,
            mcp_commands::execute_tool,
            mcp_commands::get_tool_info,
            mcp_commands::get_connection_info,
            mcp_commands::get_mcp_tools,
            mcp_commands::execute_mcp_tool,
            mcp_commands::get_execution_result,
            mcp_commands::mcp_check_server_status,
            mcp_commands::mcp_get_connections,
            mcp_commands::start_mcp_server,
            mcp_commands::stop_mcp_server,
            mcp_commands::mcp_connect_server,
            mcp_commands::mcp_disconnect_server,
            mcp_commands::mcp_list_tools,
            mcp_commands::mcp_start_tool,
            mcp_commands::mcp_stop_tool,
            mcp_commands::mcp_restart_tool,
            mcp_commands::mcp_uninstall_tool,
            mcp_commands::mcp_install_tool,
            mcp_commands::mcp_install_tool_from_url,
            mcp_commands::mcp_install_tool_from_github,
            mcp_commands::mcp_install_tool_from_registry,
            mcp_commands::mcp_create_custom_tool,
            commands::mcp::add_child_process_mcp_server,
            commands::mcp::quick_create_mcp_server,
            commands::mcp::import_mcp_servers_from_json,
            commands::mcp::get_mcp_marketplace_servers,
            commands::mcp::mcp_get_connection_tools,
            commands::mcp::mcp_update_server_config,
            commands::check_command_exists,
            commands::role::get_ai_roles,
            commands::role::create_ai_role,
            commands::role::update_ai_role,
            commands::role::delete_ai_role
        ])
    .run(context)
    .expect("Failed to start Tauri application");
}
