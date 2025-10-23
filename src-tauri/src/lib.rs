// 模块声明
pub mod agents;
pub mod commands;
pub mod database;
pub mod engines;
pub mod managers;
pub mod models;
pub mod services;
pub mod tools;  // 包含原 MCP 功能
pub mod prompt;
pub mod utils;
pub mod rag;


use crate::commands::{get_active_rag_collections, set_rag_collection_active};
// 导入依赖
use crate::tools::{McpClientManager, McpServerManager};
use crate::services::mcp::McpService;
use std::sync::Arc;
use tauri::{
    generate_handler,
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tokio::sync::RwLock;
use tracing_subscriber;
use tracing_appender;
use std::fs;

// 导入服务
use services::{
    ai::AiServiceManager, database::DatabaseService, 
    scan::ScanService, scan_session::ScanSessionService,
};

// 导入命令
use commands::{
    agent_commands, ai, ai_commands, asset, config, database as db_commands, dictionary,
    mcp as mcp_commands, performance,
    plan_execute_commands, rag_commands, scan, scan_commands, scan_session_commands, vulnerability,
    window, prompt_commands, rewoo_commands, unified_tools,
};

// Simple proxy configuration structure
#[derive(Debug, Clone)]
struct ProxyConfig {
    enabled: bool,
    scheme: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    no_proxy: Option<String>,
}

// Placeholder for proxy functions - removed ai_adapter dependency
fn set_global_proxy(_config: Option<ProxyConfig>) {
    // Proxy functionality removed with ai_adapter
    tracing::debug!("Proxy configuration ignored - ai_adapter removed");
}



#[cfg(unix)]
use signal_hook::consts::signal::*;
#[cfg(unix)]
use signal_hook_tokio::Signals;

/// 设置信号处理器
#[cfg(unix)]
async fn setup_signal_handlers(
    mcp_client_manager: Arc<McpClientManager>,
    mcp_service: Arc<McpService>,
) {
    use futures::stream::StreamExt;
    
    let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT]).expect("Failed to register signal handler");
    let handle = signals.handle();

    let signals_task = signals.for_each(move |signal| {
        let mcp_client_manager = mcp_client_manager.clone();
        let mcp_service = mcp_service.clone();
        
        async move {
            match signal {
                SIGHUP => {
                    tracing::warn!("Received SIGHUP signal, performing graceful shutdown of MCP connections");
                    
                    // 保存MCP服务器状态
                    let is_running = mcp_service.is_server_running().await;
                    if let Err(e) = mcp_service.save_server_state("builtin_security_tools", is_running).await {
                        tracing::error!("Failed to save MCP server state on SIGHUP: {}", e);
                    }
                    
                    // 优雅关闭所有MCP连接
                    if let Err(e) = mcp_client_manager.shutdown_all().await {
                        tracing::error!("Failed to shutdown MCP connections on SIGHUP: {}", e);
                    } else {
                        tracing::info!("Successfully shutdown all MCP connections on SIGHUP");
                    }
                }
                SIGTERM | SIGINT => {
                    tracing::warn!("Received termination signal ({}), shutting down gracefully", signal);
                    
                    // 保存状态并退出
                    let is_running = mcp_service.is_server_running().await;
                    if let Err(e) = mcp_service.save_server_state("builtin_security_tools", is_running).await {
                        tracing::error!("Failed to save MCP server state on exit: {}", e);
                    }
                    
                    if let Err(e) = mcp_client_manager.shutdown_all().await {
                        tracing::error!("Failed to shutdown MCP connections on exit: {}", e);
                    }
                    
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    tokio::spawn(signals_task);
    
    // 防止信号处理器被过早释放
    std::mem::forget(handle);
    
    tracing::debug!("Signal handlers set up successfully");
}

#[cfg(not(unix))]
async fn setup_signal_handlers(
    _mcp_client_manager: Arc<McpClientManager>,
    _mcp_service: Arc<McpService>,
) {
    tracing::info!("Signal handling not available on this platform");
}

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
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // 检查是否是最后一个窗口
                let app_handle = window.app_handle();
                let windows = app_handle.webview_windows();
                
                if windows.len() <= 1 {
                    // 最后一个窗口，执行清理
                    let app_handle_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // 保存MCP服务器状态
                        if let Some(mcp_service) = app_handle_clone.try_state::<Arc<McpService>>() {
                            // 检查当前状态并保存
                            let is_running = mcp_service.is_server_running().await;
                            if let Err(e) = mcp_service.save_server_state("builtin_security_tools", is_running).await {
                                eprintln!("Failed to save MCP server state on exit: {}", e);
                            } else {
                                println!("MCP server state saved on exit: enabled={}", is_running);
                            }
                        }
                        
                        // 关闭MCP进程
                        if let Some(mcp_manager) = app_handle_clone.try_state::<Arc<crate::tools::client::McpClientManager>>() {
                            if let Err(e) = mcp_manager.shutdown_all().await {
                                eprintln!("Failed to shutdown MCP processes: {}", e);
                            }
                        }
                    });
                }
                
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
                    tracing::debug!("Global tool system initialized successfully");
                }

                // 初始化全局RAG服务
                if let Err(e) = crate::commands::rag_commands::initialize_global_rag_service(db_service.clone()).await {
                    tracing::error!("Failed to initialize global RAG service: {}", e);
                } else {
                    tracing::debug!("Global RAG service initialized successfully");
                }

                // 初始化全局适配器管理器（为三个框架提供统一的工具调用接口）
                if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                    let tool_manager = tool_system.get_manager();
                    if let Err(e) = crate::tools::initialize_global_adapter_manager(tool_manager).await {
                        tracing::error!("Failed to initialize global adapter manager: {}", e);
                    } else {
                        tracing::debug!("Global adapter manager initialized successfully");
                    }
                } else {
                    tracing::error!("Cannot initialize adapter manager: global tool system not available");
                }

                // 创建MCP客户端管理器（保留用于MCP服务器连接）
                let client_manager = Arc::new(McpClientManager::with_database(db_service.clone()));
                
                // 创建MCP服务器管理器（保留用于向外提供MCP服务）
                let server_manager: Arc<McpServerManager> = Arc::new(McpServerManager::new());
                
                // 创建MCP服务（使用统一工具系统）
                let mcp_service = McpService::with_server_manager(client_manager.clone(), server_manager.clone(), db_service.clone());

                // 自动恢复MCP服务器状态
                if let Err(e) = mcp_service.auto_restore_server_state().await {
                    tracing::warn!("Failed to auto-restore MCP server state: {}", e);
                } else {
                    tracing::debug!("MCP server state auto-restored successfully");
                }

                // Initialize global proxy configuration from database
                if let Err(e) = initialize_global_proxy(&db_service).await {
                    tracing::warn!("Failed to initialize global proxy configuration: {}", e);
                } else {
                    tracing::debug!("Global proxy configuration initialized successfully");
                }


                let mut ai_manager = AiServiceManager::new(db_service.clone());
                ai_manager.set_mcp_service(Arc::new(mcp_service.clone()));
                ai_manager.set_app_handle(handle.clone());

                if let Err(e) = ai_manager.init_default_services().await {
                    tracing::error!("Failed to initialize AI services: {}", e);
                } else {
                    tracing::debug!("AI services initialized successfully");
                }

                let ai_manager = Arc::new(ai_manager);

                // AI adapter manager removed - using Rig directly

                // 初始化扫描会话服务
                let scan_session_service = Arc::new(ScanSessionService::new(db_service.clone()));

                // 初始化扫描服务
                let scan_service = Arc::new(ScanService::new(
                    db_service.clone(),
                    ai_manager.clone(),
                    Arc::new(RwLock::new(crate::tools::McpClientManager::new())),
                ));

                // 设置信号处理器
                setup_signal_handlers(client_manager.clone(), Arc::new(mcp_service.clone())).await;
        
                // 初始化资产服务
                let pool = db_service.get_pool().expect("Database pool not initialized").clone();
                let asset_service = crate::services::AssetService::new(pool);

                // 为异步任务创建 mcp_service 的克隆（在 manage 之前）
                let mcp_service_for_tools = Arc::new(mcp_service.clone());

                handle.manage(db_service);
                handle.manage(client_manager.clone());
                handle.manage(server_manager);
                handle.manage(Arc::new(mcp_service));
                // 将全局工具系统注入到 Tauri State，供扫描命令等读取
                if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                    handle.manage(tool_system);
                } else {
                    tracing::error!("Global tool system not available to manage in Tauri state");
                }
                handle.manage(ai_manager);
                // AI adapter manager removed - using Rig directly
                handle.manage(scan_session_service);
                handle.manage(scan_service);
                handle.manage(asset_service);
                



                // 初始化执行管理器
                let execution_manager = Arc::new(crate::managers::ExecutionManager::new());
                handle.manage(execution_manager);


                let client_manager_clone = client_manager.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = client_manager_clone.initialize().await {
                        tracing::error!("Failed to initialize MCP client: {}", e);
                    } else {
                        tracing::debug!("MCP client initialized successfully");
                        
                        // MCP 客户端初始化完成后，将 MCP 工具同步到全局工具系统
                        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                            // 通过 MCP 服务添加 MCP 工具提供者到全局工具系统
                            if let Err(e) = tool_system.add_mcp_provider_to_system(mcp_service_for_tools).await {
                                tracing::error!("Failed to register MCP provider to global tool system: {}", e);
                            } else {
                                tracing::info!("MCP provider registered to global tool system successfully");
                            }
                        }
                    }

                                    // 初始化Prompt服务状态
                let prompt_service_state: commands::prompt_commands::PromptServiceState = 
                Arc::new(tokio::sync::RwLock::new(None));
            handle.manage(prompt_service_state);

            // DISABLED: ReWOO测试状态 (引擎已禁用)
            let rewoo_test_state = Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::<String, String>::new()
            ));
            handle.manage(rewoo_test_state);

            // 初始化Agent管理器状态
            let agent_manager_state: commands::agent_commands::GlobalAgentManager = 
                Arc::new(tokio::sync::RwLock::new(None));
            handle.manage(agent_manager_state);

            // 初始化Plan-Execute引擎状态
            let plan_execute_engine_state: commands::plan_execute_commands::PlanExecuteEngineState = 
                Arc::new(tokio::sync::RwLock::new(None));
            handle.manage(plan_execute_engine_state);
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
            ai::save_ai_message,
            ai::send_ai_stream_message,

            ai::cancel_ai_stream,
            ai::get_ai_conversations,
            ai::get_ai_conversation_history,
            ai::delete_ai_conversation,
            ai::update_ai_conversation_title,
            ai::archive_ai_conversation,
            ai::test_ai_connection,
            ai::get_provider_models,
            ai::save_ai_config,
            ai::get_ai_config,
            ai::print_ai_conversations,
            ai::set_default_chat_model,
            ai::set_default_provider,
            // LM Studio相关命令
            ai::refresh_lm_studio_models,
            ai::get_lm_studio_status,
            ai::test_lm_studio_provider_connection,
            // 模型配置相关命令
            ai::get_scheduler_config,
            ai::save_scheduler_config,
            ai::get_service_for_stage,

            set_rag_collection_active,
            get_active_rag_collections,

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
            config::get_global_proxy_config,
            config::set_global_proxy_config,
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
            mcp_commands::start_mcp_server_with_state,
            mcp_commands::stop_mcp_server_with_state,
            mcp_commands::auto_restore_mcp_server_state,
            mcp_commands::get_mcp_server_saved_states,
            mcp_commands::save_mcp_server_state,
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
            commands::mcp::mcp_get_connection_status,
            commands::mcp::mcp_update_server_config,
            commands::mcp::retry_mcp_connection,
            commands::mcp::retry_mcp_connection_new,
            commands::mcp::toggle_builtin_tool,
            commands::mcp::get_builtin_tools_with_status,
            commands::mcp::get_mcp_external_tools,
            commands::mcp::diagnose_mcp_environment,
            commands::mcp::test_mcp_servers,
            commands::mcp::remove_local_mcp_servers,
            commands::mcp::get_running_mcp_processes,
            commands::mcp::shutdown_mcp_process,
            commands::mcp::shutdown_all_mcp_processes,
            commands::mcp::cleanup_duplicate_mcp_servers,
            commands::mcp::test_mcp_transport_types,
            commands::mcp::diagnose_mcp_connection,
            commands::mcp::connect_servers_concurrent,
            commands::mcp::get_connection_performance_stats,
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
            dictionary::get_dictionary_words_paged,
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
            // 默认字典设置（DB存储）
            dictionary::get_default_dictionary_id,
            dictionary::set_default_dictionary,
            dictionary::clear_default_dictionary,
            dictionary::get_default_dictionary_map,
            // 测试MCP相关命令
            commands::test_mcp::test_mcp_tools_registration,
            commands::test_mcp::test_ai_service_tools,
            commands::test_mcp::get_mcp_tools_status,
            // 窗口管理相关命令
            window::create_window,
            window::close_window,
            window::toggle_window,
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
            // Prompt DB-backed APIs
            commands::prompt_api::list_prompt_templates_api,
            commands::prompt_api::get_prompt_template_api,
            commands::prompt_api::create_prompt_template_api,
            commands::prompt_api::update_prompt_template_api,
            commands::prompt_api::delete_prompt_template_api,
            commands::prompt_api::get_user_prompt_configs_api,
            commands::prompt_api::update_user_prompt_config_api,
            commands::prompt_api::get_active_prompt_api,
            commands::prompt_api::list_prompt_groups_api,
            commands::prompt_api::create_prompt_group_api,
            commands::prompt_api::update_prompt_group_api,
            commands::prompt_api::delete_prompt_group_api,
            commands::prompt_api::set_arch_default_group_api,
            commands::prompt_api::upsert_prompt_group_item_api,
            commands::prompt_api::list_prompt_group_items_api,
            commands::prompt_api::remove_prompt_group_item_api,
            commands::prompt_api::preview_resolved_prompt_api,
            // Extended prompt APIs
            commands::prompt_api::list_prompt_templates_filtered_api,
            commands::prompt_api::duplicate_prompt_template_api,
            commands::prompt_api::evaluate_prompt_api,

            // ReWOO测试相关命令 - DISABLED
            rewoo_commands::test_rewoo_engine,
            rewoo_commands::get_rewoo_test_result,
            rewoo_commands::stop_rewoo_test,
            rewoo_commands::cleanup_rewoo_test_state,

            ai_commands::dispatch_scenario_task,
            ai_commands::stop_execution,
            ai_commands::get_ai_assistant_settings,
            ai_commands::save_ai_assistant_settings,
            ai_commands::get_agent_statistics,
            ai::get_ai_usage_stats,
            // Tools catalog for AgentManager (simple list)
            ai_commands::list_unified_tools,
            ai_commands::list_unified_tools_grouped,
            // 场景Agent配置
            ai_commands::list_scenario_agents,
            ai_commands::save_scenario_agent,
            ai_commands::delete_scenario_agent,
            
            // Agent系统相关命令
            agent_commands::initialize_agent_manager,
            agent_commands::list_agents,
            agent_commands::list_agent_architectures,
            agent_commands::dispatch_multi_agent_task,
            agent_commands::get_agent_task_status,
            agent_commands::cancel_agent_task,
            agent_commands::get_agent_system_stats,
            agent_commands::get_dispatch_statistics,
            
            // 工作流监控相关命令
            agent_commands::list_workflow_executions,
            agent_commands::get_workflow_statistics,
            agent_commands::get_workflow_execution,
            agent_commands::get_workflow_execution_details,
            agent_commands::cancel_workflow_execution,
            agent_commands::delete_workflow_execution,
            agent_commands::get_agent_task_logs,
            agent_commands::add_test_session_data,
            
            plan_execute_commands::execute_plan_and_execute_task,
            plan_execute_commands::get_plan_execute_statistics,
            plan_execute_commands::list_plan_execute_architectures,
            plan_execute_commands::get_plan_execute_sessions,
            plan_execute_commands::get_plan_execute_session_detail,
            plan_execute_commands::cancel_plan_execute_session,

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
            plan_execute_commands::execute_generic_prompt_task,
            // 代理测试命令 - DISABLED (ai_adapter removed)
            commands::test_proxy::test_proxy_dynamic_update,
            
            // Agent流程测试命令
            commands::test_agent_flow::test_complete_agent_flow,
            commands::test_agent_flow::test_tool_system_availability,
            commands::test_agent_flow::test_tool_execution,
            // Chat with automatic web search & summarization
            commands::send_ai_stream_with_search,
            
            // RAG相关命令
            rag_commands::rag_ingest_source,
            rag_commands::rag_query,
            rag_commands::rag_clear_collection,
            rag_commands::rag_get_status,
            rag_commands::rag_initialize_service,
            rag_commands::rag_shutdown_service,
            rag_commands::rag_get_supported_file_types,
            // 前端兼容的RAG命令
            rag_commands::get_rag_status,
            rag_commands::create_rag_collection,
            rag_commands::query_rag,
            rag_commands::delete_rag_collection,
            // RAG配置命令
            rag_commands::get_rag_config,
            rag_commands::save_rag_config,
            rag_commands::reset_rag_config,
            rag_commands::reload_rag_service,
            // 文件夹操作命令
            rag_commands::get_folder_files,
            // 文档级别操作命令
            rag_commands::list_rag_documents,
            rag_commands::get_rag_document_chunks,
            rag_commands::delete_rag_document,
            // AI助手RAG集成命令
            rag_commands::assistant_rag_answer,
            rag_commands::ensure_default_rag_collection,
            // 嵌入连接测试命令
            rag_commands::test_embedding_connection,
        ])
        .run(context)
        .expect("Failed to start Tauri application");
}

/// Initialize global proxy configuration from database
async fn initialize_global_proxy(db_service: &DatabaseService) -> anyhow::Result<()> {
    // Try to load proxy configuration from database
    match db_service.get_config("network", "global_proxy").await {
        Ok(Some(json_str)) => {
            // Parse the JSON configuration
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(config_json) => {
                    let proxy_config = ProxyConfig {
                        enabled: config_json["enabled"].as_bool().unwrap_or(false),
                        scheme: config_json["scheme"].as_str().map(|s| s.to_string()),
                        host: config_json["host"].as_str().map(|s| s.to_string()),
                        port: config_json["port"].as_u64().map(|p| p as u16),
                        username: config_json["username"].as_str().map(|s| s.to_string()),
                        password: config_json["password"].as_str().map(|s| s.to_string()),
                        no_proxy: config_json["no_proxy"].as_str().map(|s| s.to_string()),
                    };
                    
                    // Set the global proxy configuration only if enabled
                    if proxy_config.enabled {
                        set_global_proxy(Some(proxy_config));
                        tracing::debug!("Loaded and enabled proxy configuration from database");
                    } else {
                        set_global_proxy(None);
                        tracing::info!("Proxy configuration found but disabled, no proxy will be used");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse proxy configuration JSON: {}", e);
                }
            }
        }
        Ok(None) => {
            tracing::debug!("No proxy configuration found in database, using default (no proxy)");
            set_global_proxy(None);
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy configuration from database: {}", e);
            set_global_proxy(None);
        }
    }
    
    Ok(())
}
