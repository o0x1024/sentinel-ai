//! Security rules — source/sink/sanitizer definitions per language and vulnerability class.
//!
//! These rules drive the graph-based taint analysis engine.

use serde::{Deserialize, Serialize};

// ── Rule types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    pub id: String,
    pub name: String,
    pub cwe: String,
    pub severity: Severity,
    pub description: String,
    pub sources: Vec<PatternSpec>,
    pub sinks: Vec<PatternSpec>,
    pub sanitizers: Vec<PatternSpec>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSpec {
    /// Name pattern to match against function/method/variable names
    pub name_pattern: String,
    /// Optional: only match if the call has specific argument patterns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg_pattern: Option<String>,
    /// Languages this pattern applies to (empty = all languages)
    #[serde(default)]
    pub languages: Vec<String>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PatternSpec {
    pub fn new(name: &str) -> Self {
        Self {
            name_pattern: name.to_string(),
            arg_pattern: None,
            languages: Vec::new(),
            description: None,
        }
    }

    pub fn lang(mut self, lang: &str) -> Self {
        self.languages.push(lang.to_string());
        self
    }

    pub fn langs(mut self, langs: &[&str]) -> Self {
        self.languages.extend(langs.iter().map(|s| s.to_string()));
        self
    }

    pub fn desc(mut self, d: &str) -> Self {
        self.description = Some(d.to_string());
        self
    }

    /// Check if this pattern matches a given symbol name.
    pub fn matches(&self, symbol: &str) -> bool {
        let pattern = &self.name_pattern;
        if pattern.contains('*') {
            // Simple glob: foo.* matches foo.bar, *.execute matches db.execute
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                if parts[0].is_empty() {
                    return symbol.ends_with(parts[1]);
                }
                if parts[1].is_empty() {
                    return symbol.starts_with(parts[0]);
                }
                return symbol.starts_with(parts[0]) && symbol.ends_with(parts[1]);
            }
        }
        // Substring match (case-insensitive)
        symbol.to_lowercase().contains(&pattern.to_lowercase())
            || symbol == *pattern
    }

    /// Check if this pattern applies to a given language.
    pub fn applies_to_language(&self, language: &str) -> bool {
        self.languages.is_empty() || self.languages.iter().any(|l| l == language)
    }
}

// ── Built-in rule sets ──────────────────────────────────────────────────────

/// Get all built-in security rules.
pub fn all_rules() -> Vec<SecurityRule> {
    vec![
        sql_injection_rule(),
        xss_rule(),
        command_injection_rule(),
        path_traversal_rule(),
        ssrf_rule(),
        deserialization_rule(),
        ldap_injection_rule(),
        xxe_rule(),
        open_redirect_rule(),
        log_injection_rule(),
        hardcoded_secrets_rule(),
    ]
}

/// Get rules filtered by IDs.
pub fn rules_by_ids(ids: &[String]) -> Vec<SecurityRule> {
    let all = all_rules();
    if ids.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|r| ids.iter().any(|id| r.id == *id))
        .collect()
}

/// Get available rule IDs and names.
pub fn list_rule_summaries() -> Vec<(String, String, String, String)> {
    all_rules()
        .into_iter()
        .map(|r| (r.id, r.name, r.cwe, r.severity.label().to_string()))
        .collect()
}

// ── Individual rules ────────────────────────────────────────────────────────

fn sql_injection_rule() -> SecurityRule {
    SecurityRule {
        id: "sql_injection".to_string(),
        name: "SQL Injection".to_string(),
        cwe: "CWE-89".to_string(),
        severity: Severity::Critical,
        description: "User input flows into SQL query without parameterization".to_string(),
        sources: vec![
            // JavaScript/Node.js
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("ctx.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("ctx.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("ctx.request").langs(&["javascript", "typescript"]),
            // Java/Spring
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("getParameterValues").langs(&["java"]),
            PatternSpec::new("getHeader").langs(&["java"]),
            PatternSpec::new("getCookies").langs(&["java"]),
            PatternSpec::new("getInputStream").langs(&["java"]),
            PatternSpec::new("@RequestParam").langs(&["java"]),
            PatternSpec::new("@PathVariable").langs(&["java"]),
            PatternSpec::new("@RequestBody").langs(&["java"]),
            // Python/Flask/Django
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("request.form").langs(&["python"]),
            PatternSpec::new("request.data").langs(&["python"]),
            PatternSpec::new("request.json").langs(&["python"]),
            PatternSpec::new("request.GET").langs(&["python"]),
            PatternSpec::new("request.POST").langs(&["python"]),
            // PHP
            PatternSpec::new("$_GET").langs(&["php"]),
            PatternSpec::new("$_POST").langs(&["php"]),
            PatternSpec::new("$_REQUEST").langs(&["php"]),
            PatternSpec::new("$_COOKIE").langs(&["php"]),
            // Go
            PatternSpec::new("r.URL.Query").langs(&["go"]),
            PatternSpec::new("r.FormValue").langs(&["go"]),
            PatternSpec::new("r.PostFormValue").langs(&["go"]),
            PatternSpec::new("c.Query").langs(&["go"]),
            PatternSpec::new("c.Param").langs(&["go"]),
            // Ruby/Rails
            PatternSpec::new("params").langs(&["ruby"]),
        ],
        sinks: vec![
            // Generic SQL sinks
            PatternSpec::new("execute").desc("SQL execute"),
            PatternSpec::new("query").desc("SQL query"),
            PatternSpec::new("raw").desc("Raw SQL query"),
            PatternSpec::new("rawQuery").desc("Raw SQL query"),
            PatternSpec::new("exec").desc("SQL exec"),
            // JavaScript
            PatternSpec::new("sequelize.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("knex.raw").langs(&["javascript", "typescript"]),
            PatternSpec::new("pool.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("connection.query").langs(&["javascript", "typescript"]),
            // Java
            PatternSpec::new("createQuery").langs(&["java"]),
            PatternSpec::new("createNativeQuery").langs(&["java"]),
            PatternSpec::new("createSQLQuery").langs(&["java"]),
            PatternSpec::new("prepareStatement").langs(&["java"]).desc("May be safe if parameterized"),
            PatternSpec::new("executeQuery").langs(&["java"]),
            PatternSpec::new("executeUpdate").langs(&["java"]),
            PatternSpec::new("jdbcTemplate.query").langs(&["java"]),
            // Python
            PatternSpec::new("cursor.execute").langs(&["python"]),
            PatternSpec::new("session.execute").langs(&["python"]),
            PatternSpec::new("engine.execute").langs(&["python"]),
            PatternSpec::new("text(").langs(&["python"]).desc("SQLAlchemy text()"),
            // PHP
            PatternSpec::new("mysqli_query").langs(&["php"]),
            PatternSpec::new("pg_query").langs(&["php"]),
            PatternSpec::new("->query").langs(&["php"]),
            // Go
            PatternSpec::new("db.Query").langs(&["go"]),
            PatternSpec::new("db.Exec").langs(&["go"]),
            PatternSpec::new("db.QueryRow").langs(&["go"]),
            // Ruby
            PatternSpec::new("find_by_sql").langs(&["ruby"]),
            PatternSpec::new("where(").langs(&["ruby"]),
            PatternSpec::new("execute(").langs(&["ruby"]),
        ],
        sanitizers: vec![
            PatternSpec::new("parameterize"),
            PatternSpec::new("prepare"),
            PatternSpec::new("bind"),
            PatternSpec::new("escape"),
            PatternSpec::new("sanitize"),
            PatternSpec::new("placeholder"),
        ],
    }
}

fn xss_rule() -> SecurityRule {
    SecurityRule {
        id: "xss".to_string(),
        name: "Cross-Site Scripting (XSS)".to_string(),
        cwe: "CWE-79".to_string(),
        severity: Severity::High,
        description: "User input rendered in HTML without proper encoding".to_string(),
        sources: vec![
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("request.form").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
            PatternSpec::new("$_POST").langs(&["php"]),
            PatternSpec::new("params").langs(&["ruby"]),
        ],
        sinks: vec![
            PatternSpec::new("innerHTML"),
            PatternSpec::new("outerHTML"),
            PatternSpec::new("document.write"),
            PatternSpec::new("document.writeln"),
            PatternSpec::new("dangerouslySetInnerHTML"),
            PatternSpec::new("v-html"),
            PatternSpec::new("res.send").langs(&["javascript", "typescript"]),
            PatternSpec::new("res.write").langs(&["javascript", "typescript"]),
            PatternSpec::new("render_template_string").langs(&["python"]),
            PatternSpec::new("Markup(").langs(&["python"]),
            PatternSpec::new("mark_safe").langs(&["python"]),
            PatternSpec::new("echo").langs(&["php"]),
            PatternSpec::new("print_r").langs(&["php"]),
            PatternSpec::new("raw(").langs(&["ruby"]),
            PatternSpec::new("html_safe").langs(&["ruby"]),
        ],
        sanitizers: vec![
            PatternSpec::new("escape"),
            PatternSpec::new("encode"),
            PatternSpec::new("sanitize"),
            PatternSpec::new("htmlspecialchars"),
            PatternSpec::new("encodeURIComponent"),
            PatternSpec::new("DOMPurify"),
            PatternSpec::new("bleach"),
        ],
    }
}

fn command_injection_rule() -> SecurityRule {
    SecurityRule {
        id: "command_injection".to_string(),
        name: "OS Command Injection".to_string(),
        cwe: "CWE-78".to_string(),
        severity: Severity::Critical,
        description: "User input used in OS command execution".to_string(),
        sources: vec![
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("request.form").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
            PatternSpec::new("$_POST").langs(&["php"]),
            PatternSpec::new("params").langs(&["ruby"]),
            PatternSpec::new("argv").desc("Command line arguments"),
            PatternSpec::new("env::var").langs(&["rust"]),
            PatternSpec::new("os.Getenv").langs(&["go"]),
        ],
        sinks: vec![
            PatternSpec::new("exec(").desc("Child process exec"),
            PatternSpec::new("execSync"),
            PatternSpec::new("spawn"),
            PatternSpec::new("child_process"),
            PatternSpec::new("system("),
            PatternSpec::new("popen"),
            PatternSpec::new("subprocess").langs(&["python"]),
            PatternSpec::new("os.system").langs(&["python"]),
            PatternSpec::new("os.popen").langs(&["python"]),
            PatternSpec::new("Runtime.exec").langs(&["java"]),
            PatternSpec::new("ProcessBuilder").langs(&["java"]),
            PatternSpec::new("shell_exec").langs(&["php"]),
            PatternSpec::new("passthru").langs(&["php"]),
            PatternSpec::new("proc_open").langs(&["php"]),
            PatternSpec::new("Command::new").langs(&["rust"]),
            PatternSpec::new("exec.Command").langs(&["go"]),
        ],
        sanitizers: vec![
            PatternSpec::new("escapeshellarg"),
            PatternSpec::new("shlex"),
            PatternSpec::new("shellescape"),
            PatternSpec::new("quote"),
        ],
    }
}

fn path_traversal_rule() -> SecurityRule {
    SecurityRule {
        id: "path_traversal".to_string(),
        name: "Path Traversal".to_string(),
        cwe: "CWE-22".to_string(),
        severity: Severity::High,
        description: "User input used to construct file system paths".to_string(),
        sources: vec![
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
            PatternSpec::new("params").langs(&["ruby"]),
        ],
        sinks: vec![
            PatternSpec::new("readFile"),
            PatternSpec::new("writeFile"),
            PatternSpec::new("createReadStream"),
            PatternSpec::new("createWriteStream"),
            PatternSpec::new("fs.open").langs(&["javascript", "typescript"]),
            PatternSpec::new("open(").desc("File open"),
            PatternSpec::new("File(").langs(&["java"]),
            PatternSpec::new("Paths.get").langs(&["java"]),
            PatternSpec::new("fopen").langs(&["php", "c", "cpp"]),
            PatternSpec::new("file_get_contents").langs(&["php"]),
            PatternSpec::new("File.open").langs(&["ruby"]),
            PatternSpec::new("os.Open").langs(&["go"]),
            PatternSpec::new("ioutil.ReadFile").langs(&["go"]),
            PatternSpec::new("std::fs").langs(&["rust"]),
        ],
        sanitizers: vec![
            PatternSpec::new("path.normalize"),
            PatternSpec::new("path.resolve"),
            PatternSpec::new("realpath"),
            PatternSpec::new("canonicalize"),
            PatternSpec::new("basename"),
        ],
    }
}

fn ssrf_rule() -> SecurityRule {
    SecurityRule {
        id: "ssrf".to_string(),
        name: "Server-Side Request Forgery (SSRF)".to_string(),
        cwe: "CWE-918".to_string(),
        severity: Severity::High,
        description: "User input used to construct server-side HTTP requests".to_string(),
        sources: vec![
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
        ],
        sinks: vec![
            PatternSpec::new("fetch("),
            PatternSpec::new("axios"),
            PatternSpec::new("http.get"),
            PatternSpec::new("http.request"),
            PatternSpec::new("urllib").langs(&["python"]),
            PatternSpec::new("requests.get").langs(&["python"]),
            PatternSpec::new("requests.post").langs(&["python"]),
            PatternSpec::new("httpClient").langs(&["java"]),
            PatternSpec::new("HttpURLConnection").langs(&["java"]),
            PatternSpec::new("RestTemplate").langs(&["java"]),
            PatternSpec::new("curl_exec").langs(&["php"]),
            PatternSpec::new("file_get_contents").langs(&["php"]),
            PatternSpec::new("http.Get").langs(&["go"]),
            PatternSpec::new("reqwest").langs(&["rust"]),
        ],
        sanitizers: vec![
            PatternSpec::new("allowlist"),
            PatternSpec::new("whitelist"),
            PatternSpec::new("validateUrl"),
            PatternSpec::new("isValidUrl"),
        ],
    }
}

fn deserialization_rule() -> SecurityRule {
    SecurityRule {
        id: "deserialization".to_string(),
        name: "Insecure Deserialization".to_string(),
        cwe: "CWE-502".to_string(),
        severity: Severity::Critical,
        description: "User-controlled data passed to deserialization functions".to_string(),
        sources: vec![
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("getInputStream").langs(&["java"]),
            PatternSpec::new("request.data").langs(&["python"]),
            PatternSpec::new("$_POST").langs(&["php"]),
        ],
        sinks: vec![
            PatternSpec::new("JSON.parse"),
            PatternSpec::new("eval("),
            PatternSpec::new("ObjectInputStream").langs(&["java"]),
            PatternSpec::new("readObject").langs(&["java"]),
            PatternSpec::new("XMLDecoder").langs(&["java"]),
            PatternSpec::new("yaml.load").langs(&["python"]).desc("Use yaml.safe_load instead"),
            PatternSpec::new("pickle.loads").langs(&["python"]),
            PatternSpec::new("marshal.loads").langs(&["python"]),
            PatternSpec::new("unserialize").langs(&["php"]),
            PatternSpec::new("Marshal.load").langs(&["ruby"]),
        ],
        sanitizers: vec![
            PatternSpec::new("safe_load"),
            PatternSpec::new("SafeLoader"),
            PatternSpec::new("whitelist"),
            PatternSpec::new("allowedClasses"),
        ],
    }
}

fn ldap_injection_rule() -> SecurityRule {
    SecurityRule {
        id: "ldap_injection".to_string(),
        name: "LDAP Injection".to_string(),
        cwe: "CWE-90".to_string(),
        severity: Severity::High,
        description: "User input used in LDAP queries without escaping".to_string(),
        sources: vec![
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.form").langs(&["python"]),
            PatternSpec::new("$_POST").langs(&["php"]),
        ],
        sinks: vec![
            PatternSpec::new("ldap.search"),
            PatternSpec::new("search_s").langs(&["python"]),
            PatternSpec::new("DirContext.search").langs(&["java"]),
            PatternSpec::new("ldap_search").langs(&["php"]),
        ],
        sanitizers: vec![
            PatternSpec::new("escape_filter"),
            PatternSpec::new("ldap_escape"),
        ],
    }
}

fn xxe_rule() -> SecurityRule {
    SecurityRule {
        id: "xxe".to_string(),
        name: "XML External Entity (XXE)".to_string(),
        cwe: "CWE-611".to_string(),
        severity: Severity::High,
        description: "XML parsing with external entities enabled".to_string(),
        sources: vec![
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getInputStream").langs(&["java"]),
            PatternSpec::new("request.data").langs(&["python"]),
            PatternSpec::new("$_POST").langs(&["php"]),
        ],
        sinks: vec![
            PatternSpec::new("parseXML"),
            PatternSpec::new("DOMParser"),
            PatternSpec::new("DocumentBuilder").langs(&["java"]),
            PatternSpec::new("SAXParser").langs(&["java"]),
            PatternSpec::new("XMLReader").langs(&["java"]),
            PatternSpec::new("etree.parse").langs(&["python"]),
            PatternSpec::new("etree.fromstring").langs(&["python"]),
            PatternSpec::new("xml_parse").langs(&["php"]),
            PatternSpec::new("simplexml_load").langs(&["php"]),
        ],
        sanitizers: vec![
            PatternSpec::new("disallow-doctype-decl"),
            PatternSpec::new("FEATURE_SECURE_PROCESSING"),
            PatternSpec::new("defusedxml"),
            PatternSpec::new("LIBXML_NOENT"),
        ],
    }
}

fn open_redirect_rule() -> SecurityRule {
    SecurityRule {
        id: "open_redirect".to_string(),
        name: "Open Redirect".to_string(),
        cwe: "CWE-601".to_string(),
        severity: Severity::Medium,
        description: "User input used in redirect target without validation".to_string(),
        sources: vec![
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.params").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
            PatternSpec::new("params").langs(&["ruby"]),
        ],
        sinks: vec![
            PatternSpec::new("res.redirect").langs(&["javascript", "typescript"]),
            PatternSpec::new("redirect("),
            PatternSpec::new("location.href"),
            PatternSpec::new("window.location"),
            PatternSpec::new("sendRedirect").langs(&["java"]),
            PatternSpec::new("HttpResponseRedirect").langs(&["python"]),
            PatternSpec::new("header(\"Location").langs(&["php"]),
            PatternSpec::new("redirect_to").langs(&["ruby"]),
        ],
        sanitizers: vec![
            PatternSpec::new("isRelativeUrl"),
            PatternSpec::new("isSameDomain"),
            PatternSpec::new("url.parse"),
            PatternSpec::new("validateRedirect"),
        ],
    }
}

fn log_injection_rule() -> SecurityRule {
    SecurityRule {
        id: "log_injection".to_string(),
        name: "Log Injection / Forging".to_string(),
        cwe: "CWE-117".to_string(),
        severity: Severity::Medium,
        description: "User input logged without sanitization, enabling log forging".to_string(),
        sources: vec![
            PatternSpec::new("req.query").langs(&["javascript", "typescript"]),
            PatternSpec::new("req.body").langs(&["javascript", "typescript"]),
            PatternSpec::new("getParameter").langs(&["java"]),
            PatternSpec::new("request.args").langs(&["python"]),
            PatternSpec::new("$_GET").langs(&["php"]),
        ],
        sinks: vec![
            PatternSpec::new("console.log"),
            PatternSpec::new("logger.info"),
            PatternSpec::new("logger.warn"),
            PatternSpec::new("logger.error"),
            PatternSpec::new("log.info"),
            PatternSpec::new("log.warn"),
            PatternSpec::new("log.error"),
            PatternSpec::new("logging.info").langs(&["python"]),
            PatternSpec::new("logging.warning").langs(&["python"]),
            PatternSpec::new("tracing::info").langs(&["rust"]),
        ],
        sanitizers: vec![
            PatternSpec::new("replace"),
            PatternSpec::new("strip"),
            PatternSpec::new("sanitizeLog"),
        ],
    }
}

fn hardcoded_secrets_rule() -> SecurityRule {
    SecurityRule {
        id: "hardcoded_secrets".to_string(),
        name: "Hardcoded Secrets/Credentials".to_string(),
        cwe: "CWE-798".to_string(),
        severity: Severity::High,
        description: "Hardcoded passwords, API keys, or tokens in source code".to_string(),
        sources: vec![],  // Not taint-based — pattern-only
        sinks: vec![],
        sanitizers: vec![],
    }
}
