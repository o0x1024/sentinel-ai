use chrono::Utc;
use sentinel_db::{DatabaseService, ProgramScopeRow};
use serde_json::json;
use uuid::Uuid;

const COMMON_TWO_LABEL_PUBLIC_SUFFIXES: &[&str] = &[
    "ac.jp",
    "ac.uk",
    "co.jp",
    "co.kr",
    "co.nz",
    "co.uk",
    "com.au",
    "com.br",
    "com.cn",
    "com.hk",
    "com.mx",
    "com.sg",
    "edu.cn",
    "edu.hk",
    "gov.cn",
    "gov.uk",
    "net.au",
    "net.cn",
    "org.au",
    "org.cn",
    "org.hk",
    "org.uk",
];

pub fn infer_root_domain(domain: &str) -> Option<String> {
    let parts: Vec<&str> = domain.split('.').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let suffix = format!(
        "{}.{}",
        parts[parts.len().saturating_sub(2)],
        parts[parts.len().saturating_sub(1)]
    );

    if parts.len() >= 3
        && COMMON_TWO_LABEL_PUBLIC_SUFFIXES
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&suffix))
    {
        return Some(format!(
            "{}.{}.{}",
            parts[parts.len().saturating_sub(3)],
            parts[parts.len().saturating_sub(2)],
            parts[parts.len().saturating_sub(1)]
        ));
    }

    Some(suffix)
}

fn normalize_scope_domain_target(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .trim_start_matches("*.")
        .trim_matches('.')
        .to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn scope_covers_root_domain(scope: &ProgramScopeRow, root_domain: &str) -> bool {
    if scope.scope_type != "in_scope" {
        return false;
    }

    let Some(scope_target) = normalize_scope_domain_target(&scope.target) else {
        return false;
    };

    match scope.target_type.as_str() {
        "domain" | "wildcard_domain" => scope_target == root_domain,
        _ => false,
    }
}

pub async fn create_missing_in_scope_domains(
    db_service: &DatabaseService,
    program_id: &str,
    domains: &[String],
    description: &str,
    reason: &str,
) -> Result<usize, String> {
    let mut desired_root_domains = std::collections::HashSet::new();
    for domain in domains {
        if let Some(root_domain) =
            infer_root_domain(domain).and_then(|value| normalize_scope_domain_target(&value))
        {
            desired_root_domains.insert(root_domain);
        }
    }

    if desired_root_domains.is_empty() {
        return Ok(0);
    }

    let existing_scopes = db_service
        .list_program_scopes(Some(program_id), Some("in_scope"))
        .await
        .map_err(|e| e.to_string())?;

    desired_root_domains.retain(|root_domain| {
        !existing_scopes
            .iter()
            .any(|scope| scope_covers_root_domain(scope, root_domain))
    });

    if desired_root_domains.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let mut created = 0usize;

    for root_domain in desired_root_domains {
        let scope = ProgramScopeRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            scope_type: "in_scope".to_string(),
            target_type: "domain".to_string(),
            target: root_domain,
            description: Some(description.to_string()),
            allowed_tests_json: None,
            instructions_json: None,
            requires_auth: false,
            test_accounts_json: None,
            asset_count: 0,
            finding_count: 0,
            priority: 0.0,
            metadata_json: Some(
                json!({
                    "source": "surface_scope_sync",
                    "reason": reason,
                })
                .to_string(),
            ),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        db_service
            .create_program_scope(&scope)
            .await
            .map_err(|e| e.to_string())?;
        created += 1;
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::infer_root_domain;

    #[test]
    fn infer_root_domain_for_standard_domain() {
        assert_eq!(infer_root_domain("api.example.com").as_deref(), Some("example.com"));
    }

    #[test]
    fn infer_root_domain_for_common_multi_label_suffix() {
        assert_eq!(
            infer_root_domain("api.example.co.uk").as_deref(),
            Some("example.co.uk")
        );
        assert_eq!(
            infer_root_domain("edge.example.com.cn").as_deref(),
            Some("example.com.cn")
        );
    }

    #[test]
    fn infer_root_domain_for_bare_registrable_domain() {
        assert_eq!(infer_root_domain("example.co.uk").as_deref(), Some("example.co.uk"));
    }
}
