//! Plugin extensions — host function groups and virtual modules.
//!
//! With the QuickJS migration, host functions are registered directly in
//! `qjs_runtime.rs`. This module is kept for AST literal extraction (which
//! still uses `one_parser`) and the virtual module registry concept.

mod ast_ext;

pub use ast_ext::AstExtension;

/// Marker trait for extension groups. Kept for backward compatibility;
/// actual registration happens in `qjs_runtime::register_*_functions`.
pub trait Extension {
    fn name(&self) -> &'static str;
}
