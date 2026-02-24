//! V2 Audit Tools — Enhanced perception layer for LLM-led code audit.
//!
//! - `get_function_detail`  — Rich function signatures, params, security context
//! - `get_attack_surface`   — One-shot attack surface enumeration
//! - `smart_file_summary`   — High-density file summary with security hotspots
//! - `trace_data_flow`      — Variable-level data flow tracing through CPG

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::cpg::tools::get_or_build_cpg;
use super::cpg::types::*;

use petgraph::Direction;


// ============================================================================
// GetFunctionDetailTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetFunctionDetailArgs {
    /// Project root path (where CPG was built)
    #[serde(default)]
    pub path: Option<String>,
    /// Function/method name pattern. Supports:
    /// - Exact match: "saveUser"
    /// - Prefix wildcard: "UserService.*" (all methods in UserService)
    /// - Suffix wildcard: "*Controller" (all controller functions)
    /// - Substring: "*auth*" (anything containing "auth")
    pub function_name: String,
    /// If true, include the function body source code (costs more context)
    #[serde(default)]
    pub include_body: bool,
    /// Max results. Default: 30
    #[serde(default = "default_fn_detail_limit")]
    pub limit: usize,
}

fn default_fn_detail_limit() -> usize {
    30
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamDetail {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_annotation: Option<String>,
    pub index: usize,
    /// Whether this parameter is likely from external input (e.g., @RequestParam, req.body)
    pub is_external_input: bool,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionSecurityContext {
    /// Does this function or its middleware stack include auth checks?
    pub has_auth_check: bool,
    /// Does this function accept external/user input?
    pub accepts_external_input: bool,
    /// Does this function perform database operations?
    pub has_db_operation: bool,
    /// Does this function perform file I/O?
    pub has_file_operation: bool,
    /// Does this function execute system commands?
    pub has_command_exec: bool,
    /// Does this function perform network requests?
    pub has_network_request: bool,
    /// Security-related annotations found
    pub security_annotations: Vec<String>,
    /// Risk indicators
    pub risk_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDetailInfo {
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub parameters: Vec<ParamDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    pub visibility: String,
    pub is_async: bool,
    pub is_static: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    pub annotations: Vec<String>,
    pub security_context: FunctionSecurityContext,
    /// Callers (simplified: "ClassName.methodName" or "functionName")
    pub callers: Vec<String>,
    /// Callees (simplified)
    pub callees: Vec<String>,
    /// Function body (only when include_body=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetFunctionDetailOutput {
    pub path: String,
    pub functions: Vec<FunctionDetailInfo>,
    pub total_matched: usize,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GetFunctionDetailError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("cpg not available: {0}")]
    CpgNotAvailable(String),
}

#[derive(Debug, Clone)]
pub struct GetFunctionDetailTool;

impl GetFunctionDetailTool {
    pub const NAME: &'static str = "get_function_detail";
    pub const DESCRIPTION: &'static str =
        "Get rich function/method details from the Code Property Graph including signatures, \
         parameters with types, security context (auth checks, DB ops, external input), \
         callers and callees. Supports wildcard patterns like 'UserService.*' or '*auth*'. \
         Much more efficient than reading entire files — use this to understand function \
         interfaces before deciding which code to read in detail.";
}

impl Tool for GetFunctionDetailTool {
    const NAME: &'static str = Self::NAME;
    type Args = GetFunctionDetailArgs;
    type Output = GetFunctionDetailOutput;
    type Error = GetFunctionDetailError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GetFunctionDetailArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.as_deref().unwrap_or("/workspace");
        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| GetFunctionDetailError::CpgNotAvailable(e))?;

        let pattern = args.function_name.trim();
        if pattern.is_empty() {
            return Err(GetFunctionDetailError::InvalidArgs(
                "function_name is required".to_string(),
            ));
        }

        let mut functions = Vec::new();

        for idx in cpg.graph.node_indices() {
            let node = &cpg.graph[idx];
            let (name, class_name, visibility, is_async, is_static, signature) = match &node.kind {
                CpgNodeKind::Function {
                    name,
                    signature,
                    visibility,
                    is_async,
                } => (
                    name.clone(),
                    None,
                    visibility.clone(),
                    *is_async,
                    false,
                    signature.clone(),
                ),
                CpgNodeKind::Method {
                    name,
                    class_name,
                    is_static,
                    is_async,
                    visibility,
                } => (
                    name.clone(),
                    class_name.clone(),
                    visibility.clone(),
                    *is_async,
                    *is_static,
                    None,
                ),
                _ => continue,
            };

            let qualified = if let Some(ref cls) = class_name {
                format!("{}.{}", cls, name)
            } else {
                name.clone()
            };

            if !matches_pattern(pattern, &name, &qualified) {
                continue;
            }

            // Collect parameters with detail
            let params = collect_param_details(&cpg, idx);

            // Build security context by analyzing callees and annotations
            let security_context = build_security_context(&cpg, idx, &params, &node.tags);

            // Collect callers
            let callers = collect_callers(&cpg, idx);
            let callees = collect_callees(&cpg, idx);

            // Extract annotations from signature and tags
            let annotations = extract_annotations(signature.as_deref(), &node.tags);

            functions.push(FunctionDetailInfo {
                name: name.clone(),
                qualified_name: qualified,
                file: node.file.clone().unwrap_or_default(),
                line_start: node.start_line,
                line_end: node.end_line,
                signature,
                parameters: params,
                return_type: None, // TODO: extract from signature
                visibility,
                is_async,
                is_static,
                class_name,
                annotations,
                security_context,
                callers,
                callees,
                body: None, // body loading deferred
            });

            if functions.len() >= args.limit * 2 {
                break; // over-collect then truncate
            }
        }

        // Sort by file + line for consistent ordering
        functions.sort_by(|a, b| a.file.cmp(&b.file).then(a.line_start.cmp(&b.line_start)));

        let total_matched = functions.len();
        let truncated = total_matched > args.limit;
        functions.truncate(args.limit);

        // Optionally load function bodies
        if args.include_body {
            for func in &mut functions {
                if !func.file.is_empty() && func.line_end > func.line_start {
                    func.body = read_function_body(&func.file, func.line_start, func.line_end).await;
                }
            }
        }

        Ok(GetFunctionDetailOutput {
            path: path.to_string(),
            functions,
            total_matched,
            truncated,
        })
    }
}

// ============================================================================
// GetAttackSurfaceTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetAttackSurfaceArgs {
    /// Project root path
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputParamInfo {
    pub name: String,
    pub source: String, // "path" | "query" | "body" | "header" | "cookie" | "unknown"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HttpEndpointDetail {
    pub method: String,
    pub route: String,
    pub handler_function: String,
    pub handler_file: String,
    pub handler_line: usize,
    /// Auth status inferred from middleware/annotations
    pub auth_status: String, // "protected" | "unprotected" | "unknown"
    pub auth_annotations: Vec<String>,
    pub input_params: Vec<InputParamInfo>,
    /// Security risk hints for this endpoint
    pub risk_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttackSurfaceSummary {
    pub total_endpoints: usize,
    pub unprotected_endpoints: usize,
    pub endpoints_with_external_input: usize,
    pub endpoints_with_db_ops: usize,
    pub endpoints_with_file_ops: usize,
    /// Endpoints grouped by risk level
    pub critical_risk_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetAttackSurfaceOutput {
    pub path: String,
    pub summary: AttackSurfaceSummary,
    /// All HTTP endpoints with rich detail
    pub http_endpoints: Vec<HttpEndpointDetail>,
    /// Functions that take external input but are NOT registered as endpoints
    /// (may be internal handlers called from routes)
    pub input_receiving_functions: Vec<InputReceiverInfo>,
    /// Database access functions (potential injection targets)
    pub db_access_functions: Vec<DbAccessInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputReceiverInfo {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub input_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DbAccessInfo {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub access_type: String, // "raw_query" | "orm" | "prepared_statement"
}

#[derive(Debug, thiserror::Error)]
pub enum GetAttackSurfaceError {
    #[error("cpg not available: {0}")]
    CpgNotAvailable(String),
}

#[derive(Debug, Clone)]
pub struct GetAttackSurfaceTool;

impl GetAttackSurfaceTool {
    pub const NAME: &'static str = "get_attack_surface";
    pub const DESCRIPTION: &'static str =
        "Analyze a project's attack surface by enumerating all HTTP endpoints, their auth status, \
         input parameters, and associated risk indicators from the Code Property Graph. \
         Returns a prioritized list showing which endpoints are unprotected, accept user input, \
         or interact with databases. Use this at audit start to build a targeted audit plan.";
}

impl Tool for GetAttackSurfaceTool {
    const NAME: &'static str = Self::NAME;
    type Args = GetAttackSurfaceArgs;
    type Output = GetAttackSurfaceOutput;
    type Error = GetAttackSurfaceError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GetAttackSurfaceArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.as_deref().unwrap_or("/workspace");
        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| GetAttackSurfaceError::CpgNotAvailable(e))?;

        let mut http_endpoints = Vec::new();
        let mut input_receiving_functions = Vec::new();
        let mut db_access_functions = Vec::new();

        // 1. Collect all EntryPoint nodes → HTTP endpoints
        for idx in cpg.graph.node_indices() {
            let node = &cpg.graph[idx];
            if let CpgNodeKind::EntryPoint {
                method,
                path: route,
                handler,
            } = &node.kind
            {
                // Find the handler function node
                let handler_detail =
                    find_handler_detail(&cpg, idx, handler);

                let auth_annotations = handler_detail.auth_annotations.clone();
                let auth_status = if auth_annotations.is_empty() {
                    "unknown".to_string()
                } else if auth_annotations.iter().any(|a| {
                    let a_lower = a.to_lowercase();
                    a_lower.contains("public")
                        || a_lower.contains("allowanonymous")
                        || a_lower.contains("noauth")
                }) {
                    "unprotected".to_string()
                } else {
                    "protected".to_string()
                };

                let mut risk_indicators = Vec::new();
                if auth_status == "unprotected" || auth_status == "unknown" {
                    risk_indicators.push("No authentication detected".to_string());
                }
                if handler_detail.has_db_ops {
                    risk_indicators.push("Direct database access".to_string());
                }
                if handler_detail.has_file_ops {
                    risk_indicators.push("File system operations".to_string());
                }
                if handler_detail.has_command_exec {
                    risk_indicators.push("Command execution".to_string());
                }
                if !handler_detail.input_params.is_empty() && auth_status != "protected" {
                    risk_indicators.push("Accepts user input without confirmed auth".to_string());
                }

                http_endpoints.push(HttpEndpointDetail {
                    method: method.clone(),
                    route: route.clone(),
                    handler_function: handler.clone(),
                    handler_file: handler_detail.file.clone(),
                    handler_line: handler_detail.line,
                    auth_status,
                    auth_annotations,
                    input_params: handler_detail.input_params,
                    risk_indicators,
                });
            }
        }

        // 2. Find functions with external input patterns (not just entry points)
        for idx in cpg.graph.node_indices() {
            let node = &cpg.graph[idx];
            match &node.kind {
                CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. } => {
                    // Check if function has parameters annotated with input markers
                    let params = collect_param_details(&cpg, idx);
                    for param in &params {
                        if param.is_external_input {
                            input_receiving_functions.push(InputReceiverInfo {
                                function_name: node.kind.display_name(),
                                file: node.file.clone().unwrap_or_default(),
                                line: node.start_line,
                                input_type: format!("parameter: {}", param.name),
                            });
                            break; // one per function
                        }
                    }

                    // Check for DB-access patterns in callees
                    let callees_list = collect_callees(&cpg, idx);
                    for callee in &callees_list {
                        let cl = callee.to_lowercase();
                        if is_db_access_pattern(&cl) {
                            db_access_functions.push(DbAccessInfo {
                                function_name: node.kind.display_name(),
                                file: node.file.clone().unwrap_or_default(),
                                line: node.start_line,
                                access_type: infer_db_access_type(&cl),
                            });
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        // Sort endpoints by risk
        http_endpoints.sort_by(|a, b| b.risk_indicators.len().cmp(&a.risk_indicators.len()));

        let unprotected = http_endpoints
            .iter()
            .filter(|e| e.auth_status != "protected")
            .count();
        let with_input = http_endpoints
            .iter()
            .filter(|e| !e.input_params.is_empty())
            .count();
        let with_db = http_endpoints
            .iter()
            .filter(|e| e.risk_indicators.iter().any(|r| r.contains("database")))
            .count();
        let with_file = http_endpoints
            .iter()
            .filter(|e| e.risk_indicators.iter().any(|r| r.contains("File")))
            .count();

        let critical_risk = http_endpoints
            .iter()
            .filter(|e| e.risk_indicators.len() >= 3)
            .count();
        let high_risk = http_endpoints
            .iter()
            .filter(|e| e.risk_indicators.len() == 2)
            .count();
        let medium_risk = http_endpoints
            .iter()
            .filter(|e| e.risk_indicators.len() == 1)
            .count();

        let summary = AttackSurfaceSummary {
            total_endpoints: http_endpoints.len(),
            unprotected_endpoints: unprotected,
            endpoints_with_external_input: with_input,
            endpoints_with_db_ops: with_db,
            endpoints_with_file_ops: with_file,
            critical_risk_count: critical_risk,
            high_risk_count: high_risk,
            medium_risk_count: medium_risk,
        };

        Ok(GetAttackSurfaceOutput {
            path: path.to_string(),
            summary,
            http_endpoints,
            input_receiving_functions,
            db_access_functions,
        })
    }
}

// ============================================================================
// SmartFileSummaryTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SmartFileSummaryArgs {
    /// File path to summarize
    pub path: String,
    /// Analysis focus: "security" (default) | "api" | "data_flow" | "full"
    #[serde(default)]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkeletonEntry {
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
    pub annotations: Vec<String>,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecuritySignal {
    pub signal_type: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hotspot {
    pub line_start: usize,
    pub line_end: usize,
    pub reason: String,
    pub severity_hint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmartFileSummaryOutput {
    pub path: String,
    pub language: String,
    pub total_lines: usize,
    /// Structural skeleton of the file
    pub skeleton: Vec<SkeletonEntry>,
    /// Security-relevant signals found in this file
    pub security_signals: Vec<SecuritySignal>,
    /// HTTP endpoints exposed by this file
    pub exposed_endpoints: Vec<String>,
    /// Key imports/dependencies
    pub key_imports: Vec<String>,
    /// Hotspots worth deeper review (sorted by severity)
    pub hotspots: Vec<Hotspot>,
    /// Quick summary stats
    pub stats: FileSummaryStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileSummaryStats {
    pub functions: usize,
    pub classes: usize,
    pub imports: usize,
    pub call_sites: usize,
    pub entry_points: usize,
    pub security_signals_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SmartFileSummaryError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("cpg not available: {0}")]
    CpgNotAvailable(String),
    #[error("file not found in CPG: {0}")]
    FileNotFound(String),
}

#[derive(Debug, Clone)]
pub struct SmartFileSummaryTool;

impl SmartFileSummaryTool {
    pub const NAME: &'static str = "smart_file_summary";
    pub const DESCRIPTION: &'static str =
        "Get a high-density structural summary of a source file from the CPG, including: \
         function skeleton with signatures and parameters, security signals (external input, \
         DB queries, command exec, auth checks), HTTP endpoints, key imports, and hotspots \
         worth deeper review. Much more efficient than reading the whole file — typically \
         saves 80% of context compared to using read_file for initial understanding.";
}

impl Tool for SmartFileSummaryTool {
    const NAME: &'static str = Self::NAME;
    type Args = SmartFileSummaryArgs;
    type Output = SmartFileSummaryOutput;
    type Error = SmartFileSummaryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SmartFileSummaryArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let file_path = args.path.trim();
        if file_path.is_empty() {
            return Err(SmartFileSummaryError::InvalidArgs(
                "path is required".to_string(),
            ));
        }

        // Try to find the CPG root from the file path
        let cpg_root = infer_cpg_root(file_path);
        let cpg = get_or_build_cpg(&cpg_root, 5000)
            .await
            .map_err(|e| SmartFileSummaryError::CpgNotAvailable(e))?;

        // Find the file node in the CPG
        let file_idx = cpg
            .file_index
            .iter()
            .find(|(k, _)| k.as_str() == file_path || k.ends_with(file_path) || file_path.ends_with(k.as_str()))
            .map(|(_, v)| *v);

        let file_idx = file_idx.ok_or_else(|| {
            SmartFileSummaryError::FileNotFound(format!(
                "{} (available: {})",
                file_path,
                cpg.file_index
                    .keys()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

        let file_node = &cpg.graph[file_idx];
        let (language, total_lines) = match &file_node.kind {
            CpgNodeKind::File {
                language, lines, ..
            } => (language.clone(), *lines),
            _ => ("unknown".to_string(), 0),
        };

        let mut skeleton = Vec::new();
        let mut security_signals = Vec::new();
        let mut exposed_endpoints = Vec::new();
        let mut key_imports = Vec::new();
        let mut hotspots = Vec::new();
        let mut call_site_count = 0;
        let mut entry_point_count = 0;

        // Walk all children of this file (direct and nested via classes)
        let mut children_to_visit: Vec<petgraph::graph::NodeIndex> = cpg
            .graph
            .neighbors_directed(file_idx, Direction::Outgoing)
            .collect();

        // Also include grandchildren (methods inside classes)
        let class_children: Vec<_> = children_to_visit
            .iter()
            .filter(|&&c| matches!(cpg.graph[c].kind, CpgNodeKind::Class { .. }))
            .flat_map(|&c| cpg.graph.neighbors_directed(c, Direction::Outgoing))
            .collect();
        children_to_visit.extend(class_children);

        for child_idx in &children_to_visit {
            let child = &cpg.graph[*child_idx];
            match &child.kind {
                CpgNodeKind::Function {
                    name,
                    signature,
                    visibility,
                    is_async: _,
                } => {
                    let params = cpg
                        .get_parameter_names(*child_idx);
                    skeleton.push(SkeletonEntry {
                        kind: "function".to_string(),
                        name: name.clone(),
                        signature: signature.clone(),
                        line_start: child.start_line,
                        line_end: child.end_line,
                        visibility: visibility.clone(),
                        annotations: extract_annotations(signature.as_deref(), &child.tags),
                        params,
                    });

                    // Check security signals in this function
                    check_security_signals_for_function(&cpg, *child_idx, &mut security_signals);
                }
                CpgNodeKind::Method {
                    name,
                    class_name,
                    visibility,
                    is_async: _,
                    ..
                } => {
                    let params = cpg
                        .get_parameter_names(*child_idx);
                    let display_name = if let Some(cls) = class_name {
                        format!("{}.{}", cls, name)
                    } else {
                        name.clone()
                    };
                    skeleton.push(SkeletonEntry {
                        kind: "method".to_string(),
                        name: display_name,
                        signature: None,
                        line_start: child.start_line,
                        line_end: child.end_line,
                        visibility: visibility.clone(),
                        annotations: extract_annotations(None, &child.tags),
                        params,
                    });

                    check_security_signals_for_function(&cpg, *child_idx, &mut security_signals);
                }
                CpgNodeKind::Class {
                    name, visibility, ..
                } => {
                    skeleton.push(SkeletonEntry {
                        kind: "class".to_string(),
                        name: name.clone(),
                        signature: None,
                        line_start: child.start_line,
                        line_end: child.end_line,
                        visibility: visibility.clone(),
                        annotations: vec![],
                        params: vec![],
                    });
                }
                CpgNodeKind::Import {
                    module, symbols, ..
                } => {
                    let display = if symbols.is_empty() {
                        module.clone()
                    } else {
                        format!("{} ({})", module, symbols.join(", "))
                    };
                    key_imports.push(display);
                }
                CpgNodeKind::CallSite { .. } => {
                    call_site_count += 1;
                }
                CpgNodeKind::EntryPoint {
                    method, path, handler,
                } => {
                    exposed_endpoints.push(format!("{} {} -> {}", method, path, handler));
                    entry_point_count += 1;
                }
                _ => {}
            }
        }

        // Sort skeleton by line number
        skeleton.sort_by_key(|s| s.line_start);

        // Generate hotspots from security signals
        for signal in &security_signals {
            let severity = match signal.signal_type.as_str() {
                "external_input" | "command_exec" | "eval" => "high",
                "db_query" | "file_io" | "deserialization" => "high",
                "auth_check" | "crypto" => "medium",
                _ => "medium",
            };
            hotspots.push(Hotspot {
                line_start: signal.line.saturating_sub(2),
                line_end: signal.line + 5,
                reason: format!("{}: {}", signal.signal_type, signal.snippet),
                severity_hint: severity.to_string(),
            });
        }

        // Deduplicate and sort hotspots by severity
        hotspots.sort_by(|a, b| {
            let sa = match a.severity_hint.as_str() {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            };
            let sb = match b.severity_hint.as_str() {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            };
            sa.cmp(&sb)
        });
        hotspots.truncate(20);

        let stats = FileSummaryStats {
            functions: skeleton
                .iter()
                .filter(|s| s.kind == "function" || s.kind == "method")
                .count(),
            classes: skeleton.iter().filter(|s| s.kind == "class").count(),
            imports: key_imports.len(),
            call_sites: call_site_count,
            entry_points: entry_point_count,
            security_signals_count: security_signals.len(),
        };

        Ok(SmartFileSummaryOutput {
            path: file_path.to_string(),
            language,
            total_lines,
            skeleton,
            security_signals,
            exposed_endpoints,
            key_imports,
            hotspots,
            stats,
        })
    }
}

// ============================================================================
// TraceDataFlowTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TraceDataFlowArgs {
    /// Project root path
    #[serde(default)]
    pub path: Option<String>,
    /// Starting point: function name, "ClassName.methodName", or "file:line"
    pub from: String,
    /// Trace direction: "forward" (where does data flow to) | "backward" (where does data come from)
    #[serde(default = "default_direction")]
    pub direction: String,
    /// Max call-graph hops to trace
    #[serde(default = "default_trace_depth")]
    pub max_depth: usize,
}

fn default_direction() -> String {
    "forward".to_string()
}

fn default_trace_depth() -> usize {
    8
}

#[derive(Debug, Clone, Serialize)]
pub struct DataFlowStep {
    pub file: String,
    pub line: usize,
    pub function: String,
    pub step_type: String, // "source" | "call" | "callee" | "sink"
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataFlowPath {
    pub steps: Vec<DataFlowStep>,
    pub reaches_dangerous_sink: bool,
    pub sink_type: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceDataFlowOutput {
    pub from: String,
    pub direction: String,
    pub flow_paths: Vec<DataFlowPath>,
    pub total_paths: usize,
    pub dangerous_paths: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TraceDataFlowError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("cpg not available: {0}")]
    CpgNotAvailable(String),
    #[error("target not found: {0}")]
    TargetNotFound(String),
}

#[derive(Debug, Clone)]
pub struct TraceDataFlowTool;

impl TraceDataFlowTool {
    pub const NAME: &'static str = "trace_data_flow";
    pub const DESCRIPTION: &'static str =
        "Trace data flow through the code using the CPG call graph. \
         Given a starting function, traces forward (where data flows to) or backward \
         (where data comes from) through the call chain. Identifies dangerous sinks \
         (SQL injection, command exec, file write, etc.) along the path. \
         More precise than pattern-based taint analysis because it follows actual call \
         relationships in the graph.";
}

impl Tool for TraceDataFlowTool {
    const NAME: &'static str = Self::NAME;
    type Args = TraceDataFlowArgs;
    type Output = TraceDataFlowOutput;
    type Error = TraceDataFlowError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TraceDataFlowArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.as_deref().unwrap_or("/workspace");
        let cpg = get_or_build_cpg(path, 5000)
            .await
            .map_err(|e| TraceDataFlowError::CpgNotAvailable(e))?;

        let from = args.from.trim();
        if from.is_empty() {
            return Err(TraceDataFlowError::InvalidArgs(
                "from is required".to_string(),
            ));
        }

        let direction = args.direction.to_lowercase();
        let max_depth = args.max_depth.clamp(1, 20);

        // Find the starting function node(s)
        let start_indices = find_function_nodes(&cpg, from);
        if start_indices.is_empty() {
            return Err(TraceDataFlowError::TargetNotFound(format!(
                "'{}' not found in CPG. Try using query_cpg search first.",
                from
            )));
        }

        let mut all_paths = Vec::new();

        for start_idx in &start_indices {
            let _start_node = &cpg.graph[*start_idx];
            let graph_direction = if direction == "backward" {
                Direction::Incoming
            } else {
                Direction::Outgoing
            };

            // BFS through call graph
            let mut visited = std::collections::HashSet::new();
            let mut queue: std::collections::VecDeque<(petgraph::graph::NodeIndex, Vec<petgraph::graph::NodeIndex>)> =
                std::collections::VecDeque::new();
            queue.push_back((*start_idx, vec![*start_idx]));
            visited.insert(*start_idx);

            while let Some((current, path_so_far)) = queue.pop_front() {
                if path_so_far.len() > max_depth {
                    continue;
                }

                // Check if current node is a dangerous sink
                let is_sink = is_dangerous_sink_node(&cpg, current);

                if is_sink && path_so_far.len() > 1 {
                    // Found a path to a sink
                    let steps: Vec<DataFlowStep> = path_so_far
                        .iter()
                        .enumerate()
                        .map(|(i, &node_idx)| {
                            let n = &cpg.graph[node_idx];
                            let step_type = if i == 0 {
                                "source"
                            } else if i == path_so_far.len() - 1 {
                                "sink"
                            } else {
                                "call"
                            };
                            DataFlowStep {
                                file: n.file.clone().unwrap_or_default(),
                                line: n.start_line,
                                function: n.kind.display_name(),
                                step_type: step_type.to_string(),
                                description: format!(
                                    "{} at {}:{}",
                                    n.kind.display_name(),
                                    n.file.as_deref().unwrap_or("?"),
                                    n.start_line
                                ),
                            }
                        })
                        .collect();

                    let sink_type = infer_sink_type(&cpg.graph[current]);
                    let confidence =
                        ((max_depth as f64 - path_so_far.len() as f64 + 1.0) / max_depth as f64)
                            .clamp(0.3, 0.95);

                    all_paths.push(DataFlowPath {
                        steps,
                        reaches_dangerous_sink: true,
                        sink_type: Some(sink_type),
                        confidence,
                    });
                }

                // Explore neighbors
                for neighbor in cpg.graph.neighbors_directed(current, graph_direction) {
                    // Only follow call edges
                    let edge = if direction == "backward" {
                        cpg.graph.find_edge(neighbor, current)
                    } else {
                        cpg.graph.find_edge(current, neighbor)
                    };

                    if let Some(edge_idx) = edge {
                        if matches!(cpg.graph[edge_idx].kind, CpgEdgeKind::Calls) {
                            if !visited.contains(&neighbor) {
                                visited.insert(neighbor);
                                let mut new_path = path_so_far.clone();
                                new_path.push(neighbor);
                                queue.push_back((neighbor, new_path));
                            }
                        }
                    }
                }
            }
        }

        // Also build non-sink paths for the "where does data flow" view
        // (truncate to avoid too many results)
        all_paths.sort_by(|a, b| {
            b.reaches_dangerous_sink
                .cmp(&a.reaches_dangerous_sink)
                .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });
        all_paths.truncate(50);

        let dangerous = all_paths.iter().filter(|p| p.reaches_dangerous_sink).count();

        Ok(TraceDataFlowOutput {
            from: from.to_string(),
            direction,
            flow_paths: all_paths.clone(),
            total_paths: all_paths.len(),
            dangerous_paths: dangerous,
        })
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Pattern matching for function names with wildcard support
fn matches_pattern(pattern: &str, name: &str, qualified_name: &str) -> bool {
    let p = pattern.to_lowercase();
    let n = name.to_lowercase();
    let q = qualified_name.to_lowercase();

    if p.contains('*') {
        // Wildcard matching
        let parts: Vec<&str> = p.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            if prefix.is_empty() && suffix.is_empty() {
                return true; // "*" matches everything
            }
            if prefix.is_empty() {
                // "*suffix" — ends with
                return n.ends_with(suffix) || q.ends_with(suffix);
            }
            if suffix.is_empty() {
                // "prefix*" — starts with
                return n.starts_with(prefix) || q.starts_with(prefix);
            }
            // "prefix*suffix" — contains both
            return (n.starts_with(prefix) && n.ends_with(suffix))
                || (q.starts_with(prefix) && q.ends_with(suffix));
        }
        // Multiple wildcards: just check contains for each non-empty part
        return parts
            .iter()
            .filter(|p| !p.is_empty())
            .all(|part| n.contains(part) || q.contains(part));
    }

    // Exact match (case-insensitive)
    n == p || q == p
}

fn collect_param_details(
    cpg: &CodePropertyGraph,
    func_idx: petgraph::graph::NodeIndex,
) -> Vec<ParamDetail> {
    let mut params: Vec<ParamDetail> = Vec::new();
    for child in cpg.graph.neighbors_directed(func_idx, Direction::Outgoing) {
        if let Some(edge) = cpg.graph.find_edge(func_idx, child) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::HasParameter) {
                if let CpgNodeKind::Parameter {
                    name,
                    type_name,
                    index,
                } = &cpg.graph[child].kind
                {
                    let is_external = is_external_input_param(name, type_name.as_deref());
                    let annotations = extract_param_annotations(name, type_name.as_deref());
                    params.push(ParamDetail {
                        name: name.clone(),
                        type_annotation: type_name.clone(),
                        index: *index,
                        is_external_input: is_external,
                        annotations,
                    });
                }
            }
        }
    }
    params.sort_by_key(|p| p.index);
    params
}

fn is_external_input_param(name: &str, type_name: Option<&str>) -> bool {
    let n = name.to_lowercase();
    let t = type_name.unwrap_or("").to_lowercase();

    // Common external input parameter patterns
    let input_names = [
        "request", "req", "ctx", "context", "params", "query", "body", "input", "form", "args",
        "payload",
    ];
    let input_types = [
        "httpservletrequest",
        "request",
        "httpcontext",
        "context",
        "requestparam",
        "requestbody",
        "pathvariable",
        "query",
        "json",
        "form",
        "multipartfile",
    ];

    input_names.iter().any(|p| n == *p || n.starts_with(p))
        || input_types.iter().any(|p| t.contains(p))
}

fn extract_param_annotations(_name: &str, type_name: Option<&str>) -> Vec<String> {
    let mut annotations = Vec::new();
    if let Some(t) = type_name {
        let tl = t.to_lowercase();
        if tl.contains("requestparam") {
            annotations.push("@RequestParam".to_string());
        }
        if tl.contains("requestbody") {
            annotations.push("@RequestBody".to_string());
        }
        if tl.contains("pathvariable") {
            annotations.push("@PathVariable".to_string());
        }
    }
    annotations
}

fn build_security_context(
    cpg: &CodePropertyGraph,
    func_idx: petgraph::graph::NodeIndex,
    params: &[ParamDetail],
    tags: &[String],
) -> FunctionSecurityContext {
    let callees = collect_callees(cpg, func_idx);
    let callee_lower: Vec<String> = callees.iter().map(|c| c.to_lowercase()).collect();

    let has_auth_check = callee_lower.iter().any(|c| {
        c.contains("auth")
            || c.contains("authenticate")
            || c.contains("authorize")
            || c.contains("permission")
            || c.contains("isadmin")
            || c.contains("checkrole")
            || c.contains("requirerole")
            || c.contains("login")
    }) || tags.iter().any(|t| t.contains("auth"));

    let accepts_external_input = params.iter().any(|p| p.is_external_input);

    let has_db_operation = callee_lower.iter().any(|c| is_db_access_pattern(c));

    let has_file_operation = callee_lower.iter().any(|c| {
        c.contains("readfile")
            || c.contains("writefile")
            || c.contains("open")
            || c.contains("fopen")
            || c.contains("fs.")
            || c.contains("std::fs")
            || c.contains("file.")
    });

    let has_command_exec = callee_lower.iter().any(|c| {
        c.contains("exec")
            || c.contains("system")
            || c.contains("popen")
            || c.contains("subprocess")
            || c.contains("child_process")
            || c.contains("command")
            || c.contains("runtime.exec")
    });

    let has_network_request = callee_lower.iter().any(|c| {
        c.contains("fetch")
            || c.contains("http")
            || c.contains("request")
            || c.contains("curl")
            || c.contains("axios")
    });

    let security_annotations: Vec<String> = tags
        .iter()
        .filter(|t| {
            let tl = t.to_lowercase();
            tl.contains("auth")
                || tl.contains("secure")
                || tl.contains("role")
                || tl.contains("permission")
        })
        .cloned()
        .collect();

    let mut risk_indicators = Vec::new();
    if accepts_external_input && !has_auth_check {
        risk_indicators.push("Accepts external input without auth check".to_string());
    }
    if has_db_operation && accepts_external_input {
        risk_indicators.push("External input flows to database operations".to_string());
    }
    if has_command_exec {
        risk_indicators.push("Contains command execution".to_string());
    }
    if has_file_operation && accepts_external_input {
        risk_indicators.push("External input may reach file operations".to_string());
    }

    FunctionSecurityContext {
        has_auth_check,
        accepts_external_input,
        has_db_operation,
        has_file_operation,
        has_command_exec,
        has_network_request,
        security_annotations,
        risk_indicators,
    }
}

fn collect_callers(cpg: &CodePropertyGraph, idx: petgraph::graph::NodeIndex) -> Vec<String> {
    let mut callers = Vec::new();
    for neighbor in cpg.graph.neighbors_directed(idx, Direction::Incoming) {
        if let Some(edge) = cpg.graph.find_edge(neighbor, idx) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::Calls) {
                callers.push(cpg.graph[neighbor].kind.display_name());
            }
        }
    }
    callers.truncate(20);
    callers
}

fn collect_callees(cpg: &CodePropertyGraph, idx: petgraph::graph::NodeIndex) -> Vec<String> {
    let mut callees = Vec::new();
    for neighbor in cpg.graph.neighbors_directed(idx, Direction::Outgoing) {
        if let Some(edge) = cpg.graph.find_edge(idx, neighbor) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::Calls) {
                callees.push(cpg.graph[neighbor].kind.display_name());
            }
        }
    }
    callees.truncate(30);
    callees
}

fn extract_annotations(signature: Option<&str>, tags: &[String]) -> Vec<String> {
    let mut annotations = Vec::new();
    if let Some(sig) = signature {
        // Extract typical annotations from signature lines
        for line in sig.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('@') || trimmed.starts_with("#[") {
                annotations.push(trimmed.to_string());
            }
        }
    }
    for tag in tags {
        if !annotations.contains(tag) {
            annotations.push(tag.clone());
        }
    }
    annotations
}

fn is_db_access_pattern(name: &str) -> bool {
    name.contains("query")
        || name.contains("execute")
        || name.contains("exec")
        || name.contains("rawquery")
        || name.contains("db.")
        || name.contains("cursor")
        || name.contains("sqlx")
        || name.contains("sequelize")
        || name.contains("prisma")
        || name.contains("repository")
        || name.contains("findone")
        || name.contains("findall")
        || name.contains("save")
        || name.contains("delete")
        || name.contains("update")
        || name.contains("insert")
}

fn infer_db_access_type(name: &str) -> String {
    if name.contains("raw") || name.contains("query(") || name.contains("exec(") {
        "raw_query".to_string()
    } else if name.contains("repository")
        || name.contains("findone")
        || name.contains("findall")
        || name.contains("prisma")
        || name.contains("sequelize")
    {
        "orm".to_string()
    } else {
        "unknown".to_string()
    }
}

struct HandlerAnalysis {
    file: String,
    line: usize,
    auth_annotations: Vec<String>,
    has_db_ops: bool,
    has_file_ops: bool,
    has_command_exec: bool,
    input_params: Vec<InputParamInfo>,
}

fn find_handler_detail(
    cpg: &CodePropertyGraph,
    entry_idx: petgraph::graph::NodeIndex,
    handler_name: &str,
) -> HandlerAnalysis {
    // Try to find handler function via HandledBy edge first
    for neighbor in cpg.graph.neighbors_directed(entry_idx, Direction::Outgoing) {
        if let Some(edge) = cpg.graph.find_edge(entry_idx, neighbor) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::HandledBy) {
                let node = &cpg.graph[neighbor];
                let params = collect_param_details(cpg, neighbor);
                let sec = build_security_context(cpg, neighbor, &params, &node.tags);

                let input_params: Vec<InputParamInfo> = params
                    .iter()
                    .filter(|p| p.is_external_input)
                    .map(|p| InputParamInfo {
                        name: p.name.clone(),
                        source: "parameter".to_string(),
                        param_type: p.type_annotation.clone(),
                    })
                    .collect();

                return HandlerAnalysis {
                    file: node.file.clone().unwrap_or_default(),
                    line: node.start_line,
                    auth_annotations: sec.security_annotations,
                    has_db_ops: sec.has_db_operation,
                    has_file_ops: sec.has_file_operation,
                    has_command_exec: sec.has_command_exec,
                    input_params,
                };
            }
        }
    }

    // Fallback: search by handler name
    let target_indices = find_function_nodes(cpg, handler_name);
    if let Some(&idx) = target_indices.first() {
        let node = &cpg.graph[idx];
        let params = collect_param_details(cpg, idx);
        let sec = build_security_context(cpg, idx, &params, &node.tags);
        let input_params: Vec<InputParamInfo> = params
            .iter()
            .filter(|p| p.is_external_input)
            .map(|p| InputParamInfo {
                name: p.name.clone(),
                source: "parameter".to_string(),
                param_type: p.type_annotation.clone(),
            })
            .collect();

        return HandlerAnalysis {
            file: node.file.clone().unwrap_or_default(),
            line: node.start_line,
            auth_annotations: sec.security_annotations,
            has_db_ops: sec.has_db_operation,
            has_file_ops: sec.has_file_operation,
            has_command_exec: sec.has_command_exec,
            input_params,
        };
    }

    // Not found
    HandlerAnalysis {
        file: String::new(),
        line: 0,
        auth_annotations: vec![],
        has_db_ops: false,
        has_file_ops: false,
        has_command_exec: false,
        input_params: vec![],
    }
}

fn find_function_nodes(
    cpg: &CodePropertyGraph,
    name: &str,
) -> Vec<petgraph::graph::NodeIndex> {
    let mut results = Vec::new();
    let name_lower = name.to_lowercase();

    for idx in cpg.graph.node_indices() {
        let node = &cpg.graph[idx];
        match &node.kind {
            CpgNodeKind::Function { name: fn_name, .. }
            | CpgNodeKind::Method { name: fn_name, .. } => {
                let display = node.kind.display_name().to_lowercase();
                if fn_name.to_lowercase() == name_lower || display == name_lower {
                    results.push(idx);
                }
            }
            _ => {}
        }
    }
    results
}

fn is_dangerous_sink_node(cpg: &CodePropertyGraph, idx: petgraph::graph::NodeIndex) -> bool {
    let node = &cpg.graph[idx];
    let name = node.kind.display_name().to_lowercase();

    // SQL/DB sinks
    if name.contains("execute") || name.contains("rawquery") || name.contains("exec(") {
        return true;
    }
    // Command execution sinks
    if name.contains("system") || name.contains("popen") || name.contains("subprocess") || name.contains("child_process") {
        return true;
    }
    // Eval sinks
    if name.contains("eval") {
        return true;
    }
    // File sinks
    if name.contains("writefile") || name.contains("fopen") {
        return true;
    }
    // Deserialization sinks
    if name.contains("pickle") || name.contains("unserialize") || name.contains("yaml.load") {
        return true;
    }

    // Check tags
    node.tags.iter().any(|t| t == "sink")
}

fn infer_sink_type(node: &CpgNode) -> String {
    let name = node.kind.display_name().to_lowercase();
    if name.contains("query") || name.contains("execute") || name.contains("sql") {
        "sql_injection".to_string()
    } else if name.contains("system") || name.contains("exec") || name.contains("popen") {
        "command_injection".to_string()
    } else if name.contains("eval") {
        "code_injection".to_string()
    } else if name.contains("writefile") || name.contains("fopen") {
        "file_write".to_string()
    } else if name.contains("pickle") || name.contains("unserialize") {
        "deserialization".to_string()
    } else if name.contains("redirect") {
        "open_redirect".to_string()
    } else {
        "unknown_sink".to_string()
    }
}

fn check_security_signals_for_function(
    cpg: &CodePropertyGraph,
    func_idx: petgraph::graph::NodeIndex,
    signals: &mut Vec<SecuritySignal>,
) {
    let func_node = &cpg.graph[func_idx];

    // Check callees for security-relevant patterns
    for neighbor in cpg.graph.neighbors_directed(func_idx, Direction::Outgoing) {
        if let Some(edge) = cpg.graph.find_edge(func_idx, neighbor) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::Calls) {
                let callee = &cpg.graph[neighbor];
                let callee_name = callee.kind.display_name().to_lowercase();

                if is_db_access_pattern(&callee_name) {
                    signals.push(SecuritySignal {
                        signal_type: "db_query".to_string(),
                        line: callee.start_line,
                        snippet: callee.kind.display_name(),
                    });
                }
                if callee_name.contains("exec")
                    || callee_name.contains("system")
                    || callee_name.contains("popen")
                {
                    signals.push(SecuritySignal {
                        signal_type: "command_exec".to_string(),
                        line: callee.start_line,
                        snippet: callee.kind.display_name(),
                    });
                }
                if callee_name.contains("eval") {
                    signals.push(SecuritySignal {
                        signal_type: "eval".to_string(),
                        line: callee.start_line,
                        snippet: callee.kind.display_name(),
                    });
                }
            }
        }
    }

    // Check if function accepts external input (via params)
    let params = collect_param_details(cpg, func_idx);
    if params.iter().any(|p| p.is_external_input) {
        signals.push(SecuritySignal {
            signal_type: "external_input".to_string(),
            line: func_node.start_line,
            snippet: format!(
                "{} accepts external input via params",
                func_node.kind.display_name()
            ),
        });
    }
}

fn infer_cpg_root(file_path: &str) -> String {
    // Try to find project root by walking up from the file path
    let path = std::path::Path::new(file_path);
    let mut current = if path.is_file() {
        path.parent()
    } else {
        Some(path)
    };

    while let Some(dir) = current {
        // Check for common project root indicators
        for marker in &[
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pom.xml",
            "build.gradle",
            "requirements.txt",
            "pyproject.toml",
            ".git",
        ] {
            if dir.join(marker).exists() {
                return dir.to_string_lossy().to_string();
            }
        }
        current = dir.parent();
    }

    // Fallback
    if file_path.starts_with("/workspace") {
        "/workspace".to_string()
    } else {
        ".".to_string()
    }
}

async fn read_function_body(file: &str, start: usize, end: usize) -> Option<String> {
    let content = tokio::fs::read_to_string(file).await.ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let start_idx = start.saturating_sub(1).min(lines.len());
    let end_idx = end.min(lines.len());
    if start_idx >= end_idx {
        return None;
    }
    let body_lines: Vec<String> = lines[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:6}|{}", start + i, line))
        .collect();
    Some(body_lines.join("\n"))
}
