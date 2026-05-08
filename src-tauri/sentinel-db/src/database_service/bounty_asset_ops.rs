//! Bug bounty asset database operations.

use anyhow::Result;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_asset, timestamp_string_to_datetime,
    BountyAssetRow, BountyAssetStats,
};
use super::bounty_domain_support::populate_bounty_asset_domain_fields;
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Bounty Asset CRUD
    // ------------------------------------------------------------------------

    /// Create a bounty asset
    pub async fn create_bounty_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        let mut asset = asset.clone();
        populate_bounty_asset_domain_fields(&mut asset);

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let insert_columns = r#"
                    id, program_id, scope_id, asset_type, canonical_url, original_urls_json,
                    hostname, port, path, protocol, ip_addresses_json, dns_records_json,
                    tech_stack_json, fingerprint, tags_json, labels_json, priority_score,
                    risk_score, is_alive, last_checked_at, first_seen_at, last_seen_at,
                    findings_count, change_events_count, metadata_json, created_at, updated_at,
                    ip_version, asn, asn_org, isp, country, city, latitude, longitude,
                    is_cloud, cloud_provider, service_name, service_version, service_product, banner,
                    transport_protocol, cpe, domain_registrar, registration_date, expiration_date,
                    nameservers_json, mx_records_json, txt_records_json, whois_data_json,
                    is_wildcard, parent_domain, root_domain, subdomain_level,
                    http_status, response_time_ms, content_length,
                    content_type, title, favicon_hash, headers_json, waf_detected, cdn_detected,
                    screenshot_path, body_hash, certificate_id, ssl_enabled, certificate_subject,
                    certificate_issuer, certificate_valid_from, certificate_valid_to, certificate_san_json,
                    exposure_level, attack_surface_score, vulnerability_count, cvss_max_score,
                    exploit_available, asset_category, asset_owner, business_unit, criticality,
                    discovery_method, data_sources_json, confidence_score, monitoring_enabled,
                    scan_frequency, last_scan_type, parent_asset_id, related_assets_json
        "#;
        let insert_column_count = insert_columns
            .split(',')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .count();

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let sqlite_placeholders = std::iter::repeat("?")
                .take(insert_column_count)
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "INSERT INTO bounty_assets ({}) VALUES ({})",
                insert_columns, sqlite_placeholders
            );
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(&query)
                        .bind(&asset.id)
                        .bind(&asset.program_id)
                        .bind(&asset.scope_id)
                        .bind(&asset.asset_type)
                        .bind(&asset.canonical_url)
                        .bind(&asset.original_urls_json)
                        .bind(&asset.hostname)
                        .bind(asset.port)
                        .bind(&asset.path)
                        .bind(&asset.protocol)
                        .bind(&asset.ip_addresses_json)
                        .bind(&asset.dns_records_json)
                        .bind(&asset.tech_stack_json)
                        .bind(&asset.fingerprint)
                        .bind(&asset.tags_json)
                        .bind(&asset.labels_json)
                        .bind(asset.priority_score)
                        .bind(asset.risk_score)
                        .bind(asset.is_alive)
                        .bind(&asset.last_checked_at)
                        .bind(&asset.first_seen_at)
                        .bind(&asset.last_seen_at)
                        .bind(asset.findings_count)
                        .bind(asset.change_events_count)
                        .bind(&asset.metadata_json)
                        .bind(&asset.created_at)
                        .bind(&asset.updated_at)
                        .bind(&asset.ip_version)
                        .bind(asset.asn)
                        .bind(&asset.asn_org)
                        .bind(&asset.isp)
                        .bind(&asset.country)
                        .bind(&asset.city)
                        .bind(asset.latitude)
                        .bind(asset.longitude)
                        .bind(asset.is_cloud)
                        .bind(&asset.cloud_provider)
                        .bind(&asset.service_name)
                        .bind(&asset.service_version)
                        .bind(&asset.service_product)
                        .bind(&asset.banner)
                        .bind(&asset.transport_protocol)
                        .bind(&asset.cpe)
                        .bind(&asset.domain_registrar)
                        .bind(&asset.registration_date)
                        .bind(&asset.expiration_date)
                        .bind(&asset.nameservers_json)
                        .bind(&asset.mx_records_json)
                        .bind(&asset.txt_records_json)
                        .bind(&asset.whois_data_json)
                        .bind(asset.is_wildcard)
                        .bind(&asset.parent_domain)
                        .bind(&asset.root_domain)
                        .bind(asset.subdomain_level)
                        .bind(asset.http_status)
                        .bind(asset.response_time_ms)
                        .bind(asset.content_length)
                        .bind(&asset.content_type)
                        .bind(&asset.title)
                        .bind(&asset.favicon_hash)
                        .bind(&asset.headers_json)
                        .bind(&asset.waf_detected)
                        .bind(&asset.cdn_detected)
                        .bind(&asset.screenshot_path)
                        .bind(&asset.body_hash)
                        .bind(&asset.certificate_id)
                        .bind(asset.ssl_enabled)
                        .bind(&asset.certificate_subject)
                        .bind(&asset.certificate_issuer)
                        .bind(&asset.certificate_valid_from)
                        .bind(&asset.certificate_valid_to)
                        .bind(&asset.certificate_san_json)
                        .bind(&asset.exposure_level)
                        .bind(asset.attack_surface_score)
                        .bind(asset.vulnerability_count)
                        .bind(asset.cvss_max_score)
                        .bind(asset.exploit_available)
                        .bind(&asset.asset_category)
                        .bind(&asset.asset_owner)
                        .bind(&asset.business_unit)
                        .bind(&asset.criticality)
                        .bind(&asset.discovery_method)
                        .bind(&asset.data_sources_json)
                        .bind(asset.confidence_score)
                        .bind(asset.monitoring_enabled)
                        .bind(&asset.scan_frequency)
                        .bind(&asset.last_scan_type)
                        .bind(&asset.parent_asset_id)
                        .bind(&asset.related_assets_json)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(&query)
                        .bind(&asset.id)
                        .bind(&asset.program_id)
                        .bind(&asset.scope_id)
                        .bind(&asset.asset_type)
                        .bind(&asset.canonical_url)
                        .bind(&asset.original_urls_json)
                        .bind(&asset.hostname)
                        .bind(asset.port)
                        .bind(&asset.path)
                        .bind(&asset.protocol)
                        .bind(&asset.ip_addresses_json)
                        .bind(&asset.dns_records_json)
                        .bind(&asset.tech_stack_json)
                        .bind(&asset.fingerprint)
                        .bind(&asset.tags_json)
                        .bind(&asset.labels_json)
                        .bind(asset.priority_score)
                        .bind(asset.risk_score)
                        .bind(asset.is_alive)
                        .bind(&asset.last_checked_at)
                        .bind(&asset.first_seen_at)
                        .bind(&asset.last_seen_at)
                        .bind(asset.findings_count)
                        .bind(asset.change_events_count)
                        .bind(&asset.metadata_json)
                        .bind(&asset.created_at)
                        .bind(&asset.updated_at)
                        .bind(&asset.ip_version)
                        .bind(asset.asn)
                        .bind(&asset.asn_org)
                        .bind(&asset.isp)
                        .bind(&asset.country)
                        .bind(&asset.city)
                        .bind(asset.latitude)
                        .bind(asset.longitude)
                        .bind(asset.is_cloud)
                        .bind(&asset.cloud_provider)
                        .bind(&asset.service_name)
                        .bind(&asset.service_version)
                        .bind(&asset.service_product)
                        .bind(&asset.banner)
                        .bind(&asset.transport_protocol)
                        .bind(&asset.cpe)
                        .bind(&asset.domain_registrar)
                        .bind(&asset.registration_date)
                        .bind(&asset.expiration_date)
                        .bind(&asset.nameservers_json)
                        .bind(&asset.mx_records_json)
                        .bind(&asset.txt_records_json)
                        .bind(&asset.whois_data_json)
                        .bind(asset.is_wildcard)
                        .bind(&asset.parent_domain)
                        .bind(&asset.root_domain)
                        .bind(asset.subdomain_level)
                        .bind(asset.http_status)
                        .bind(asset.response_time_ms)
                        .bind(asset.content_length)
                        .bind(&asset.content_type)
                        .bind(&asset.title)
                        .bind(&asset.favicon_hash)
                        .bind(&asset.headers_json)
                        .bind(&asset.waf_detected)
                        .bind(&asset.cdn_detected)
                        .bind(&asset.screenshot_path)
                        .bind(&asset.body_hash)
                        .bind(&asset.certificate_id)
                        .bind(asset.ssl_enabled)
                        .bind(&asset.certificate_subject)
                        .bind(&asset.certificate_issuer)
                        .bind(&asset.certificate_valid_from)
                        .bind(&asset.certificate_valid_to)
                        .bind(&asset.certificate_san_json)
                        .bind(&asset.exposure_level)
                        .bind(asset.attack_surface_score)
                        .bind(asset.vulnerability_count)
                        .bind(asset.cvss_max_score)
                        .bind(asset.exploit_available)
                        .bind(&asset.asset_category)
                        .bind(&asset.asset_owner)
                        .bind(&asset.business_unit)
                        .bind(&asset.criticality)
                        .bind(&asset.discovery_method)
                        .bind(&asset.data_sources_json)
                        .bind(asset.confidence_score)
                        .bind(asset.monitoring_enabled)
                        .bind(&asset.scan_frequency)
                        .bind(&asset.last_scan_type)
                        .bind(&asset.parent_asset_id)
                        .bind(&asset.related_assets_json)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty asset: {}", asset.id);
            return Ok(());
        }

        let postgres_placeholders = (1..=insert_column_count)
            .map(|i| format!("${}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT INTO bounty_assets ({}) VALUES ({})",
            insert_columns, postgres_placeholders
        );

        sqlx::query(&query)
            .bind(&asset.id)
            .bind(&asset.program_id)
            .bind(&asset.scope_id)
            .bind(&asset.asset_type)
            .bind(&asset.canonical_url)
            .bind(&asset.original_urls_json)
            .bind(&asset.hostname)
            .bind(asset.port)
            .bind(&asset.path)
            .bind(&asset.protocol)
            .bind(&asset.ip_addresses_json)
            .bind(&asset.dns_records_json)
            .bind(&asset.tech_stack_json)
            .bind(&asset.fingerprint)
            .bind(&asset.tags_json)
            .bind(&asset.labels_json)
            .bind(asset.priority_score)
            .bind(asset.risk_score)
            .bind(asset.is_alive)
            .bind(optional_timestamp_string_to_datetime(
                &asset.last_checked_at,
            ))
            .bind(timestamp_string_to_datetime(&asset.first_seen_at))
            .bind(timestamp_string_to_datetime(&asset.last_seen_at))
            .bind(asset.findings_count)
            .bind(asset.change_events_count)
            .bind(&asset.metadata_json)
            .bind(timestamp_string_to_datetime(&asset.created_at))
            .bind(timestamp_string_to_datetime(&asset.updated_at))
            .bind(&asset.ip_version)
            .bind(asset.asn)
            .bind(&asset.asn_org)
            .bind(&asset.isp)
            .bind(&asset.country)
            .bind(&asset.city)
            .bind(asset.latitude)
            .bind(asset.longitude)
            .bind(asset.is_cloud)
            .bind(&asset.cloud_provider)
            .bind(&asset.service_name)
            .bind(&asset.service_version)
            .bind(&asset.service_product)
            .bind(&asset.banner)
            .bind(&asset.transport_protocol)
            .bind(&asset.cpe)
            .bind(&asset.domain_registrar)
            .bind(&asset.registration_date)
            .bind(&asset.expiration_date)
            .bind(&asset.nameservers_json)
            .bind(&asset.mx_records_json)
            .bind(&asset.txt_records_json)
            .bind(&asset.whois_data_json)
            .bind(asset.is_wildcard)
            .bind(&asset.parent_domain)
            .bind(&asset.root_domain)
            .bind(asset.subdomain_level)
            .bind(asset.http_status)
            .bind(asset.response_time_ms)
            .bind(asset.content_length)
            .bind(&asset.content_type)
            .bind(&asset.title)
            .bind(&asset.favicon_hash)
            .bind(&asset.headers_json)
            .bind(&asset.waf_detected)
            .bind(&asset.cdn_detected)
            .bind(&asset.screenshot_path)
            .bind(&asset.body_hash)
            .bind(&asset.certificate_id)
            .bind(asset.ssl_enabled)
            .bind(&asset.certificate_subject)
            .bind(&asset.certificate_issuer)
            .bind(&asset.certificate_valid_from)
            .bind(&asset.certificate_valid_to)
            .bind(&asset.certificate_san_json)
            .bind(&asset.exposure_level)
            .bind(asset.attack_surface_score)
            .bind(asset.vulnerability_count)
            .bind(asset.cvss_max_score)
            .bind(asset.exploit_available)
            .bind(&asset.asset_category)
            .bind(&asset.asset_owner)
            .bind(&asset.business_unit)
            .bind(&asset.criticality)
            .bind(&asset.discovery_method)
            .bind(&asset.data_sources_json)
            .bind(asset.confidence_score)
            .bind(asset.monitoring_enabled)
            .bind(&asset.scan_frequency)
            .bind(&asset.last_scan_type)
            .bind(&asset.parent_asset_id)
            .bind(&asset.related_assets_json)
            .execute(self.get_pool()?)
            .await?;

        info!("Created bounty asset: {}", asset.id);
        Ok(())
    }

    /// Get a bounty asset by ID
    pub async fn get_bounty_asset(&self, id: &str) -> Result<Option<BountyAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_assets WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::MySQL(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_assets WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_assets WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_asset))
    }

    /// Get a bounty asset by canonical URL
    pub async fn get_bounty_asset_by_canonical_url(
        &self,
        program_id: &str,
        canonical_url: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_assets WHERE program_id = ? AND canonical_url = ?",
                )
                .bind(program_id)
                .bind(canonical_url)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_assets WHERE program_id = ? AND canonical_url = ?",
                )
                .bind(program_id)
                .bind(canonical_url)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row =
            sqlx::query("SELECT * FROM bounty_assets WHERE program_id = $1 AND canonical_url = $2")
                .bind(program_id)
                .bind(canonical_url)
                .fetch_optional(self.get_pool()?)
                .await?;

        Ok(row.map(row_to_bounty_asset))
    }

    /// Get a bounty asset by fingerprint
    pub async fn get_bounty_asset_by_fingerprint(
        &self,
        program_id: &str,
        fingerprint: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_assets WHERE program_id = ? AND fingerprint = ?",
                )
                .bind(program_id)
                .bind(fingerprint)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_assets WHERE program_id = ? AND fingerprint = ?",
                )
                .bind(program_id)
                .bind(fingerprint)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row =
            sqlx::query("SELECT * FROM bounty_assets WHERE program_id = $1 AND fingerprint = $2")
                .bind(program_id)
                .bind(fingerprint)
                .fetch_optional(self.get_pool()?)
                .await?;

        Ok(row.map(row_to_bounty_asset))
    }

    /// List bounty assets
    pub async fn list_bounty_assets(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        asset_type: Option<&str>,
        is_alive: Option<bool>,
        has_findings: Option<bool>,
        search: Option<&str>,
        sort_by: Option<&str>,
        sort_dir: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BountyAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let mut assets: Vec<BountyAssetRow> = match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query_as("SELECT * FROM bounty_assets")
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query_as("SELECT * FROM bounty_assets")
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };

            if let Some(pid) = program_id {
                assets.retain(|a| a.program_id == pid);
            }
            if let Some(sid) = scope_id {
                assets.retain(|a| a.scope_id.as_deref() == Some(sid));
            }
            if let Some(at) = asset_type {
                assets.retain(|a| a.asset_type == at);
            }
            if let Some(alive) = is_alive {
                assets.retain(|a| a.is_alive == alive);
            }
            if let Some(findings) = has_findings {
                if findings {
                    assets.retain(|a| a.findings_count > 0);
                } else {
                    assets.retain(|a| a.findings_count == 0);
                }
            }
            if let Some(keyword) = search {
                if !keyword.is_empty() {
                    let needle = keyword.to_lowercase();
                    assets.retain(|a| {
                        a.hostname
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&needle)
                            || a.canonical_url.to_lowercase().contains(&needle)
                            || a.service_name
                                .as_deref()
                                .unwrap_or_default()
                                .to_lowercase()
                                .contains(&needle)
                            || a.root_domain
                                .as_deref()
                                .unwrap_or_default()
                                .to_lowercase()
                                .contains(&needle)
                            || a.title
                                .as_deref()
                                .unwrap_or_default()
                                .to_lowercase()
                                .contains(&needle)
                    });
                }
            }

            let order_by = sort_by.unwrap_or("priority_score");
            let direction = sort_dir.unwrap_or("desc").to_lowercase();
            assets.sort_by(|a, b| {
                let ord = match order_by {
                    "priority_score" => a
                        .priority_score
                        .partial_cmp(&b.priority_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    "risk_score" => a
                        .risk_score
                        .partial_cmp(&b.risk_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    "last_seen_at" => a.last_seen_at.cmp(&b.last_seen_at),
                    "created_at" => a.created_at.cmp(&b.created_at),
                    "hostname" => a.hostname.cmp(&b.hostname),
                    "canonical_url" => a.canonical_url.cmp(&b.canonical_url),
                    _ => a
                        .priority_score
                        .partial_cmp(&b.priority_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                };
                if direction == "asc" {
                    ord
                } else {
                    ord.reverse()
                }
            });

            if let Some(off) = offset {
                assets = assets.into_iter().skip(off.max(0) as usize).collect();
            }
            if let Some(lim) = limit {
                assets.truncate(lim.max(0) as usize);
            }
            return Ok(assets);
        }

        let mut query = String::from("SELECT * FROM bounty_assets WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }
        if let Some(sid) = scope_id {
            params.push(sid.to_string());
            query.push_str(&format!(" AND scope_id = ${}", params.len()));
        }
        if let Some(at) = asset_type {
            params.push(at.to_string());
            query.push_str(&format!(" AND asset_type = ${}", params.len()));
        }
        if let Some(alive) = is_alive {
            query.push_str(&format!(" AND is_alive = {}", alive));
        }
        if let Some(findings) = has_findings {
            if findings {
                query.push_str(" AND findings_count > 0");
            } else {
                query.push_str(" AND findings_count = 0");
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                let p1 = params.len() + 1;
                let p2 = params.len() + 2;
                let p3 = params.len() + 3;
                let p4 = params.len() + 4;
                let p5 = params.len() + 5;
                query.push_str(&format!(
                    " AND (hostname ILIKE ${} OR canonical_url ILIKE ${} OR service_name ILIKE ${} OR title ILIKE ${} OR root_domain ILIKE ${})",
                    p1, p2, p3, p4, p5
                ));
                let search_pattern = format!("%{}%", search);
                params.push(search_pattern.clone());
                params.push(search_pattern.clone());
                params.push(search_pattern.clone());
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        let order_by = match sort_by.unwrap_or("priority_score") {
            "priority_score" => "priority_score",
            "risk_score" => "risk_score",
            "last_seen_at" => "last_seen_at",
            "created_at" => "created_at",
            "hostname" => "hostname",
            "canonical_url" => "canonical_url",
            _ => "priority_score",
        };
        let direction = match sort_dir.unwrap_or("desc").to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };
        query.push_str(&format!(" ORDER BY {} {}", order_by, direction));

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }
        if let Some(off) = offset {
            query.push_str(&format!(" OFFSET {}", off));
        }

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;

        Ok(rows.into_iter().map(row_to_bounty_asset).collect())
    }

    /// Update a bounty asset
    pub async fn update_bounty_asset(&self, asset: &BountyAssetRow) -> Result<bool> {
        let mut asset = asset.clone();
        populate_bounty_asset_domain_fields(&mut asset);

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_assets SET
                    scope_id = ?, asset_type = ?, canonical_url = ?, original_urls_json = ?,
                    hostname = ?, port = ?, path = ?, protocol = ?, ip_addresses_json = ?,
                    dns_records_json = ?, tech_stack_json = ?, fingerprint = ?, tags_json = ?,
                    labels_json = ?, priority_score = ?, risk_score = ?, is_alive = ?,
                    last_checked_at = ?, last_seen_at = ?, findings_count = ?,
                    change_events_count = ?, metadata_json = ?, updated_at = ?,
                    ip_version = ?, asn = ?, asn_org = ?, isp = ?, country = ?, city = ?, latitude = ?, longitude = ?,
                    is_cloud = ?, cloud_provider = ?, service_name = ?, service_version = ?, service_product = ?, banner = ?,
                    transport_protocol = ?, cpe = ?, domain_registrar = ?, registration_date = ?, expiration_date = ?,
                    nameservers_json = ?, mx_records_json = ?, txt_records_json = ?, whois_data_json = ?,
                    is_wildcard = ?, parent_domain = ?, root_domain = ?, subdomain_level = ?,
                    http_status = ?, response_time_ms = ?, content_length = ?,
                    content_type = ?, title = ?, favicon_hash = ?, headers_json = ?, waf_detected = ?, cdn_detected = ?,
                    screenshot_path = ?, body_hash = ?, certificate_id = ?, ssl_enabled = ?, certificate_subject = ?,
                    certificate_issuer = ?, certificate_valid_from = ?, certificate_valid_to = ?, certificate_san_json = ?,
                    exposure_level = ?, attack_surface_score = ?, vulnerability_count = ?, cvss_max_score = ?,
                    exploit_available = ?, asset_category = ?, asset_owner = ?, business_unit = ?, criticality = ?,
                    discovery_method = ?, data_sources_json = ?, confidence_score = ?, monitoring_enabled = ?,
                    scan_frequency = ?, last_scan_type = ?, parent_asset_id = ?, related_assets_json = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
                        .bind(&asset.scope_id)
                        .bind(&asset.asset_type)
                        .bind(&asset.canonical_url)
                        .bind(&asset.original_urls_json)
                        .bind(&asset.hostname)
                        .bind(asset.port)
                        .bind(&asset.path)
                        .bind(&asset.protocol)
                        .bind(&asset.ip_addresses_json)
                        .bind(&asset.dns_records_json)
                        .bind(&asset.tech_stack_json)
                        .bind(&asset.fingerprint)
                        .bind(&asset.tags_json)
                        .bind(&asset.labels_json)
                        .bind(asset.priority_score)
                        .bind(asset.risk_score)
                        .bind(asset.is_alive)
                        .bind(&asset.last_checked_at)
                        .bind(&asset.last_seen_at)
                        .bind(asset.findings_count)
                        .bind(asset.change_events_count)
                        .bind(&asset.metadata_json)
                        .bind(&asset.updated_at)
                        .bind(&asset.ip_version)
                        .bind(asset.asn)
                        .bind(&asset.asn_org)
                        .bind(&asset.isp)
                        .bind(&asset.country)
                        .bind(&asset.city)
                        .bind(asset.latitude)
                        .bind(asset.longitude)
                        .bind(asset.is_cloud)
                        .bind(&asset.cloud_provider)
                        .bind(&asset.service_name)
                        .bind(&asset.service_version)
                        .bind(&asset.service_product)
                        .bind(&asset.banner)
                        .bind(&asset.transport_protocol)
                        .bind(&asset.cpe)
                        .bind(&asset.domain_registrar)
                        .bind(&asset.registration_date)
                        .bind(&asset.expiration_date)
                        .bind(&asset.nameservers_json)
                        .bind(&asset.mx_records_json)
                        .bind(&asset.txt_records_json)
                        .bind(&asset.whois_data_json)
                        .bind(asset.is_wildcard)
                        .bind(&asset.parent_domain)
                        .bind(&asset.root_domain)
                        .bind(asset.subdomain_level)
                        .bind(asset.http_status)
                        .bind(asset.response_time_ms)
                        .bind(asset.content_length)
                        .bind(&asset.content_type)
                        .bind(&asset.title)
                        .bind(&asset.favicon_hash)
                        .bind(&asset.headers_json)
                        .bind(&asset.waf_detected)
                        .bind(&asset.cdn_detected)
                        .bind(&asset.screenshot_path)
                        .bind(&asset.body_hash)
                        .bind(&asset.certificate_id)
                        .bind(asset.ssl_enabled)
                        .bind(&asset.certificate_subject)
                        .bind(&asset.certificate_issuer)
                        .bind(&asset.certificate_valid_from)
                        .bind(&asset.certificate_valid_to)
                        .bind(&asset.certificate_san_json)
                        .bind(&asset.exposure_level)
                        .bind(asset.attack_surface_score)
                        .bind(asset.vulnerability_count)
                        .bind(asset.cvss_max_score)
                        .bind(asset.exploit_available)
                        .bind(&asset.asset_category)
                        .bind(&asset.asset_owner)
                        .bind(&asset.business_unit)
                        .bind(&asset.criticality)
                        .bind(&asset.discovery_method)
                        .bind(&asset.data_sources_json)
                        .bind(asset.confidence_score)
                        .bind(asset.monitoring_enabled)
                        .bind(&asset.scan_frequency)
                        .bind(&asset.last_scan_type)
                        .bind(&asset.parent_asset_id)
                        .bind(&asset.related_assets_json)
                        .bind(&asset.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
                        .bind(&asset.scope_id)
                        .bind(&asset.asset_type)
                        .bind(&asset.canonical_url)
                        .bind(&asset.original_urls_json)
                        .bind(&asset.hostname)
                        .bind(asset.port)
                        .bind(&asset.path)
                        .bind(&asset.protocol)
                        .bind(&asset.ip_addresses_json)
                        .bind(&asset.dns_records_json)
                        .bind(&asset.tech_stack_json)
                        .bind(&asset.fingerprint)
                        .bind(&asset.tags_json)
                        .bind(&asset.labels_json)
                        .bind(asset.priority_score)
                        .bind(asset.risk_score)
                        .bind(asset.is_alive)
                        .bind(&asset.last_checked_at)
                        .bind(&asset.last_seen_at)
                        .bind(asset.findings_count)
                        .bind(asset.change_events_count)
                        .bind(&asset.metadata_json)
                        .bind(&asset.updated_at)
                        .bind(&asset.ip_version)
                        .bind(asset.asn)
                        .bind(&asset.asn_org)
                        .bind(&asset.isp)
                        .bind(&asset.country)
                        .bind(&asset.city)
                        .bind(asset.latitude)
                        .bind(asset.longitude)
                        .bind(asset.is_cloud)
                        .bind(&asset.cloud_provider)
                        .bind(&asset.service_name)
                        .bind(&asset.service_version)
                        .bind(&asset.service_product)
                        .bind(&asset.banner)
                        .bind(&asset.transport_protocol)
                        .bind(&asset.cpe)
                        .bind(&asset.domain_registrar)
                        .bind(&asset.registration_date)
                        .bind(&asset.expiration_date)
                        .bind(&asset.nameservers_json)
                        .bind(&asset.mx_records_json)
                        .bind(&asset.txt_records_json)
                        .bind(&asset.whois_data_json)
                        .bind(asset.is_wildcard)
                        .bind(&asset.parent_domain)
                        .bind(&asset.root_domain)
                        .bind(asset.subdomain_level)
                        .bind(asset.http_status)
                        .bind(asset.response_time_ms)
                        .bind(asset.content_length)
                        .bind(&asset.content_type)
                        .bind(&asset.title)
                        .bind(&asset.favicon_hash)
                        .bind(&asset.headers_json)
                        .bind(&asset.waf_detected)
                        .bind(&asset.cdn_detected)
                        .bind(&asset.screenshot_path)
                        .bind(&asset.body_hash)
                        .bind(&asset.certificate_id)
                        .bind(asset.ssl_enabled)
                        .bind(&asset.certificate_subject)
                        .bind(&asset.certificate_issuer)
                        .bind(&asset.certificate_valid_from)
                        .bind(&asset.certificate_valid_to)
                        .bind(&asset.certificate_san_json)
                        .bind(&asset.exposure_level)
                        .bind(asset.attack_surface_score)
                        .bind(asset.vulnerability_count)
                        .bind(asset.cvss_max_score)
                        .bind(asset.exploit_available)
                        .bind(&asset.asset_category)
                        .bind(&asset.asset_owner)
                        .bind(&asset.business_unit)
                        .bind(&asset.criticality)
                        .bind(&asset.discovery_method)
                        .bind(&asset.data_sources_json)
                        .bind(asset.confidence_score)
                        .bind(asset.monitoring_enabled)
                        .bind(&asset.scan_frequency)
                        .bind(&asset.last_scan_type)
                        .bind(&asset.parent_asset_id)
                        .bind(&asset.related_assets_json)
                        .bind(&asset.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_assets SET
                scope_id = $1, asset_type = $2, canonical_url = $3, original_urls_json = $4,
                hostname = $5, port = $6, path = $7, protocol = $8, ip_addresses_json = $9,
                dns_records_json = $10, tech_stack_json = $11, fingerprint = $12, tags_json = $13,
                labels_json = $14, priority_score = $15, risk_score = $16, is_alive = $17,
                last_checked_at = $18, last_seen_at = $19, findings_count = $20,
                change_events_count = $21, metadata_json = $22, updated_at = $23,
                ip_version = $24, asn = $25, asn_org = $26, isp = $27, country = $28, city = $29, latitude = $30, longitude = $31,
                is_cloud = $32, cloud_provider = $33, service_name = $34, service_version = $35, service_product = $36, banner = $37,
                transport_protocol = $38, cpe = $39, domain_registrar = $40, registration_date = $41, expiration_date = $42,
                nameservers_json = $43, mx_records_json = $44, txt_records_json = $45, whois_data_json = $46,
                is_wildcard = $47, parent_domain = $48, root_domain = $49, subdomain_level = $50,
                http_status = $51, response_time_ms = $52, content_length = $53,
                content_type = $54, title = $55, favicon_hash = $56, headers_json = $57, waf_detected = $58, cdn_detected = $59,
                screenshot_path = $60, body_hash = $61, certificate_id = $62, ssl_enabled = $63, certificate_subject = $64,
                certificate_issuer = $65, certificate_valid_from = $66, certificate_valid_to = $67, certificate_san_json = $68,
                exposure_level = $69, attack_surface_score = $70, vulnerability_count = $71, cvss_max_score = $72,
                exploit_available = $73, asset_category = $74, asset_owner = $75, business_unit = $76, criticality = $77,
                discovery_method = $78, data_sources_json = $79, confidence_score = $80, monitoring_enabled = $81,
                scan_frequency = $82, last_scan_type = $83, parent_asset_id = $84, related_assets_json = $85
            WHERE id = $86"#,
        )
        .bind(&asset.scope_id)
        .bind(&asset.asset_type)
        .bind(&asset.canonical_url)
        .bind(&asset.original_urls_json)
        .bind(&asset.hostname)
        .bind(asset.port)
        .bind(&asset.path)
        .bind(&asset.protocol)
        .bind(&asset.ip_addresses_json)
        .bind(&asset.dns_records_json)
        .bind(&asset.tech_stack_json)
        .bind(&asset.fingerprint)
        .bind(&asset.tags_json)
        .bind(&asset.labels_json)
        .bind(asset.priority_score)
        .bind(asset.risk_score)
        .bind(asset.is_alive)
        .bind(optional_timestamp_string_to_datetime(
            &asset.last_checked_at,
        ))
        .bind(timestamp_string_to_datetime(&asset.last_seen_at))
        .bind(asset.findings_count)
        .bind(asset.change_events_count)
        .bind(&asset.metadata_json)
        .bind(timestamp_string_to_datetime(&asset.updated_at))
        .bind(&asset.ip_version)
        .bind(asset.asn)
        .bind(&asset.asn_org)
        .bind(&asset.isp)
        .bind(&asset.country)
        .bind(&asset.city)
        .bind(asset.latitude)
        .bind(asset.longitude)
        .bind(asset.is_cloud)
        .bind(&asset.cloud_provider)
        .bind(&asset.service_name)
        .bind(&asset.service_version)
        .bind(&asset.service_product)
        .bind(&asset.banner)
        .bind(&asset.transport_protocol)
        .bind(&asset.cpe)
        .bind(&asset.domain_registrar)
        .bind(&asset.registration_date)
        .bind(&asset.expiration_date)
        .bind(&asset.nameservers_json)
        .bind(&asset.mx_records_json)
        .bind(&asset.txt_records_json)
        .bind(&asset.whois_data_json)
        .bind(asset.is_wildcard)
        .bind(&asset.parent_domain)
        .bind(&asset.root_domain)
        .bind(asset.subdomain_level)
        .bind(asset.http_status)
        .bind(asset.response_time_ms)
        .bind(asset.content_length)
        .bind(&asset.content_type)
        .bind(&asset.title)
        .bind(&asset.favicon_hash)
        .bind(&asset.headers_json)
        .bind(&asset.waf_detected)
        .bind(&asset.cdn_detected)
        .bind(&asset.screenshot_path)
        .bind(&asset.body_hash)
        .bind(&asset.certificate_id)
        .bind(asset.ssl_enabled)
        .bind(&asset.certificate_subject)
        .bind(&asset.certificate_issuer)
        .bind(&asset.certificate_valid_from)
        .bind(&asset.certificate_valid_to)
        .bind(&asset.certificate_san_json)
        .bind(&asset.exposure_level)
        .bind(asset.attack_surface_score)
        .bind(asset.vulnerability_count)
        .bind(asset.cvss_max_score)
        .bind(asset.exploit_available)
        .bind(&asset.asset_category)
        .bind(&asset.asset_owner)
        .bind(&asset.business_unit)
        .bind(&asset.criticality)
        .bind(&asset.discovery_method)
        .bind(&asset.data_sources_json)
        .bind(asset.confidence_score)
        .bind(asset.monitoring_enabled)
        .bind(&asset.scan_frequency)
        .bind(&asset.last_scan_type)
        .bind(&asset.parent_asset_id)
        .bind(&asset.related_assets_json)
        .bind(&asset.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty asset
    pub async fn delete_bounty_asset(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_assets WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_assets WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_assets WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get bounty asset statistics
    pub async fn get_bounty_asset_stats(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyAssetStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let assets = self
                .list_bounty_assets(
                    program_id, None, None, None, None, None, None, None, None, None,
                )
                .await?;

            let total_assets = assets.len() as i32;
            let alive_assets = assets.iter().filter(|a| a.is_alive).count() as i32;
            let with_findings = assets.iter().filter(|a| a.findings_count > 0).count() as i32;
            let high_priority = assets
                .iter()
                .filter(|a| a.priority_score.unwrap_or(0.0) >= 7.0)
                .count() as i32;

            let mut by_type = std::collections::HashMap::new();
            for asset in assets {
                *by_type.entry(asset.asset_type).or_insert(0) += 1;
            }

            return Ok(BountyAssetStats {
                total_assets,
                alive_assets,
                by_type,
                with_findings,
                high_priority,
            });
        }

        let pool = self.get_pool()?;
        let filter = program_id
            .map(|_| " WHERE program_id = $1")
            .unwrap_or_default();

        // Note: For stats query with simple counts, binding $1 works even if ignored by some drivers, but Postgres is strict.
        // We need to use `bind` only if program_id exists.

        // Total
        let sql_total = format!("SELECT COUNT(*) FROM bounty_assets{}", filter);
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_total);
        if let Some(p) = program_id {
            q = q.bind(p);
        }
        let total = q.fetch_one(pool).await?;

        // Alive
        let sql_alive = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND is_alive = TRUE",
            if filter.is_empty() {
                " WHERE 1=1"
            } else {
                &filter
            }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_alive);
        if let Some(p) = program_id {
            q = q.bind(p);
        }
        let alive = q.fetch_one(pool).await?;

        // With findings
        let sql_findings = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND findings_count > 0",
            if filter.is_empty() {
                " WHERE 1=1"
            } else {
                &filter
            }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_findings);
        if let Some(p) = program_id {
            q = q.bind(p);
        }
        let with_findings = q.fetch_one(pool).await?;

        // High priority
        let sql_high = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND priority_score >= 7.0",
            if filter.is_empty() {
                " WHERE 1=1"
            } else {
                &filter
            }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_high);
        if let Some(p) = program_id {
            q = q.bind(p);
        }
        let high_priority = q.fetch_one(pool).await?;

        // Get by type
        let sql_by_type = format!(
            "SELECT asset_type, COUNT(*) FROM bounty_assets{} GROUP BY asset_type",
            filter
        );
        let mut q = sqlx::query_as::<_, (String, i32)>(&sql_by_type);
        if let Some(p) = program_id {
            q = q.bind(p);
        }
        let type_rows = q.fetch_all(pool).await?;

        let mut by_type = std::collections::HashMap::new();
        for (t, c) in type_rows {
            by_type.insert(t, c);
        }

        Ok(BountyAssetStats {
            total_assets: total.0,
            alive_assets: alive.0,
            by_type,
            with_findings: with_findings.0,
            high_priority: high_priority.0,
        })
    }

    /// Merge asset URLs (add original URL to existing asset)
    pub async fn merge_bounty_asset_url(&self, asset_id: &str, original_url: &str) -> Result<bool> {
        let asset = self.get_bounty_asset(asset_id).await?;
        let Some(mut asset) = asset else {
            return Ok(false);
        };

        let mut urls: Vec<String> = asset
            .original_urls_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if !urls.contains(&original_url.to_string()) {
            urls.push(original_url.to_string());
            asset.original_urls_json = Some(serde_json::to_string(&urls).unwrap_or_default());
            asset.updated_at = chrono::Utc::now().to_rfc3339();
            return self.update_bounty_asset(&asset).await;
        }

        Ok(true)
    }

    /// Get top priority assets
    pub async fn get_top_priority_assets(
        &self,
        program_id: &str,
        limit: i64,
    ) -> Result<Vec<BountyAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let mut assets = self
                .list_bounty_assets(
                    Some(program_id),
                    None,
                    None,
                    Some(true),
                    None,
                    None,
                    Some("priority_score"),
                    Some("desc"),
                    None,
                    None,
                )
                .await?;
            assets.truncate(limit.max(0) as usize);
            return Ok(assets);
        }

        let rows = sqlx::query(
            r#"SELECT * FROM bounty_assets
               WHERE program_id = $1 AND is_alive = TRUE
               ORDER BY priority_score DESC
               LIMIT $2"#,
        )
        .bind(program_id)
        .bind(limit)
        .fetch_all(self.get_pool()?)
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_asset).collect())
    }
}
