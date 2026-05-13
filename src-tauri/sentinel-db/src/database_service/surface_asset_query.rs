use crate::database_service::surface::{SurfaceAssetColumnFilter, SurfaceAssetFilter};
use crate::database_service::surface_inventory::SurfaceInventoryCursor;
use sqlx::{Database, Encode, QueryBuilder, Type};

#[derive(Clone, Copy)]
enum SurfaceColumnFilterKind {
    Text,
    Integer,
}

enum SurfaceColumnFilterTarget {
    Asset {
        column: &'static str,
        kind: SurfaceColumnFilterKind,
    },
    Typed {
        asset_type: &'static str,
        table: &'static str,
        column: &'static str,
        kind: SurfaceColumnFilterKind,
    },
}

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

fn resolve_surface_column_filter_target(
    selected_asset_type: Option<&str>,
    key: &str,
) -> Option<SurfaceColumnFilterTarget> {
    let asset_target = match key {
        "type" => Some(SurfaceColumnFilterTarget::Asset {
            column: "asset_type",
            kind: SurfaceColumnFilterKind::Text,
        }),
        "name" => Some(SurfaceColumnFilterTarget::Asset {
            column: "asset_name",
            kind: SurfaceColumnFilterKind::Text,
        }),
        "display" => Some(SurfaceColumnFilterTarget::Asset {
            column: "display_name",
            kind: SurfaceColumnFilterKind::Text,
        }),
        "status" => Some(SurfaceColumnFilterTarget::Asset {
            column: "status",
            kind: SurfaceColumnFilterKind::Text,
        }),
        "exposure" => Some(SurfaceColumnFilterTarget::Asset {
            column: "internet_exposure",
            kind: SurfaceColumnFilterKind::Text,
        }),
        "source" => Some(SurfaceColumnFilterTarget::Asset {
            column: "source",
            kind: SurfaceColumnFilterKind::Text,
        }),
        _ => None,
    };
    if asset_target.is_some() {
        return asset_target;
    }

    let typed_target = match key {
        "org_name" => Some((
            "org",
            "surface_org_assets",
            "org_name",
            SurfaceColumnFilterKind::Text,
        )),
        "business_line" => Some((
            "org",
            "surface_org_assets",
            "business_line",
            SurfaceColumnFilterKind::Text,
        )),
        "importance_level" => Some((
            "org",
            "surface_org_assets",
            "importance_level",
            SurfaceColumnFilterKind::Text,
        )),
        "root_domain" => Some((
            "domain",
            "surface_domain_assets",
            "root_domain",
            SurfaceColumnFilterKind::Text,
        )),
        "subdomain_level" => Some((
            "domain",
            "surface_domain_assets",
            "subdomain_level",
            SurfaceColumnFilterKind::Integer,
        )),
        "record_type" => Some((
            "domain",
            "surface_domain_assets",
            "record_type",
            SurfaceColumnFilterKind::Text,
        )),
        "record_value" => Some((
            "domain",
            "surface_domain_assets",
            "record_value",
            SurfaceColumnFilterKind::Text,
        )),
        "registrar" => Some((
            "domain",
            "surface_domain_assets",
            "registrar",
            SurfaceColumnFilterKind::Text,
        )),
        "cidr" => Some((
            "ip",
            "surface_ip_assets",
            "cidr",
            SurfaceColumnFilterKind::Text,
        )),
        "asn" => Some((
            "ip",
            "surface_ip_assets",
            "asn",
            SurfaceColumnFilterKind::Integer,
        )),
        "cloud_provider" => Some((
            "ip",
            "surface_ip_assets",
            "cloud_provider",
            SurfaceColumnFilterKind::Text,
        )),
        "network_boundary_type" => Some((
            "ip",
            "surface_ip_assets",
            "network_boundary_type",
            SurfaceColumnFilterKind::Text,
        )),
        "fqdn" => Some((
            "host",
            "surface_host_assets",
            "fqdn",
            SurfaceColumnFilterKind::Text,
        )),
        "operating_system" => Some((
            "host",
            "surface_host_assets",
            "operating_system",
            SurfaceColumnFilterKind::Text,
        )),
        "device_type" => Some((
            "host",
            "surface_host_assets",
            "device_type",
            SurfaceColumnFilterKind::Text,
        )),
        "region_or_datacenter" => Some((
            "host",
            "surface_host_assets",
            "region_or_datacenter",
            SurfaceColumnFilterKind::Text,
        )),
        "ip_address" => Some((
            "port",
            "surface_port_assets",
            "ip_address",
            SurfaceColumnFilterKind::Text,
        )),
        "port_number" => match selected_asset_type {
            Some("port") => Some((
                "port",
                "surface_port_assets",
                "port_number",
                SurfaceColumnFilterKind::Integer,
            )),
            _ => Some((
                "service",
                "surface_service_assets",
                "port_number",
                SurfaceColumnFilterKind::Integer,
            )),
        },
        "transport_protocol" => match selected_asset_type {
            Some("port") => Some((
                "port",
                "surface_port_assets",
                "transport_protocol",
                SurfaceColumnFilterKind::Text,
            )),
            _ => Some((
                "service",
                "surface_service_assets",
                "transport_protocol",
                SurfaceColumnFilterKind::Text,
            )),
        },
        "port_state" => Some((
            "port",
            "surface_port_assets",
            "port_state",
            SurfaceColumnFilterKind::Text,
        )),
        "application_service_name" => Some((
            "service",
            "surface_service_assets",
            "application_service_name",
            SurfaceColumnFilterKind::Text,
        )),
        "product_name" => Some((
            "service",
            "surface_service_assets",
            "product_name",
            SurfaceColumnFilterKind::Text,
        )),
        "version" => Some((
            "service",
            "surface_service_assets",
            "version",
            SurfaceColumnFilterKind::Text,
        )),
        "auth_type" => Some((
            "service",
            "surface_service_assets",
            "auth_type",
            SurfaceColumnFilterKind::Text,
        )),
        "canonical_url" => Some((
            "web",
            "surface_web_assets",
            "canonical_url",
            SurfaceColumnFilterKind::Text,
        )),
        "site_title" => Some((
            "web",
            "surface_web_assets",
            "site_title",
            SurfaceColumnFilterKind::Text,
        )),
        "http_status_code" => Some((
            "web",
            "surface_web_assets",
            "http_status_code",
            SurfaceColumnFilterKind::Integer,
        )),
        "favicon_hash" => Some((
            "web",
            "surface_web_assets",
            "favicon_hash",
            SurfaceColumnFilterKind::Text,
        )),
        "subject" => Some((
            "certificate",
            "surface_cert_assets",
            "subject",
            SurfaceColumnFilterKind::Text,
        )),
        "issuer" => Some((
            "certificate",
            "surface_cert_assets",
            "issuer",
            SurfaceColumnFilterKind::Text,
        )),
        "risk_status" => Some((
            "certificate",
            "surface_cert_assets",
            "risk_status",
            SurfaceColumnFilterKind::Text,
        )),
        "sha256" => Some((
            "certificate",
            "surface_cert_assets",
            "sha256",
            SurfaceColumnFilterKind::Text,
        )),
        _ => None,
    }?;

    if let Some(selected_asset_type) = selected_asset_type {
        if selected_asset_type != typed_target.0 {
            return None;
        }
    }

    Some(SurfaceColumnFilterTarget::Typed {
        asset_type: typed_target.0,
        table: typed_target.1,
        column: typed_target.2,
        kind: typed_target.3,
    })
}

fn push_no_match_clause<'args, DB>(query_builder: &mut QueryBuilder<'args, DB>)
where
    DB: Database,
{
    query_builder.push(" AND 1=0");
}

fn push_text_column_filter<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    expression_prefix: &str,
    value: &str,
    operator: &str,
) -> bool
where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    match operator {
        "equals" => {
            query_builder.push(expression_prefix).push(" = ");
            query_builder.push_bind(value.to_lowercase());
            true
        }
        "contains" => {
            query_builder.push(expression_prefix).push(" LIKE ");
            query_builder.push_bind(format!("%{}%", value.to_lowercase()));
            true
        }
        _ => false,
    }
}

fn push_integer_column_filter<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    expression_prefix: &str,
    value: &str,
) -> bool
where
    DB: Database,
    i32: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Ok(parsed) = value.parse::<i32>() else {
        return false;
    };
    query_builder.push(expression_prefix).push(" = ");
    query_builder.push_bind(parsed);
    true
}

fn push_surface_asset_column_filter<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    filter: &SurfaceAssetColumnFilter,
    selected_asset_type: Option<&str>,
) -> bool
where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
    i32: for<'q> Encode<'q, DB> + Type<DB>,
{
    let key = filter.key.trim();
    let value = filter.value.trim();
    if key.is_empty() || value.is_empty() {
        return true;
    }

    let operator = filter
        .operator
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("contains");
    let Some(target) = resolve_surface_column_filter_target(selected_asset_type, key) else {
        return false;
    };

    match target {
        SurfaceColumnFilterTarget::Asset { column, kind } => match kind {
            SurfaceColumnFilterKind::Text => {
                if !matches!(operator, "equals" | "contains") {
                    return false;
                }
                query_builder.push(" AND ");
                let expression = format!("LOWER(COALESCE(surface_assets.{column}, ''))");
                push_text_column_filter(query_builder, &expression, value, operator)
            }
            SurfaceColumnFilterKind::Integer => {
                if value.parse::<i32>().is_err() {
                    return false;
                }
                query_builder.push(" AND ");
                let expression = format!("surface_assets.{column}");
                push_integer_column_filter(query_builder, &expression, value)
            }
        },
        SurfaceColumnFilterTarget::Typed {
            asset_type,
            table,
            column,
            kind,
        } => {
            match kind {
                SurfaceColumnFilterKind::Text => {
                    if !matches!(operator, "equals" | "contains") {
                        return false;
                    }
                }
                SurfaceColumnFilterKind::Integer => {
                    if value.parse::<i32>().is_err() {
                        return false;
                    }
                }
            }
            query_builder
                .push(" AND surface_assets.asset_type = ")
                .push_bind(asset_type.to_string())
                .push(" AND EXISTS (SELECT 1 FROM ")
                .push(table)
                .push(" WHERE ")
                .push(table)
                .push(".asset_id = surface_assets.id AND ");
            let applied = match kind {
                SurfaceColumnFilterKind::Text => {
                    let expression = format!("LOWER(COALESCE({table}.{column}, ''))");
                    push_text_column_filter(query_builder, &expression, value, operator)
                }
                SurfaceColumnFilterKind::Integer => {
                    let expression = format!("{table}.{column}");
                    push_integer_column_filter(query_builder, &expression, value)
                }
            };
            query_builder.push(")");
            applied
        }
    }
}

fn push_surface_asset_column_filters<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    filter: &SurfaceAssetFilter,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
    i32: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Some(column_filters) = filter.column_filters.as_ref() else {
        return;
    };

    for column_filter in column_filters {
        if !push_surface_asset_column_filter(
            query_builder,
            column_filter,
            filter.asset_type.as_deref(),
        ) {
            push_no_match_clause(query_builder);
        }
    }
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
        query_builder.push_bind(favicon_hash.to_string()).push(")");
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
    if let Some(is_favorite) = filter.is_favorite {
        query_builder.push(if is_favorite {
            " AND is_favorite = TRUE"
        } else {
            " AND is_favorite = FALSE"
        });
    }
    push_surface_asset_column_filters(query_builder, filter);
    push_surface_asset_search_filters(query_builder, filter);
}

pub(crate) fn push_surface_asset_cursor<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    cursor: Option<&SurfaceInventoryCursor>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Some(cursor) = cursor else {
        return;
    };
    if cursor.last_seen_at.trim().is_empty() || cursor.id.trim().is_empty() {
        return;
    }

    query_builder.push(" AND (last_seen_at < ");
    query_builder.push_bind(cursor.last_seen_at.clone());
    query_builder.push(" OR (last_seen_at = ");
    query_builder.push_bind(cursor.last_seen_at.clone());
    query_builder.push(" AND id < ");
    query_builder.push_bind(cursor.id.clone());
    query_builder.push("))");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_service::surface::SurfaceAssetColumnFilter;
    use crate::database_service::surface_inventory::SurfaceInventoryCursor;

    #[test]
    fn column_filter_targets_visible_typed_column() {
        let filter = SurfaceAssetFilter {
            asset_type: Some("web".to_string()),
            column_filters: Some(vec![SurfaceAssetColumnFilter {
                key: "canonical_url".to_string(),
                operator: Some("contains".to_string()),
                value: "example.com".to_string(),
            }]),
            ..Default::default()
        };
        let mut query_builder =
            QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE 1=1");

        push_surface_asset_filters(&mut query_builder, &filter);

        let sql = query_builder.sql();
        assert!(sql.contains("surface_assets.asset_type = ?"));
        assert!(sql.contains("surface_web_assets.asset_id = surface_assets.id"));
        assert!(sql.contains("LOWER(COALESCE(surface_web_assets.canonical_url, '')) LIKE ?"));
    }

    #[test]
    fn invalid_column_filter_cannot_broaden_scope() {
        let filter = SurfaceAssetFilter {
            asset_type: Some("web".to_string()),
            column_filters: Some(vec![SurfaceAssetColumnFilter {
                key: "not_a_column".to_string(),
                operator: Some("contains".to_string()),
                value: "example.com".to_string(),
            }]),
            ..Default::default()
        };
        let mut query_builder =
            QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE 1=1");

        push_surface_asset_filters(&mut query_builder, &filter);

        assert!(query_builder.sql().contains("AND 1=0"));
    }

    #[test]
    fn cursor_filter_uses_keyset_boundary() {
        let cursor = SurfaceInventoryCursor {
            last_seen_at: "2026-05-13T10:00:00Z".to_string(),
            id: "asset-100".to_string(),
        };
        let mut query_builder =
            QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE 1=1");

        push_surface_asset_cursor(&mut query_builder, Some(&cursor));

        let sql = query_builder.sql();
        assert!(sql.contains("last_seen_at < ?"));
        assert!(sql.contains("last_seen_at = ?"));
        assert!(sql.contains("id < ?"));
    }
}
