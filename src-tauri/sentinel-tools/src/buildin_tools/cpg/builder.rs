//! CPG builder — walks source files, parses with tree-sitter, populates the graph.

use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::parser::{self, Language};
use super::types::*;

/// Directories to skip during scanning.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    "vendor",
    "coverage",
    ".idea",
    ".vscode",
    ".DS_Store",
    "bin",
    "obj",
];

/// Max source file size to parse (2 MB).
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

/// Build a CPG from a project root (blocking I/O — intended to run via `spawn_blocking`).
pub fn build_cpg(root: &str, max_files: usize) -> Result<CodePropertyGraph, String> {
    let root_path = PathBuf::from(root);
    if !root_path.is_dir() {
        return Err(format!("Not a directory: {}", root));
    }

    let mut cpg = CodePropertyGraph::new(root.to_string());
    let mut lang_counts: HashMap<String, usize> = HashMap::new();

    let mut file_count = 0;
    for entry in WalkDir::new(&root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                return !SKIP_DIRS.contains(&name.as_ref());
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if file_count >= max_files {
            break;
        }

        let path = entry.path();
        let lang = match Language::from_path(path) {
            Some(l) => l,
            None => continue,
        };

        // Skip huge files.
        let fsize = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if fsize > MAX_FILE_SIZE || fsize == 0 {
            continue;
        }

        let source = match std::fs::read(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let rel_path = path
            .strip_prefix(&root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let line_count = bytecount_lines(&source);

        // Record language stats
        *lang_counts.entry(lang.label().to_string()).or_insert(0) += 1;

        // Parse with tree-sitter
        let tree = match parser::parse_source(&source, lang) {
            Some(t) => t,
            None => continue,
        };

        // Add file node
        let file_idx = cpg.add_node(CpgNode {
            kind: CpgNodeKind::File {
                path: rel_path.clone(),
                language: lang.label().to_string(),
                lines: line_count,
            },
            file: Some(rel_path.clone()),
            start_line: 1,
            end_line: line_count,
            tags: Vec::new(),
        });
        cpg.file_index.insert(rel_path.clone(), file_idx);

        // Walk AST and populate CPG
        let root_node = tree.root_node();
        walk_ast(&mut cpg, &source, &rel_path, lang, root_node, file_idx);

        file_count += 1;
    }

    // Determine primary language
    if let Some((primary, _)) = lang_counts.iter().max_by_key(|&(_, count)| count) {
        cpg.primary_language = primary.clone();
    }
    cpg.languages = {
        let mut langs: Vec<(String, usize)> = lang_counts.into_iter().collect();
        langs.sort_by(|a, b| b.1.cmp(&a.1));
        langs.into_iter().map(|(l, _)| l).collect()
    };

    // Post-processing: resolve intra-project call edges
    resolve_call_edges(&mut cpg);

    Ok(cpg)
}

// ── AST walking ─────────────────────────────────────────────────────────────

fn walk_ast(
    cpg: &mut CodePropertyGraph,
    source: &[u8],
    file_path: &str,
    lang: Language,
    node: tree_sitter::Node,
    parent_idx: NodeIndex,
) {
    let kind = node.kind();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let func_kinds = lang.function_node_kinds();
    let class_kinds = lang.class_node_kinds();
    let import_kinds = lang.import_node_kinds();
    let call_kinds = lang.call_node_kinds();

    // ── Function definitions ────────────────────────────────────────────
    if func_kinds.contains(&kind) {
        if let Some(name) = extract_function_name(node, source, lang) {
            let (is_method, class_name) = detect_method_context(node, source, lang);
            let is_async = detect_async(node, source, lang);
            let visibility = extract_visibility(node, source, lang);

            let node_kind = if is_method {
                CpgNodeKind::Method {
                    name: name.clone(),
                    class_name,
                    is_static: detect_static(node, source, lang),
                    is_async,
                    visibility,
                }
            } else {
                CpgNodeKind::Function {
                    name: name.clone(),
                    signature: extract_signature(node, source),
                    visibility,
                    is_async,
                }
            };

            let func_idx = cpg.add_node(CpgNode {
                kind: node_kind,
                file: Some(file_path.to_string()),
                start_line,
                end_line,
                tags: Vec::new(),
            });
            cpg.add_edge(parent_idx, func_idx, CpgEdgeKind::Contains);

            // Extract parameters
            extract_parameters(cpg, node, source, lang, file_path, func_idx);

            // Recurse into function body with func_idx as parent
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    walk_ast(cpg, source, file_path, lang, child, func_idx);
                }
            }
            return; // Don't recurse again below
        }
    }

    // ── Class definitions ───────────────────────────────────────────────
    if class_kinds.contains(&kind) {
        if let Some(name) = extract_class_name(node, source, lang) {
            let parent_class = extract_parent_class(node, source, lang);
            let visibility = extract_visibility(node, source, lang);

            let class_idx = cpg.add_node(CpgNode {
                kind: CpgNodeKind::Class {
                    name: name.clone(),
                    parent_class: parent_class.clone(),
                    visibility,
                },
                file: Some(file_path.to_string()),
                start_line,
                end_line,
                tags: Vec::new(),
            });
            cpg.add_edge(parent_idx, class_idx, CpgEdgeKind::Contains);

            // If inheriting, record an inherits edge placeholder (name-based, resolved later)
            // For now we just store the parent_class field.

            // Recurse into class body with class_idx as parent
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    walk_ast(cpg, source, file_path, lang, child, class_idx);
                }
            }
            return;
        }
    }

    // ── Import statements ───────────────────────────────────────────────
    if import_kinds.contains(&kind) {
        if let Some((module, alias, symbols)) = extract_import_info(node, source, lang) {
            let import_idx = cpg.add_node(CpgNode {
                kind: CpgNodeKind::Import {
                    module,
                    alias,
                    symbols,
                },
                file: Some(file_path.to_string()),
                start_line,
                end_line,
                tags: Vec::new(),
            });
            cpg.add_edge(parent_idx, import_idx, CpgEdgeKind::Imports);
        }
        // Don't recurse deeper for imports.
        return;
    }

    // ── Call sites ──────────────────────────────────────────────────────
    if call_kinds.contains(&kind) {
        if let Some(callee) = extract_callee_name(node, source) {
            let call_idx = cpg.add_node(CpgNode {
                kind: CpgNodeKind::CallSite {
                    callee: callee.clone(),
                },
                file: Some(file_path.to_string()),
                start_line,
                end_line,
                tags: Vec::new(),
            });
            cpg.add_edge(parent_idx, call_idx, CpgEdgeKind::Contains);
        }
    }

    // ── Default: recurse into children ──────────────────────────────────
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_ast(cpg, source, file_path, lang, child, parent_idx);
        }
    }
}

// ── Name extraction helpers ─────────────────────────────────────────────────

fn extract_function_name(node: tree_sitter::Node, source: &[u8], lang: Language) -> Option<String> {
    // For Rust impl_item: extract the impl target + individual methods inside
    if lang == Language::Rust && node.kind() == "impl_item" {
        return None; // We handle impl methods via recursion
    }

    // Try to find a "name" or "identifier" child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let ck = child.kind();
            if ck == "identifier" || ck == "name" || ck == "property_identifier" {
                return node_text(child, source);
            }
        }
    }

    // Fallback: named child "name"
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(name_node, source);
    }

    None
}

fn extract_class_name(node: tree_sitter::Node, source: &[u8], _lang: Language) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(name_node, source);
    }
    // Fallback: find first identifier child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier"
                || child.kind() == "type_identifier"
                || child.kind() == "name"
            {
                return node_text(child, source);
            }
        }
    }
    None
}

fn extract_parent_class(node: tree_sitter::Node, source: &[u8], _lang: Language) -> Option<String> {
    // Look for superclass field or "extends" / ":" pattern
    if let Some(sc) = node.child_by_field_name("superclass") {
        return node_text(sc, source);
    }
    // Java/C# extends
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "superclass"
                || child.kind() == "class_heritage"
                || child.kind() == "base_list"
            {
                // Get the type name inside
                for j in 0..child.child_count() {
                    if let Some(gc) = child.child(j) {
                        if gc.kind() == "identifier" || gc.kind() == "type_identifier" {
                            return node_text(gc, source);
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_import_info(
    node: tree_sitter::Node,
    source: &[u8],
    lang: Language,
) -> Option<(String, Option<String>, Vec<String>)> {
    let text = node_text(node, source)?;

    match lang {
        Language::Python => {
            // import foo / from foo import bar, baz
            if text.starts_with("from ") {
                let parts: Vec<&str> = text.splitn(2, " import ").collect();
                if parts.len() == 2 {
                    let module = parts[0].trim_start_matches("from ").trim().to_string();
                    let symbols: Vec<String> = parts[1]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    return Some((module, None, symbols));
                }
            }
            let module = text.trim_start_matches("import ").trim().to_string();
            Some((module, None, vec![]))
        }
        Language::JavaScript | Language::TypeScript => {
            // import X from 'module' / const X = require('module')
            // Extract module path from quotes
            if let Some(source_node) = node.child_by_field_name("source") {
                let module = node_text(source_node, source)?
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
                return Some((module, None, vec![]));
            }
            // Fallback: regex-like extraction
            let module = extract_quoted_string(&text)?;
            Some((module, None, vec![]))
        }
        Language::Rust => {
            // use foo::bar::baz;
            let module = text
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            Some((module, None, vec![]))
        }
        Language::Java | Language::CSharp => {
            let module = text
                .trim_start_matches("import ")
                .trim_start_matches("using ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            Some((module, None, vec![]))
        }
        Language::Go => {
            // Go import paths are in quotes
            let module = extract_quoted_string(&text)?;
            Some((module, None, vec![]))
        }
        Language::C | Language::Cpp => {
            // #include <foo.h> or #include "foo.h"
            let module = text
                .trim_start_matches("#include")
                .trim()
                .trim_matches(|c| c == '<' || c == '>' || c == '"')
                .to_string();
            Some((module, None, vec![]))
        }
        Language::Php => {
            let module = text.trim().to_string();
            Some((module, None, vec![]))
        }
        Language::Ruby => {
            // require / require_relative
            if text.contains("require") {
                let module = extract_quoted_string(&text)?;
                return Some((module, None, vec![]));
            }
            None
        }
    }
}

fn extract_callee_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Try the "function" field (common in JS/Python/Java)
    if let Some(func_node) = node.child_by_field_name("function") {
        return node_text(func_node, source);
    }
    // Try "method" field
    if let Some(method_node) = node.child_by_field_name("method") {
        return node_text(method_node, source);
    }
    // Try the "name" field
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(name_node, source);
    }
    // Fallback: first child that looks like an identifier or member access
    if let Some(child) = node.child(0) {
        let ck = child.kind();
        if ck == "identifier"
            || ck == "member_expression"
            || ck == "field_expression"
            || ck == "scoped_identifier"
            || ck == "attribute"
        {
            return node_text(child, source);
        }
    }
    None
}

fn extract_parameters(
    cpg: &mut CodePropertyGraph,
    func_node: tree_sitter::Node,
    source: &[u8],
    _lang: Language,
    file_path: &str,
    func_idx: NodeIndex,
) {
    // Look for "parameters" or "formal_parameters" child
    let param_list = func_node.child_by_field_name("parameters").or_else(|| {
        for i in 0..func_node.child_count() {
            if let Some(child) = func_node.child(i) {
                let ck = child.kind();
                if ck == "parameters"
                    || ck == "formal_parameters"
                    || ck == "parameter_list"
                    || ck == "formal_parameter_list"
                {
                    return Some(child);
                }
            }
        }
        None
    });

    if let Some(params) = param_list {
        let mut idx = 0;
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                let pk = param.kind();
                // Skip delimiters
                if pk == "," || pk == "(" || pk == ")" {
                    continue;
                }
                if let Some(name) = param
                    .child_by_field_name("name")
                    .or_else(|| param.child_by_field_name("pattern"))
                    .and_then(|n| node_text(n, source))
                    .or_else(|| {
                        if param.kind() == "identifier" {
                            node_text(param, source)
                        } else {
                            None
                        }
                    })
                {
                    let type_name = param
                        .child_by_field_name("type")
                        .and_then(|n| node_text(n, source));

                    let line = param.start_position().row + 1;
                    let param_idx = cpg.add_node(CpgNode {
                        kind: CpgNodeKind::Parameter {
                            name,
                            type_name,
                            index: idx,
                        },
                        file: Some(file_path.to_string()),
                        start_line: line,
                        end_line: line,
                        tags: Vec::new(),
                    });
                    cpg.add_edge(func_idx, param_idx, CpgEdgeKind::HasParameter);
                    idx += 1;
                }
            }
        }
    }
}

fn extract_visibility(node: tree_sitter::Node, source: &[u8], lang: Language) -> String {
    match lang {
        Language::Rust => {
            // Check for "pub" keyword
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "visibility_modifier" {
                        return node_text(child, source).unwrap_or_else(|| "pub".to_string());
                    }
                }
            }
            "private".to_string()
        }
        Language::Java | Language::CSharp | Language::Php => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if let Some(text) = node_text(child, source) {
                        if text == "public"
                            || text == "private"
                            || text == "protected"
                            || text == "internal"
                        {
                            return text;
                        }
                    }
                    if child.kind() == "modifiers" || child.kind() == "modifier_list" {
                        if let Some(text) = node_text(child, source) {
                            if text.contains("public") {
                                return "public".to_string();
                            }
                            if text.contains("private") {
                                return "private".to_string();
                            }
                            if text.contains("protected") {
                                return "protected".to_string();
                            }
                        }
                    }
                }
            }
            "package".to_string()
        }
        _ => "public".to_string(), // JS/Python/Go/Ruby/C default
    }
}

fn extract_signature(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Take the first line of the function as signature.
    let text = node_text(node, source)?;
    let first_line = text.lines().next()?;
    if first_line.len() > 200 {
        Some(format!("{}...", &first_line[..197]))
    } else {
        Some(first_line.to_string())
    }
}

fn detect_async(node: tree_sitter::Node, source: &[u8], _lang: Language) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(text) = node_text(child, source) {
                if text == "async" {
                    return true;
                }
            }
        }
    }
    false
}

fn detect_static(node: tree_sitter::Node, source: &[u8], _lang: Language) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(text) = node_text(child, source) {
                if text == "static" {
                    return true;
                }
            }
        }
    }
    false
}

fn detect_method_context(
    node: tree_sitter::Node,
    source: &[u8],
    lang: Language,
) -> (bool, Option<String>) {
    // Walk up through named ancestors to see if inside a class
    if let Some(parent) = node.parent() {
        let pk = parent.kind();
        let class_kinds = lang.class_node_kinds();
        // Some languages wrap methods in a class_body / declaration_list
        if pk == "class_body"
            || pk == "declaration_list"
            || pk == "body"
            || pk == "block"
            || class_kinds.contains(&pk)
        {
            let class_node = if class_kinds.contains(&pk) {
                parent
            } else {
                parent
                    .parent()
                    .filter(|gp| class_kinds.contains(&gp.kind()))
                    .unwrap_or(parent)
            };
            if let Some(name) = extract_class_name(class_node, source, lang) {
                return (true, Some(name));
            }
        }

        // Rust impl blocks
        if lang == Language::Rust && pk == "impl_item" {
            if let Some(type_node) = parent.child_by_field_name("type") {
                if let Some(name) = node_text(type_node, source) {
                    return (true, Some(name));
                }
            }
        }
    }
    (false, None)
}

// ── Call edge resolution ────────────────────────────────────────────────────

/// After all files are parsed, try to resolve call sites to function definitions
/// within the same project.
fn resolve_call_edges(cpg: &mut CodePropertyGraph) {
    // Collect CallSites
    let call_sites: Vec<(NodeIndex, String)> = cpg
        .graph
        .node_indices()
        .filter_map(|idx| {
            if let CpgNodeKind::CallSite { ref callee } = cpg.graph[idx].kind {
                Some((idx, callee.clone()))
            } else {
                None
            }
        })
        .collect();

    // Build name → function/method node map
    let mut func_map: HashMap<String, Vec<NodeIndex>> = HashMap::new();
    for idx in cpg.graph.node_indices() {
        match &cpg.graph[idx].kind {
            CpgNodeKind::Function { ref name, .. } | CpgNodeKind::Method { ref name, .. } => {
                func_map.entry(name.clone()).or_default().push(idx);
            }
            _ => {}
        }
    }

    // Create Calls edges
    let mut edges_to_add = Vec::new();
    for (call_idx, callee) in call_sites {
        // Try exact match first
        let target_name = callee
            .rsplit('.')
            .next()
            .or_else(|| callee.rsplit("::").next())
            .unwrap_or(&callee);

        if let Some(targets) = func_map.get(target_name) {
            // Find call site's containing function
            let caller_func = find_containing_function(cpg, call_idx);
            for &target_idx in targets {
                // Avoid self-referencing
                if Some(target_idx) == caller_func {
                    continue;
                }
                let from = caller_func.unwrap_or(call_idx);
                edges_to_add.push((from, target_idx));
            }
        }
    }

    for (from, to) in edges_to_add {
        cpg.add_edge(from, to, CpgEdgeKind::Calls);
    }
}

/// Walk parents to find the containing function/method node for a call site.
fn find_containing_function(cpg: &CodePropertyGraph, node_idx: NodeIndex) -> Option<NodeIndex> {
    // Walk Contains edges backward
    use petgraph::Direction;
    for parent in cpg.graph.neighbors_directed(node_idx, Direction::Incoming) {
        if let Some(edge) = cpg.graph.find_edge(parent, node_idx) {
            if matches!(cpg.graph[edge].kind, CpgEdgeKind::Contains) {
                match &cpg.graph[parent].kind {
                    CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. } => {
                        return Some(parent);
                    }
                    _ => {
                        // Recurse up
                        return find_containing_function(cpg, parent);
                    }
                }
            }
        }
    }
    None
}

// ── Utilities ───────────────────────────────────────────────────────────────

fn node_text(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let start = node.start_byte();
    let end = node.end_byte();
    if end <= start || end > source.len() {
        return None;
    }
    String::from_utf8_lossy(&source[start..end])
        .to_string()
        .into()
}

fn extract_quoted_string(text: &str) -> Option<String> {
    let start = text.find(|c: char| c == '\'' || c == '"')?;
    let quote = text.as_bytes()[start] as char;
    let rest = &text[start + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn bytecount_lines(source: &[u8]) -> usize {
    source.iter().filter(|&&b| b == b'\n').count() + 1
}
