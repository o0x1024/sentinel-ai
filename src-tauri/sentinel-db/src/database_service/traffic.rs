//! Traffic analysis database operations
//!
//! Handles data persistence for traffic analysis:
//! - Vulnerability storage and queries
//! - Evidence storage
//! - Plugin registry
//! - Proxy request history
//! - Proxy configuration

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::service::DatabaseService;

impl DatabaseService {
    /// Migrate old table names
   
    // ============================================================
    // Vulnerability Operations
    // ============================================================

    /// Insert new vulnerability
    pub async fn insert_traffic_vulnerability(&self, finding: &TrafficFinding) -> Result<()> {
        let pool = self.get_pool()?;
        let signature = finding.calculate_signature();

        debug!("Inserting vulnerability: title='{}', description='{}'", 
              finding.title, finding.description);

        sqlx::query(
            r#"
            INSERT INTO traffic_vulnerabilities (
                id, plugin_id, vuln_type, severity, confidence, title, description,
                cwe, owasp, remediation, signature, first_seen_at, last_seen_at, session_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            "#,
        )
        .bind(&finding.id)
        .bind(&finding.plugin_id)
        .bind(&finding.vuln_type)
        .bind(finding.severity.to_string())
        .bind(format!("{:?}", finding.confidence))
        .bind(&finding.title)
        .bind(&finding.description)
        .bind(&finding.cwe)
        .bind(&finding.owasp)
        .bind(&finding.remediation)
        .bind(&signature)
        .bind(finding.created_at)
        .bind(finding.created_at)
        .execute(pool)
        .await?;

        // Insert deduplication index
        sqlx::query(
            r#"
            INSERT INTO traffic_dedupe_index (signature, vuln_id) VALUES (?, ?)
            "#,
        )
        .bind(&signature)
        .bind(&finding.id)
        .execute(pool)
        .await?;

        // Insert evidence record
        let evidence = TrafficEvidenceRecord {
            id: format!("{}-evidence-{}", finding.id, chrono::Utc::now().timestamp_millis()),
            vuln_id: finding.id.clone(),
            url: finding.url.clone(),
            method: finding.method.clone(),
            location: finding.location.clone(),
            evidence_snippet: finding.evidence.clone(),
            request_headers: finding.request_headers.clone(),
            request_body: finding.request_body.clone(),
            response_status: finding.response_status,
            response_headers: finding.response_headers.clone(),
            response_body: finding.response_body.clone(),
            timestamp: finding.created_at,
        };
        
        self.insert_traffic_evidence(&evidence).await?;

        debug!("Vulnerability inserted with evidence: {} - {}", finding.id, finding.title);
        Ok(())
    }

    /// Update vulnerability hit count
    pub async fn update_traffic_vulnerability_hit(&self, signature: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"
            UPDATE traffic_vulnerabilities 
            SET hit_count = hit_count + 1, 
                last_seen_at = ?,
                updated_at = ?
            WHERE signature = ?
            "#,
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(signature)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Check if signature exists
    pub async fn check_traffic_signature_exists(&self, signature: &str) -> Result<bool> {
        let pool = self.get_pool()?;
        
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM traffic_dedupe_index WHERE signature = ?
            "#,
        )
        .bind(signature)
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    /// List vulnerabilities with pagination and filters
    pub async fn list_traffic_vulnerabilities(
        &self,
        filters: TrafficVulnerabilityFilters,
    ) -> Result<Vec<TrafficVulnerabilityRecord>> {
        let pool = self.get_pool()?;
        
        let mut query = String::from(
            r#"
            SELECT id, plugin_id, vuln_type, severity, confidence, title, description,
                   cwe, owasp, remediation, status, signature, first_seen_at, last_seen_at,
                   hit_count, session_id, created_at, updated_at
            FROM traffic_vulnerabilities
            WHERE 1=1
            "#,
        );

        // Dynamic filter conditions
        if let Some(ref vuln_type) = filters.vuln_type {
            query.push_str(&format!(" AND vuln_type = '{}'", vuln_type));
        }
        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(ref status) = filters.status {
            query.push_str(&format!(" AND status = '{}'", status));
        }
        if let Some(ref plugin_id) = filters.plugin_id {
            query.push_str(&format!(" AND plugin_id = '{}'", plugin_id));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let records = sqlx::query_as::<_, TrafficVulnerabilityRecord>(&query)
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// List vulnerabilities with evidence
    pub async fn list_traffic_vulnerabilities_with_evidence(
        &self,
        filters: TrafficVulnerabilityFilters,
    ) -> Result<Vec<TrafficVulnerabilityWithEvidence>> {
        let vulnerabilities = self.list_traffic_vulnerabilities(filters).await?;
        
        let mut results = Vec::new();
        for vuln in vulnerabilities {
            let evidence = self.get_traffic_evidence_by_vuln_id(&vuln.id).await?;
            
            let (url, method) = evidence.first()
                .map(|e| (Some(e.url.clone()), Some(e.method.clone())))
                .unwrap_or((None, None));
            
            results.push(TrafficVulnerabilityWithEvidence {
                vulnerability: vuln,
                evidence,
                url,
                method,
            });
        }
        
        Ok(results)
    }

    /// Count vulnerabilities
    pub async fn count_traffic_vulnerabilities(&self, filters: TrafficVulnerabilityFilters) -> Result<i64> {
        let pool = self.get_pool()?;
        
        let mut query = String::from(
            r#"
            SELECT COUNT(*) as count
            FROM traffic_vulnerabilities
            WHERE 1=1
            "#,
        );

        if let Some(ref vuln_type) = filters.vuln_type {
            query.push_str(&format!(" AND vuln_type = '{}'", vuln_type));
        }
        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(ref status) = filters.status {
            query.push_str(&format!(" AND status = '{}'", status));
        }
        if let Some(ref plugin_id) = filters.plugin_id {
            query.push_str(&format!(" AND plugin_id = '{}'", plugin_id));
        }

        let row: (i64,) = sqlx::query_as(&query)
            .fetch_one(pool)
            .await?;

        Ok(row.0)
    }

    /// Get vulnerability by ID
    pub async fn get_traffic_vulnerability_by_id(&self, vuln_id: &str) -> Result<Option<TrafficVulnerabilityRecord>> {
        let pool = self.get_pool()?;
        
        let record = sqlx::query_as::<_, TrafficVulnerabilityRecord>(
            r#"
            SELECT id, plugin_id, vuln_type, severity, confidence, title, description,
                   cwe, owasp, remediation, status, signature, first_seen_at, last_seen_at,
                   hit_count, session_id, created_at, updated_at
            FROM traffic_vulnerabilities
            WHERE id = ?
            "#,
        )
        .bind(vuln_id)
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    /// Get evidence by vulnerability ID
    pub async fn get_traffic_evidence_by_vuln_id(&self, vuln_id: &str) -> Result<Vec<TrafficEvidenceRecord>> {
        let pool = self.get_pool()?;
        
        let records = sqlx::query_as::<_, TrafficEvidenceRecord>(
            r#"
            SELECT id, vuln_id, url, method, location, evidence_snippet,
                   request_headers, request_body, response_status, response_headers,
                   response_body, timestamp
            FROM traffic_evidence
            WHERE vuln_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(vuln_id)
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// Update vulnerability status
    pub async fn update_traffic_vulnerability_status(&self, vuln_id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        let result = sqlx::query(
            r#"
            UPDATE traffic_vulnerabilities 
            SET status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(Utc::now())
        .bind(vuln_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Vulnerability not found: {}", vuln_id));
        }

        info!("Vulnerability status updated: {} -> {}", vuln_id, status);
        Ok(())
    }

    /// Delete vulnerability and related evidence
    /// Returns the signature of the deleted vulnerability for cache cleanup
    pub async fn delete_traffic_vulnerability(&self, vuln_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        
        // First get the signature before deleting
        let signature: Option<String> = sqlx::query_scalar(
            r#"
            SELECT signature FROM traffic_vulnerabilities WHERE id = ?
            "#,
        )
        .bind(vuln_id)
        .fetch_optional(pool)
        .await?;

        if signature.is_none() {
            return Err(anyhow::anyhow!("Vulnerability not found: {}", vuln_id));
        }

        // Delete the vulnerability
        let result = sqlx::query(
            r#"
            DELETE FROM traffic_vulnerabilities WHERE id = ?
            "#,
        )
        .bind(vuln_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Vulnerability not found: {}", vuln_id));
        }

        info!("Vulnerability deleted: {}", vuln_id);
        Ok(signature)
    }

    /// Delete all vulnerabilities
    /// Also clears the deduplication index
    pub async fn delete_all_traffic_vulnerabilities(&self) -> Result<()> {
        let pool = self.get_pool()?;
        
        // Delete all vulnerabilities
        sqlx::query("DELETE FROM traffic_vulnerabilities")
            .execute(pool)
            .await?;

        // Clear deduplication index
        sqlx::query("DELETE FROM traffic_dedupe_index")
            .execute(pool)
            .await?;

        info!("All vulnerabilities and dedupe index deleted");
        Ok(())
    }

    // ============================================================
    // Evidence Operations
    // ============================================================

    /// Insert evidence
    pub async fn insert_traffic_evidence(&self, evidence: &TrafficEvidenceRecord) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"
            INSERT INTO traffic_evidence (
                id, vuln_id, url, method, location, evidence_snippet,
                request_headers, request_body, response_status, response_headers,
                response_body, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&evidence.id)
        .bind(&evidence.vuln_id)
        .bind(&evidence.url)
        .bind(&evidence.method)
        .bind(&evidence.location)
        .bind(&evidence.evidence_snippet)
        .bind(&evidence.request_headers)
        .bind(&evidence.request_body)
        .bind(evidence.response_status)
        .bind(&evidence.response_headers)
        .bind(&evidence.response_body)
        .bind(evidence.timestamp)
        .execute(pool)
        .await?;

        Ok(())
    }

    // ============================================================
    // Plugin Registry Operations
    // ============================================================

    /// Register plugin with code
    pub async fn register_traffic_plugin_with_code(
        &self, 
        plugin: &TrafficPluginMetadata, 
        plugin_code: &str
    ) -> Result<()> {
        self.register_traffic_plugin_with_code_and_quality(plugin, plugin_code, None, Some("Approved")).await
    }

    /// Register plugin with code and quality score
    pub async fn register_traffic_plugin_with_code_and_quality(
        &self, 
        plugin: &TrafficPluginMetadata, 
        plugin_code: &str,
        quality_score: Option<f64>,
        validation_status: Option<&str>
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let tags_json = serde_json::to_string(&plugin.tags).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO plugin_registry (
                id, name, version, author, main_category, category, description, default_severity,
                tags, enabled, plugin_code, quality_score, validation_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
            "#,
        )
        .bind(&plugin.id)
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(&plugin.author)
        .bind(&plugin.main_category)
        .bind(&plugin.category)
        .bind(&plugin.description)
        .bind(plugin.default_severity.to_string())
        .bind(&tags_json)
        .bind(plugin_code)
        .bind(quality_score)
        .bind(validation_status)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update plugin
    pub async fn update_traffic_plugin(&self, plugin: &TrafficPluginMetadata, plugin_code: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let tags_json = serde_json::to_string(&plugin.tags).unwrap_or_default();
        
        sqlx::query(
            r#"
            UPDATE plugin_registry 
            SET name = ?, version = ?, author = ?, main_category = ?, category = ?,
                description = ?, default_severity = ?, tags = ?, plugin_code = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(&plugin.author)
        .bind(&plugin.main_category)
        .bind(&plugin.category)
        .bind(&plugin.description)
        .bind(plugin.default_severity.to_string())
        .bind(&tags_json)
        .bind(plugin_code)
        .bind(Utc::now())
        .bind(&plugin.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get plugin code
    pub async fn get_traffic_plugin_code(&self, plugin_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        
        let result = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT plugin_code
            FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.flatten())
    }

    /// Get plugin by ID
    pub async fn get_traffic_plugin_by_id(&self, plugin_id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        
        let row: Option<(
            String, String, String, Option<String>, String, String, Option<String>, 
            String, Option<String>, bool, Option<String>
        )> = sqlx::query_as(
            r#"
            SELECT id, name, version, author, main_category, category, description, 
                   default_severity, tags, enabled, plugin_code
            FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(pool)
        .await?;

        if let Some((id, name, version, author, main_category, category, description, 
                     default_severity, tags, enabled, plugin_code)) = row {
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            
            Ok(Some(serde_json::json!({
                "id": id,
                "name": name,
                "version": version,
                "author": author,
                "main_category": main_category,
                "category": category,
                "description": description,
                "default_severity": default_severity,
                "tags": tags_array,
                "enabled": enabled,
                "plugin_code": plugin_code
            })))
        } else {
            Ok(None)
        }
    }

    /// Find reusable plugins by category
    pub async fn find_reusable_traffic_plugins_by_category(
        &self,
        category: &str,
        min_quality_score: f64,
    ) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        
        let rows: Vec<(
            String, String, String, Option<String>, String, String, Option<String>,
            String, Option<String>, bool, Option<String>, Option<f64>, Option<String>
        )> = sqlx::query_as(
            r#"
            SELECT id, name, version, author, main_category, category, description,
                   default_severity, tags, enabled, plugin_code, quality_score, validation_status
            FROM plugin_registry
            WHERE category = ? 
              AND (quality_score IS NULL OR quality_score >= ?)
              AND validation_status IN ('Approved', 'Passed')
              AND main_category = 'traffic'
            ORDER BY quality_score DESC NULLS LAST, updated_at DESC
            LIMIT 5
            "#,
        )
        .bind(category)
        .bind(min_quality_score)
        .fetch_all(pool)
        .await?;

        let mut plugins = Vec::new();
        for (id, name, version, author, main_category, category, description,
             default_severity, tags, enabled, plugin_code, quality_score, validation_status) in rows
        {
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            plugins.push(serde_json::json!({
                "id": id,
                "name": name,
                "version": version,
                "author": author,
                "main_category": main_category,
                "category": category,
                "description": description,
                "default_severity": default_severity,
                "tags": tags_array,
                "enabled": enabled,
                "plugin_code": plugin_code,
                "quality_score": quality_score,
                "validation_status": validation_status
            }));
        }

        Ok(plugins)
    }

    /// Update plugin enabled status
    pub async fn update_traffic_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"
            UPDATE plugin_registry 
            SET enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(plugin_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete plugin
    pub async fn delete_traffic_plugin(&self, plugin_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"
            DELETE FROM plugin_registry 
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    // ============================================================
    // Proxy Request History Operations
    // ============================================================

    /// Insert proxy request record
    pub async fn insert_proxy_request(&self, request: &ProxyRequestRecord) -> Result<i64> {
        let pool = self.get_pool()?;
        
        let result = sqlx::query(
            r#"
            INSERT INTO proxy_requests (
                url, host, protocol, method, status_code,
                request_headers, request_body, response_headers, response_body,
                response_size, response_time, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.url)
        .bind(&request.host)
        .bind(&request.protocol)
        .bind(&request.method)
        .bind(request.status_code)
        .bind(&request.request_headers)
        .bind(&request.request_body)
        .bind(&request.response_headers)
        .bind(&request.response_body)
        .bind(request.response_size)
        .bind(request.response_time)
        .bind(request.timestamp)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// List proxy requests with filters
    pub async fn list_proxy_requests(
        &self,
        filters: ProxyRequestFilters,
    ) -> Result<Vec<ProxyRequestRecord>> {
        let pool = self.get_pool()?;
        
        let mut query = String::from(
            r#"
            SELECT id, url, host, protocol, method, status_code,
                   request_headers, request_body, response_headers, response_body,
                   response_size, response_time, timestamp
            FROM proxy_requests
            WHERE 1=1
            "#,
        );

        if let Some(ref protocol) = filters.protocol {
            query.push_str(&format!(" AND protocol = '{}'", protocol));
        }
        if let Some(ref method) = filters.method {
            query.push_str(&format!(" AND method = '{}'", method));
        }
        if let Some(ref host) = filters.host {
            query.push_str(&format!(" AND host LIKE '%{}%'", host));
        }
        if let Some(status_min) = filters.status_code_min {
            query.push_str(&format!(" AND status_code >= {}", status_min));
        }
        if let Some(status_max) = filters.status_code_max {
            query.push_str(&format!(" AND status_code <= {}", status_max));
        }

        query.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let records = sqlx::query_as::<_, ProxyRequestRecord>(&query)
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// List proxy requests by host
    pub async fn list_proxy_requests_by_host(
        &self,
        host: &str,
        limit: i64,
    ) -> Result<Vec<ProxyRequestRecord>> {
        self.list_proxy_requests(ProxyRequestFilters {
            host: Some(host.to_string()),
            limit: Some(limit),
            ..Default::default()
        }).await
    }

    /// Count proxy requests
    pub async fn count_proxy_requests(&self, filters: ProxyRequestFilters) -> Result<i64> {
        let pool = self.get_pool()?;
        
        let mut query = String::from(
            r#"
            SELECT COUNT(*) as count
            FROM proxy_requests
            WHERE 1=1
            "#,
        );

        if let Some(ref protocol) = filters.protocol {
            query.push_str(&format!(" AND protocol = '{}'", protocol));
        }
        if let Some(ref method) = filters.method {
            query.push_str(&format!(" AND method = '{}'", method));
        }
        if let Some(ref host) = filters.host {
            query.push_str(&format!(" AND host LIKE '%{}%'", host));
        }
        if let Some(status_min) = filters.status_code_min {
            query.push_str(&format!(" AND status_code >= {}", status_min));
        }
        if let Some(status_max) = filters.status_code_max {
            query.push_str(&format!(" AND status_code <= {}", status_max));
        }

        let row: (i64,) = sqlx::query_as(&query)
            .fetch_one(pool)
            .await?;

        Ok(row.0)
    }

    /// Get proxy request by ID
    pub async fn get_proxy_request_by_id(&self, id: i64) -> Result<Option<ProxyRequestRecord>> {
        let pool = self.get_pool()?;
        
        let record = sqlx::query_as::<_, ProxyRequestRecord>(
            r#"
            SELECT id, url, host, protocol, method, status_code,
                   request_headers, request_body, response_headers, response_body,
                   response_size, response_time, timestamp
            FROM proxy_requests
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    /// Clear all proxy requests
    pub async fn clear_proxy_requests(&self) -> Result<u64> {
        let pool = self.get_pool()?;
        
        let mut tx = pool.begin().await?;

        let result = sqlx::query("DELETE FROM proxy_requests")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM sqlite_sequence WHERE name='proxy_requests'")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Cleared {} proxy request records and reset ID", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// Delete proxy requests before specified time
    pub async fn delete_proxy_requests_before(&self, before: DateTime<Utc>) -> Result<u64> {
        let pool = self.get_pool()?;
        
        let result = sqlx::query("DELETE FROM proxy_requests WHERE timestamp < ?")
            .bind(before)
            .execute(pool)
            .await?;

        info!("Deleted {} old proxy request records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// Save proxy config
    pub async fn save_proxy_config(&self, key: &str, value: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"
            INSERT INTO proxy_config (key, value, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;

        info!("Saved config: {} = {}", key, value);
        Ok(())
    }

    /// Load proxy config
    pub async fn load_proxy_config(&self, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM proxy_config WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(value,)| value))
    }

    /// Delete proxy config
    pub async fn delete_proxy_config(&self, key: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query("DELETE FROM proxy_config WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;

        info!("Deleted config: {}", key);
        Ok(())
    }
}

// ============================================================
// Data Structures
// ============================================================

/// Vulnerability filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficVulnerabilityFilters {
    pub vuln_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub plugin_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Vulnerability record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrafficVulnerabilityRecord {
    pub id: String,
    pub plugin_id: String,
    pub vuln_type: String,
    pub severity: String,
    pub confidence: String,
    pub title: String,
    pub description: String,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub remediation: Option<String>,
    pub status: String,
    pub signature: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub hit_count: i64,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Evidence record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrafficEvidenceRecord {
    pub id: String,
    pub vuln_id: String,
    pub url: String,
    pub method: String,
    pub location: String,
    pub evidence_snippet: String,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Vulnerability with evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficVulnerabilityWithEvidence {
    #[serde(flatten)]
    pub vulnerability: TrafficVulnerabilityRecord,
    pub evidence: Vec<TrafficEvidenceRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// Proxy request record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyRequestRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub url: String,
    pub host: String,
    pub protocol: String,
    pub method: String,
    pub status_code: i32,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub response_size: i64,
    pub response_time: i64,
    pub timestamp: DateTime<Utc>,
}

/// Proxy request filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyRequestFilters {
    pub protocol: Option<String>,
    pub method: Option<String>,
    pub host: Option<String>,
    pub status_code_min: Option<i32>,
    pub status_code_max: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Traffic finding (temporary structure for compatibility)
#[derive(Debug, Clone)]
pub struct TrafficFinding {
    pub id: String,
    pub plugin_id: String,
    pub vuln_type: String,
    pub severity: String,
    pub confidence: String,
    pub title: String,
    pub description: String,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub remediation: Option<String>,
    pub url: String,
    pub method: String,
    pub location: String,
    pub evidence: String,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TrafficFinding {
    pub fn calculate_signature(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.plugin_id);
        hasher.update(&self.vuln_type);
        hasher.update(&self.url);
        hasher.update(&self.location);
        format!("{:x}", hasher.finalize())
    }
}

/// Traffic plugin metadata (temporary structure for compatibility)
#[derive(Debug, Clone)]
pub struct TrafficPluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub main_category: String,
    pub category: String,
    pub description: Option<String>,
    pub default_severity: String,
    pub tags: Vec<String>,
}

