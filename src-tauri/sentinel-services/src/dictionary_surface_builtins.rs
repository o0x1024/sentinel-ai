use sentinel_core::models::dictionary::DictionaryWordInput;

pub fn builtin_service_fingerprint_entries() -> Vec<DictionaryWordInput> {
    fn service_rule_metadata(
        name: &str,
        product: &str,
        vendor: &str,
        asset_category: &str,
        asset_family: &str,
        protocol: Option<&str>,
        probe_name: Option<&str>,
        ports: &[u16],
        ssl_ports: &[u16],
        softmatch: bool,
        matchers: serde_json::Value,
        service: Option<&str>,
    ) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "service": service,
            "product": product,
            "vendor": vendor,
            "asset_category": asset_category,
            "asset_family": asset_family,
            "priority": 100,
            "protocol": protocol,
            "probeName": probe_name,
            "ports": ports,
            "sslPorts": ssl_ports,
            "operator": "or",
            "softmatch": softmatch,
            "matchers": matchers,
            "confidence": if softmatch { 0.78 } else { 0.9 }
        })
    }

    vec![
        (
            "nginx",
            "web_server",
            service_rule_metadata(
                "Nginx",
                "Nginx",
                "NGINX",
                "web_server",
                "http_server",
                None,
                Some("http_head"),
                &[80, 81, 8000, 8080, 8081, 8888, 9000],
                &[443, 8443, 9443],
                false,
                serde_json::json!([
                    {"part": "header", "type": "regex", "value": "nginx(?:/([0-9.]+))?"},
                    {"part": "product", "type": "contains", "value": "nginx"}
                ]),
                None,
            ),
        ),
        (
            "apache_http_server",
            "web_server",
            service_rule_metadata(
                "Apache HTTP Server",
                "Apache",
                "Apache Software Foundation",
                "web_server",
                "http_server",
                None,
                Some("http_head"),
                &[80, 81, 8000, 8080, 8081, 8888, 9000],
                &[443, 8443, 9443],
                false,
                serde_json::json!([
                    {"part": "header", "type": "regex", "value": "apache(?:/([0-9.]+))?"},
                    {"part": "product", "type": "contains", "value": "apache"}
                ]),
                None,
            ),
        ),
        (
            "microsoft_iis",
            "web_server",
            service_rule_metadata(
                "Microsoft IIS",
                "Microsoft-IIS",
                "Microsoft",
                "web_server",
                "http_server",
                None,
                Some("http_head"),
                &[80, 81, 8000, 8080, 8081, 8888, 9000],
                &[443, 8443, 9443],
                false,
                serde_json::json!([
                    {"part": "header", "type": "regex", "value": "microsoft-iis(?:/([0-9.]+))?"},
                    {"part": "product", "type": "contains", "value": "iis"}
                ]),
                None,
            ),
        ),
        (
            "openssh",
            "remote_access",
            service_rule_metadata(
                "OpenSSH",
                "OpenSSH",
                "OpenSSH",
                "remote_access",
                "ssh",
                Some("tcp"),
                Some("tcp_banner"),
                &[22],
                &[],
                false,
                serde_json::json!([
                    {"part": "banner", "type": "regex", "value": "openssh[-_ ]([0-9.p]+)"},
                    {"part": "product", "type": "contains", "value": "openssh"},
                    {"part": "service", "type": "contains", "value": "ssh"}
                ]),
                Some("ssh"),
            ),
        ),
        (
            "redis",
            "cache",
            service_rule_metadata(
                "Redis",
                "Redis",
                "Redis",
                "cache",
                "in_memory_store",
                Some("tcp"),
                Some("tcp_banner"),
                &[6379],
                &[],
                false,
                serde_json::json!([
                    {"part": "banner", "type": "contains", "value": "redis"},
                    {"part": "product", "type": "contains", "value": "redis"},
                    {"part": "service", "type": "contains", "value": "redis"}
                ]),
                Some("redis"),
            ),
        ),
        (
            "mysql",
            "database",
            service_rule_metadata(
                "MySQL",
                "MySQL",
                "Oracle",
                "database",
                "relational_database",
                Some("tcp"),
                Some("tcp_banner"),
                &[3306],
                &[],
                true,
                serde_json::json!([
                    {"part": "banner", "type": "regex", "value": "(mysql|mariadb)"},
                    {"part": "product", "type": "contains", "value": "mysql"},
                    {"part": "service", "type": "contains", "value": "mysql"}
                ]),
                Some("mysql"),
            ),
        ),
        (
            "postgresql",
            "database",
            service_rule_metadata(
                "PostgreSQL",
                "PostgreSQL",
                "PostgreSQL Global Development Group",
                "database",
                "relational_database",
                Some("tcp"),
                Some("tcp_banner"),
                &[5432],
                &[],
                true,
                serde_json::json!([
                    {"part": "banner", "type": "contains", "value": "postgres"},
                    {"part": "service", "type": "contains", "value": "postgres"}
                ]),
                Some("postgresql"),
            ),
        ),
        (
            "elasticsearch",
            "search",
            service_rule_metadata(
                "Elasticsearch",
                "Elasticsearch",
                "Elastic",
                "search",
                "search_engine",
                None,
                Some("http_head"),
                &[9200],
                &[],
                true,
                serde_json::json!([
                    {"part": "banner", "type": "contains", "value": "elasticsearch"},
                    {"part": "service", "type": "contains", "value": "elasticsearch"}
                ]),
                Some("elasticsearch"),
            ),
        ),
    ]
    .into_iter()
    .map(|(word, asset_category, metadata)| DictionaryWordInput {
        word: word.to_string(),
        weight: Some(8.0),
        category: Some(asset_category.to_string()),
        metadata: Some(metadata),
    })
    .collect()
}

pub fn builtin_favicon_fingerprint_entries() -> Vec<DictionaryWordInput> {
    vec![
        (
            "grafana_favicon",
            "Grafana",
            "Grafana",
            "Grafana Labs",
            "observability",
            "monitoring",
            serde_json::json!([
                {"part": "url", "type": "contains", "value": "/public/img/fav32.png"},
                {"part": "url", "type": "contains", "value": "/public/img/grafana_icon.svg"}
            ]),
        ),
        (
            "kibana_favicon",
            "Kibana",
            "Kibana",
            "Elastic",
            "observability",
            "log_analytics",
            serde_json::json!([
                {"part": "url", "type": "contains", "value": "/ui/favicons/"},
                {"part": "url", "type": "contains", "value": "/plugins/kibana/assets/"}
            ]),
        ),
        (
            "gitlab_favicon",
            "GitLab",
            "GitLab",
            "GitLab",
            "devops",
            "source_control",
            serde_json::json!([
                {"part": "url", "type": "regex", "value": "/assets/favicon[^/]*\\.(png|ico|svg)"},
                {"part": "url", "type": "contains", "value": "/assets/gitlab-logo"}
            ]),
        ),
        (
            "harbor_favicon",
            "Harbor",
            "Harbor",
            "VMware",
            "devops",
            "registry",
            serde_json::json!([
                {"part": "url", "type": "contains", "value": "/harbor.ico"},
                {"part": "url", "type": "contains", "value": "/src/images/harbor-logo"}
            ]),
        ),
        (
            "jenkins_favicon",
            "Jenkins",
            "Jenkins",
            "Jenkins",
            "ci_cd",
            "devops",
            serde_json::json!([
                {"part": "url", "type": "regex", "value": "/static/.*/images/(headless|jenkins)\\.(png|svg)"},
                {"part": "url", "type": "contains", "value": "/images/headless.png"}
            ]),
        ),
    ]
    .into_iter()
    .map(
        |(word, name, product, vendor, asset_category, asset_family, matchers)| {
            DictionaryWordInput {
                word: word.to_string(),
                weight: Some(8.0),
                category: Some(asset_category.to_string()),
                metadata: Some(serde_json::json!({
                    "name": name,
                    "product": product,
                    "vendor": vendor,
                    "asset_category": asset_category,
                    "asset_family": asset_family,
                    "priority": 95,
                    "operator": "or",
                    "matchers": matchers,
                    "confidence": 0.85
                })),
            }
        },
    )
    .collect()
}
