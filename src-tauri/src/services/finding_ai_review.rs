use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use sentinel_db::{DatabaseService, TrafficEvidenceRecord, TrafficVulnerabilityRecord};
use sentinel_llm::LlmClient;

const FINDING_AI_REVIEW_SYSTEM_PROMPT: &str = r#"你是一个安全漏洞复核助手。

任务目标：
1. 仅基于输入中的漏洞信息和证据进行判定。
2. 你的输出只能是 JSON，对象结构必须严格符合要求。
3. 你只能给出两种结论：
   - real_vulnerability: 证据足以支持这是真实漏洞
   - false_positive: 证据不足、与漏洞不符、或更像误报
4. 不允许输出 markdown，不允许输出代码块，不允许解释 JSON 之外的任何文字。
5. 不要假设你做了额外验证；如果证据不足，优先判为 false_positive，并在 uncertainties 中说明。
6. `primaryEvidence` 是主证据，包含完整请求体和完整响应体；`relatedEvidence` 是辅助证据，可能只保留摘要。判定时优先核对主证据里的请求-响应闭环。

输出 JSON Schema:
{
  "decision": "real_vulnerability" | "false_positive",
  "confidence": "high" | "medium" | "low",
  "summary": "一句话总结判定",
  "reasoning": ["判定依据1", "判定依据2"],
  "keyEvidence": ["最关键证据1", "最关键证据2"],
  "uncertainties": ["不确定点1", "不确定点2"],
  "recommendedAction": "后续建议"
}"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingAiReviewResult {
    pub finding_id: String,
    pub decision: String,
    pub applied_status: String,
    pub confidence: String,
    pub summary: String,
    pub reasoning: Vec<String>,
    pub key_evidence: Vec<String>,
    pub uncertainties: Vec<String>,
    pub recommended_action: String,
    pub provider: String,
    pub model: String,
    pub reviewed_at: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingAiReviewPromptInput {
    finding: FindingAiReviewPromptFinding,
    primary_evidence: Option<FindingAiReviewPromptEvidence>,
    related_evidence: Vec<FindingAiReviewPromptEvidence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingAiReviewPromptFinding {
    id: String,
    plugin_id: String,
    vuln_type: String,
    severity: String,
    confidence: String,
    title: String,
    description: String,
    cwe: Option<String>,
    owasp: Option<String>,
    remediation: Option<String>,
    status: String,
    hit_count: i64,
    first_seen_at: String,
    last_seen_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingAiReviewPromptEvidence {
    id: String,
    location: String,
    method: String,
    url: String,
    evidence_snippet: String,
    request_headers: Option<String>,
    request_body: Option<String>,
    response_status: Option<i32>,
    response_headers: Option<String>,
    response_body: Option<String>,
    timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindingAiReviewOutput {
    decision: String,
    confidence: String,
    summary: String,
    #[serde(default)]
    reasoning: Vec<String>,
    #[serde(default)]
    key_evidence: Vec<String>,
    #[serde(default)]
    uncertainties: Vec<String>,
    #[serde(default)]
    recommended_action: String,
}

pub async fn run_finding_ai_review(
    db_service: &DatabaseService,
    ai_manager: &crate::services::AiServiceManager,
    finding_id: &str,
) -> Result<FindingAiReviewResult> {
    let finding = db_service
        .get_traffic_vulnerability_by_id(finding_id)
        .await?
        .ok_or_else(|| anyhow!("未找到对应漏洞: {}", finding_id))?;
    let evidence = db_service.get_traffic_evidence_by_vuln_id(finding_id).await?;

    let prompt_input = build_prompt_input(&finding, &evidence);
    let prompt_json =
        serde_json::to_string_pretty(&prompt_input).context("Failed to serialize AI review input")?;

    let llm_config = ai_manager.resolve_generation_llm_config(None, None).await?;
    let provider = llm_config.provider.clone();
    let model = llm_config.model.clone();
    let client = LlmClient::new(llm_config);

    let raw = client
        .completion(Some(FINDING_AI_REVIEW_SYSTEM_PROMPT), &prompt_json)
        .await
        .context("AI finding review completion failed")?;
    let parsed = parse_review_output(&raw)?;
    let applied_status = map_decision_to_status(&parsed.decision)?;
    let reviewed_at = Utc::now();
    let evidence_id = format!("tai-{}", Uuid::new_v4());

    db_service
        .update_traffic_vulnerability_status(finding_id, applied_status)
        .await?;

    let review_evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding_id.to_string(),
        url: primary_review_url(&evidence),
        method: "AI".to_string(),
        location: "ai_review".to_string(),
        evidence_snippet: parsed.summary.clone(),
        request_headers: None,
        request_body: Some(prompt_json),
        response_status: None,
        response_headers: Some(
            json!({
                "analysisStage": "ai_review",
                "decision": parsed.decision,
                "confidence": parsed.confidence,
                "appliedStatus": applied_status,
                "provider": provider,
                "model": model,
                "reviewedAt": reviewed_at.to_rfc3339(),
            })
            .to_string(),
        ),
        response_body: Some(
            serde_json::to_string_pretty(&json!({
                "decision": parsed.decision,
                "confidence": parsed.confidence,
                "summary": parsed.summary,
                "reasoning": parsed.reasoning,
                "keyEvidence": parsed.key_evidence,
                "uncertainties": parsed.uncertainties,
                "recommendedAction": parsed.recommended_action,
            }))
            .context("Failed to serialize AI review output")?,
        ),
        timestamp: reviewed_at,
    };

    db_service.insert_traffic_evidence(&review_evidence).await?;

    Ok(FindingAiReviewResult {
        finding_id: finding_id.to_string(),
        decision: parsed.decision,
        applied_status: applied_status.to_string(),
        confidence: parsed.confidence,
        summary: parsed.summary,
        reasoning: parsed.reasoning,
        key_evidence: parsed.key_evidence,
        uncertainties: parsed.uncertainties,
        recommended_action: parsed.recommended_action,
        provider,
        model,
        reviewed_at: reviewed_at.to_rfc3339(),
        evidence_id,
    })
}

fn build_prompt_input(
    finding: &TrafficVulnerabilityRecord,
    evidence: &[TrafficEvidenceRecord],
) -> FindingAiReviewPromptInput {
    let filtered_evidence = evidence
        .iter()
        .filter(|item| item.location != "ai_review" && item.location != "system_agent_feedback")
        .collect::<Vec<_>>();
    let primary_evidence_id = select_primary_prompt_evidence(&filtered_evidence)
        .map(|item| item.id.as_str());

    FindingAiReviewPromptInput {
        finding: FindingAiReviewPromptFinding {
            id: finding.id.clone(),
            plugin_id: finding.plugin_id.clone(),
            vuln_type: finding.vuln_type.clone(),
            severity: finding.severity.clone(),
            confidence: finding.confidence.clone(),
            title: finding.title.clone(),
            description: finding.description.clone(),
            cwe: finding.cwe.clone(),
            owasp: finding.owasp.clone(),
            remediation: finding.remediation.clone(),
            status: finding.status.clone(),
            hit_count: finding.hit_count,
            first_seen_at: finding.first_seen_at.to_rfc3339(),
            last_seen_at: finding.last_seen_at.to_rfc3339(),
        },
        primary_evidence: primary_evidence_id.and_then(|id| {
            filtered_evidence
                .iter()
                .find(|item| item.id == id)
                .map(|item| build_prompt_evidence(item, true))
        }),
        related_evidence: filtered_evidence
            .iter()
            .filter(|item| Some(item.id.as_str()) != primary_evidence_id)
            .take(7)
            .map(|item| build_prompt_evidence(item, false))
            .collect(),
    }
}

fn select_primary_prompt_evidence<'a>(
    evidence: &'a [&'a TrafficEvidenceRecord],
) -> Option<&'a TrafficEvidenceRecord> {
    evidence
        .iter()
        .copied()
        .find(|item| !item.location.starts_with("system_agent_") && !item.url.trim().is_empty())
        .or_else(|| {
            evidence.iter().copied().find(|item| {
                !matches!(
                    item.location.as_str(),
                    "system_agent_verification" | "system_agent_context" | "system_agent_observation"
                )
            })
        })
        .or_else(|| evidence.first().copied())
}

fn build_prompt_evidence(
    item: &TrafficEvidenceRecord,
    include_full_exchange: bool,
) -> FindingAiReviewPromptEvidence {
    FindingAiReviewPromptEvidence {
        id: item.id.clone(),
        location: item.location.clone(),
        method: item.method.clone(),
        url: item.url.clone(),
        evidence_snippet: truncate_text(&item.evidence_snippet, 1600),
        request_headers: item
            .request_headers
            .as_deref()
            .map(|value| truncate_text(value, 1200)),
        request_body: item.request_body.as_deref().map(|value| {
            if include_full_exchange {
                value.trim().to_string()
            } else {
                truncate_text(value, 2400)
            }
        }),
        response_status: item.response_status,
        response_headers: item
            .response_headers
            .as_deref()
            .map(|value| truncate_text(value, 1200)),
        response_body: item.response_body.as_deref().map(|value| {
            if include_full_exchange {
                value.trim().to_string()
            } else {
                truncate_text(value, 2400)
            }
        }),
        timestamp: item.timestamp.to_rfc3339(),
    }
}

fn primary_review_url(evidence: &[TrafficEvidenceRecord]) -> String {
    evidence
        .iter()
        .find(|item| !item.url.trim().is_empty())
        .map(|item| item.url.clone())
        .unwrap_or_default()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    trimmed.chars().take(max_chars).collect::<String>()
}

fn parse_review_output(raw: &str) -> Result<FindingAiReviewOutput> {
    let normalized = extract_json_object(raw).ok_or_else(|| anyhow!("AI 未返回有效 JSON"))?;
    let parsed: FindingAiReviewOutput =
        serde_json::from_str(&normalized).context("AI 返回 JSON 结构不符合要求")?;
    validate_review_output(parsed)
}

fn extract_json_object(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim();
    let candidate = without_prefix
        .strip_suffix("```")
        .unwrap_or(without_prefix)
        .trim()
        .to_string();

    if candidate.starts_with('{') && candidate.ends_with('}') {
        return Some(candidate);
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    Some(trimmed[start..=end].to_string())
}

fn validate_review_output(parsed: FindingAiReviewOutput) -> Result<FindingAiReviewOutput> {
    let decision = parsed.decision.trim();
    if !matches!(decision, "real_vulnerability" | "false_positive") {
        return Err(anyhow!("AI decision 无效: {}", parsed.decision));
    }

    let confidence = parsed.confidence.trim();
    if !matches!(confidence, "high" | "medium" | "low") {
        return Err(anyhow!("AI confidence 无效: {}", parsed.confidence));
    }

    if parsed.summary.trim().is_empty() {
        return Err(anyhow!("AI summary 不能为空"));
    }

    Ok(FindingAiReviewOutput {
        decision: decision.to_string(),
        confidence: confidence.to_string(),
        summary: parsed.summary.trim().to_string(),
        reasoning: normalize_string_list(parsed.reasoning, 6),
        key_evidence: normalize_string_list(parsed.key_evidence, 6),
        uncertainties: normalize_string_list(parsed.uncertainties, 6),
        recommended_action: parsed.recommended_action.trim().to_string(),
    })
}

fn normalize_string_list(items: Vec<String>, max_items: usize) -> Vec<String> {
    items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .take(max_items)
        .collect()
}

fn map_decision_to_status(decision: &str) -> Result<&'static str> {
    match decision {
        "real_vulnerability" => Ok("reviewed"),
        "false_positive" => Ok("false_positive"),
        other => Err(anyhow!("不支持的 AI 判定: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{
        build_prompt_input, map_decision_to_status, parse_review_output, TrafficEvidenceRecord,
        TrafficVulnerabilityRecord,
    };

    #[test]
    fn parses_json_review_output() {
        let parsed = parse_review_output(
            r#"{
              "decision": "real_vulnerability",
              "confidence": "high",
              "summary": "可确认为真实漏洞",
              "reasoning": ["存在稳定的异常回显"],
              "keyEvidence": ["请求与响应形成闭环"],
              "uncertainties": [],
              "recommendedAction": "进入正式跟进"
            }"#,
        )
        .expect("expected valid review output");

        assert_eq!(parsed.decision, "real_vulnerability");
        assert_eq!(parsed.confidence, "high");
        assert_eq!(parsed.reasoning, vec!["存在稳定的异常回显"]);
    }

    #[test]
    fn rejects_invalid_decision() {
        let error = parse_review_output(
            r#"{
              "decision": "maybe",
              "confidence": "high",
              "summary": "invalid",
              "reasoning": [],
              "keyEvidence": [],
              "uncertainties": [],
              "recommendedAction": ""
            }"#,
        )
        .expect_err("expected invalid decision");

        assert!(error.to_string().contains("decision"));
    }

    #[test]
    fn maps_decision_to_status() {
        assert_eq!(
            map_decision_to_status("real_vulnerability").expect("status"),
            "reviewed"
        );
        assert_eq!(
            map_decision_to_status("false_positive").expect("status"),
            "false_positive"
        );
    }

    #[test]
    fn keeps_full_exchange_for_primary_evidence_only() {
        let finding = TrafficVulnerabilityRecord {
            id: "f-1".to_string(),
            plugin_id: "plugin:test".to_string(),
            vuln_type: "xss".to_string(),
            severity: "high".to_string(),
            confidence: "high".to_string(),
            title: "xss".to_string(),
            description: "desc".to_string(),
            cwe: None,
            owasp: None,
            remediation: None,
            status: "open".to_string(),
            signature: "sig".to_string(),
            first_seen_at: Utc::now(),
            last_seen_at: Utc::now(),
            hit_count: 1,
            session_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let primary_body = "A".repeat(5000);
        let secondary_body = "B".repeat(5000);
        let evidence = vec![
            TrafficEvidenceRecord {
                id: "e-1".to_string(),
                vuln_id: "f-1".to_string(),
                url: "https://example.com".to_string(),
                method: "POST".to_string(),
                location: "response_body".to_string(),
                evidence_snippet: "primary".to_string(),
                request_headers: None,
                request_body: Some(primary_body.clone()),
                response_status: Some(200),
                response_headers: None,
                response_body: Some(primary_body.clone()),
                timestamp: Utc::now(),
            },
            TrafficEvidenceRecord {
                id: "e-2".to_string(),
                vuln_id: "f-1".to_string(),
                url: "https://example.com/2".to_string(),
                method: "POST".to_string(),
                location: "response_body".to_string(),
                evidence_snippet: "secondary".to_string(),
                request_headers: None,
                request_body: Some(secondary_body.clone()),
                response_status: Some(200),
                response_headers: None,
                response_body: Some(secondary_body),
                timestamp: Utc::now(),
            },
        ];

        let prompt = build_prompt_input(&finding, &evidence);
        let primary = prompt.primary_evidence.expect("primary evidence");
        let secondary = prompt.related_evidence.first().expect("secondary evidence");

        assert_eq!(primary.request_body.as_deref(), Some(primary_body.as_str()));
        assert_eq!(primary.response_body.as_deref(), Some(primary_body.as_str()));
        assert!(secondary.request_body.as_deref().unwrap_or_default().len() < 5000);
        assert!(secondary.response_body.as_deref().unwrap_or_default().len() < 5000);
    }
}
