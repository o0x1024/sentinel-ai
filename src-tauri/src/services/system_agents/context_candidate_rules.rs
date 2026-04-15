use std::collections::{BTreeSet, HashMap, HashSet};

use crate::services::system_agents::context_candidate_types::{
    ContextDictionaryCandidate, ContextDictionaryCandidateCategory,
};
use crate::services::system_agents::TrafficContextExtractionSettings;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateSource {
    Path,
    Query,
    Body,
    Header,
    Cookie,
}

impl CandidateSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Body => "body",
            Self::Header => "header",
            Self::Cookie => "cookie",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateBehaviorHint {
    pub intent_hints: Vec<String>,
    pub behavior_steps: Vec<String>,
    pub last_page_title: Option<String>,
    pub last_page_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CandidateObservation {
    pub key: String,
    pub normalized_key: String,
    pub source: CandidateSource,
    pub value_sample: Option<String>,
    pub location: String,
    pub request_id: i64,
    pub behavior_hint: Option<CandidateBehaviorHint>,
}

#[derive(Debug, Clone)]
pub struct CandidateAggregate {
    pub preferred_key: String,
    pub normalized_key: String,
    pub evidence_count: usize,
    pub distinct_value_count: usize,
    pub sources: Vec<String>,
    pub example_values: Vec<String>,
    pub example_locations: Vec<String>,
    pub observations: Vec<CandidateObservation>,
    pub behavior_hints: Vec<CandidateBehaviorHint>,
    pub behavior_match_count: usize,
}

pub fn normalize_lookup_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

pub fn build_candidate_aggregate(
    observations: Vec<CandidateObservation>,
) -> Option<CandidateAggregate> {
    let first = observations.first()?;
    let mut key_counts: HashMap<String, usize> = HashMap::new();
    let mut request_ids = HashSet::new();
    let mut distinct_values = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut example_values = Vec::new();
    let mut example_locations = Vec::new();
    let mut behavior_hints = Vec::new();

    for observation in &observations {
        *key_counts.entry(observation.key.clone()).or_insert(0) += 1;
        request_ids.insert(observation.request_id);
        sources.insert(observation.source.as_str().to_string());

        if let Some(sample) = observation
            .value_sample
            .as_ref()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
        {
            distinct_values.insert(sample.to_string());
            if example_values.len() < 3 && !example_values.iter().any(|item| item == sample) {
                example_values.push(sample.to_string());
            }
        }

        if example_locations.len() < 3
            && !example_locations
                .iter()
                .any(|item| item == &observation.location)
        {
            example_locations.push(observation.location.clone());
        }

        if let Some(behavior_hint) = observation.behavior_hint.as_ref() {
            push_behavior_hint(&mut behavior_hints, behavior_hint);
        }
    }

    let preferred_key = key_counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(key, _)| key)
        .unwrap_or_else(|| first.key.clone());

    Some(CandidateAggregate {
        preferred_key,
        normalized_key: first.normalized_key.clone(),
        evidence_count: request_ids.len(),
        distinct_value_count: distinct_values.len(),
        sources: sources.into_iter().collect(),
        example_values,
        example_locations,
        observations,
        behavior_match_count: behavior_hints.len(),
        behavior_hints,
    })
}

pub fn score_field_candidate(
    aggregate: &CandidateAggregate,
) -> Option<(
    ContextDictionaryCandidateCategory,
    i32,
    Vec<String>,
    Option<String>,
    bool,
)> {
    if aggregate.normalized_key.is_empty() || is_noise_key(&aggregate.normalized_key) {
        return None;
    }

    let mut principal_score = 0;
    let mut principal_reasons = Vec::new();
    let mut resource_score = 0;
    let mut resource_reasons = Vec::new();
    let mut auth_token_score = 0;
    let mut auth_token_reasons = Vec::new();

    if matches_any_token(
        &aggregate.normalized_key,
        &[
            "user", "member", "operator", "owner", "account", "staff", "tenant", "org", "role",
        ],
    ) {
        principal_score += 25;
        principal_reasons.push("字段名更像主体/身份字段".to_string());
    }

    if matches_any_token(
        &aggregate.normalized_key,
        &[
            "id", "key", "code", "ref", "no", "order", "project", "case", "ticket", "biz", "ledger",
        ],
    ) {
        resource_score += 25;
        resource_reasons.push("字段名更像资源主键或业务对象标识".to_string());
    }

    if matches_any_token(
        &aggregate.normalized_key,
        &[
            "token",
            "session",
            "sid",
            "auth",
            "accesskey",
            "apikey",
            "jwt",
        ],
    ) {
        auth_token_score += 30;
        auth_token_reasons.push("字段名更像 token/session 参数".to_string());
    }

    let has_path_or_query = aggregate
        .sources
        .iter()
        .any(|source| source == "path" || source == "query");
    if has_path_or_query {
        resource_score += 15;
        resource_reasons.push("主要出现在 path/query，更像资源定位字段".to_string());
    }

    let has_body_or_header = aggregate
        .sources
        .iter()
        .any(|source| source == "body" || source == "header");
    if has_body_or_header {
        principal_score += 10;
        principal_reasons.push("出现在 body/header 中，常见于主体身份传递".to_string());
    }

    if aggregate.distinct_value_count >= 2 {
        principal_score += 10;
        resource_score += 10;
        auth_token_score += 10;
        principal_reasons.push("同一字段在多条请求里出现不同值".to_string());
        resource_reasons.push("同一字段在多条请求里出现不同值".to_string());
        auth_token_reasons.push("同一字段在多条请求里出现不同值".to_string());
    }

    if looks_like_identifier_value(aggregate.example_values.as_slice()) {
        resource_score += 10;
        resource_reasons.push("示例值形态更像对象 ID/业务单号".to_string());
    }

    if looks_like_secret_value(aggregate.example_values.as_slice()) {
        auth_token_score += 15;
        auth_token_reasons.push("示例值形态更像高熵 token/session".to_string());
    }

    if has_resource_behavior_signal(aggregate) {
        resource_score += 12;
        resource_reasons.push("浏览器行为显示这些请求更像对象详情或流程操作页面".to_string());
    }

    if has_principal_behavior_signal(aggregate) {
        principal_score += 8;
        principal_reasons.push("浏览器行为显示这些请求更像带有操作者语义的提交动作".to_string());
    }

    if has_auth_behavior_signal(aggregate) {
        auth_token_score += 6;
        auth_token_reasons.push("浏览器行为提示这些请求和登录态/认证动作相关".to_string());
    }

    if aggregate.evidence_count >= 3 {
        principal_score += 5;
        resource_score += 5;
        auth_token_score += 5;
    }

    let candidates = [
        (
            ContextDictionaryCandidateCategory::Principal,
            principal_score,
            principal_reasons,
        ),
        (
            ContextDictionaryCandidateCategory::Resource,
            resource_score,
            resource_reasons,
        ),
        (
            ContextDictionaryCandidateCategory::AuthToken,
            auth_token_score,
            auth_token_reasons,
        ),
    ];

    let (category, score, reasons) = candidates
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1))?;

    if score < 35 {
        return None;
    }

    let confidence = if score >= 60 {
        "high"
    } else if score >= 45 {
        "medium"
    } else {
        "low"
    };

    Some((category, score, reasons, None, confidence == "high"))
}

pub fn score_auth_header_candidate(aggregate: &CandidateAggregate) -> Option<(i32, Vec<String>)> {
    if aggregate.normalized_key.is_empty() {
        return None;
    }

    let mut score = 0;
    let mut reasons = Vec::new();
    if matches_any_token(
        &aggregate.normalized_key,
        &[
            "authorization",
            "token",
            "apikey",
            "auth",
            "session",
            "access",
        ],
    ) {
        score += 40;
        reasons.push("Header 名更像认证头".to_string());
    }
    if looks_like_secret_value(aggregate.example_values.as_slice()) {
        score += 15;
        reasons.push("Header 示例值更像高熵凭证".to_string());
    }
    if aggregate.evidence_count >= 3 {
        score += 10;
        reasons.push("该 header 在多条请求中持续出现".to_string());
    }
    (score >= 35).then_some((score, reasons))
}

pub fn score_cookie_hint_candidate(aggregate: &CandidateAggregate) -> Option<(i32, Vec<String>)> {
    if aggregate.normalized_key.is_empty() || is_noise_key(&aggregate.normalized_key) {
        return None;
    }

    let mut score = 0;
    let mut reasons = Vec::new();
    if matches_any_token(
        &aggregate.normalized_key,
        &["session", "sid", "token", "auth", "jwt", "tenant", "member"],
    ) {
        score += 35;
        reasons.push("Cookie 名更像登录态或租户/成员凭证".to_string());
    }
    if aggregate.evidence_count >= 2 {
        score += 10;
        reasons.push("Cookie 名在多条请求里复用".to_string());
    }
    (score >= 30).then_some((score, reasons))
}

pub fn score_action_alias_candidate(
    aggregate: &CandidateAggregate,
) -> Option<(i32, Vec<String>, Option<String>)> {
    let key = aggregate.preferred_key.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }

    let canonical = infer_canonical_action(&key)?;
    let mut score = 40;
    let mut reasons = vec!["路径 token 更像动作词".to_string()];
    if aggregate.evidence_count >= 2 {
        score += 10;
        reasons.push("动作 token 在多条请求里重复出现".to_string());
    }
    if action_token_appears_in_behavior(aggregate, &key) {
        score += 10;
        reasons.push("浏览器行为步骤或页面语义中也出现了同类动作词".to_string());
    }
    Some((score, reasons, Some(canonical.to_string())))
}

pub fn annotate_existing_coverage(
    candidate: &mut ContextDictionaryCandidate,
    settings: &TrafficContextExtractionSettings,
) {
    let normalized = candidate.normalized_key.as_str();
    let covered_by = match candidate.category.as_str() {
        "principal" => find_existing_match(settings.principal_keys(), normalized),
        "resource" => find_existing_match(settings.resource_key_hints(), normalized),
        "auth_header" => find_existing_match(settings.auth_header_keys(), normalized),
        "auth_token" => find_existing_match(settings.auth_token_keys(), normalized),
        "cookie_hint" => find_existing_match(settings.cookie_hint_keys(), normalized),
        "action_alias" => {
            let suggested = candidate
                .suggested_canonical_action
                .as_deref()
                .unwrap_or_default();
            settings
                .ordered_action_aliases()
                .into_iter()
                .find(|(action, aliases)| {
                    normalize_lookup_key(action) == normalize_lookup_key(suggested)
                        && aliases
                            .iter()
                            .any(|alias| normalize_lookup_key(alias) == normalized)
                })
                .map(|(_, aliases)| aliases.join(", "))
        }
        _ => None,
    };

    candidate.already_covered_by = covered_by;
    candidate.conflict_with_existing = false;
}

fn find_existing_match(values: Vec<String>, normalized: &str) -> Option<String> {
    values
        .into_iter()
        .find(|value| normalize_lookup_key(value) == normalized)
}

fn is_noise_key(normalized: &str) -> bool {
    matches!(
        normalized,
        "page"
            | "size"
            | "limit"
            | "offset"
            | "sort"
            | "order"
            | "q"
            | "query"
            | "keyword"
            | "search"
            | "t"
            | "ts"
            | "timestamp"
            | "nonce"
            | "callback"
            | "redirect"
            | "from"
            | "source"
            | "scene"
            | "tab"
            | "lang"
            | "locale"
            | "traceid"
            | "requestid"
            | "sign"
            | "signature"
            | "version"
            | "clientversion"
    )
}

fn matches_any_token(normalized_key: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| normalized_key.contains(needle))
}

fn push_behavior_hint(target: &mut Vec<CandidateBehaviorHint>, hint: &CandidateBehaviorHint) {
    if target.iter().any(|existing| {
        existing.intent_hints == hint.intent_hints
            && existing.behavior_steps == hint.behavior_steps
            && existing.last_page_title == hint.last_page_title
            && existing.last_page_url == hint.last_page_url
    }) {
        return;
    }
    if target.len() < 4 {
        target.push(hint.clone());
    }
}

fn aggregate_behavior_text(aggregate: &CandidateAggregate) -> String {
    aggregate
        .behavior_hints
        .iter()
        .flat_map(|hint| {
            hint.intent_hints
                .iter()
                .cloned()
                .chain(hint.behavior_steps.iter().cloned())
                .chain(hint.last_page_title.iter().cloned())
                .chain(hint.last_page_url.iter().cloned())
                .collect::<Vec<_>>()
        })
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_resource_behavior_signal(aggregate: &CandidateAggregate) -> bool {
    if aggregate.behavior_match_count == 0
        || aggregate.distinct_value_count < 2
        || !aggregate
            .sources
            .iter()
            .any(|source| source == "path" || source == "query")
    {
        return false;
    }

    let behavior_text = aggregate_behavior_text(aggregate);
    matches_any_token(
        &behavior_text,
        &[
            "detail",
            "details",
            "approve",
            "approval",
            "confirm",
            "workflow",
            "record",
            "refund",
            "order",
            "case",
            "ticket",
            "project",
            "打开页面",
            "切换路由",
            "点击",
            "提交",
        ],
    )
}

fn has_principal_behavior_signal(aggregate: &CandidateAggregate) -> bool {
    if aggregate.behavior_match_count == 0
        || !aggregate
            .sources
            .iter()
            .any(|source| source == "body" || source == "header")
    {
        return false;
    }

    let behavior_text = aggregate_behavior_text(aggregate);
    matches_any_token(
        &behavior_text,
        &[
            "operator", "owner", "assignee", "member", "user", "tenant", "account", "submit",
            "approve", "login", "sign in",
        ],
    )
}

fn has_auth_behavior_signal(aggregate: &CandidateAggregate) -> bool {
    if aggregate.behavior_match_count == 0 {
        return false;
    }
    let behavior_text = aggregate_behavior_text(aggregate);
    matches_any_token(
        &behavior_text,
        &["login", "sign in", "auth", "token", "session", "oauth"],
    )
}

fn action_token_appears_in_behavior(aggregate: &CandidateAggregate, token: &str) -> bool {
    if aggregate.behavior_match_count == 0 {
        return false;
    }
    aggregate_behavior_text(aggregate).contains(token)
}

pub fn derive_behavior_reason(
    aggregate: &CandidateAggregate,
    category: &ContextDictionaryCandidateCategory,
    key: &str,
    suggested_canonical_action: Option<&str>,
) -> Option<String> {
    if aggregate.behavior_match_count == 0 {
        return None;
    }

    let sample = aggregate
        .behavior_hints
        .iter()
        .flat_map(|hint| {
            hint.behavior_steps
                .iter()
                .cloned()
                .chain(hint.intent_hints.iter().cloned())
                .chain(hint.last_page_title.iter().cloned())
                .collect::<Vec<_>>()
        })
        .find(|item| !item.trim().is_empty())?;

    match category {
        ContextDictionaryCandidateCategory::Resource if has_resource_behavior_signal(aggregate) => {
            Some(format!(
                "浏览器行为里出现“{sample}”，这些请求更像对象详情或流程页面，因此 {key} 更可能是资源主键。"
            ))
        }
        ContextDictionaryCandidateCategory::Principal if has_principal_behavior_signal(aggregate) => {
            Some(format!(
                "浏览器行为里出现“{sample}”，这些请求更像带操作者语义的提交动作，因此 {key} 更可能是主体字段。"
            ))
        }
        ContextDictionaryCandidateCategory::AuthToken if has_auth_behavior_signal(aggregate) => {
            Some(format!(
                "浏览器行为里出现“{sample}”，这些请求和登录态/认证动作更接近，因此 {key} 更可能是凭证参数。"
            ))
        }
        ContextDictionaryCandidateCategory::ActionAlias if action_token_appears_in_behavior(aggregate, key) => {
            let canonical = suggested_canonical_action.unwrap_or("该动作");
            Some(format!(
                "浏览器行为步骤或页面语义中也出现了“{sample}”，说明 {key} 很可能是 {canonical} 的动作别名。"
            ))
        }
        _ => None,
    }
}

pub fn derive_behavior_evidence(aggregate: &CandidateAggregate) -> Vec<String> {
    let mut evidence = Vec::new();

    for hint in &aggregate.behavior_hints {
        if let Some(title) = hint
            .last_page_title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            push_unique_evidence(&mut evidence, format!("页面标题：{title}"));
        }

        for step in &hint.behavior_steps {
            let step = step.trim();
            if !step.is_empty() {
                push_unique_evidence(&mut evidence, format!("行为步骤：{step}"));
            }
        }

        for intent in &hint.intent_hints {
            let intent = intent.trim();
            if !intent.is_empty() {
                push_unique_evidence(&mut evidence, format!("意图提示：{intent}"));
            }
        }

        if evidence.len() >= 4 {
            break;
        }
    }

    evidence
}

fn push_unique_evidence(target: &mut Vec<String>, value: String) {
    if target.iter().any(|item| item == &value) {
        return;
    }
    if target.len() < 4 {
        target.push(value);
    }
}

fn looks_like_identifier_value(samples: &[String]) -> bool {
    samples.iter().any(|sample| {
        let value = sample.trim();
        value.len() >= 3
            && value.len() <= 128
            && (value.chars().all(|ch| ch.is_ascii_digit())
                || (value.chars().any(|ch| ch.is_ascii_digit())
                    && value
                        .chars()
                        .any(|ch| ch == '-' || ch == '_' || ch.is_ascii_uppercase())))
    })
}

fn looks_like_secret_value(samples: &[String]) -> bool {
    samples.iter().any(|sample| {
        let value = sample.trim();
        value.len() >= 12
            && value.chars().any(|ch| ch.is_ascii_alphabetic())
            && value.chars().any(|ch| ch.is_ascii_digit())
    })
}

fn infer_canonical_action(token: &str) -> Option<&'static str> {
    if matches_any_token(token, &["finalize", "finish", "writeoff", "complete"]) {
        return Some("complete");
    }
    if matches_any_token(token, &["approve", "approval"]) {
        return Some("approve");
    }
    if matches_any_token(token, &["confirm"]) {
        return Some("confirm");
    }
    if matches_any_token(token, &["submit"]) {
        return Some("submit");
    }
    if matches_any_token(token, &["issue", "grant", "quota"]) {
        return Some("grant");
    }
    if matches_any_token(token, &["export", "download", "preview"]) {
        return Some("export");
    }
    if matches_any_token(token, &["refund"]) {
        return Some("refund");
    }
    if matches_any_token(token, &["redeem"]) {
        return Some("redeem");
    }
    if matches_any_token(token, &["execute", "trigger", "run"]) {
        return Some("execute");
    }
    if matches_any_token(token, &["create", "add", "draft", "new"]) {
        return Some("create");
    }
    if matches_any_token(token, &["cancel", "revoke"]) {
        return Some("cancel");
    }
    if matches_any_token(token, &["pay", "payment", "checkout"]) {
        return Some("pay");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        build_candidate_aggregate, normalize_lookup_key, score_action_alias_candidate,
        score_auth_header_candidate, score_cookie_hint_candidate, score_field_candidate,
        CandidateBehaviorHint, CandidateObservation, CandidateSource,
    };

    fn build_observation(
        key: &str,
        source: CandidateSource,
        value_sample: Option<&str>,
        request_id: i64,
    ) -> CandidateObservation {
        CandidateObservation {
            key: key.to_string(),
            normalized_key: normalize_lookup_key(key),
            source,
            value_sample: value_sample.map(|item| item.to_string()),
            location: format!("query.{key}"),
            request_id,
            behavior_hint: None,
        }
    }

    #[test]
    fn behavior_can_promote_unknown_query_key_to_resource_candidate() {
        let aggregate = build_candidate_aggregate(vec![
            CandidateObservation {
                key: "fleof".to_string(),
                normalized_key: normalize_lookup_key("fleof"),
                source: CandidateSource::Query,
                value_sample: Some("CASE-9".to_string()),
                location: "query.fleof".to_string(),
                request_id: 1,
                behavior_hint: Some(CandidateBehaviorHint {
                    intent_hints: vec!["/workflows/detail".to_string(), "approve".to_string()],
                    behavior_steps: vec![
                        "打开页面 工单详情".to_string(),
                        "点击 approve".to_string(),
                    ],
                    last_page_title: Some("工单详情".to_string()),
                    last_page_url: Some("https://example.com/workflows/detail".to_string()),
                }),
            },
            CandidateObservation {
                key: "fleof".to_string(),
                normalized_key: normalize_lookup_key("fleof"),
                source: CandidateSource::Query,
                value_sample: Some("CASE-10".to_string()),
                location: "query.fleof".to_string(),
                request_id: 2,
                behavior_hint: Some(CandidateBehaviorHint {
                    intent_hints: vec!["/workflows/detail".to_string(), "approve".to_string()],
                    behavior_steps: vec![
                        "打开页面 工单详情".to_string(),
                        "点击 approve".to_string(),
                    ],
                    last_page_title: Some("工单详情".to_string()),
                    last_page_url: Some("https://example.com/workflows/detail".to_string()),
                }),
            },
        ])
        .expect("aggregate");

        let (category, score, _, _, _) = score_field_candidate(&aggregate).expect("candidate");
        assert_eq!(category.as_str(), "resource");
        assert!(score >= 35);
    }

    #[test]
    fn resource_candidate_scores_high_for_business_refs() {
        let aggregate = build_candidate_aggregate(vec![
            build_observation("biz_ref", CandidateSource::Query, Some("RF-9912"), 1),
            build_observation("biz-ref", CandidateSource::Query, Some("RF-9913"), 2),
        ])
        .expect("aggregate");

        let (category, score, _, _, _) = score_field_candidate(&aggregate).expect("candidate");
        assert_eq!(category.as_str(), "resource");
        assert!(score >= 45);
    }

    #[test]
    fn auth_header_candidate_detects_authorization() {
        let aggregate = build_candidate_aggregate(vec![
            build_observation(
                "Authorization",
                CandidateSource::Header,
                Some("Bearer abc123secret"),
                1,
            ),
            build_observation(
                "Authorization",
                CandidateSource::Header,
                Some("Bearer def456secret"),
                2,
            ),
        ])
        .expect("aggregate");

        assert!(score_auth_header_candidate(&aggregate).is_some());
    }

    #[test]
    fn cookie_hint_candidate_detects_session_cookie() {
        let aggregate = build_candidate_aggregate(vec![
            build_observation("tenant_session", CandidateSource::Cookie, Some("abc"), 1),
            build_observation("tenant_session", CandidateSource::Cookie, Some("def"), 2),
        ])
        .expect("aggregate");

        assert!(score_cookie_hint_candidate(&aggregate).is_some());
    }

    #[test]
    fn action_alias_candidate_maps_finalize_to_complete() {
        let aggregate = build_candidate_aggregate(vec![
            build_observation("finalize", CandidateSource::Path, None, 1),
            build_observation("finalize", CandidateSource::Path, None, 2),
        ])
        .expect("aggregate");

        let (_, _, canonical) = score_action_alias_candidate(&aggregate).expect("action candidate");
        assert_eq!(canonical.as_deref(), Some("complete"));
    }
}
