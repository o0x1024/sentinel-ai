//! Engine Pool - lightweight engine reuse without per-plugin threads
//!
//! Replaces the Deno-era pattern of one OS thread per plugin with a
//! fixed pool of worker threads. Engines are created and kept on
//! their worker thread (never moved across threads). Work is dispatched
//! via channels.

use crate::error::{PluginError, Result};
use crate::plugin_engine::PluginEngine;
use crate::types::{Finding, HttpTransaction, PluginMetadata};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

/// Configuration for the engine pool.
#[derive(Debug, Clone)]
pub struct EnginePoolConfig {
    pub worker_count: usize,
    pub max_engines_per_worker: usize,
    pub max_executions_per_engine: u64,
    pub idle_evict_secs: u64,
}

impl Default for EnginePoolConfig {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            worker_count: (cpus / 2).max(2),
            max_engines_per_worker: 16,
            max_executions_per_engine: 10_000,
            idle_evict_secs: 300,
        }
    }
}

enum PoolCommand {
    Scan {
        plugin_id: String,
        metadata: PluginMetadata,
        code: String,
        transaction: Box<HttpTransaction>,
        finding_sink: Option<mpsc::UnboundedSender<Finding>>,
        reply: oneshot::Sender<Result<Vec<Finding>>>,
    },
    Agent {
        plugin_id: String,
        metadata: PluginMetadata,
        code: String,
        input: serde_json::Value,
        run_id: Option<String>,
        execution_context: Option<String>,
        reply: oneshot::Sender<Result<(Vec<Finding>, Option<serde_json::Value>)>>,
    },
    Evict {
        plugin_id: String,
    },
    Stats {
        reply: oneshot::Sender<WorkerStats>,
    },
    Shutdown,
}

struct CachedEngine {
    engine: PluginEngine,
    plugin_id: String,
    executions: u64,
    last_used: Instant,
}

/// Pool of PluginEngines distributed across fixed worker threads.
pub struct EnginePool {
    senders: Vec<mpsc::Sender<PoolCommand>>,
    config: EnginePoolConfig,
}

impl EnginePool {
    pub fn new(config: EnginePoolConfig) -> Self {
        let mut senders = Vec::with_capacity(config.worker_count);

        for worker_id in 0..config.worker_count {
            let (tx, rx) = mpsc::channel::<PoolCommand>(256);
            let worker_config = config.clone();

            std::thread::Builder::new()
                .name(format!("engine-pool-worker-{worker_id}"))
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build()
                        .expect("Failed to build tokio runtime for engine pool worker");

                    rt.block_on(worker_loop(rx, worker_config));
                })
                .expect("Failed to spawn engine pool worker thread");

            senders.push(tx);
        }

        Self { senders, config }
    }

    pub fn with_defaults() -> Self {
        Self::new(EnginePoolConfig::default())
    }

    /// Execute a traffic scan plugin.
    pub async fn execute_scan(
        &self,
        plugin_id: &str,
        metadata: &PluginMetadata,
        code: &str,
        transaction: &HttpTransaction,
        finding_sink: Option<mpsc::UnboundedSender<Finding>>,
    ) -> Result<Vec<Finding>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let sender = self.pick_worker(plugin_id);

        sender
            .send(PoolCommand::Scan {
                plugin_id: plugin_id.to_string(),
                metadata: metadata.clone(),
                code: code.to_string(),
                transaction: Box::new(transaction.clone()),
                finding_sink,
                reply: reply_tx,
            })
            .await
            .map_err(|_| PluginError::Execution("Engine pool worker channel closed".to_string()))?;

        reply_rx
            .await
            .map_err(|_| PluginError::Execution("Engine pool reply dropped".to_string()))?
    }

    /// Execute an agent-style plugin call.
    pub async fn execute_agent(
        &self,
        plugin_id: &str,
        metadata: &PluginMetadata,
        code: &str,
        input: serde_json::Value,
        run_id: Option<String>,
        execution_context: Option<String>,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let sender = self.pick_worker(plugin_id);

        sender
            .send(PoolCommand::Agent {
                plugin_id: plugin_id.to_string(),
                metadata: metadata.clone(),
                code: code.to_string(),
                input,
                run_id,
                execution_context,
                reply: reply_tx,
            })
            .await
            .map_err(|_| PluginError::Execution("Engine pool worker channel closed".to_string()))?;

        reply_rx
            .await
            .map_err(|_| PluginError::Execution("Engine pool reply dropped".to_string()))?
    }

    /// Evict all cached engines for a plugin across all workers.
    pub async fn evict(&self, plugin_id: &str) {
        for sender in &self.senders {
            let _ = sender
                .send(PoolCommand::Evict {
                    plugin_id: plugin_id.to_string(),
                })
                .await;
        }
    }

    /// Get pool statistics.
    pub async fn stats(&self) -> EnginePoolStats {
        let mut total_cached = 0;
        let mut total_plugins = std::collections::HashSet::new();

        for sender in &self.senders {
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender.send(PoolCommand::Stats { reply: reply_tx }).await.is_ok() {
                if let Ok(worker_stats) = reply_rx.await {
                    total_cached += worker_stats.cached_engines;
                    total_plugins.extend(worker_stats.plugin_ids);
                }
            }
        }

        EnginePoolStats {
            total_cached,
            plugins_cached: total_plugins.len(),
            worker_count: self.config.worker_count,
        }
    }

    /// Shutdown all workers.
    pub async fn shutdown(&self) {
        for sender in &self.senders {
            let _ = sender.send(PoolCommand::Shutdown).await;
        }
    }

    /// Pick a worker for a given plugin_id (consistent hashing for cache affinity).
    fn pick_worker(&self, plugin_id: &str) -> &mpsc::Sender<PoolCommand> {
        let hash = fxhash(plugin_id);
        let idx = hash % self.senders.len();
        &self.senders[idx]
    }
}

fn fxhash(s: &str) -> usize {
    let mut hash: usize = 0;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(0x100000001b3).wrapping_add(byte as usize);
    }
    hash
}

struct WorkerStats {
    cached_engines: usize,
    plugin_ids: Vec<String>,
}

async fn worker_loop(mut rx: mpsc::Receiver<PoolCommand>, config: EnginePoolConfig) {
    let mut cache: HashMap<String, Vec<CachedEngine>> = HashMap::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            PoolCommand::Scan {
                plugin_id,
                metadata,
                code,
                transaction,
                finding_sink,
                reply,
            } => {
                let engine = acquire_engine(&mut cache, &plugin_id, &metadata, &code, &config).await;
                let result = match engine {
                    Ok(mut cached) => {
                        let r = cached
                            .engine
                            .scan_transaction_with_sink(&transaction, finding_sink)
                            .await;
                        return_engine(&mut cache, cached, &config);
                        r
                    }
                    Err(e) => Err(e),
                };
                let _ = reply.send(result);
            }
            PoolCommand::Agent {
                plugin_id,
                metadata,
                code,
                input,
                run_id,
                execution_context,
                reply,
            } => {
                let engine = acquire_engine(&mut cache, &plugin_id, &metadata, &code, &config).await;
                let result = match engine {
                    Ok(mut cached) => {
                        let r = cached.engine.execute_agent_with_runtime_context(
                            &input,
                            execution_context,
                            run_id,
                        ).await;
                        return_engine(&mut cache, cached, &config);
                        r
                    }
                    Err(e) => Err(e),
                };
                let _ = reply.send(result);
            }
            PoolCommand::Evict { plugin_id } => {
                if let Some(removed) = cache.remove(&plugin_id) {
                    info!(
                        "Evicted {} cached engines for plugin {plugin_id}",
                        removed.len()
                    );
                }
            }
            PoolCommand::Stats { reply } => {
                let cached_engines: usize = cache.values().map(|v| v.len()).sum();
                let plugin_ids: Vec<String> = cache.keys().cloned().collect();
                let _ = reply.send(WorkerStats {
                    cached_engines,
                    plugin_ids,
                });
            }
            PoolCommand::Shutdown => break,
        }
    }
}

async fn acquire_engine(
    cache: &mut HashMap<String, Vec<CachedEngine>>,
    plugin_id: &str,
    metadata: &PluginMetadata,
    code: &str,
    config: &EnginePoolConfig,
) -> Result<CachedEngine> {
    if let Some(engines) = cache.get_mut(plugin_id) {
        if let Some(mut cached) = engines.pop() {
            if cached.executions < config.max_executions_per_engine {
                cached.last_used = Instant::now();
                cached.executions += 1;
                debug!("Reusing cached engine for plugin {plugin_id}");
                return Ok(cached);
            }
            debug!(
                "Engine for {plugin_id} exceeded max executions ({}), creating fresh",
                cached.executions
            );
        }
    }

    debug!("Creating new engine for plugin {plugin_id}");
    let mut engine = PluginEngine::new()
        .map_err(|e| PluginError::Load(format!("Engine creation failed: {e}")))?;
    engine
        .load_plugin_with_metadata(code, metadata.clone())
        .await?;

    Ok(CachedEngine {
        engine,
        plugin_id: plugin_id.to_string(),
        executions: 1,
        last_used: Instant::now(),
    })
}

fn return_engine(
    cache: &mut HashMap<String, Vec<CachedEngine>>,
    cached: CachedEngine,
    config: &EnginePoolConfig,
) {
    let entries = cache.entry(cached.plugin_id.clone()).or_default();
    if entries.len() < config.max_engines_per_worker {
        entries.push(cached);
    }
}

#[derive(Debug, Clone)]
pub struct EnginePoolStats {
    pub total_cached: usize,
    pub plugins_cached: usize,
    pub worker_count: usize,
}
