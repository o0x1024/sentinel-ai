// 模块声明
pub mod ai_adapter;
pub mod commands;
pub mod database;
pub mod engines;
pub mod mcp;
pub mod models;
pub mod services;
pub mod tools;
pub mod prompt;


// 导入依赖
use crate::{mcp::{McpClientManager, McpServerManager}, tools::ToolProvider};
use std::sync::Arc;
use tauri::{
    generate_handler,
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tracing_subscriber;
use tracing_appender;
use std::fs;

// 导入服务
use services::{
    ai::AiServiceManager, database::DatabaseService, mcp::McpService,
    scan::ScanService, scan_session::ScanSessionService,
};

// 导入命令
use commands::{
    ai, asset, config, database as db_commands, dictionary, mcp as mcp_commands, performance,
    plan_execute_commands, scan, scan_commands, scan_session_commands, vulnerability,
    window, prompt_commands, rewoo_commands, unified_tools,
};



/// 应用程序主入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    // 创建logs目录 - 使用当前运行目录
    let logs_dir = "logs";
    
    if let Err(e) = fs::create_dir_all(logs_dir) {
        eprintln!("Failed to create logs directory: {}", e);
    } else {
        println!("Logs directory created at: {}", logs_dir);
    }

    // 配置文件日志输出
    let file_appender = tracing_appender::rolling::daily(logs_dir, "sentinel-ai.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sentinel_ai=info".parse().unwrap())
        )
        .with_writer(non_blocking)
        .without_time()  // 不显示时间戳
        .with_line_number(true)  // 显示行号
        .with_ansi(false)  // 禁用ANSI颜色代码
        .init();

    // 保持_guard在整个应用生命周期中
    std::mem::forget(_guard);
        

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
        }))
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
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } => {
                        if button == tauri::tray::MouseButton::Left
                            && button_state == tauri::tray::MouseButtonState::Up
                        {
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

                // 初始化全局工具系统（会自动注册内置工具提供者）
                if let Err(e) = crate::tools::initialize_global_tool_system(db_service.clone()).await {
                    tracing::error!("Failed to initialize global tool system: {}", e);
                } else {
                    tracing::info!("Global tool system initialized successfully");
                }

                // 创建MCP客户端管理器（保留用于MCP服务器连接）
                let client_manager = Arc::new(McpClientManager::new(db_service.clone()));
                
                // 创建MCP服务器管理器（保留用于向外提供MCP服务）
                let server_manager: Arc<McpServerManager> = Arc::new(McpServerManager::new());
                
                // 创建MCP服务（使用统一工具系统）
                let mcp_service = McpService::with_server_manager(client_manager.clone(), server_manager.clone());

                let mut ai_manager = AiServiceManager::new(db_service.clone());
                ai_manager.set_mcp_service(Arc::new(mcp_service.clone()));
                ai_manager.set_app_handle(handle.clone());

                if let Err(e) = ai_manager.init_default_services().await {
                    tracing::error!("Failed to initialize AI services: {}", e);
                } else {
                    tracing::info!("AI services initialized successfully");
                }

                let ai_manager = Arc::new(ai_manager);

                // 初始化扫描会话服务
                let scan_session_service = Arc::new(ScanSessionService::new(db_service.clone()));

                // 初始化扫描服务
                let scan_service = Arc::new(ScanService::new(
                    db_service.clone(),
                    ai_manager.clone(),
                    client_manager.get_client(),
                ));

        
                // 初始化资产服务
                let pool = db_service.get_pool().expect("Database pool not initialized").clone();
                let asset_service = crate::services::AssetService::new(pool);

                handle.manage(db_service);
                handle.manage(client_manager.clone());
                handle.manage(server_manager);
                handle.manage(Arc::new(mcp_service));
                handle.manage(ai_manager);
                handle.manage(scan_session_service);
                handle.manage(scan_service);
                handle.manage(asset_service);
                

                // 初始化Prompt服务状态
                let prompt_service_state: commands::prompt_commands::PromptServiceState = 
                    Arc::new(tokio::sync::RwLock::new(None));
                handle.manage(prompt_service_state);

                // 初始化ReWOO测试状态
                let rewoo_test_state = Arc::new(std::sync::Mutex::new(
                    commands::rewoo_commands::ReWOOTestState::default()
                ));
                handle.manage(rewoo_test_state);

                // 初始化智能调度器状态
                // let intelligent_dispatcher_state: commands::intelligent_dispatcher_commands::IntelligentDispatcherState = 
                //     Arc::new(tokio::sync::RwLock::new(None));
                // handle.manage(intelligent_dispatcher_state);

                // 初始化Plan-Execute引擎状态
                let plan_execute_engine_state: commands::plan_execute_commands::PlanExecuteEngineState = 
                    Arc::new(tokio::sync::RwLock::new(None));
                handle.manage(plan_execute_engine_state);

                // 启动时检查AI窗口并启动同步
                let app_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = client_manager.initialize().await {
                        tracing::error!("Failed to initialize MCP client: {}", e);
                    } else {
                        tracing::info!("MCP client initialized successfully");
                    }
                    
                    // 检查是否存在AI窗口，如果存在则启动同步
                    if let Some(ai_window) = app_handle.get_webview_window("ai-chat") {
                        if ai_window.is_visible().unwrap_or(false) {
                            tracing::info!("AI chat window found on startup, starting window sync");
                            // if let Err(e) = commands::window::start_window_sync(app_handle.clone()).await {
                            //     tracing::error!("Failed to start window sync on startup: {}", e);
                            // }
                        }
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
            ai::get_available_ai_models,
            ai::update_ai_models,
            ai::test_ai_connection,
            ai::get_provider_models,
            ai::save_ai_config,
            ai::get_ai_config,
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
            ai::stop_ai_stream,
            // 调度策略相关命令
            ai::get_scheduler_config,
            ai::save_scheduler_config,
            ai::get_service_for_stage,
            // 数据库相关命令
            db_commands::execute_query,
            db_commands::get_query_history,
            db_commands::clear_query_history,
            // 资产管理相关命令
            asset::init_asset_service,
            asset::create_asset,
            asset::get_asset_detail,
            asset::update_asset,
            asset::delete_asset,
            asset::list_assets,
            asset::get_asset_stats,
            asset::create_asset_relationship,
            asset::import_assets,
            asset::extract_assets_from_scan,
            asset::search_assets,
            asset::get_related_assets,
            asset::verify_asset,
            asset::update_asset_last_seen,
            asset::get_asset_types,
            asset::get_risk_levels,
            asset::get_asset_statuses,
            asset::get_relationship_types,
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
            // 统一工具管理命令
            unified_tools::unified_list_tools,
            unified_tools::unified_search_tools,
            unified_tools::unified_execute_tool,
            unified_tools::unified_execute_batch_tools,
            unified_tools::unified_get_tool_info,
            unified_tools::unified_get_execution_history,
            unified_tools::unified_get_tool_statistics,
            unified_tools::unified_refresh_all_tools,
            unified_tools::unified_clear_execution_history,
            unified_tools::unified_get_tool_categories,
            unified_tools::unified_is_tool_available,
            // MCP服务器管理命令（保留）
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
            commands::mcp::toggle_builtin_tool,
            commands::mcp::get_builtin_tools_with_status,
            commands::check_command_exists,
            commands::role::get_ai_roles,
            commands::role::create_ai_role,
            commands::role::update_ai_role,
            commands::role::delete_ai_role,
            // 扫描工具相关命令
            scan_commands::list_scan_tools,
            scan_commands::get_available_scan_tools,
            scan_commands::start_scan,
            scan_commands::get_scan_result,
            scan_commands::cancel_scan,
            scan_commands::list_running_scans,
            // 扫描任务相关命令
            scan::create_scan_task,
            scan::start_scan_task,
            scan::stop_scan_task,
            scan::get_scan_tasks,
            scan::get_scan_task,
            scan::get_scan_results,
            scan::delete_scan_task,
            scan::get_scan_task_stats,
            scan_session_commands::create_scan_session,
            scan_session_commands::get_scan_session,
            scan_session_commands::update_scan_session,
            scan_session_commands::list_scan_sessions,
            scan_session_commands::delete_scan_session,
            scan_session_commands::get_scan_progress,
            scan_session_commands::get_session_stages,
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
            dictionary::export_subdomain_dictionary,
            // 测试MCP相关命令
            commands::test_mcp::test_mcp_tools_registration,
            commands::test_mcp::test_ai_service_tools,
            commands::test_mcp::get_mcp_tools_status,
            // 窗口管理相关命令
            window::create_ai_chat_window,
            window::close_ai_chat_window,
            window::toggle_ai_chat_window,
            window::get_window_info,
            window::set_window_position,
            window::set_window_size,

            // Prompt相关命令
            prompt_commands::init_prompt_service,
            prompt_commands::get_prompt_service_status,
            prompt_commands::create_prompt_session,
            prompt_commands::get_prompt_session,
            prompt_commands::close_prompt_session,
            prompt_commands::build_prompt,
            prompt_commands::optimize_prompt_config,
            prompt_commands::get_prompt_optimization_suggestions,
            prompt_commands::create_ab_test,
            prompt_commands::record_performance_data,
            prompt_commands::list_prompt_configs,
            prompt_commands::save_prompt_config,
            prompt_commands::list_prompt_templates,
            prompt_commands::save_prompt_template,

            // ReWOO测试相关命令
            rewoo_commands::init_rewoo_engine,
            rewoo_commands::get_rewoo_engine_status,
            rewoo_commands::execute_rewoo_test,
            rewoo_commands::get_test_result,
            rewoo_commands::get_all_test_results,
            rewoo_commands::clear_test_results,
            rewoo_commands::get_predefined_test_configs,
            rewoo_commands::validate_rewoo_config,
            rewoo_commands::get_available_tools,
            rewoo_commands::simulate_tool_execution,

            // 智能调度器相关命令
            // commands::intelligent_dispatcher_commands::intelligent_process_query,
            // commands::intelligent_dispatcher_commands::get_execution_status,
            // commands::intelligent_dispatcher_commands::get_execution_history,
            // commands::intelligent_dispatcher_commands::cancel_execution,
            // commands::intelligent_dispatcher_commands::get_dispatcher_statistics,

            // Plan-Execute引擎相关命令
            plan_execute_commands::start_plan_execute_engine,
            plan_execute_commands::stop_plan_execute_engine,
            plan_execute_commands::get_plan_execute_engine_status,
            plan_execute_commands::dispatch_plan_execute_task,
            plan_execute_commands::get_plan_execute_task_status,
            plan_execute_commands::get_plan_execute_task_result,
            plan_execute_commands::cancel_plan_execute_task,
            plan_execute_commands::get_plan_execute_active_tasks,
            plan_execute_commands::get_plan_execute_task_history,
        ])
        .run(context)
        .expect("Failed to start Tauri application");
}
