use sentinel_bounty::services::{ChangeMonitorConfig, MonitorPluginConfig, MonitorTask};

pub fn normalize_loaded_monitor_task(mut task: MonitorTask) -> MonitorTask {
    task.config.migrate_legacy_port_service_plugins();
    task
}

pub fn normalize_monitor_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dns" => Some("dns"),
        "ip" => Some("ip"),
        "cert" => Some("cert"),
        "content" => Some("content"),
        "api" => Some("api"),
        "port" => Some("port"),
        "service" => Some("service"),
        "web" => Some("web"),
        "risk" => Some("risk"),
        _ => None,
    }
}

pub fn infer_monitor_type_for_plugin(
    normalized_name: &str,
    category: &str,
) -> Option<&'static str> {
    match normalized_name {
        "subdomain_enumerator" | "subdomain_brute" => Some("dns"),
        "dns_resolver" => Some("ip"),
        "cert_monitor" | "ssl_scanner" => Some("cert"),
        "content_monitor" => Some("content"),
        "api_monitor" | "js_analyzer" | "js_link_finder" => Some("api"),
        "cidr_mapper" => Some("ip"),
        "port_monitor" => Some("port"),
        "service_monitor" | "service_probe" => Some("service"),
        "http_prober" | "tech_fingerprinter" | "favicon_fingerprinter" => Some("web"),
        "sensitive_file_scanner" | "risk_scanner" => Some("risk"),
        _ => match category.to_lowercase().as_str() {
            "monitor" | "recon" | "reconnaissance" if normalized_name.contains("subdomain") => {
                Some("dns")
            }
            "monitor" | "recon" | "reconnaissance" if normalized_name.contains("resolver") => {
                Some("ip")
            }
            "monitor" if normalized_name.contains("cert") || normalized_name.contains("ssl") => {
                Some("cert")
            }
            "monitor" if normalized_name.contains("content") => Some("content"),
            "monitor" if normalized_name.contains("api") || normalized_name.contains("js") => {
                Some("api")
            }
            "monitor" | "recon"
                if normalized_name.contains("port") || normalized_name.contains("cidr") =>
            {
                Some("port")
            }
            "monitor" | "recon"
                if normalized_name.contains("service")
                    || normalized_name.contains("banner")
                    || normalized_name.contains("fingerprinter") =>
            {
                Some("service")
            }
            "monitor" | "recon"
                if normalized_name.contains("web")
                    || normalized_name.contains("tech")
                    || normalized_name.contains("http") =>
            {
                Some("web")
            }
            "monitor" | "scanner" | "scan"
                if normalized_name.contains("risk")
                    || normalized_name.contains("vuln")
                    || normalized_name.contains("sensitive") =>
            {
                Some("risk")
            }
            _ => None,
        },
    }
}

pub fn apply_plugins_to_monitor_type(
    config: &mut ChangeMonitorConfig,
    monitor_type: &str,
    plugins: Vec<MonitorPluginConfig>,
) {
    match monitor_type {
        "dns" => config.dns_plugins = plugins,
        "ip" => config.ip_plugins = plugins,
        "cert" => config.cert_plugins = plugins,
        "content" => config.content_plugins = plugins,
        "api" => config.api_plugins = plugins,
        "port" => config.port_plugins = plugins,
        "service" => config.service_plugins = plugins,
        "web" => config.web_plugins = plugins,
        "risk" => config.risk_plugins = plugins,
        _ => {}
    }
}
