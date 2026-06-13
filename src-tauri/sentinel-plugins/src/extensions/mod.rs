//! Plugin extensions — host function groups and virtual modules.
//!
//! Host functions are registered directly in `sentinel_js_runtime.rs`.
//! This module is kept for AST literal extraction and the virtual module
//! registry concept.

mod ast_ext;

pub use ast_ext::AstExtension;

/// Marker trait for extension groups. Kept for backward compatibility;
/// actual registration happens in `sentinel_js_runtime::register_*_bindings`.
pub trait Extension {
    fn name(&self) -> &'static str;
}
