//! Finding management service

use chrono::Utc;
use crate::error::{BountyError, Result};
use sentinel_db::{BountyFindingRow, DatabaseService};
use uuid::Uuid;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CreateFindingInput {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: String,
    pub description: String,
    pub finding_type: String,
    pub severity: Option<String>,
    pub confidence: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct UpdateFindingInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub finding_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub tags: Option<Vec<String>>,
    pub duplicate_of: Option<String>,
}

pub struct FindingService;

#[derive(Debug, Clone)]
pub struct SimilarityConfig {
    pub title_weight: f64,
    pub description_weight: f64,
    pub url_weight: f64,
    pub parameter_weight: f64,
    pub threshold: f64,
    pub candidate_limit: i64,
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            title_weight: 0.35,
            description_weight: 0.35,
            url_weight: 0.2,
            parameter_weight: 0.1,
            threshold: 0.85,
            candidate_limit: 200,
        }
    }
}

impl FindingService {
    pub async fn create_finding(
        db: &DatabaseService,
        input: CreateFindingInput,
    ) -> Result<BountyFindingRow> {
        validate_required(&input.title, "title")?;
        validate_required(&input.description, "description")?;
        validate_required(&input.finding_type, "finding_type")?;

        let now = Utc::now().to_rfc3339();

        let fingerprint = calculate_finding_fingerprint(
            &input.program_id,
            &input.finding_type,
            input.affected_url.as_deref(),
            input.affected_parameter.as_deref(),
            &input.title,
            &input.description,
            input.asset_id.as_deref(),
        );

        if let Some(existing) = db
            .get_bounty_finding_by_fingerprint(&fingerprint)
            .await?
        {
            return Err(BountyError::DuplicateFinding(format!("Duplicate finding exists: {}", existing.id)));
        }

        let mut duplicate_of: Option<String> = None;
        let mut status_override: Option<String> = None;

        let similarity = SimilarityConfig::default();
        if let Some((id, score)) = find_similar_finding(db, &input, &similarity).await? {
            if score >= similarity.threshold {
                duplicate_of = Some(id);
                status_override = Some("duplicate".to_string());
            }
        }

        let finding = BountyFindingRow {
            id: Uuid::new_v4().to_string(),
            program_id: input.program_id,
            scope_id: input.scope_id,
            asset_id: input.asset_id,
            title: input.title,
            description: input.description,
            finding_type: input.finding_type,
            severity: input.severity.unwrap_or_else(|| "medium".to_string()),
            status: status_override.unwrap_or_else(|| "new".to_string()),
            confidence: input.confidence.unwrap_or_else(|| "medium".to_string()),
            cvss_score: input.cvss_score,
            cwe_id: input.cwe_id,
            affected_url: input.affected_url,
            affected_parameter: input.affected_parameter,
            reproduction_steps_json: input
                .reproduction_steps
                .map(|s| serde_json::to_string(&s).unwrap_or_default()),
            impact: input.impact,
            remediation: input.remediation,
            evidence_ids_json: None,
            tags_json: input.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
            metadata_json: None,
            fingerprint,
            duplicate_of,
            first_seen_at: now.clone(),
            last_seen_at: now.clone(),
            verified_at: None,
            created_at: now.clone(),
            updated_at: now,
            created_by: "user".to_string(),
        };

        db.create_bounty_finding(&finding)
            .await?;
        Ok(finding)
    }

    pub async fn update_finding(
        db: &DatabaseService,
        id: &str,
        input: UpdateFindingInput,
    ) -> Result<bool> {
        let existing = db
            .get_bounty_finding(id)
            .await?;

        let Some(mut finding) = existing else {
            return Ok(false);
        };

        if let Some(title) = input.title { finding.title = title; }
        if let Some(description) = input.description { finding.description = description; }
        if let Some(finding_type) = input.finding_type { finding.finding_type = finding_type; }
        if let Some(severity) = input.severity { finding.severity = severity; }
        if let Some(status) = input.status { finding.status = status; }
        if let Some(confidence) = input.confidence { finding.confidence = confidence; }
        if input.cvss_score.is_some() { finding.cvss_score = input.cvss_score; }
        if input.cwe_id.is_some() { finding.cwe_id = input.cwe_id; }
        if input.affected_url.is_some() { finding.affected_url = input.affected_url; }
        if input.affected_parameter.is_some() { finding.affected_parameter = input.affected_parameter; }
        if let Some(steps) = input.reproduction_steps {
            finding.reproduction_steps_json = Some(serde_json::to_string(&steps).unwrap_or_default());
        }
        if input.impact.is_some() { finding.impact = input.impact; }
        if input.remediation.is_some() { finding.remediation = input.remediation; }
        if let Some(tags) = input.tags {
            finding.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
        }
        if input.duplicate_of.is_some() { finding.duplicate_of = input.duplicate_of; }

        let new_fingerprint = calculate_finding_fingerprint(
            &finding.program_id,
            &finding.finding_type,
            finding.affected_url.as_deref(),
            finding.affected_parameter.as_deref(),
            &finding.title,
            &finding.description,
            finding.asset_id.as_deref(),
        );
        if new_fingerprint != finding.fingerprint {
            if let Some(existing) = db
                .get_bounty_finding_by_fingerprint_excluding(&new_fingerprint, &finding.id)
                .await?
            {
                return Err(BountyError::DuplicateFinding(format!("Duplicate finding exists: {}", existing.id)));
            }
            finding.fingerprint = new_fingerprint;
        }

        let similarity = SimilarityConfig::default();
        if let Some((similar_id, score)) = find_similar_finding_from_row(db, &finding, &similarity, Some(&finding.id)).await? {
            if score >= similarity.threshold {
                return Err(BountyError::DuplicateFinding(format!("Similar finding exists: {} (score {:.2})", similar_id, score)));
            }
        }

        finding.last_seen_at = Utc::now().to_rfc3339();
        finding.updated_at = Utc::now().to_rfc3339();

        db.update_bounty_finding(&finding)
            .await
            .map_err(|e| e.into())
    }
}

fn validate_required(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(BountyError::Validation(format!("{} is required", field)));
    }
    Ok(())
}

fn calculate_finding_fingerprint(
    program_id: &str,
    finding_type: &str,
    affected_url: Option<&str>,
    affected_parameter: Option<&str>,
    title: &str,
    description: &str,
    asset_id: Option<&str>,
) -> String {
    let url_key = affected_url
        .and_then(|u| canonicalize_url(u))
        .unwrap_or_default();
    let param_key = affected_parameter.unwrap_or("").trim().to_lowercase();
    let title_key = title.trim().to_lowercase();
    let desc_key = description.trim().to_lowercase();
    let asset_key = asset_id.unwrap_or("").trim().to_lowercase();

    let basis = if !url_key.is_empty() || !param_key.is_empty() {
        format!("{}:{}:{}:{}", program_id, finding_type, url_key, param_key)
    } else {
        format!("{}:{}:{}:{}:{}", program_id, finding_type, title_key, desc_key, asset_key)
    };

    format!("{:x}", md5::compute(basis.as_bytes()))
}

fn canonicalize_url(url: &str) -> Option<String> {
    use url::Url;
    if let Ok(parsed) = Url::parse(url) {
        let hostname = parsed.host_str().map(|s| s.to_lowercase());
        let port = parsed.port().map(|p| p as i32);
        let path = parsed.path();
        let canonical = format!(
            "{}://{}{}{}",
            parsed.scheme(),
            hostname.as_deref().unwrap_or(""),
            port.map(|p| format!(":{}", p)).unwrap_or_default(),
            path
        )
        .to_lowercase();
        return Some(canonical);
    }
    if url.trim().is_empty() {
        None
    } else {
        Some(url.trim().to_lowercase())
    }
}

async fn find_similar_finding(
    db: &DatabaseService,
    input: &CreateFindingInput,
    config: &SimilarityConfig,
) -> Result<Option<(String, f64)>> {
    let candidates = db
        .list_bounty_findings_for_similarity(&input.program_id, &input.finding_type, config.candidate_limit)
        .await?;

    let mut best: Option<(String, f64)> = None;
    for candidate in candidates {
        let score = similarity_score(
            &input.title,
            &input.description,
            input.affected_url.as_deref(),
            input.affected_parameter.as_deref(),
            &candidate.title,
            &candidate.description,
            candidate.affected_url.as_deref(),
            candidate.affected_parameter.as_deref(),
            config,
        );
        if best.as_ref().map(|b| score > b.1).unwrap_or(true) {
            best = Some((candidate.id, score));
        }
    }
    Ok(best)
}

async fn find_similar_finding_from_row(
    db: &DatabaseService,
    row: &BountyFindingRow,
    config: &SimilarityConfig,
    exclude_id: Option<&str>,
) -> Result<Option<(String, f64)>> {
    let candidates = db
        .list_bounty_findings_for_similarity(&row.program_id, &row.finding_type, config.candidate_limit)
        .await?;

    let mut best: Option<(String, f64)> = None;
    for candidate in candidates {
        if exclude_id.map(|id| id == candidate.id).unwrap_or(false) {
            continue;
        }
        let score = similarity_score(
            &row.title,
            &row.description,
            row.affected_url.as_deref(),
            row.affected_parameter.as_deref(),
            &candidate.title,
            &candidate.description,
            candidate.affected_url.as_deref(),
            candidate.affected_parameter.as_deref(),
            config,
        );
        if best.as_ref().map(|b| score > b.1).unwrap_or(true) {
            best = Some((candidate.id, score));
        }
    }
    Ok(best)
}

fn similarity_score(
    title_a: &str,
    desc_a: &str,
    url_a: Option<&str>,
    param_a: Option<&str>,
    title_b: &str,
    desc_b: &str,
    url_b: Option<&str>,
    param_b: Option<&str>,
    config: &SimilarityConfig,
) -> f64 {
    let mut total_weight = 0.0;
    let mut score = 0.0;

    let title_sim = jaccard_similarity(title_a, title_b);
    total_weight += config.title_weight;
    score += config.title_weight * title_sim;

    let desc_sim = jaccard_similarity(desc_a, desc_b);
    total_weight += config.description_weight;
    score += config.description_weight * desc_sim;

    if let (Some(a), Some(b)) = (url_a.and_then(canonicalize_url), url_b.and_then(canonicalize_url)) {
        total_weight += config.url_weight;
        score += config.url_weight * if a == b { 1.0 } else { 0.0 };
    }

    if let (Some(a), Some(b)) = (param_a, param_b) {
        if !a.trim().is_empty() && !b.trim().is_empty() {
            total_weight += config.parameter_weight;
            score += config.parameter_weight * if a.trim().eq_ignore_ascii_case(b.trim()) { 1.0 } else { 0.0 };
        }
    }

    if total_weight <= 0.0 {
        0.0
    } else {
        score / total_weight
    }
}

fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a = tokenize(a);
    let set_b = tokenize(b);
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 { 0.0 } else { intersection / union }
}

fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}
