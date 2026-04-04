use std::net::{IpAddr, Ipv6Addr};

use ipnet::IpNet;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyScopeRule {
    #[serde(default = "default_rule_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub host_or_ip_range: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub file: String,
}

#[derive(Debug, Clone)]
struct ScopeTarget {
    protocol: String,
    host: String,
    port: String,
    file: String,
}

fn default_rule_enabled() -> bool {
    true
}

impl Default for ProxyScopeRule {
    fn default() -> Self {
        Self {
            enabled: true,
            protocol: "any".to_string(),
            host_or_ip_range: String::new(),
            port: String::new(),
            file: String::new(),
        }
    }
}

pub fn url_is_in_scope(
    url: &str,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
) -> bool {
    let Some(target) = build_scope_target(url) else {
        return include_rules.is_empty() && exclude_rules.is_empty();
    };

    target_is_in_scope(&target, include_rules, exclude_rules)
}

fn build_scope_target(url: &str) -> Option<ScopeTarget> {
    let parsed = Url::parse(url).ok()?;
    let protocol = parsed.scheme().trim().to_ascii_lowercase();
    let host = parsed.host_str()?.trim().trim_end_matches('.').to_ascii_lowercase();
    let port = parsed
        .port_or_known_default()
        .map(|value| value.to_string())
        .unwrap_or_default();
    let file = if parsed.path().is_empty() {
        "/".to_string()
    } else {
        parsed.path().to_string()
    };

    Some(ScopeTarget {
        protocol,
        host,
        port,
        file,
    })
}

fn target_is_in_scope(
    target: &ScopeTarget,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
) -> bool {
    let include_matches = include_rules.is_empty()
        || include_rules
            .iter()
            .filter(|rule| rule.enabled)
            .any(|rule| rule_matches_target(rule, target));

    if !include_matches {
        return false;
    }

    !exclude_rules
        .iter()
        .filter(|rule| rule.enabled)
        .any(|rule| rule_matches_target(rule, target))
}

fn rule_matches_target(rule: &ProxyScopeRule, target: &ScopeTarget) -> bool {
    protocol_matches(&rule.protocol, &target.protocol)
        && host_or_ip_matches(&rule.host_or_ip_range, &target.host)
        && text_pattern_matches(&rule.port, &target.port)
        && text_pattern_matches(&rule.file, &target.file)
}

fn protocol_matches(pattern: &str, actual: &str) -> bool {
    let normalized = pattern.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "any" {
        return true;
    }

    exact_or_regex_matches(&normalized, actual)
}

fn host_or_ip_matches(pattern: &str, actual_host: &str) -> bool {
    let normalized = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }

    if normalized == "*" || actual_host == normalized {
        return true;
    }

    if let Some(suffix) = normalized.strip_prefix("*.") {
        return actual_host == suffix || actual_host.ends_with(&format!(".{suffix}"));
    }

    if let Ok(net) = normalized.parse::<IpNet>() {
        if let Ok(host_ip) = actual_host.parse::<IpAddr>() {
            return net.contains(&host_ip);
        }
    }

    if let Some((start, end)) = parse_ip_range(&normalized) {
        if let Ok(host_ip) = actual_host.parse::<IpAddr>() {
            return ip_in_range(host_ip, start, end);
        }
    }

    exact_or_regex_matches(&normalized, actual_host)
}

fn parse_ip_range(input: &str) -> Option<(IpAddr, IpAddr)> {
    let (start, end) = input.split_once('-')?;
    let start_ip = start.trim().parse::<IpAddr>().ok()?;
    let end_ip = end.trim().parse::<IpAddr>().ok()?;
    if std::mem::discriminant(&start_ip) != std::mem::discriminant(&end_ip) {
        return None;
    }
    Some((start_ip, end_ip))
}

fn ip_in_range(ip: IpAddr, start: IpAddr, end: IpAddr) -> bool {
    match (ip, start, end) {
        (IpAddr::V4(ip), IpAddr::V4(start), IpAddr::V4(end)) => {
            let value = u32::from(ip);
            let start = u32::from(start);
            let end = u32::from(end);
            value >= start.min(end) && value <= start.max(end)
        }
        (IpAddr::V6(ip), IpAddr::V6(start), IpAddr::V6(end)) => {
            let value = ipv6_to_u128(ip);
            let start = ipv6_to_u128(start);
            let end = ipv6_to_u128(end);
            value >= start.min(end) && value <= start.max(end)
        }
        _ => false,
    }
}

fn ipv6_to_u128(ip: Ipv6Addr) -> u128 {
    u128::from_be_bytes(ip.octets())
}

fn text_pattern_matches(pattern: &str, actual: &str) -> bool {
    let normalized = pattern.trim();
    if normalized.is_empty() {
        return true;
    }

    exact_or_regex_matches(normalized, actual)
}

fn exact_or_regex_matches(pattern: &str, actual: &str) -> bool {
    if actual.eq_ignore_ascii_case(pattern) {
        return true;
    }

    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map(|regex| regex.is_match(actual))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{url_is_in_scope, ProxyScopeRule};

    #[test]
    fn empty_scope_allows_all() {
        assert!(url_is_in_scope(
            "https://example.com/api/orders/1",
            &[],
            &[],
        ));
    }

    #[test]
    fn include_rule_filters_by_host() {
        let include = vec![ProxyScopeRule {
            host_or_ip_range: "*.example.com".to_string(),
            ..Default::default()
        }];
        assert!(url_is_in_scope(
            "https://api.example.com/orders",
            &include,
            &[],
        ));
        assert!(!url_is_in_scope(
            "https://api.other.com/orders",
            &include,
            &[],
        ));
    }

    #[test]
    fn exclude_rule_wins() {
        let include = vec![ProxyScopeRule {
            host_or_ip_range: "*.example.com".to_string(),
            ..Default::default()
        }];
        let exclude = vec![ProxyScopeRule {
            file: "^/admin".to_string(),
            ..Default::default()
        }];
        assert!(!url_is_in_scope(
            "https://api.example.com/admin/users",
            &include,
            &exclude,
        ));
    }

    #[test]
    fn cidr_host_match_is_supported() {
        let include = vec![ProxyScopeRule {
            host_or_ip_range: "127.0.0.0/24".to_string(),
            ..Default::default()
        }];
        assert!(url_is_in_scope("http://127.0.0.1:8080/test", &include, &[]));
    }
}
