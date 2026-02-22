//! Graph-based taint analysis using BFS/DFS on the CPG.
//!
//! Instead of the regex+line-distance heuristic in `taint_slice_lite`,
//! this module uses the actual call graph edges in the CPG to trace
//! data flow from sources to sinks, with sanitizer detection.

use petgraph::Direction;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};

use super::security_rules::{PatternSpec, SecurityRule};
use super::types::*;

// ── Taint finding types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TaintFinding {
    pub rule_id: String,
    pub rule_name: String,
    pub cwe: String,
    pub severity: String,
    pub source: TaintLocation,
    pub sink: TaintLocation,
    /// Intermediate call chain from source to sink
    pub trace_path: Vec<TraceHop>,
    /// Graph distance (number of hops)
    pub distance: usize,
    /// Was a sanitizer found on the path?
    pub sanitized: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaintLocation {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub kind: String, // "source" or "sink"
    pub pattern_matched: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceHop {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub hop_type: String, // "call", "contains", "parameter"
}

#[derive(Debug, Clone, Serialize)]
pub struct TaintAnalysisResult {
    pub total_sources: usize,
    pub total_sinks: usize,
    pub total_findings: usize,
    pub unsanitized_findings: usize,
    pub findings: Vec<TaintFinding>,
    pub rules_checked: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityScanResult {
    pub total_rules: usize,
    pub total_findings: usize,
    pub by_severity: SeverityBreakdown,
    pub findings: Vec<TaintFinding>,
    pub pattern_findings: Vec<PatternFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeverityBreakdown {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

/// A non-taint pattern-match finding (e.g., hardcoded secrets).
#[derive(Debug, Clone, Serialize)]
pub struct PatternFinding {
    pub rule_id: String,
    pub rule_name: String,
    pub cwe: String,
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub name: String,
    pub description: String,
}

// ── Taint analysis engine ───────────────────────────────────────────────────

/// Run taint analysis on the CPG for a set of security rules.
pub fn run_taint_analysis(
    cpg: &CodePropertyGraph,
    rules: &[SecurityRule],
    max_depth: usize,
    max_findings_per_rule: usize,
) -> TaintAnalysisResult {
    let language = &cpg.primary_language;
    let mut all_findings = Vec::new();
    let mut rules_checked = Vec::new();
    let mut total_sources = 0;
    let mut total_sinks = 0;

    for rule in rules {
        // Skip rules with empty sources/sinks (non-taint rules like hardcoded_secrets)
        if rule.sources.is_empty() || rule.sinks.is_empty() {
            continue;
        }

        rules_checked.push(rule.id.clone());

        // Find source nodes
        let sources = find_matching_nodes(cpg, &rule.sources, language);
        // Find sink nodes
        let sinks = find_matching_nodes(cpg, &rule.sinks, language);

        total_sources += sources.len();
        total_sinks += sinks.len();

        // For each source, BFS to find reachable sinks
        let mut rule_findings = Vec::new();
        for &(src_idx, ref src_pattern) in &sources {
            if rule_findings.len() >= max_findings_per_rule {
                break;
            }

            let src_node = &cpg.graph[src_idx];
            let src_func = find_containing_function(cpg, src_idx);

            // BFS from source's containing function
            let start_idx = src_func.unwrap_or(src_idx);
            let reachable = bfs_call_graph(cpg, start_idx, max_depth);

            for &(sink_idx, ref sink_pattern) in &sinks {
                let sink_node = &cpg.graph[sink_idx];
                let sink_func = find_containing_function(cpg, sink_idx);
                let sink_search_idx = sink_func.unwrap_or(sink_idx);

                // Check if sink is reachable from source
                if let Some(path) = reachable.iter().find(|p| p.last() == Some(&sink_search_idx)) {
                    // Check for sanitizers on the path
                    let sanitized = check_sanitizers_on_path(cpg, path, &rule.sanitizers, language);

                    // Build trace path
                    let trace = build_trace(cpg, path);
                    let distance = path.len().saturating_sub(1);

                    // Calculate confidence
                    let confidence = calculate_confidence(distance, sanitized, max_depth);

                    rule_findings.push(TaintFinding {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        cwe: rule.cwe.clone(),
                        severity: rule.severity.label().to_string(),
                        source: TaintLocation {
                            name: src_node.kind.display_name(),
                            file: src_node.file.clone().unwrap_or_default(),
                            line: src_node.start_line,
                            kind: "source".to_string(),
                            pattern_matched: src_pattern.clone(),
                        },
                        sink: TaintLocation {
                            name: sink_node.kind.display_name(),
                            file: sink_node.file.clone().unwrap_or_default(),
                            line: sink_node.start_line,
                            kind: "sink".to_string(),
                            pattern_matched: sink_pattern.clone(),
                        },
                        trace_path: trace,
                        distance,
                        sanitized,
                        confidence,
                    });
                }
                // Also check same-function (intra-procedural) taint
                else if same_function_taint(cpg, src_idx, sink_idx) {
                    let sanitized = false; // simplified for same-function
                    rule_findings.push(TaintFinding {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        cwe: rule.cwe.clone(),
                        severity: rule.severity.label().to_string(),
                        source: TaintLocation {
                            name: src_node.kind.display_name(),
                            file: src_node.file.clone().unwrap_or_default(),
                            line: src_node.start_line,
                            kind: "source".to_string(),
                            pattern_matched: src_pattern.clone(),
                        },
                        sink: TaintLocation {
                            name: sink_node.kind.display_name(),
                            file: sink_node.file.clone().unwrap_or_default(),
                            line: sink_node.start_line,
                            kind: "sink".to_string(),
                            pattern_matched: sink_pattern.clone(),
                        },
                        trace_path: vec![],
                        distance: 0,
                        sanitized,
                        confidence: 0.7, // intra-procedural is heuristic
                    });
                }
            }
        }

        all_findings.extend(rule_findings);
    }

    // Sort by severity, then by confidence
    all_findings.sort_by(|a, b| {
        severity_ord(&a.severity)
            .cmp(&severity_ord(&b.severity))
            .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
    });

    let unsanitized = all_findings.iter().filter(|f| !f.sanitized).count();

    TaintAnalysisResult {
        total_sources,
        total_sinks,
        total_findings: all_findings.len(),
        unsanitized_findings: unsanitized,
        findings: all_findings,
        rules_checked,
    }
}

/// Run a full security scan (taint analysis + pattern matching).
pub fn run_security_scan(
    cpg: &CodePropertyGraph,
    rules: &[SecurityRule],
    max_depth: usize,
    max_findings: usize,
) -> SecurityScanResult {
    // 1. Run taint analysis
    let taint_result = run_taint_analysis(cpg, rules, max_depth, max_findings);

    // 2. Run pattern-based checks (non-taint rules)
    let pattern_findings = run_pattern_checks(cpg, rules);

    // 3. Count by severity
    let mut breakdown = SeverityBreakdown {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        info: 0,
    };

    for f in &taint_result.findings {
        match f.severity.as_str() {
            "critical" => breakdown.critical += 1,
            "high" => breakdown.high += 1,
            "medium" => breakdown.medium += 1,
            "low" => breakdown.low += 1,
            _ => breakdown.info += 1,
        }
    }
    for f in &pattern_findings {
        match f.severity.as_str() {
            "critical" => breakdown.critical += 1,
            "high" => breakdown.high += 1,
            "medium" => breakdown.medium += 1,
            "low" => breakdown.low += 1,
            _ => breakdown.info += 1,
        }
    }

    let total_findings = taint_result.total_findings + pattern_findings.len();

    SecurityScanResult {
        total_rules: rules.len(),
        total_findings,
        by_severity: breakdown,
        findings: taint_result.findings,
        pattern_findings,
    }
}

// ── Core algorithms ─────────────────────────────────────────────────────────

/// Find CPG nodes (CallSites, Functions, Methods) matching source/sink patterns.
fn find_matching_nodes(
    cpg: &CodePropertyGraph,
    patterns: &[PatternSpec],
    language: &str,
) -> Vec<(petgraph::graph::NodeIndex, String)> {
    let mut results = Vec::new();
    for idx in cpg.graph.node_indices() {
        let node = &cpg.graph[idx];
        let name = node.kind.display_name();
        if name.is_empty() {
            continue;
        }
        for pattern in patterns {
            if !pattern.applies_to_language(language) {
                continue;
            }
            if pattern.matches(&name) {
                results.push((idx, pattern.name_pattern.clone()));
                break; // One match per node is enough
            }
        }
    }
    results
}

/// BFS on the call graph from a start node, returning all paths up to max_depth.
fn bfs_call_graph(
    cpg: &CodePropertyGraph,
    start: petgraph::graph::NodeIndex,
    max_depth: usize,
) -> Vec<Vec<petgraph::graph::NodeIndex>> {
    let mut paths = Vec::new();
    let mut queue: VecDeque<Vec<petgraph::graph::NodeIndex>> = VecDeque::new();
    queue.push_back(vec![start]);

    let mut visited = HashSet::new();
    visited.insert(start);

    while let Some(path) = queue.pop_front() {
        if path.len() > max_depth + 1 {
            continue;
        }

        let current = *path.last().unwrap();

        // Follow Calls edges and Contains edges for deeper traversal
        for neighbor in cpg.graph.neighbors_directed(current, Direction::Outgoing) {
            if visited.contains(&neighbor) {
                continue;
            }

            if let Some(edge) = cpg.graph.find_edge(current, neighbor) {
                let edge_kind = &cpg.graph[edge].kind;
                if matches!(edge_kind, CpgEdgeKind::Calls | CpgEdgeKind::Contains) {
                    visited.insert(neighbor);
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    paths.push(new_path.clone());
                    if new_path.len() <= max_depth {
                        queue.push_back(new_path);
                    }
                }
            }
        }
    }

    paths
}

/// Check if any node on the path matches a sanitizer pattern.
fn check_sanitizers_on_path(
    cpg: &CodePropertyGraph,
    path: &[petgraph::graph::NodeIndex],
    sanitizers: &[PatternSpec],
    language: &str,
) -> bool {
    if sanitizers.is_empty() {
        return false;
    }

    for &idx in path {
        let node = &cpg.graph[idx];
        let name = node.kind.display_name();

        for sanitizer in sanitizers {
            if sanitizer.applies_to_language(language) && sanitizer.matches(&name) {
                return true;
            }
        }

        // Also check children (call sites within the function)
        for child in cpg.graph.neighbors_directed(idx, Direction::Outgoing) {
            let child_name = cpg.graph[child].kind.display_name();
            for sanitizer in sanitizers {
                if sanitizer.applies_to_language(language) && sanitizer.matches(&child_name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if source and sink exist in the same function/scope.
fn same_function_taint(
    cpg: &CodePropertyGraph,
    source_idx: petgraph::graph::NodeIndex,
    sink_idx: petgraph::graph::NodeIndex,
) -> bool {
    let src_node = &cpg.graph[source_idx];
    let sink_node = &cpg.graph[sink_idx];

    // Same file check
    if src_node.file != sink_node.file {
        return false;
    }

    // Find containing functions
    let src_func = find_containing_function(cpg, source_idx);
    let sink_func = find_containing_function(cpg, sink_idx);

    match (src_func, sink_func) {
        (Some(sf), Some(skf)) => sf == skf && src_node.start_line <= sink_node.start_line,
        _ => {
            // Fallback: line distance heuristic within same file
            src_node.start_line <= sink_node.start_line
                && (sink_node.start_line - src_node.start_line) < 50
        }
    }
}

/// Build a human-readable trace from a graph path.
fn build_trace(
    cpg: &CodePropertyGraph,
    path: &[petgraph::graph::NodeIndex],
) -> Vec<TraceHop> {
    path.iter()
        .map(|&idx| {
            let node = &cpg.graph[idx];
            let hop_type = match &node.kind {
                CpgNodeKind::CallSite { .. } => "call",
                CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. } => "function",
                CpgNodeKind::Parameter { .. } => "parameter",
                _ => "step",
            };
            TraceHop {
                name: node.kind.display_name(),
                file: node.file.clone().unwrap_or_default(),
                line: node.start_line,
                hop_type: hop_type.to_string(),
            }
        })
        .collect()
}

/// Calculate confidence score based on distance and sanitization.
fn calculate_confidence(distance: usize, sanitized: bool, max_depth: usize) -> f64 {
    let base = if distance == 0 {
        0.95 // Direct, same function
    } else if distance == 1 {
        0.90 // One hop
    } else if distance <= 3 {
        0.80
    } else {
        // Decay for longer paths
        (0.70 * (1.0 - (distance as f64 / max_depth as f64).min(1.0))).max(0.30)
    };

    if sanitized {
        base * 0.3 // Heavily penalize sanitized paths
    } else {
        base
    }
}

/// Run pattern-match checks for non-taint rules (e.g., hardcoded secrets).
fn run_pattern_checks(
    cpg: &CodePropertyGraph,
    rules: &[SecurityRule],
) -> Vec<PatternFinding> {
    let mut findings = Vec::new();

    for rule in rules {
        // Only run for non-taint rules
        if !rule.sources.is_empty() || !rule.sinks.is_empty() {
            continue;
        }

        if rule.id == "hardcoded_secrets" {
            // Look for string literals that look like secrets
            for idx in cpg.graph.node_indices() {
                let node = &cpg.graph[idx];
                if let CpgNodeKind::StringLiteral { value } = &node.kind {
                    if looks_like_secret(value) {
                        findings.push(PatternFinding {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            cwe: rule.cwe.clone(),
                            severity: rule.severity.label().to_string(),
                            file: node.file.clone().unwrap_or_default(),
                            line: node.start_line,
                            name: format!("\"{}\"", truncate_str(value, 20)),
                            description: rule.description.clone(),
                        });
                    }
                }

                // Check variable names that suggest secrets
                if let CpgNodeKind::Variable { name, .. } = &node.kind {
                    let lower = name.to_lowercase();
                    if lower.contains("password")
                        || lower.contains("secret")
                        || lower.contains("api_key")
                        || lower.contains("apikey")
                        || lower.contains("token")
                        || lower.contains("private_key")
                    {
                        findings.push(PatternFinding {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            cwe: rule.cwe.clone(),
                            severity: "medium".to_string(), // Lower severity for just variable names
                            file: node.file.clone().unwrap_or_default(),
                            line: node.start_line,
                            name: name.clone(),
                            description: format!(
                                "Variable '{}' suggests sensitive data. Verify it's not hardcoded.",
                                name
                            ),
                        });
                    }
                }
            }
        }
    }

    findings
}

/// Walk parents to find the containing function.
fn find_containing_function(
    cpg: &CodePropertyGraph,
    node_idx: petgraph::graph::NodeIndex,
) -> Option<petgraph::graph::NodeIndex> {
    for parent in cpg.graph.neighbors_directed(node_idx, Direction::Incoming) {
        if let Some(edge) = cpg.graph.find_edge(parent, node_idx) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::Contains) {
                match &cpg.graph[parent].kind {
                    CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. } => {
                        return Some(parent);
                    }
                    _ => return find_containing_function(cpg, parent),
                }
            }
        }
    }
    None
}

fn severity_ord(s: &str) -> u8 {
    match s {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn looks_like_secret(value: &str) -> bool {
    if value.len() < 16 || value.len() > 512 {
        return false;
    }
    // Check entropy-like patterns
    let has_upper = value.chars().any(|c| c.is_uppercase());
    let has_lower = value.chars().any(|c| c.is_lowercase());
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    let has_special = value.chars().any(|c| !c.is_alphanumeric() && c != ' ');

    // High entropy strings (API keys, tokens)
    if has_upper && has_lower && has_digit && has_special {
        return true;
    }

    // Common secret prefixes
    let lower = value.to_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("pk-")
        || lower.starts_with("ghp_")
        || lower.starts_with("glpat-")
        || lower.starts_with("xox")
        || lower.starts_with("ey")  // JWT-like
        || lower.contains("bearer ")
        || lower.contains("basic ")
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}
