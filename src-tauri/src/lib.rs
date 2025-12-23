//! Sentinel AI - Security Analysis Platform

pub mod agents;
pub mod analyzers;
pub mod commands;
pub mod engines;
pub mod events;
pub mod generators;
pub mod managers;
pub mod models;
pub mod services;
pub mod tools;
pub mod utils;

use std::fs;
use std::sync::Arc;
use tauri::{
    generate_handler,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use tracing_appender;
use tracing_subscriber;

use services::{ai::AiServiceManager, database::DatabaseService, scan_session::ScanSessionService};

use commands::{
    ai, asset, config, database as db_commands, dictionary,
    packet_capture_commands::{self, PacketCaptureState},
    passive_scan_commands::{self, PassiveScanState},
    performance, prompt_commands,
    proxifier_commands::{self, ProxifierState},
    rag_commands, scan_session_commands, scan_task_commands, tool_commands, window,
};

// Workflow engine and scheduler
use sentinel_workflow::{WorkflowEngine, WorkflowScheduler};

use utils::global_proxy::{set_global_proxy as set_proxy_async, GlobalProxyConfig};

struct TrayProxyMenuItem(MenuItem<tauri::Wry>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install rustls CryptoProvider (required for rustls 0.23+)
    // Must be called before any rustls usage
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let context = tauri::generate_context!();

    let logs_dir = "logs";
    let _ = fs::create_dir_all(logs_dir);

    let file_appender = tracing_appender::rolling::daily(logs_dir, "sentinel-ai.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sentinel_ai=info".parse().unwrap())
                .add_directive("sentinel_plugins=info".parse().unwrap())
                .add_directive("sentinel_workflow=info".parse().unwrap())
                .add_directive("sentinel_passive=info".parse().unwrap())
                .add_directive("hudsucker=off".parse().unwrap())
                .add_directive(
                    "rig::agent::prompt_request::streaming=warn"
                        .parse()
                        .unwrap(),
                ),
        )
        .with_writer(non_blocking)
        .without_time()
        .with_line_number(true)
        .with_ansi(false)
        .init();

    std::mem::forget(_guard);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let app_handle = window.app_handle();
                let _ = app_handle.save_window_state(StateFlags::all());
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(move |app| {
            let handle = app.handle().clone();

            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let proxy_item = MenuItem::with_id(app, "proxy", "开启代理", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            handle.manage(TrayProxyMenuItem(proxy_item.clone()));
            let tray_menu = Menu::with_items(app, &[&show_item, &proxy_item, &quit_item])?;

            let _tray_icon = TrayIconBuilder::with_id("main")
                .tooltip("Sentinel AI")
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "proxy" => {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            toggle_proxy(&app_clone).await;
                        });
                    }
                    "quit" => {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            cleanup_and_exit(&app_clone).await;
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } => {
                        // Left click: show main window
                        if button == tauri::tray::MouseButton::Left
                            && button_state == tauri::tray::MouseButtonState::Up
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        // Right click menu is handled automatically by .menu()
                    }
                    _ => {}
                })
                .icon(app.default_window_icon().unwrap().clone())
                .build(app)?;

            // Initialize license system (skip in debug mode)
            #[cfg(not(debug_assertions))]
            {
                use sentinel_license::ValidationResult;
                let license_result = sentinel_license::initialize();
                match license_result {
                    ValidationResult::Valid => {
                        tracing::info!("License validation successful");
                    }
                    ValidationResult::NotActivated => {
                        tracing::info!("License not activated, activation required");
                        // Will show activation dialog in frontend
                    }
                    ValidationResult::Invalid(reason) => {
                        tracing::warn!("License validation failed: {}", reason);
                    }
                }
            }
            #[cfg(debug_assertions)]
            {
                let _ = sentinel_license::initialize();
                tracing::info!("Debug mode: license check skipped");
            }

            tauri::async_runtime::block_on(async move {
                let mut db_service = DatabaseService::new();
                if let Err(e) = db_service.initialize().await {
                    tracing::error!("Database initialize failed: {:#}", e);
                    panic!("Failed to initialize database: {}", e);
                }
                let db_service = Arc::new(db_service);

                if let Err(e) =
                    crate::commands::rag_commands::initialize_global_rag_service(db_service.clone())
                        .await
                {
                    tracing::error!("Failed to initialize global RAG service: {}", e);
                }

                let passive_state = Arc::new(PassiveScanState::new());
                let passive_state_for_manage = (*passive_state).clone();

                // Extract PluginManager for workflow executor access
                let plugin_manager_for_workflow = passive_state.get_plugin_manager();

                if let Err(e) = initialize_global_proxy(&db_service).await {
                    tracing::warn!("Failed to initialize global proxy configuration: {}", e);
                }

                let mcp_service = Arc::new(crate::services::mcp::McpService::new());
                handle.manage(mcp_service.clone());

                let mut ai_manager = AiServiceManager::new(db_service.clone());
                ai_manager.set_app_handle(handle.clone());

                if let Err(e) = ai_manager.init_default_services().await {
                    tracing::error!("Failed to initialize AI services: {}", e);
                }

                let ai_manager = Arc::new(ai_manager);
                let scan_session_service = Arc::new(ScanSessionService::new(db_service.clone()));

                let pool = db_service
                    .get_pool()
                    .expect("Database pool not initialized")
                    .clone();
                let asset_service = crate::services::AssetService::new(pool.clone());
                let vulnerability_service = Arc::new(crate::services::VulnerabilityService::new(
                    db_service.clone(),
                    ai_manager.clone(),
                ));

                // Initialize workflow engine
                let workflow_engine = Arc::new(WorkflowEngine::new());

                // Initialize workflow scheduler (needs db_service before it's moved)
                let scheduler_executor: Arc<
                    dyn sentinel_workflow::scheduler::ScheduleExecutor + Send + Sync,
                > = Arc::new(
                    sentinel_workflow::commands::WorkflowScheduleExecutor::new(
                        workflow_engine.clone(),
                        db_service.clone(),
                        handle.clone(),
                    )
                    .with_plugin_manager(plugin_manager_for_workflow.clone()),
                );
                let workflow_scheduler = Arc::new(WorkflowScheduler::new(scheduler_executor));

                // Save a clone before manage() moves db_service
                let db_service_for_mcp = db_service.clone();

                handle.manage(db_service);
                handle.manage(ai_manager);
                handle.manage(scan_session_service);
                handle.manage(asset_service);
                handle.manage(vulnerability_service);
                handle.manage(passive_state_for_manage);
                handle.manage(plugin_manager_for_workflow);
                handle.manage(ProxifierState::new());
                handle.manage(PacketCaptureState::default());
                handle.manage(workflow_engine);
                handle.manage(workflow_scheduler);
                handle.manage(commands::vision_explorer_v2::VisionExplorerV2State::default());

                // Initialize shell permission handler
                if let Err(e) = tool_commands::init_shell_permission_handler(handle.clone()).await {
                    tracing::error!("Failed to init shell permission handler: {}", e);
                }

                match commands::prompt_api::initialize_default_prompts().await {
                    Ok(msg) => tracing::info!("Prompt initialization: {}", msg),
                    Err(e) => tracing::warn!("Failed to initialize default prompts: {}", e),
                }

                let prompt_service_state: commands::prompt_commands::PromptServiceState =
                    Arc::new(tokio::sync::RwLock::new(None));
                handle.manage(prompt_service_state);

                tracing::info!("Workflow engine and scheduler initialized");

                // Delay MCP server auto-connect to avoid blocking main process startup
                let handle_for_mcp = handle.clone();
                tokio::spawn(async move {
                    // Wait for main window to be ready
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    tracing::info!("Starting delayed MCP server auto-connect...");
                    // Register workflow tools
                    {
                        let tool_server = sentinel_tools::get_tool_server();
                        let db_workflow = db_service_for_mcp.clone();
                        sentinel_tools::workflow_adapter::load_workflows_from_db(
                            &tool_server,
                            || async move {
                                db_workflow
                                    .list_workflow_definitions(Some(false))
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                        )
                        .await;
                    }

                    // Register plugin tools
                    {
                        let tool_server = sentinel_tools::get_tool_server();
                        let db_plugin = db_service_for_mcp.clone();
                        
                        if let Ok(plugins) = db_plugin.get_plugins_from_registry().await {
                            let mut plugin_metas = Vec::new();
                            for plugin in plugins {
                                // Filter enabled and agent category
                                let enabled = plugin
                                    .get("enabled")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let main_category = plugin
                                    .get("main_category")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                
                                if enabled && main_category == "agent" {
                                    let id = plugin
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    if id.is_empty() {
                                        continue;
                                    }
                                    
                                    let name = plugin
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Unknown")
                                        .to_string();
                                        
                                    let description = plugin
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Agent plugin tool")
                                        .to_string();
                                        
                                    // Note: database field is 'plugin_code' not 'code'
                                    let code = plugin
                                        .get("plugin_code")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    
                                    // 使用运行时调用获取 input_schema
                                    let input_schema = if let Some(code_str) = &code {
                                        let metadata = sentinel_plugins::PluginMetadata {
                                            id: id.clone(),
                                            name: name.clone(),
                                            version: "1.0.0".to_string(),
                                            author: None,
                                            main_category: "agent".to_string(),
                                            category: "tool".to_string(),
                                            default_severity: sentinel_plugins::Severity::Medium,
                                            tags: vec![],
                                            description: Some(description.clone()),
                                        };
                                        sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
                                            code_str,
                                            metadata,
                                        ).await
                                    } else {
                                        serde_json::json!({
                                            "type": "object",
                                            "properties": {}
                                        })
                                    };
                                    
                                    plugin_metas.push(sentinel_tools::plugin_adapter::PluginToolMeta {
                                        plugin_id: id,
                                        name,
                                        description,
                                        input_schema,
                                        code,
                                    });
                                }
                            }
                            
                            if !plugin_metas.is_empty() {
                                tracing::info!("Loading {} plugin tools...", plugin_metas.len());
                                sentinel_tools::plugin_adapter::load_plugin_tools_to_server(
                                    &tool_server, 
                                    plugin_metas
                                ).await;
                            }
                        }
                    }

                    commands::mcp_commands::mcp_auto_connect_servers(
                        db_service_for_mcp,
                        handle_for_mcp,
                    )
                    .await;
                });
            });

            Ok(())
        })
        .invoke_handler(generate_handler![
            // AI commands
            ai::list_ai_services,
            ai::add_ai_service,
            ai::remove_ai_service,
            ai::create_ai_conversation,
            ai::save_ai_message,
            ai::cancel_ai_stream,
            ai::get_ai_conversations,
            ai::get_ai_messages_by_conversation,
            ai::clear_conversation_messages,
            ai::save_tool_config,
            ai::get_tool_config,
            ai::get_ai_conversation_history,
            ai::delete_ai_conversation,
            ai::update_ai_conversation_title,
            ai::archive_ai_conversation,
            ai::delete_ai_message,
            ai::test_ai_connection,
            ai::get_provider_models,
            ai::save_ai_config,
            ai::get_ai_config,
            ai::print_ai_conversations,
            ai::set_default_llm_model,
            ai::set_default_vlm_model,
            ai::set_default_llm_provider,
            ai::upload_image_attachment,
            ai::upload_multiple_images,
            ai::agent_execute,
            ai::refresh_lm_studio_models,
            ai::get_lm_studio_status,
            ai::test_lm_studio_provider_connection,
            ai::get_scheduler_config,
            ai::save_scheduler_config,
            ai::get_service_for_stage,
            ai::get_ai_usage_stats,
            ai::generate_workflow_from_nl,
            ai::generate_plugin_stream,
            ai::cancel_plugin_generation,
            commands::get_active_rag_collections,
            commands::set_rag_collection_active,
            // Database commands
            db_commands::execute_query,
            db_commands::get_query_history,
            db_commands::clear_query_history,
            db_commands::get_database_status,
            db_commands::get_database_path,
            db_commands::test_database_connection,
            db_commands::create_database_backup,
            db_commands::restore_database_backup,
            db_commands::optimize_database,
            db_commands::rebuild_database_indexes,
            db_commands::cleanup_database,
            db_commands::list_database_backups,
            db_commands::delete_database_backup,
            db_commands::export_database_json,
            db_commands::import_database_json,
            db_commands::get_database_statistics,
            db_commands::reset_database,
            // Asset commands

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
            // Config commands
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
            commands::check_command_exists,
            commands::role::get_ai_roles,
            commands::role::create_ai_role,
            commands::role::update_ai_role,
            commands::role::delete_ai_role,
            commands::role::set_current_ai_role,
            commands::role::get_current_ai_role,
            // Scan session commands
            scan_session_commands::create_scan_session,
            scan_session_commands::get_scan_session,
            scan_session_commands::update_scan_session,
            scan_session_commands::list_scan_sessions,
            scan_session_commands::delete_scan_session,
            scan_session_commands::get_scan_progress,
            scan_session_commands::get_session_stages,
            // Scan task commands
            scan_task_commands::get_scan_tasks,
            scan_task_commands::create_scan_task,
            scan_task_commands::update_scan_task_status,
            scan_task_commands::delete_scan_task,
            scan_task_commands::stop_scan_task,
            // Performance commands
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
            // Dictionary commands
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
            dictionary::get_subdomain_dictionary,
            dictionary::set_subdomain_dictionary,
            dictionary::add_subdomain_words,
            dictionary::remove_subdomain_words,
            dictionary::reset_subdomain_dictionary,
            dictionary::import_subdomain_dictionary,
            dictionary::export_subdomain_dictionary,
            dictionary::get_default_dictionary_id,
            dictionary::set_default_dictionary,
            dictionary::clear_default_dictionary,
            dictionary::get_default_dictionary_map,
            // Window commands
            window::create_window,
            window::close_window,
            window::toggle_window,
            window::get_window_info,
            window::set_window_position,
            window::set_window_size,
            // Prompt commands
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
            commands::prompt_api::list_prompt_templates_api,
            commands::prompt_api::get_prompt_template_api,
            commands::prompt_api::create_prompt_template_api,
            commands::prompt_api::update_prompt_template_api,
            commands::prompt_api::delete_prompt_template_api,
            commands::prompt_api::preview_resolved_prompt_api,
            commands::prompt_api::list_prompt_templates_filtered_api,
            commands::prompt_api::duplicate_prompt_template_api,
            commands::prompt_api::evaluate_prompt_api,
            commands::prompt_api::get_plugin_generation_prompt_api,
            commands::prompt_api::get_combined_plugin_prompt_api,
            commands::prompt_api::get_default_prompt_content,
            commands::prompt_api::initialize_default_prompts,
            // RAG commands
            rag_commands::rag_ingest_source,
            rag_commands::rag_ingest_text,
            rag_commands::rag_query,
            rag_commands::rag_clear_collection,
            rag_commands::rag_initialize_service,
            rag_commands::rag_shutdown_service,
            rag_commands::rag_get_supported_file_types,
            rag_commands::get_rag_status,
            rag_commands::create_rag_collection,
            rag_commands::update_rag_collection,
            rag_commands::query_rag,
            rag_commands::delete_rag_collection,
            rag_commands::get_rag_config,
            rag_commands::save_rag_config,
            rag_commands::reset_rag_config,
            rag_commands::reload_rag_service,
            rag_commands::get_folder_files,
            rag_commands::list_rag_documents,
            rag_commands::get_rag_document_chunks,
            rag_commands::delete_rag_document,
            rag_commands::assistant_rag_answer,
            rag_commands::ensure_default_rag_collection,
            rag_commands::test_embedding_connection,
            // Passive scan commands
            passive_scan_commands::start_passive_scan,
            passive_scan_commands::stop_passive_scan,
            passive_scan_commands::get_proxy_status,
            passive_scan_commands::reload_plugin_in_pipeline,
            passive_scan_commands::list_findings,
            passive_scan_commands::count_findings,
            passive_scan_commands::enable_plugin,
            passive_scan_commands::disable_plugin,
            passive_scan_commands::batch_enable_plugins,
            passive_scan_commands::batch_disable_plugins,
            passive_scan_commands::list_plugins,
            passive_scan_commands::download_ca_cert,
            passive_scan_commands::get_ca_cert_path,
            passive_scan_commands::trust_ca_cert,
            passive_scan_commands::regenerate_ca_cert,
            passive_scan_commands::get_ca_fingerprint,
            passive_scan_commands::open_ca_cert_dir,
            passive_scan_commands::export_ca_cert,
            passive_scan_commands::export_ca_key,
            passive_scan_commands::export_ca_pkcs12,
            passive_scan_commands::import_ca_pkcs12,
            passive_scan_commands::import_ca_der,
            passive_scan_commands::get_finding,
            passive_scan_commands::update_finding_status,
            passive_scan_commands::export_findings_html,
            passive_scan_commands::list_proxy_requests,
            passive_scan_commands::get_proxy_request,
            passive_scan_commands::clear_proxy_requests,
            passive_scan_commands::count_proxy_requests,
            passive_scan_commands::create_plugin_in_db,
            passive_scan_commands::update_plugin,
            passive_scan_commands::get_plugin_code,
            passive_scan_commands::get_plugin_by_id,
            passive_scan_commands::test_plugin,
            passive_scan_commands::delete_plugin,
            passive_scan_commands::delete_passive_vulnerability,
            passive_scan_commands::delete_passive_vulnerabilities_batch,
            passive_scan_commands::delete_all_passive_vulnerabilities,
            passive_scan_commands::test_plugin_advanced,
            passive_scan_commands::test_agent_plugin,
            passive_scan_commands::get_plugin_input_schema,
            passive_scan_commands::start_proxy_listener,
            passive_scan_commands::stop_proxy_listener,
            passive_scan_commands::save_proxy_config,
            passive_scan_commands::get_proxy_config,
            passive_scan_commands::set_intercept_enabled,
            passive_scan_commands::get_intercept_enabled,
            passive_scan_commands::get_intercepted_requests,
            passive_scan_commands::forward_intercepted_request,
            passive_scan_commands::drop_intercepted_request,
            passive_scan_commands::set_response_intercept_enabled,
            passive_scan_commands::get_response_intercept_enabled,
            passive_scan_commands::get_intercepted_responses,
            passive_scan_commands::forward_intercepted_response,
            passive_scan_commands::drop_intercepted_response,
            passive_scan_commands::replay_request,
            passive_scan_commands::replay_raw_request,
            passive_scan_commands::list_websocket_connections,
            passive_scan_commands::list_websocket_messages,
            passive_scan_commands::clear_websocket_history,
            passive_scan_commands::get_history_stats,
            passive_scan_commands::clear_all_history,
            passive_scan_commands::set_websocket_intercept_enabled,
            passive_scan_commands::get_websocket_intercept_enabled,
            passive_scan_commands::forward_intercepted_websocket,
            passive_scan_commands::drop_intercepted_websocket,
            passive_scan_commands::add_intercept_filter_rule,
            passive_scan_commands::get_intercept_filter_rules,
            passive_scan_commands::remove_intercept_filter_rule,
            passive_scan_commands::update_intercept_filter_rule,
            passive_scan_commands::update_runtime_filter_rules,
            // Plugin store commands
            passive_scan_commands::fetch_store_plugins,
            passive_scan_commands::fetch_plugin_code,
            passive_scan_commands::install_store_plugin,
            // Proxifier commands
            proxifier_commands::get_proxifier_config,
            proxifier_commands::start_proxifier,
            proxifier_commands::stop_proxifier,
            proxifier_commands::save_proxifier_proxies,
            proxifier_commands::save_proxifier_rules,
            proxifier_commands::get_proxifier_connections,
            proxifier_commands::clear_proxifier_connections,
            proxifier_commands::get_transparent_proxy_status,
            proxifier_commands::start_transparent_proxy,
            proxifier_commands::stop_transparent_proxy,
            proxifier_commands::add_transparent_redirect_port,
            proxifier_commands::remove_transparent_redirect_port,
            proxifier_commands::load_proxifier_proxies_from_db,
            proxifier_commands::save_proxifier_proxies_to_db,
            proxifier_commands::load_proxifier_rules_from_db,
            proxifier_commands::save_proxifier_rules_to_db,
            // Plugin review commands
            commands::plugin_review_commands::get_plugins_for_review,
            commands::plugin_review_commands::list_generated_plugins,
            commands::plugin_review_commands::get_plugin_detail,
            commands::plugin_review_commands::approve_plugin,
            commands::plugin_review_commands::reject_plugin,
            commands::plugin_review_commands::review_update_plugin_code,
            commands::config_commands::get_auto_approval_config,
            commands::config_commands::update_auto_approval_config,
            commands::config_commands::get_config_presets,
            commands::config_commands::test_config_impact,
            commands::plugin_review_commands::batch_approve_plugins,
            commands::plugin_review_commands::batch_reject_plugins,
            commands::plugin_review_commands::get_plugin_statistics,
            commands::plugin_review_commands::search_plugins,
            commands::plugin_review_commands::export_plugin,
            commands::plugin_review_commands::review_delete_plugin,
            commands::plugin_review_commands::get_plugins_paginated,
            commands::plugin_review_commands::toggle_plugin_favorite,
            commands::plugin_review_commands::get_favorited_plugins,
            commands::plugin_review_commands::get_plugin_review_statistics,
            // Notifications
            commands::notifications::send_notification,
            commands::notifications::create_notification_rule,
            commands::notifications::update_notification_rule,
            commands::notifications::delete_notification_rule,
            commands::notifications::list_notification_rules,
            commands::notifications::get_notification_rule,
            commands::notifications::test_notification_rule_connection,
            // Packet capture
            packet_capture_commands::get_network_interfaces,
            packet_capture_commands::start_packet_capture,
            packet_capture_commands::stop_packet_capture,
            packet_capture_commands::is_capture_running,
            packet_capture_commands::open_pcap_file,
            packet_capture_commands::save_pcap_file,
            packet_capture_commands::extract_files_preview,
            packet_capture_commands::extract_files_to_dir,
            packet_capture_commands::save_extracted_file,
            packet_capture_commands::get_file_related_packets,
            packet_capture_commands::get_file_stream_packets,
            packet_capture_commands::save_selected_files,
            // Test commands
            commands::test_proxy::test_proxy_dynamic_update,
            commands::test_proxy::test_proxy_persistence,
            commands::test_proxy::test_http_client_proxy_update,
            commands::test_proxy::test_proxy_connection,
            commands::test_proxy::get_current_proxy_config,
            // Tool commands
            tool_commands::get_builtin_tools_with_status,
            tool_commands::toggle_builtin_tool,
            tool_commands::unified_execute_tool,
            tool_commands::list_unified_tools,
            tool_commands::list_node_catalog,
            tool_commands::get_all_tool_metadata,
            tool_commands::get_tools_by_category,
            tool_commands::search_tools,
            tool_commands::get_tool_statistics,
            tool_commands::get_tool_metadata,
            tool_commands::get_tool_usage_stats,
            tool_commands::clear_tool_usage_stats,
            tool_commands::vision_explorer_receive_credentials,
            tool_commands::vision_explorer_send_user_message,
            tool_commands::vision_explorer_skip_login,
            tool_commands::vision_explorer_manual_login_complete,
            // Vision Explorer V2 commands
            commands::vision_explorer_v2::start_vision_explorer_v2,
            commands::vision_explorer_v2::stop_vision_explorer_v2,
            commands::vision_explorer_v2::vision_explorer_v2_receive_credentials,
            commands::vision_explorer_v2::vision_explorer_v2_skip_login,
            commands::vision_explorer_v2::get_vision_explorer_v2_status,
            commands::vision_explorer_v2::list_vision_explorer_v2_sessions,

            // Shell Tool commands
            tool_commands::init_shell_permission_handler,
            tool_commands::get_shell_tool_config,
            tool_commands::set_shell_tool_config,
            tool_commands::respond_shell_permission,
            tool_commands::get_pending_shell_permissions,
            // Agent config commands
            tool_commands::get_agent_config,
            tool_commands::save_agent_config,
            // MCP commands
            commands::mcp_commands::mcp_get_connections,
            commands::mcp_commands::mcp_get_connection_status,
            commands::mcp_commands::add_child_process_mcp_server,
            commands::mcp_commands::mcp_disconnect_server,
            commands::mcp_commands::mcp_delete_server_config,
            commands::mcp_commands::mcp_update_server_config,
            commands::mcp_commands::mcp_get_connection_tools,
            commands::mcp_commands::mcp_call_tool,
            commands::mcp_commands::mcp_test_server_tool,
            commands::mcp_commands::mcp_get_all_tools,
            commands::mcp_commands::quick_create_mcp_server,
            commands::mcp_commands::import_mcp_servers_from_json,
            commands::mcp_commands::cleanup_duplicate_mcp_servers,
            commands::mcp_commands::mcp_set_auto_connect,
            // License commands
            commands::license_commands::get_license_info,
            commands::license_commands::activate_license,
            commands::license_commands::check_license,
            commands::license_commands::get_machine_id,
            commands::license_commands::get_machine_id_full,
            commands::license_commands::deactivate_license,
            // Workflow commands
            sentinel_workflow::commands::start_workflow_run,
            sentinel_workflow::commands::stop_workflow_run,
            sentinel_workflow::commands::get_workflow_run_status,
            sentinel_workflow::commands::list_workflow_runs,
            sentinel_workflow::commands::list_workflow_runs_paginated,
            sentinel_workflow::commands::get_workflow_run_detail,
            sentinel_workflow::commands::delete_workflow_run,
            sentinel_workflow::commands::save_workflow_definition,
            sentinel_workflow::commands::get_workflow_definition,
            sentinel_workflow::commands::list_workflow_definitions,
            sentinel_workflow::commands::list_workflow_tools,
            sentinel_workflow::commands::delete_workflow_definition,
            sentinel_workflow::commands::validate_workflow_graph,
            sentinel_workflow::commands::start_workflow_schedule,
            sentinel_workflow::commands::stop_workflow_schedule,
            sentinel_workflow::commands::list_workflow_schedules,
            sentinel_workflow::commands::get_workflow_schedule,
        ])
        .run(context)
        .expect("Failed to start Tauri application");
}

async fn toggle_proxy(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<PassiveScanState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;

        if is_running {
            match passive_scan_commands::stop_passive_scan_internal(app, &state).await {
                Ok(_) => {
                    tracing::info!("Proxy stopped from tray menu");
                    update_proxy_menu_text(app, false);
                }
                Err(e) => tracing::error!("Failed to stop proxy: {}", e),
            }
        } else {
            match passive_scan_commands::start_passive_scan_internal(app, &state, None).await {
                Ok(port) => {
                    tracing::info!("Proxy started from tray menu on port {}", port);
                    update_proxy_menu_text(app, true);
                }
                Err(e) => tracing::error!("Failed to start proxy: {}", e),
            }
        }
    }
}

fn update_proxy_menu_text(app: &tauri::AppHandle, is_running: bool) {
    if let Some(proxy_item) = app.try_state::<TrayProxyMenuItem>() {
        let text = if is_running {
            "关闭代理"
        } else {
            "开启代理"
        };
        let _ = proxy_item.0.set_text(text);
    }
}

async fn cleanup_and_exit(app: &tauri::AppHandle) {
    let _ = app.save_window_state(StateFlags::all());

    if let Some(state) = app.try_state::<PassiveScanState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;
        if is_running {
            let _ = passive_scan_commands::stop_passive_scan_internal(app, &state).await;
        }
    }

    tracing::info!("Application cleanup completed, exiting");
    std::process::exit(0);
}

async fn initialize_global_proxy(db_service: &DatabaseService) -> anyhow::Result<()> {
    match db_service.get_config("network", "global_proxy").await {
        Ok(Some(json_str)) => match serde_json::from_str::<GlobalProxyConfig>(&json_str) {
            Ok(proxy_config) => {
                if proxy_config.enabled {
                    set_proxy_async(proxy_config.clone()).await;

                    let tools_proxy_config = sentinel_tools::GlobalProxyConfig {
                        enabled: proxy_config.enabled,
                        scheme: proxy_config.scheme.clone(),
                        host: proxy_config.host.clone(),
                        port: proxy_config.port,
                        username: proxy_config.username.clone(),
                        password: proxy_config.password.clone(),
                        no_proxy: proxy_config.no_proxy.clone(),
                    };
                    sentinel_tools::set_global_proxy(tools_proxy_config).await;
                } else {
                    utils::global_proxy::clear_global_proxy().await;
                    sentinel_tools::set_global_proxy(sentinel_tools::GlobalProxyConfig::default())
                        .await;
                }
            }
            Err(e) => tracing::warn!("Failed to parse proxy configuration JSON: {}", e),
        },
        Ok(None) => {
            utils::global_proxy::clear_global_proxy().await;
            sentinel_tools::set_global_proxy(sentinel_tools::GlobalProxyConfig::default()).await;
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy configuration from database: {}", e);
            utils::global_proxy::clear_global_proxy().await;
            sentinel_tools::set_global_proxy(sentinel_tools::GlobalProxyConfig::default()).await;
        }
    }
    Ok(())
}
