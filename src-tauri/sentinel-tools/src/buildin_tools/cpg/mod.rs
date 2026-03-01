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

pub mod builder;
pub mod context;
pub mod parser;
pub mod query;
pub mod security_rules;
pub mod taint;
pub mod tools;
pub mod types;

pub use context::{cpg_availability_notice, generate_audit_context};
pub use tools::{BuildCpgTool, CpgSecurityScanTool, CpgTaintAnalysisTool, QueryCpgTool};
pub use types::{CodePropertyGraph, CpgEdgeKind, CpgNodeKind};
