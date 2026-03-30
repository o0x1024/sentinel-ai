use crate::service_probe_runtime::{ServiceProbeResult, ServiceProbeRule};
use serde_json::Value;
use std::collections::BTreeMap;

use super::common::{extract_product_info, infer_service_name, service_key};
use super::probe::ProbeEvidence;
use super::rules::{
    metadata_object_field, metadata_string_field, parse_rule_metadata, ServiceProbeRuleMatcher,
};

#[derive(Debug, Clone)]
struct MatchContext {
    port: u16,
    protocol: String,
    probe_name: String,
    banner: String,
    server_header: String,
    headers: BTreeMap<String, String>,
    status_code: Option<u16>,
    service_name: String,
    product_name: String,
}

fn metadata_object(rule: &ServiceProbeRule) -> Option<&serde_json::Map<String, Value>> {
    rule.metadata.as_object()
}

fn string_field(metadata: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<String> {
    metadata
        .and_then(|map| map.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn match_source_for_part(ctx: &MatchContext, part: &str, key: Option<&str>) -> Option<String> {
    match part {
        "header" => {
            if let Some(key) = key {
                ctx.headers.get(&key.to_ascii_lowercase()).cloned()
            } else if !ctx.server_header.is_empty() {
                Some(ctx.server_header.clone())
            } else {
                None
            }
        }
        "product" => (!ctx.product_name.is_empty()).then_some(ctx.product_name.clone()),
        "service" => (!ctx.service_name.is_empty()).then_some(ctx.service_name.clone()),
        "status" => ctx.status_code.map(|status| status.to_string()),
        "title" | "body" => None,
        _ => (!ctx.banner.is_empty()).then_some(ctx.banner.clone()),
    }
}

fn match_string_value(source: &str, matcher_type: &str, value: &Value) -> bool {
    match matcher_type {
        "equals" => value
            .as_str()
            .map(|candidate| source.eq_ignore_ascii_case(candidate))
            .unwrap_or(false),
        "regex" => value
            .as_str()
            .and_then(|pattern| {
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .ok()
            })
            .map(|re| re.is_match(source))
            .unwrap_or(false),
        "in" => {
            if let Some(items) = value.as_array() {
                return items.iter().any(|item| {
                    item.as_str()
                        .map(|candidate| candidate.eq_ignore_ascii_case(source))
                        .or_else(|| {
                            item.as_i64()
                                .map(|candidate| candidate.to_string() == source)
                        })
                        .or_else(|| {
                            item.as_u64()
                                .map(|candidate| candidate.to_string() == source)
                        })
                        .unwrap_or(false)
                });
            }

            value
                .as_str()
                .map(|candidate_list| {
                    candidate_list
                        .split(',')
                        .any(|candidate| candidate.trim().eq_ignore_ascii_case(source))
                })
                .unwrap_or(false)
        }
        _ => value
            .as_str()
            .map(|candidate| source.to_lowercase().contains(&candidate.to_lowercase()))
            .unwrap_or(false),
    }
}

fn matcher_hit(ctx: &MatchContext, matcher: &ServiceProbeRuleMatcher) -> bool {
    let part = matcher.part.as_deref().unwrap_or("banner").to_lowercase();
    let matcher_type = matcher
        .r#type
        .as_deref()
        .unwrap_or("contains")
        .to_lowercase();
    let key = matcher
        .key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let source = match_source_for_part(ctx, &part, key);

    match matcher_type.as_str() {
        "exists" => source.is_some(),
        _ => {
            let Some(source) = source else {
                return false;
            };
            let Some(value) = matcher.value.as_ref() else {
                return false;
            };
            match_string_value(&source, &matcher_type, value)
        }
    }
}

fn rule_matches(ctx: &MatchContext, rule: &ServiceProbeRule) -> bool {
    let metadata = metadata_object(rule);
    let typed = parse_rule_metadata(rule);

    if let Some(protocol) = typed
        .protocol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !ctx.protocol.eq_ignore_ascii_case(protocol)
            && !ctx.service_name.eq_ignore_ascii_case(protocol)
        {
            return false;
        }
    }

    if let Some(probe_name) = typed
        .probe_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !ctx.probe_name.eq_ignore_ascii_case(probe_name) {
            return false;
        }
    }

    if !typed.ports.is_empty() && !typed.ports.contains(&ctx.port) {
        return false;
    }

    if !typed.ssl_ports.is_empty() && !typed.ssl_ports.contains(&ctx.port) {
        return false;
    }

    if !typed.matchers.is_empty() {
        let operator = typed.operator.as_deref().unwrap_or("or").to_lowercase();

        return if operator == "and" {
            typed
                .matchers
                .iter()
                .all(|matcher| matcher_hit(ctx, matcher))
        } else {
            typed
                .matchers
                .iter()
                .any(|matcher| matcher_hit(ctx, matcher))
        };
    }

    let matchers = metadata
        .and_then(|map| metadata_object_field(map, "matchers"))
        .and_then(Value::as_array);

    if let Some(matchers) = matchers.filter(|items| !items.is_empty()) {
        let operator = metadata
            .and_then(|map| metadata_object_field(map, "operator"))
            .and_then(Value::as_str)
            .unwrap_or("or")
            .to_lowercase();

        return if operator == "and" {
            matchers.iter().all(|matcher| {
                serde_json::from_value::<ServiceProbeRuleMatcher>(matcher.clone())
                    .map(|parsed| matcher_hit(ctx, &parsed))
                    .unwrap_or(false)
            })
        } else {
            matchers.iter().any(|matcher| {
                serde_json::from_value::<ServiceProbeRuleMatcher>(matcher.clone())
                    .map(|parsed| matcher_hit(ctx, &parsed))
                    .unwrap_or(false)
            })
        };
    }

    let probe = format!(
        "{} {} {} {}",
        ctx.product_name, ctx.service_name, ctx.server_header, ctx.banner
    )
    .to_lowercase();

    probe.contains(&rule.word.to_lowercase())
}

fn match_rule<'a>(
    ctx: &MatchContext,
    rules: &'a [ServiceProbeRule],
) -> Option<&'a ServiceProbeRule> {
    rules.iter().find(|rule| rule_matches(ctx, rule))
}

pub(crate) fn match_evidence(
    evidence: ProbeEvidence,
    rules: &[ServiceProbeRule],
) -> ServiceProbeResult {
    let ProbeEvidence {
        host,
        port,
        protocol,
        probe_name,
        success,
        available,
        banner,
        server_header,
        headers,
        status_code,
        error,
    } = evidence;

    let service_name =
        infer_service_name(port, &protocol, banner.as_deref(), server_header.as_deref());
    let product = extract_product_info(banner.as_deref(), server_header.as_deref());
    let match_ctx = MatchContext {
        port,
        protocol: protocol.clone(),
        probe_name,
        banner: banner.clone().unwrap_or_default(),
        server_header: server_header.clone().unwrap_or_default(),
        headers,
        status_code,
        service_name: service_name.clone(),
        product_name: product.product_name.clone().unwrap_or_default(),
    };
    let matched_rule = match_rule(&match_ctx, rules);
    let typed_metadata = matched_rule.map(parse_rule_metadata);
    let matched_metadata = matched_rule.and_then(metadata_object);
    let matched_service = typed_metadata
        .as_ref()
        .and_then(|metadata| metadata_string_field(metadata.service.as_ref()))
        .or_else(|| string_field(matched_metadata, "service"));
    let matched_product = typed_metadata
        .as_ref()
        .and_then(|metadata| metadata_string_field(metadata.product.as_ref()))
        .or_else(|| string_field(matched_metadata, "product"));
    let matched_vendor = typed_metadata
        .as_ref()
        .and_then(|metadata| metadata_string_field(metadata.vendor.as_ref()))
        .or_else(|| string_field(matched_metadata, "vendor"));
    let matched_version = typed_metadata
        .as_ref()
        .and_then(|metadata| metadata_string_field(metadata.version.as_ref()))
        .or_else(|| string_field(matched_metadata, "version"));
    let matched_confidence = typed_metadata
        .as_ref()
        .and_then(|metadata| metadata.confidence)
        .or_else(|| {
            typed_metadata.as_ref().map(|metadata| {
                if metadata.softmatch.unwrap_or(false) {
                    0.72
                } else {
                    0.94
                }
            })
        })
        .or_else(|| matched_rule.map(|_| 0.94));

    ServiceProbeResult {
        target: service_key(&host, port),
        success,
        available,
        host,
        port,
        protocol,
        service_name: Some(matched_service.unwrap_or(service_name)),
        product_name: matched_product.or(product.product_name),
        vendor: matched_vendor.or(product.vendor),
        version: matched_version.or(product.version),
        banner,
        server_header,
        title: None,
        status_code,
        confidence: Some(matched_confidence.unwrap_or(0.82)),
        matched_rule_id: matched_rule.and_then(|rule| rule.id.clone()),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_context() -> MatchContext {
        let mut headers = BTreeMap::new();
        headers.insert("server".to_string(), "nginx/1.25.3".to_string());
        headers.insert("x-jenkins".to_string(), "2.452.1".to_string());
        MatchContext {
            port: 8080,
            protocol: "http".to_string(),
            probe_name: "http_head".to_string(),
            banner: "OpenSSH_9.7".to_string(),
            server_header: "nginx/1.25.3".to_string(),
            headers,
            status_code: Some(200),
            service_name: "http".to_string(),
            product_name: "nginx".to_string(),
        }
    }

    #[test]
    fn matcher_supports_header_key_exists() {
        let matcher = ServiceProbeRuleMatcher {
            part: Some("header".to_string()),
            r#type: Some("exists".to_string()),
            key: Some("x-jenkins".to_string()),
            value: None,
        };
        assert!(matcher_hit(&test_context(), &matcher));
    }

    #[test]
    fn matcher_supports_header_key_contains() {
        let matcher = ServiceProbeRuleMatcher {
            part: Some("header".to_string()),
            r#type: Some("contains".to_string()),
            key: Some("server".to_string()),
            value: Some(json!("nginx")),
        };
        assert!(matcher_hit(&test_context(), &matcher));
    }

    #[test]
    fn matcher_supports_status_in_array() {
        let matcher = ServiceProbeRuleMatcher {
            part: Some("status".to_string()),
            r#type: Some("in".to_string()),
            key: None,
            value: Some(json!([200, 401, 403])),
        };
        assert!(matcher_hit(&test_context(), &matcher));
    }
}
