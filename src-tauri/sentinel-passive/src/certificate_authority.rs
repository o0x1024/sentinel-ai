//! 自定义证书颁发机构
//!
//! 基于 Hudsucker 的 CertificateAuthority trait，实现完整证书链支持。
//! 解决 Hudsucker 默认实现只发送叶子证书导致某些客户端验证失败的问题。
//! 同时支持从 TLS SNI 获取真实域名，解决 Proxifier 虚拟 IP 模式下的证书验证问题。

use http::uri::Authority;
use hudsucker::certificate_authority::CertificateAuthority;
use hudsucker::rcgen::{CertificateParams, DistinguishedName, DnType, Issuer, KeyPair, SanType};
use moka::sync::Cache as SyncCache;
use rand::Rng;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use std::fmt::Debug;
use std::net::IpAddr;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

const TTL_SECS: i64 = 365 * 24 * 60 * 60;
const CACHE_TTL: u64 = TTL_SECS as u64 / 2;
const NOT_BEFORE_OFFSET: i64 = 60;

/// 基于 SNI 的动态证书解析器
///
/// 在 TLS 握手时根据 ClientHello 中的 SNI 动态生成证书。
/// 解决 Proxifier 虚拟 IP 模式下，CONNECT 请求使用虚拟 IP
/// 但客户端仍然期望原始域名证书的问题。
struct SniCertResolver {
    issuer: Arc<Issuer<'static, KeyPair>>,
    private_key: PrivateKeyDer<'static>,
    ca_cert: CertificateDer<'static>,
    cache: SyncCache<String, Arc<CertifiedKey>>,
    provider: Arc<CryptoProvider>,
    /// 来自 CONNECT 请求的 authority（可能是虚拟 IP）
    fallback_authority: String,
}

impl Debug for SniCertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SniCertResolver")
            .field("fallback_authority", &self.fallback_authority)
            .field("cache_size", &self.cache.entry_count())
            .finish()
    }
}

impl SniCertResolver {
    fn new(
        issuer: Arc<Issuer<'static, KeyPair>>,
        private_key: PrivateKeyDer<'static>,
        ca_cert: CertificateDer<'static>,
        provider: Arc<CryptoProvider>,
        fallback_authority: String,
    ) -> Self {
        Self {
            issuer,
            private_key,
            ca_cert,
            cache: SyncCache::builder()
                .max_capacity(1000)
                .time_to_live(std::time::Duration::from_secs(CACHE_TTL))
                .build(),
            provider,
            fallback_authority,
        }
    }

    /// 为指定主机生成证书
    fn gen_certified_key(&self, host: &str) -> Arc<CertifiedKey> {
        // 检查缓存
        if let Some(key) = self.cache.get(host) {
            tracing::debug!("Using cached certificate for {}", host);
            return key;
        }

        tracing::info!(
            "SniCertResolver: Generating certificate for SNI host: {}",
            host
        );

        let mut params = CertificateParams::default();
        params.serial_number = Some(rand::thread_rng().gen::<u64>().into());

        let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
        params.not_before = not_before;
        params.not_after = not_before + Duration::seconds(TTL_SECS);

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, host);
        params.distinguished_name = distinguished_name;

        // 添加 SAN (Subject Alternative Name)
        if let Ok(ip) = host.parse::<IpAddr>() {
            params.subject_alt_names.push(SanType::IpAddress(ip));
            // tracing::info!("Generated cert with IP SAN: {}", ip);
        } else {
            if let Ok(ia5) = hudsucker::rcgen::string::Ia5String::try_from(host) {
                params.subject_alt_names.push(SanType::DnsName(ia5));
                // tracing::info!("Generated cert with DNS SAN: {}", host);
            }
        }

        let leaf_cert: CertificateDer<'static> = params
            .signed_by(self.issuer.key(), self.issuer.as_ref())
            .expect("Failed to sign certificate")
            .into();

        // 构建完整证书链
        let cert_chain = vec![leaf_cert, self.ca_cert.clone()];

        // 创建签名密钥
        let signing_key = self
            .provider
            .key_provider
            .load_private_key(self.private_key.clone_key())
            .expect("Failed to load private key");

        let certified_key = Arc::new(CertifiedKey::new(cert_chain, signing_key));

        // 缓存
        self.cache.insert(host.to_string(), certified_key.clone());

        certified_key
    }
}

impl ResolvesServerCert for SniCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        // 优先使用 SNI 中的域名
        let host = if let Some(sni) = client_hello.server_name() {
            // tracing::info!(
            //     "SniCertResolver: Using SNI hostname: {} (fallback was: {})",
            //     sni,
            //     self.fallback_authority
            // );
            sni.to_string()
        } else {
            // 没有 SNI，使用 fallback（来自 CONNECT 请求的 authority）
            // tracing::info!(
            //     "SniCertResolver: No SNI, using fallback: {}",
            //     self.fallback_authority
            // );
            // 从 fallback_authority 中移除端口号
            self.fallback_authority
                .split(':')
                .next()
                .unwrap_or(&self.fallback_authority)
                .to_string()
        };

        Some(self.gen_certified_key(&host))
    }
}

/// 支持完整证书链和 SNI 的证书颁发机构
///
/// 特性：
/// 1. 发送完整证书链（叶子证书 + CA 证书）
/// 2. 根据 TLS SNI 动态生成证书（解决 Proxifier 虚拟 IP 问题）
pub struct ChainedCertificateAuthority {
    issuer: Arc<Issuer<'static, KeyPair>>,
    private_key: PrivateKeyDer<'static>,
    ca_cert: CertificateDer<'static>,
    provider: Arc<CryptoProvider>,
}

impl ChainedCertificateAuthority {
    /// 创建支持完整证书链的证书颁发机构
    pub fn new(
        issuer: Issuer<'static, KeyPair>,
        ca_cert_der: Vec<u8>,
        _cache_size: u64,
        provider: CryptoProvider,
    ) -> Self {
        let private_key =
            PrivateKeyDer::from(PrivatePkcs8KeyDer::from(issuer.key().serialize_der()));

        Self {
            issuer: Arc::new(issuer),
            private_key,
            ca_cert: CertificateDer::from(ca_cert_der),
            provider: Arc::new(provider),
        }
    }
}

impl CertificateAuthority for ChainedCertificateAuthority {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        // tracing::info!(
        //     "ChainedCertificateAuthority: Creating ServerConfig with SNI resolver for {}",
        //     authority
        // );

        // 创建 SNI 证书解析器
        let cert_resolver = SniCertResolver::new(
            self.issuer.clone(),
            self.private_key.clone_key(),
            self.ca_cert.clone(),
            self.provider.clone(),
            authority.to_string(),
        );

        // 创建带有 cert_resolver 的 ServerConfig
        let mut server_cfg = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_safe_default_protocol_versions()
            .expect("Failed to specify protocol versions")
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(cert_resolver));

        // 配置 ALPN 协议
        server_cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Arc::new(server_cfg)
    }
}
