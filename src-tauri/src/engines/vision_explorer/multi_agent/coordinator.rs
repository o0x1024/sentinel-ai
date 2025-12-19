//! Multi-Agent Exploration Coordinator
//!
//! Orchestrates Manager and Worker agents for parallel website exploration

use super::global_state::GlobalExplorerState;
use super::manager_agent::ManagerAgent;
use super::types::*;
use super::worker_agent::WorkerAgent;
use crate::commands::passive_scan_commands::PassiveScanState;
use crate::engines::vision_explorer::integrations::TakeoverManager;
use crate::engines::vision_explorer::message_emitter::{
    VisionExplorerMessageEmitter, WorkerTaskInfo, WorkerProgressInfo, MultiAgentModeInfo,
};
use crate::engines::vision_explorer::{register_takeover_manager, unregister_takeover_manager};
use crate::engines::vision_explorer::state::{ExplorationSummary, StateManager};
use crate::engines::vision_explorer::tools::BrowserTools;
use crate::engines::vision_explorer::types::{
    ApiEndpoint, ExplorationState, ExplorationStatus, VisionExplorerConfig,
};
use crate::services::mcp::McpService;
use crate::engines::{LlmClient, LlmConfig};
use sentinel_passive::ProxyConfig;
use anyhow::Result;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Multi-Agent Vision Explorer
///
/// Coordinates Manager-Worker exploration pattern:
/// 1. Manager analyzes homepage, divides into scopes
/// 2. Workers explore assigned scopes in parallel/sequential
/// 3. Global state aggregates discoveries
pub struct MultiAgentExplorer {
    /// Base configuration
    config: VisionExplorerConfig,
    /// Multi-agent specific config
    multi_config: MultiAgentConfig,
    /// LLM configuration
    llm_config: LlmConfig,
    /// MCP service for browser tools
    mcp_service: Arc<McpService>,
    /// Global shared state
    global_state: Arc<GlobalExplorerState>,
    /// State manager for compatibility
    state_manager: Arc<RwLock<StateManager>>,
    /// Message emitter for frontend
    message_emitter: Option<Arc<VisionExplorerMessageEmitter>>,
    /// Cancellation token
    cancellation_token: Option<CancellationToken>,
    /// Worker results
    worker_results: Arc<RwLock<Vec<WorkerResult>>>,
    /// Is running flag
    is_running: Arc<RwLock<bool>>,
    /// App handle for Tauri commands
    app_handle: Option<AppHandle>,
    /// Passive scan state for proxy management
    passive_scan_state: Option<Arc<PassiveScanState>>,
    /// Takeover manager
    takeover_manager: Option<Arc<RwLock<TakeoverManager>>>,
}

impl MultiAgentExplorer {
    /// Create new multi-agent explorer
    pub fn new(
        config: VisionExplorerConfig,
        multi_config: MultiAgentConfig,
        mcp_service: Arc<McpService>,
        llm_config: LlmConfig,
    ) -> Self {
        let global_state = Arc::new(GlobalExplorerState::new(
            &config.target_url,
            multi_config.global_ignore_patterns.clone(),
        ));

        let state_manager = Arc::new(RwLock::new(StateManager::new(
            config.target_url.clone(),
            config.max_iterations,
        )));

        Self {
            config,
            multi_config,
            llm_config,
            mcp_service,
            global_state,
            state_manager,
            message_emitter: None,
            cancellation_token: None,
            worker_results: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
            app_handle: None,
            passive_scan_state: None,
            takeover_manager: None,
        }
    }

    /// Set message emitter
    pub fn with_message_emitter(mut self, emitter: Arc<VisionExplorerMessageEmitter>) -> Self {
        self.message_emitter = Some(emitter);
        self
    }

    /// Set cancellation token
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Set app handle
    pub fn with_app_handle(mut self, app_handle: AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Set passive scan state
    pub fn with_passive_scan_state(mut self, state: Arc<PassiveScanState>) -> Self {
        self.passive_scan_state = Some(state);
        self
    }

    /// Start multi-agent exploration
    pub async fn start(&self) -> Result<ExplorationSummary> {
        // Check if already running
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(anyhow::anyhow!("Explorer is already running"));
            }
            *is_running = true;
        }

        info!(
            "MultiAgentExplorer: Starting exploration for {} (mode: {:?})",
            self.config.target_url, self.multi_config.mode
        );

        // Initialize TakeoverManager if execution_id is present
        let takeover_manager = if let Some(exec_id) = &self.config.execution_id {
            // Check if one already exists for this ID (from legacy explorer maybe?), or create new
            // Since we are the engine, we should own it.
            let tm = Arc::new(RwLock::new(TakeoverManager::new(true)));
            register_takeover_manager(exec_id.clone(), tm.clone()).await;
            Some(tm)
        } else {
            None
        };

        // Update state
        {
            let mut state = self.state_manager.write().await;
            state.update_status(ExplorationStatus::Exploring);
        }

        // Emit start message
        if let Some(emitter) = &self.message_emitter {
            emitter.emit_start(&self.config.target_url);
            // Emit multi-agent mode start
            emitter.emit_multi_agent_start(
                &format!("{:?}", self.multi_config.mode),
                0, // Will update after manager analysis
            );
        }

        // Execute exploration
        let result = self.execute_exploration(takeover_manager.clone()).await;

        // Cleanup TakeoverManager
        if let Some(exec_id) = &self.config.execution_id {
            unregister_takeover_manager(exec_id).await;
        }

        // Mark stopped
        {
            let mut is_running = self.is_running.write().await;
            *is_running = false;
        }

        // Get summary
        let summary = self.build_summary().await;

        // Emit complete message
        if let Some(emitter) = &self.message_emitter {
            let status = match &result {
                Ok(_) => "completed".to_string(),
                Err(e) => format!("failed: {}", e),
            };
            emitter.emit_complete(crate::engines::vision_explorer::message_emitter::VisionExplorationStats {
                total_iterations: summary.total_iterations,
                pages_visited: summary.pages_visited,
                apis_discovered: summary.apis_discovered,
                elements_interacted: summary.elements_interacted,
                total_duration_ms: summary.duration_seconds * 1000,
                status,
            });
        }

        match result {
            Ok(_) => Ok(summary),
            Err(e) => {
                error!("Multi-agent exploration failed: {}", e);
                Ok(summary)
            }
        }
    }

    /// Execute the exploration workflow
    async fn execute_exploration(&self, takeover_manager: Option<Arc<RwLock<TakeoverManager>>>) -> Result<()> {
        // Phase 0: Start passive proxy listener before any navigation
        if let (Some(app), Some(state)) = (&self.app_handle, &self.passive_scan_state) {
            info!("Phase 0: Starting passive proxy listener");

            let proxy_port = self
                .config
                .browser_proxy
                .as_ref()
                .and_then(|proxy_url| {
                    proxy_url
                        .split(':')
                        .last()
                        .and_then(|p| p.parse::<u16>().ok())
                })
                .unwrap_or(8080);

            let proxy_config = ProxyConfig {
                start_port: proxy_port,
                max_port_attempts: 1,
                mitm_enabled: true,
                max_request_body_size: 2 * 1024 * 1024,
                max_response_body_size: 2 * 1024 * 1024,
                mitm_bypass_fail_threshold: 3,
                upstream_proxy: None,
            };

            match crate::commands::passive_scan_commands::start_passive_scan_internal(
                app,
                state.as_ref(),
                Some(proxy_config),
            )
            .await
            {
                Ok(port) => {
                    info!("Passive proxy started on port {}", port);
                }
                Err(e) => {
                    if e.contains("already running") {
                        info!("Passive proxy already running, continuing...");
                    } else {
                        warn!(
                            "Failed to start passive proxy: {}, continuing without proxy",
                            e
                        );
                    }
                }
            }
        } else {
            warn!("AppHandle or PassiveScanState not set, skipping proxy startup");
        }

        // Phase 1: Manager analyzes and plans
        info!("Phase 1: Manager analyzing homepage navigation");
        
        let mut browser_tools = BrowserTools::new(
            self.mcp_service.clone(),
            self.config.clone(),
        );

        // Inject passive scan state to browser tools for dynamic proxy address
        if let Some(state) = &self.passive_scan_state {
            browser_tools.set_passive_scan_state(state.clone());
        }

        let browser_tools = Arc::new(browser_tools);

        let mut manager = ManagerAgent::new(
            browser_tools.clone(),
            self.llm_config.clone(),
            self.global_state.clone(),
            self.config.target_url.clone(),
            self.multi_config.clone(),
        );

        let tasks = match manager.analyze_and_plan().await {
            Ok(tasks) => tasks,
            Err(e) => {
                error!("Manager analysis failed: {}", e);
                // Fallback: create single scope task
                vec![WorkerTask {
                    id: "task-fallback".to_string(),
                    scope: ExplorationScope {
                        id: "main".to_string(),
                        name: "Main".to_string(),
                        url_patterns: vec!["/".to_string()],
                        entry_url: self.config.target_url.clone(),
                        max_depth: self.multi_config.default_max_depth,
                        priority: 1,
                    },
                    context: "Fallback single-scope exploration".to_string(),
                    max_iterations: self.config.max_iterations,
                }]
            }
        };

        info!("Manager created {} worker tasks", tasks.len());

        // Emit plan and worker tasks
        if let Some(emitter) = &self.message_emitter {
            let task_names: Vec<String> = tasks.iter().map(|t| t.scope.name.clone()).collect();
            emitter.emit_plan(
                "multi_agent",
                &format!("{} scopes to explore", tasks.len()),
                "",
                &task_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                "",
                "navigation_analysis",
            );

            // Emit worker tasks for frontend display
            let task_infos: Vec<WorkerTaskInfo> = tasks.iter().map(|t| WorkerTaskInfo {
                task_id: t.id.clone(),
                scope_name: t.scope.name.clone(),
                entry_url: t.scope.entry_url.clone(),
                url_patterns: t.scope.url_patterns.clone(),
                max_iterations: t.max_iterations,
                priority: t.scope.priority,
            }).collect();
            emitter.emit_worker_tasks(&task_infos);

            // Update multi-agent mode with actual worker count
            emitter.emit_multi_agent_start(
                &format!("{:?}", self.multi_config.mode),
                tasks.len(),
            );
        }

        // Phase 2: Execute workers based on mode
        info!("Phase 2: Executing {} worker tasks (mode: {:?})", tasks.len(), self.multi_config.mode);

        // Clone takeover_manager for each phase to avoid move issues
        let tm_for_phase2 = takeover_manager.clone();
        let tm_for_phase3 = takeover_manager.clone();
        let tm_for_replan = takeover_manager;

        let needs_replan = match self.multi_config.mode {
            ExplorationMode::Sequential => {
                self.execute_sequential(tasks, browser_tools.clone(), tm_for_phase2).await?
            }
            ExplorationMode::Parallel => {
                self.execute_parallel(tasks, tm_for_phase2).await?;
                false
            }
            ExplorationMode::Adaptive => {
                // Check if there's a login task - if so, use sequential to ensure login completes first
                let has_login_task = tasks.iter().any(|t| 
                    t.scope.id == "login" || t.scope.id == "auth" || t.scope.name.to_lowercase().contains("login")
                );
                
                if has_login_task {
                    // Force sequential mode when login is required
                    // This ensures login completes first and credentials are shared
                    info!("Adaptive mode: Using sequential execution due to login requirement");
                    self.execute_sequential(tasks, browser_tools.clone(), tm_for_phase2).await?
                } else if tasks.len() >= 2 {
                    // No login needed, can parallelize
                    self.execute_parallel(tasks, tm_for_phase2).await?;
                    false
                } else {
                    self.execute_sequential(tasks, browser_tools.clone(), tm_for_phase2).await?
                }
            }
        };

        // Phase 2.5: Re-planning after login success
        if needs_replan && !self.is_cancelled() {
            info!("Phase 2.5: Re-planning after login - analyzing post-login navigation");
            
            // Analyze current page without re-navigating (browser is already on post-login page)
            let new_tasks = match manager.analyze_current_page().await {
                Ok(tasks) => tasks,
                Err(e) => {
                    warn!("Re-planning analysis failed: {}", e);
                    vec![]
                }
            };

            if !new_tasks.is_empty() {
                info!("Re-planning created {} new worker tasks", new_tasks.len());
                
                // Emit updated plan
                if let Some(emitter) = &self.message_emitter {
                    let task_names: Vec<String> = new_tasks.iter().map(|t| t.scope.name.clone()).collect();
                    emitter.emit_plan(
                        "multi_agent",
                        &format!("{} scopes to explore (post-login)", new_tasks.len()),
                        "",
                        &task_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        "",
                        "post_login_analysis",
                    );

                    // Emit new worker tasks
                    let task_infos: Vec<WorkerTaskInfo> = new_tasks.iter().map(|t| WorkerTaskInfo {
                        task_id: t.id.clone(),
                        scope_name: t.scope.name.clone(),
                        entry_url: t.scope.entry_url.clone(),
                        url_patterns: t.scope.url_patterns.clone(),
                        max_iterations: t.max_iterations,
                        priority: t.scope.priority,
                    }).collect();
                    emitter.emit_worker_tasks(&task_infos);

                    emitter.emit_multi_agent_start(
                        &format!("{:?}", self.multi_config.mode),
                        new_tasks.len(),
                    );
                }

                // Execute new tasks (reuse browser session)
                match self.multi_config.mode {
                    ExplorationMode::Sequential => {
                        self.execute_sequential(new_tasks, browser_tools.clone(), tm_for_replan).await?;
                    }
                    ExplorationMode::Parallel => {
                        self.execute_parallel(new_tasks, tm_for_replan).await?;
                    }
                    ExplorationMode::Adaptive => {
                        if new_tasks.len() >= 2 {
                            self.execute_parallel(new_tasks, tm_for_replan).await?;
                        } else {
                            self.execute_sequential(new_tasks, browser_tools.clone(), tm_for_replan).await?;
                        }
                    }
                }
            }
        }

        // Phase 3: Handle cross-scope discoveries
        let cross_tasks = manager.handle_cross_scope_discoveries().await;
        if !cross_tasks.is_empty() {
            info!("Phase 3: Handling {} cross-scope discoveries", cross_tasks.len());
            // Execute cross-scope tasks with lower priority
            for task in cross_tasks {
                if self.is_cancelled() {
                    break;
                }
                self.execute_single_worker(task, tm_for_phase3.clone()).await?;
            }
        }

        // Phase 4: Handle dynamically discovered scopes from Workers
        // Workers may have discovered new navigation areas during exploration
        let dynamic_tasks = self.global_state.drain_pending_scope_tasks().await;
        if !dynamic_tasks.is_empty() && !self.is_cancelled() {
            info!(
                "Phase 4: Exploring {} dynamically discovered scopes",
                dynamic_tasks.len()
            );

            // Emit dynamic discovery info
            if let Some(emitter) = &self.message_emitter {
                let task_infos: Vec<WorkerTaskInfo> = dynamic_tasks.iter().map(|t| WorkerTaskInfo {
                    task_id: t.id.clone(),
                    scope_name: format!("[Dynamic] {}", t.scope.name),
                    entry_url: t.scope.entry_url.clone(),
                    url_patterns: t.scope.url_patterns.clone(),
                    max_iterations: t.max_iterations,
                    priority: t.scope.priority,
                }).collect();
                emitter.emit_worker_tasks(&task_infos);
            }

            // Execute dynamic tasks with limited iterations
            for task in dynamic_tasks {
                if self.is_cancelled() {
                    break;
                }
                
                info!(
                    "Exploring dynamic scope: {} ({})",
                    task.scope.name, task.scope.entry_url
                );
                
                self.execute_single_worker(task, None).await?;
            }
        }

        // Sync APIs to state manager
        self.sync_to_state_manager().await;

        Ok(())
    }

    /// Execute workers sequentially (reusing browser session)
    /// Returns true if re-planning is needed (e.g., after login success)
    async fn execute_sequential(&self, tasks: Vec<WorkerTask>, browser_tools: Arc<BrowserTools>, takeover_manager: Option<Arc<RwLock<TakeoverManager>>>) -> Result<bool> {
        let total_tasks = tasks.len();
        let mut needs_replan = false;

        for task in tasks {
            if self.is_cancelled() {
                info!("Exploration cancelled");
                break;
            }

            // Skip remaining tasks if re-planning is needed
            if needs_replan {
                info!("Skipping remaining tasks due to re-planning requirement");
                break;
            }

            let task_id = task.id.clone();
            let scope_name = task.scope.name.clone();
            info!("Executing worker for scope: {}", scope_name);
            
            // Emit worker progress - starting
            if let Some(emitter) = &self.message_emitter {
                let results = self.worker_results.read().await;
                emitter.emit_progress(
                    results.len() as u32,
                    total_tasks as u32,
                    &scope_name,
                    self.global_state.get_stats().await.total_urls_visited,
                    self.global_state.get_stats().await.total_apis_discovered,
                    0,
                );

                emitter.emit_worker_progress(&WorkerProgressInfo {
                    task_id: task_id.clone(),
                    scope_name: scope_name.clone(),
                    status: "running".to_string(),
                    pages_visited: 0,
                    apis_discovered: 0,
                    elements_interacted: 0,
                    iterations_used: 0,
                    progress: 0.0,
                    completion_reason: None,
                });
            }

            let mut worker = WorkerAgent::new(
                task,
                browser_tools.clone(),
                self.llm_config.clone(),
                self.global_state.clone(),
                self.config.enable_multimodal,
                takeover_manager.clone(),
            );

            // Inject message emitter for frontend communication (login takeover, etc.)
            if let Some(emitter) = &self.message_emitter {
                worker.set_message_emitter(emitter.clone());
            }

            match worker.execute().await {
                Ok(result) => {
                    info!(
                        "Worker completed scope '{}': {} pages, {} APIs (needs_replan={})",
                        result.scope_id,
                        result.stats.pages_visited,
                        result.stats.apis_discovered,
                        result.needs_replan
                    );
                    
                    // Check if re-planning is needed
                    if result.needs_replan {
                        needs_replan = true;
                        info!("Re-planning requested after login success");
                    }
                    
                    // Emit worker completion
                    if let Some(emitter) = &self.message_emitter {
                        emitter.emit_worker_complete(
                            &task_id,
                            &scope_name,
                            &serde_json::json!({
                                "pages_visited": result.stats.pages_visited,
                                "apis_discovered": result.stats.apis_discovered,
                                "elements_interacted": result.stats.elements_interacted,
                                "completion_reason": result.completion_reason,
                            }),
                        );
                    }
                    
                    // Store result
                    {
                        let mut results = self.worker_results.write().await;
                        results.push(result);
                    } // Release write lock before acquiring read lock

                    // Emit global stats update
                    if let Some(emitter) = &self.message_emitter {
                        let results_count = self.worker_results.read().await.len();
                        let stats = self.global_state.get_stats().await;
                        emitter.emit_multi_agent_stats(
                            &MultiAgentModeInfo {
                                is_multi_agent: true,
                                mode: format!("{:?}", self.multi_config.mode),
                                total_workers: total_tasks,
                                completed_workers: results_count,
                            },
                            &serde_json::json!({
                                "total_urls_visited": stats.total_urls_visited,
                                "total_apis_discovered": stats.total_apis_discovered,
                                "workers_completed": results_count,
                                "total_elements_interacted": stats.total_elements_interacted,
                            }),
                        );
                    }
                }
                Err(e) => {
                    warn!("Worker failed for scope {}: {}", scope_name, e);
                    // Emit worker failure
                    if let Some(emitter) = &self.message_emitter {
                        emitter.emit_worker_progress(&WorkerProgressInfo {
                            task_id: task_id.clone(),
                            scope_name: scope_name.clone(),
                            status: "failed".to_string(),
                            pages_visited: 0,
                            apis_discovered: 0,
                            elements_interacted: 0,
                            iterations_used: 0,
                            progress: 0.0,
                            completion_reason: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        // Don't close browser if re-planning is needed
        if !needs_replan {
            if let Err(e) = browser_tools.close_browser().await {
                warn!("Failed to close browser: {}", e);
            }
        }

        Ok(needs_replan)
    }

    /// Execute workers in parallel (separate browser contexts)
    async fn execute_parallel(&self, tasks: Vec<WorkerTask>, takeover_manager: Option<Arc<RwLock<TakeoverManager>>>) -> Result<()> {
        use futures::stream::{self, StreamExt};

        let max_concurrent = self.multi_config.max_concurrent_workers;
        let emitter = self.message_emitter.clone();
        
        let results: Vec<_> = stream::iter(tasks)
            .map(|task| {
                let mcp_service = self.mcp_service.clone();
                let config = self.config.clone();
                let llm_config = self.llm_config.clone();
                let global_state = self.global_state.clone();
                let enable_multimodal = self.config.enable_multimodal;
                let tm = takeover_manager.clone();
                let emitter = emitter.clone();
                
                async move {
                    // Create separate browser tools for this worker
                    let browser_tools = Arc::new(BrowserTools::new(mcp_service, config));
                    
                    let mut worker = WorkerAgent::new(
                        task,
                        browser_tools.clone(),
                        llm_config,
                        global_state,
                        enable_multimodal,
                        tm,
                    );

                    // Inject message emitter for frontend communication
                    if let Some(e) = emitter {
                        worker.set_message_emitter(e);
                    }

                    let result = worker.execute().await;
                    
                    // Close browser
                    let _ = browser_tools.close_browser().await;
                    
                    result
                }
            })
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        // Store results
        let mut stored = self.worker_results.write().await;
        for result in results {
            match result {
                Ok(r) => {
                    info!(
                        "Parallel worker completed scope '{}': {} pages, {} APIs",
                        r.scope_id, r.stats.pages_visited, r.stats.apis_discovered
                    );
                    stored.push(r);
                }
                Err(e) => {
                    warn!("Parallel worker failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Execute a single worker task
    async fn execute_single_worker(&self, task: WorkerTask, takeover_manager: Option<Arc<RwLock<TakeoverManager>>>) -> Result<()> {
        let browser_tools = Arc::new(BrowserTools::new(
            self.mcp_service.clone(),
            self.config.clone(),
        ));

        let mut worker = WorkerAgent::new(
            task,
            browser_tools.clone(),
            self.llm_config.clone(),
            self.global_state.clone(),
            self.config.enable_multimodal,
            takeover_manager,
        );

        // Inject message emitter for frontend communication
        if let Some(emitter) = &self.message_emitter {
            worker.set_message_emitter(emitter.clone());
        }

        match worker.execute().await {
            Ok(result) => {
                let mut results = self.worker_results.write().await;
                results.push(result);
            }
            Err(e) => {
                warn!("Single worker failed: {}", e);
            }
        }

        let _ = browser_tools.close_browser().await;
        Ok(())
    }

    /// Check if exploration is cancelled
    fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .as_ref()
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    /// Sync discoveries to state manager (for compatibility)
    async fn sync_to_state_manager(&self) {
        let apis = self.global_state.get_all_apis().await;
        let stats = self.global_state.get_stats().await;

        let mut state = self.state_manager.write().await;
        
        // Add APIs
        for api in apis {
            state.add_discovered_api(ApiEndpoint {
                method: api.method,
                path: api.path,
                full_url: api.full_url,
                headers: Default::default(),
                parameters: api.parameters,
                body: None,
                status_code: api.status_code,
                discovered_at: chrono::Utc::now(),
                source_action_id: None,
            });
        }

        // Mark as completed
        state.mark_completed(&format!(
            "Multi-agent exploration complete: {} pages, {} APIs",
            stats.total_urls_visited, stats.total_apis_discovered
        ));
    }

    /// Build exploration summary
    async fn build_summary(&self) -> ExplorationSummary {
        let stats = self.global_state.get_stats().await;
        let results = self.worker_results.read().await;
        
        let total_iterations: u32 = results.iter().map(|r| r.stats.iterations_used).sum();
        let total_elements: usize = results.iter().map(|r| r.stats.elements_interacted).sum();
        let total_duration: u64 = results.iter().map(|r| r.stats.duration_ms).sum();

        ExplorationSummary {
            session_id: uuid::Uuid::new_v4().to_string(),
            target_url: self.config.target_url.clone(),
            status: ExplorationStatus::Completed,
            total_iterations,
            pages_visited: stats.total_urls_visited,
            elements_interacted: total_elements,
            apis_discovered: stats.total_apis_discovered,
            forms_discovered: 0,
            exploration_progress: 100.0,
            duration_seconds: total_duration / 1000,
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> ExplorationState {
        let state = self.state_manager.read().await;
        state.state().clone()
    }

    /// Get global stats
    pub async fn get_global_stats(&self) -> super::global_state::GlobalStats {
        self.global_state.get_stats().await
    }

    /// Stop exploration
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("MultiAgentExplorer stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_agent_config_default() {
        let config = MultiAgentConfig::default();
        assert_eq!(config.mode, ExplorationMode::Sequential);
        assert_eq!(config.max_concurrent_workers, 3);
    }
}

