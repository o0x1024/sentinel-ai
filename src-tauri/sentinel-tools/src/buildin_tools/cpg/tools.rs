use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::builder;
use super::types::CodePropertyGraph;

// ── Global CPG cache (multiple projects) ────────────────────────────────────

/// Maximum number of CPGs to keep cached simultaneously.
/// When exceeded, the oldest entry is evicted.
const MAX_CACHE_ENTRIES: usize = 5;

static CPG_CACHE: Lazy<Arc<RwLock<HashMap<String, CachedCpg>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

struct CachedCpg {
    cpg: Arc<CodePropertyGraph>,
    /// Monotonically increasing access counter for LRU eviction.
    access_order: u64,
}

/// Global counter for LRU ordering.
static ACCESS_COUNTER: Lazy<std::sync::atomic::AtomicU64> =
    Lazy::new(|| std::sync::atomic::AtomicU64::new(0));

fn next_access_order() -> u64 {
    ACCESS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub async fn get_or_build_cpg(root: &str, max_files: usize) -> Result<Arc<CodePropertyGraph>, String> {
    // Check cache (use original path as key for consistency)
    {
        let mut cache = CPG_CACHE.write().await;
        if let Some(cached) = cache.get_mut(root) {
            cached.access_order = next_access_order();
            return Ok(cached.cpg.clone());
        }
    }

    // Resolve path: in Docker mode, /workspace/xxx in the container maps to
    // a host-side volume mount (typically /tmp/workspace/xxx).
    // The CPG builder runs on the host, so we need the host path.
    let resolved_root = resolve_host_path(root).await;

    // Build (CPU-intensive, use spawn_blocking)
    let cpg = tokio::task::spawn_blocking(move || builder::build_cpg(&resolved_root, max_files))
        .await
        .map_err(|e| format!("CPG build task failed: {}", e))??;

    let cpg = Arc::new(cpg);

    // Update cache
    {
        let mut cache = CPG_CACHE.write().await;

        // Evict oldest if at capacity
        if cache.len() >= MAX_CACHE_ENTRIES && !cache.contains_key(root) {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, v)| v.access_order)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(
            root.to_string(),
            CachedCpg {
                cpg: cpg.clone(),
                access_order: next_access_order(),
            },
        );
    }

    Ok(cpg)
}

/// Force-rebuild the cache (used when `force` is set).
async fn rebuild_cpg(root: &str, max_files: usize) -> Result<Arc<CodePropertyGraph>, String> {
    let resolved_root = resolve_host_path(root).await;
    let cpg = tokio::task::spawn_blocking(move || builder::build_cpg(&resolved_root, max_files))
        .await
        .map_err(|e| format!("CPG build task failed: {}", e))??;

    let cpg = Arc::new(cpg);

    {
        let mut cache = CPG_CACHE.write().await;
        cache.insert(
            root.to_string(),
            CachedCpg {
                cpg: cpg.clone(),
                access_order: next_access_order(),
            },
        );
    }

    Ok(cpg)
}

/// Try to get audit context from a cached CPG (non-blocking).
/// Returns None if no CPG is cached — the AI can always build one later.
/// If multiple CPGs are cached, returns context for the most recently accessed one.
pub async fn try_get_cpg_audit_context() -> Option<String> {
    let cache = CPG_CACHE.read().await;
    // Return the most recently accessed CPG
    if let Some((root, cached)) = cache.iter().max_by_key(|(_, v)| v.access_order) {
        let cpg = cached.cpg.clone();
        let context = super::context::generate_audit_context(&cpg);
        let notice = super::context::cpg_availability_notice(root);
        Some(format!("{}\n{}", context, notice))
    } else {
        None
    }
}

/// Try to auto-build CPG for a given path (fire-and-forget, non-blocking).
/// If the CPG is already cached for this path, returns immediately.
pub async fn try_auto_build_cpg(path: &str) -> Option<String> {
    // Check if already cached
    {
        let cache = CPG_CACHE.read().await;
        if let Some(cached) = cache.get(path) {
            let cpg = cached.cpg.clone();
            let context = super::context::generate_audit_context(&cpg);
            let notice = super::context::cpg_availability_notice(path);
            return Some(format!("{}\n{}", context, notice));
        }
    }

    // Build CPG
    match get_or_build_cpg(path, 5000).await {
        Ok(cpg) => {
            let context = super::context::generate_audit_context(&cpg);
            let notice = super::context::cpg_availability_notice(path);
            Some(format!("{}\n{}", context, notice))
        }
        Err(e) => {
            tracing::warn!("Auto-build CPG failed for '{}': {}", path, e);
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BuildCpgTool
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BuildCpgArgs {
    /// Project root directory to analyze
    pub path: String,
    /// Maximum number of source files to parse (default: 5000)
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    /// Force rebuild even if a cached CPG exists for this path
    #[serde(default)]
    pub force: bool,
}

fn default_max_files() -> usize {
    5000
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildCpgOutput {
    pub success: bool,
    pub root: String,
    pub primary_language: String,
    pub languages: Vec<String>,
    pub total_files: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub total_imports: usize,
    pub total_call_edges: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildCpgError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("build failed: {0}")]
    BuildFailed(String),
}

/// Build a Code Property Graph for a project.
///
/// Parses all supported source files using tree-sitter, extracts functions,
/// classes, imports, call sites, and resolves intra-project call edges.
/// The CPG is cached in memory for subsequent `query_cpg` calls.
#[derive(Debug, Clone)]
pub struct BuildCpgTool;

impl BuildCpgTool {
    pub const NAME: &'static str = "build_cpg";
    pub const DESCRIPTION: &'static str =
        "Build a Code Property Graph (CPG) from a project directory. \
         Parses all source files using AST analysis to extract functions, classes, imports, \
         call relationships, and code structure. The CPG is cached for fast querying via `query_cpg`. \
         Supports: Rust, JavaScript/TypeScript, Python, Java, Go, C/C++, C#, PHP, Ruby. \
         Call this first before using `query_cpg` for structural code analysis.";
}

impl Tool for BuildCpgTool {
    const NAME: &'static str = Self::NAME;
    type Args = BuildCpgArgs;
    type Output = BuildCpgOutput;
    type Error = BuildCpgError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BuildCpgArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.trim();
        if path.is_empty() {
            return Err(BuildCpgError::InvalidArgs("path is required".to_string()));
        }

        let max_files = args.max_files.clamp(1, 20_000);

        let cpg = if args.force {
            rebuild_cpg(path, max_files)
                .await
                .map_err(BuildCpgError::BuildFailed)?
        } else {
            get_or_build_cpg(path, max_files)
                .await
                .map_err(BuildCpgError::BuildFailed)?
        };

        let summary = cpg.summary();

        Ok(BuildCpgOutput {
            success: true,
            root: summary.root,
            primary_language: summary.primary_language,
            languages: summary.languages,
            total_files: summary.total_files,
            total_functions: summary.total_functions,
            total_classes: summary.total_classes,
            total_imports: summary.total_imports,
            total_call_edges: summary.total_call_edges,
            total_nodes: summary.total_nodes,
            total_edges: summary.total_edges,
            message: Some(format!(
                "CPG built successfully. Use `query_cpg` to explore the code structure."
            )),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QueryCpgTool
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct QueryCpgArgs {
    /// Project root directory (must match a previously built CPG path)
    pub path: String,

    /// Query type to execute
    #[serde(default = "default_query")]
    pub query: CpgQuery,
}

fn default_query() -> CpgQuery {
    CpgQuery::Summary
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CpgQuery {
    /// Overview statistics (nodes, edges, languages)
    Summary,
    /// List all functions/methods (optionally filtered by file)
    Functions {
        #[serde(skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// List all classes/structs
    Classes {
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// List all imports
    Imports {
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// List resolved call edges (function → function)
    CallEdges {
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// Find all callers of a specific function
    CallersOf {
        function_name: String,
    },
    /// Find all functions that a specific function calls
    CalleesOf {
        function_name: String,
    },
    /// List all files with summaries (function count, class count, etc.)
    Files {
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// Search functions/methods by name substring
    Search {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryCpgOutput {
    pub success: bool,
    pub query_type: String,
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryCpgError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("cpg not found: {0}")]
    CpgNotFound(String),
    #[error("query failed: {0}")]
    QueryFailed(String),
}

/// Query a previously built Code Property Graph.
///
/// Supports: summary, functions, classes, imports, call_edges, callers_of,
/// callees_of, files, search.
#[derive(Debug, Clone)]
pub struct QueryCpgTool;

impl QueryCpgTool {
    pub const NAME: &'static str = "query_cpg";
    pub const DESCRIPTION: &'static str =
        "Query the Code Property Graph (CPG) built by `build_cpg`. \
         Supports structural queries: list functions, classes, imports, call edges, \
         find callers/callees of a function, list file summaries, and search symbols by name. \
         The CPG must be built first via `build_cpg`. \
         \n\nQuery types:\n\
         - summary: Overview stats\n\
         - functions: List functions (optional file filter)\n\
         - classes: List classes/structs\n\
         - imports: List imports\n\
         - call_edges: List function→function call relationships\n\
         - callers_of: Who calls function X?\n\
         - callees_of: What does function X call?\n\
         - files: List files with function/class counts\n\
         - search: Find symbols by name substring";
}

impl Tool for QueryCpgTool {
    const NAME: &'static str = Self::NAME;
    type Args = QueryCpgArgs;
    type Output = QueryCpgOutput;
    type Error = QueryCpgError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(QueryCpgArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.trim();
        if path.is_empty() {
            return Err(QueryCpgError::InvalidArgs("path is required".to_string()));
        }

        // Get CPG from cache (auto-build if not present)
        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| QueryCpgError::CpgNotFound(format!(
                "CPG not found for '{}'. Call `build_cpg` first. Error: {}", path, e
            )))?;

        let (query_type, result) = match args.query {
            CpgQuery::Summary => {
                let s = cpg.summary();
                ("summary".to_string(), serde_json::to_value(s).unwrap_or_default())
            }
            CpgQuery::Functions { file, limit } => {
                let limit = limit.clamp(1, 500);
                let funcs = if let Some(file_path) = file {
                    cpg.functions_in_file(&file_path)
                } else {
                    cpg.list_functions(limit)
                };
                ("functions".to_string(), serde_json::to_value(&funcs).unwrap_or_default())
            }
            CpgQuery::Classes { limit } => {
                let classes = cpg.list_classes(limit.clamp(1, 500));
                ("classes".to_string(), serde_json::to_value(&classes).unwrap_or_default())
            }
            CpgQuery::Imports { limit } => {
                let imports = cpg.list_imports(limit.clamp(1, 500));
                ("imports".to_string(), serde_json::to_value(&imports).unwrap_or_default())
            }
            CpgQuery::CallEdges { limit } => {
                let edges = cpg.list_call_edges(limit.clamp(1, 500));
                ("call_edges".to_string(), serde_json::to_value(&edges).unwrap_or_default())
            }
            CpgQuery::CallersOf { function_name } => {
                let callers = cpg.callers_of(&function_name);
                ("callers_of".to_string(), serde_json::to_value(&callers).unwrap_or_default())
            }
            CpgQuery::CalleesOf { function_name } => {
                let callees = cpg.callees_of(&function_name);
                ("callees_of".to_string(), serde_json::to_value(&callees).unwrap_or_default())
            }
            CpgQuery::Files { limit } => {
                let files = cpg.list_file_summaries(limit.clamp(1, 500));
                ("files".to_string(), serde_json::to_value(&files).unwrap_or_default())
            }
            CpgQuery::Search { query, limit } => {
                let results = cpg.search_symbols(&query, limit.clamp(1, 200));
                ("search".to_string(), serde_json::to_value(&results).unwrap_or_default())
            }
        };

        Ok(QueryCpgOutput {
            success: true,
            query_type,
            result,
            message: None,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CpgTaintAnalysisTool
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CpgTaintAnalysisArgs {
    /// Project root directory
    pub path: String,
    /// Rule IDs to check (empty = all rules).
    /// Available: sql_injection, xss, command_injection, path_traversal,
    /// ssrf, deserialization, ldap_injection, xxe, open_redirect, log_injection
    #[serde(default)]
    pub rules: Vec<String>,
    /// Maximum call-graph traversal depth (default: 8)
    #[serde(default = "default_taint_depth")]
    pub max_depth: usize,
    /// Maximum findings per rule (default: 30)
    #[serde(default = "default_max_findings_per_rule")]
    pub max_findings_per_rule: usize,
}

fn default_taint_depth() -> usize {
    8
}

fn default_max_findings_per_rule() -> usize {
    30
}

#[derive(Debug, Clone, Serialize)]
pub struct CpgTaintAnalysisOutput {
    pub success: bool,
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CpgTaintAnalysisError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Run graph-based taint analysis on a project's CPG.
///
/// Traces data flow from user-input sources to dangerous sinks through
/// the call graph, detecting sanitizers on the path. Much more accurate
/// than regex-based approaches because it uses actual call relationships.
#[derive(Debug, Clone)]
pub struct CpgTaintAnalysisTool;

impl CpgTaintAnalysisTool {
    pub const NAME: &'static str = "cpg_taint_analysis";
    pub const DESCRIPTION: &'static str =
         "Run graph-based taint analysis on a project using the Code Property Graph. \
          Traces data flow from user-input sources (req.params, getParameter, request.args, etc.) \
          to dangerous sinks (execute, query, eval, innerHTML, etc.) through the actual call graph. \
          Detects sanitizers on the path. Supports 15 vulnerability classes: \
          sql_injection, xss, command_injection, path_traversal, ssrf, deserialization, \
          ldap_injection, xxe, open_redirect, log_injection, hardcoded_secrets, \
          crypto_misuse, auth_bypass, insecure_random, config_security. \
          Requires `build_cpg` to be called first (auto-builds if not cached).";
}

impl Tool for CpgTaintAnalysisTool {
    const NAME: &'static str = Self::NAME;
    type Args = CpgTaintAnalysisArgs;
    type Output = CpgTaintAnalysisOutput;
    type Error = CpgTaintAnalysisError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CpgTaintAnalysisArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.trim();
        if path.is_empty() {
            return Err(CpgTaintAnalysisError::InvalidArgs("path is required".to_string()));
        }

        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| CpgTaintAnalysisError::AnalysisFailed(format!("CPG error: {}", e)))?;

        let rules = super::security_rules::rules_by_ids(&args.rules);
        if rules.is_empty() {
            return Err(CpgTaintAnalysisError::InvalidArgs(
                "No matching rules found. Available: sql_injection, xss, command_injection, \
                 path_traversal, ssrf, deserialization, ldap_injection, xxe, open_redirect, \
                 log_injection".to_string(),
            ));
        }

        let max_depth = args.max_depth.clamp(1, 20);
        let max_per_rule = args.max_findings_per_rule.clamp(1, 200);

        // Run taint analysis (CPU-bound)
        let cpg_clone = cpg.clone();
        let result = tokio::task::spawn_blocking(move || {
            super::taint::run_taint_analysis(&cpg_clone, &rules, max_depth, max_per_rule)
        })
        .await
        .map_err(|e| CpgTaintAnalysisError::AnalysisFailed(format!("Task failed: {}", e)))?;

        let msg = format!(
            "Found {} taint flows ({} unsanitized) across {} rules. Sources: {}, Sinks: {}",
            result.total_findings,
            result.unsanitized_findings,
            result.rules_checked.len(),
            result.total_sources,
            result.total_sinks,
        );

        Ok(CpgTaintAnalysisOutput {
            success: true,
            result: serde_json::to_value(&result).unwrap_or_default(),
            message: Some(msg),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CpgSecurityScanTool
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CpgSecurityScanArgs {
    /// Project root directory
    pub path: String,
    /// Maximum call-graph depth (default: 6)
    #[serde(default = "default_scan_depth")]
    pub max_depth: usize,
    /// Maximum total findings (default: 100)
    #[serde(default = "default_scan_max_findings")]
    pub max_findings: usize,
}

fn default_scan_depth() -> usize {
    6
}

fn default_scan_max_findings() -> usize {
    100
}

#[derive(Debug, Clone, Serialize)]
pub struct CpgSecurityScanOutput {
    pub success: bool,
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CpgSecurityScanError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("scan failed: {0}")]
    ScanFailed(String),
}

/// Run a full security scan using all built-in rules.
///
/// Combines taint analysis (source→sink tracing) with pattern matching
/// (hardcoded secrets, dangerous patterns) for a comprehensive baseline assessment.
#[derive(Debug, Clone)]
pub struct CpgSecurityScanTool;

impl CpgSecurityScanTool {
    pub const NAME: &'static str = "cpg_security_scan";
    pub const DESCRIPTION: &'static str =
        "Run a full security scan on a project using all built-in vulnerability rules. \
         Combines graph-based taint analysis (source→sink tracing through call graph) \
         with pattern matching (hardcoded secrets, dangerous API usage). \
         Returns findings grouped by severity with source/sink locations and trace paths. \
         Use this for initial baseline assessment before deep-diving with `cpg_taint_analysis`.";
}

impl Tool for CpgSecurityScanTool {
    const NAME: &'static str = Self::NAME;
    type Args = CpgSecurityScanArgs;
    type Output = CpgSecurityScanOutput;
    type Error = CpgSecurityScanError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CpgSecurityScanArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.trim();
        if path.is_empty() {
            return Err(CpgSecurityScanError::InvalidArgs("path is required".to_string()));
        }

        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| CpgSecurityScanError::ScanFailed(format!("CPG error: {}", e)))?;

        let all_rules = super::security_rules::all_rules();
        let max_depth = args.max_depth.clamp(1, 15);
        let max_findings = args.max_findings.clamp(1, 500);

        let cpg_clone = cpg.clone();
        let result = tokio::task::spawn_blocking(move || {
            super::taint::run_security_scan(&cpg_clone, &all_rules, max_depth, max_findings)
        })
        .await
        .map_err(|e| CpgSecurityScanError::ScanFailed(format!("Task failed: {}", e)))?;

        let msg = format!(
            "Security scan complete. {} rules checked, {} findings total \
             (Critical: {}, High: {}, Medium: {}, Low: {}, Info: {}). \
             {} pattern findings.",
            result.total_rules,
            result.total_findings,
            result.by_severity.critical,
            result.by_severity.high,
            result.by_severity.medium,
            result.by_severity.low,
            result.by_severity.info,
            result.pattern_findings.len(),
        );

        Ok(CpgSecurityScanOutput {
            success: true,
            result: serde_json::to_value(&result).unwrap_or_default(),
            message: Some(msg),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Docker path resolution
// ═══════════════════════════════════════════════════════════════════════════

/// Resolve a container path to a host path.
///
/// When running in Docker mode, paths like `/workspace/xxx` exist inside the
/// container but not on the host. The CPG builder runs directly on the host
/// and needs the actual filesystem path. This function reads the Docker volume
/// mount configuration and remaps accordingly.
///
/// For example:
///   container path: `/workspace/fastgpt-be`
///   volume mount:   `/tmp/workspace` → `/workspace`
///   host path:      `/tmp/workspace/fastgpt-be`
async fn resolve_host_path(path: &str) -> String {
    let path_buf = std::path::PathBuf::from(path);

    // If the path already exists on the host filesystem, use it as-is
    if path_buf.is_dir() {
        return path.to_string();
    }

    // Load shell config to see if we're in Docker mode
    let shell_cfg = crate::buildin_tools::shell::get_shell_config().await;
    if shell_cfg.default_execution_mode != crate::buildin_tools::shell::ShellExecutionMode::Docker {
        // Not Docker mode, return the path as-is
        return path.to_string();
    }

    // In Docker mode: try to map container path to host path using volumes
    if let Some(docker_cfg) = &shell_cfg.docker_config {
        for (host_mount, container_mount) in &docker_cfg.volumes {
            // container_mount: "/workspace", host_mount: "/tmp/workspace"
            let container_prefix = container_mount.trim_end_matches('/');

            if path == container_prefix || path.starts_with(&format!("{}/", container_prefix)) {
                // Map: replace container_mount prefix with host_mount
                let relative = path
                    .strip_prefix(container_prefix)
                    .unwrap_or("")
                    .trim_start_matches('/');

                let host_path = if relative.is_empty() {
                    host_mount.clone()
                } else {
                    format!("{}/{}", host_mount.trim_end_matches('/'), relative)
                };

                tracing::info!(
                    "CPG path remapped: {} -> {} (Docker volume: {} -> {})",
                    path,
                    host_path,
                    host_mount,
                    container_mount
                );

                // Verify the host path exists
                if std::path::Path::new(&host_path).is_dir() {
                    return host_path;
                }

                tracing::warn!(
                    "Remapped host path '{}' does not exist. Docker container may not have synced yet.",
                    host_path
                );
                // Return it anyway — the builder will give a better error
                return host_path;
            }
        }
    }

    // Fallback: no matching volume mount found.
    // Try common default: /workspace -> /tmp/workspace  
    if path.starts_with("/workspace") {
        let host_fallback = format!("/tmp{}", path);
        if std::path::Path::new(&host_fallback).is_dir() {
            tracing::info!("CPG path remapped (fallback): {} -> {}", path, host_fallback);
            return host_fallback;
        }
    }

    // Return original path
    path.to_string()
}
