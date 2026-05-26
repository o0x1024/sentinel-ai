//! AST literal extraction extension.
//!
//! Provides `parse_js_literals` which uses `one_parser` to extract string
//! literals from JavaScript/TypeScript source code. The host function bridge
//! to QuickJS lives in `qjs_runtime.rs`.

use super::Extension;

pub struct AstExtension;

impl Extension for AstExtension {
    fn name(&self) -> &'static str {
        "ast"
    }
}
