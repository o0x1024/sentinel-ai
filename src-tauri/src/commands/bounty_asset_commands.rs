use crate::commands::bounty_commands::ensure_bounty_feature;
use chrono::Utc;
use sentinel_db::{BountyAssetRow, DatabaseService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

// ============================================================================
// Bounty Asset Commands (P1-B3: Asset Consolidation)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssetRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_type: Option<String>,
    pub url: String,
    pub tags: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
}

/// Canonicalize a URL
pub(crate) fn canonicalize_url(
    url: &str,
) -> (
    String,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
) {
    use url::Url;

    if let Ok(parsed) = Url::parse(url) {
        let hostname = parsed.host_str().map(|s| s.to_lowercase());
        let port = parsed.port().map(|p| p as i32);
        let path = Some(parsed.path().to_string());
        let protocol = Some(parsed.scheme().to_string());

        // Build canonical URL (normalized)
        let canonical = format!(
            "{}://{}{}{}",
            parsed.scheme(),
            hostname.as_deref().unwrap_or(""),
            port.map(|p| format!(":{}", p)).unwrap_or_default(),
            parsed.path()
        )
        .to_lowercase();

        (canonical, hostname, port, path, protocol)
    } else {
        // If URL parsing fails, use the original
        (url.to_lowercase(), None, None, None, None)
    }
}

/// Create or merge a bounty asset
#[tauri::command]
pub async fn bounty_create_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateAssetRequest,
) -> Result<BountyAssetRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();
    let (canonical_url, hostname, port, path, protocol) = canonicalize_url(&request.url);

    // Check if asset already exists by canonical URL
    if let Some(mut existing) = db_service
        .get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
        .await
        .map_err(|e| e.to_string())?
    {
        // Merge: add original URL if different
        db_service
            .merge_bounty_asset_url(&existing.id, &request.url)
            .await
            .map_err(|e| e.to_string())?;

        // Update last_seen_at
        existing.last_seen_at = now;
        db_service
            .update_bounty_asset(&existing)
            .await
            .map_err(|e| e.to_string())?;

        return Ok(existing);
    }

    let asset = BountyAssetRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_id: request.scope_id,
        asset_type: request.asset_type.unwrap_or_else(|| "url".to_string()),
        canonical_url,
        original_urls_json: Some(serde_json::to_string(&vec![request.url]).unwrap_or_default()),
        hostname,
        port,
        path,
        protocol,
        ip_addresses_json: None,
        dns_records_json: None,
        tech_stack_json: None,
        fingerprint: None,
        tags_json: request
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        labels_json: request
            .labels
            .map(|l| serde_json::to_string(&l).unwrap_or_default()),
        priority_score: Some(0.0),
        risk_score: Some(0.0),
        is_alive: true,
        last_checked_at: None,
        first_seen_at: now.clone(),
        last_seen_at: now.clone(),
        findings_count: 0,
        change_events_count: 0,
        metadata_json: None,
        created_at: now.clone(),
        updated_at: now,
        // ASM fields - all None by default
        ip_version: None,
        asn: None,
        asn_org: None,
        isp: None,
        country: None,
        city: None,
        latitude: None,
        longitude: None,
        is_cloud: None,
        cloud_provider: None,
        service_name: None,
        service_version: None,
        service_product: None,
        banner: None,
        transport_protocol: None,
        cpe: None,
        domain_registrar: None,
        registration_date: None,
        expiration_date: None,
        nameservers_json: None,
        mx_records_json: None,
        txt_records_json: None,
        whois_data_json: None,
        is_wildcard: None,
        parent_domain: None,
        root_domain: None,
        subdomain_level: None,
        http_status: None,
        response_time_ms: None,
        content_length: None,
        content_type: None,
        title: None,
        favicon_hash: None,
        headers_json: None,
        waf_detected: None,
        cdn_detected: None,
        screenshot_path: None,
        body_hash: None,
        certificate_id: None,
        ssl_enabled: None,
        certificate_subject: None,
        certificate_issuer: None,
        certificate_valid_from: None,
        certificate_valid_to: None,
        certificate_san_json: None,
        exposure_level: None,
        attack_surface_score: None,
        vulnerability_count: None,
        cvss_max_score: None,
        exploit_available: None,
        asset_category: None,
        asset_owner: None,
        business_unit: None,
        criticality: None,
        discovery_method: None,
        data_sources_json: None,
        confidence_score: None,
        monitoring_enabled: None,
        scan_frequency: None,
        last_scan_type: None,
        parent_asset_id: None,
        related_assets_json: None,
    };

    db_service
        .create_bounty_asset(&asset)
        .await
        .map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Bulk import assets from scope
#[tauri::command]
pub async fn bounty_import_assets_from_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    scope_id: String,
) -> Result<i32, String> {
    ensure_bounty_feature()?;

    // Get scope
    let scope = db_service
        .get_program_scope(&scope_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Scope not found".to_string())?;

    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    // Parse scope target as URLs or domains
    let values: Vec<&str> = scope
        .target
        .lines()
        .filter(|l: &&str| !l.trim().is_empty())
        .collect();

    let mut candidates = Vec::with_capacity(values.len());
    let mut canonical_urls = Vec::with_capacity(values.len());
    for value in values {
        let url = if value.starts_with("http://") || value.starts_with("https://") {
            value.to_string()
        } else {
            format!("https://{}", value)
        };

        let (canonical_url, hostname, port, path, protocol) = canonicalize_url(&url);
        canonical_urls.push(canonical_url.clone());
        candidates.push((url, canonical_url, hostname, port, path, protocol));
    }

    let existing_urls = db_service
        .list_existing_bounty_asset_canonical_urls(&program_id, &canonical_urls)
        .await
        .map_err(|e| e.to_string())?;
    let mut imported_urls = HashSet::new();

    for (url, canonical_url, hostname, port, path, protocol) in candidates {
        if existing_urls.contains(&canonical_url) || !imported_urls.insert(canonical_url.clone()) {
            continue;
        }

        let asset = BountyAssetRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.clone(),
            scope_id: Some(scope_id.clone()),
            asset_type: match scope.scope_type.as_str() {
                "domain" | "wildcard" => "domain".to_string(),
                "url" => "url".to_string(),
                "ip" | "ip_range" => "ip".to_string(),
                _ => "other".to_string(),
            },
            canonical_url,
            original_urls_json: Some(serde_json::to_string(&vec![url]).unwrap_or_default()),
            hostname,
            port,
            path,
            protocol,
            ip_addresses_json: None,
            dns_records_json: None,
            tech_stack_json: None,
            fingerprint: None,
            tags_json: None,
            labels_json: None,
            priority_score: Some(0.0),
            risk_score: Some(0.0),
            is_alive: true,
            last_checked_at: None,
            first_seen_at: now.clone(),
            last_seen_at: now.clone(),
            findings_count: 0,
            change_events_count: 0,
            metadata_json: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            // ASM fields - all None by default
            ip_version: None,
            asn: None,
            asn_org: None,
            isp: None,
            country: None,
            city: None,
            latitude: None,
            longitude: None,
            is_cloud: None,
            cloud_provider: None,
            service_name: None,
            service_version: None,
            service_product: None,
            banner: None,
            transport_protocol: None,
            cpe: None,
            domain_registrar: None,
            registration_date: None,
            expiration_date: None,
            nameservers_json: None,
            mx_records_json: None,
            txt_records_json: None,
            whois_data_json: None,
            is_wildcard: None,
            parent_domain: None,
            root_domain: None,
            subdomain_level: None,
            http_status: None,
            response_time_ms: None,
            content_length: None,
            content_type: None,
            title: None,
            favicon_hash: None,
            headers_json: None,
            waf_detected: None,
            cdn_detected: None,
            screenshot_path: None,
            body_hash: None,
            certificate_id: None,
            ssl_enabled: None,
            certificate_subject: None,
            certificate_issuer: None,
            certificate_valid_from: None,
            certificate_valid_to: None,
            certificate_san_json: None,
            exposure_level: None,
            attack_surface_score: None,
            vulnerability_count: None,
            cvss_max_score: None,
            exploit_available: None,
            asset_category: None,
            asset_owner: None,
            business_unit: None,
            criticality: None,
            discovery_method: None,
            data_sources_json: None,
            confidence_score: None,
            monitoring_enabled: None,
            scan_frequency: None,
            last_scan_type: None,
            parent_asset_id: None,
            related_assets_json: None,
        };

        db_service
            .create_bounty_asset(&asset)
            .await
            .map_err(|e| e.to_string())?;
        count += 1;
    }

    Ok(count)
}

// ============================================================================
// P1-B4: Fingerprint & Label System
// ============================================================================

/// Predefined high-value labels
pub const HIGH_VALUE_LABELS: &[&str] = &[
    "admin-panel",
    "api-endpoint",
    "auth-system",
    "payment-gateway",
    "user-data",
    "file-upload",
    "debug-enabled",
    "exposed-config",
    "vulnerable-tech",
    "outdated-software",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssetFingerprintRequest {
    pub asset_id: String,
    pub tech_stack: Option<Vec<TechStackItem>>,
    pub ip_addresses: Option<Vec<String>>,
    pub dns_records: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechStackItem {
    pub name: String,
    pub version: Option<String>,
    pub category: String, // "framework", "server", "cms", "library", "language"
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAssetLabelsRequest {
    pub asset_id: String,
    pub labels: Vec<String>,
}

/// Generate fingerprint from asset data
fn generate_asset_fingerprint(
    hostname: Option<&str>,
    tech_stack: &[TechStackItem],
    port: Option<i32>,
) -> String {
    let mut data = String::new();

    if let Some(h) = hostname {
        data.push_str(h);
    }
    if let Some(p) = port {
        data.push_str(&format!(":{}", p));
    }

    // Sort tech stack for consistent fingerprint
    let mut techs: Vec<String> = tech_stack
        .iter()
        .map(|t| format!("{}:{}", t.name, t.version.as_deref().unwrap_or("")))
        .collect();
    techs.sort();
    data.push_str(&techs.join(","));

    format!("{:x}", md5::compute(data.as_bytes()))
}

/// Update asset fingerprint and tech stack
#[tauri::command]
pub async fn bounty_update_asset_fingerprint(
    db_service: State<'_, Arc<DatabaseService>>,
    request: UpdateAssetFingerprintRequest,
) -> Result<BountyAssetRow, String> {
    ensure_bounty_feature()?;

    let mut asset = db_service
        .get_bounty_asset(&request.asset_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;

    let now = Utc::now().to_rfc3339();

    // Update tech stack
    if let Some(tech_stack) = &request.tech_stack {
        asset.tech_stack_json = Some(serde_json::to_string(tech_stack).unwrap_or_default());

        // Generate fingerprint
        asset.fingerprint = Some(generate_asset_fingerprint(
            asset.hostname.as_deref(),
            tech_stack,
            asset.port,
        ));

        // Auto-add labels based on tech stack
        let mut labels: Vec<String> = asset
            .labels_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        for tech in tech_stack {
            // Check for high-value technologies
            let tech_lower = tech.name.to_lowercase();
            if tech_lower.contains("admin") || tech_lower.contains("管理") {
                if !labels.contains(&"admin-panel".to_string()) {
                    labels.push("admin-panel".to_string());
                }
            }
            if tech_lower.contains("api")
                || tech_lower.contains("graphql")
                || tech_lower.contains("rest")
            {
                if !labels.contains(&"api-endpoint".to_string()) {
                    labels.push("api-endpoint".to_string());
                }
            }
            if tech_lower.contains("debug") || tech_lower.contains("devtools") {
                if !labels.contains(&"debug-enabled".to_string()) {
                    labels.push("debug-enabled".to_string());
                }
            }
            // Check for outdated/vulnerable versions
            if let Some(version) = &tech.version {
                if is_potentially_vulnerable(&tech.name, version) {
                    if !labels.contains(&"vulnerable-tech".to_string()) {
                        labels.push("vulnerable-tech".to_string());
                    }
                }
            }
        }

        asset.labels_json = Some(serde_json::to_string(&labels).unwrap_or_default());
    }

    // Update IP addresses
    if let Some(ips) = request.ip_addresses {
        asset.ip_addresses_json = Some(serde_json::to_string(&ips).unwrap_or_default());
    }

    // Update DNS records
    if let Some(dns) = request.dns_records {
        asset.dns_records_json = Some(serde_json::to_string(&dns).unwrap_or_default());
    }

    asset.last_checked_at = Some(now.clone());
    asset.updated_at = now;

    db_service
        .update_bounty_asset(&asset)
        .await
        .map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Check if a technology version is potentially vulnerable
fn is_potentially_vulnerable(name: &str, version: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Known vulnerable version patterns (simplified)
    let vulnerable_patterns: &[(&str, &str)] = &[
        ("apache", "2.4.49"), // Path traversal
        ("apache", "2.4.50"),
        ("log4j", "2.14"),
        ("log4j", "2.15"),
        ("spring", "5.3.17"), // Spring4Shell
        ("wordpress", "5.8"),
        ("jquery", "1."),
        ("jquery", "2."),
        ("angular", "1."),
    ];

    for (tech, ver_pattern) in vulnerable_patterns {
        if name_lower.contains(tech) && version.starts_with(ver_pattern) {
            return true;
        }
    }

    false
}

/// Add labels to an asset
#[tauri::command]
pub async fn bounty_add_asset_labels(
    db_service: State<'_, Arc<DatabaseService>>,
    request: AddAssetLabelsRequest,
) -> Result<BountyAssetRow, String> {
    ensure_bounty_feature()?;

    let mut asset = db_service
        .get_bounty_asset(&request.asset_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;

    let mut labels: Vec<String> = asset
        .labels_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    for label in request.labels {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }

    asset.labels_json = Some(serde_json::to_string(&labels).unwrap_or_default());
    asset.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_bounty_asset(&asset)
        .await
        .map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Get available high-value labels
#[tauri::command]
pub async fn bounty_get_high_value_labels() -> Result<Vec<String>, String> {
    Ok(HIGH_VALUE_LABELS.iter().map(|s| s.to_string()).collect())
}

/// Get assets by label
#[tauri::command]
pub async fn bounty_get_assets_by_label(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    label: String,
) -> Result<Vec<BountyAssetRow>, String> {
    // Get all assets for program
    let assets = db_service
        .list_bounty_assets(
            Some(&program_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    // Filter by label
    let filtered: Vec<BountyAssetRow> = assets
        .into_iter()
        .filter(|a| {
            if let Some(ref labels_json) = a.labels_json {
                if let Ok(labels) = serde_json::from_str::<Vec<String>>(labels_json) {
                    return labels.contains(&label);
                }
            }
            false
        })
        .collect();

    Ok(filtered)
}

/// Get assets by tech stack
#[tauri::command]
pub async fn bounty_get_assets_by_tech(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    tech_name: String,
) -> Result<Vec<BountyAssetRow>, String> {
    let assets = db_service
        .list_bounty_assets(
            Some(&program_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    let tech_lower = tech_name.to_lowercase();
    let filtered: Vec<BountyAssetRow> = assets
        .into_iter()
        .filter(|a| {
            if let Some(ref tech_json) = a.tech_stack_json {
                if let Ok(techs) = serde_json::from_str::<Vec<TechStackItem>>(tech_json) {
                    return techs
                        .iter()
                        .any(|t| t.name.to_lowercase().contains(&tech_lower));
                }
            }
            false
        })
        .collect();

    Ok(filtered)
}

// ============================================================================
// P1-B5: Priority Scoring System
// ============================================================================

/// Label weights for priority calculation
fn get_label_weight(label: &str) -> f64 {
    match label {
        "admin-panel" => 3.0,
        "payment-gateway" => 3.0,
        "user-data" => 2.5,
        "auth-system" => 2.5,
        "file-upload" => 2.0,
        "api-endpoint" => 1.5,
        "debug-enabled" => 2.0,
        "exposed-config" => 2.5,
        "vulnerable-tech" => 3.0,
        "outdated-software" => 2.0,
        _ => 0.5,
    }
}

/// Calculate priority score for an asset
fn calculate_priority_score(
    labels: &[String],
    tech_stack: &[TechStackItem],
    findings_count: i32,
    change_events_count: i32,
    is_alive: bool,
) -> f64 {
    if !is_alive {
        return 0.0;
    }

    let mut score = 0.0;

    // Label-based score (max ~9.0)
    for label in labels {
        score += get_label_weight(label);
    }

    // Tech stack complexity bonus (max ~2.0)
    let tech_bonus = (tech_stack.len() as f64 * 0.2).min(2.0);
    score += tech_bonus;

    // Findings history bonus (max ~3.0)
    let findings_bonus = (findings_count as f64 * 0.5).min(3.0);
    score += findings_bonus;

    // Change frequency bonus (max ~2.0)
    let change_bonus = (change_events_count as f64 * 0.3).min(2.0);
    score += change_bonus;

    // Vulnerable tech stack multiplier
    let has_vulnerable = labels.iter().any(|l| l == "vulnerable-tech");
    if has_vulnerable {
        score *= 1.2;
    }

    // Normalize to 0-10 scale
    (score.min(10.0) * 10.0).round() / 10.0
}

/// Recalculate priority score for an asset
#[tauri::command]
pub async fn bounty_recalculate_asset_priority(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
) -> Result<BountyAssetRow, String> {
    ensure_bounty_feature()?;

    let mut asset = db_service
        .get_bounty_asset(&asset_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;

    let labels: Vec<String> = asset
        .labels_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let tech_stack: Vec<TechStackItem> = asset
        .tech_stack_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let priority = calculate_priority_score(
        &labels,
        &tech_stack,
        asset.findings_count,
        asset.change_events_count,
        asset.is_alive,
    );

    asset.priority_score = Some(priority);
    asset.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_bounty_asset(&asset)
        .await
        .map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Recalculate priority for all assets in a program
#[tauri::command]
pub async fn bounty_recalculate_all_asset_priorities(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<i32, String> {
    ensure_bounty_feature()?;

    let assets = db_service
        .list_bounty_assets(
            Some(&program_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut count = 0;
    let now = Utc::now().to_rfc3339();

    for mut asset in assets {
        let labels: Vec<String> = asset
            .labels_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let tech_stack: Vec<TechStackItem> = asset
            .tech_stack_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let priority = calculate_priority_score(
            &labels,
            &tech_stack,
            asset.findings_count,
            asset.change_events_count,
            asset.is_alive,
        );

        if asset.priority_score != Some(priority) {
            asset.priority_score = Some(priority);
            asset.updated_at = now.clone();
            db_service
                .update_bounty_asset(&asset)
                .await
                .map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    Ok(count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueueItem {
    pub asset: BountyAssetRow,
    pub reason: String,
}

/// Get high-value priority queue
#[tauri::command]
pub async fn bounty_get_priority_queue(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    limit: Option<i64>,
) -> Result<Vec<PriorityQueueItem>, String> {
    let assets = db_service
        .get_top_priority_assets(&program_id, limit.unwrap_or(20))
        .await
        .map_err(|e| e.to_string())?;

    let mut queue: Vec<PriorityQueueItem> = assets
        .into_iter()
        .map(|asset| {
            let labels: Vec<String> = asset
                .labels_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            let reason =
                generate_priority_reason(&labels, asset.findings_count, asset.change_events_count);

            PriorityQueueItem { asset, reason }
        })
        .collect();

    // Sort by priority score descending
    queue.sort_by(|a, b| {
        b.asset
            .priority_score
            .unwrap_or(0.0)
            .partial_cmp(&a.asset.priority_score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(queue)
}

fn generate_priority_reason(
    labels: &[String],
    findings_count: i32,
    change_events_count: i32,
) -> String {
    let mut reasons = Vec::new();

    let high_value_labels: Vec<&str> = labels
        .iter()
        .filter(|l| get_label_weight(l) >= 2.0)
        .map(|s| s.as_str())
        .collect();

    if !high_value_labels.is_empty() {
        reasons.push(format!(
            "High-value labels: {}",
            high_value_labels.join(", ")
        ));
    }

    if findings_count > 0 {
        reasons.push(format!("{} previous findings", findings_count));
    }

    if change_events_count > 0 {
        reasons.push(format!("{} change events", change_events_count));
    }

    if reasons.is_empty() {
        "Standard priority".to_string()
    } else {
        reasons.join("; ")
    }
}
