//! Core CPG data structures.

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Node types ──────────────────────────────────────────────────────────────

/// What a CPG node represents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CpgNodeKind {
    File {
        path: String,
        language: String,
        lines: usize,
    },
    /// Module / namespace / package
    Module {
        name: String,
    },
    Class {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_class: Option<String>,
        visibility: String,
    },
    Function {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
        visibility: String,
        is_async: bool,
    },
    /// Class/struct method
    Method {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        class_name: Option<String>,
        is_static: bool,
        is_async: bool,
        visibility: String,
    },
    Parameter {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
        index: usize,
    },
    Variable {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
    },
    Import {
        module: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alias: Option<String>,
        #[serde(default)]
        symbols: Vec<String>,
    },
    CallSite {
        callee: String,
    },
    StringLiteral {
        value: String,
    },
    /// HTTP / REST entry point inferred from framework patterns
    EntryPoint {
        method: String, // GET, POST, etc.
        path: String,
        handler: String,
    },
}

impl CpgNodeKind {
    pub fn label(&self) -> &str {
        match self {
            Self::File { .. } => "file",
            Self::Module { .. } => "module",
            Self::Class { .. } => "class",
            Self::Function { .. } => "function",
            Self::Method { .. } => "method",
            Self::Parameter { .. } => "parameter",
            Self::Variable { .. } => "variable",
            Self::Import { .. } => "import",
            Self::CallSite { .. } => "call_site",
            Self::StringLiteral { .. } => "string_literal",
            Self::EntryPoint { .. } => "entry_point",
        }
    }

    /// Node display name for queries.
    pub fn display_name(&self) -> String {
        match self {
            Self::File { path, .. } => path.clone(),
            Self::Module { name } => name.clone(),
            Self::Class { name, .. } => name.clone(),
            Self::Function { name, .. } => name.clone(),
            Self::Method { name, class_name, .. } => {
                if let Some(cls) = class_name {
                    format!("{}.{}", cls, name)
                } else {
                    name.clone()
                }
            }
            Self::Parameter { name, .. } => name.clone(),
            Self::Variable { name, .. } => name.clone(),
            Self::Import { module, .. } => module.clone(),
            Self::CallSite { callee } => callee.clone(),
            Self::StringLiteral { value } => {
                if value.len() > 30 {
                    format!("\"{}...\"", &value[..27])
                } else {
                    format!("\"{}\"", value)
                }
            }
            Self::EntryPoint { method, path, .. } => format!("{} {}", method, path),
        }
    }
}

// ── Edge types ──────────────────────────────────────────────────────────────

/// Relationship between two CPG nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpgEdgeKind {
    /// Structural containment (File→Class, Class→Method)
    Contains,
    /// Function/method call
    Calls,
    /// Import dependency
    Imports,
    /// Parameter binding
    HasParameter,
    /// Class inheritance
    Inherits,
    /// Handler for entry point
    HandledBy,
}

impl CpgEdgeKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Contains => "contains",
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::HasParameter => "has_parameter",
            Self::Inherits => "inherits",
            Self::HandledBy => "handled_by",
        }
    }
}

// ── Node properties ─────────────────────────────────────────────────────────

/// All metadata attached to a CPG node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpgNode {
    pub kind: CpgNodeKind,
    /// Source file (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    /// Security-relevant tags: `source`, `sink`, `sanitizer`, `entry_point`, etc.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Edge weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpgEdge {
    pub kind: CpgEdgeKind,
}

impl CpgEdge {
    pub fn new(kind: CpgEdgeKind) -> Self {
        Self { kind }
    }
}

// ── Code Property Graph ─────────────────────────────────────────────────────

/// The main CPG structure (in-memory).
pub struct CodePropertyGraph {
    pub graph: DiGraph<CpgNode, CpgEdge>,
    /// file_path → file node index
    pub file_index: HashMap<String, NodeIndex>,
    /// symbol_name → node indices (functions, classes, methods)
    pub symbol_index: HashMap<String, Vec<NodeIndex>>,
    /// (file_path, start_line) → node index (for lookup by location)
    pub location_index: HashMap<(String, usize), NodeIndex>,
    /// Project root
    pub root: String,
    /// Primary language
    pub primary_language: String,
    /// All detected languages
    pub languages: Vec<String>,
    /// Build timestamp
    pub built_at: String,
}

impl CodePropertyGraph {
    pub fn new(root: String) -> Self {
        Self {
            graph: DiGraph::new(),
            file_index: HashMap::new(),
            symbol_index: HashMap::new(),
            location_index: HashMap::new(),
            root,
            primary_language: String::new(),
            languages: Vec::new(),
            built_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add a node and update indices.
    pub fn add_node(&mut self, node: CpgNode) -> NodeIndex {
        let name = node.kind.display_name();
        let file = node.file.clone();
        let line = node.start_line;
        let idx = self.graph.add_node(node);

        // Update symbol index
        if !name.is_empty() {
            self.symbol_index
                .entry(name)
                .or_default()
                .push(idx);
        }

        // Update location index
        if let Some(ref f) = file {
            self.location_index.insert((f.clone(), line), idx);
        }

        idx
    }

    /// Add a directed edge.
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, kind: CpgEdgeKind) {
        self.graph.add_edge(from, to, CpgEdge::new(kind));
    }

    // ── Statistics ───────────────────────────────────────────────────────

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn file_count(&self) -> usize {
        self.file_index.len()
    }

    pub fn function_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, CpgNodeKind::Function { .. } | CpgNodeKind::Method { .. }))
            .count()
    }

    pub fn class_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, CpgNodeKind::Class { .. }))
            .count()
    }

    pub fn import_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, CpgNodeKind::Import { .. }))
            .count()
    }

    pub fn call_edge_count(&self) -> usize {
        self.graph
            .edge_weights()
            .filter(|e| matches!(e.kind, CpgEdgeKind::Calls))
            .count()
    }

    pub fn entry_point_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, CpgNodeKind::EntryPoint { .. }))
            .count()
    }
}
