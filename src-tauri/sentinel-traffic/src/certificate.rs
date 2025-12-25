//! 证书管理模块
//!
//! 负责：
//! - Root CA 生成与存储（~/.sentinel-ai/ca/）
//! - 为 Hudsucker 提供 RcgenAuthority 实例
//! - macOS Keychain 信任助手（可选）

use crate::certificate_authority::ChainedCertificateAuthority;
use crate::{TrafficError, Result};
use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::rcgen::{CertificateParams, Issuer, KeyPair, SerialNumber};
use rand::RngCore;
use rustls::crypto::ring;
use std::fs;
use std::path::PathBuf;

/// 证书管理服务
pub struct CertificateService {
    ca_dir: PathBuf,
}

impl CertificateService {
    /// 创建证书服务实例
    pub fn new(ca_dir: PathBuf) -> Self {
        Self { ca_dir }
    }

    /// 强制重新生成 Root CA（删除旧的）
    pub async fn regenerate_root_ca(&self) -> Result<()> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");

        // 删除旧证书（如果存在）
        if cert_path.exists() {
            fs::remove_file(&cert_path).map_err(|e| {
                TrafficError::Certificate(format!("Failed to delete old certificate: {}", e))
            })?;
        }
        if key_path.exists() {
            fs::remove_file(&key_path).map_err(|e| {
                TrafficError::Certificate(format!("Failed to delete old key: {}", e))
            })?;
        }

        // 生成新证书
        self.generate_root_ca().await
    }

    /// 生成新的 Root CA（内部方法）
    async fn generate_root_ca(&self) -> Result<()> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");

        // 创建 CA 目录
        fs::create_dir_all(&self.ca_dir).map_err(|e| {
            TrafficError::Certificate(format!("Failed to create CA directory: {}", e))
        })?;

        // 生成密钥对
        let key_pair = KeyPair::generate().map_err(|e| {
            TrafficError::Certificate(format!("Failed to generate key pair: {}", e))
        })?;

        // 生成自签名 CA
        let mut params = CertificateParams::new(vec!["Sentinel AI Traffic Scan CA".to_string()])
            .map_err(|e| TrafficError::Certificate(format!("Failed to create params: {}", e)))?;

        params.is_ca =
            hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);
        params.distinguished_name.push(
            hudsucker::rcgen::DnType::CommonName,
            "Sentinel AI Traffic Scan CA",
        );

        // 设置证书有效期（10年）
        use chrono::Datelike;
        use hudsucker::rcgen::date_time_ymd;
        let now = chrono::Utc::now();
        let not_before = date_time_ymd(now.year(), now.month() as u8, now.day() as u8);
        let future = now + chrono::Duration::days(3650);
        let not_after = date_time_ymd(future.year(), future.month() as u8, future.day() as u8);
        params.not_before = not_before;
        params.not_after = not_after;

        // 添加 Key Usage 扩展
        params.key_usages = vec![
            hudsucker::rcgen::KeyUsagePurpose::KeyCertSign,
            hudsucker::rcgen::KeyUsagePurpose::CrlSign,
            hudsucker::rcgen::KeyUsagePurpose::DigitalSignature,
        ];

        // 添加 Extended Key Usage（某些应用需要）
        params.extended_key_usages = vec![
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // 添加更多 Distinguished Name 属性（提高兼容性）
        params.distinguished_name.push(
            hudsucker::rcgen::DnType::OrganizationName,
            "Sentinel AI",
        );
        params.distinguished_name.push(
            hudsucker::rcgen::DnType::CountryName,
            "CN",
        );

        // 随机序列号（避免所有环境都为固定值 1 导致某些客户端缓存冲突）
        let mut rng = rand::rngs::OsRng;
        let random_serial: u64 = rng.next_u64();
        params.serial_number = Some(SerialNumber::from(random_serial));
        tracing::debug!("Generated root CA serial number={:#x}", random_serial);

        // 使用 params 生成证书
        let cert = params.self_signed(&key_pair).map_err(|e| {
            TrafficError::Certificate(format!("Failed to generate certificate: {}", e))
        })?;

        // 保存证书（PEM 格式）
        let cert_pem = cert.pem();
        fs::write(&cert_path, &cert_pem).map_err(|e| {
            TrafficError::Certificate(format!("Failed to write certificate: {}", e))
        })?;

        // 保存私钥（PEM 格式）
        let key_pem = key_pair.serialize_pem();
        fs::write(&key_path, key_pem).map_err(|e| {
            TrafficError::Certificate(format!("Failed to write private key: {}", e))
        })?;

        tracing::info!("Root CA generated at {}", cert_path.display());
        Ok(())
    }

    /// 确保 Root CA 存在，不存在则生成
    pub async fn ensure_root_ca(&self) -> Result<()> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");

        if cert_path.exists() && key_path.exists() {
            tracing::info!("Root CA already exists at {}", cert_path.display());
            return Ok(());
        }

        self.generate_root_ca().await
    }

    /// 获取证书指纹（SHA-256）
    pub fn get_certificate_fingerprint(&self) -> Result<String> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        if !cert_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".to_string()));
        }

        let cert_pem = fs::read(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&cert_pem);
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }

    /// 获取 RcgenAuthority 实例（用于 Hudsucker，已弃用）
    #[deprecated(note = "Use get_chained_ca() instead for full certificate chain support")]
    pub fn get_ca(&self) -> Result<RcgenAuthority> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");

        if !key_path.exists() || !cert_path.exists() {
            return Err(TrafficError::Certificate(
                "Root CA not found. Call ensure_root_ca() first.".to_string(),
            ));
        }

        // 读取证书和私钥
        let cert_pem = fs::read_to_string(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;
        let key_pem = fs::read_to_string(&key_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read private key: {}", e)))?;

        // 解析私钥
        let key_pair = KeyPair::from_pem(&key_pem).map_err(|e| {
            TrafficError::Certificate(format!("Failed to parse private key: {}", e))
        })?;

        // 使用 rcgen 的公开 API 创建 Issuer
        let issuer = Issuer::from_ca_cert_pem(&cert_pem, key_pair)
            .map_err(|e| TrafficError::Certificate(format!("Failed to create issuer: {}", e)))?;

        // 创建 RcgenAuthority（使用 ring 作为 crypto provider）
        Ok(RcgenAuthority::new(issuer, 1000, ring::default_provider()))
    }

    /// 获取 CA 证书的 DER 格式（用于构建完整证书链）
    pub fn get_ca_cert_der(&self) -> Result<Vec<u8>> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        if !cert_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".to_string()));
        }

        let cert_pem = fs::read(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;

        // 解析 PEM 并转换为 DER
        let pem = pem::parse(&cert_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse PEM: {}", e)))?;

        Ok(pem.contents().to_vec())
    }

    /// 获取支持完整证书链的 CertificateAuthority 实例
    ///
    /// 与 get_ca() 不同，此方法返回的 authority 在 TLS 握手时
    /// 会发送完整证书链（叶子证书 + CA 证书），解决某些客户端的验证问题。
    pub fn get_chained_ca(&self) -> Result<ChainedCertificateAuthority> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");

        if !key_path.exists() || !cert_path.exists() {
            return Err(TrafficError::Certificate(
                "Root CA not found. Call ensure_root_ca() first.".to_string(),
            ));
        }

        // 读取证书和私钥
        let cert_pem = fs::read_to_string(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;
        let key_pem = fs::read_to_string(&key_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read private key: {}", e)))?;

        // 解析私钥
        let key_pair = KeyPair::from_pem(&key_pem).map_err(|e| {
            TrafficError::Certificate(format!("Failed to parse private key: {}", e))
        })?;

        // 创建 Issuer
        let issuer = Issuer::from_ca_cert_pem(&cert_pem, key_pair)
            .map_err(|e| TrafficError::Certificate(format!("Failed to create issuer: {}", e)))?;

        // 获取 CA 证书 DER
        let ca_cert_der = self.get_ca_cert_der()?;

        // 创建支持完整证书链的 authority
        Ok(ChainedCertificateAuthority::new(
            issuer,
            ca_cert_der,
            1000,
            ring::default_provider(),
        ))
    }

    /// 读取 Root CA PEM 内容
    pub fn read_root_ca_pem(&self) -> Result<String> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        if !cert_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".into()));
        }
        let pem = std::fs::read_to_string(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read root CA: {}", e)))?;
        Ok(pem)
    }

    /// 构造一个完整链（当前只有 leaf + root）。
    /// 由于 hudsucker 动态签发 leaf，我们无法直接在 handshake 阶段插入，
    /// 但可以在需要分发或导出时供其他客户端使用。
    pub fn build_chain_for_leaf(&self, leaf_pem: &str) -> Result<String> {
        let root = self.read_root_ca_pem()?;
        // 简单拼接，保持 leaf 在前（常见顺序 leaf -> intermediate(s) -> root）
        Ok(format!("{}\n{}", leaf_pem.trim_end(), root.trim_end()))
    }

    /// macOS 上检测 Root CA 是否已受信（系统钥匙串）
    #[cfg(target_os = "macos")]
    pub async fn is_root_ca_trusted_macos(&self) -> Result<bool> {
        use tokio::process::Command;
        let cert_path = self.export_root_ca()?;
        let subject_cn = "Sentinel AI Traffic Scan CA"; // 与生成时保持一致
                                                        // 使用 security 查找匹配 CN 的证书
        let output = Command::new("security")
            .args([
                "find-certificate",
                "-c",
                subject_cn,
                "/Library/Keychains/System.keychain",
            ])
            .output()
            .await
            .map_err(|e| TrafficError::Certificate(format!("Failed to execute security: {}", e)))?;

        if !output.status.success() {
            // 未找到或命令失败，返回 false（不报错以便上层决定是否自动信任）
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // 简单判定：若包含证书路径或主题 CN，视为已信任（严格校验可加 fingerprint 比对）
        let trusted = stdout.contains(subject_cn);
        tracing::debug!(
            "macOS trust check for root CA path {:?}: trusted={}",
            cert_path,
            trusted
        );
        Ok(trusted)
    }

    /// 导出 Root CA 证书路径（用于手动导入浏览器）
    pub fn export_root_ca(&self) -> Result<PathBuf> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        if !cert_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".to_string()));
        }
        Ok(cert_path)
    }

    /// 导出证书为 DER 格式
    pub fn export_cert_der(&self) -> Result<PathBuf> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let der_path = self.ca_dir.join("root-ca.der");
        
        if !cert_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".to_string()));
        }

        let cert_pem = fs::read(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;
        
        let pem = pem::parse(&cert_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse PEM: {}", e)))?;
        
        fs::write(&der_path, pem.contents())
            .map_err(|e| TrafficError::Certificate(format!("Failed to write DER: {}", e)))?;
        
        tracing::info!("Exported certificate in DER format: {}", der_path.display());
        Ok(der_path)
    }

    /// 导出私钥为 DER 格式
    pub fn export_key_der(&self) -> Result<PathBuf> {
        let key_path = self.ca_dir.join("root-ca.key");
        let der_path = self.ca_dir.join("root-ca-key.der");
        
        if !key_path.exists() {
            return Err(TrafficError::Certificate("Root CA key not found".to_string()));
        }

        let key_pem = fs::read(&key_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read key: {}", e)))?;
        
        let pem = pem::parse(&key_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse PEM: {}", e)))?;
        
        fs::write(&der_path, pem.contents())
            .map_err(|e| TrafficError::Certificate(format!("Failed to write DER: {}", e)))?;
        
        tracing::info!("Exported private key in DER format: {}", der_path.display());
        Ok(der_path)
    }

    /// 导出为 PKCS#12 格式 (Windows compatible)
    pub fn export_pkcs12(&self, password: Option<&str>) -> Result<PathBuf> {
        let cert_path = self.ca_dir.join("root-ca.pem");
        let key_path = self.ca_dir.join("root-ca.key");
        let p12_path = self.ca_dir.join("root-ca.p12");
        
        if !cert_path.exists() || !key_path.exists() {
            return Err(TrafficError::Certificate("Root CA not found".to_string()));
        }

        let cert_pem = fs::read_to_string(&cert_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read certificate: {}", e)))?;
        let key_pem = fs::read_to_string(&key_path)
            .map_err(|e| TrafficError::Certificate(format!("Failed to read key: {}", e)))?;
        
        // Parse certificate
        let cert = openssl::x509::X509::from_pem(cert_pem.as_bytes())
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse certificate: {}", e)))?;
        
        // Parse private key
        let pkey = openssl::pkey::PKey::private_key_from_pem(key_pem.as_bytes())
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse private key: {}", e)))?;
        
        // Create PKCS#12
        let pwd = password.unwrap_or("");
        let pkcs12 = openssl::pkcs12::Pkcs12::builder()
            .name("Sentinel AI CA")
            .pkey(&pkey)
            .cert(&cert)
            .build2(pwd)
            .map_err(|e| TrafficError::Certificate(format!("Failed to create PKCS#12: {}", e)))?;
        
        let der = pkcs12.to_der()
            .map_err(|e| TrafficError::Certificate(format!("Failed to encode PKCS#12: {}", e)))?;
        
        fs::write(&p12_path, der)
            .map_err(|e| TrafficError::Certificate(format!("Failed to write PKCS#12: {}", e)))?;
        
        tracing::info!("Exported certificate in PKCS#12 format: {}", p12_path.display());
        Ok(p12_path)
    }

    /// 从 PKCS#12 文件导入证书和私钥
    pub fn import_pkcs12(&self, p12_data: &[u8], password: &str) -> Result<()> {
        let pkcs12 = openssl::pkcs12::Pkcs12::from_der(p12_data)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse PKCS#12: {}", e)))?;
        
        let parsed = pkcs12.parse2(password)
            .map_err(|e| TrafficError::Certificate(format!("Failed to decrypt PKCS#12: {}", e)))?;
        
        let cert = parsed.cert.ok_or_else(|| {
            TrafficError::Certificate("PKCS#12 does not contain certificate".to_string())
        })?;
        let pkey = parsed.pkey.ok_or_else(|| {
            TrafficError::Certificate("PKCS#12 does not contain private key".to_string())
        })?;
        
        // Save certificate
        let cert_pem = cert.to_pem()
            .map_err(|e| TrafficError::Certificate(format!("Failed to encode certificate: {}", e)))?;
        let cert_path = self.ca_dir.join("root-ca.pem");
        fs::write(&cert_path, cert_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to write certificate: {}", e)))?;
        
        // Save private key
        let key_pem = pkey.private_key_to_pem_pkcs8()
            .map_err(|e| TrafficError::Certificate(format!("Failed to encode private key: {}", e)))?;
        let key_path = self.ca_dir.join("root-ca.key");
        fs::write(&key_path, key_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to write private key: {}", e)))?;
        
        tracing::info!("Imported certificate and key from PKCS#12");
        Ok(())
    }

    /// 从 DER 格式导入证书和私钥
    pub fn import_der(&self, cert_der: &[u8], key_der: &[u8]) -> Result<()> {
        // Parse and convert certificate
        let cert = openssl::x509::X509::from_der(cert_der)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse DER certificate: {}", e)))?;
        
        // Parse and convert private key
        let pkey = openssl::pkey::PKey::private_key_from_der(key_der)
            .map_err(|e| TrafficError::Certificate(format!("Failed to parse DER private key: {}", e)))?;
        
        // Save certificate
        let cert_pem = cert.to_pem()
            .map_err(|e| TrafficError::Certificate(format!("Failed to encode certificate: {}", e)))?;
        let cert_path = self.ca_dir.join("root-ca.pem");
        fs::write(&cert_path, cert_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to write certificate: {}", e)))?;
        
        // Save private key
        let key_pem = pkey.private_key_to_pem_pkcs8()
            .map_err(|e| TrafficError::Certificate(format!("Failed to encode private key: {}", e)))?;
        let key_path = self.ca_dir.join("root-ca.key");
        fs::write(&key_path, key_pem)
            .map_err(|e| TrafficError::Certificate(format!("Failed to write private key: {}", e)))?;
        
        tracing::info!("Imported certificate and key from DER format");
        Ok(())
    }

    /// 获取 CA 目录路径
    pub fn get_ca_dir(&self) -> &PathBuf {
        &self.ca_dir
    }

    /// macOS Keychain 信任 Root CA（需要用户授权）
    #[cfg(target_os = "macos")]
    pub async fn trust_root_ca_macos(&self) -> Result<()> {
        let cert_path = self.export_root_ca()?;

        let cert_path_str = cert_path.to_str().ok_or_else(|| {
            TrafficError::Certificate("Invalid certificate path encoding".to_string())
        })?;

        // 调用 security 命令添加到系统 Keychain
        let output = tokio::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
                cert_path_str,
            ])
            .output()
            .await
            .map_err(|e| {
                TrafficError::Certificate(format!("Failed to execute security command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TrafficError::Certificate(format!(
                "Failed to trust CA: {}",
                stderr
            )));
        }

        tracing::info!("Root CA trusted in macOS Keychain");
        Ok(())
    }

    /// Windows 证书存储信任 Root CA
    #[cfg(target_os = "windows")]
    pub async fn trust_root_ca_windows(&self) -> Result<()> {
        let cert_path = self.export_root_ca()?;

        let cert_path_str = cert_path.to_str().ok_or_else(|| {
            TrafficError::Certificate("Invalid certificate path encoding".to_string())
        })?;

        // 使用 certutil 命令添加到用户级受信任根证书存储
        // -user 表示当前用户, "Root" 表示受信任的根证书颁发机构
        let output = tokio::process::Command::new("certutil")
            .args(["-addstore", "-user", "Root", cert_path_str])
            .output()
            .await
            .map_err(|e| {
                TrafficError::Certificate(format!("Failed to execute certutil: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(TrafficError::Certificate(format!(
                "Failed to trust CA: {} {}",
                stderr, stdout
            )));
        }

        tracing::info!("Root CA trusted in Windows Certificate Store (Current User)");
        Ok(())
    }

    /// Windows 检测 Root CA 是否已受信
    #[cfg(target_os = "windows")]
    pub async fn is_root_ca_trusted_windows(&self) -> Result<bool> {
        let subject_cn = "Sentinel AI Traffic Scan CA";

        // 使用 certutil 查找证书
        let output = tokio::process::Command::new("certutil")
            .args(["-store", "-user", "Root"])
            .output()
            .await
            .map_err(|e| {
                TrafficError::Certificate(format!("Failed to execute certutil: {}", e))
            })?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trusted = stdout.contains(subject_cn);
        
        tracing::debug!("Windows trust check for root CA: trusted={}", trusted);
        Ok(trusted)
    }

    /// Windows 移除已信任的 Root CA
    #[cfg(target_os = "windows")]
    pub async fn untrust_root_ca_windows(&self) -> Result<()> {
        let subject_cn = "Sentinel AI Traffic Scan CA";

        // 使用 certutil 删除证书
        let output = tokio::process::Command::new("certutil")
            .args(["-delstore", "-user", "Root", subject_cn])
            .output()
            .await
            .map_err(|e| {
                TrafficError::Certificate(format!("Failed to execute certutil: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // 如果证书不存在，不算错误
            if !stderr.contains("找不到") && !stderr.contains("not found") {
                return Err(TrafficError::Certificate(format!(
                    "Failed to remove CA: {}",
                    stderr
                )));
            }
        }

        tracing::info!("Root CA removed from Windows Certificate Store");
        Ok(())
    }
}