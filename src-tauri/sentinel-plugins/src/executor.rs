/// Plugin Executor with restart capability
///
/// Wraps PluginEngine in a dedicated thread and provides restart functionality.
/// Each restart creates a new thread with a fresh One Engine instance.
use crate::error::{PluginError, Result};
use crate::plugin_engine::PluginEngine;
use crate::types::{Finding, HttpTransaction, PluginMetadata};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, info};

/// Commands sent to the plugin executor thread
enum PluginCommand {
    ScanTransaction(
        Box<HttpTransaction>,
        Option<mpsc::UnboundedSender<Finding>>,
        oneshot::Sender<Result<Vec<Finding>>>,
    ),
    ExecuteAgent(
        serde_json::Value,
        Option<String>,
        Option<String>,
        oneshot::Sender<Result<(Vec<Finding>, Option<serde_json::Value>)>>,
    ),
    Restart(oneshot::Sender<Result<()>>),
    GetStats(oneshot::Sender<ExecutorStats>),
    Shutdown,
}

/// Statistics about executor execution
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    pub total_executions: usize,
    pub current_instance_executions: usize,
    pub restart_count: usize,
    pub last_restart_time: Option<std::time::Instant>,
}

/// Plugin executor with restart capability
pub struct PluginExecutor {
    /// Current worker thread handle
    worker_thread: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Channel to send commands to worker
    sender: Arc<RwLock<mpsc::Sender<PluginCommand>>>,
    /// Signal to terminate the currently running execution (best-effort)
    should_terminate: Arc<AtomicBool>,
    /// Plugin ID
    plugin_id: String,
    /// Plugin metadata
    metadata: PluginMetadata,
    /// Plugin code
    code: String,
    /// Maximum executions before restart warning
    max_executions_before_restart: usize,
    /// Total execution count across all instances
    total_executions: Arc<AtomicUsize>,
    /// Current instance execution count
    current_instance_executions: Arc<AtomicUsize>,
    /// Restart count
    restart_count: Arc<AtomicUsize>,
    /// Last restart time
    last_restart_time: Arc<RwLock<Option<std::time::Instant>>>,
    /// Shutdown signal
    should_shutdown: Arc<AtomicBool>,
}

impl PluginExecutor {
    /// Create a new executor with restart capability
    ///
    /// # Arguments
    /// - `metadata`: Plugin metadata
    /// - `code`: Plugin code
    /// - `max_executions_before_restart`: Warn after this many executions (recommended: 1000-10000)
    pub fn new(
        metadata: PluginMetadata,
        code: String,
        max_executions_before_restart: usize,
    ) -> Result<Self> {
        let plugin_id = metadata.id.clone();

        let total_executions = Arc::new(AtomicUsize::new(0));
        let current_instance_executions = Arc::new(AtomicUsize::new(0));
        let restart_count = Arc::new(AtomicUsize::new(0));
        let last_restart_time = Arc::new(RwLock::new(Some(std::time::Instant::now())));
        let should_shutdown = Arc::new(AtomicBool::new(false));
        let should_terminate = Arc::new(AtomicBool::new(false));

        let (tx, worker_thread) = Self::spawn_worker(
            &metadata,
            &code,
            max_executions_before_restart,
            total_executions.clone(),
            current_instance_executions.clone(),
            restart_count.clone(),
            last_restart_time.clone(),
            should_shutdown.clone(),
        )?;

        Ok(Self {
            worker_thread: Arc::new(RwLock::new(Some(worker_thread))),
            sender: Arc::new(RwLock::new(tx)),
            should_terminate,
            plugin_id,
            metadata,
            code,
            max_executions_before_restart,
            total_executions,
            current_instance_executions,
            restart_count,
            last_restart_time,
            should_shutdown,
        })
    }

    /// Create default executor (warn every 1000 executions)
    pub fn new_default(metadata: PluginMetadata, code: String) -> Result<Self> {
        Self::new(metadata, code, 1000)
    }

    /// Spawn a new worker thread
    #[allow(clippy::too_many_arguments)]
    fn spawn_worker(
        metadata: &PluginMetadata,
        code: &str,
        max_executions_before_restart: usize,
        total_executions: Arc<AtomicUsize>,
        current_instance_executions: Arc<AtomicUsize>,
        restart_count: Arc<AtomicUsize>,
        last_restart_time: Arc<RwLock<Option<std::time::Instant>>>,
        should_shutdown: Arc<AtomicBool>,
    ) -> Result<(mpsc::Sender<PluginCommand>, JoinHandle<()>)> {
        let (tx, mut rx) = mpsc::channel::<PluginCommand>(100);
        let (init_tx, init_rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();

        let metadata_clone = metadata.clone();
        let code_clone = code.to_string();
        let plugin_id_clone = metadata.id.clone();

            let handle = std::thread::Builder::new()
            .name(format!("plugin-executor-{}", metadata.id))
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let msg = format!(
                            "Failed to build runtime for plugin {}: {}",
                            plugin_id_clone, e
                        );
                        tracing::error!("{}", msg);
                        let _ = init_tx.send(Err(msg));
                        return;
                    }
                };

                rt.block_on(async move {
                    let mut engine = match Self::create_engine(
                        &code_clone,
                        &metadata_clone,
                        &plugin_id_clone,
                    )
                    .await
                    {
                        Ok(e) => e,
                        Err(e) => {
                            let msg = format!(
                                "Failed to create engine for plugin {}: {}",
                                plugin_id_clone, e
                            );
                            tracing::error!("{}", msg);
                            let _ = init_tx.send(Err(msg));
                            return;
                        }
                    };

                    let _ = init_tx.send(Ok(()));
                    info!("Plugin executor started for {}", plugin_id_clone);

                    // Command processing loop
                    while let Some(cmd) = rx.recv().await {
                        // Check shutdown signal
                        if should_shutdown.load(Ordering::Relaxed) {
                            info!("Shutdown signal received for plugin {}", plugin_id_clone);
                            break;
                        }

                        match cmd {
                            PluginCommand::ScanTransaction(boxed_txn, finding_sink, reply) => {
                                let txn = *boxed_txn;
                                // Check if restart threshold reached (for monitoring only)
                                let current_count =
                                    current_instance_executions.load(Ordering::Relaxed);
                                if current_count >= max_executions_before_restart {
                                    debug!(
                                        "Plugin {} execution count: {} (threshold: {}). Auto-restart should be triggered externally.",
                                        plugin_id_clone, current_count, max_executions_before_restart
                                    );
                                }

                                // Execute scan
                                let res = engine.scan_transaction_with_sink(&txn, finding_sink).await;
                                current_instance_executions.fetch_add(1, Ordering::Relaxed);
                                total_executions.fetch_add(1, Ordering::Relaxed);
                                let _ = reply.send(res);
                            }

                            PluginCommand::ExecuteAgent(input, execution_context, run_id, reply) => {
                                // Check if restart threshold reached (for monitoring only)
                                let current_count =
                                    current_instance_executions.load(Ordering::Relaxed);
                                if current_count >= max_executions_before_restart {
                                    debug!(
                                        "Plugin {} execution count: {} (threshold: {}). Auto-restart should be triggered externally.",
                                        plugin_id_clone, current_count, max_executions_before_restart
                                    );
                                }

                                // Execute agent
                                let res = engine
                                    .execute_agent_with_runtime_context(
                                        &input,
                                        execution_context,
                                        run_id,
                                    )
                                    .await;
                                current_instance_executions.fetch_add(1, Ordering::Relaxed);
                                total_executions.fetch_add(1, Ordering::Relaxed);
                                let _ = reply.send(res);
                            }

                            PluginCommand::Restart(_reply) => {
                                info!("Restart requested for plugin {}", plugin_id_clone);
                                // Signal shutdown - the main thread will create a new worker
                                should_shutdown.store(true, Ordering::Relaxed);
                                break;
                            }

                            PluginCommand::GetStats(reply) => {
                                let stats = ExecutorStats {
                                    total_executions: total_executions.load(Ordering::Relaxed),
                                    current_instance_executions: current_instance_executions
                                        .load(Ordering::Relaxed),
                                    restart_count: restart_count.load(Ordering::Relaxed),
                                    last_restart_time: *last_restart_time.read().await,
                                };
                                let _ = reply.send(stats);
                            }

                            PluginCommand::Shutdown => {
                                info!("Shutdown command received for plugin {}", plugin_id_clone);
                                break;
                            }
                        }
                    }

                    debug!("Plugin executor stopped for {}", plugin_id_clone);
                });
            })
            .map_err(|e| PluginError::Execution(format!("Failed to spawn thread: {}", e)))?;

        match init_rx.recv() {
            Ok(Ok(())) => Ok((tx, handle)),
            Ok(Err(msg)) => Err(PluginError::Load(msg)),
            Err(_) => Err(PluginError::Load(
                "Worker thread exited before engine initialization completed".to_string(),
            )),
        }
    }

    /// Create engine instance
    async fn create_engine(
        code: &str,
        metadata: &PluginMetadata,
        plugin_id: &str,
    ) -> Result<PluginEngine> {
        let mut engine = PluginEngine::new()?;
        engine
            .load_plugin_with_metadata(code, metadata.clone())
            .await?;
        debug!("Engine created for plugin {}", plugin_id);
        Ok(engine)
    }

    /// Execute scan transaction
    pub async fn scan_transaction(&self, transaction: HttpTransaction) -> Result<Vec<Finding>> {
        self.scan_transaction_with_sink(transaction, None).await
    }

    pub async fn scan_transaction_with_sink(
        &self,
        transaction: HttpTransaction,
        finding_sink: Option<mpsc::UnboundedSender<Finding>>,
    ) -> Result<Vec<Finding>> {
        let sender = self.sender.read().await;
        let (reply_tx, reply_rx) = oneshot::channel();

        sender
            .send(PluginCommand::ScanTransaction(
                Box::new(transaction),
                finding_sink,
                reply_tx,
            ))
            .await
            .map_err(|e| PluginError::Execution(format!("Executor channel closed: {}", e)))?;

        reply_rx
            .await
            .map_err(|e| PluginError::Execution(format!("Failed to receive reply: {}", e)))?
    }

    /// Execute agent plugin (analyze/run/execute)
    pub async fn execute_agent(
        &self,
        input: &serde_json::Value,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        self.execute_agent_with_runtime_context(input, None, None)
            .await
    }

    pub async fn execute_agent_with_runtime_context(
        &self,
        input: &serde_json::Value,
        execution_context: Option<String>,
        run_id: Option<String>,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        let sender = self.sender.read().await;
        let (reply_tx, reply_rx) = oneshot::channel();

        sender
            .send(PluginCommand::ExecuteAgent(
                input.clone(),
                execution_context,
                run_id,
                reply_tx,
            ))
            .await
            .map_err(|e| PluginError::Execution(format!("Executor channel closed: {}", e)))?;

        reply_rx
            .await
            .map_err(|e| PluginError::Execution(format!("Failed to receive reply: {}", e)))?
    }

    /// Request the engine to terminate the currently running script (best-effort).
    ///
    /// Sets a flag that can be checked by fuel exhaustion or cooperative yields.
    /// For synchronous infinite loops without await points, the engine's fuel
    /// limit provides the actual termination guarantee.
    pub async fn terminate_execution(&self) {
        self.should_terminate.store(true, Ordering::Relaxed);
    }

    /// Manually trigger restart
    ///
    /// This will:
    /// 1. Signal the old thread to shutdown
    /// 2. Wait for it to exit
    /// 3. Spawn a new thread with a fresh engine instance
    pub async fn restart(&self) -> Result<()> {
        info!("Manual restart triggered for plugin {}", self.plugin_id);

        // Signal shutdown to old thread
        self.should_shutdown.store(true, Ordering::Relaxed);

        // Send shutdown command
        {
            let sender = self.sender.read().await;
            let (reply_tx, _reply_rx) = oneshot::channel();
            let _ = sender.send(PluginCommand::Restart(reply_tx)).await;
        }

        // Wait for old thread to exit
        {
            let mut worker_thread = self.worker_thread.write().await;
            if let Some(handle) = worker_thread.take() {
                info!(
                    "Waiting for old worker thread to exit for plugin {}",
                    self.plugin_id
                );
                // Give it some time to exit gracefully
                tokio::task::spawn_blocking(move || {
                    handle.join().ok();
                })
                .await
                .map_err(|e| PluginError::Execution(format!("Failed to join thread: {}", e)))?;
            }
        }

        info!("Old worker thread exited for plugin {}", self.plugin_id);

        // Reset shutdown flag
        self.should_shutdown.store(false, Ordering::Relaxed);
        self.should_terminate.store(false, Ordering::Relaxed);

        // Reset current instance execution count
        self.current_instance_executions.store(0, Ordering::Relaxed);

        // Spawn new worker thread
        let (new_tx, new_handle) = Self::spawn_worker(
            &self.metadata,
            &self.code,
            self.max_executions_before_restart,
            self.total_executions.clone(),
            self.current_instance_executions.clone(),
            self.restart_count.clone(),
            self.last_restart_time.clone(),
            self.should_shutdown.clone(),
        )?;

        // Update sender and worker thread
        {
            let mut sender = self.sender.write().await;
            *sender = new_tx;
        }
        {
            let mut worker_thread = self.worker_thread.write().await;
            *worker_thread = Some(new_handle);
        }

        // Update stats
        self.restart_count.fetch_add(1, Ordering::Relaxed);
        *self.last_restart_time.write().await = Some(std::time::Instant::now());

        info!(
            "Plugin {} restarted successfully (restart #{})",
            self.plugin_id,
            self.restart_count.load(Ordering::Relaxed)
        );

        Ok(())
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> Result<ExecutorStats> {
        let sender = self.sender.read().await;
        let (reply_tx, reply_rx) = oneshot::channel();

        sender
            .send(PluginCommand::GetStats(reply_tx))
            .await
            .map_err(|e| PluginError::Execution(format!("Executor channel closed: {}", e)))?;

        reply_rx
            .await
            .map_err(|e| PluginError::Execution(format!("Failed to receive stats: {}", e)))
    }

    /// Get plugin ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Get restart threshold
    pub fn max_executions_before_restart(&self) -> usize {
        self.max_executions_before_restart
    }

    /// Shutdown executor
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down executor for plugin {}", self.plugin_id);

        self.should_shutdown.store(true, Ordering::Relaxed);

        {
            let sender = self.sender.read().await;
            let _ = sender.send(PluginCommand::Shutdown).await;
        }

        // Wait for thread to exit
        {
            let mut worker_thread = self.worker_thread.write().await;
            if let Some(handle) = worker_thread.take() {
                tokio::task::spawn_blocking(move || {
                    handle.join().ok();
                })
                .await
                .map_err(|e| PluginError::Execution(format!("Failed to join thread: {}", e)))?;
            }
        }

        Ok(())
    }
}

impl Drop for PluginExecutor {
    fn drop(&mut self) {
        // Signal shutdown
        self.should_shutdown.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PluginCategory, PluginMainCategory, Severity};

    fn create_test_metadata() -> PluginMetadata {
        PluginMetadata {
            id: "test-executor".to_string(),
            name: "Test Executor Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: PluginMainCategory::Traffic,
            category: PluginCategory::parse_for_main_category(PluginMainCategory::Traffic, "test")
                .expect("test category should parse"),
            default_severity: Severity::Info,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        }
    }

    fn create_test_code() -> String {
        r#"
function scan_transaction(transaction) {
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Test",
        description: "Test",
        evidence: "test",
        location: "test",
        severity: "info",
        confidence: "high"
    });
}
"#
        .to_string()
    }

    fn create_streaming_test_code() -> String {
        r#"
function scan_transaction(transaction) {
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Streamed Finding",
        description: "Streamed before scan completion",
        evidence: "streamed",
        param_name: "id",
        severity: "info",
        confidence: "high"
    });
    return [];
}
"#
        .to_string()
    }

    fn create_test_transaction() -> HttpTransaction {
        use chrono::Utc;
        use std::collections::HashMap;

        HttpTransaction {
            request: crate::types::RequestContext {
                id: uuid::Uuid::new_v4().to_string(),
                method: "GET".to_string(),
                url: "https://example.com/test".to_string(),
                http_version: Some("HTTP/1.1".to_string()),
                headers: HashMap::new(),
                body: vec![],
                content_type: None,
                query_params: HashMap::new(),
                is_https: true,
                timestamp: Utc::now(),
                was_edited: false,
                edited_method: None,
                edited_url: None,
                edited_headers: None,
                edited_body: None,
            },
            response: None,
        }
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let metadata = create_test_metadata();
        let code = create_test_code();

        let executor = PluginExecutor::new(metadata, code, 100).unwrap();
        assert_eq!(executor.plugin_id(), "test-executor");
        assert_eq!(executor.max_executions_before_restart(), 100);

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_basic_execution() {
        let metadata = create_test_metadata();
        let code = create_test_code();
        let executor = PluginExecutor::new(metadata, code, 1000).unwrap();

        let txn = create_test_transaction();
        let findings = executor.scan_transaction(txn).await.unwrap();
        assert!(!findings.is_empty());

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_restart() {
        let metadata = create_test_metadata();
        let code = create_test_code();
        let executor = PluginExecutor::new(metadata, code, 10).unwrap();

        // Execute some tasks
        for _ in 0..5 {
            let txn = create_test_transaction();
            executor.scan_transaction(txn).await.unwrap();
        }

        let stats_before = executor.get_stats().await.unwrap();
        assert_eq!(stats_before.current_instance_executions, 5);
        assert_eq!(stats_before.restart_count, 0);

        // Restart
        executor.restart().await.unwrap();

        // Execute more tasks
        for _ in 0..3 {
            let txn = create_test_transaction();
            executor.scan_transaction(txn).await.unwrap();
        }

        let stats_after = executor.get_stats().await.unwrap();
        assert_eq!(stats_after.total_executions, 8);
        assert_eq!(stats_after.current_instance_executions, 3);
        assert_eq!(stats_after.restart_count, 1);

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_transaction_streams_findings_before_completion() {
        let metadata = create_test_metadata();
        let code = create_streaming_test_code();
        let executor = PluginExecutor::new(metadata, code, 1000).unwrap();
        let txn = create_test_transaction();
        let (finding_tx, mut finding_rx) = mpsc::unbounded_channel();

        let scan_task = tokio::spawn(async move {
            let findings = executor
                .scan_transaction_with_sink(txn, Some(finding_tx))
                .await
                .expect("scan ok");
            executor.shutdown().await.expect("shutdown ok");
            findings
        });

        let streamed =
            tokio::time::timeout(std::time::Duration::from_millis(150), finding_rx.recv())
                .await
                .expect("finding should arrive before scan completes")
                .expect("streamed finding");
        assert_eq!(streamed.title, "Streamed Finding");

        let findings = scan_task.await.expect("join ok");
        assert!(
            findings.is_empty(),
            "streamed findings should not be returned again"
        );
    }
}
