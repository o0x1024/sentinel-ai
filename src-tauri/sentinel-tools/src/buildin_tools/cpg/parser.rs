//! Multi-language tree-sitter parser.
//!
//! Detects file language by extension, creates a tree-sitter parser, and
//! returns the parsed syntax tree for the CPG builder to consume.

use std::path::Path;

/// Supported languages with their tree-sitter grammars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Java,
    Go,
    C,
    Cpp,
    CSharp,
    Php,
    Ruby,
}

impl Language {
    /// Detect from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "py" | "pyw" => Some(Self::Python),
            "java" => Some(Self::Java),
            "go" => Some(Self::Go),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            "cs" => Some(Self::CSharp),
            "php" => Some(Self::Php),
            "rb" => Some(Self::Ruby),
            _ => None,
        }
    }

    /// Detect from file path.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Python => "python",
            Self::Java => "java",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Php => "php",
            Self::Ruby => "ruby",
        }
    }

    /// Get the tree-sitter Language ABI handle.
    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        }
    }

    // ── Tree-sitter node kind queries per language ───────────────────────

    /// Node kinds that represent function/method definitions.
    pub fn function_node_kinds(&self) -> &[&str] {
        match self {
            Self::Rust => &["function_item", "impl_item"],
            Self::JavaScript | Self::TypeScript => &[
                "function_declaration",
                "method_definition",
                "arrow_function",
                "function",
            ],
            Self::Python => &["function_definition"],
            Self::Java => &["method_declaration", "constructor_declaration"],
            Self::Go => &["function_declaration", "method_declaration"],
            Self::C | Self::Cpp => &["function_definition"],
            Self::CSharp => &["method_declaration", "constructor_declaration"],
            Self::Php => &["function_definition", "method_declaration"],
            Self::Ruby => &["method", "singleton_method"],
        }
    }

    /// Node kinds that represent class-like declarations.
    pub fn class_node_kinds(&self) -> &[&str] {
        match self {
            Self::Rust => &["struct_item", "enum_item", "trait_item"],
            Self::JavaScript | Self::TypeScript => &["class_declaration", "class"],
            Self::Python => &["class_definition"],
            Self::Java => &[
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
            ],
            Self::Go => &["type_declaration"],
            Self::C => &["struct_specifier"],
            Self::Cpp => &["class_specifier", "struct_specifier"],
            Self::CSharp => &[
                "class_declaration",
                "interface_declaration",
                "struct_declaration",
            ],
            Self::Php => &[
                "class_declaration",
                "interface_declaration",
                "trait_declaration",
            ],
            Self::Ruby => &["class", "module"],
        }
    }

    /// Node kinds that represent import/use/require statements.
    pub fn import_node_kinds(&self) -> &[&str] {
        match self {
            Self::Rust => &["use_declaration"],
            Self::JavaScript | Self::TypeScript => &["import_statement", "import_declaration"],
            Self::Python => &["import_statement", "import_from_statement"],
            Self::Java => &["import_declaration"],
            Self::Go => &["import_declaration", "import_spec"],
            Self::C | Self::Cpp => &["preproc_include"],
            Self::CSharp => &["using_directive"],
            Self::Php => &["namespace_use_declaration"],
            Self::Ruby => &["call"], // require / require_relative
        }
    }

    /// Node kinds that represent function calls.
    pub fn call_node_kinds(&self) -> &[&str] {
        match self {
            Self::Rust => &["call_expression", "macro_invocation"],
            Self::JavaScript | Self::TypeScript => &["call_expression", "new_expression"],
            Self::Python => &["call"],
            Self::Java => &["method_invocation", "object_creation_expression"],
            Self::Go => &["call_expression"],
            Self::C | Self::Cpp => &["call_expression"],
            Self::CSharp => &["invocation_expression", "object_creation_expression"],
            Self::Php => &["function_call_expression", "member_call_expression"],
            Self::Ruby => &["call", "method_call"],
        }
    }
}

/// Parse source code and return a tree-sitter Tree.
pub fn parse_source(source: &[u8], language: Language) -> Option<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language.ts_language()).ok()?;
    parser.parse(source, None)
}
