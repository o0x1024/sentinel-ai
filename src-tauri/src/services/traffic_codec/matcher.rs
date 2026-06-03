// Glob-based rule matcher
use super::types::{CodecMatchRule, CodecRequestMeta, TrafficCodecRule};

pub fn match_rules<'a>(
    rules: &'a [TrafficCodecRule],
    meta: &CodecRequestMeta,
) -> Vec<&'a TrafficCodecRule> {
    rules
        .iter()
        .filter(|rule| rule.enabled && matches_rule(&rule.match_rule, meta))
        .collect()
}

fn matches_rule(rule: &CodecMatchRule, meta: &CodecRequestMeta) -> bool {
    // All conditions must pass (AND logic). Empty list = match all.
    let host_ok =
        rule.hosts.is_empty() || rule.hosts.iter().any(|p| glob_match(p, &meta.host));
    let path_ok =
        rule.paths.is_empty() || rule.paths.iter().any(|p| glob_match(p, &meta.path));
    let method_ok = rule.methods.is_empty()
        || rule
            .methods
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&meta.method));
    let ct_ok = rule.content_types.is_empty()
        || rule
            .content_types
            .iter()
            .any(|ct| meta.content_type.contains(ct));
    host_ok && path_ok && method_ok && ct_ok
}

/// Simple glob matching supporting: *, **, *.suffix, prefix/**, prefix/*
pub fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern == "*" || pattern == "**" {
        return true;
    }

    glob_match_inner(
        &pattern.to_ascii_lowercase(),
        &value.to_ascii_lowercase(),
    )
}

fn glob_match_inner(pattern: &str, value: &str) -> bool {
    let mut pattern_bytes = pattern.as_bytes();
    let mut value_bytes = value.as_bytes();

    while !pattern_bytes.is_empty() {
        match pattern_bytes[0] {
            b'*' => {
                if pattern_bytes.len() > 1 && pattern_bytes[1] == b'*' {
                    pattern_bytes = &pattern_bytes[2..];
                    if pattern_bytes.is_empty() {
                        return true;
                    }
                    for i in 0..=value_bytes.len() {
                        let suffix = std::str::from_utf8(&value_bytes[i..]).unwrap_or("");
                        let rest = std::str::from_utf8(pattern_bytes).unwrap_or("");
                        if glob_match_inner(rest, suffix) {
                            return true;
                        }
                    }
                    return false;
                }

                pattern_bytes = &pattern_bytes[1..];
                if pattern_bytes.is_empty() {
                    return !value_bytes.contains(&b'/') && !value_bytes.contains(&b'.');
                }

                let next = pattern_bytes[0];
                while !value_bytes.is_empty() {
                    if value_bytes[0] == next {
                        let suffix = std::str::from_utf8(value_bytes).unwrap_or("");
                        let rest = std::str::from_utf8(pattern_bytes).unwrap_or("");
                        if glob_match_inner(rest, suffix) {
                            return true;
                        }
                    }
                    if value_bytes[0] == b'/' || value_bytes[0] == b'.' {
                        return false;
                    }
                    value_bytes = &value_bytes[1..];
                }
                return false;
            }
            byte => {
                if value_bytes.is_empty() || value_bytes[0] != byte {
                    return false;
                }
                pattern_bytes = &pattern_bytes[1..];
                value_bytes = &value_bytes[1..];
            }
        }
    }

    value_bytes.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_star_matches_all() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("**", "anything/deep"));
    }

    #[test]
    fn glob_host_subdomain() {
        assert!(glob_match("*.example.com", "foo.example.com"));
        assert!(glob_match("*.example.com", "bar.example.com"));
        assert!(!glob_match("*.example.com", "foo.bar.example.com"));
        assert!(glob_match("example.com", "example.com"));
        assert!(!glob_match("example.com", "foo.example.com"));
    }

    #[test]
    fn glob_path_prefix() {
        assert!(glob_match("/api/**", "/api/v1/users"));
        assert!(glob_match("/api/**", "/api/anything"));
        assert!(!glob_match("/api/**", "/other/api/v1"));
    }

    #[test]
    fn glob_path_single_segment() {
        assert!(glob_match("/api/*", "/api/foo"));
        assert!(!glob_match("/api/*", "/api/foo/bar"));
    }

    #[test]
    fn glob_inline_wildcard() {
        assert!(glob_match("api-*.com", "api-test.com"));
        assert!(!glob_match("api-*.com", "api-test.org"));
    }

    #[test]
    fn glob_is_case_insensitive() {
        assert!(glob_match("*.Example.COM", "foo.example.com"));
        assert!(glob_match("/API/**", "/api/v1/users"));
    }
}
