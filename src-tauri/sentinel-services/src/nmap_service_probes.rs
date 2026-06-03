use anyhow::{anyhow, Result};
use sentinel_core::models::dictionary::DictionaryWordInput;

#[derive(Debug, Clone, Default)]
struct ProbeContext {
    protocol: String,
    name: String,
    ports: Vec<u16>,
    ssl_ports: Vec<u16>,
    rarity: Option<u8>,
    fallback_probes: Vec<String>,
}

pub fn parse_nmap_service_probes(content: &str) -> Result<Vec<DictionaryWordInput>> {
    let mut entries = Vec::new();
    let mut current = ProbeContext::default();
    let mut match_index = 0usize;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(context) = parse_probe_line(line)? {
            current = context;
            continue;
        }

        if let Some(ports) = line.strip_prefix("ports ") {
            current.ports = parse_port_list(ports);
            continue;
        }

        if let Some(ssl_ports) = line.strip_prefix("sslports ") {
            current.ssl_ports = parse_port_list(ssl_ports);
            continue;
        }

        if let Some(rarity) = line.strip_prefix("rarity ") {
            current.rarity = rarity.trim().parse::<u8>().ok();
            continue;
        }

        if let Some(fallback) = line.strip_prefix("fallback ") {
            current.fallback_probes = fallback
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(normalize_probe_name)
                .collect();
            continue;
        }

        if line.starts_with("match ") || line.starts_with("softmatch ") {
            if current.name.is_empty() {
                continue;
            }

            if let Some(entry) = parse_match_line(line, &current, match_index)? {
                entries.push(entry);
                match_index += 1;
            }
        }
    }

    if entries.is_empty() {
        return Err(anyhow!(
            "No supported Probe/match entries were found in nmap-service-probes content"
        ));
    }

    Ok(entries)
}

fn parse_probe_line(line: &str) -> Result<Option<ProbeContext>> {
    let Some(rest) = line.strip_prefix("Probe ") else {
        return Ok(None);
    };

    let mut parts = rest.split_whitespace();
    let protocol = parts
        .next()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Invalid Probe line: missing protocol"))?;
    let name = parts
        .next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Invalid Probe line: missing name"))?;

    Ok(Some(ProbeContext {
        protocol,
        name,
        ports: Vec::new(),
        ssl_ports: Vec::new(),
        rarity: None,
        fallback_probes: Vec::new(),
    }))
}

fn parse_match_line(
    line: &str,
    context: &ProbeContext,
    match_index: usize,
) -> Result<Option<DictionaryWordInput>> {
    let (softmatch, rest) = if let Some(value) = line.strip_prefix("softmatch ") {
        (true, value)
    } else if let Some(value) = line.strip_prefix("match ") {
        (false, value)
    } else {
        return Ok(None);
    };

    let mut chars = rest.chars().peekable();
    let mut service = String::new();
    while let Some(ch) = chars.peek() {
        if ch.is_whitespace() {
            break;
        }
        service.push(*ch);
        chars.next();
    }
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }

    if service.is_empty() {
        return Ok(None);
    }

    let remaining: String = chars.collect();
    let Some((pattern, version_info)) = parse_match_expression(&remaining) else {
        return Ok(None);
    };

    let service_name = normalize_service_name(&service, &context);
    let product =
        literal_version_field(&version_info, 'p').or_else(|| default_product_name(&service_name));
    let version = literal_version_field(&version_info, 'v');
    let vendor = infer_vendor(&service_name, product.as_deref());
    let (asset_category, asset_family) = infer_asset_classification(&service_name);
    let matcher_part = infer_matcher_part(&service_name, context);
    let rule_word = format!(
        "nmap_{}_{}_{}",
        sanitize_identifier(&service_name),
        sanitize_identifier(&context.name),
        match_index + 1
    );

    Ok(Some(DictionaryWordInput {
        word: rule_word.clone(),
        weight: Some(if softmatch { 6.0 } else { 8.0 }),
        category: Some(asset_category.to_string()),
        metadata: Some(serde_json::json!({
            "rule_id": rule_word,
            "name": product.clone().unwrap_or_else(|| title_case(&service_name)),
            "service": service_name,
            "product": product,
            "vendor": vendor,
            "version": version,
            "asset_category": asset_category,
            "asset_family": asset_family,
            "priority": 90,
            "protocol": context.protocol,
            "probeName": normalize_probe_name(&context.name),
            "ports": context.ports,
            "sslPorts": context.ssl_ports,
            "rarity": context.rarity,
            "fallbackProbes": context.fallback_probes,
            "operator": "or",
            "softmatch": softmatch,
            "confidence": if softmatch { 0.72 } else { 0.88 },
            "matchers": [
                {
                    "part": matcher_part,
                    "type": "regex",
                    "value": pattern
                }
            ],
            "source": "nmap-service-probes"
        })),
    }))
}

fn parse_match_expression(input: &str) -> Option<(String, String)> {
    let mut chars = input.chars();
    if chars.next()? != 'm' {
        return None;
    }
    let delimiter = chars.next()?;
    let remaining = chars.as_str();
    let (pattern, rest) = split_delimited(remaining, delimiter)?;

    let tail = rest.trim_start();
    let tail = if tail.starts_with('i')
        || tail.starts_with('s')
        || tail.starts_with('x')
        || tail.starts_with('A')
    {
        tail[1..].trim_start()
    } else {
        tail
    };

    Some((pattern, tail.to_string()))
}

fn split_delimited(input: &str, delimiter: char) -> Option<(String, String)> {
    let mut escaped = false;
    let mut result = String::new();

    for (index, ch) in input.char_indices() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            result.push(ch);
            escaped = true;
            continue;
        }
        if ch == delimiter {
            let rest = input[index + ch.len_utf8()..].to_string();
            return Some((result, rest));
        }
        result.push(ch);
    }

    None
}

fn parse_port_list(input: &str) -> Vec<u16> {
    let mut ports = Vec::new();

    for item in input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if let Some((start, end)) = item.split_once('-') {
            let Ok(start_port) = start.trim().parse::<u16>() else {
                continue;
            };
            let Ok(end_port) = end.trim().parse::<u16>() else {
                continue;
            };
            if start_port <= end_port {
                ports.extend(start_port..=end_port);
            }
            continue;
        }

        if let Ok(port) = item.parse::<u16>() {
            ports.push(port);
        }
    }

    ports.sort_unstable();
    ports.dedup();
    ports
}

fn literal_version_field(input: &str, field: char) -> Option<String> {
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch != field {
            continue;
        }
        if chars.next()? != '/' {
            continue;
        }

        let mut value = String::new();
        let mut escaped = false;
        for ch in chars.by_ref() {
            if escaped {
                value.push(ch);
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '/' {
                let trimmed = value.trim();
                if trimmed.is_empty() || trimmed.contains('$') {
                    return None;
                }
                return Some(trimmed.to_string());
            }
            value.push(ch);
        }
    }

    None
}

fn normalize_service_name(service: &str, context: &ProbeContext) -> String {
    let normalized = service.trim().to_lowercase();
    match normalized.as_str() {
        "ssl/http" | "https" => "https".to_string(),
        "http-alt" | "http-proxy" | "ssl/http-proxy" => "http".to_string(),
        "ssh" if context.protocol == "tcp" => "ssh".to_string(),
        _ => normalized,
    }
}

fn normalize_probe_name(name: &str) -> String {
    match name.trim() {
        "NULL" => "tcp_banner".to_string(),
        "GetRequest" | "HTTPOptions" | "FourOhFourRequest" | "GenericLines" => {
            "http_head".to_string()
        }
        other => sanitize_identifier(other),
    }
}

fn sanitize_identifier(value: &str) -> String {
    let lowered = value.trim().to_lowercase();
    let mut output = String::with_capacity(lowered.len());
    let mut last_was_sep = false;

    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            output.push('_');
            last_was_sep = true;
        }
    }

    output.trim_matches('_').to_string()
}

fn title_case(value: &str) -> String {
    value
        .split(['_', '-', '/', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_product_name(service: &str) -> Option<String> {
    let title = title_case(service);
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn infer_vendor(service: &str, product: Option<&str>) -> Option<String> {
    let source = format!("{} {}", service, product.unwrap_or_default()).to_lowercase();
    let vendor = if source.contains("nginx") {
        Some("NGINX")
    } else if source.contains("apache") {
        Some("Apache Software Foundation")
    } else if source.contains("iis") || source.contains("microsoft") {
        Some("Microsoft")
    } else if source.contains("openssh") {
        Some("OpenSSH")
    } else if source.contains("redis") {
        Some("Redis")
    } else if source.contains("mysql") || source.contains("mariadb") {
        Some("Oracle")
    } else if source.contains("postgres") {
        Some("PostgreSQL Global Development Group")
    } else if source.contains("elasticsearch") {
        Some("Elastic")
    } else {
        None
    };

    vendor.map(str::to_string)
}

fn infer_asset_classification(service: &str) -> (&'static str, &'static str) {
    match service {
        "http" | "https" => ("web_server", "http_server"),
        "ssh" => ("remote_access", "ssh"),
        "redis" => ("cache", "in_memory_store"),
        "mysql" | "postgresql" | "mssql" | "oracle" => ("database", "relational_database"),
        "mongodb" => ("database", "document_database"),
        "ftp" => ("file_transfer", "ftp"),
        "smtp" | "imap" | "pop3" | "imaps" | "pop3s" => ("messaging", "mail_server"),
        "elasticsearch" => ("search", "search_engine"),
        _ => ("service", "network_service"),
    }
}

fn infer_matcher_part(service: &str, context: &ProbeContext) -> &'static str {
    if matches!(service, "http" | "https")
        || context.name.contains("Request")
        || context.name.contains("HTTP")
    {
        "header"
    } else {
        "banner"
    }
}

#[cfg(test)]
mod tests {
    use super::parse_nmap_service_probes;

    #[test]
    fn parses_supported_nmap_probe_subset() {
        let content = r#"
Probe TCP NULL q||
ports 22,6379
rarity 3
fallback GetRequest
match ssh m|^SSH-([\d.]+)-OpenSSH[_-]([\w.]+)| p/OpenSSH/
match redis m|redis_version:([0-9.]+)| p/Redis/

Probe TCP GetRequest q|GET / HTTP/1.0\r\n\r\n|
ports 80,8080
sslports 443
match http m|^Server: nginx(?:/([0-9.]+))?| p/Nginx/
"#;

        let entries = parse_nmap_service_probes(content).expect("should parse nmap subset");
        assert_eq!(entries.len(), 3);

        let nginx = entries
            .iter()
            .find(|entry| entry.word.contains("http_getrequest"))
            .expect("nginx entry");
        let metadata = nginx.metadata.as_ref().expect("metadata");
        assert_eq!(metadata["probeName"], "http_head");
        assert_eq!(metadata["asset_category"], "web_server");
        assert_eq!(metadata["ports"], serde_json::json!([80, 8080]));
        assert_eq!(metadata["sslPorts"], serde_json::json!([443]));

        let ssh = entries
            .iter()
            .find(|entry| entry.word.contains("ssh_null"))
            .expect("ssh entry");
        let ssh_metadata = ssh.metadata.as_ref().expect("ssh metadata");
        assert_eq!(ssh_metadata["service"], "ssh");
        assert_eq!(ssh_metadata["probeName"], "tcp_banner");
        assert_eq!(ssh_metadata["asset_family"], "ssh");
        assert_eq!(ssh_metadata["rarity"], serde_json::json!(3));
        assert_eq!(
            ssh_metadata["fallbackProbes"],
            serde_json::json!(["http_head"])
        );
    }
}
