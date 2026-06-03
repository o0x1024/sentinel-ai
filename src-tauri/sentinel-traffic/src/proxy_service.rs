use crate::proxy::{InterceptState, ProxyConfig, ScanSender, TrafficProxyHandler};
use crate::proxy_connector::{create_insecure_rustls_config, create_upstream_proxy_connector};
use crate::{ProxyStats, Result, TrafficError};
use hudsucker::Proxy;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tracing::{error, info, warn};

/// 代理服务
pub struct ProxyService {
    config: ProxyConfig,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    stats: Arc<RwLock<ProxyStats>>,
    actual_port: Arc<RwLock<Option<u16>>>,
    ca_dir: std::path::PathBuf,
    intercept_state: Option<InterceptState>,
}

impl ProxyService {
    pub fn new(config: ProxyConfig) -> Self {
        // 默认使用当前工作目录下的 ./ca 目录
        let ca_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("ca");

        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: None,
        }
    }

    /// 创建代理服务实例（指定 CA 目录）
    pub fn with_ca_dir(config: ProxyConfig, ca_dir: std::path::PathBuf) -> Self {
        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: None,
        }
    }

    /// 创建代理服务实例（支持拦截）
    pub fn with_intercept(
        config: ProxyConfig,
        ca_dir: std::path::PathBuf,
        intercept_state: InterceptState,
    ) -> Self {
        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: Some(intercept_state),
        }
    }

    /// 启动代理服务（端口自动递增）
    pub async fn start(&self, scan_tx: Option<ScanSender>) -> Result<u16> {
        // 检查是否已启动
        {
            let handle = self.handle.read().await;
            if handle.is_some() {
                return Err(TrafficError::Proxy("Proxy already running".to_string()));
            }
        }

        // 尝试绑定端口（自动递增）
        let mut port = self.config.start_port;
        let listener = loop {
            if port >= self.config.start_port + self.config.max_port_attempts {
                return Err(TrafficError::Proxy(format!(
                    "Failed to bind port after {} attempts",
                    self.config.max_port_attempts
                )));
            }

            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            match TcpListener::bind(addr).await {
                Ok(listener) => {
                    info!("Proxy bound to port {}", port);
                    break listener;
                }
                Err(e) => {
                    warn!("Port {} in use, trying next: {}", port, e);
                    port += 1;
                }
            }
        };

        // 保存实际端口
        *self.actual_port.write().await = Some(port);

        // 确保 Root CA 存在
        let ca_service = crate::certificate::CertificateService::new(self.ca_dir.clone());
        ca_service.ensure_root_ca().await?;

        // macOS: 启动前检查是否已受信；未受信则尝试提示或自动导入
        #[cfg(target_os = "macos")]
        {
            match ca_service.is_root_ca_trusted_macos().await {
                Ok(true) => info!("Root CA is trusted in macOS System keychain"),
                Ok(false) => {
                    warn!(
                        "Root CA not trusted in macOS System keychain. Attempting to add trust..."
                    );
                    if let Err(e) = ca_service.trust_root_ca_macos().await {
                        warn!(
                            "Auto trust failed: {}. You may need to import {} into System keychain and set to 'Always Trust'.",
                            e,
                            ca_service.export_root_ca().map(|p| p.display().to_string()).unwrap_or_else(|_| "<unknown>".into())
                        );
                    } else {
                        info!("Root CA added to System keychain");
                    }
                }
                Err(e) => warn!("Failed to check macOS trust status: {}", e),
            }
        }

        // Windows: 启动前检查是否已受信；未受信则尝试自动导入
        #[cfg(target_os = "windows")]
        {
            match ca_service.is_root_ca_trusted_windows().await {
                Ok(true) => info!("Root CA is trusted in Windows Certificate Store"),
                Ok(false) => {
                    warn!("Root CA not trusted in Windows Certificate Store. Attempting to add trust...");
                    if let Err(e) = ca_service.trust_root_ca_windows().await {
                        warn!(
                            "Auto trust failed: {}. You may need to manually import {} into 'Trusted Root Certification Authorities'.",
                            e,
                            ca_service.export_root_ca().map(|p| p.display().to_string()).unwrap_or_else(|_| "<unknown>".into())
                        );
                    } else {
                        info!("Root CA added to Windows Certificate Store");
                    }
                }
                Err(e) => warn!("Failed to check Windows trust status: {}", e),
            }
        }

        // 获取 CA authority（使用完整证书链版本）
        let ca = ca_service.get_chained_ca()?;

        // 创建处理器（如果有拦截状态，则使用支持拦截的构造器）
        let handler = if let Some(intercept_state) = &self.intercept_state {
            TrafficProxyHandler::with_intercept(
                self.config.clone(),
                scan_tx,
                intercept_state.clone(),
            )
        } else {
            TrafficProxyHandler::new(self.config.clone(), scan_tx)
        };
        let stats = handler.stats();

        // 检查是否配置了 upstream proxy
        // 检查并打印 upstream proxy 配置
        info!(
            "Checking upstream proxy config: {:?}",
            self.config.upstream_proxy
        );
        let use_upstream_proxy = self
            .config
            .upstream_proxy
            .as_ref()
            .map(|up| {
                info!(
                    "Upstream proxy found - enabled: {}, host: {}, port: {}",
                    up.enabled, up.proxy_host, up.proxy_port
                );
                up.enabled
            })
            .unwrap_or(false);

        info!("Use upstream proxy decision: {}", use_upstream_proxy);

        // 创建 HTTPS 连接器（根据是否使用 upstream proxy）
        let proxy_task = if use_upstream_proxy {
            if let Some(upstream_config) = &self.config.upstream_proxy {
                info!(
                    "Starting HTTPS MITM proxy on port {} with upstream proxy {}:{} (destination: {})",
                    port, upstream_config.proxy_host, upstream_config.proxy_port, upstream_config.destination_host
                );

                // 创建带 upstream proxy 的连接器
                let proxy_connector = match create_upstream_proxy_connector(upstream_config) {
                    Ok(connector) => connector,
                    Err(e) => {
                        error!("Failed to create upstream proxy connector: {}", e);
                        return Err(e);
                    }
                };

                // 包装 connector 以返回 hyper_util::rt::TokioIo
                let http_connector = ServiceBuilder::new()
                    .map_response(hyper_util::rt::TokioIo::new)
                    .service(proxy_connector.clone());

                // WebSocket 连接器: tokio-tungstenite 0.28 不再使用 Connector enum
                // 改用 with_rustls_connector，但它需要直接的 TLS stream
                // 由于我们的 ProxyStream 已经处理了 upstream proxy，这里简化处理
                // 暂时不配置 ws_connector，使用默认行为（可能会有问题，但至少能编译）

                tokio::spawn(async move {
                    match Proxy::builder()
                        .with_listener(listener)
                        .with_ca(ca)
                        .with_http_connector(http_connector)
                        .with_http_handler(handler.clone())
                        .with_websocket_handler(handler)
                        .build()
                    {
                        Ok(proxy) => {
                            if let Err(e) = proxy.start().await {
                                error!("Proxy error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to build proxy: {}", e);
                        }
                    }
                })
            } else {
                return Err(TrafficError::Proxy(
                    "Upstream proxy enabled but not configured".to_string(),
                ));
            }
        } else {
            // 不使用 upstream proxy，创建忽略证书验证的连接器
            info!(
                "Starting HTTPS MITM proxy on port {} (ignoring upstream cert errors)",
                port
            );

            // 创建忽略证书验证的 rustls ClientConfig
            let rustls_config = create_insecure_rustls_config();

            // 使用 hyper-rustls connector with custom TLS config
            use hyper_rustls::HttpsConnectorBuilder;

            let https_connector = HttpsConnectorBuilder::new()
                .with_tls_config(rustls_config)
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build();

            tokio::spawn(async move {
                match Proxy::builder()
                    .with_listener(listener)
                    .with_ca(ca)
                    .with_http_connector(https_connector)
                    .with_http_handler(handler.clone())
                    .with_websocket_handler(handler)
                    .build()
                {
                    Ok(proxy) => {
                        if let Err(e) = proxy.start().await {
                            error!("Proxy error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to build proxy: {}", e);
                    }
                }
            })
        };

        // 保存任务句柄和统计
        *self.handle.write().await = Some(proxy_task);
        *self.stats.write().await = stats.read().await.clone();

        Ok(port)
    }

    /// 停止代理服务
    pub async fn stop(&self) -> Result<()> {
        let mut handle = self.handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Proxy stopped");
        }
        *self.actual_port.write().await = None;
        Ok(())
    }

    /// 获取当前端口
    pub async fn get_port(&self) -> Option<u16> {
        *self.actual_port.read().await
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> ProxyStats {
        self.stats.read().await.clone()
    }
}
