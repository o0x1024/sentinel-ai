/**
 * 插件 Op 系统单元测试
 *
 * 测试范围：
 * 1. PluginContext 状态管理
 * 2. JsFinding → Finding 转换
 * 3. Severity/Confidence 解析
 */
use sentinel_plugins::{
    plugin_ops::{JsFinding, PluginContext},
    types::{Confidence, Finding, Severity},
};

#[test]
fn test_plugin_context_new() {
    let context = PluginContext::new();
    let findings = context.take_findings();
    assert_eq!(findings.len(), 0, "New context should have no findings");
}

#[test]
fn test_plugin_context_take_findings_clears_vec() {
    let context = PluginContext::new();

    let finding = Finding {
        id: "test-id".to_string(),
        plugin_id: "test-plugin".to_string(),
        vuln_type: "xss".to_string(),
        severity: Severity::Medium,
        confidence: Confidence::High,
        title: "Test XSS".to_string(),
        description: "Test description".to_string(),
        evidence: "Test evidence".to_string(),
        location: "param:comment".to_string(),
        url: "http://example.com".to_string(),
        method: "POST".to_string(),
        cwe: None,
        owasp: None,
        remediation: None,
        created_at: chrono::Utc::now(),
        request_headers: None,
        request_body: None,
        response_status: None,
        response_headers: None,
        response_body: None,
    };

    context.findings.lock().unwrap().push(finding);

    // 第一次 take
    let findings1 = context.take_findings();
    assert_eq!(findings1.len(), 1);

    // 第二次 take 应该为空
    let findings2 = context.take_findings();
    assert_eq!(findings2.len(), 0, "take_findings should clear the vector");
}

#[test]
fn test_js_finding_to_finding_conversion() {
    let js_finding = JsFinding {
        vuln_type: "sqli".to_string(),
        severity: "high".to_string(),
        confidence: "medium".to_string(),
        url: "http://example.com/login".to_string(),
        method: "POST".to_string(),
        param_name: "username".to_string(),
        param_value: "admin' --".to_string(),
        evidence: "SQL comment detected".to_string(),
        description: "SQL injection in login".to_string(),
        title: "SQL Injection".to_string(),
        request: None,
        response: None,
        cwe: String::new(),
        owasp: String::new(),
        remediation: String::new(),
    };

    let finding: Finding = js_finding.into();

    assert_eq!(finding.vuln_type, "sqli");
    assert_eq!(finding.severity, Severity::High);
    assert_eq!(finding.confidence, Confidence::Medium);
    assert_eq!(finding.url, "http://example.com/login");
    assert_eq!(finding.method, "POST");
    assert!(finding.location.contains("username"));
    assert!(!finding.id.is_empty(), "ID should be auto-generated");
}

#[test]
fn test_severity_conversion() {
    let test_cases = vec![
        ("critical", Severity::Critical),
        ("high", Severity::High),
        ("medium", Severity::Medium),
        ("low", Severity::Low),
        ("info", Severity::Info),
        ("unknown", Severity::Medium), // 未知值默认 Medium
    ];

    for (input, expected) in test_cases {
        let js_finding = JsFinding {
            vuln_type: "test".to_string(),
            severity: input.to_string(),
            confidence: "high".to_string(),
            url: "http://test.com".to_string(),
            method: "GET".to_string(),
            param_name: "".to_string(),
            param_value: "".to_string(),
            evidence: "".to_string(),
            description: "".to_string(),
            title: "".to_string(),
            request: None,
            response: None,
            cwe: "".to_string(),
            owasp: "".to_string(),
            remediation: "".to_string(),
        };

        let finding: Finding = js_finding.into();
        assert_eq!(
            finding.severity, expected,
            "Severity mismatch for '{}'",
            input
        );
    }
}

#[test]
fn test_confidence_conversion() {
    let test_cases = vec![
        ("high", Confidence::High),
        ("medium", Confidence::Medium),
        ("low", Confidence::Low),
        ("unknown", Confidence::Medium), // 未知值默认 Medium
    ];

    for (input, expected) in test_cases {
        let js_finding = JsFinding {
            vuln_type: "test".to_string(),
            severity: "medium".to_string(),
            confidence: input.to_string(),
            url: "http://test.com".to_string(),
            method: "GET".to_string(),
            param_name: "".to_string(),
            param_value: "".to_string(),
            evidence: "".to_string(),
            description: "".to_string(),
            title: "".to_string(),
            request: None,
            response: None,
            cwe: "".to_string(),
            owasp: "".to_string(),
            remediation: "".to_string(),
        };

        let finding: Finding = js_finding.into();
        assert_eq!(
            finding.confidence, expected,
            "Confidence mismatch for '{}'",
            input
        );
    }
}

#[test]
fn test_finding_id_is_unique() {
    let js_finding1 = JsFinding {
        vuln_type: "sqli".to_string(),
        severity: "high".to_string(),
        confidence: "medium".to_string(),
        url: "http://example.com".to_string(),
        method: "GET".to_string(),
        param_name: "id".to_string(),
        param_value: "1".to_string(),
        evidence: "test".to_string(),
        description: "test".to_string(),
        title: "".to_string(),
        request: None,
        response: None,
        cwe: "".to_string(),
        owasp: "".to_string(),
        remediation: "".to_string(),
    };

    let js_finding2 = js_finding1.clone();

    let finding1: Finding = js_finding1.into();
    let finding2: Finding = js_finding2.into();

    assert_ne!(
        finding1.id, finding2.id,
        "Each finding should have a unique ID"
    );
}

#[test]
fn test_location_from_param_name() {
    let js_finding = JsFinding {
        vuln_type: "test".to_string(),
        severity: "medium".to_string(),
        confidence: "high".to_string(),
        url: "http://example.com".to_string(),
        method: "GET".to_string(),
        param_name: "test_param".to_string(),
        param_value: "test_value".to_string(),
        evidence: "".to_string(),
        description: "".to_string(),
        title: "".to_string(),
        request: None,
        response: None,
        cwe: "".to_string(),
        owasp: "".to_string(),
        remediation: "".to_string(),
    };

    let finding: Finding = js_finding.into();
    assert_eq!(finding.location, "param:test_param");
}

#[test]
fn test_title_fallback_logic() {
    // 1. 显式指定 title
    let js_finding = JsFinding {
        title: "Custom Title".to_string(),
        ..JsFinding::default()
    };
    let finding: Finding = js_finding.into();
    assert_eq!(finding.title, "Custom Title");

    // 2. 从描述首行获取
    let js_finding = JsFinding {
        description: "First line\nSecond line".to_string(),
        ..JsFinding::default()
    };
    let finding: Finding = js_finding.into();
    assert_eq!(finding.title, "First line");

    // 3. 从漏洞类型构造
    let js_finding = JsFinding {
        vuln_type: "XSS".to_string(),
        ..JsFinding::default()
    };
    let finding: Finding = js_finding.into();
    assert_eq!(finding.title, "XSS detected");
}
