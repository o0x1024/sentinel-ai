//! Proxifier 相关命令
//!
//! 提供类似 Proxifier 的功能：
//! - 管理代理服务器列表
//! - 管理应用程序代理规则
//! - 跟踪连接
//! - pf 透明代理
//! - 数据库持久化

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use tracing::{info, error};
use sentinel_db::database::proxifier_dao::{self, ProxifierProxyRecord, ProxifierRuleRecord};
use sentinel_db::DatabaseService;

#[cfg(target_os = "macos")]
use sentinel_passive::system_proxy::pf_firewall::TransparentProxyManager;

/// 代理服务器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxifierProxy {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(rename = "type")]
    pub proxy_type: String,  // HTTP, HTTPS, SOCKS5
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: bool,
}

/// 代理规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxifierRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub applications: String,  // ; 分隔
    #[serde(rename = "targetHosts")]
    pub target_hosts: String,  // ; 分隔
    #[serde(rename = "targetPorts")]
    pub target_ports: String,  // ; 分隔
    pub action: String,        // Direct, Block, 或 Proxy 名称
}

/// 连接记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxifierConnection {
    pub id: String,
    pub application: String,
    pub target: String,
    #[serde(rename = "timeOrStatus")]
    pub time_or_status: String,
    pub status: String,  // open, closed, error
    pub rule: String,
    pub proxy: String,
    pub sent: u64,
    pub received: u64,
}

/// Proxifier 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxifierConfig {
    pub enabled: bool,
    pub proxies: Vec<ProxifierProxy>,
    pub rules: Vec<ProxifierRule>,
}

/// pf 透明代理状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransparentProxyStatus {
    pub enabled: bool,
    pub proxy_port: u16,
    pub redirect_ports: Vec<u16>,
    pub pf_enabled: bool,
}

/// Proxifier 状态
pub struct ProxifierState {
    config: Arc<RwLock<ProxifierConfig>>,
    connections: Arc<RwLock<Vec<ProxifierConnection>>>,
    #[cfg(target_os = "macos")]
    transparent_proxy: Arc<RwLock<Option<TransparentProxyManager>>>,
    transparent_status: Arc<RwLock<TransparentProxyStatus>>,
}

impl Default for ProxifierState {
    fn default() -> Self {
        Self {
            config: Arc::new(RwLock::new(ProxifierConfig::default())),
            connections: Arc::new(RwLock::new(Vec::new())),
            #[cfg(target_os = "macos")]
            transparent_proxy: Arc::new(RwLock::new(None)),
            transparent_status: Arc::new(RwLock::new(TransparentProxyStatus::default())),
        }
    }
}

impl ProxifierState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 命令响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: impl ToString) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// 获取 Proxifier 配置
#[tauri::command]
pub async fn get_proxifier_config(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<ProxifierConfig>, String> {
    let config = state.config.read().await;
    Ok(CommandResponse::ok(config.clone()))
}

/// 启动 Proxifier
#[tauri::command]
pub async fn start_proxifier(
    state: State<'_, ProxifierState>,
    proxies: Vec<ProxifierProxy>,
    rules: Vec<ProxifierRule>,
) -> Result<CommandResponse<()>, String> {
    let mut config = state.config.write().await;
    config.enabled = true;
    config.proxies = proxies;
    config.rules = rules;
    
    info!("Proxifier started with {} proxies and {} rules", 
          config.proxies.len(), config.rules.len());
    
    Ok(CommandResponse::ok(()))
}

/// 停止 Proxifier
#[tauri::command]
pub async fn stop_proxifier(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<()>, String> {
    let mut config = state.config.write().await;
    config.enabled = false;
    
    info!("Proxifier stopped");
    
    Ok(CommandResponse::ok(()))
}

/// 保存代理服务器列表
#[tauri::command]
pub async fn save_proxifier_proxies(
    state: State<'_, ProxifierState>,
    proxies: Vec<ProxifierProxy>,
) -> Result<CommandResponse<()>, String> {
    let mut config = state.config.write().await;
    config.proxies = proxies;
    
    info!("Saved {} proxifier proxies", config.proxies.len());
    
    Ok(CommandResponse::ok(()))
}

/// 保存代理规则
#[tauri::command]
pub async fn save_proxifier_rules(
    state: State<'_, ProxifierState>,
    rules: Vec<ProxifierRule>,
) -> Result<CommandResponse<()>, String> {
    let mut config = state.config.write().await;
    config.rules = rules;
    
    info!("Saved {} proxifier rules", config.rules.len());
    
    Ok(CommandResponse::ok(()))
}

/// 获取连接列表
#[tauri::command]
pub async fn get_proxifier_connections(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<Vec<ProxifierConnection>>, String> {
    let connections = state.connections.read().await;
    Ok(CommandResponse::ok(connections.clone()))
}

/// 清空连接列表
#[tauri::command]
pub async fn clear_proxifier_connections(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<()>, String> {
    let mut connections = state.connections.write().await;
    connections.clear();
    Ok(CommandResponse::ok(()))
}

/// 添加连接记录（内部使用）
pub async fn add_connection(state: &ProxifierState, connection: ProxifierConnection) {
    let mut connections = state.connections.write().await;
    connections.insert(0, connection);
    
    // 保持最多 1000 条连接记录
    if connections.len() > 1000 {
        connections.pop();
    }
}

/// 更新连接记录（内部使用）
pub async fn update_connection(state: &ProxifierState, connection: ProxifierConnection) {
    let mut connections = state.connections.write().await;
    if let Some(existing) = connections.iter_mut().find(|c| c.id == connection.id) {
        *existing = connection;
    }
}

// ============================================================================
// pf 透明代理相关命令
// ============================================================================

/// 获取透明代理状态
#[tauri::command]
pub async fn get_transparent_proxy_status(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<TransparentProxyStatus>, String> {
    let status = state.transparent_status.read().await;
    
    #[cfg(target_os = "macos")]
    {
        use sentinel_passive::system_proxy::pf_firewall::is_pf_enabled;
        let mut result = status.clone();
        result.pf_enabled = is_pf_enabled();
        return Ok(CommandResponse::ok(result));
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::ok(status.clone()))
    }
}

/// 启动透明代理
#[tauri::command]
pub async fn start_transparent_proxy(
    state: State<'_, ProxifierState>,
    proxy_port: u16,
    redirect_ports: Vec<u16>,
) -> Result<CommandResponse<()>, String> {
    #[cfg(target_os = "macos")]
    {
        let mut manager_lock = state.transparent_proxy.write().await;
        
        // 如果已经有一个管理器，先停止它
        if let Some(ref mut manager) = *manager_lock {
            let _ = manager.stop();
        }
        
        // 创建新的管理器
        let ports = if redirect_ports.is_empty() {
            vec![80, 443]
        } else {
            redirect_ports.clone()
        };
        
        let mut manager = TransparentProxyManager::with_ports(proxy_port, ports.clone());
        
        match manager.start() {
            Ok(()) => {
                info!("Transparent proxy started on port {}, redirecting ports: {:?}", proxy_port, ports);
                
                // 更新状态
                let mut status = state.transparent_status.write().await;
                status.enabled = true;
                status.proxy_port = proxy_port;
                status.redirect_ports = ports;
                status.pf_enabled = true;
                
                *manager_lock = Some(manager);
                Ok(CommandResponse::ok(()))
            }
            Err(e) => {
                error!("Failed to start transparent proxy: {}", e);
                Ok(CommandResponse::err(format!("启动透明代理失败: {}。\n请确保以管理员权限运行应用。", e)))
            }
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err("透明代理仅支持 macOS 平台"))
    }
}

/// 停止透明代理
#[tauri::command]
pub async fn stop_transparent_proxy(
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<()>, String> {
    #[cfg(target_os = "macos")]
    {
        let mut manager_lock = state.transparent_proxy.write().await;
        
        if let Some(ref mut manager) = *manager_lock {
            match manager.stop() {
                Ok(()) => {
                    info!("Transparent proxy stopped");
                    
                    // 更新状态
                    let mut status = state.transparent_status.write().await;
                    status.enabled = false;
                    
                    *manager_lock = None;
                    Ok(CommandResponse::ok(()))
                }
                Err(e) => {
                    error!("Failed to stop transparent proxy: {}", e);
                    Ok(CommandResponse::err(format!("停止透明代理失败: {}", e)))
                }
            }
        } else {
            // 更新状态
            let mut status = state.transparent_status.write().await;
            status.enabled = false;
            Ok(CommandResponse::ok(()))
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err("透明代理仅支持 macOS 平台"))
    }
}

/// 添加重定向端口
#[tauri::command]
pub async fn add_transparent_redirect_port(
    state: State<'_, ProxifierState>,
    port: u16,
) -> Result<CommandResponse<()>, String> {
    #[cfg(target_os = "macos")]
    {
        let mut manager_lock = state.transparent_proxy.write().await;
        
        if let Some(ref mut manager) = *manager_lock {
            match manager.add_redirect_port(port) {
                Ok(()) => {
                    let mut status = state.transparent_status.write().await;
                    if !status.redirect_ports.contains(&port) {
                        status.redirect_ports.push(port);
                    }
                    info!("Added redirect port: {}", port);
                    Ok(CommandResponse::ok(()))
                }
                Err(e) => {
                    error!("Failed to add redirect port: {}", e);
                    Ok(CommandResponse::err(e))
                }
            }
        } else {
            Ok(CommandResponse::err("透明代理未启动"))
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err("透明代理仅支持 macOS 平台"))
    }
}

/// 移除重定向端口
#[tauri::command]
pub async fn remove_transparent_redirect_port(
    state: State<'_, ProxifierState>,
    port: u16,
) -> Result<CommandResponse<()>, String> {
    #[cfg(target_os = "macos")]
    {
        let mut manager_lock = state.transparent_proxy.write().await;
        
        if let Some(ref mut manager) = *manager_lock {
            match manager.remove_redirect_port(port) {
                Ok(()) => {
                    let mut status = state.transparent_status.write().await;
                    status.redirect_ports.retain(|&p| p != port);
                    info!("Removed redirect port: {}", port);
                    Ok(CommandResponse::ok(()))
                }
                Err(e) => {
                    error!("Failed to remove redirect port: {}", e);
                    Ok(CommandResponse::err(e))
                }
            }
        } else {
            Ok(CommandResponse::err("透明代理未启动"))
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err("透明代理仅支持 macOS 平台"))
    }
}

// ============================================================================
// Network Extension 相关命令（暂时注释，需要 Apple 签名后启用）
// ============================================================================

/*
/// Network Extension 状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkExtensionStatus {
    pub extension_status: String,
    pub proxy_status: String,
    pub supported: bool,
}

/// 获取 Network Extension 状态
#[tauri::command]
pub async fn get_network_extension_status() -> Result<CommandResponse<NetworkExtensionStatus>, String> {
    use sentinel_passive::system_proxy::NetworkExtensionManager;
    
    let ext_status = NetworkExtensionManager::check_status();
    let proxy_status = NetworkExtensionManager::get_proxy_status();
    let available = NetworkExtensionManager::is_available();
    
    let status = NetworkExtensionStatus {
        extension_status: format!("{:?}", ext_status),
        proxy_status: format!("{:?}", proxy_status),
        supported: cfg!(target_os = "macos") && available,
    };
    
    Ok(CommandResponse::ok(status))
}

/// 安装 Network Extension
#[tauri::command]
pub async fn install_network_extension() -> Result<CommandResponse<()>, String> {
    use sentinel_passive::system_proxy::NetworkExtensionManager;
    
    match NetworkExtensionManager::install() {
        Ok(()) => {
            info!("Network Extension installed");
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to install Network Extension: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// 卸载 Network Extension
#[tauri::command]
pub async fn uninstall_network_extension() -> Result<CommandResponse<()>, String> {
    use sentinel_passive::system_proxy::NetworkExtensionManager;
    
    match NetworkExtensionManager::uninstall() {
        Ok(()) => {
            info!("Network Extension uninstalled");
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to uninstall Network Extension: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// 启动 Network Extension 代理
#[tauri::command]
pub async fn start_network_extension_proxy(
    host: String,
    port: u16,
    target_apps: Vec<String>,
) -> Result<CommandResponse<()>, String> {
    use sentinel_passive::system_proxy::NetworkExtensionManager;
    
    match NetworkExtensionManager::start_proxy(&host, port, &target_apps) {
        Ok(()) => {
            info!("Network Extension proxy started: {}:{}", host, port);
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to start Network Extension proxy: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// 停止 Network Extension 代理
#[tauri::command]
pub async fn stop_network_extension_proxy() -> Result<CommandResponse<()>, String> {
    use sentinel_passive::system_proxy::NetworkExtensionManager;
    
    match NetworkExtensionManager::stop_proxy() {
        Ok(()) => {
            info!("Network Extension proxy stopped");
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to stop Network Extension proxy: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}
*/

// ============================================================================
// 数据库持久化相关命令
// ============================================================================

/// 从数据库加载代理服务器列表
#[tauri::command]
pub async fn load_proxifier_proxies_from_db(
    db_service: State<'_, Arc<DatabaseService>>,
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<Vec<ProxifierProxy>>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    
    match proxifier_dao::get_all_proxies(pool).await {
        Ok(records) => {
            // 转换为前端格式
            let proxies: Vec<ProxifierProxy> = records.iter().map(|r| ProxifierProxy {
                id: r.id.clone(),
                name: r.name.clone(),
                host: r.host.clone(),
                port: r.port as u16,
                proxy_type: r.proxy_type.clone(),
                username: r.username.clone(),
                password: r.password.clone(),
                enabled: r.enabled,
            }).collect();
            
            // 更新内存状态
            let mut config = state.config.write().await;
            config.proxies = proxies.clone();
            
            info!("Loaded {} proxies from database", proxies.len());
            Ok(CommandResponse::ok(proxies))
        }
        Err(e) => {
            error!("Failed to load proxies from database: {}", e);
            Ok(CommandResponse::err(format!("加载代理服务器失败: {}", e)))
        }
    }
}

/// 保存代理服务器列表到数据库
#[tauri::command]
pub async fn save_proxifier_proxies_to_db(
    db_service: State<'_, Arc<DatabaseService>>,
    state: State<'_, ProxifierState>,
    proxies: Vec<ProxifierProxy>,
) -> Result<CommandResponse<()>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    
    // 转换为数据库格式
    let records: Vec<ProxifierProxyRecord> = proxies.iter().map(|p| ProxifierProxyRecord {
        id: p.id.clone(),
        name: p.name.clone(),
        host: p.host.clone(),
        port: p.port as i64,
        proxy_type: p.proxy_type.clone(),
        username: p.username.clone(),
        password: p.password.clone(),
        enabled: p.enabled,
        sort_order: 0,
        created_at: String::new(),
        updated_at: String::new(),
    }).collect();
    
    match proxifier_dao::save_all_proxies(pool, &records).await {
        Ok(()) => {
            // 更新内存状态
            let mut config = state.config.write().await;
            config.proxies = proxies;
            
            info!("Saved {} proxies to database", records.len());
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to save proxies to database: {}", e);
            Ok(CommandResponse::err(format!("保存代理服务器失败: {}", e)))
        }
    }
}

/// 从数据库加载代理规则列表
#[tauri::command]
pub async fn load_proxifier_rules_from_db(
    db_service: State<'_, Arc<DatabaseService>>,
    state: State<'_, ProxifierState>,
) -> Result<CommandResponse<Vec<ProxifierRule>>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    
    match proxifier_dao::get_all_rules(pool).await {
        Ok(records) => {
            // 转换为前端格式
            let rules: Vec<ProxifierRule> = records.iter().map(|r| ProxifierRule {
                id: r.id.clone(),
                name: r.name.clone(),
                enabled: r.enabled,
                applications: r.applications.clone(),
                target_hosts: r.target_hosts.clone(),
                target_ports: r.target_ports.clone(),
                action: r.action.clone(),
            }).collect();
            
            // 更新内存状态
            let mut config = state.config.write().await;
            config.rules = rules.clone();
            
            info!("Loaded {} rules from database", rules.len());
            Ok(CommandResponse::ok(rules))
        }
        Err(e) => {
            error!("Failed to load rules from database: {}", e);
            Ok(CommandResponse::err(format!("加载代理规则失败: {}", e)))
        }
    }
}

/// 保存代理规则列表到数据库
#[tauri::command]
pub async fn save_proxifier_rules_to_db(
    db_service: State<'_, Arc<DatabaseService>>,
    state: State<'_, ProxifierState>,
    rules: Vec<ProxifierRule>,
) -> Result<CommandResponse<()>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    
    // 转换为数据库格式
    let records: Vec<ProxifierRuleRecord> = rules.iter().map(|r| ProxifierRuleRecord {
        id: r.id.clone(),
        name: r.name.clone(),
        enabled: r.enabled,
        applications: r.applications.clone(),
        target_hosts: r.target_hosts.clone(),
        target_ports: r.target_ports.clone(),
        action: r.action.clone(),
        proxy_id: None,
        sort_order: 0,
        created_at: String::new(),
        updated_at: String::new(),
    }).collect();
    
    match proxifier_dao::save_all_rules(pool, &records).await {
        Ok(()) => {
            // 更新内存状态
            let mut config = state.config.write().await;
            config.rules = rules;
            
            info!("Saved {} rules to database", records.len());
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to save rules to database: {}", e);
            Ok(CommandResponse::err(format!("保存代理规则失败: {}", e)))
        }
    }
}
