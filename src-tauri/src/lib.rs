//! Sentinel AI - Security Analysis Platform

pub mod agents;
pub mod analyzers;
pub mod commands;
pub mod engines;
pub mod events;
pub mod generators;
pub mod managers;
pub mod memory;
pub mod models;
pub mod services;
pub mod skills;
pub mod tools;
pub mod trackers;
pub mod utils;

use sentinel_db::Database;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    generate_handler,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use services::{
    ai::AiServiceManager,
    database::DatabaseService,
    system_agents::{ensure_default_system_agent_profiles, start_behavior_extension_bridge},
    SystemAgentRuntime,
};

use crate::skills::scan_and_upsert_skills;
use crate::utils::plugin_registry_cleanup::cleanup_removed_agent_plugins;
use commands::{
    ai, ai_conversation_binding_support, ai_execution_state_support, ai_turn_logs, aisettings,
    asset, cleanup_expired_cache, config, database as db_commands, delete_cache, dictionary,
    get_all_cache_keys, get_cache, llm_test_commands, memory_commands,
    monitor_commands::MonitorSchedulerState,
    packet_capture_commands::{self, PacketCaptureState},
    performance,
    proxifier_commands::{self, ProxifierState},
    rag_commands, scan_session_commands, scan_task_commands, set_cache, tool_commands,
    traffic::{self, TrafficAnalysisState},
    window,
};

// Workflow engine and scheduler
use sentinel_workflow::{WorkflowEngine, WorkflowScheduler};

use sentinel_core::global_proxy::{set_global_proxy as set_proxy_async, GlobalProxyConfig};

struct TrayProxyMenuItem(MenuItem<tauri::Wry>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseButtonAction {
    Hide,
    Minimize,
    Exit,
}

impl CloseButtonAction {
    fn from_config(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "hide" | "tray" | "close_to_tray" => Some(Self::Hide),
            "minimize" | "minimise" => Some(Self::Minimize),
            "exit" | "quit" | "close" => Some(Self::Exit),
            _ => None,
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::Hide => "hide",
            Self::Minimize => "minimize",
            Self::Exit => "exit",
        }
    }
}

fn apply_hide_close_behavior(window: &tauri::Window) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_skip_taskbar(true);
    }
    let _ = window.hide();
}

fn apply_minimize_close_behavior(window: &tauri::Window) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_skip_taskbar(false);
    }
    let _ = window.minimize();
}

async fn execute_close_button_action(
    app: &tauri::AppHandle,
    window: &tauri::Window,
    action: CloseButtonAction,
) {
    match action {
        CloseButtonAction::Hide => apply_hide_close_behavior(window),
        CloseButtonAction::Minimize => apply_minimize_close_behavior(window),
        CloseButtonAction::Exit => cleanup_and_exit(app).await,
    }
}

fn prompt_and_apply_close_button_action(
    app_handle: tauri::AppHandle,
    db_service: Arc<DatabaseService>,
    window: tauri::Window,
) {
    const HIDE_LABEL: &str = "隐藏窗口";
    const EXIT_LABEL: &str = "退出程序";

    app_handle
        .dialog()
        .message("点击关闭按钮时请选择行为：隐藏窗口（后台运行）或退出程序。")
        .title("关闭行为设置")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            HIDE_LABEL.to_string(),
            EXIT_LABEL.to_string(),
        ))
        .show_with_result(move |result| {
            let selected = match result {
                MessageDialogResult::Custom(label) if label == HIDE_LABEL => {
                    Some(CloseButtonAction::Hide)
                }
                MessageDialogResult::Custom(label) if label == EXIT_LABEL => {
                    Some(CloseButtonAction::Exit)
                }
                MessageDialogResult::Ok | MessageDialogResult::Yes => Some(CloseButtonAction::Hide),
                MessageDialogResult::No => Some(CloseButtonAction::Exit),
                MessageDialogResult::Cancel => None,
                _ => None,
            };

            let app_for_action = app_handle.clone();
            let db_for_action = db_service.clone();
            let window_for_action = window.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(action) = selected {
                    if let Err(e) = db_for_action
                        .set_config(
                            "ui",
                            "close_button_action",
                            action.as_config_value(),
                            Some("Window close button action: hide, minimize or exit"),
                        )
                        .await
                    {
                        tracing::warn!("Failed to save close button action: {}", e);
                    }
                    execute_close_button_action(&app_for_action, &window_for_action, action).await;
                } else {
                    // Dialog dismissed: keep app running and avoid accidental exit.
                    apply_hide_close_behavior(&window_for_action);
                }
            });
        });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install rustls CryptoProvider (required for rustls 0.23+)
    // Must be called before any rustls usage
    let _ = rustls::crypto::ring::default_provider().install_default();

    let context = tauri::generate_context!();

    // Configure log directory:
    // - Debug: project working directory `logs/`
    // - Release: system data directory `<data_dir>/sentinel-ai/logs`
    #[cfg(debug_assertions)]
    let logs_dir = {
        let path = PathBuf::from("logs");
        let _ = fs::create_dir_all(&path);
        path
    };

    #[cfg(not(debug_assertions))]
    let logs_dir = {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("logs");
        let _ = fs::create_dir_all(&path);
        path
    };
    let logs_dir = logs_dir.to_string_lossy().to_string();

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "sentinel-ai.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("sentinel_ai=info".parse().unwrap())
        .add_directive("sentinel_plugins=info".parse().unwrap())
        .add_directive("sentinel_workflow=info".parse().unwrap())
        .add_directive("sentinel_traffic=info".parse().unwrap())
        .add_directive("sentinel_rag=info".parse().unwrap())
        .add_directive("sentinel_llm=info".parse().unwrap())
        .add_directive("hudsucker=off".parse().unwrap())
        .add_directive(
            "rig::agent::prompt_request::streaming=warn"
                .parse()
                .unwrap(),
        );

    // let rig_debug = std::env::var("SENTINEL_RIG_DEBUG")
    //     .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
    //     .unwrap_or(false);
    // if rig_debug {
    //     env_filter = env_filter
    //         .add_directive("sentinel_llm=debug".parse().unwrap())
    //         .add_directive("rig=debug".parse().unwrap())
    //         .add_directive(
    //             "rig::agent::prompt_request::streaming=debug"
    //                 .parse()
    //                 .unwrap(),
    //         );
    // }

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
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
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                let app_handle = window.app_handle();
                let _ = app_handle.save_window_state(StateFlags::all());
                let window_for_action = window.clone();

                let db_service = app_handle
                    .try_state::<Arc<DatabaseService>>()
                    .map(|state| state.inner().clone());

                if let Some(db_service) = db_service {
                    let app_for_action = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match db_service.get_config("ui", "close_button_action").await {
                            Ok(Some(value)) => {
                                if let Some(action) = CloseButtonAction::from_config(&value) {
                                    execute_close_button_action(
                                        &app_for_action,
                                        &window_for_action,
                                        action,
                                    )
                                    .await;
                                } else {
                                    prompt_and_apply_close_button_action(
                                        app_for_action,
                                        db_service.clone(),
                                        window_for_action,
                                    );
                                }
                            }
                            Ok(None) => {
                                prompt_and_apply_close_button_action(
                                    app_for_action,
                                    db_service.clone(),
                                    window_for_action,
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to load close button action, fallback to minimize: {}",
                                    e
                                );
                                apply_minimize_close_behavior(&window_for_action);
                            }
                        }
                    });
                } else {
                    // Database state unavailable, fallback to minimize to keep app running.
                    apply_minimize_close_behavior(&window_for_action);
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();

            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let proxy_item = MenuItem::with_id(app, "proxy", "开启代理", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            handle.manage(TrayProxyMenuItem(proxy_item.clone()));
            let tray_menu = Menu::with_items(app, &[&show_item, &proxy_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::with_id("main")
                .tooltip("Sentinel AI")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            #[cfg(not(target_os = "macos"))]
                            {
                                let _ = window.set_skip_taskbar(false);
                            }
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
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
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                            button,
                            button_state,
                            ..
                        } = event {
                        // Left click: show main window
                        if button == tauri::tray::MouseButton::Left
                            && button_state == tauri::tray::MouseButtonState::Up
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                #[cfg(not(target_os = "macos"))]
                                {
                                    let _ = window.set_skip_taskbar(false);
                                }
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.unminimize();
                            }
                        }
                        // Right click menu is handled automatically by .menu()
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _tray_icon = tray_builder.build(app)?;

            // Initialize license system (skip in debug mode)
            #[cfg(not(debug_assertions))]
            {
                use sentinel_license::ValidationResult;
                let license_result = sentinel_license::initialize();
                if !sentinel_license::is_enforcement_enabled() {
                    tracing::info!("License enforcement is disabled by hardcoded config");
                }
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
                // Ensure database config directory exists
                use sentinel_db::database_service::db_config_toml_path;
                use std::fs;

                let config_path = db_config_toml_path();

                let data_dir = config_path.parent().unwrap();
                let _ = fs::create_dir_all(data_dir);

                let mut db_service = DatabaseService::new();
                if let Err(e) = db_service.initialize().await {
                    tracing::error!("Database initialize failed: {:#}", e);
                    eprintln!("Database initialization failed: {:#}", e);
                    eprintln!(
                        "Database config is loaded from:\n  {}\n\
                         The current default backend is SQLite unless db_config.toml selects another database.",
                        config_path.display()
                    );
                    std::process::exit(1);
                }
                let db_service = Arc::new(db_service);

                match crate::commands::aisettings::cleanup_legacy_ai_config_keys(
                    db_service.as_ref(),
                )
                .await
                {
                    Ok(removed) if removed > 0 => {
                        tracing::info!(
                            "Removed {} legacy AI config key(s) during startup",
                            removed
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("Failed to clean legacy default VLM config keys: {}", e);
                    }
                }

                // Initialize dictionary pool for plugins using the active runtime backend.
                tracing::info!("Preparing dictionary pool for plugins from runtime database");
                match db_service.get_runtime_pool() {
                    #[cfg(all(not(feature = "db-postgres"), not(feature = "db-mysql")))]
                    Ok(sentinel_db::database_service::connection_manager::DatabasePool::SQLite(
                        pool,
                    )) => {
                        sentinel_plugins::init_dictionary_pool(pool);
                        tracing::info!("Dictionary pool initialized for plugins using SQLite");
                    }
                    #[cfg(feature = "db-postgres")]
                    Ok(
                        sentinel_db::database_service::connection_manager::DatabasePool::PostgreSQL(
                            pool,
                        ),
                    ) => {
                        sentinel_plugins::init_dictionary_pool(pool);
                        tracing::info!(
                            "Dictionary pool initialized for plugins using PostgreSQL"
                        );
                    }
                    #[cfg(all(not(feature = "db-postgres"), feature = "db-mysql"))]
                    Ok(sentinel_db::database_service::connection_manager::DatabasePool::MySQL(
                        pool,
                    )) => {
                        sentinel_plugins::init_dictionary_pool(pool);
                        tracing::info!("Dictionary pool initialized for plugins using MySQL");
                    }
                    Ok(other) => {
                        tracing::warn!(
                            "Skipping dictionary pool initialization for unsupported database type: {:?}",
                            other.db_type()
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Failed to initialize dictionary pool for plugins: {error:#}"
                        );
                    }
                }

                let plugin_runtime_settings = match db_service
                    .load_proxy_config(
                        crate::commands::traffic::runtime_settings_commands::TRAFFIC_PLUGIN_RUNTIME_SETTINGS_KEY,
                    )
                    .await
                {
                    Ok(Some(raw)) => serde_json::from_str::<sentinel_plugins::PluginRuntimeSettings>(
                        &raw,
                    )
                    .map(|value| value.sanitized())
                    .unwrap_or_default(),
                    Ok(None) => sentinel_plugins::PluginRuntimeSettings::default(),
                    Err(error) => {
                        tracing::warn!(
                            "Failed to load traffic plugin runtime settings during startup: {}",
                            error
                        );
                        sentinel_plugins::PluginRuntimeSettings::default()
                    }
                };
                sentinel_plugins::set_plugin_runtime_settings(plugin_runtime_settings);

                if let Err(e) =
                    crate::services::builtin_bounty_plugins::initialize_builtin_bounty_resources(
                        &db_service,
                    )
                    .await
                {
                    tracing::warn!("Failed to initialize builtin bounty resources: {}", e);
                }

                // Initialize agent configuration from database
                if let Err(e) = tool_commands::init_agent_config(&db_service).await {
                    tracing::error!("Failed to initialize agent config: {}", e);
                }

                if let Err(e) = scan_and_upsert_skills(&db_service).await {
                    tracing::warn!("Skills scan warning: {}", e);
                }

                if let Err(e) =
                    crate::commands::rag_commands::initialize_global_rag_service(db_service.clone())
                        .await
                {
                    tracing::error!("Failed to initialize global RAG service: {}", e);
                } else {
                    // Ensure memory collection exists
                    match crate::commands::rag_commands::ensure_memory_collection_exists(db_service.clone()).await {
                        Ok(collection_id) => {
                            tracing::info!("Memory collection ready: {}", collection_id);
                            match crate::commands::rag_commands::get_or_init_rag_service(db_service.clone()).await {
                                Ok(service) => match service.backfill_memory_lexical_from_collection("agent_memory").await {
                                    Ok(count) => {
                                        tracing::info!("Memory lexical backfill completed: {} document(s)", count);
                                        match crate::skills::candidates::backfill_skill_candidates_from_memory(
                                            &db_service,
                                            "agent_memory",
                                        )
                                        .await
                                        {
                                            Ok(candidate_count) => {
                                                tracing::info!(
                                                    "Skill candidate backfill completed: {} candidate(s)",
                                                    candidate_count
                                                );
                                            }
                                            Err(e) => {
                                                tracing::warn!(
                                                    "Skill candidate backfill failed: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Memory lexical backfill failed: {}", e);
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("Failed to access RAG service for lexical backfill: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to ensure memory collection exists: {}", e);
                        }
                    }

                    // Initialize Memory Tool hooks through the unified memory service.
                    let app_handle_for_store = handle.clone();
                    let store_fn = Box::new(
                        move |content: String, title: Option<String>, tags: Vec<String>| {
                            let app_handle = app_handle_for_store.clone();
                            Box::pin(async move {
                                crate::memory::store_memory(&app_handle, content, title, tags)
                                    .await
                                    .map(crate::memory::map_store_result)
                            })
                                as std::pin::Pin<
                                    Box<
                                        dyn std::future::Future<
                                                Output = anyhow::Result<
                                                    sentinel_tools::buildin_tools::memory::MemoryManagerStoreResult,
                                                >,
                                            > + Send,
                                    >,
                                >
                        },
                    );

                    let app_handle_for_retrieve = handle.clone();
                    let retrieve_fn = Box::new(move |query: String, limit: usize| {
                        let app_handle = app_handle_for_retrieve.clone();
                        Box::pin(async move {
                            crate::memory::retrieve_memory_items_structured(
                                &app_handle,
                                query,
                                limit,
                            )
                            .await
                        })
                            as std::pin::Pin<
                                Box<
                                    dyn std::future::Future<
                                            Output = anyhow::Result<
                                                sentinel_tools::buildin_tools::memory::MemoryManagerRetrieveResult,
                                            >,
                                        > + Send,
                                >,
                            >
                    });

                    sentinel_tools::buildin_tools::memory::register_memory_functions(
                        store_fn,
                        retrieve_fn,
                    );
                }


                let traffic_state = Arc::new(TrafficAnalysisState::new(db_service.clone()));
                let traffic_state_for_manage = (*traffic_state).clone();

                // Extract PluginManager for workflow executor access
                let plugin_manager_for_workflow = traffic_state.get_plugin_manager();

                if let Err(e) = initialize_global_proxy(&db_service).await {
                    tracing::warn!("Failed to initialize global proxy configuration: {}", e);
                }
                if let Err(e) = tool_commands::init_exploitdb_runtime_config(&db_service).await {
                    tracing::warn!("Failed to initialize ExploitDB runtime configuration: {}", e);
                }

                let mcp_service = Arc::new(crate::services::mcp::McpService::new());
                handle.manage(mcp_service.clone());

                let ai_manager = AiServiceManager::new(db_service.clone());
                ai_manager.set_app_handle(handle.clone());

                if let Err(e) = ai_manager.init_default_services().await {
                    tracing::error!("Failed to initialize AI services: {}", e);
                }

                let ai_manager = Arc::new(ai_manager);

                if let Err(e) = ensure_default_system_agent_profiles(db_service.clone()).await {
                    tracing::warn!("Failed to seed default system agent profiles: {}", e);
                }
                if let Err(e) = traffic_state.hydrate_system_agent_proxy_settings().await {
                    tracing::warn!("Failed to hydrate traffic system-agent settings: {}", e);
                }

                let system_agent_runtime = Arc::new(SystemAgentRuntime::new(
                    db_service.clone(),
                    ai_manager.clone(),
                    handle.clone(),
                ));
                if let Err(e) = start_behavior_extension_bridge(
                    db_service.clone(),
                    traffic_state.clone(),
                    handle.clone(),
                )
                .await
                {
                    tracing::warn!("Failed to start browser behavior extension bridge: {}", e);
                }
                if let Err(e) = system_agent_runtime.recover_pending_runs().await {
                    tracing::warn!("Failed to recover pending system agent runs: {}", e);
                }

                let asset_service = crate::services::AssetService::new(db_service.clone());
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
                let db_for_tracker = db_service.clone();
                let db_service_for_enrichment = db_service.clone();
                let ai_manager_for_gateway = ai_manager.clone();

                // Register as concrete type
                handle.manage(db_service.clone());
                // Register as trait object for commands requesting Arc<dyn Database>
                // db_service is already Arc<DatabaseService>, so we just clone it and cast/coerce
                let db_trait_obj: Arc<dyn Database> = db_service.clone();
                handle.manage(db_trait_obj);
                handle.manage(ai_manager);
                handle.manage(system_agent_runtime.clone());
                handle.manage(asset_service);
                handle.manage(vulnerability_service);
                handle.manage(traffic_state_for_manage);
                handle.manage(plugin_manager_for_workflow);
                handle.manage(ProxifierState::new());
                handle.manage(PacketCaptureState::default());
                handle.manage(Arc::new(tokio::sync::RwLock::new(MonitorSchedulerState::new())));
                handle.manage(workflow_engine);
                handle.manage(workflow_scheduler);
                // Initialize asset enrichment service
                let enrichment_service = Arc::new(sentinel_bounty::services::AssetEnrichmentService::new(
                    db_service_for_enrichment
                ));
                handle.manage(enrichment_service);

                sentinel_tools::buildin_tools::plugin_authoring::register_plugin_authoring_executor(
                    std::sync::Arc::new({
                        let app_handle = handle.clone();
                        move |args| {
                            let app_handle = app_handle.clone();
                            Box::pin(async move {
                                let traffic_state = app_handle
                                    .try_state::<TrafficAnalysisState>()
                                    .ok_or_else(|| {
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringError {
                                            message: "TrafficAnalysisState not initialized".to_string(),
                                        }
                                    })?;
                                let ai_manager = app_handle
                                    .try_state::<Arc<AiServiceManager>>()
                                    .ok_or_else(|| {
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringError {
                                            message: "AiServiceManager not initialized".to_string(),
                                        }
                                    })?;
                                let runtime = app_handle
                                    .try_state::<Arc<SystemAgentRuntime>>()
                                    .ok_or_else(|| {
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringError {
                                            message: "SystemAgentRuntime not initialized".to_string(),
                                        }
                                    })?;

                                let request = crate::services::PluginAuthoringRequest {
                                    action: match args.action {
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::Generate => crate::services::PluginAuthoringAction::Generate,
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::Improve => crate::services::PluginAuthoringAction::Improve,
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::Validate => crate::services::PluginAuthoringAction::Validate,
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::Test => crate::services::PluginAuthoringAction::Test,
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::SaveDraft => crate::services::PluginAuthoringAction::SaveDraft,
                                        sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringAction::Enable => crate::services::PluginAuthoringAction::Enable,
                                    },
                                    main_category: args.main_category,
                                    category: args.category,
                                    requirements: args.requirements,
                                    existing_plugin_id: args.existing_plugin_id,
                                    plugin_id: args.plugin_id,
                                    name: args.name,
                                    description: args.description,
                                    author: args.author,
                                    default_severity: args.default_severity,
                                    monitor_type: args.monitor_type,
                                    traffic_samples: args.traffic_samples,
                                    code: args.code,
                                    enable_after_test: args.enable_after_test,
                                };

                                let result = crate::services::execute_plugin_authoring(
                                    &app_handle,
                                    &traffic_state,
                                    ai_manager.inner(),
                                    runtime.inner(),
                                    request,
                                )
                                .await
                                .map_err(|error| {
                                    sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringError {
                                        message: error.to_string(),
                                    }
                                })?;

                                Ok(
                                    sentinel_tools::buildin_tools::plugin_authoring::PluginAuthoringOutput {
                                        success: true,
                                        data: serde_json::to_value(result).ok(),
                                        error: None,
                                    },
                                )
                            })
                        }
                    }),
                );

                // Initialize Tenth Man executor
                crate::agents::tenth_man_executor::set_app_handle(handle.clone());
                crate::agents::tenth_man_executor::init_tenth_man_executor();
                tracing::info!("Tenth Man executor initialized");

                // Initialize Subagent executor
                crate::agents::subagent_executor::set_app_handle(handle.clone());
                crate::agents::subagent_executor::init_subagent_executor();
                tracing::info!("Subagent executor initialized");

                sentinel_plugins::register_app_handle(handle.clone());
                tracing::info!("Sentinel plugin runtime event bridge initialized");

                // Initialize tool execution tracker
                crate::trackers::init_tracker(db_for_tracker, handle.clone());
                tracing::info!("Tool execution tracker initialized");

                // Initialize shell permission handler
                if let Err(e) = tool_commands::init_shell_permission_handler(handle.clone()).await {
                    tracing::error!("Failed to init shell permission handler: {}", e);
                }
                if let Err(e) = tool_commands::init_ask_user_question_handler(handle.clone()).await
                {
                    tracing::error!("Failed to init ask user question handler: {}", e);
                }
                if let Err(e) = tool_commands::init_shell_background_runtime(handle.clone()).await {
                    tracing::error!("Failed to init shell background runtime: {}", e);
                }

                tracing::info!("Workflow engine and scheduler initialized");

                // Auto-start proxy listener if enabled in config
                let handle_for_proxy = handle.clone();
                let traffic_state_for_proxy = traffic_state.clone();
                tokio::spawn(async move {
                    // Wait a bit for app to be fully ready
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    if let Err(e) = auto_start_proxy_if_enabled(&handle_for_proxy, &traffic_state_for_proxy).await {
                        tracing::warn!("Failed to auto-start proxy listener: {}", e);
                    }
                });

                // Auto-start HTTP gateway if enabled in config
                let db_service_for_gateway = db_service.clone();
                let handle_for_gateway = handle.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    if let Err(e) = commands::http_gateway_commands::auto_start_http_gateway_if_enabled(
                        db_service_for_gateway,
                        ai_manager_for_gateway,
                        handle_for_gateway,
                    ).await {
                        tracing::warn!("Failed to auto-start HTTP gateway: {}", e);
                    }
                });

                // Auto-start monitor scheduler so enabled monitor tasks can run without visiting the UI.
                let handle_for_monitor = handle.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    match commands::monitor_start_scheduler(
                        handle_for_monitor
                            .state::<Arc<tokio::sync::RwLock<MonitorSchedulerState>>>(),
                        handle_for_monitor.state::<Arc<DatabaseService>>(),
                        handle_for_monitor.state::<Arc<sentinel_traffic::PluginManager>>(),
                        handle_for_monitor.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            tracing::info!("Monitor scheduler auto-started");
                        }
                        Err(error) if error.contains("Scheduler is already running") => {}
                        Err(error) => {
                            tracing::warn!("Failed to auto-start monitor scheduler: {}", error);
                        }
                    }
                });

                // Delay MCP server auto-connect to avoid blocking main process startup
                let handle_for_mcp = handle.clone();
                tokio::spawn(async move {
                    // Wait for main window to be ready
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    tracing::info!("Starting delayed MCP server auto-connect...");
                    // Register workflow tools
                    {
                        let tool_server = sentinel_tools::get_tool_server();
                        // Ensure builtin tools are initialized before loading workflow tools
                        // so that extract_input_schema can look up builtin tool schemas
                        tool_server.init_builtin_tools().await;
                        let db_workflow = db_service_for_mcp.clone();
                        sentinel_tools::workflow_adapter::load_workflows_from_db(
                            &tool_server,
                            || async move {
                                db_workflow
                                    .list_workflow_definitions(false)
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

                        if let Err(e) = cleanup_removed_agent_plugins(db_plugin.as_ref()).await {
                            tracing::warn!("Failed to clean up removed agent plugins: {}", e);
                        }

                        // 使用 Database trait 获取已启用的 agent 插件
                        let active_plugins = db_plugin.get_active_agent_plugins().await;

                        match active_plugins {
                            Ok(plugins) => {
                                let mut plugin_metas = Vec::new();
                                for p in plugins {
                                    let id = p.metadata.id.clone();
                                    if id.is_empty() {
                                        continue;
                                    }

                                    let description_str = p.metadata.description.as_deref().unwrap_or("Agent plugin tool");
                                    let code = db_plugin.get_plugin_code(&id).await.unwrap_or(None);

                                    // 使用运行时调用获取 input_schema
                                    let input_schema = if let Some(code_str) = &code {
                                        sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
                                            code_str,
                                            p.metadata.clone(),
                                        ).await
                                    } else {
                                        serde_json::json!({
                                            "type": "object",
                                            "properties": {}
                                        })
                                    };

                                    tracing::debug!("Plugin {} input_schema: {:?}", id, input_schema);

                                    plugin_metas.push(sentinel_tools::plugin_adapter::PluginToolMeta {
                                        plugin_id: id.clone(),
                                        name: p.metadata.name.clone(),
                                        description: description_str.to_string(),
                                        input_schema,
                                        code,
                                        category: Some(p.metadata.category.clone()),
                                    });
                                }

                                if !plugin_metas.is_empty() {
                                    tracing::info!("Loading {} plugin tools...", plugin_metas.len());
                                    sentinel_tools::plugin_adapter::load_plugin_tools_to_server(
                                        &tool_server,
                                        plugin_metas
                                    ).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to query plugin tools from database: {}", e);
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
            commands::agent_task_commands::get_agent_tasks,
            commands::assistant_profile_commands::list_assistant_profiles,
            commands::assistant_profile_commands::get_assistant_profile,
            commands::assistant_profile_commands::save_assistant_profiles,
            commands::assistant_profile_commands::get_default_assistant_profile_id,
            commands::assistant_profile_commands::save_default_assistant_profile_id,
            ai::save_ai_message,
            ai::cancel_ai_stream,
            ai::cancel_shell_execution,
            ai_conversation_binding_support::get_ai_conversation_binding,
            ai_execution_state_support::get_ai_conversation,
            ai_execution_state_support::get_ai_conversations,
            ai_execution_state_support::get_ai_conversations_paginated,
            ai_execution_state_support::get_ai_conversations_count,
            ai_turn_logs::get_ai_turn_logs,
            ai_turn_logs::get_ai_turn_log_detail,
            ai::get_ai_messages_by_conversation,
            ai::get_subagent_runs,
            ai::get_subagent_messages,
            ai::delete_subagent_runs_after,
            ai::clear_conversation_messages,
            ai::save_tool_config,
            ai_conversation_binding_support::save_ai_conversation_binding,
            ai::get_tool_config,
            ai::get_ai_conversation_history,
            ai::delete_ai_conversation,
            ai::update_ai_conversation_title,
            ai::archive_ai_conversation,
            ai::delete_ai_message,
            ai::delete_ai_messages_after,
            aisettings::test_ai_connection,
            aisettings::get_provider_models,
            aisettings::save_ai_config,
            aisettings::add_custom_provider,
            aisettings::delete_ai_provider,
            aisettings::restore_builtin_ai_provider,
            aisettings::get_ai_config,
            ai::print_ai_conversations,
            aisettings::set_default_llm_model,
            aisettings::set_default_llm_provider,
            aisettings::clear_model_vision_capability_cache,
            ai::upload_image_attachment,
            ai::upload_multiple_images,
            ai::agent_execute,
            ai::refresh_lm_studio_models,
            ai::get_lm_studio_status,
            ai::test_lm_studio_provider_connection,
            ai::save_scheduler_config,
            ai::get_ai_usage_stats,
            ai::get_detailed_ai_usage_stats,
            ai::clear_ai_usage_stats,
            ai::generate_workflow_from_nl,
            ai::generate_plugin_stream,
            ai::generate_ai_role,
            ai::cancel_plugin_generation,
            ai::plugin_assistant_chat_stream,
            ai::cancel_plugin_assistant_chat,
            commands::get_active_rag_collections,
            commands::set_rag_collection_active,
            tool_commands::get_pending_ask_user_questions,
            tool_commands::respond_ask_user_question,
            tool_commands::reject_ask_user_question,
            tool_commands::get_background_shell_tasks,
            tool_commands::stop_background_shell_task,
            // Plugin generation commands
            commands::get_combined_plugin_prompt_api,
            // System Agent commands
            commands::list_system_agent_profiles,
            commands::get_system_agent_profile,
            commands::save_system_agent_profile,
            commands::set_system_agent_profiles_enabled,
            commands::delete_system_agent_profile,
            commands::list_system_agent_runs,
            commands::delete_system_agent_run,
            commands::clear_system_agent_runs,
            commands::list_system_agent_profile_versions,
            commands::trigger_system_agent_profile,
            commands::dispatch_system_agent_event,
            commands::seed_system_agent_profiles,
            commands::get_system_agent_auto_verification_status,
            commands::set_system_agent_auto_verification_enabled,
            // AI task command plus temporary compatibility alias
            commands::fix_plugin_with_ai_task,
            commands::fix_plugin_with_system_agent,
            commands::verify_finding_with_system_agent,
            commands::submit_system_agent_finding_feedback,
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
            db_commands::test_db_connection,
            db_commands::export_db_to_json,
            db_commands::export_db_to_sql,
            db_commands::import_db_from_json,
            db_commands::migrate_database,
            db_commands::save_db_config,
            db_commands::load_db_config,
            // Cache commands
            get_cache,
            set_cache,
            delete_cache,
            cleanup_expired_cache,
            get_all_cache_keys,
            // Asset commands
            asset::init_asset_service,
            asset::create_asset,
            asset::get_asset_detail,
            asset::update_asset,
            asset::delete_asset,
            asset::delete_assets,
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
            // Bug Bounty commands
            commands::bounty_create_program,
            commands::bounty_get_program,
            commands::bounty_update_program,
            commands::bounty_delete_program,
            commands::bounty_list_programs,
            commands::bounty_get_program_stats,
            commands::bounty_create_scope,
            commands::bounty_get_scope,
            commands::bounty_update_scope,
            commands::bounty_delete_scope,
            commands::bounty_list_scopes,
            commands::bounty_backfill_domain_scopes_from_assets,
            commands::bounty_backfill_domain_asset_hierarchy,
            commands::bounty_validate_scope,
            // Bug Bounty Finding commands
            commands::bounty_create_finding,
            commands::bounty_get_finding,
            commands::bounty_update_finding,
            commands::bounty_delete_finding,
            commands::bounty_batch_update_finding_status,
            commands::bounty_batch_delete_findings,
            commands::bounty_delete_all_findings,
            commands::bounty_list_findings,
            commands::bounty_count_findings,
            commands::bounty_get_finding_stats,
            // Bug Bounty Evidence commands
            commands::bounty_create_evidence,
            commands::bounty_get_evidence,
            commands::bounty_update_evidence,
            commands::bounty_delete_evidence,
            commands::bounty_list_evidence,
            // Bug Bounty Submission commands
            commands::bounty_create_submission,
            commands::bounty_get_submission,
            commands::bounty_update_submission,
            commands::bounty_delete_submission,
            commands::bounty_batch_update_submission_status,
            commands::bounty_batch_delete_submissions,
            commands::bounty_list_submissions,
            commands::bounty_count_submissions,
            commands::bounty_get_submission_stats,
            // Bug Bounty Knowledge Base commands
            commands::bounty_create_knowledge_note,
            commands::bounty_get_knowledge_note,
            commands::bounty_update_knowledge_note,
            commands::bounty_delete_knowledge_note,
            commands::bounty_list_knowledge_notes,
            commands::bounty_search_knowledge_notes,
            commands::bounty_get_knowledge_note_stats,
            // Bug Bounty Change Event commands
            commands::bounty_create_change_event,
            commands::bounty_get_change_event,
            commands::bounty_update_change_event,
            commands::bounty_delete_change_event,
            commands::bounty_list_change_events,
            commands::bounty_get_change_event_stats,
            commands::bounty_update_change_event_status,
            commands::bounty_add_generated_finding,
            commands::bounty_batch_delete_api_inventory_targets,
            commands::bounty_get_api_inventory_target,
            commands::bounty_list_api_inventory_target_keys,
            commands::bounty_list_api_inventory_targets,
            commands::bounty_import_traffic_finding,
            commands::bounty_batch_import_traffic_findings,
            commands::bounty_export_report,
            // Bug Bounty Workflow Template commands
            commands::bounty_create_workflow_template,
            commands::bounty_get_workflow_template,
            commands::bounty_list_workflow_templates,
            commands::bounty_delete_workflow_template,
            commands::bounty_update_workflow_template,
            commands::bounty_run_workflow_template,
            commands::bounty_run_workflow_template_for_event,
            commands::bounty_list_change_event_workflow_runs,
            commands::bounty_retry_change_event_workflow_run,
            commands::bounty_create_workflow_binding,
            commands::bounty_update_workflow_binding,
            commands::bounty_list_workflow_bindings,
            commands::bounty_delete_workflow_binding,
            commands::bounty_init_builtin_templates,
            commands::bounty_sink_workflow_outputs,
            commands::bounty_get_triggered_workflows,
            commands::bounty_trigger_workflows_for_event,
            // Bug Bounty Asset commands
            commands::bounty_create_asset,
            commands::bounty_import_assets_from_scope,
            // Bug Bounty Asset Fingerprint & Labels (P1-B4)
            commands::bounty_update_asset_fingerprint,
            commands::bounty_add_asset_labels,
            commands::bounty_get_high_value_labels,
            commands::bounty_get_assets_by_label,
            commands::bounty_get_assets_by_tech,
            // Bug Bounty Priority Scoring (P1-B5)
            commands::bounty_recalculate_asset_priority,
            commands::bounty_recalculate_all_asset_priorities,
            commands::bounty_get_priority_queue,
            // Bug Bounty Submission Operations (D3)
            commands::bounty_add_submission_timeline_event,
            commands::bounty_get_submission_with_timeline,
            commands::bounty_get_submissions_needing_followup,
            commands::bounty_schedule_retest,
            commands::bounty_record_retest_result,
            // Bug Bounty Workflow Orchestration (P0)
            commands::bounty_resolve_step_inputs,
            commands::bounty_process_step_output,
            commands::bounty_sink_artifacts,
            commands::bounty_get_default_retry_config,
            commands::bounty_get_rate_limiter_stats,
            commands::bounty_get_plugin_ports,
            commands::bounty_list_plugin_ports,
            // Monitor commands
            commands::monitor_start_scheduler,
            commands::monitor_stop_scheduler,
            commands::monitor_is_running,
            commands::monitor_get_stats,
            commands::monitor_create_task,
            commands::monitor_get_task,
            commands::monitor_list_tasks,
            commands::monitor_get_running_tasks,
            commands::monitor_delete_task,
            commands::monitor_enable_task,
            commands::monitor_disable_task,
            commands::monitor_trigger_task,
            commands::monitor_stop_task,
            commands::monitor_update_task,
            commands::monitor_create_default_tasks,
            commands::monitor_discover_and_import_assets,
            commands::monitor_get_available_plugins,
            commands::monitor_test_plugin,
            commands::monitor_list_run_history,
            commands::monitor_update_task_plugins,
            // Surface graph commands
            commands::surface_get_overview,
            commands::surface_get_fingerprint_category_aggregation,
            commands::surface_list_fingerprint_assets,
            commands::surface_list_assets,
            commands::surface_list_inventory,
            commands::surface_get_inventory_facets,
            commands::surface_manual_import_assets,
            commands::surface_update_asset,
            commands::surface_delete_asset,
            commands::surface_batch_delete_assets,
            commands::surface_delete_inventory,
            commands::surface_list_relations,
            commands::surface_list_discovery_runs,
            commands::surface_get_topology,
            commands::surface_get_asset_detail,
            commands::surface_get_discovery_run_detail,
            commands::surface_create_observation,
            // Asset enrichment commands
            commands::asset_enrichment_commands::enrich_asset,
            commands::asset_enrichment_commands::start_asset_enrichment,
            commands::asset_enrichment_commands::stop_asset_enrichment,
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
            // LLM test commands
            llm_test_commands::llm_test_create_run,
            llm_test_commands::llm_test_execute_case,
            llm_test_commands::llm_test_execute_cases,
            llm_test_commands::llm_test_get_run,
            llm_test_commands::llm_test_list_runs,
            llm_test_commands::llm_test_stop_run,
            llm_test_commands::llm_test_reset_run,
            llm_test_commands::llm_test_delete_run,
            llm_test_commands::llm_test_list_suites,
            llm_test_commands::llm_test_save_suite,
            llm_test_commands::llm_test_delete_suite,
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
            dictionary::get_dictionaries_paged,
            dictionary::get_dictionary,
            dictionary::create_dictionary,
            dictionary::update_dictionary,
            dictionary::delete_dictionary,
            dictionary::get_dictionary_words,
            dictionary::get_dictionary_words_paged,
            dictionary::add_dictionary_words,
            dictionary::add_dictionary_entries,
            dictionary::update_dictionary_word,
            dictionary::update_dictionary_words_batch,
            dictionary::remove_dictionary_words,
            dictionary::search_dictionary_words,
            dictionary::clear_dictionary,
            dictionary::export_dictionary,
            dictionary::import_dictionary,
            dictionary::import_dictionary_from_file,
            dictionary::import_nmap_service_probes,
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
            rag_commands::list_rag_documents_paginated,
            rag_commands::rag_batch_ingest_sources,
            rag_commands::get_rag_document_chunks,
            rag_commands::delete_rag_document,
            rag_commands::ensure_default_rag_collection,
            rag_commands::test_embedding_connection,
            memory_commands::list_durable_memory_diagnostics,
            memory_commands::get_durable_memory_projection_states,
            memory_commands::get_durable_memory_diagnostics_by_ids,
            // CPG security rule commands
            // Traffic scan commands
            traffic::start_traffic_analysis,
            traffic::stop_traffic_analysis,
            traffic::get_proxy_status,
            traffic::reload_plugin_in_pipeline,
            traffic::list_findings,
            traffic::count_findings,
            commands::get_finding_lifecycle_stats,
            commands::get_traffic_behavior_effect_stats,

            traffic::enable_plugin,
            traffic::disable_plugin,
            traffic::batch_enable_plugins,
            traffic::batch_disable_plugins,
            traffic::list_plugins,
            traffic::intruder_list_plugins,
            traffic::intruder_generate_payloads,
            traffic::intruder_process_payload,
            traffic::intruder_transform_request,
            commands::download_ca_cert,
            commands::get_ca_cert_path,
            commands::trust_ca_cert,
            commands::regenerate_ca_cert,
            commands::get_ca_fingerprint,
            commands::open_ca_cert_dir,
            commands::export_ca_cert,
            commands::export_ca_key,
            commands::export_ca_pkcs12,
            traffic::get_finding,
            traffic::review_finding_with_ai,
            traffic::update_finding_status,
            traffic::export_findings_html,
            traffic::list_proxy_requests,
            traffic::get_proxy_request,
            traffic::resolve_proxy_history_request_id_by_db_request_id,
            traffic::load_traffic_draft_store,
            traffic::save_traffic_draft_store,
            traffic::load_attack_workspace_store,
            traffic::save_attack_workspace_store,
            traffic::load_replay_run_store,
            traffic::save_replay_run_store,
            traffic::load_intruder_workspace_session_store,
            traffic::save_intruder_workspace_session_store,
            traffic::recommend_traffic_context_dictionary_candidates_command,
            traffic::preview_traffic_context_extraction_changes_command,
            traffic::clear_proxy_requests,
            traffic::count_proxy_requests,
            traffic::create_plugin_in_db,
            commands::upload_plugin,
            traffic::update_plugin,
            traffic::get_plugin_code,
            traffic::get_plugin_by_id,
            traffic::test_plugin,
            traffic::delete_plugin,
            traffic::delete_traffic_vulnerability,
            traffic::delete_traffic_vulnerabilities_batch,
            traffic::delete_all_traffic_vulnerabilities,
            traffic::clear_vulnerability_dedupe_cache,
            traffic::get_vulnerability_dedupe_cache_info,
            traffic::test_plugin_advanced,
            traffic::test_agent_plugin,
            traffic::get_plugin_input_schema,
            traffic::get_plugin_output_schema,
            traffic::start_proxy_listener,
            traffic::stop_proxy_listener,
            traffic::save_proxy_config,
            traffic::get_proxy_config,
            traffic::set_proxy_auto_start,
            traffic::get_proxy_auto_start,
            traffic::set_traffic_analysis_plugin_enabled,
            traffic::get_traffic_analysis_plugin_enabled,
            traffic::get_traffic_behavior_signal_settings,
            traffic::get_traffic_behavior_extension_installation,
            traffic::copy_traffic_behavior_extension_to_directory,
            traffic::read_traffic_clipboard_text,
            traffic::set_traffic_behavior_signal_settings,
            traffic::get_traffic_oast_config,
            traffic::set_traffic_oast_config,
            traffic::test_traffic_oast_config_command,
            traffic::create_traffic_oast_token,
            traffic::list_traffic_oast_records,
            traffic::sync_traffic_oast_records,
            traffic::delete_traffic_oast_record,
            traffic::hide_traffic_oast_events_command,
            traffic::get_traffic_context_extraction_settings,
            traffic::set_traffic_context_extraction_settings,
            traffic::get_traffic_plugin_runtime_settings,
            traffic::set_traffic_plugin_runtime_settings,
            traffic::get_active_probe_queue_snapshot,
            commands::security_workbench_list_cases,
            commands::security_workbench_get_or_create_case_for_finding,
            commands::security_workbench_get_case_detail,
            commands::security_workbench_update_case,
            commands::security_workbench_delete_cases,
            commands::security_workbench_ignore_cases,
            commands::security_workbench_list_ignored_findings,
            commands::security_workbench_restore_ignored_findings,
            commands::security_workbench_add_note,
            commands::security_workbench_sync_case_to_finding,
            commands::security_workbench_create_execution_draft,
            commands::security_workbench_update_execution_draft,
            commands::security_workbench_execute_execution_draft,
            traffic::set_intercept_enabled,
            traffic::get_intercept_enabled,
            traffic::set_request_intercept_enabled,
            traffic::get_request_intercept_enabled,
            traffic::get_intercepted_requests,
            traffic::forward_intercepted_request,
            traffic::drop_intercepted_request,
            traffic::set_response_intercept_enabled,
            traffic::get_response_intercept_enabled,
            traffic::get_intercepted_responses,
            traffic::forward_intercepted_response,
            traffic::drop_intercepted_response,
            traffic::replay_request,
            traffic::replay_raw_request,
            traffic::list_websocket_connections,
            traffic::list_websocket_messages,
            traffic::clear_websocket_history,
            traffic::get_history_stats,
            traffic::clear_all_history,
            traffic::set_websocket_intercept_enabled,
            traffic::get_websocket_intercept_enabled,
            traffic::forward_intercepted_websocket,
            traffic::drop_intercepted_websocket,
            traffic::add_intercept_filter_rule,
            traffic::get_intercept_filter_rules,
            traffic::remove_intercept_filter_rule,
            traffic::update_intercept_filter_rule,
            traffic::update_runtime_filter_rules,
            // Plugin store commands
            traffic::fetch_store_plugins,
            traffic::fetch_plugin_code,
            traffic::install_store_plugin,
            traffic::update_store_plugin,
            // Proxifier commands
            proxifier_commands::get_proxifier_config,
            proxifier_commands::start_proxifier,
            proxifier_commands::stop_proxifier,
            proxifier_commands::save_proxifier_proxies,
            proxifier_commands::save_proxifier_rules,
            proxifier_commands::get_proxifier_connections,
            proxifier_commands::clear_proxifier_connections,
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
            commands::plugin_review_commands::validate_plugin_code,
            commands::plugin_review_commands::validate_plugin_runtime_schema,
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
            commands::plugin_authoring_commands::plugin_authoring_execute,
            // Notifications
            commands::notifications::send_notification,
            commands::notifications::create_notification_rule,
            commands::notifications::update_notification_rule,
            commands::notifications::delete_notification_rule,
            commands::notifications::list_notification_rules,
            commands::notifications::get_notification_rule,
            commands::notifications::test_notification_rule_connection,
            // Packet capture (requires Npcap on Windows)
            packet_capture_commands::get_network_interfaces,
            packet_capture_commands::start_packet_capture,
            packet_capture_commands::stop_packet_capture,
            packet_capture_commands::is_capture_running,
            packet_capture_commands::clear_packet_capture_cache,
            packet_capture_commands::open_pcap_file,
            packet_capture_commands::save_pcap_file,
            packet_capture_commands::extract_files_preview,
            packet_capture_commands::extract_files_to_dir,
            packet_capture_commands::save_extracted_file,
            packet_capture_commands::get_file_related_packets,
            packet_capture_commands::get_file_stream_packets,
            packet_capture_commands::get_packet_details,
            packet_capture_commands::get_packet_stream_packets,
            packet_capture_commands::match_packet_advanced_filter,
            packet_capture_commands::save_selected_files,
            // Test commands
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
            tool_commands::get_exploitdb_settings,
            tool_commands::save_exploitdb_settings,
            tool_commands::get_exploitdb_sync_status,
            tool_commands::sync_exploitdb,
            tool_commands::browse_exploitdb_entries,
            tool_commands::get_exploitdb_entry_detail,
            // Task Tool Integration commands
            commands::task_tool_commands::get_task_active_tools,
            commands::task_tool_commands::get_task_tool_statistics,
            commands::task_tool_commands::get_tool_execution_history,
            commands::task_tool_commands::record_tool_execution_start,
            commands::task_tool_commands::record_tool_execution_complete,
            commands::task_tool_commands::get_all_active_tools,

            // Test tracking commands
            commands::test_tracking_commands::test_plugin_tracking,
            commands::test_tracking_commands::test_mcp_tracking,
            commands::test_tracking_commands::test_builtin_tracking,
            commands::test_tracking_commands::test_error_tracking,

            // Shell Tool commands
            tool_commands::init_ask_user_question_handler,
            tool_commands::init_shell_background_runtime,
            tool_commands::init_shell_permission_handler,
            tool_commands::get_shell_tool_config,
            tool_commands::set_shell_tool_config,
            tool_commands::respond_shell_permission,
            tool_commands::allow_shell_permission_forever,
            tool_commands::get_pending_shell_permissions,
            tool_commands::get_shell_permission_history,
            // Shell Docker Sandbox commands
            commands::get_shell_configuration,
            commands::update_shell_configuration,
            commands::initialize_docker_sandbox,
            commands::check_docker_availability,
            commands::build_docker_sandbox_image,
            commands::cleanup_docker_containers,
            commands::cleanup_shell_container,
            commands::get_shell_container_info,
            // Document attachment commands
            commands::get_file_stat,
            commands::check_docker_for_file_analysis,
            commands::upload_document_attachment,
            commands::run_file_security_analysis,
            commands::get_workspace_settings,
            commands::save_workspace_settings,
            commands::list_uploaded_files,
            commands::clear_uploaded_files,
            commands::search_recent_proxy_requests,
            commands::search_working_directory_files,
            commands::read_working_directory_file_preview,
            // Terminal WebSocket commands
            commands::start_terminal_server,
            commands::stop_terminal_server,
            commands::get_terminal_server_status,
            commands::list_terminal_sessions,
            commands::stop_terminal_session,
            commands::get_terminal_websocket_url,
            commands::cleanup_terminal_containers,
            commands::get_terminal_container_info,
            commands::start_http_gateway,
            commands::stop_http_gateway,
            commands::get_http_gateway_status,
            commands::get_http_gateway_config,
            commands::save_http_gateway_config,
            commands::rotate_http_gateway_api_key,
            // Agent config commands
            tool_commands::get_agent_config,
            tool_commands::save_agent_config,
            // Skill commands
            tool_commands::list_skills,
            tool_commands::list_skills_full,
            tool_commands::get_skill_detail,
            tool_commands::get_skill,
            tool_commands::get_skill_markdown,
            tool_commands::create_skill,
            tool_commands::update_skill,
            tool_commands::delete_skill,
            tool_commands::refresh_skills_index,
            tool_commands::list_skill_candidates,
            tool_commands::list_skill_candidate_suppression_rules,
            tool_commands::delete_skill_candidate_suppression_rule,
            tool_commands::extend_skill_candidate_suppression_rule,
            tool_commands::expire_skill_candidate_suppression_rule,
            tool_commands::promote_skill_candidate,
            tool_commands::review_skill_candidate,
            tool_commands::list_skill_files,
            tool_commands::read_skill_file,
            tool_commands::save_skill_file,
            tool_commands::delete_skill_file,
            tool_commands::import_skill_file,
            tool_commands::discover_skills_from_path,
            tool_commands::discover_skills_from_git,
            tool_commands::install_skills_from_path,
            tool_commands::list_skill_install_history,
            tool_commands::delete_skill_install_history,
            // Tool Server commands
            tool_commands::tool_server::init_tool_server,
            tool_commands::tool_server::list_tool_server_tools,
            tool_commands::tool_server::list_tools_by_source,
            tool_commands::tool_server::get_tool_server_tool,
            tool_commands::tool_server::get_tool_input_schema,
            tool_commands::tool_server::get_tool_output_schema,
            tool_commands::tool_server::execute_tool_server_tool,
            tool_commands::tool_server::get_tool_server_stats,
            tool_commands::tool_server::register_mcp_tools_from_server,
            tool_commands::tool_server::register_workflow_tools,
            tool_commands::tool_server::refresh_all_dynamic_tools,
            // MCP commands
            commands::mcp_commands::mcp_get_connections,
            commands::mcp_commands::mcp_get_connection_status,
            commands::mcp_commands::add_child_process_mcp_server,
            commands::mcp_commands::mcp_connect_server,
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
            commands::license_commands::get_app_entitlements,
            commands::license_commands::store_entitlement_token,
            commands::license_commands::clear_entitlement_token,
            commands::license_commands::get_entitlement_token_status,
            commands::license_commands::get_entitlement_refresh_config,
            commands::license_commands::save_entitlement_refresh_config,
            commands::license_commands::refresh_entitlement_token,
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
            // Team V3 commands (non-backward-compatible cutover)
            commands::team_v3_schema::team_v3_ensure_schema,
            commands::team_v3_schema::team_v3_reset_schema,
            commands::team_v3_api::team_v3_create_session,
            commands::team_v3_api::team_v3_get_session,
            commands::team_v3_api::team_v3_list_sessions,
            commands::team_v3_api::team_v3_update_session,
            commands::team_v3_api::team_v3_start_execution,
            commands::team_v3_api::team_v3_stop_execution,
            commands::team_v3_api::team_v3_finalize_execution,
            commands::team_v3_api::team_v3_get_run_status,
            commands::team_v3_api::team_v3_create_task,
            commands::team_v3_api::team_v3_list_tasks,
            commands::team_v3_api::team_v3_claim_task,
            commands::team_v3_api::team_v3_release_task_claim,
            commands::team_v3_api::team_v3_update_task_status,
            commands::team_v3_api::team_v3_send_message,
            commands::team_v3_api::team_v3_list_thread_messages,
            commands::team_v3_api::team_v3_list_messages,
            commands::team_v3_api::team_v3_list_blackboard_entries,
            commands::team_v3_api::team_v3_submit_plan_revision,
            commands::team_v3_api::team_v3_review_plan_revision,
        ])
        .run(context)
        .expect("Failed to start Tauri application");
}

async fn toggle_proxy(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<TrafficAnalysisState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;

        if is_running {
            match traffic::stop_traffic_analysis_internal(app, &state).await {
                Ok(_) => {
                    tracing::info!("Proxy stopped from tray menu");
                    update_proxy_menu_text(app, false);
                }
                Err(e) => tracing::error!("Failed to stop proxy: {}", e),
            }
        } else {
            match traffic::start_traffic_analysis_internal(app, &state, None).await {
                Ok(port) => {
                    tracing::info!("Proxy started from tray menu on port {}", port);
                    update_proxy_menu_text(app, true);
                }
                Err(e) => tracing::error!("Failed to start proxy: {}", e),
            }
        }
    }
}

pub(crate) fn update_proxy_menu_text(app: &tauri::AppHandle, is_running: bool) {
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

    if let Some(state) = app.try_state::<TrafficAnalysisState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;
        if is_running {
            let _ = traffic::stop_traffic_analysis_internal(app, &state).await;
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
                } else {
                    sentinel_core::global_proxy::clear_global_proxy().await;
                }
            }
            Err(e) => tracing::warn!("Failed to parse proxy configuration JSON: {}", e),
        },
        Ok(None) => {
            sentinel_core::global_proxy::clear_global_proxy().await;
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy configuration from database: {}", e);
            sentinel_core::global_proxy::clear_global_proxy().await;
        }
    }
    Ok(())
}

/// Auto-start proxy listener if enabled in configuration
async fn auto_start_proxy_if_enabled(
    app: &tauri::AppHandle,
    traffic_state: &TrafficAnalysisState,
) -> anyhow::Result<()> {
    tracing::info!("Checking if proxy auto-start is enabled...");

    // Get database service
    let db_service = traffic_state.get_db_service();

    // Load proxy auto-start configuration
    let auto_start_enabled = match db_service
        .load_proxy_config("proxy_auto_start_enabled")
        .await
    {
        Ok(Some(value)) => value.parse::<bool>().unwrap_or(false),
        _ => false,
    };

    if !auto_start_enabled {
        tracing::info!("Proxy auto-start is disabled in configuration");
        return Ok(());
    }

    // Check if proxy is already running using public method
    let is_running_arc = traffic_state.get_is_running();
    let is_running = *is_running_arc.read().await;
    if is_running {
        tracing::info!("Proxy is already running, skipping auto-start");
        return Ok(());
    }

    // Load proxy configuration from database
    let config = match db_service.load_proxy_config("proxy_config").await {
        Ok(Some(config_json)) => {
            match serde_json::from_str::<sentinel_traffic::ProxyConfig>(&config_json) {
                Ok(config) => {
                    tracing::info!(
                        "Loaded proxy configuration for auto-start: port {}",
                        config.start_port
                    );
                    Some(config)
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize proxy config, using default: {}", e);
                    None
                }
            }
        }
        Ok(None) => {
            tracing::info!("No saved proxy configuration found, using default");
            None
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy config from database: {}", e);
            None
        }
    };

    // Start the proxy listener
    match traffic::start_traffic_analysis_internal(app, traffic_state, config).await {
        Ok(port) => {
            tracing::info!(
                "✅ Proxy listener auto-started successfully on port {}",
                port
            );
            update_proxy_menu_text(app, true);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to auto-start proxy listener: {}", e);
            Err(anyhow::anyhow!("Failed to start proxy: {}", e))
        }
    }
}
