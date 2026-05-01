use super::*;

#[test]
fn test_parse_severity() {
    assert!(matches!(parse_severity("critical"), Severity::Critical));
    assert!(matches!(parse_severity("HIGH"), Severity::High));
    assert!(matches!(parse_severity("medium"), Severity::Medium));
    assert!(matches!(parse_severity("low"), Severity::Low));
    assert!(matches!(parse_severity("info"), Severity::Info));
    assert!(matches!(parse_severity("unknown"), Severity::Medium));
}

#[test]
fn test_parse_confidence() {
    assert!(matches!(parse_confidence("HIGH"), Confidence::High));
    assert!(matches!(parse_confidence("medium"), Confidence::Medium));
    assert!(matches!(parse_confidence("low"), Confidence::Low));
    assert!(matches!(parse_confidence("unknown"), Confidence::Medium));
}

#[test]
fn test_build_active_probe_cooldown_key_uses_host_and_path() {
    assert_eq!(
        build_active_probe_cooldown_key("https://example.com:8443/api/items?id=1"),
        "example.com:8443/api/items"
    );
    assert_eq!(
        build_active_probe_cooldown_key("https://example.com"),
        "example.com/"
    );
    assert_eq!(build_active_probe_cooldown_key("not-a-url"), "global");
}
