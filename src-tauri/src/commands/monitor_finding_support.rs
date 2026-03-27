use std::sync::Arc;

use chrono::Utc;
use sentinel_bounty::services::MonitorTask;
use sentinel_db::{BountyFindingRow, DatabaseService};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub(crate) struct MonitorFindingImportStats {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
struct MonitorFindingCandidate {
    title: String,
    description: String,
    finding_type: String,
    severity: String,
    confidence: String,
    cwe_id: Option<String>,
    affected_url: Option<String>,
    remediation: Option<String>,
    tags: Vec<String>,
    metadata: Value,
}

pub(crate) async fn import_monitor_findings_from_output(
    db_service: &Arc<DatabaseService>,
    task: &MonitorTask,
    plugin_id: &str,
    output: &Value,
    execution_mode: &str,
) -> Result<MonitorFindingImportStats, String> {
    let data = output.get("data").unwrap_or(output);
    let findings = data
        .get("findings")
        .and_then(|value| value.as_array())
        .filter(|items| !items.is_empty())
        .or_else(|| {
            data.get("surface_artifacts")
                .and_then(|value| value.get("findings"))
                .and_then(|value| value.as_array())
                .filter(|items| !items.is_empty())
        });

    let Some(findings) = findings else {
        return Ok(MonitorFindingImportStats::default());
    };

    let mut stats = MonitorFindingImportStats::default();

    for finding in findings {
        let Some(candidate) =
            build_monitor_finding_candidate(finding, plugin_id, task, execution_mode)
        else {
            stats.skipped += 1;
            continue;
        };

        let fingerprint_seed = format!(
            "{}|{}|{}|{}|{}",
            task.program_id,
            plugin_id,
            candidate.finding_type,
            candidate.affected_url.clone().unwrap_or_default(),
            candidate.title
        );
        let fingerprint = format!("{:x}", md5::compute(fingerprint_seed.as_bytes()));
        let now = Utc::now().to_rfc3339();

        if let Some(mut existing) = db_service
            .get_bounty_finding_by_fingerprint(&fingerprint)
            .await
            .map_err(|error| error.to_string())?
        {
            existing.title = candidate.title;
            existing.description = candidate.description;
            existing.finding_type = candidate.finding_type;
            existing.severity = candidate.severity;
            existing.confidence = candidate.confidence;
            existing.cwe_id = candidate.cwe_id;
            existing.affected_url = candidate.affected_url;
            existing.remediation = candidate.remediation;
            existing.tags_json = serialize_json_array(&candidate.tags);
            existing.metadata_json = Some(candidate.metadata.to_string());
            existing.last_seen_at = now.clone();
            existing.updated_at = now.clone();

            db_service
                .update_bounty_finding(&existing)
                .await
                .map_err(|error| error.to_string())?;
            stats.updated += 1;
            continue;
        }

        let created_finding = BountyFindingRow {
            id: Uuid::new_v4().to_string(),
            program_id: task.program_id.clone(),
            scope_id: None,
            asset_id: None,
            title: candidate.title,
            description: candidate.description,
            finding_type: candidate.finding_type,
            severity: candidate.severity,
            status: "new".to_string(),
            confidence: candidate.confidence,
            cvss_score: None,
            cwe_id: candidate.cwe_id,
            affected_url: candidate.affected_url,
            affected_parameter: None,
            reproduction_steps_json: None,
            impact: None,
            remediation: candidate.remediation,
            evidence_ids_json: None,
            tags_json: serialize_json_array(&candidate.tags),
            metadata_json: Some(candidate.metadata.to_string()),
            fingerprint,
            duplicate_of: None,
            first_seen_at: now.clone(),
            last_seen_at: now.clone(),
            verified_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            created_by: "monitor".to_string(),
        };

        db_service
            .create_bounty_finding(&created_finding)
            .await
            .map_err(|error| error.to_string())?;
        stats.created += 1;
    }

    Ok(stats)
}

fn build_monitor_finding_candidate(
    finding: &Value,
    plugin_id: &str,
    task: &MonitorTask,
    execution_mode: &str,
) -> Option<MonitorFindingCandidate> {
    let title = finding
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;

    let finding_type = finding
        .get("finding_type")
        .and_then(Value::as_str)
        .or_else(|| finding.get("vulnerability_type").and_then(Value::as_str))
        .or_else(|| finding.get("category").and_then(Value::as_str))
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalized_plugin_name(plugin_id));

    let affected_url = finding
        .get("affected_url")
        .and_then(Value::as_str)
        .or_else(|| finding.get("url").and_then(Value::as_str))
        .or_else(|| finding.get("target").and_then(Value::as_str))
        .map(normalize_text)
        .filter(|value| !value.is_empty());

    let description = finding
        .get("description")
        .and_then(Value::as_str)
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            affected_url
                .as_ref()
                .map(|url| format!("Monitor plugin {} reported a finding on {}", plugin_id, url))
        })
        .unwrap_or_else(|| format!("Monitor plugin {} reported a finding", plugin_id));

    let severity = finding
        .get("severity")
        .and_then(Value::as_str)
        .map(normalize_severity)
        .unwrap_or_else(|| "medium".to_string());

    let confidence = finding
        .get("confidence")
        .and_then(Value::as_str)
        .map(normalize_confidence)
        .unwrap_or_else(|| "medium".to_string());

    let remediation = finding
        .get("remediation")
        .and_then(Value::as_str)
        .map(normalize_text)
        .filter(|value| !value.is_empty());

    let cwe_id = finding
        .get("cwe_id")
        .and_then(Value::as_str)
        .or_else(|| finding.get("cwe").and_then(Value::as_str))
        .map(normalize_text)
        .filter(|value| !value.is_empty());

    let tags = finding
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_text)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let metadata = json!({
        "source": "monitor",
        "plugin_id": plugin_id,
        "task_id": task.id,
        "task_name": task.name,
        "execution_mode": execution_mode,
        "affected_url": affected_url,
        "evidence": finding.get("evidence").cloned(),
        "raw_finding": finding,
    });

    Some(MonitorFindingCandidate {
        title,
        description,
        finding_type,
        severity,
        confidence,
        cwe_id,
        affected_url,
        remediation,
        tags,
        metadata,
    })
}

fn normalized_plugin_name(plugin_id: &str) -> String {
    plugin_id
        .strip_prefix("plugin__")
        .unwrap_or(plugin_id)
        .trim()
        .to_lowercase()
}

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_severity(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "critical" | "high" | "medium" | "low" | "info" => value.trim().to_lowercase(),
        _ => "medium".to_string(),
    }
}

fn normalize_confidence(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "high" | "medium" | "low" => value.trim().to_lowercase(),
        _ => "medium".to_string(),
    }
}

fn serialize_json_array(items: &[String]) -> Option<String> {
    if items.is_empty() {
        None
    } else {
        serde_json::to_string(items).ok()
    }
}
