use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use serde_json::Value;

fn as_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => Some(n.as_i64().unwrap_or_default() != 0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => Some(true),
            "false" | "0" | "no" | "n" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn as_i32(value: Option<&Value>) -> Option<i32> {
    value.and_then(|v| match v {
        Value::Number(n) => n.as_i64().map(|n| n as i32),
        Value::String(s) => s.parse::<i32>().ok(),
        _ => None,
    })
}

fn as_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn as_json_string(value: Option<&Value>) -> Option<String> {
    value.map(ToString::to_string)
}

impl DatabaseService {
    pub async fn upsert_surface_extension_from_artifact(
        &self,
        asset_type: &str,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        match asset_type {
            "domain" => {
                self.upsert_surface_domain_extension(asset_id, artifact)
                    .await
            }
            "ip" => self.upsert_surface_ip_extension(asset_id, artifact).await,
            "host" => self.upsert_surface_host_extension(asset_id, artifact).await,
            "port" => self.upsert_surface_port_extension(asset_id, artifact).await,
            "service" => {
                self.upsert_surface_service_extension(asset_id, artifact)
                    .await
            }
            "web" => self.upsert_surface_web_extension(asset_id, artifact).await,
            "certificate" => self.upsert_surface_cert_extension(asset_id, artifact).await,
            _ => Ok(()),
        }
    }

    async fn delete_surface_extension_row(&self, table: &str, asset_id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let sql = format!("DELETE FROM {table} WHERE asset_id = ?");
                sqlx::query(&sql).bind(asset_id).execute(pool).await?;
            }
            DatabasePool::MySQL(pool) => {
                let sql = format!("DELETE FROM {table} WHERE asset_id = ?");
                sqlx::query(&sql).bind(asset_id).execute(pool).await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                let sql = format!("DELETE FROM {table} WHERE asset_id = $1");
                sqlx::query(&sql).bind(asset_id).execute(pool).await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_domain_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_domain_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let fqdn = as_string(artifact.get("fqdn")).unwrap_or_default();
        let main_domain = as_string(artifact.get("main_domain"));
        let root_domain = as_string(artifact.get("root_domain"));
        let subdomain_level = as_i32(artifact.get("subdomain_level"));
        let record_type = as_string(artifact.get("record_type"));
        let record_value = as_string(artifact.get("record_value"));
        let ttl = as_i32(artifact.get("ttl"));
        let registrar = as_string(artifact.get("registrar"));
        let registered_at = as_string(artifact.get("registered_at"));
        let expires_at = as_string(artifact.get("expires_at"));
        let whois_json = as_json_string(artifact.get("whois"));
        let wildcard_enabled = as_bool(artifact.get("wildcard_enabled"));
        let dnssec_enabled = as_bool(artifact.get("dnssec_enabled"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_domain_assets (asset_id, fqdn, main_domain, root_domain, subdomain_level, record_type, record_value, ttl, registrar, registered_at, expires_at, whois_json, wildcard_enabled, dnssec_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&fqdn)
                .bind(&main_domain)
                .bind(&root_domain)
                .bind(subdomain_level)
                .bind(&record_type)
                .bind(&record_value)
                .bind(ttl)
                .bind(&registrar)
                .bind(&registered_at)
                .bind(&expires_at)
                .bind(&whois_json)
                .bind(wildcard_enabled)
                .bind(dnssec_enabled)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_domain_assets (asset_id, fqdn, main_domain, root_domain, subdomain_level, record_type, record_value, ttl, registrar, registered_at, expires_at, whois_json, wildcard_enabled, dnssec_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&fqdn)
                .bind(&main_domain)
                .bind(&root_domain)
                .bind(subdomain_level)
                .bind(&record_type)
                .bind(&record_value)
                .bind(ttl)
                .bind(&registrar)
                .bind(&registered_at)
                .bind(&expires_at)
                .bind(&whois_json)
                .bind(wildcard_enabled)
                .bind(dnssec_enabled)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_domain_assets (asset_id, fqdn, main_domain, root_domain, subdomain_level, record_type, record_value, ttl, registrar, registered_at, expires_at, whois_json, wildcard_enabled, dnssec_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
                )
                .bind(asset_id)
                .bind(&fqdn)
                .bind(&main_domain)
                .bind(&root_domain)
                .bind(subdomain_level)
                .bind(&record_type)
                .bind(&record_value)
                .bind(ttl)
                .bind(&registrar)
                .bind(&registered_at)
                .bind(&expires_at)
                .bind(&whois_json)
                .bind(wildcard_enabled)
                .bind(dnssec_enabled)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_ip_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_ip_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let ip_address = as_string(artifact.get("ip_address"))
            .or_else(|| as_string(artifact.get("ip")))
            .unwrap_or_default();
        let ip_version = as_string(artifact.get("ip_version"));
        let cidr = as_string(artifact.get("cidr"));
        let asn = as_i32(artifact.get("asn"));
        let bgp_prefix = as_string(artifact.get("bgp_prefix"));
        let country = as_string(artifact.get("country"));
        let region = as_string(artifact.get("region"));
        let city = as_string(artifact.get("city"));
        let isp = as_string(artifact.get("isp"));
        let reverse_dns = as_string(artifact.get("reverse_dns"));
        let cloud_provider = as_string(artifact.get("cloud_provider"));
        let network_boundary_type = as_string(artifact.get("network_boundary_type"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_ip_assets (asset_id, ip_address, ip_version, cidr, asn, bgp_prefix, country, region, city, isp, reverse_dns, cloud_provider, network_boundary_type) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&ip_address)
                .bind(&ip_version)
                .bind(&cidr)
                .bind(asn)
                .bind(&bgp_prefix)
                .bind(&country)
                .bind(&region)
                .bind(&city)
                .bind(&isp)
                .bind(&reverse_dns)
                .bind(&cloud_provider)
                .bind(&network_boundary_type)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_ip_assets (asset_id, ip_address, ip_version, cidr, asn, bgp_prefix, country, region, city, isp, reverse_dns, cloud_provider, network_boundary_type) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&ip_address)
                .bind(&ip_version)
                .bind(&cidr)
                .bind(asn)
                .bind(&bgp_prefix)
                .bind(&country)
                .bind(&region)
                .bind(&city)
                .bind(&isp)
                .bind(&reverse_dns)
                .bind(&cloud_provider)
                .bind(&network_boundary_type)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_ip_assets (asset_id, ip_address, ip_version, cidr, asn, bgp_prefix, country, region, city, isp, reverse_dns, cloud_provider, network_boundary_type) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                )
                .bind(asset_id)
                .bind(&ip_address)
                .bind(&ip_version)
                .bind(&cidr)
                .bind(asn)
                .bind(&bgp_prefix)
                .bind(&country)
                .bind(&region)
                .bind(&city)
                .bind(&isp)
                .bind(&reverse_dns)
                .bind(&cloud_provider)
                .bind(&network_boundary_type)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_host_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_host_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let hostname = as_string(artifact.get("hostname"))
            .or_else(|| as_string(artifact.get("fqdn")))
            .unwrap_or_default();
        let fqdn = as_string(artifact.get("fqdn"));
        let ip_addresses_json = as_json_string(artifact.get("ip_addresses"))
            .or_else(|| as_json_string(artifact.get("ip_list")));
        let operating_system = as_string(artifact.get("operating_system"));
        let device_type = as_string(artifact.get("device_type"));
        let cloud_instance_id = as_string(artifact.get("cloud_instance_id"));
        let region_or_datacenter = as_string(artifact.get("region_or_datacenter"));
        let vpc_or_subnet = as_string(artifact.get("vpc_or_subnet"));
        let mac_address = as_string(artifact.get("mac_address"));
        let agent_status = as_string(artifact.get("agent_status"));
        let labels_json = as_json_string(artifact.get("labels"));
        let lifecycle_status = as_string(artifact.get("lifecycle_status"));
        let last_online_at = as_string(artifact.get("last_online_at"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_host_assets (asset_id, hostname, fqdn, ip_addresses_json, operating_system, device_type, cloud_instance_id, region_or_datacenter, vpc_or_subnet, mac_address, agent_status, labels_json, lifecycle_status, last_online_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&hostname)
                .bind(&fqdn)
                .bind(&ip_addresses_json)
                .bind(&operating_system)
                .bind(&device_type)
                .bind(&cloud_instance_id)
                .bind(&region_or_datacenter)
                .bind(&vpc_or_subnet)
                .bind(&mac_address)
                .bind(&agent_status)
                .bind(&labels_json)
                .bind(&lifecycle_status)
                .bind(&last_online_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_host_assets (asset_id, hostname, fqdn, ip_addresses_json, operating_system, device_type, cloud_instance_id, region_or_datacenter, vpc_or_subnet, mac_address, agent_status, labels_json, lifecycle_status, last_online_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&hostname)
                .bind(&fqdn)
                .bind(&ip_addresses_json)
                .bind(&operating_system)
                .bind(&device_type)
                .bind(&cloud_instance_id)
                .bind(&region_or_datacenter)
                .bind(&vpc_or_subnet)
                .bind(&mac_address)
                .bind(&agent_status)
                .bind(&labels_json)
                .bind(&lifecycle_status)
                .bind(&last_online_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_host_assets (asset_id, hostname, fqdn, ip_addresses_json, operating_system, device_type, cloud_instance_id, region_or_datacenter, vpc_or_subnet, mac_address, agent_status, labels_json, lifecycle_status, last_online_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
                )
                .bind(asset_id)
                .bind(&hostname)
                .bind(&fqdn)
                .bind(&ip_addresses_json)
                .bind(&operating_system)
                .bind(&device_type)
                .bind(&cloud_instance_id)
                .bind(&region_or_datacenter)
                .bind(&vpc_or_subnet)
                .bind(&mac_address)
                .bind(&agent_status)
                .bind(&labels_json)
                .bind(&lifecycle_status)
                .bind(&last_online_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_port_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_port_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let host_asset_id = as_string(artifact.get("host_asset_id"));
        let ip_address =
            as_string(artifact.get("ip_or_host")).or_else(|| as_string(artifact.get("ip_address")));
        let port_number = as_i32(artifact.get("port")).unwrap_or_default();
        let transport_protocol =
            as_string(artifact.get("transport_protocol")).unwrap_or_else(|| "tcp".to_string());
        let port_state = as_string(artifact.get("state"));
        let scan_source = as_string(artifact.get("source"));
        let related_service_asset_id = as_string(artifact.get("related_service_asset_id"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_port_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, port_state, scan_source, related_service_asset_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&port_state)
                .bind(&scan_source)
                .bind(&related_service_asset_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_port_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, port_state, scan_source, related_service_asset_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&port_state)
                .bind(&scan_source)
                .bind(&related_service_asset_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_port_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, port_state, scan_source, related_service_asset_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&port_state)
                .bind(&scan_source)
                .bind(&related_service_asset_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_service_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_service_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let host_asset_id = as_string(artifact.get("host_asset_id"));
        let ip_address =
            as_string(artifact.get("ip_or_host")).or_else(|| as_string(artifact.get("ip_address")));
        let port_number = as_i32(artifact.get("port"));
        let transport_protocol = as_string(artifact.get("transport_protocol"));
        let protocol_name = as_string(artifact.get("protocol_name"));
        let application_service_name = as_string(artifact.get("application_service_name"));
        let banner = as_string(artifact.get("banner"));
        let product_name = as_string(artifact.get("product_name"));
        let vendor = as_string(artifact.get("vendor"));
        let version = as_string(artifact.get("version"));
        let middleware_type = as_string(artifact.get("middleware_type"));
        let component_fingerprint = as_string(artifact.get("component_fingerprint"));
        let auth_type = as_string(artifact.get("auth_type"));
        let login_required = as_bool(artifact.get("login_required"));
        let weak_password_risk = as_bool(artifact.get("weak_password_risk"));
        let encrypted_transport = as_bool(artifact.get("encrypted_transport"));
        let related_cves_json = as_json_string(artifact.get("related_cves"));
        let related_domain_asset_id = as_string(artifact.get("related_domain_asset_id"));
        let system_asset_id = as_string(artifact.get("system_asset_id"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_service_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, protocol_name, application_service_name, banner, product_name, vendor, version, middleware_type, component_fingerprint, auth_type, login_required, weak_password_risk, encrypted_transport, related_cves_json, related_domain_asset_id, system_asset_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&protocol_name)
                .bind(&application_service_name)
                .bind(&banner)
                .bind(&product_name)
                .bind(&vendor)
                .bind(&version)
                .bind(&middleware_type)
                .bind(&component_fingerprint)
                .bind(&auth_type)
                .bind(login_required)
                .bind(weak_password_risk)
                .bind(encrypted_transport)
                .bind(&related_cves_json)
                .bind(&related_domain_asset_id)
                .bind(&system_asset_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_service_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, protocol_name, application_service_name, banner, product_name, vendor, version, middleware_type, component_fingerprint, auth_type, login_required, weak_password_risk, encrypted_transport, related_cves_json, related_domain_asset_id, system_asset_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&protocol_name)
                .bind(&application_service_name)
                .bind(&banner)
                .bind(&product_name)
                .bind(&vendor)
                .bind(&version)
                .bind(&middleware_type)
                .bind(&component_fingerprint)
                .bind(&auth_type)
                .bind(login_required)
                .bind(weak_password_risk)
                .bind(encrypted_transport)
                .bind(&related_cves_json)
                .bind(&related_domain_asset_id)
                .bind(&system_asset_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_service_assets (asset_id, host_asset_id, ip_address, port_number, transport_protocol, protocol_name, application_service_name, banner, product_name, vendor, version, middleware_type, component_fingerprint, auth_type, login_required, weak_password_risk, encrypted_transport, related_cves_json, related_domain_asset_id, system_asset_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
                )
                .bind(asset_id)
                .bind(&host_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&transport_protocol)
                .bind(&protocol_name)
                .bind(&application_service_name)
                .bind(&banner)
                .bind(&product_name)
                .bind(&vendor)
                .bind(&version)
                .bind(&middleware_type)
                .bind(&component_fingerprint)
                .bind(&auth_type)
                .bind(login_required)
                .bind(weak_password_risk)
                .bind(encrypted_transport)
                .bind(&related_cves_json)
                .bind(&related_domain_asset_id)
                .bind(&system_asset_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_web_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_web_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let canonical_url = as_string(artifact.get("canonical_url"))
            .or_else(|| as_string(artifact.get("url")))
            .unwrap_or_default();
        let scheme = as_string(artifact.get("scheme"));
        let service_asset_id = as_string(artifact.get("service_asset_id"));
        let domain_asset_id = as_string(artifact.get("domain_asset_id"));
        let ip_address =
            as_string(artifact.get("ip_address")).or_else(|| as_string(artifact.get("hostname")));
        let port_number = as_i32(artifact.get("port"));
        let site_title = as_string(artifact.get("site_title"));
        let http_status_code = as_i32(artifact.get("http_status_code"));
        let server_header = as_string(artifact.get("server_header"));
        let response_headers_json = as_json_string(artifact.get("response_headers"));
        let page_fingerprint = as_string(artifact.get("page_fingerprint"));
        let favicon_hash = as_string(artifact.get("favicon_hash"));
        let screenshot_path = as_string(artifact.get("screenshot_path"));
        let framework = as_string(artifact.get("framework"));
        let cms = as_string(artifact.get("cms"));
        let waf_flag = as_bool(artifact.get("waf_flag"));
        let cdn_flag = as_bool(artifact.get("cdn_flag"));
        let login_flag = as_bool(artifact.get("login_flag"));
        let api_flag = as_bool(artifact.get("api_flag"));
        let openapi_url = as_string(artifact.get("openapi_url"));
        let business_type = as_string(artifact.get("business_type"));
        let language = as_string(artifact.get("language"));
        let filing_info = as_string(artifact.get("filing_info"));
        let content_summary = as_string(artifact.get("content_summary"));
        let sensitive_path_results_json = as_json_string(artifact.get("sensitive_path_results"));
        let last_accessed_at = as_string(artifact.get("last_accessed_at"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_web_assets (asset_id, service_asset_id, canonical_url, scheme, domain_asset_id, ip_address, port_number, site_title, http_status_code, server_header, response_headers_json, page_fingerprint, favicon_hash, screenshot_path, framework, cms, waf_flag, cdn_flag, login_flag, api_flag, openapi_url, business_type, language, filing_info, content_summary, sensitive_path_results_json, last_accessed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&service_asset_id)
                .bind(&canonical_url)
                .bind(&scheme)
                .bind(&domain_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&site_title)
                .bind(http_status_code)
                .bind(&server_header)
                .bind(&response_headers_json)
                .bind(&page_fingerprint)
                .bind(&favicon_hash)
                .bind(&screenshot_path)
                .bind(&framework)
                .bind(&cms)
                .bind(waf_flag)
                .bind(cdn_flag)
                .bind(login_flag)
                .bind(api_flag)
                .bind(&openapi_url)
                .bind(&business_type)
                .bind(&language)
                .bind(&filing_info)
                .bind(&content_summary)
                .bind(&sensitive_path_results_json)
                .bind(&last_accessed_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_web_assets (asset_id, service_asset_id, canonical_url, scheme, domain_asset_id, ip_address, port_number, site_title, http_status_code, server_header, response_headers_json, page_fingerprint, favicon_hash, screenshot_path, framework, cms, waf_flag, cdn_flag, login_flag, api_flag, openapi_url, business_type, language, filing_info, content_summary, sensitive_path_results_json, last_accessed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&service_asset_id)
                .bind(&canonical_url)
                .bind(&scheme)
                .bind(&domain_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&site_title)
                .bind(http_status_code)
                .bind(&server_header)
                .bind(&response_headers_json)
                .bind(&page_fingerprint)
                .bind(&favicon_hash)
                .bind(&screenshot_path)
                .bind(&framework)
                .bind(&cms)
                .bind(waf_flag)
                .bind(cdn_flag)
                .bind(login_flag)
                .bind(api_flag)
                .bind(&openapi_url)
                .bind(&business_type)
                .bind(&language)
                .bind(&filing_info)
                .bind(&content_summary)
                .bind(&sensitive_path_results_json)
                .bind(&last_accessed_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_web_assets (asset_id, service_asset_id, canonical_url, scheme, domain_asset_id, ip_address, port_number, site_title, http_status_code, server_header, response_headers_json, page_fingerprint, favicon_hash, screenshot_path, framework, cms, waf_flag, cdn_flag, login_flag, api_flag, openapi_url, business_type, language, filing_info, content_summary, sensitive_path_results_json, last_accessed_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)",
                )
                .bind(asset_id)
                .bind(&service_asset_id)
                .bind(&canonical_url)
                .bind(&scheme)
                .bind(&domain_asset_id)
                .bind(&ip_address)
                .bind(port_number)
                .bind(&site_title)
                .bind(http_status_code)
                .bind(&server_header)
                .bind(&response_headers_json)
                .bind(&page_fingerprint)
                .bind(&favicon_hash)
                .bind(&screenshot_path)
                .bind(&framework)
                .bind(&cms)
                .bind(waf_flag)
                .bind(cdn_flag)
                .bind(login_flag)
                .bind(api_flag)
                .bind(&openapi_url)
                .bind(&business_type)
                .bind(&language)
                .bind(&filing_info)
                .bind(&content_summary)
                .bind(&sensitive_path_results_json)
                .bind(&last_accessed_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_surface_cert_extension(
        &self,
        asset_id: &str,
        artifact: &Value,
    ) -> Result<()> {
        self.delete_surface_extension_row("surface_cert_assets", asset_id)
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sha256 = as_string(artifact.get("sha256")).unwrap_or_default();
        let sha1 = as_string(artifact.get("sha1"));
        let serial_number = as_string(artifact.get("serial_number"));
        let public_key_algorithm = as_string(artifact.get("public_key_algorithm"));
        let signature_algorithm = as_string(artifact.get("signature_algorithm"))
            .or_else(|| as_string(artifact.get("protocol")));
        let issuer = as_string(artifact.get("issuer"));
        let subject = as_string(artifact.get("subject"));
        let san_list_json = as_json_string(artifact.get("san_list"));
        let valid_from = as_string(artifact.get("valid_from"));
        let valid_to = as_string(artifact.get("valid_to"));
        let self_signed_flag = as_bool(artifact.get("self_signed_flag"));
        let related_domains_json = as_json_string(artifact.get("related_domains"))
            .or_else(|| as_json_string(artifact.get("san_list")));
        let related_ips_json = as_json_string(artifact.get("related_ips"));
        let certificate_chain_json = as_json_string(artifact.get("certificate_chain"));
        let risk_status = as_string(artifact.get("risk_status"));

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_cert_assets (asset_id, sha256, sha1, serial_number, public_key_algorithm, signature_algorithm, issuer, subject, san_list_json, valid_from, valid_to, self_signed_flag, related_domains_json, related_ips_json, certificate_chain_json, risk_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&sha256)
                .bind(&sha1)
                .bind(&serial_number)
                .bind(&public_key_algorithm)
                .bind(&signature_algorithm)
                .bind(&issuer)
                .bind(&subject)
                .bind(&san_list_json)
                .bind(&valid_from)
                .bind(&valid_to)
                .bind(self_signed_flag)
                .bind(&related_domains_json)
                .bind(&related_ips_json)
                .bind(&certificate_chain_json)
                .bind(&risk_status)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_cert_assets (asset_id, sha256, sha1, serial_number, public_key_algorithm, signature_algorithm, issuer, subject, san_list_json, valid_from, valid_to, self_signed_flag, related_domains_json, related_ips_json, certificate_chain_json, risk_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(asset_id)
                .bind(&sha256)
                .bind(&sha1)
                .bind(&serial_number)
                .bind(&public_key_algorithm)
                .bind(&signature_algorithm)
                .bind(&issuer)
                .bind(&subject)
                .bind(&san_list_json)
                .bind(&valid_from)
                .bind(&valid_to)
                .bind(self_signed_flag)
                .bind(&related_domains_json)
                .bind(&related_ips_json)
                .bind(&certificate_chain_json)
                .bind(&risk_status)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_cert_assets (asset_id, sha256, sha1, serial_number, public_key_algorithm, signature_algorithm, issuer, subject, san_list_json, valid_from, valid_to, self_signed_flag, related_domains_json, related_ips_json, certificate_chain_json, risk_status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
                )
                .bind(asset_id)
                .bind(&sha256)
                .bind(&sha1)
                .bind(&serial_number)
                .bind(&public_key_algorithm)
                .bind(&signature_algorithm)
                .bind(&issuer)
                .bind(&subject)
                .bind(&san_list_json)
                .bind(&valid_from)
                .bind(&valid_to)
                .bind(self_signed_flag)
                .bind(&related_domains_json)
                .bind(&related_ips_json)
                .bind(&certificate_chain_json)
                .bind(&risk_status)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}
