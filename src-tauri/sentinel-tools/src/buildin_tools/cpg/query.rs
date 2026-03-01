//! CPG query operations — extract structured information from the graph.

use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::Serialize;

use super::types::*;

// ── Query result types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub visibility: String,
    pub is_async: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassInfo {
    pub name: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_class: Option<String>,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportInfo {
    pub module: String,
    pub file: String,
    pub line: usize,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallEdgeInfo {
    pub caller: String,
    pub caller_file: String,
    pub caller_line: usize,
    pub callee: String,
    pub callee_file: String,
    pub callee_line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntryPointInfo {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub language: String,
    pub lines: usize,
    pub functions: usize,
    pub classes: usize,
    pub imports: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpgSummary {
    pub root: String,
    pub primary_language: String,
    pub languages: Vec<String>,
    pub total_files: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub total_imports: usize,
    pub total_call_edges: usize,
    pub total_entry_points: usize,
    pub built_at: String,
}

// ── Query implementations ───────────────────────────────────────────────────

impl CodePropertyGraph {
    /// Generate high-level summary statistics.
    pub fn summary(&self) -> CpgSummary {
        CpgSummary {
            root: self.root.clone(),
            primary_language: self.primary_language.clone(),
            languages: self.languages.clone(),
            total_files: self.file_count(),
            total_nodes: self.node_count(),
            total_edges: self.edge_count(),
            total_functions: self.function_count(),
            total_classes: self.class_count(),
            total_imports: self.import_count(),
            total_call_edges: self.call_edge_count(),
            total_entry_points: self.entry_point_count(),
            built_at: self.built_at.clone(),
        }
    }

    /// List all functions/methods.
    pub fn list_functions(&self, limit: usize) -> Vec<FunctionInfo> {
        let mut results = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            match &node.kind {
                CpgNodeKind::Function {
                    name,
                    visibility,
                    is_async,
                    ..
                } => {
                    let params = self.get_parameter_names(idx);
                    results.push(FunctionInfo {
                        name: name.clone(),
                        file: node.file.clone().unwrap_or_default(),
                        start_line: node.start_line,
                        end_line: node.end_line,
                        visibility: visibility.clone(),
                        is_async: *is_async,
                        class_name: None,
                        params,
                    });
                }
                CpgNodeKind::Method {
                    name,
                    class_name,
                    is_async,
                    visibility,
                    ..
                } => {
                    let params = self.get_parameter_names(idx);
                    results.push(FunctionInfo {
                        name: name.clone(),
                        file: node.file.clone().unwrap_or_default(),
                        start_line: node.start_line,
                        end_line: node.end_line,
                        visibility: visibility.clone(),
                        is_async: *is_async,
                        class_name: class_name.clone(),
                        params,
                    });
                }
                _ => {}
            }
            if results.len() >= limit {
                break;
            }
        }
        results
    }

    /// List all classes/structs.
    pub fn list_classes(&self, limit: usize) -> Vec<ClassInfo> {
        let mut results = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if let CpgNodeKind::Class {
                name,
                parent_class,
                visibility,
            } = &node.kind
            {
                let methods = self.get_child_method_names(idx);
                results.push(ClassInfo {
                    name: name.clone(),
                    file: node.file.clone().unwrap_or_default(),
                    start_line: node.start_line,
                    end_line: node.end_line,
                    visibility: visibility.clone(),
                    parent_class: parent_class.clone(),
                    methods,
                });
                if results.len() >= limit {
                    break;
                }
            }
        }
        results
    }

    /// List all imports.
    pub fn list_imports(&self, limit: usize) -> Vec<ImportInfo> {
        let mut results = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if let CpgNodeKind::Import {
                module, symbols, ..
            } = &node.kind
            {
                results.push(ImportInfo {
                    module: module.clone(),
                    file: node.file.clone().unwrap_or_default(),
                    line: node.start_line,
                    symbols: symbols.clone(),
                });
                if results.len() >= limit {
                    break;
                }
            }
        }
        results
    }

    /// List resolved call edges (function → function).
    pub fn list_call_edges(&self, limit: usize) -> Vec<CallEdgeInfo> {
        let mut results = Vec::new();
        for edge_ref in self.graph.edge_references() {
            if !matches!(edge_ref.weight().kind, CpgEdgeKind::Calls) {
                continue;
            }
            let src = &self.graph[edge_ref.source()];
            let tgt = &self.graph[edge_ref.target()];
            results.push(CallEdgeInfo {
                caller: src.kind.display_name(),
                caller_file: src.file.clone().unwrap_or_default(),
                caller_line: src.start_line,
                callee: tgt.kind.display_name(),
                callee_file: tgt.file.clone().unwrap_or_default(),
                callee_line: tgt.start_line,
            });
            if results.len() >= limit {
                break;
            }
        }
        results
    }

    /// Find all callers of a function/method by name.
    pub fn callers_of(&self, function_name: &str) -> Vec<CallEdgeInfo> {
        let mut results = Vec::new();
        let target_indices: Vec<_> = self
            .graph
            .node_indices()
            .filter(|&idx| {
                let node = &self.graph[idx];
                match &node.kind {
                    CpgNodeKind::Function { name, .. } | CpgNodeKind::Method { name, .. } => {
                        name == function_name
                    }
                    _ => false,
                }
            })
            .collect();

        for target_idx in target_indices {
            let tgt = &self.graph[target_idx];
            for caller_idx in self
                .graph
                .neighbors_directed(target_idx, Direction::Incoming)
            {
                if let Some(edge) = self.graph.find_edge(caller_idx, target_idx) {
                    if matches!(self.graph[edge].kind, CpgEdgeKind::Calls) {
                        let src = &self.graph[caller_idx];
                        results.push(CallEdgeInfo {
                            caller: src.kind.display_name(),
                            caller_file: src.file.clone().unwrap_or_default(),
                            caller_line: src.start_line,
                            callee: tgt.kind.display_name(),
                            callee_file: tgt.file.clone().unwrap_or_default(),
                            callee_line: tgt.start_line,
                        });
                    }
                }
            }
        }
        results
    }

    /// Find all functions that a given function calls.
    pub fn callees_of(&self, function_name: &str) -> Vec<CallEdgeInfo> {
        let mut results = Vec::new();
        let src_indices: Vec<_> = self
            .graph
            .node_indices()
            .filter(|&idx| {
                let node = &self.graph[idx];
                match &node.kind {
                    CpgNodeKind::Function { name, .. } | CpgNodeKind::Method { name, .. } => {
                        name == function_name
                    }
                    _ => false,
                }
            })
            .collect();

        for src_idx in src_indices {
            let src = &self.graph[src_idx];
            for callee_idx in self.graph.neighbors_directed(src_idx, Direction::Outgoing) {
                if let Some(edge) = self.graph.find_edge(src_idx, callee_idx) {
                    if matches!(self.graph[edge].kind, CpgEdgeKind::Calls) {
                        let tgt = &self.graph[callee_idx];
                        results.push(CallEdgeInfo {
                            caller: src.kind.display_name(),
                            caller_file: src.file.clone().unwrap_or_default(),
                            caller_line: src.start_line,
                            callee: tgt.kind.display_name(),
                            callee_file: tgt.file.clone().unwrap_or_default(),
                            callee_line: tgt.start_line,
                        });
                    }
                }
            }
        }
        results
    }

    /// List all functions in a specific file.
    pub fn functions_in_file(&self, file_path: &str) -> Vec<FunctionInfo> {
        self.list_functions(usize::MAX)
            .into_iter()
            .filter(|f| f.file == file_path || f.file.ends_with(file_path))
            .collect()
    }

    /// List all files with their summary.
    pub fn list_file_summaries(&self, limit: usize) -> Vec<FileSummary> {
        let mut results = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if let CpgNodeKind::File {
                path,
                language,
                lines,
            } = &node.kind
            {
                let children: Vec<_> = self
                    .graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .collect();
                let functions = children
                    .iter()
                    .filter(|&&c| {
                        matches!(
                            self.graph[c].kind,
                            CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. }
                        )
                    })
                    .count();
                let classes = children
                    .iter()
                    .filter(|&&c| matches!(self.graph[c].kind, CpgNodeKind::Class { .. }))
                    .count();
                let imports = children
                    .iter()
                    .filter(|&&c| matches!(self.graph[c].kind, CpgNodeKind::Import { .. }))
                    .count();

                // Count functions inside classes too
                let nested_functions: usize = children
                    .iter()
                    .filter(|&&c| matches!(self.graph[c].kind, CpgNodeKind::Class { .. }))
                    .map(|&c| {
                        self.graph
                            .neighbors_directed(c, Direction::Outgoing)
                            .filter(|&gc| {
                                matches!(
                                    self.graph[gc].kind,
                                    CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. }
                                )
                            })
                            .count()
                    })
                    .sum();

                results.push(FileSummary {
                    path: path.clone(),
                    language: language.clone(),
                    lines: *lines,
                    functions: functions + nested_functions,
                    classes,
                    imports,
                });
                if results.len() >= limit {
                    break;
                }
            }
        }
        results.sort_by(|a, b| b.functions.cmp(&a.functions));
        results
    }

    /// Search functions/methods/classes by name substring.
    pub fn search_symbols(&self, query: &str, limit: usize) -> Vec<FunctionInfo> {
        let q = query.to_lowercase();
        let mut results = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            let name = node.kind.display_name();
            if name.to_lowercase().contains(&q) {
                match &node.kind {
                    CpgNodeKind::Function {
                        name,
                        visibility,
                        is_async,
                        ..
                    } => {
                        results.push(FunctionInfo {
                            name: name.clone(),
                            file: node.file.clone().unwrap_or_default(),
                            start_line: node.start_line,
                            end_line: node.end_line,
                            visibility: visibility.clone(),
                            is_async: *is_async,
                            class_name: None,
                            params: self.get_parameter_names(idx),
                        });
                    }
                    CpgNodeKind::Method {
                        name,
                        class_name,
                        is_async,
                        visibility,
                        ..
                    } => {
                        results.push(FunctionInfo {
                            name: name.clone(),
                            file: node.file.clone().unwrap_or_default(),
                            start_line: node.start_line,
                            end_line: node.end_line,
                            visibility: visibility.clone(),
                            is_async: *is_async,
                            class_name: class_name.clone(),
                            params: self.get_parameter_names(idx),
                        });
                    }
                    _ => {}
                }
            }
            if results.len() >= limit {
                break;
            }
        }
        results
    }

    // ── Private helpers ─────────────────────────────────────────────────

    pub fn get_parameter_names(&self, func_idx: petgraph::graph::NodeIndex) -> Vec<String> {
        let mut params: Vec<(usize, String)> = Vec::new();
        for child in self.graph.neighbors_directed(func_idx, Direction::Outgoing) {
            if let Some(edge) = self.graph.find_edge(func_idx, child) {
                if matches!(self.graph[edge].kind, CpgEdgeKind::HasParameter) {
                    if let CpgNodeKind::Parameter { name, index, .. } = &self.graph[child].kind {
                        params.push((*index, name.clone()));
                    }
                }
            }
        }
        params.sort_by_key(|(i, _)| *i);
        params.into_iter().map(|(_, n)| n).collect()
    }

    fn get_child_method_names(&self, class_idx: petgraph::graph::NodeIndex) -> Vec<String> {
        let mut names = Vec::new();
        for child in self
            .graph
            .neighbors_directed(class_idx, Direction::Outgoing)
        {
            if let Some(edge) = self.graph.find_edge(class_idx, child) {
                if matches!(self.graph[edge].kind, CpgEdgeKind::Contains) {
                    match &self.graph[child].kind {
                        CpgNodeKind::Function { name, .. } | CpgNodeKind::Method { name, .. } => {
                            names.push(name.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        names
    }
}
