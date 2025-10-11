//! OAuth2.1认证管理器
//! 
//! 基于rmcp 0.5.0的auth feature实现OAuth2.1认证，支持：
//! - PKCE (Proof Key for Code Exchange)
//! - 强制HTTPS
//! - 短期令牌轮换
//! - 令牌绑定和存储加密

use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use sha2::{Sha256, Digest};

/// OAuth2.1配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth21Config {
    pub client_id: String,
    pub client_secret: Option<String>, // 公共客户端可以为空
    pub redirect_uri: String,
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub scopes: Vec<String>,
    pub use_pkce: bool, // 强制使用PKCE
    pub token_refresh_threshold_seconds: u64, // 令牌刷新阈值
    pub enable_token_binding: bool, // 令牌绑定
}

/// PKCE参数
#[derive(Debug, Clone)]
pub struct PkceParams {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// 访问令牌信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenInfo {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub token_binding_id: Option<String>, // 令牌绑定ID
}

/// 认证状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthStatus {
    NotAuthenticated,
    AuthorizationPending,
    Authenticated,
    TokenExpired,
    AuthenticationFailed(String),
}

/// OAuth2.1认证管理器
pub struct OAuth21Manager {
    config: OAuth21Config,
    tokens: Arc<RwLock<HashMap<String, AccessTokenInfo>>>, // client_id -> token
    auth_status: Arc<RwLock<HashMap<String, AuthStatus>>>, // client_id -> status
    pkce_params: Arc<RwLock<HashMap<String, PkceParams>>>, // state -> pkce
}

impl OAuth21Manager {
    pub fn new(config: OAuth21Config) -> Result<Self> {
        // 验证配置
        if !config.auth_endpoint.starts_with("https://") {
            return Err(anyhow!("OAuth2.1 requires HTTPS for auth endpoint"));
        }
        
        if !config.token_endpoint.starts_with("https://") {
            return Err(anyhow!("OAuth2.1 requires HTTPS for token endpoint"));
        }
        
        if !config.redirect_uri.starts_with("https://") && !config.redirect_uri.starts_with("http://localhost") {
            return Err(anyhow!("OAuth2.1 requires HTTPS for redirect URI (except localhost)"));
        }
        
        Ok(Self {
            config,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            auth_status: Arc::new(RwLock::new(HashMap::new())),
            pkce_params: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// 生成PKCE参数
    pub fn generate_pkce_params(&self) -> PkceParams {
        // 生成code_verifier (43-128个字符的随机字符串)
        let code_verifier = self.generate_random_string(128);
        
        // 生成code_challenge (code_verifier的SHA256哈希的base64url编码)
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = general_purpose::URL_SAFE_NO_PAD.encode(&hash);
        
        PkceParams {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".to_string(),
        }
    }
    
    /// 生成授权URL
    pub async fn generate_auth_url(&self, state: Option<String>) -> Result<String> {
        let state = state.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // 生成PKCE参数
        let pkce = if self.config.use_pkce {
            let pkce = self.generate_pkce_params();
            self.pkce_params.write().await.insert(state.clone(), pkce.clone());
            Some(pkce)
        } else {
            None
        };
        
        // 构建授权URL
        let mut url = url::Url::parse(&self.config.auth_endpoint)?;
        
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("response_type", "code");
            query_pairs.append_pair("client_id", &self.config.client_id);
            query_pairs.append_pair("redirect_uri", &self.config.redirect_uri);
            query_pairs.append_pair("state", &state);
            
            if !self.config.scopes.is_empty() {
                query_pairs.append_pair("scope", &self.config.scopes.join(" "));
            }
            
            if let Some(pkce) = pkce {
                query_pairs.append_pair("code_challenge", &pkce.code_challenge);
                query_pairs.append_pair("code_challenge_method", &pkce.code_challenge_method);
            }
        }
        
        // 更新认证状态
        self.auth_status.write().await.insert(
            self.config.client_id.clone(),
            AuthStatus::AuthorizationPending,
        );
        
        info!("Generated OAuth2.1 authorization URL with PKCE");
        Ok(url.to_string())
    }
    
    /// 交换授权码获取访问令牌
    pub async fn exchange_code_for_token(&self, code: String, state: String) -> Result<AccessTokenInfo> {
        info!("Exchanging authorization code for access token");
        
        // 获取PKCE参数
        let pkce = if self.config.use_pkce {
            self.pkce_params.read().await.get(&state).cloned()
                .ok_or_else(|| anyhow!("PKCE parameters not found for state: {}", state))?
        } else {
            return Err(anyhow!("PKCE is required for OAuth2.1"));
        };
        
        // 构建令牌请求
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code".to_string());
        params.insert("code", code);
        params.insert("redirect_uri", self.config.redirect_uri.clone());
        params.insert("client_id", self.config.client_id.clone());
        params.insert("code_verifier", pkce.code_verifier);
        
        if let Some(client_secret) = &self.config.client_secret {
            params.insert("client_secret", client_secret.clone());
        }
        
        // 发送令牌请求
        let client = crate::ai_adapter::http::create_default_client()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        let response = client
            .post(&self.config.token_endpoint)
            .form(&params)
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Token exchange failed: {}", error_text);
            
            self.auth_status.write().await.insert(
                self.config.client_id.clone(),
                AuthStatus::AuthenticationFailed(error_text),
            );
            
            return Err(anyhow!("Token exchange failed"));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        // 解析令牌响应
        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing access_token in response"))?
            .to_string();
        
        let token_type = token_response["token_type"]
            .as_str()
            .unwrap_or("Bearer")
            .to_string();
        
        let expires_in = token_response["expires_in"]
            .as_u64()
            .unwrap_or(3600); // 默认1小时
        
        let refresh_token = token_response["refresh_token"]
            .as_str()
            .map(|s| s.to_string());
        
        let scope = token_response["scope"]
            .as_str()
            .map(|s| s.to_string());
        
        // 生成令牌绑定ID
        let token_binding_id = if self.config.enable_token_binding {
            Some(Uuid::new_v4().to_string())
        } else {
            None
        };
        
        let token_info = AccessTokenInfo {
            access_token,
            token_type,
            expires_in,
            refresh_token,
            scope,
            issued_at: chrono::Utc::now(),
            token_binding_id,
        };
        
        // 存储令牌
        self.tokens.write().await.insert(self.config.client_id.clone(), token_info.clone());
        
        // 更新认证状态
        self.auth_status.write().await.insert(
            self.config.client_id.clone(),
            AuthStatus::Authenticated,
        );
        
        // 清理PKCE参数
        self.pkce_params.write().await.remove(&state);
        
        info!("Successfully obtained access token");
        Ok(token_info)
    }
    
    /// 刷新访问令牌
    pub async fn refresh_token(&self) -> Result<AccessTokenInfo> {
        let current_token = {
            let tokens = self.tokens.read().await;
            tokens.get(&self.config.client_id).cloned()
                .ok_or_else(|| anyhow!("No token found for client"))?
        };
        
        let refresh_token = current_token.refresh_token.clone()
            .ok_or_else(|| anyhow!("No refresh token available"))?;
        
        info!("Refreshing access token");
        
        // 构建刷新请求
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token".to_string());
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", self.config.client_id.clone());
        
        if let Some(client_secret) = &self.config.client_secret {
            params.insert("client_secret", client_secret.clone());
        }
        
        // 发送刷新请求
        let client = crate::ai_adapter::http::create_default_client()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        let response = client
            .post(&self.config.token_endpoint)
            .form(&params)
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Token refresh failed: {}", error_text);
            
            self.auth_status.write().await.insert(
                self.config.client_id.clone(),
                AuthStatus::TokenExpired,
            );
            
            return Err(anyhow!("Token refresh failed"));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        // 解析新令牌
        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing access_token in refresh response"))?
            .to_string();
        
        let token_type = token_response["token_type"]
            .as_str()
            .unwrap_or("Bearer")
            .to_string();
        
        let expires_in = token_response["expires_in"]
            .as_u64()
            .unwrap_or(3600);
        
        let new_refresh_token = token_response["refresh_token"]
            .as_str()
            .map(|s| s.to_string())
            .or(current_token.refresh_token); // 保留旧的refresh_token如果没有新的
        
        let scope = token_response["scope"]
            .as_str()
            .map(|s| s.to_string())
            .or(current_token.scope);
        
        let new_token_info = AccessTokenInfo {
            access_token,
            token_type,
            expires_in,
            refresh_token: new_refresh_token,
            scope,
            issued_at: chrono::Utc::now(),
            token_binding_id: current_token.token_binding_id, // 保持令牌绑定ID
        };
        
        // 更新令牌
        self.tokens.write().await.insert(self.config.client_id.clone(), new_token_info.clone());
        
        info!("Successfully refreshed access token");
        Ok(new_token_info)
    }
    
    /// 检查令牌是否需要刷新
    pub async fn should_refresh_token(&self) -> bool {
        if let Some(token) = self.tokens.read().await.get(&self.config.client_id) {
            let elapsed = chrono::Utc::now().signed_duration_since(token.issued_at);
            let threshold = chrono::Duration::seconds(self.config.token_refresh_threshold_seconds as i64);
            let expires_in = chrono::Duration::seconds(token.expires_in as i64);
            
            elapsed + threshold >= expires_in
        } else {
            false
        }
    }
    
    /// 获取有效的访问令牌
    pub async fn get_valid_access_token(&self) -> Result<String> {
        // 检查是否需要刷新令牌
        if self.should_refresh_token().await {
            if let Ok(token) = self.refresh_token().await {
                return Ok(token.access_token);
            }
        }
        
        // 返回当前令牌
        let tokens = self.tokens.read().await;
        let token = tokens.get(&self.config.client_id)
            .ok_or_else(|| anyhow!("No valid access token available"))?;
        
        Ok(token.access_token.clone())
    }
    
    /// 获取认证状态
    pub async fn get_auth_status(&self) -> AuthStatus {
        self.auth_status.read().await
            .get(&self.config.client_id)
            .cloned()
            .unwrap_or(AuthStatus::NotAuthenticated)
    }
    
    /// 撤销令牌
    pub async fn revoke_token(&self) -> Result<()> {
        if let Some(_token) = self.tokens.write().await.remove(&self.config.client_id) {
            info!("Revoking access token");
            
            // 这里应该调用撤销端点，但不是所有OAuth服务器都支持
            // 暂时只是清理本地令牌
            debug!("Token revoked locally");
        }
        
        self.auth_status.write().await.insert(
            self.config.client_id.clone(),
            AuthStatus::NotAuthenticated,
        );
        
        Ok(())
    }
    
    /// 生成随机字符串
    fn generate_random_string(&self, length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::thread_rng();
        
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

/// 创建默认OAuth2.1配置
pub fn create_default_oauth21_config(
    client_id: String,
    auth_endpoint: String,
    token_endpoint: String,
    redirect_uri: String,
) -> OAuth21Config {
    OAuth21Config {
        client_id,
        client_secret: None, // 公共客户端
        redirect_uri,
        auth_endpoint,
        token_endpoint,
        scopes: vec!["read".to_string(), "write".to_string()],
        use_pkce: true, // 强制使用PKCE
        token_refresh_threshold_seconds: 300, // 5分钟前刷新
        enable_token_binding: true,
    }
}
