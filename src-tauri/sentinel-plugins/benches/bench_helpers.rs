use sentinel_plugins::types::*;
use std::collections::HashMap;

pub fn simple_js_plugin() -> &'static str {
    r#"
function scan_transaction(transaction) {
    var url = transaction.request.url;
    if (url.indexOf("admin") !== -1) {
        Sentinel.emit({
            vuln_type: "info_disclosure",
            title: "Admin path detected",
            description: "URL contains admin path",
            severity: "info",
            confidence: "high"
        });
    }
}
"#
}

pub fn medium_js_plugin() -> &'static str {
    r#"
var SENSITIVE_HEADERS = ["authorization", "x-api-key", "cookie", "set-cookie"];
var SENSITIVE_PATTERNS = [
    /password[\"']?\s*[:=]\s*[\"'][^\"']+[\"']/gi,
    /api[_-]?key[\"']?\s*[:=]\s*[\"'][^\"']+[\"']/gi,
    /token[\"']?\s*[:=]\s*[\"'][^\"']+[\"']/gi,
    /secret[\"']?\s*[:=]\s*[\"'][^\"']+[\"']/gi,
    /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z]{2,}\b/gi,
    /\b(?:\d{4}[- ]?){3}\d{4}\b/g,
];

function checkHeaders(headers) {
    var findings = [];
    for (var key in headers) {
        if (SENSITIVE_HEADERS.indexOf(key.toLowerCase()) !== -1) {
            findings.push({
                vuln_type: "sensitive_header",
                title: "Sensitive header: " + key,
                description: "Response contains sensitive header",
                severity: "low",
                confidence: "high"
            });
        }
    }
    return findings;
}

function checkBody(body) {
    var findings = [];
    for (var i = 0; i < SENSITIVE_PATTERNS.length; i++) {
        var matches = body.match(SENSITIVE_PATTERNS[i]);
        if (matches && matches.length > 0) {
            findings.push({
                vuln_type: "sensitive_data",
                title: "Sensitive data pattern found",
                description: "Found " + matches.length + " matches",
                severity: "medium",
                confidence: "medium"
            });
        }
    }
    return findings;
}

function scan_transaction(transaction) {
    var url = transaction.request.url;

    if (transaction.response) {
        var headerFindings = checkHeaders(transaction.response.headers || {});
        for (var i = 0; i < headerFindings.length; i++) {
            Sentinel.emit(headerFindings[i]);
        }

        var body = transaction.response.body || "";
        if (body.length > 0) {
            var bodyFindings = checkBody(body);
            for (var j = 0; j < bodyFindings.length; j++) {
                Sentinel.emit(bodyFindings[j]);
            }
        }
    }

    if (url.indexOf(".env") !== -1 || url.indexOf("wp-config") !== -1) {
        Sentinel.emit({
            vuln_type: "sensitive_file",
            title: "Sensitive file access",
            description: "Access to potentially sensitive file: " + url,
            severity: "high",
            confidence: "medium"
        });
    }
}
"#
}

pub fn typescript_plugin() -> &'static str {
    r#"
declare const Sentinel: {
    emit: (finding: Record<string, unknown>) => void;
    resolve: (result: unknown) => void;
};

interface ScanTarget {
    url: string;
    method: string;
    headers: Record<string, string>;
}

interface Finding {
    vuln_type: string;
    title: string;
    description: string;
    severity: "info" | "low" | "medium" | "high" | "critical";
    confidence: "low" | "medium" | "high";
}

type SeverityLevel = "info" | "low" | "medium" | "high" | "critical";

const HEADER_CHECKS: Array<{ header: string; severity: SeverityLevel; message: string }> = [
    { header: "x-frame-options", severity: "medium", message: "Missing X-Frame-Options header" },
    { header: "content-security-policy", severity: "medium", message: "Missing CSP header" },
    { header: "strict-transport-security", severity: "low", message: "Missing HSTS header" },
    { header: "x-content-type-options", severity: "low", message: "Missing X-Content-Type-Options" },
];

function checkSecurityHeaders(headers: Record<string, string>): Finding[] {
    const results: Finding[] = [];
    for (const check of HEADER_CHECKS) {
        const hasHeader = Object.keys(headers).some(
            (key) => key.toLowerCase() === check.header
        );
        if (!hasHeader) {
            results.push({
                vuln_type: "missing_header",
                title: check.message,
                description: `The response is missing the ${check.header} security header`,
                severity: check.severity,
                confidence: "high",
            });
        }
    }
    return results;
}

function scan_transaction(transaction: { request: ScanTarget; response?: { headers: Record<string, string>; body: string; status_code: number } }) {
    if (!transaction.response) return;

    const headerFindings = checkSecurityHeaders(transaction.response.headers);
    for (const finding of headerFindings) {
        Sentinel.emit(finding);
    }

    if (transaction.response.status_code >= 500) {
        Sentinel.emit({
            vuln_type: "server_error",
            title: "Server Error Response",
            description: `Server returned ${transaction.response.status_code}`,
            severity: "info",
            confidence: "high",
        });
    }
}
"#
}

pub fn agent_plugin() -> &'static str {
    r#"
function get_input_schema() {
    return {
        type: "object",
        properties: {
            targets: { type: "array", items: { type: "string" } },
            timeout: { type: "number" },
        },
        required: ["targets"],
    };
}

function analyze(input) {
    if (!input.targets || !Array.isArray(input.targets)) {
        Sentinel.resolve({ success: false, error: "Invalid input" });
        return;
    }
    var results = [];
    for (var i = 0; i < input.targets.length; i++) {
        results.push({
            target: input.targets[i],
            status: "scanned",
            findings: 0,
        });
    }
    Sentinel.resolve({
        success: true,
        data: { scanned: results.length, results: results },
    });
}
"#
}

pub fn heavy_computation_plugin() -> &'static str {
    r#"
function scan_transaction(transaction) {
    var url = transaction.request.url;
    var body = "";
    if (transaction.response) {
        body = transaction.response.body || "";
    }

    var hash = 0;
    for (var i = 0; i < body.length; i++) {
        var chr = body.charCodeAt(i);
        hash = ((hash << 5) - hash) + chr;
        hash |= 0;
    }

    var tokens = body.split(/[\s<>"'=;:{}()\[\],./\\]+/);
    var uniqueTokens = {};
    var tokenCount = 0;
    for (var j = 0; j < tokens.length; j++) {
        if (tokens[j].length > 0 && !uniqueTokens[tokens[j]]) {
            uniqueTokens[tokens[j]] = true;
            tokenCount++;
        }
    }

    var patterns = [
        /SELECT\s+.*?\s+FROM/gi,
        /INSERT\s+INTO/gi,
        /UPDATE\s+.*?\s+SET/gi,
        /DELETE\s+FROM/gi,
        /<script[^>]*>/gi,
        /javascript:/gi,
        /on\w+\s*=/gi,
    ];
    var matchCount = 0;
    for (var k = 0; k < patterns.length; k++) {
        var m = body.match(patterns[k]);
        if (m) matchCount += m.length;
    }

    if (matchCount > 0) {
        Sentinel.emit({
            vuln_type: "pattern_match",
            title: "Suspicious patterns detected",
            description: "Found " + matchCount + " pattern matches in " + tokenCount + " tokens",
            severity: matchCount > 5 ? "high" : "medium",
            confidence: "medium"
        });
    }
}
"#
}

pub fn make_metadata(id: &str, main_category: PluginMainCategory) -> PluginMetadata {
    PluginMetadata {
        id: id.to_string(),
        name: id.to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category,
        category: PluginCategory::parse_for_main_category(main_category, "test").unwrap(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
        monitor_type: None,
        target_asset_types: vec![],
        input_mode: None,
        seed_bindings: vec![],
    }
}

pub fn make_simple_transaction() -> HttpTransaction {
    HttpTransaction {
        request: RequestContext {
            id: "bench-req-1".to_string(),
            method: "GET".to_string(),
            url: "https://example.com/api/admin/users".to_string(),
            http_version: Some("HTTP/1.1".to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("host".to_string(), "example.com".to_string());
                h.insert("user-agent".to_string(), "Mozilla/5.0".to_string());
                h.insert("accept".to_string(), "text/html".to_string());
                h
            },
            body: vec![],
            content_type: None,
            query_params: HashMap::new(),
            is_https: true,
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        },
        response: Some(ResponseContext {
            request_id: "bench-req-1".to_string(),
            status: 200,
            http_version: Some("HTTP/1.1".to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h.insert("server".to_string(), "nginx/1.18".to_string());
                h
            },
            body: r#"{"users":[{"id":1,"name":"admin","email":"admin@example.com"}]}"#
                .as_bytes()
                .to_vec(),
            content_type: Some("application/json".to_string()),
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        }),
    }
}

pub fn make_large_transaction() -> HttpTransaction {
    let large_body = (0..500)
        .map(|i| {
            format!(
                r#"{{"id":{},"name":"user_{}","email":"user{}@example.com","role":"viewer","password":"hunter2","api_key":"sk-{:032x}","token":"eyJ{:048x}"}}"#,
                i, i, i, i as u128, i as u128
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let body = format!(r#"{{"users":[{}],"total":500,"page":1}}"#, large_body);

    HttpTransaction {
        request: RequestContext {
            id: "bench-req-large".to_string(),
            method: "GET".to_string(),
            url: "https://example.com/api/v2/users?page=1&limit=500".to_string(),
            http_version: Some("HTTP/2".to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("host".to_string(), "example.com".to_string());
                h.insert("authorization".to_string(), "Bearer eyJhbGciOiJIUzI1NiJ9.test".to_string());
                h.insert("user-agent".to_string(), "Mozilla/5.0".to_string());
                h.insert("accept".to_string(), "application/json".to_string());
                h.insert("x-request-id".to_string(), "req-12345".to_string());
                h
            },
            body: vec![],
            content_type: None,
            query_params: {
                let mut q = HashMap::new();
                q.insert("page".to_string(), "1".to_string());
                q.insert("limit".to_string(), "500".to_string());
                q
            },
            is_https: true,
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        },
        response: Some(ResponseContext {
            request_id: "bench-req-large".to_string(),
            status: 200,
            http_version: Some("HTTP/2".to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h.insert("server".to_string(), "nginx/1.18".to_string());
                h.insert("x-ratelimit-remaining".to_string(), "99".to_string());
                h.insert("set-cookie".to_string(), "session=abc123; HttpOnly; Secure".to_string());
                h
            },
            body: body.into_bytes(),
            content_type: Some("application/json".to_string()),
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        }),
    }
}

pub fn make_small_json() -> serde_json::Value {
    serde_json::json!({
        "url": "https://example.com",
        "method": "GET",
        "status": 200
    })
}

pub fn make_medium_json() -> serde_json::Value {
    serde_json::json!({
        "request": {
            "id": "req-1",
            "method": "POST",
            "url": "https://api.example.com/users",
            "headers": {
                "content-type": "application/json",
                "authorization": "Bearer token123",
                "user-agent": "Mozilla/5.0"
            },
            "body": {"username": "test", "password": "secret123"}
        },
        "response": {
            "status_code": 201,
            "headers": {
                "content-type": "application/json",
                "x-request-id": "abc-123"
            },
            "body": {"id": 42, "username": "test", "created_at": "2024-01-01"}
        }
    })
}

pub fn make_large_json() -> serde_json::Value {
    let txn = make_large_transaction();
    serde_json::to_value(&txn).unwrap()
}
