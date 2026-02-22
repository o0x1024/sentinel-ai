//! Code Property Graph (CPG) engine for structural code analysis.
//!
//! Combines tree-sitter AST parsing with petgraph to build a queryable
//! code structure graph for security auditing.
//!
//! # Modules
//! - `types`           — Core CPG node/edge data structures
//! - `parser`          — Tree-sitter multi-language parser
//! - `builder`         — Builds CPG from parsed ASTs
//! - `query`           — Graph query operations
//! - `security_rules`  — Source/sink/sanitizer pattern definitions
//! - `taint`           — Graph-based taint analysis engine
//! - `context`         — Audit context generator (system prompt injection)
//! - `tools`           — Agent-callable tool wrappers

pub mod types;
pub mod parser;
pub mod builder;
pub mod query;
pub mod security_rules;
pub mod taint;
pub mod context;
pub mod tools;

pub use tools::{BuildCpgTool, QueryCpgTool, CpgTaintAnalysisTool, CpgSecurityScanTool};
pub use types::{CodePropertyGraph, CpgNodeKind, CpgEdgeKind};
pub use context::{generate_audit_context, cpg_availability_notice};
