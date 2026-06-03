use std::sync::Arc;

use sentinel_db::{Database, DatabaseService};
use tauri::Manager;

use super::lifecycle;
use crate::commands::{
    packet_capture_commands::PacketCaptureState,
    proxifier_commands::ProxifierState, tool_commands, traffic::TrafficAnalysisState,
};
use crate::services::{
    ai::AiServiceManager,
    load_plugin_default_inputs,
    system_agents::{ensure_default_system_agent_profiles, start_behavior_extension_bridge},
    SystemAgentRuntime,
};
use crate::skills::scan_and_upsert_skills;
use crate::utils::plugin_registry_cleanup::cleanup_removed_agent_plugins;

use sentinel_workflow::{WorkflowEngine, WorkflowScheduler};

pub fn setup_app(app: &mut tauri::App<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    lifecycle::setup_tray(app, &handle)?;

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

        match crate::commands::aisettings::cleanup_legacy_ai_config_keys(db_service.as_ref()).await
        {
            Ok(removed) if removed > 0 => {
                tracing::info!("Removed {} legacy AI config key(s) during startup", removed);
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
            Ok(sentinel_db::database_service::connection_manager::DatabasePool::SQLite(pool)) => {
                sentinel_plugins::init_dictionary_pool(pool);
                tracing::info!("Dictionary pool initialized for plugins using SQLite");
            }
            #[cfg(feature = "db-postgres")]
            Ok(sentinel_db::database_service::connection_manager::DatabasePool::PostgreSQL(
                pool,
            )) => {
                sentinel_plugins::init_dictionary_pool(pool);
                tracing::info!("Dictionary pool initialized for plugins using PostgreSQL");
            }
            #[cfg(all(not(feature = "db-postgres"), feature = "db-mysql"))]
            Ok(sentinel_db::database_service::connection_manager::DatabasePool::MySQL(pool)) => {
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
                tracing::warn!("Failed to initialize dictionary pool for plugins: {error:#}");
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

        // Initialize agent configuration from database
        if let Err(e) = tool_commands::init_agent_config(&db_service).await {
            tracing::error!("Failed to initialize agent config: {}", e);
        }

        if let Err(e) = scan_and_upsert_skills(&db_service).await {
            tracing::warn!("Skills scan warning: {}", e);
        }

        if let Err(e) =
            crate::commands::rag_commands::initialize_global_rag_service(db_service.clone()).await
        {
            tracing::error!("Failed to initialize global RAG service: {}", e);
        } else {
            // Ensure memory collection exists
            match crate::commands::rag_commands::ensure_memory_collection_exists(db_service.clone())
                .await
            {
                Ok(collection_id) => {
                    tracing::info!("Memory collection ready: {}", collection_id);
                    match crate::commands::rag_commands::get_or_init_rag_service(db_service.clone())
                        .await
                    {
                        Ok(service) => match service
                            .backfill_memory_lexical_from_collection("agent_memory")
                            .await
                        {
                            Ok(count) => {
                                tracing::info!(
                                    "Memory lexical backfill completed: {} document(s)",
                                    count
                                );
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
                            tracing::warn!(
                                "Failed to access RAG service for lexical backfill: {}",
                                e
                            );
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

            sentinel_tools::buildin_tools::memory::register_memory_functions(store_fn, retrieve_fn);
        }

        sentinel_tools::buildin_tools::mission_scheduler::set_mission_scheduler_app_handle(
            handle.clone(),
        )
        .await;

        let traffic_state = Arc::new(TrafficAnalysisState::new(db_service.clone()));
        let traffic_state_for_manage = (*traffic_state).clone();

        // Extract PluginManager for workflow executor access
        let plugin_manager_for_workflow = traffic_state.get_plugin_manager();

        if let Err(e) = lifecycle::initialize_global_proxy(&db_service).await {
            tracing::warn!("Failed to initialize global proxy configuration: {}", e);
        }
        if let Err(e) = tool_commands::init_exploitdb_runtime_config(&db_service).await {
            tracing::warn!(
                "Failed to initialize ExploitDB runtime configuration: {}",
                e
            );
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
        let ai_manager_for_gateway = ai_manager.clone();
        let ai_manager_for_weixin = ai_manager.clone();
        let ai_manager_for_mission_scheduler = ai_manager.clone();

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
        handle.manage(workflow_engine);
        handle.manage(workflow_scheduler);

        sentinel_tools::buildin_tools::set_browser_shell_handler(
            crate::agents::executor::build_browser_shell_handler(handle.clone()),
        )
        .await;

        // Register browser automation handler (CDP-based humanized browser control)
        sentinel_tools::buildin_tools::set_browser_automation_handler(
            std::sync::Arc::new(sentinel_tools::buildin_tools::BrowserAutomationState::new()),
        )
        .await;

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
        if let Err(e) = tool_commands::init_ask_user_question_handler(handle.clone()).await {
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

            if let Err(e) =
                lifecycle::auto_start_proxy_if_enabled(&handle_for_proxy, &traffic_state_for_proxy)
                    .await
            {
                tracing::warn!("Failed to auto-start proxy listener: {}", e);
            }
        });

        // Auto-start HTTP gateway if enabled in config
        let db_service_for_gateway = db_service.clone();
        let handle_for_gateway = handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Err(e) =
                crate::commands::http_gateway_commands::auto_start_http_gateway_if_enabled(
                    db_service_for_gateway,
                    ai_manager_for_gateway,
                    handle_for_gateway,
                )
                .await
            {
                tracing::warn!("Failed to auto-start HTTP gateway: {}", e);
            }
        });

        // Auto-start Weixin remote-control gateway if enabled in config.
        let db_service_for_weixin = db_service.clone();
        let handle_for_weixin = handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Err(e) =
                crate::commands::weixin_gateway_commands::auto_start_weixin_gateway_if_enabled(
                    db_service_for_weixin,
                    ai_manager_for_weixin,
                    handle_for_weixin,
                )
                .await
            {
                tracing::warn!("Failed to auto-start Weixin gateway: {}", e);
            }
        });

        // Auto-start mission scheduler with startup recovery
        let db_for_mission_scheduler = db_service.clone();
        let handle_for_mission_scheduler = handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            crate::services::mission_scheduler::run_startup_recovery(&db_for_mission_scheduler)
                .await;

            let cancel = tokio_util::sync::CancellationToken::new();
            crate::services::mission_scheduler::spawn_mission_scheduler(
                db_for_mission_scheduler,
                ai_manager_for_mission_scheduler,
                handle_for_mission_scheduler,
                cancel,
            );
            tracing::info!("Mission scheduler auto-started");
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

                            let description_str = p
                                .metadata
                                .description
                                .as_deref()
                                .unwrap_or("Agent plugin tool");
                            let code = db_plugin.get_plugin_code(&id).await.unwrap_or(None);

                            // 使用运行时调用获取 input_schema / output_schema
                            let (input_schema, output_schema) = if let Some(code_str) = &code {
                                let input_schema = sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
                                            code_str,
                                            p.metadata.clone(),
                                        ).await;
                                let output_schema = sentinel_tools::plugin_adapter::PluginToolAdapter::get_output_schema_runtime_optional(
                                            code_str,
                                            p.metadata.clone(),
                                        ).await;
                                (input_schema, output_schema)
                            } else {
                                (
                                    serde_json::json!({
                                        "type": "object",
                                        "properties": {}
                                    }),
                                    None,
                                )
                            };
                            let default_input = load_plugin_default_inputs(db_plugin.as_ref(), &id)
                                .await
                                .unwrap_or_else(|_| serde_json::json!({}));

                            tracing::debug!("Plugin {} input_schema: {:?}", id, input_schema);

                            plugin_metas.push(sentinel_tools::plugin_adapter::PluginToolMeta {
                                plugin_id: id.clone(),
                                name: p.metadata.name.clone(),
                                description: description_str.to_string(),
                                input_schema,
                                output_schema,
                                default_input,
                                code,
                            });
                        }

                        if !plugin_metas.is_empty() {
                            tracing::info!("Loading {} plugin tools...", plugin_metas.len());
                            sentinel_tools::plugin_adapter::load_plugin_tools_to_server(
                                &tool_server,
                                plugin_metas,
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to query plugin tools from database: {}", e);
                    }
                }
            }

            crate::commands::mcp_commands::mcp_auto_connect_servers(
                db_service_for_mcp,
                handle_for_mcp,
            )
            .await;
        });
    });

    Ok(())
}
