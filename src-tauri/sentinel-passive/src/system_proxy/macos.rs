//! macOS 系统代理管理
//!
//! 提供 macOS 系统级代理配置功能：
//! - 设置/清除系统 HTTP/HTTPS/SOCKS 代理
//! - 获取系统网络服务列表

use std::process::Command;
use tracing::{info, warn, debug};
use super::{SystemProxyConfig, ProxyType, SystemProxyStatus};

/// 获取所有网络服务名称
pub fn get_network_services() -> Result<Vec<String>, String> {
    let output = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .map_err(|e| format!("Failed to execute networksetup: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "networksetup failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let services: Vec<String> = stdout
        .lines()
        .skip(1)
        .filter(|line| !line.starts_with('*') && !line.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    Ok(services)
}

/// 获取活动的网络服务（有 IP 地址的）
pub fn get_active_network_services() -> Result<Vec<String>, String> {
    let all_services = get_network_services()?;
    let mut active = Vec::new();

    for service in all_services.iter() {
        let output = Command::new("networksetup")
            .args(["-getinfo", service])
            .output()
            .map_err(|e| format!("Failed to get info for {}: {}", service, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("IP address:") && !stdout.contains("IP address: none") {
            active.push(service.clone());
        }
    }

    if active.is_empty() {
        for service in ["Wi-Fi", "Ethernet", "USB 10/100/1000 LAN"] {
            if all_services.iter().any(|s| s == service) {
                active.push(service.to_string());
            }
        }
    }

    Ok(active)
}

/// 设置 HTTP 代理
pub fn set_http_proxy(service: &str, host: &str, port: u16) -> Result<(), String> {
    info!("Setting HTTP proxy for {} to {}:{}", service, host, port);

    let output = Command::new("networksetup")
        .args(["-setwebproxy", service, host, &port.to_string()])
        .output()
        .map_err(|e| format!("Failed to set web proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to set web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output = Command::new("networksetup")
        .args(["-setwebproxystate", service, "on"])
        .output()
        .map_err(|e| format!("Failed to enable web proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to enable web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// 设置 HTTPS 代理
pub fn set_https_proxy(service: &str, host: &str, port: u16) -> Result<(), String> {
    info!("Setting HTTPS proxy for {} to {}:{}", service, host, port);

    let output = Command::new("networksetup")
        .args(["-setsecurewebproxy", service, host, &port.to_string()])
        .output()
        .map_err(|e| format!("Failed to set secure web proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to set secure web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output = Command::new("networksetup")
        .args(["-setsecurewebproxystate", service, "on"])
        .output()
        .map_err(|e| format!("Failed to enable secure web proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to enable secure web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// 设置 SOCKS 代理
pub fn set_socks_proxy(service: &str, host: &str, port: u16) -> Result<(), String> {
    info!("Setting SOCKS proxy for {} to {}:{}", service, host, port);

    let output = Command::new("networksetup")
        .args(["-setsocksfirewallproxy", service, host, &port.to_string()])
        .output()
        .map_err(|e| format!("Failed to set SOCKS proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to set SOCKS proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output = Command::new("networksetup")
        .args(["-setsocksfirewallproxystate", service, "on"])
        .output()
        .map_err(|e| format!("Failed to enable SOCKS proxy: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to enable SOCKS proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// 清除 HTTP 代理
pub fn clear_http_proxy(service: &str) -> Result<(), String> {
    info!("Clearing HTTP proxy for {}", service);

    let output = Command::new("networksetup")
        .args(["-setwebproxystate", service, "off"])
        .output()
        .map_err(|e| format!("Failed to disable web proxy: {}", e))?;

    if !output.status.success() {
        warn!(
            "Failed to disable web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// 清除 HTTPS 代理
pub fn clear_https_proxy(service: &str) -> Result<(), String> {
    info!("Clearing HTTPS proxy for {}", service);

    let output = Command::new("networksetup")
        .args(["-setsecurewebproxystate", service, "off"])
        .output()
        .map_err(|e| format!("Failed to disable secure web proxy: {}", e))?;

    if !output.status.success() {
        warn!(
            "Failed to disable secure web proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// 清除 SOCKS 代理
pub fn clear_socks_proxy(service: &str) -> Result<(), String> {
    info!("Clearing SOCKS proxy for {}", service);

    let output = Command::new("networksetup")
        .args(["-setsocksfirewallproxystate", service, "off"])
        .output()
        .map_err(|e| format!("Failed to disable SOCKS proxy: {}", e))?;

    if !output.status.success() {
        warn!(
            "Failed to disable SOCKS proxy: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// 为所有活动网络服务设置代理
pub fn set_system_proxy(config: &SystemProxyConfig) -> Result<(), String> {
    let services = get_active_network_services()?;
    
    if services.is_empty() {
        return Err("No active network services found".to_string());
    }

    info!("Setting system proxy for services: {:?}", services);

    for service in &services {
        match config.proxy_type {
            ProxyType::Http => {
                set_http_proxy(service, &config.host, config.port)?;
            }
            ProxyType::Https => {
                set_https_proxy(service, &config.host, config.port)?;
            }
            ProxyType::Socks => {
                set_socks_proxy(service, &config.host, config.port)?;
            }
        }
    }

    Ok(())
}

/// 为所有活动网络服务设置 HTTP 和 HTTPS 代理
pub fn set_system_http_https_proxy(host: &str, port: u16) -> Result<(), String> {
    let services = get_active_network_services()?;
    
    if services.is_empty() {
        return Err("No active network services found".to_string());
    }

    info!("Setting HTTP/HTTPS proxy for services: {:?}", services);

    for service in &services {
        set_http_proxy(service, host, port)?;
        set_https_proxy(service, host, port)?;
    }

    Ok(())
}

/// 清除所有活动网络服务的代理
pub fn clear_system_proxy() -> Result<(), String> {
    let services = get_active_network_services()?;
    
    info!("Clearing system proxy for services: {:?}", services);

    for service in &services {
        let _ = clear_http_proxy(service);
        let _ = clear_https_proxy(service);
        let _ = clear_socks_proxy(service);
    }

    Ok(())
}

/// 获取指定服务的代理状态
pub fn get_proxy_status(service: &str) -> Result<SystemProxyStatus, String> {
    // HTTP
    let http_output = Command::new("networksetup")
        .args(["-getwebproxy", service])
        .output()
        .map_err(|e| format!("Failed to get web proxy: {}", e))?;
    
    let http_info = String::from_utf8_lossy(&http_output.stdout);
    let http_enabled = http_info.contains("Enabled: Yes");
    let http_host = parse_proxy_field(&http_info, "Server:");
    let http_port = parse_proxy_port(&http_info, "Port:");

    // HTTPS
    let https_output = Command::new("networksetup")
        .args(["-getsecurewebproxy", service])
        .output()
        .map_err(|e| format!("Failed to get secure web proxy: {}", e))?;
    
    let https_info = String::from_utf8_lossy(&https_output.stdout);
    let https_enabled = https_info.contains("Enabled: Yes");
    let https_host = parse_proxy_field(&https_info, "Server:");
    let https_port = parse_proxy_port(&https_info, "Port:");

    // SOCKS
    let socks_output = Command::new("networksetup")
        .args(["-getsocksfirewallproxy", service])
        .output()
        .map_err(|e| format!("Failed to get SOCKS proxy: {}", e))?;
    
    let socks_info = String::from_utf8_lossy(&socks_output.stdout);
    let socks_enabled = socks_info.contains("Enabled: Yes");
    let socks_host = parse_proxy_field(&socks_info, "Server:");
    let socks_port = parse_proxy_port(&socks_info, "Port:");

    Ok(SystemProxyStatus {
        http_enabled,
        http_host,
        http_port,
        https_enabled,
        https_host,
        https_port,
        socks_enabled,
        socks_host,
        socks_port,
        network_services: vec![service.to_string()],
    })
}

fn parse_proxy_field(info: &str, field: &str) -> Option<String> {
    for line in info.lines() {
        if line.starts_with(field) {
            let value = line.trim_start_matches(field).trim();
            if !value.is_empty() && value != "(null)" {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_proxy_port(info: &str, field: &str) -> Option<u16> {
    for line in info.lines() {
        if line.starts_with(field) {
            let value = line.trim_start_matches(field).trim();
            if let Ok(port) = value.parse::<u16>() {
                if port > 0 {
                    return Some(port);
                }
            }
        }
    }
    None
}

/// 设置代理绕过列表
pub fn set_proxy_bypass(service: &str, bypass_domains: &[&str]) -> Result<(), String> {
    let domains = bypass_domains.join(" ");
    
    let output = Command::new("networksetup")
        .args(["-setproxybypassdomains", service, &domains])
        .output()
        .map_err(|e| format!("Failed to set proxy bypass domains: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to set proxy bypass domains: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// 设置默认的代理绕过列表（本地地址）
pub fn set_default_proxy_bypass(service: &str) -> Result<(), String> {
    let default_bypass = [
        "*.local",
        "169.254/16",
    ];
    
    set_proxy_bypass(service, &default_bypass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_services() {
        let services = get_network_services();
        assert!(services.is_ok());
        println!("Network services: {:?}", services.unwrap());
    }

    #[test]
    fn test_get_active_network_services() {
        let services = get_active_network_services();
        assert!(services.is_ok());
        println!("Active network services: {:?}", services.unwrap());
    }

    #[test]
    fn test_get_proxy_status() {
        let services = get_active_network_services().unwrap();
        if let Some(service) = services.first() {
            let status = get_proxy_status(service);
            assert!(status.is_ok());
            println!("Proxy status for {}: {:?}", service, status.unwrap());
        }
    }
}

