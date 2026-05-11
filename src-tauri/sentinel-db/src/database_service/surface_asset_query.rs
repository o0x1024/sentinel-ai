use crate::database_service::surface::SurfaceAssetFilter;
use sqlx::{Database, Encode, QueryBuilder, Type};

pub(crate) fn push_surface_text_like_clause<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    expression: &str,
    pattern: &str,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder
        .push(expression)
        .push_bind(pattern.to_string());
}

pub(crate) fn push_surface_extension_search_clause<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    asset_type: &str,
    table: &str,
    columns: &[&str],
    pattern: &str,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder
        .push("(surface_assets.asset_type = ")
        .push_bind(asset_type.to_string())
        .push(" AND EXISTS (SELECT 1 FROM ")
        .push(table)
        .push(" WHERE ")
        .push(table)
        .push(".asset_id = surface_assets.id AND (");

    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            query_builder.push(" OR ");
        }
        query_builder
            .push("LOWER(COALESCE(")
            .push(table)
            .push(".")
            .push(*column)
            .push(", '')) LIKE ");
        query_builder.push_bind(pattern.to_string());
    }

    query_builder.push(")))");
}

pub(crate) fn push_surface_asset_search_filters<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    filter: &SurfaceAssetFilter,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
    i32: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Some(search) = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    let pattern = format!("%{}%", search.to_lowercase());
    query_builder.push(" AND (");
    push_surface_text_like_clause(query_builder, "LOWER(asset_name) LIKE ", &pattern);
    query_builder.push(" OR ");
    push_surface_text_like_clause(
        query_builder,
        "LOWER(COALESCE(display_name, '')) LIKE ",
        &pattern,
    );
    query_builder.push(" OR ");
    push_surface_text_like_clause(query_builder, "LOWER(status) LIKE ", &pattern);
    query_builder.push(" OR ");
    push_surface_text_like_clause(
        query_builder,
        "LOWER(COALESCE(internet_exposure, '')) LIKE ",
        &pattern,
    );
    query_builder.push(" OR ");
    push_surface_text_like_clause(query_builder, "LOWER(COALESCE(source, '')) LIKE ", &pattern);

    let typed_search_targets: &[(&str, &str, &[&str])] = match filter.asset_type.as_deref() {
        Some("org") => &[(
            "org",
            "surface_org_assets",
            &[
                "org_name",
                "org_short_name",
                "business_line",
                "importance_level",
                "notes",
            ],
        )],
        Some("domain") => &[(
            "domain",
            "surface_domain_assets",
            &[
                "fqdn",
                "main_domain",
                "root_domain",
                "record_type",
                "record_value",
                "registrar",
                "whois_json",
            ],
        )],
        Some("ip") => &[(
            "ip",
            "surface_ip_assets",
            &[
                "ip_address",
                "ip_version",
                "cidr",
                "bgp_prefix",
                "country",
                "region",
                "city",
                "isp",
                "reverse_dns",
                "cloud_provider",
                "network_boundary_type",
            ],
        )],
        Some("host") => &[(
            "host",
            "surface_host_assets",
            &[
                "hostname",
                "fqdn",
                "operating_system",
                "device_type",
                "cloud_instance_id",
                "region_or_datacenter",
                "vpc_or_subnet",
                "mac_address",
                "agent_status",
                "labels_json",
                "lifecycle_status",
            ],
        )],
        Some("port") => &[(
            "port",
            "surface_port_assets",
            &[
                "ip_address",
                "transport_protocol",
                "port_state",
                "scan_source",
                "related_service_asset_id",
            ],
        )],
        Some("service") => &[(
            "service",
            "surface_service_assets",
            &[
                "ip_address",
                "transport_protocol",
                "protocol_name",
                "application_service_name",
                "banner",
                "product_name",
                "vendor",
                "version",
                "middleware_type",
                "component_fingerprint",
                "auth_type",
                "related_cves_json",
            ],
        )],
        Some("web") => &[(
            "web",
            "surface_web_assets",
            &[
                "canonical_url",
                "scheme",
                "ip_address",
                "site_title",
                "server_header",
                "response_headers_json",
                "page_fingerprint",
                "favicon_hash",
                "framework",
                "cms",
                "openapi_url",
                "business_type",
                "language",
                "filing_info",
                "content_summary",
                "sensitive_path_results_json",
            ],
        )],
        Some("certificate") => &[(
            "certificate",
            "surface_cert_assets",
            &[
                "sha256",
                "sha1",
                "serial_number",
                "public_key_algorithm",
                "signature_algorithm",
                "issuer",
                "subject",
                "san_list_json",
                "related_domains_json",
                "related_ips_json",
                "certificate_chain_json",
                "risk_status",
            ],
        )],
        _ => &[
            (
                "org",
                "surface_org_assets",
                &[
                    "org_name",
                    "org_short_name",
                    "business_line",
                    "importance_level",
                    "notes",
                ],
            ),
            (
                "domain",
                "surface_domain_assets",
                &[
                    "fqdn",
                    "main_domain",
                    "root_domain",
                    "record_type",
                    "record_value",
                    "registrar",
                    "whois_json",
                ],
            ),
            (
                "ip",
                "surface_ip_assets",
                &[
                    "ip_address",
                    "ip_version",
                    "cidr",
                    "bgp_prefix",
                    "country",
                    "region",
                    "city",
                    "isp",
                    "reverse_dns",
                    "cloud_provider",
                    "network_boundary_type",
                ],
            ),
            (
                "host",
                "surface_host_assets",
                &[
                    "hostname",
                    "fqdn",
                    "operating_system",
                    "device_type",
                    "cloud_instance_id",
                    "region_or_datacenter",
                    "vpc_or_subnet",
                    "mac_address",
                    "agent_status",
                    "labels_json",
                    "lifecycle_status",
                ],
            ),
            (
                "port",
                "surface_port_assets",
                &[
                    "ip_address",
                    "transport_protocol",
                    "port_state",
                    "scan_source",
                    "related_service_asset_id",
                ],
            ),
            (
                "service",
                "surface_service_assets",
                &[
                    "ip_address",
                    "transport_protocol",
                    "protocol_name",
                    "application_service_name",
                    "banner",
                    "product_name",
                    "vendor",
                    "version",
                    "middleware_type",
                    "component_fingerprint",
                    "auth_type",
                    "related_cves_json",
                ],
            ),
            (
                "web",
                "surface_web_assets",
                &[
                    "canonical_url",
                    "scheme",
                    "ip_address",
                    "site_title",
                    "server_header",
                    "response_headers_json",
                    "page_fingerprint",
                    "favicon_hash",
                    "framework",
                    "cms",
                    "openapi_url",
                    "business_type",
                    "language",
                    "filing_info",
                    "content_summary",
                    "sensitive_path_results_json",
                ],
            ),
            (
                "certificate",
                "surface_cert_assets",
                &[
                    "sha256",
                    "sha1",
                    "serial_number",
                    "public_key_algorithm",
                    "signature_algorithm",
                    "issuer",
                    "subject",
                    "san_list_json",
                    "related_domains_json",
                    "related_ips_json",
                    "certificate_chain_json",
                    "risk_status",
                ],
            ),
        ],
    };

    for (asset_type, table, columns) in typed_search_targets {
        query_builder.push(" OR ");
        push_surface_extension_search_clause(query_builder, asset_type, table, columns, &pattern);
    }

    query_builder.push(")");
}

pub(crate) fn push_surface_asset_filters<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    filter: &SurfaceAssetFilter,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
    i32: for<'q> Encode<'q, DB> + Type<DB>,
{
    if let Some(program_id) = filter.program_id.as_deref() {
        query_builder
            .push(" AND program_id = ")
            .push_bind(program_id.to_string());
    }
    if let Some(asset_type) = filter.asset_type.as_deref() {
        query_builder
            .push(" AND asset_type = ")
            .push_bind(asset_type.to_string());
    }
    if let Some(status) = filter.status.as_deref() {
        query_builder
            .push(" AND status = ")
            .push_bind(status.to_string());
    }
    if let Some(favicon_hash) = filter
        .favicon_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query_builder.push(
            " AND EXISTS (SELECT 1 FROM surface_web_assets WHERE surface_web_assets.asset_id = surface_assets.id AND COALESCE(surface_web_assets.favicon_hash, '') = ",
        );
        query_builder
            .push_bind(favicon_hash.to_string())
            .push(")");
    }
    if let Some(has_favicon_hash) = filter.has_favicon_hash {
        if has_favicon_hash {
            query_builder.push(
                " AND EXISTS (SELECT 1 FROM surface_web_assets WHERE surface_web_assets.asset_id = surface_assets.id AND LENGTH(TRIM(COALESCE(surface_web_assets.favicon_hash, ''))) > 0)",
            );
        } else {
            query_builder.push(
                " AND EXISTS (SELECT 1 FROM surface_web_assets WHERE surface_web_assets.asset_id = surface_assets.id AND LENGTH(TRIM(COALESCE(surface_web_assets.favicon_hash, ''))) = 0)",
            );
        }
    }
    if let Some(http_status_code) = filter.http_status_code {
        query_builder.push(
            " AND EXISTS (SELECT 1 FROM surface_web_assets WHERE surface_web_assets.asset_id = surface_assets.id AND surface_web_assets.http_status_code = ",
        );
        query_builder.push_bind(http_status_code).push(")");
    }
    if let Some(service_name) = filter
        .service_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query_builder.push(
            " AND EXISTS (SELECT 1 FROM surface_service_assets WHERE surface_service_assets.asset_id = surface_assets.id AND LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) = ",
        );
        query_builder
            .push_bind(service_name.to_lowercase())
            .push(")");
    }
    if let Some(transport_protocol) = filter
        .transport_protocol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let normalized = transport_protocol.to_lowercase();
        query_builder.push(" AND (");
        query_builder.push(
            "EXISTS (SELECT 1 FROM surface_service_assets WHERE surface_service_assets.asset_id = surface_assets.id AND LOWER(COALESCE(surface_service_assets.transport_protocol, '')) = ",
        );
        query_builder.push_bind(normalized.clone()).push(")");
        query_builder.push(" OR ");
        query_builder.push(
            "EXISTS (SELECT 1 FROM surface_port_assets WHERE surface_port_assets.asset_id = surface_assets.id AND LOWER(COALESCE(surface_port_assets.transport_protocol, '')) = ",
        );
        query_builder.push_bind(normalized).push(")");
        query_builder.push(")");
    }
    if let Some(view_state) = filter
        .view_state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match view_state {
            "new" => query_builder.push(" AND viewed_at IS NULL"),
            "viewed" => query_builder.push(" AND viewed_at IS NOT NULL"),
            _ => query_builder.push(" AND 1=0"),
        };
    }
    push_surface_asset_search_filters(query_builder, filter);
}

pub(crate) fn push_surface_asset_pagination<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    filter: &SurfaceAssetFilter,
    offset_without_limit_sql: Option<&str>,
) where
    DB: Database,
    i64: for<'q> Encode<'q, DB> + Type<DB>,
{
    let limit = filter.limit.map(|value| value.max(0));
    let offset = filter.offset.map(|value| value.max(0));

    if let Some(limit) = limit {
        query_builder.push(" LIMIT ").push_bind(limit);
        if let Some(offset) = offset {
            query_builder.push(" OFFSET ").push_bind(offset);
        }
        return;
    }

    if let Some(offset) = offset {
        if let Some(sql) = offset_without_limit_sql {
            query_builder.push(sql).push_bind(offset);
        } else {
            query_builder.push(" OFFSET ").push_bind(offset);
        }
    }
}
