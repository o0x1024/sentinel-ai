use crate::service_probe_runtime::{ServiceProbeResult, ServiceProbeTarget};

pub(crate) const MAX_BANNER_BYTES: usize = 512;

#[derive(Debug, Clone)]
pub(crate) struct ProductInfo {
    pub product_name: Option<String>,
    pub vendor: Option<String>,
    pub version: Option<String>,
}

pub(crate) fn invalid_target_result(
    target: ServiceProbeTarget,
    message: &str,
) -> ServiceProbeResult {
    ServiceProbeResult {
        target: service_key(&target.host, target.port),
        success: false,
        available: false,
        host: target.host,
        port: target.port,
        protocol: target.protocol,
        service_name: None,
        product_name: None,
        vendor: None,
        version: None,
        banner: None,
        server_header: None,
        title: None,
        status_code: None,
        confidence: None,
        matched_rule_id: None,
        error: Some(message.to_string()),
    }
}

pub fn service_key(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

pub fn normalize_protocol(protocol: &str, port: u16) -> String {
    let normalized = protocol.trim().to_lowercase();
    if !normalized.is_empty() {
        return normalized;
    }

    if is_tls_port(port) {
        "https".to_string()
    } else if is_http_port(port) {
        "http".to_string()
    } else {
        "tcp".to_string()
    }
}

pub(crate) fn is_http_port(port: u16) -> bool {
    matches!(
        port,
        80 | 81 | 443 | 8000 | 8080 | 8081 | 8443 | 8888 | 9000
    )
}

pub(crate) fn is_tls_port(port: u16) -> bool {
    matches!(port, 443 | 8443 | 9443)
}

pub(crate) fn is_http_target(port: u16, protocol: &str) -> bool {
    matches!(protocol, "http" | "https") || is_http_port(port)
}

pub(crate) fn normalize_banner(input: &[u8]) -> Option<String> {
    let normalized = String::from_utf8_lossy(input)
        .replace(
            |ch: char| ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t',
            " ",
        )
        .trim()
        .chars()
        .take(240)
        .collect::<String>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub(crate) fn infer_service_name(
    port: u16,
    protocol: &str,
    banner: Option<&str>,
    server_header: Option<&str>,
) -> String {
    let normalized = format!(
        "{} {}",
        banner.unwrap_or_default(),
        server_header.unwrap_or_default()
    )
    .to_lowercase();

    if normalized.contains("ssh") {
        return "ssh".to_string();
    }
    if normalized.contains("smtp") {
        return "smtp".to_string();
    }
    if normalized.contains("redis") {
        return "redis".to_string();
    }
    if normalized.contains("mysql") {
        return "mysql".to_string();
    }
    if normalized.contains("postgres") {
        return "postgresql".to_string();
    }
    if normalized.contains("mongodb") {
        return "mongodb".to_string();
    }
    if normalized.contains("elasticsearch") {
        return "elasticsearch".to_string();
    }
    if is_http_target(port, protocol) || normalized.contains("http") {
        return if is_tls_port(port) || protocol == "https" {
            "https".to_string()
        } else {
            "http".to_string()
        };
    }

    match port {
        21 => "ftp",
        22 => "ssh",
        23 => "telnet",
        25 => "smtp",
        53 => "dns",
        80 => "http",
        110 => "pop3",
        143 => "imap",
        443 => "https",
        445 => "smb",
        993 => "imaps",
        995 => "pop3s",
        1433 => "mssql",
        1521 => "oracle",
        3306 => "mysql",
        3389 => "rdp",
        5432 => "postgresql",
        5900 => "vnc",
        6379 => "redis",
        8080 => "http",
        8443 => "https",
        9200 => "elasticsearch",
        27017 => "mongodb",
        _ if !protocol.is_empty() => protocol,
        _ => "unknown",
    }
    .to_string()
}

pub(crate) fn extract_product_info(
    banner: Option<&str>,
    server_header: Option<&str>,
) -> ProductInfo {
    let source = format!(
        "{} {}",
        server_header.unwrap_or_default(),
        banner.unwrap_or_default()
    )
    .trim()
    .to_string();

    if source.is_empty() {
        return ProductInfo {
            product_name: None,
            vendor: None,
            version: None,
        };
    }

    let lowered = source.to_lowercase();
    let mappings = [
        ("nginx/", "nginx", Some("NGINX")),
        ("apache/", "apache", Some("Apache")),
        ("apache httpd/", "apache httpd", Some("Apache")),
        ("microsoft-iis/", "microsoft-iis", Some("Microsoft")),
        ("openssh_", "openssh", Some("OpenSSH")),
        ("openssh-", "openssh", Some("OpenSSH")),
        ("elasticsearch/", "elasticsearch", Some("Elastic")),
    ];

    for (needle, product_name, vendor) in mappings {
        if let Some(index) = lowered.find(needle) {
            let version_start = index + needle.len();
            let version = lowered[version_start..]
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
                .collect::<String>();

            return ProductInfo {
                product_name: Some(product_name.to_string()),
                vendor: vendor.map(str::to_string),
                version: if version.is_empty() {
                    None
                } else {
                    Some(version)
                },
            };
        }
    }

    if let Some(index) = lowered.find("redis") {
        let version = lowered[index..]
            .split("v=")
            .nth(1)
            .map(|value| {
                value
                    .chars()
                    .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
                    .collect::<String>()
            })
            .filter(|value| !value.is_empty());

        return ProductInfo {
            product_name: Some("redis".to_string()),
            vendor: Some("Redis".to_string()),
            version,
        };
    }

    let first_token = source
        .split([' ', '/'])
        .find(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_lowercase());

    ProductInfo {
        product_name: first_token,
        vendor: None,
        version: None,
    }
}
