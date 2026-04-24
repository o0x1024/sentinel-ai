use std::time::Duration;

const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 30 * 60;
const SUBDOMAIN_BRUTE_TIMEOUT_SECS: u64 = 4 * 60 * 60;

pub fn resolve_tool_timeout(tool_name: &str) -> Duration {
    match tool_name {
        "subdomain_brute" => Duration::from_secs(SUBDOMAIN_BRUTE_TIMEOUT_SECS),
        _ => Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdomain_brute_uses_extended_timeout_budget() {
        assert_eq!(
            resolve_tool_timeout("subdomain_brute"),
            Duration::from_secs(SUBDOMAIN_BRUTE_TIMEOUT_SECS)
        );
    }

    #[test]
    fn other_tools_use_default_timeout_budget() {
        assert_eq!(
            resolve_tool_timeout("http_request"),
            Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS)
        );
    }
}
