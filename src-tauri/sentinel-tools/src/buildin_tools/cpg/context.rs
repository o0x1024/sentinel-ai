//! CPG context generator — produces a compact code structure briefing
//! for injection into the AI's system prompt during audit mode.
//!
//! This gives the AI a "bird's eye view" of the project at the start
//! of the conversation, drastically reducing the need for exploratory
//! tool calls to understand the codebase.

use crate::buildin_tools::cpg::types::*;
use crate::buildin_tools::cpg::security_rules;
use crate::buildin_tools::cpg::taint;

/// Generate a compact project briefing from an existing CPG,
/// suitable for injection into the system prompt.
///
/// Target: ~600-1200 tokens of high-signal context.
pub fn generate_audit_context(cpg: &CodePropertyGraph) -> String {
    let summary = cpg.summary();
    let mut out = String::with_capacity(4096);

    // ── Project overview ────────────────────────────────────────────────
    out.push_str("<code_structure>\n");
    out.push_str(&format!(
        "Project: {} ({}, {} files, {} functions, {} classes)\n",
        &summary.root
            .rsplit('/')
            .next()
            .unwrap_or(&summary.root),
        summary.primary_language,
        summary.total_files,
        summary.total_functions,
        summary.total_classes,
    ));

    if summary.languages.len() > 1 {
        out.push_str(&format!(
            "Languages: {}\n",
            summary.languages.join(", ")
        ));
    }

    out.push_str(&format!(
        "Graph: {} nodes, {} edges, {} call edges\n\n",
        summary.total_nodes,
        summary.total_edges,
        summary.total_call_edges,
    ));

    // ── Top files (by function count) ───────────────────────────────────
    let files = cpg.list_file_summaries(15);
    if !files.is_empty() {
        out.push_str("Key Files (by complexity):\n");
        for f in files.iter().take(15) {
            let markers = if f.functions > 10 { " ⚠ complex" } else { "" };
            out.push_str(&format!(
                "  {} ({}, {}L, {} fn, {} cls){}\n",
                f.path, f.language, f.lines, f.functions, f.classes, markers,
            ));
        }
        out.push('\n');
    }

    // ── Public API / Entry points ───────────────────────────────────────
    let entry_points: Vec<_> = cpg
        .graph
        .node_weights()
        .filter(|n| matches!(n.kind, CpgNodeKind::EntryPoint { .. }))
        .collect();

    if !entry_points.is_empty() {
        out.push_str("Entry Points:\n");
        for ep in entry_points.iter().take(20) {
            if let CpgNodeKind::EntryPoint { method, path, handler } = &ep.kind {
                out.push_str(&format!(
                    "  {} {} → {} ({}:{})\n",
                    method,
                    path,
                    handler,
                    ep.file.as_deref().unwrap_or("?"),
                    ep.start_line,
                ));
            }
        }
        out.push('\n');
    }

    // ── Key functions (high fan-out = complex, high fan-in = critical) ──
    let call_edges = cpg.list_call_edges(500);
    if !call_edges.is_empty() {
        // Fan-in analysis: most-called functions
        let mut fan_in: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for edge in &call_edges {
            *fan_in.entry(edge.callee.clone()).or_insert(0) += 1;
        }
        let mut most_called: Vec<_> = fan_in.into_iter().collect();
        most_called.sort_by(|a, b| b.1.cmp(&a.1));

        if !most_called.is_empty() {
            out.push_str("Most-Called Functions (high fan-in → critical):\n");
            for (name, count) in most_called.iter().take(10) {
                out.push_str(&format!("  {} (called {} times)\n", name, count));
            }
            out.push('\n');
        }

        // Fan-out analysis: functions that call the most others
        let mut fan_out: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for edge in &call_edges {
            *fan_out.entry(edge.caller.clone()).or_insert(0) += 1;
        }
        let mut most_calling: Vec<_> = fan_out.into_iter().collect();
        most_calling.sort_by(|a, b| b.1.cmp(&a.1));

        if !most_calling.is_empty() {
            out.push_str("Most-Complex Functions (high fan-out):\n");
            for (name, count) in most_calling.iter().take(8) {
                out.push_str(&format!("  {} (calls {} functions)\n", name, count));
            }
            out.push('\n');
        }
    }

    // ── Quick security baseline (fast pattern scan) ─────────────────────
    let all_rules = security_rules::all_rules();
    let quick_result = taint::run_taint_analysis(cpg, &all_rules, 4, 10);

    if quick_result.total_findings > 0 {
        out.push_str("⚠ Auto-detected High-Risk Data Flows:\n");

        // Group by rule
        let mut by_rule: std::collections::HashMap<String, Vec<&taint::TaintFinding>> =
            std::collections::HashMap::new();
        for finding in &quick_result.findings {
            by_rule
                .entry(finding.rule_id.clone())
                .or_default()
                .push(finding);
        }

        for (_rule_id, findings) in &by_rule {
            let unsanitized = findings.iter().filter(|f| !f.sanitized).count();
            if unsanitized > 0 {
                let first = &findings[0];
                out.push_str(&format!(
                    "  {} ({}, {}) — {} flow(s), {} unsanitized\n",
                    first.rule_name,
                    first.cwe,
                    first.severity,
                    findings.len(),
                    unsanitized,
                ));
                // Show first few traces
                for f in findings.iter().filter(|f| !f.sanitized).take(3) {
                    out.push_str(&format!(
                        "    {} ({}:{}) → {} ({}:{})\n",
                        f.source.name,
                        f.source.file,
                        f.source.line,
                        f.sink.name,
                        f.sink.file,
                        f.sink.line,
                    ));
                }
            }
        }
        out.push('\n');
    }

    // ── Import summary (external dependencies) ──────────────────────────
    let imports = cpg.list_imports(200);
    if !imports.is_empty() {
        // Deduplicate modules
        let mut unique_modules: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut security_relevant: Vec<String> = Vec::new();

        for imp in &imports {
            let module = &imp.module;
            unique_modules.insert(module.clone());

            // Flag security-relevant imports
            let lower = module.to_lowercase();
            if lower.contains("crypto")
                || lower.contains("auth")
                || lower.contains("jwt")
                || lower.contains("session")
                || lower.contains("sql")
                || lower.contains("exec")
                || lower.contains("eval")
                || lower.contains("serialize")
                || lower.contains("xml")
                || lower.contains("ldap")
                || lower.contains("password")
                || lower.contains("hash")
                || lower.contains("tls")
                || lower.contains("ssl")
            {
                security_relevant.push(module.clone());
            }
        }

        if !security_relevant.is_empty() {
            out.push_str("Security-Relevant Imports:\n");
            let mut seen = std::collections::HashSet::new();
            for module in &security_relevant {
                if seen.insert(module.clone()) {
                    out.push_str(&format!("  {}\n", module));
                }
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "Total unique imports: {}\n",
            unique_modules.len()
        ));
    }

    out.push_str("</code_structure>\n");

    // Limit total size to ~4000 chars (~1000 tokens)
    if out.len() > 4000 {
        out.truncate(3950);
        out.push_str("\n... (truncated)\n</code_structure>\n");
    }

    out
}

/// Generate a one-line CPG availability notice for system prompt.
pub fn cpg_availability_notice(root: &str) -> String {
    format!(
        "\n[CPG Available: A Code Property Graph has been built for '{}'. \
         Use `query_cpg` to explore code structure, `cpg_taint_analysis` for \
         source→sink tracing, and `cpg_security_scan` for baseline assessment.]\n",
        root.rsplit('/').next().unwrap_or(root)
    )
}
