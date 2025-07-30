// 模块声明
pub mod commands;
pub mod engines;
pub mod mcp;
pub mod models;
pub mod services;
pub mod tools;

// 导入依赖
use crate::mcp::{McpClientManager, McpServerManager};
use std::sync::Arc;
use tauri::{generate_handler, Manager, WindowEvent, tray::{TrayIconBuilder, TrayIconEvent}, menu::{Menu, MenuItem, PredefinedMenuItem}, image::Image};
use tracing_subscriber;

// 导入服务
use services::{
    ai::AiServiceManager, database::DatabaseService, mcp::McpService,
    scan_session::ScanSessionService,
};
use tools::tool_manager::ToolManager;

// 导入命令
use commands::{
    ai, config, database as db_commands, dictionary, mcp as mcp_commands, performance,
    scan_commands, scan_engine_commands, scan_session_commands, vulnerability,
};

/// 应用程序主入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sentinel_ai=debug".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            
            // 创建系统托盘图标
            let _tray_icon = TrayIconBuilder::with_id("main")
                .tooltip("Sentinel AI")
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click { button, button_state, .. } => {
                            if button == tauri::tray::MouseButton::Left && button_state == tauri::tray::MouseButtonState::Up {
                                let app = tray.app_handle();
                                if let Some(window) = app.get_webview_window("main") {
                                    if window.is_visible().unwrap_or(false) {
                                        let _ = window.hide();
                                    } else {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .icon(app.default_window_icon().unwrap().clone())
                .build(app)?;
            
            tauri::async_runtime::block_on(async move {
                let mut db_service = DatabaseService::new();
                db_service
                    .initialize()
                    .await
                    .expect("Failed to initialize database");
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

                // 初始化工具管理器
                let tool_manager = ToolManager::new(db_service.clone())
                    .await
                    .expect("Failed to initialize tool manager");

                // 初始化扫描会话服务
                let scan_session_service = Arc::new(ScanSessionService::new(db_service.clone()));

                handle.manage(db_service);
                handle.manage(client_manager.clone());
                handle.manage(server_manager);
                handle.manage(mcp_service);
                handle.manage(ai_manager);
                handle.manage(tool_manager);
                handle.manage(scan_session_service);

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
            commands::role::delete_ai_role,
            // 扫描工具相关命令
            scan_commands::list_scan_tools,
            scan_commands::start_scan,
            scan_commands::get_scan_result,
            scan_commands::cancel_scan,
            scan_commands::list_running_scans,
            scan_session_commands::create_scan_session,
            scan_session_commands::get_scan_session,
            scan_session_commands::update_scan_session,
            scan_session_commands::list_scan_sessions,
            scan_session_commands::delete_scan_session,
            scan_session_commands::get_scan_progress,
            scan_session_commands::get_session_stages,
            scan_engine_commands::initialize_scan_engine,
            scan_engine_commands::start_scan_engine,
            scan_engine_commands::stop_scan_engine,
            scan_engine_commands::get_scan_engine_status,
            scan_engine_commands::get_scan_engine_config,
            scan_engine_commands::update_scan_engine_config,
            // 性能监控相关命令
            performance::get_performance_metrics,
            performance::get_performance_report,
            performance::get_optimization_suggestions,
            performance::start_performance_monitoring,
            performance::update_performance_config,
            performance::get_performance_config,
            performance::reset_performance_stats,
            performance::record_operation_timing,
            performance::record_request,
            performance::record_error,
            // 字典管理相关命令
            dictionary::get_dictionaries,
            dictionary::get_dictionary,
            dictionary::create_dictionary,
            dictionary::update_dictionary,
            dictionary::delete_dictionary,
            dictionary::get_dictionary_words,
            dictionary::add_dictionary_words,
            dictionary::remove_dictionary_words,
            dictionary::search_dictionary_words,
            dictionary::clear_dictionary,
            dictionary::export_dictionary,
            dictionary::import_dictionary,
            dictionary::import_dictionary_from_file,
            dictionary::export_dictionary_to_file,
            dictionary::get_dictionary_stats,
            dictionary::create_dictionary_set,
            dictionary::add_dictionary_to_set,
            dictionary::get_set_dictionaries,
            dictionary::initialize_builtin_dictionaries,
            // 兼容性命令（保持与原有子域名字典API的兼容性）
            dictionary::get_subdomain_dictionary,
            dictionary::set_subdomain_dictionary,
            dictionary::add_subdomain_words,
            dictionary::remove_subdomain_words,
            dictionary::reset_subdomain_dictionary,
            dictionary::import_subdomain_dictionary,
            dictionary::export_subdomain_dictionary
        ])
        .run(context)
        .expect("Failed to start Tauri application");
}
