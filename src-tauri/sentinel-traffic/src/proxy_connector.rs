use crate::proxy::UpstreamProxyConfig;
use crate::Result;
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::Service;
use tracing::{debug, info, warn};

/// 忽略证书验证的 ServerCertVerifier
/// 用于抓取证书异常的站点（如自签名、过期、版本不支持等）
#[derive(Debug)]
struct InsecureServerCertVerifier;

impl ServerCertVerifier for InsecureServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        // 忽略所有证书验证错误
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// 代理流封装，统一 HTTP 和 HTTPS 连接
pub enum ProxyStream {
    Http(tokio::net::TcpStream),
    Https(tokio_rustls::client::TlsStream<tokio::net::TcpStream>),
}

impl tokio::io::AsyncRead for ProxyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_read(cx, buf),
            ProxyStream::Https(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for ProxyStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_write(cx, buf),
            ProxyStream::Https(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_flush(cx),
            ProxyStream::Https(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_shutdown(cx),
            ProxyStream::Https(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl hyper::rt::Read for ProxyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        let n = unsafe {
            let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
            match tokio::io::AsyncRead::poll_read(self, cx, &mut tbuf) {
                Poll::Ready(Ok(())) => tbuf.filled().len(),
                other => return other,
            }
        };

        unsafe {
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}

impl hyper::rt::Write for ProxyStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write(self, cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_flush(self, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_shutdown(self, cx)
    }
}

impl hyper_util::client::legacy::connect::Connection for ProxyStream {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}

/// 自定义 Proxy Connector，用于处理 upstream proxy 连接
/// 替代 hyper-proxy2，提供更稳定的 CONNECT 隧道处理
#[derive(Clone)]
pub struct CustomProxyConnector {
    proxy_host: String,
    proxy_port: u16,
    tls_connector: tokio_rustls::TlsConnector,
}

impl CustomProxyConnector {
    pub fn new(host: String, port: u16, tls_config: Arc<rustls::ClientConfig>) -> Self {
        Self {
            proxy_host: host,
            proxy_port: port,
            tls_connector: tokio_rustls::TlsConnector::from(tls_config),
        }
    }
}

impl Service<hyper::Uri> for CustomProxyConnector {
    type Response = ProxyStream;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: hyper::Uri) -> Self::Future {
        let proxy_host = self.proxy_host.clone();
        let proxy_port = self.proxy_port;
        let tls_connector = self.tls_connector.clone();

        Box::pin(async move {
            debug!(
                "CustomProxyConnector: connecting to proxy {}:{}",
                proxy_host, proxy_port
            );

            // 1. 连接到 Upstream Proxy
            let mut stream = tokio::net::TcpStream::connect((proxy_host.as_str(), proxy_port))
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            // 2. 决定是否使用 CONNECT
            let host = dst.host().unwrap_or("").to_string();
            let port = dst
                .port_u16()
                .unwrap_or(if dst.scheme_str() == Some("https") {
                    443
                } else {
                    80
                });

            let is_https = dst.scheme_str() == Some("https") || port == 443;

            if is_https {
                debug!("CustomProxyConnector: creating tunnel to {}:{}", host, port);
                let connect_req = format!(
                    "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
                    host, port, host, port
                );

                stream
                    .write_all(connect_req.as_bytes())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                // 逐字节读取响应头，避免多读
                let mut header = Vec::new();
                loop {
                    let b = stream
                        .read_u8()
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    header.push(b);

                    if header.len() > 4096 {
                        return Err("Proxy response too large".into());
                    }

                    if header.ends_with(b"\r\n\r\n") {
                        let response = String::from_utf8_lossy(&header);
                        if !response.contains(" 200 ") {
                            return Err(format!("Proxy connect failed: {}", response).into());
                        }
                        debug!("CustomProxyConnector: tunnel established");
                        break;
                    }
                }

                // 3. 建立 TLS 连接
                let domain = ServerName::try_from(host.as_str())
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let tls_stream = tls_connector
                    .connect(domain.to_owned(), stream)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok(ProxyStream::Https(tls_stream))
            } else {
                Ok(ProxyStream::Http(stream))
            }
        })
    }
}

/// 创建忽略证书验证的 rustls ClientConfig
/// 支持弱加密套件和旧版本 TLS，以便抓取证书异常的站点
/// 创建忽略证书验证的 rustls 配置（不含 ALPN，用于 HttpsConnectorBuilder）
pub(crate) fn create_insecure_rustls_config() -> rustls::ClientConfig {
    // 注意：不要在这里设置 ALPN 协议
    // HttpsConnectorBuilder 的 enable_http1/enable_http2 会自动设置
    // 如果这里预设了 ALPN，会导致 panic: "ALPN protocols should not be pre-defined"
    rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))
        .with_no_client_auth()
}

/// 创建忽略证书验证的 rustls 配置（含 ALPN，用于 tokio-rustls TlsConnector）
fn create_insecure_rustls_config_with_alpn() -> rustls::ClientConfig {
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))
        .with_no_client_auth();

    // 配置 ALPN 协议（用于 tokio-rustls 的直接连接）
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    config
}

/// 创建带 upstream proxy 的 HTTPS connector
pub(crate) fn create_upstream_proxy_connector(
    upstream_config: &UpstreamProxyConfig,
) -> Result<CustomProxyConnector> {
    info!(
        "Creating upstream proxy connector: host={}, port={}, auth_type={}",
        upstream_config.proxy_host, upstream_config.proxy_port, upstream_config.auth_type
    );

    // 使用带 ALPN 的配置，因为 CustomProxyConnector 使用 tokio-rustls
    let rustls_config = create_insecure_rustls_config_with_alpn();
    let proxy_connector = CustomProxyConnector::new(
        upstream_config.proxy_host.clone(),
        upstream_config.proxy_port,
        Arc::new(rustls_config),
    );

    // TODO: Basic 认证支持将在后续版本实现
    if upstream_config.auth_type == "Basic" {
        warn!("Basic authentication for upstream proxy is not yet implemented in CustomProxyConnector");
    }

    info!("Upstream proxy connector created successfully");
    Ok(proxy_connector)
}
