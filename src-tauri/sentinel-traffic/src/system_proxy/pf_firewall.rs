//! macOS pf (Packet Filter) 防火墙管理
//!
//! 使用 pf 实现透明代理，将特定流量重定向到本地代理服务器

use std::fs;
use std::process::Command;
use tracing::{info, warn, debug};

/// pf 规则配置
pub struct PfConfig {
    /// 代理监听端口
    pub proxy_port: u16,
    /// 要重定向的端口列表
    pub redirect_ports: Vec<u16>,
    /// 配置文件路径
    pub config_path: String,
    /// 锚点名称
    pub anchor_name: String,
}

impl Default for PfConfig {
    fn default() -> Self {
        Self {
            proxy_port: 8080,
            redirect_ports: vec![80, 443],
            config_path: "/tmp/sentinel-pf.conf".to_string(),
            anchor_name: "sentinel-proxy".to_string(),
        }
    }
}

impl PfConfig {
    pub fn new(proxy_port: u16) -> Self {
        Self {
            proxy_port,
            ..Default::default()
        }
    }

    /// 生成 pf 重定向规则
    /// 
    /// 注意：pf 的 rdr 规则无法直接拦截本机发出的出站流量
    /// 这些规则主要用于拦截经过本机的转发流量（如作为网关时）
    pub fn generate_rules(&self) -> String {
        let mut rules = String::new();
        
        rules.push_str("# Sentinel AI Transparent Proxy Rules\n");
        rules.push_str(&format!("# Anchor: {}\n", self.anchor_name));
        rules.push_str(&format!("# Proxy port: {}\n", self.proxy_port));
        rules.push_str(&format!("# Redirect ports: {:?}\n\n", self.redirect_ports));

        // 获取所有常见网络接口
        let interfaces = ["lo0", "en0", "en1", "en2", "bridge0", "utun0", "utun1"];
        
        for iface in interfaces {
            for port in &self.redirect_ports {
                rules.push_str(&format!(
                    "rdr pass on {} inet proto tcp from any to any port {} -> 127.0.0.1 port {}\n",
                    iface, port, self.proxy_port
                ));
            }
        }

        rules.push_str("\n# Pass all redirected traffic\n");
        rules.push_str(&format!("pass out quick proto tcp to any port {}\n", self.proxy_port));
        rules.push_str("pass in quick proto tcp to any port 80\n");
        rules.push_str("pass in quick proto tcp to any port 443\n");

        rules
    }
}

/// 检查 pf 是否启用
pub fn is_pf_enabled() -> bool {
    let output = Command::new("pfctl")
        .args(["-s", "info"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("Status: Enabled")
        }
        Err(_) => false,
    }
}

/// 启用 pf 防火墙
pub fn enable_pf() -> Result<(), String> {
    info!("Enabling pf firewall");

    let output = Command::new("pfctl")
        .args(["-e"])
        .output()
        .map_err(|e| format!("Failed to enable pf: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && !stderr.contains("already enabled") {
        return Err(format!("Failed to enable pf: {}", stderr));
    }

    Ok(())
}

/// 禁用 pf 防火墙
pub fn disable_pf() -> Result<(), String> {
    info!("Disabling pf firewall");

    let output = Command::new("pfctl")
        .args(["-d"])
        .output()
        .map_err(|e| format!("Failed to disable pf: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("already disabled") {
            warn!("pfctl disable warning: {}", stderr);
        }
    }

    Ok(())
}

/// 加载 pf 规则
pub fn load_pf_rules(config_path: &str) -> Result<(), String> {
    info!("Loading pf rules from {}", config_path);

    let output = Command::new("pfctl")
        .args(["-f", config_path])
        .output()
        .map_err(|e| format!("Failed to load pf rules: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to load pf rules: {}", stderr));
    }

    Ok(())
}

/// 加载 pf 锚点规则
pub fn load_anchor_rules(anchor_name: &str, config_path: &str) -> Result<(), String> {
    info!("Loading pf anchor {} from {}", anchor_name, config_path);

    let output = Command::new("pfctl")
        .args(["-a", anchor_name, "-f", config_path])
        .output()
        .map_err(|e| format!("Failed to load anchor rules: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to load anchor rules: {}", stderr));
    }

    Ok(())
}

/// 清除 pf 锚点规则
pub fn flush_anchor_rules(anchor_name: &str) -> Result<(), String> {
    info!("Flushing pf anchor {}", anchor_name);

    let output = Command::new("pfctl")
        .args(["-a", anchor_name, "-F", "all"])
        .output()
        .map_err(|e| format!("Failed to flush anchor rules: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning while flushing anchor: {}", stderr);
    }

    Ok(())
}

/// 获取当前 pf 规则
pub fn get_pf_rules() -> Result<String, String> {
    let output = Command::new("pfctl")
        .args(["-s", "rules"])
        .output()
        .map_err(|e| format!("Failed to get pf rules: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 获取锚点规则
pub fn get_anchor_rules(anchor_name: &str) -> Result<String, String> {
    let output = Command::new("pfctl")
        .args(["-a", anchor_name, "-s", "rules"])
        .output()
        .map_err(|e| format!("Failed to get anchor rules: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ============================================================================
// macOS 系统代理设置
// ============================================================================

/// 获取当前活动的网络服务名称
fn get_active_network_service() -> Result<String, String> {
    // 获取默认路由的网络接口
    let output = Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .map_err(|e| format!("Failed to get default route: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let interface = stdout.lines()
        .find(|line| line.contains("interface:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Could not find default interface".to_string())?;
    
    // 根据接口名获取网络服务名称
    let output = Command::new("networksetup")
        .args(["-listallhardwareports"])
        .output()
        .map_err(|e| format!("Failed to list hardware ports: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("Device: {}", interface)) {
            // 前一行应该是 "Hardware Port: xxx"
            if i > 0 {
                if let Some(service) = lines[i - 1].strip_prefix("Hardware Port: ") {
                    return Ok(service.to_string());
                }
            }
        }
    }
    
    // 如果找不到，默认使用 "Wi-Fi"
    Ok("Wi-Fi".to_string())
}

/// 设置系统 HTTP 代理
pub fn set_system_http_proxy(host: &str, port: u16) -> Result<(), String> {
    let service = get_active_network_service()?;
    info!("Setting HTTP proxy on service '{}': {}:{}", service, host, port);
    
    let output = Command::new("networksetup")
        .args(["-setwebproxy", &service, host, &port.to_string()])
        .output()
        .map_err(|e| format!("Failed to set HTTP proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set HTTP proxy: {}", stderr));
    }
    
    // 启用代理
    let output = Command::new("networksetup")
        .args(["-setwebproxystate", &service, "on"])
        .output()
        .map_err(|e| format!("Failed to enable HTTP proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning enabling HTTP proxy: {}", stderr);
    }
    
    Ok(())
}

/// 设置系统 HTTPS 代理
pub fn set_system_https_proxy(host: &str, port: u16) -> Result<(), String> {
    let service = get_active_network_service()?;
    info!("Setting HTTPS proxy on service '{}': {}:{}", service, host, port);
    
    let output = Command::new("networksetup")
        .args(["-setsecurewebproxy", &service, host, &port.to_string()])
        .output()
        .map_err(|e| format!("Failed to set HTTPS proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set HTTPS proxy: {}", stderr));
    }
    
    // 启用代理
    let output = Command::new("networksetup")
        .args(["-setsecurewebproxystate", &service, "on"])
        .output()
        .map_err(|e| format!("Failed to enable HTTPS proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning enabling HTTPS proxy: {}", stderr);
    }
    
    Ok(())
}

/// 清除系统 HTTP 代理
pub fn clear_system_http_proxy() -> Result<(), String> {
    let service = get_active_network_service()?;
    info!("Clearing HTTP proxy on service '{}'", service);
    
    let output = Command::new("networksetup")
        .args(["-setwebproxystate", &service, "off"])
        .output()
        .map_err(|e| format!("Failed to disable HTTP proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning disabling HTTP proxy: {}", stderr);
    }
    
    Ok(())
}

/// 清除系统 HTTPS 代理
pub fn clear_system_https_proxy() -> Result<(), String> {
    let service = get_active_network_service()?;
    info!("Clearing HTTPS proxy on service '{}'", service);
    
    let output = Command::new("networksetup")
        .args(["-setsecurewebproxystate", &service, "off"])
        .output()
        .map_err(|e| format!("Failed to disable HTTPS proxy: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning disabling HTTPS proxy: {}", stderr);
    }
    
    Ok(())
}

// ============================================================================
// 透明代理管理器
// ============================================================================

/// 透明代理管理器
/// 
/// 使用两种方式实现透明代理：
/// 1. 系统代理设置（主要方式）- 通过 networksetup 设置系统 HTTP/HTTPS 代理
/// 2. pf 防火墙规则（辅助方式）- 用于拦截不走系统代理的流量
pub struct TransparentProxyManager {
    config: PfConfig,
    enabled: bool,
    /// 是否设置了系统代理
    system_proxy_enabled: bool,
}

impl TransparentProxyManager {
    pub fn new(proxy_port: u16) -> Self {
        Self {
            config: PfConfig::new(proxy_port),
            enabled: false,
            system_proxy_enabled: false,
        }
    }

    pub fn with_ports(proxy_port: u16, redirect_ports: Vec<u16>) -> Self {
        let mut config = PfConfig::new(proxy_port);
        config.redirect_ports = redirect_ports;
        Self {
            config,
            enabled: false,
            system_proxy_enabled: false,
        }
    }

    /// 启动透明代理
    pub fn start(&mut self) -> Result<(), String> {
        if self.enabled {
            return Ok(());
        }

        info!("Starting transparent proxy on port {}", self.config.proxy_port);

        // 1. 设置系统代理（这是主要的代理方式）
        let proxy_host = "127.0.0.1";
        let proxy_port = self.config.proxy_port;
        
        if let Err(e) = set_system_http_proxy(proxy_host, proxy_port) {
            warn!("Failed to set system HTTP proxy: {}", e);
        } else {
            info!("System HTTP proxy set to {}:{}", proxy_host, proxy_port);
        }
        
        if let Err(e) = set_system_https_proxy(proxy_host, proxy_port) {
            warn!("Failed to set system HTTPS proxy: {}", e);
        } else {
            info!("System HTTPS proxy set to {}:{}", proxy_host, proxy_port);
            self.system_proxy_enabled = true;
        }

        // 2. 设置 pf 规则（辅助方式，用于拦截不走系统代理的流量）
        let rules = self.config.generate_rules();
        debug!("Generated pf rules:\n{}", rules);

        fs::write(&self.config.config_path, &rules)
            .map_err(|e| format!("Failed to write pf config: {}", e))?;

        enable_pf()?;
        load_anchor_rules(&self.config.anchor_name, &self.config.config_path)?;

        self.enabled = true;
        info!("Transparent proxy started (system proxy + pf rules)");
        Ok(())
    }

    /// 停止透明代理
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        info!("Stopping transparent proxy");

        // 1. 清除系统代理
        if self.system_proxy_enabled {
            if let Err(e) = clear_system_http_proxy() {
                warn!("Failed to clear system HTTP proxy: {}", e);
            }
            if let Err(e) = clear_system_https_proxy() {
                warn!("Failed to clear system HTTPS proxy: {}", e);
            }
            self.system_proxy_enabled = false;
            info!("System proxy cleared");
        }

        // 2. 清除 pf 规则
        flush_anchor_rules(&self.config.anchor_name)?;
        let _ = fs::remove_file(&self.config.config_path);

        self.enabled = false;
        info!("Transparent proxy stopped");
        Ok(())
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 添加重定向端口
    pub fn add_redirect_port(&mut self, port: u16) -> Result<(), String> {
        if !self.config.redirect_ports.contains(&port) {
            self.config.redirect_ports.push(port);
            if self.enabled {
                let rules = self.config.generate_rules();
                fs::write(&self.config.config_path, &rules)
                    .map_err(|e| format!("Failed to write pf config: {}", e))?;
                load_anchor_rules(&self.config.anchor_name, &self.config.config_path)?;
            }
        }
        Ok(())
    }

    /// 移除重定向端口
    pub fn remove_redirect_port(&mut self, port: u16) -> Result<(), String> {
        self.config.redirect_ports.retain(|&p| p != port);
        if self.enabled {
            let rules = self.config.generate_rules();
            fs::write(&self.config.config_path, &rules)
                .map_err(|e| format!("Failed to write pf config: {}", e))?;
            load_anchor_rules(&self.config.anchor_name, &self.config.config_path)?;
        }
        Ok(())
    }
}

impl Drop for TransparentProxyManager {
    fn drop(&mut self) {
        if self.enabled {
            let _ = self.stop();
        }
    }
}

/// 应用程序级代理过滤器
pub struct AppProxyFilter {
    /// 要代理的应用程序名称或 bundle ID
    pub apps: Vec<String>,
    /// 排除的应用程序
    pub excluded_apps: Vec<String>,
}

impl Default for AppProxyFilter {
    fn default() -> Self {
        Self {
            apps: Vec::new(),
            excluded_apps: vec![
                "Finder".to_string(),
                "System Preferences".to_string(),
                "Spotlight".to_string(),
            ],
        }
    }
}

impl AppProxyFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_app(&mut self, app: &str) {
        if !self.apps.contains(&app.to_string()) {
            self.apps.push(app.to_string());
        }
    }

    pub fn remove_app(&mut self, app: &str) {
        self.apps.retain(|a| a != app);
    }

    pub fn should_proxy(&self, app_name: &str) -> bool {
        if self.excluded_apps.iter().any(|e| app_name.contains(e)) {
            return false;
        }
        
        if self.apps.is_empty() {
            return true;
        }
        
        self.apps.iter().any(|a| app_name.contains(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pf_config_generate_rules() {
        let config = PfConfig::default();
        let rules = config.generate_rules();
        println!("Generated rules:\n{}", rules);
        assert!(rules.contains("rdr pass"));
    }

    #[test]
    fn test_is_pf_enabled() {
        let enabled = is_pf_enabled();
        println!("pf enabled: {}", enabled);
    }

    #[test]
    fn test_app_proxy_filter() {
        let mut filter = AppProxyFilter::new();
        filter.add_app("Safari");
        filter.add_app("Chrome");
        
        assert!(filter.should_proxy("Safari"));
        assert!(filter.should_proxy("Google Chrome"));
        assert!(!filter.should_proxy("Firefox"));
        assert!(!filter.should_proxy("Finder"));
    }
}

