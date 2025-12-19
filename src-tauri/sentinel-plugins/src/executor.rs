use crate::error::{PluginError, Result};
use crate::plugin_engine::PluginEngine;
use crate::types::{Finding, HttpTransaction, PluginMetadata};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// 插件命令
enum PluginCommand {
    /// 扫描事务
    ScanTransaction(HttpTransaction, oneshot::Sender<Result<Vec<Finding>>>),
}

/// 插件执行器
///
/// 维护一个长期运行的专用线程和 V8 Runtime，避免重复创建开销。
pub struct PluginExecutor {
    sender: mpsc::Sender<PluginCommand>,
    #[allow(dead_code)]
    plugin_id: String,
}

impl PluginExecutor {
    /// 创建新的插件执行器
    ///
    /// 启动一个后台线程，初始化 PluginEngine 并等待命令。
    pub fn new(metadata: PluginMetadata, code: String) -> Result<Self> {
        let plugin_id = metadata.id.clone();
        let (tx, mut rx) = mpsc::channel::<PluginCommand>(100);

        let plugin_id_clone = plugin_id.clone();

        // 启动专用线程
        std::thread::Builder::new()
            .name(format!("plugin-executor-{}", plugin_id))
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!(
                            "Failed to build runtime for plugin {}: {}",
                            plugin_id_clone, e
                        );
                        return;
                    }
                };

                rt.block_on(async move {
                    // 初始化引擎
                    let mut engine = match PluginEngine::new() {
                        Ok(e) => e,
                        Err(e) => {
                            error!(
                                "Failed to create engine for plugin {}: {}",
                                plugin_id_clone, e
                            );
                            return;
                        }
                    };

                    // 加载代码
                    if let Err(e) = engine.load_plugin_with_metadata(&code, metadata).await {
                        error!("Failed to load code for plugin {}: {}", plugin_id_clone, e);
                        return;
                    }

                    debug!("Plugin executor started for {}", plugin_id_clone);

                    // 循环处理命令
                    while let Some(cmd) = rx.recv().await {
                        match cmd {
                            PluginCommand::ScanTransaction(txn, reply) => {
                                let res = engine.scan_transaction(&txn).await;
                                let _ = reply.send(res);
                            }
                        }
                    }

                    debug!("Plugin executor stopped for {}", plugin_id_clone);
                });
            })
            .map_err(|e| PluginError::Execution(format!("Failed to spawn thread: {}", e)))?;

        Ok(Self {
            sender: tx,
            plugin_id,
        })
    }

    /// 执行扫描事务
    pub async fn scan_transaction(&self, transaction: HttpTransaction) -> Result<Vec<Finding>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(PluginCommand::ScanTransaction(transaction, reply_tx))
            .await
            .map_err(|e| PluginError::Execution(format!("Executor channel closed: {}", e)))?;

        reply_rx
            .await
            .map_err(|e| PluginError::Execution(format!("Failed to receive reply: {}", e)))?
    }
}
