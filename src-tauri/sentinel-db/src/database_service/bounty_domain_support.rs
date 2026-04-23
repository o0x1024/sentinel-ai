use std::net::IpAddr;

use crate::database_service::bounty::BountyAssetRow;

const COMMON_TWO_LABEL_PUBLIC_SUFFIXES: &[&str] = &[
    "ac.jp", "ac.uk", "co.jp", "co.kr", "co.nz", "co.uk", "com.au", "com.br", "com.cn", "com.hk",
    "com.mx", "com.sg", "edu.cn", "edu.hk", "gov.cn", "gov.uk", "net.au", "net.cn", "org.au",
    "org.cn", "org.hk", "org.uk",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainHierarchy {
    pub hostname: String,
    pub root_domain: String,
    pub parent_domain: Option<String>,
    pub subdomain_level: i32,
}

fn infer_root_domain(domain: &str) -> Option<String> {
    let parts: Vec<&str> = domain.split('.').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let suffix = format!(
        "{}.{}",
        parts[parts.len().saturating_sub(2)],
        parts[parts.len().saturating_sub(1)]
    );

    if parts.len() >= 3
        && COMMON_TWO_LABEL_PUBLIC_SUFFIXES
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&suffix))
    {
        return Some(format!(
            "{}.{}.{}",
            parts[parts.len().saturating_sub(3)],
            parts[parts.len().saturating_sub(2)],
            parts[parts.len().saturating_sub(1)]
        ));
    }

    Some(suffix)
}

fn extract_hostname(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let host = if let Some((_, remainder)) = trimmed.split_once("://") {
        remainder
            .split('/')
            .next()
            .unwrap_or(remainder)
            .split('@')
            .next_back()
            .unwrap_or(remainder)
            .split(':')
            .next()
            .unwrap_or(remainder)
            .to_string()
    } else {
        trimmed
            .split('/')
            .next()
            .unwrap_or(trimmed)
            .split(':')
            .next()
            .unwrap_or(trimmed)
            .to_string()
    };

    let normalized = host
        .trim()
        .trim_start_matches("*.")
        .trim_matches('.')
        .to_ascii_lowercase();

    if normalized.is_empty() || normalized.parse::<IpAddr>().is_ok() {
        return None;
    }

    Some(normalized)
}

pub fn derive_domain_hierarchy(value: &str) -> Option<DomainHierarchy> {
    let hostname = extract_hostname(value)?;
    let root_domain = infer_root_domain(&hostname)?;

    let hostname_parts: Vec<&str> = hostname
        .split('.')
        .filter(|part| !part.is_empty())
        .collect();
    let root_parts: Vec<&str> = root_domain
        .split('.')
        .filter(|part| !part.is_empty())
        .collect();

    if hostname_parts.len() < root_parts.len() {
        return None;
    }

    let subdomain_level = hostname_parts.len().saturating_sub(root_parts.len()) as i32;
    let parent_domain = if subdomain_level > 0 && hostname_parts.len() > 1 {
        Some(hostname_parts[1..].join("."))
    } else {
        None
    };

    Some(DomainHierarchy {
        hostname,
        root_domain,
        parent_domain,
        subdomain_level,
    })
}

pub fn populate_bounty_asset_domain_fields(asset: &mut BountyAssetRow) {
    if asset.asset_type != "domain" {
        return;
    }

    let Some(source) = asset
        .hostname
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(Some(asset.canonical_url.as_str()))
    else {
        asset.root_domain = None;
        asset.parent_domain = None;
        asset.subdomain_level = None;
        return;
    };

    let Some(hierarchy) = derive_domain_hierarchy(source) else {
        asset.root_domain = None;
        asset.parent_domain = None;
        asset.subdomain_level = None;
        return;
    };

    asset.hostname = Some(hierarchy.hostname);
    asset.root_domain = Some(hierarchy.root_domain);
    asset.parent_domain = hierarchy.parent_domain;
    asset.subdomain_level = Some(hierarchy.subdomain_level);
}

#[cfg(test)]
mod tests {
    use super::{derive_domain_hierarchy, DomainHierarchy};

    #[test]
    fn derives_root_domain_and_level_for_registrable_domain() {
        assert_eq!(
            derive_domain_hierarchy("example.com"),
            Some(DomainHierarchy {
                hostname: "example.com".to_string(),
                root_domain: "example.com".to_string(),
                parent_domain: None,
                subdomain_level: 0,
            })
        );
    }

    #[test]
    fn derives_root_domain_and_level_for_nested_subdomain() {
        assert_eq!(
            derive_domain_hierarchy("https://dev.api.example.com/login"),
            Some(DomainHierarchy {
                hostname: "dev.api.example.com".to_string(),
                root_domain: "example.com".to_string(),
                parent_domain: Some("api.example.com".to_string()),
                subdomain_level: 2,
            })
        );
    }

    #[test]
    fn handles_common_multi_label_public_suffix() {
        assert_eq!(
            derive_domain_hierarchy("edge.dev.example.co.uk"),
            Some(DomainHierarchy {
                hostname: "edge.dev.example.co.uk".to_string(),
                root_domain: "example.co.uk".to_string(),
                parent_domain: Some("dev.example.co.uk".to_string()),
                subdomain_level: 2,
            })
        );
    }
}
