use regex::Regex;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LspAction {
    WorkspaceSymbol,
    DocumentSymbol,
    GoToDefinition,
    FindReferences,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct LspArgs {
    pub action: LspAction,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspSymbolMatch {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: usize,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspReferenceMatch {
    pub file_path: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspOutput {
    pub action: String,
    pub symbols: Vec<LspSymbolMatch>,
    pub references: Vec<LspReferenceMatch>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("query is required for this action")]
    MissingQuery,
    #[error("file_path is required for this action")]
    MissingFilePath,
    #[error("symbol is required for this action")]
    MissingSymbol,
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("failed to read file: {0}")]
    ReadFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct LspTool;

impl LspTool {
    pub const NAME: &'static str = "lsp";
    pub const DESCRIPTION: &'static str = concat!(
        "Navigate source code by symbols instead of plain text. ",
        "Supports workspace_symbol, document_symbol, go_to_definition, and find_references. ",
        "Use this before broad grep when you need code-aware symbol navigation."
    );
}

impl Tool for LspTool {
    const NAME: &'static str = Self::NAME;
    type Args = LspArgs;
    type Output = LspOutput;
    type Error = LspError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(LspArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = args.limit.max(1).min(500);
        match args.action {
            LspAction::WorkspaceSymbol => {
                let query = args.query.as_deref().ok_or(LspError::MissingQuery)?;
                let base_dir = resolve_base_dir(args.path.as_deref())?;
                let focus_file = args.file_path.as_deref().map(PathBuf::from);
                let (symbols, truncated) =
                    workspace_symbol(&base_dir, query, focus_file.as_deref(), limit).await?;
                Ok(LspOutput {
                    action: "workspace_symbol".to_string(),
                    symbols,
                    references: Vec::new(),
                    truncated,
                })
            }
            LspAction::DocumentSymbol => {
                let file_path = args.file_path.as_deref().ok_or(LspError::MissingFilePath)?;
                let path = resolve_path(file_path)?;
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|error| LspError::ReadFailed(error.to_string()))?;
                let display_path = display_path(&path, path.parent().unwrap_or(Path::new("")));
                let mut symbols =
                    extract_symbols(&display_path, &content, &symbol_patterns_for_path(&path));
                symbols.truncate(limit);
                Ok(LspOutput {
                    action: "document_symbol".to_string(),
                    symbols,
                    references: Vec::new(),
                    truncated: false,
                })
            }
            LspAction::GoToDefinition => {
                let symbol = args.symbol.as_deref().ok_or(LspError::MissingSymbol)?;
                let base_dir = resolve_base_dir(args.path.as_deref())?;
                let focus_file = args.file_path.as_deref().map(resolve_path).transpose()?;
                let mut symbols = Vec::new();
                if let Some(path) = focus_file.as_ref() {
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|error| LspError::ReadFailed(error.to_string()))?;
                    let display = display_path(path, &base_dir);
                    symbols.extend(
                        extract_symbols(&display, &content, &symbol_patterns_for_path(&path))
                            .into_iter()
                            .filter(|item| item.name == symbol),
                    );
                }
                if symbols.is_empty() {
                    let (workspace_hits, _) =
                        workspace_symbol(&base_dir, symbol, focus_file.as_deref(), limit * 4)
                            .await?;
                    symbols = workspace_hits
                        .into_iter()
                        .filter(|item| item.name == symbol)
                        .collect();
                    sort_definition_candidates(&mut symbols, symbol, focus_file.as_deref());
                    symbols.truncate(limit);
                }
                Ok(LspOutput {
                    action: "go_to_definition".to_string(),
                    symbols,
                    references: Vec::new(),
                    truncated: false,
                })
            }
            LspAction::FindReferences => {
                let symbol = args.symbol.as_deref().ok_or(LspError::MissingSymbol)?;
                let base_dir = resolve_base_dir(args.path.as_deref())?;
                let focus_file = args.file_path.as_deref().map(resolve_path).transpose()?;
                let (references, truncated) =
                    find_references(&base_dir, symbol, focus_file.as_deref(), limit).await?;
                Ok(LspOutput {
                    action: "find_references".to_string(),
                    symbols: Vec::new(),
                    references,
                    truncated,
                })
            }
        }
    }
}

async fn workspace_symbol(
    base_dir: &Path,
    query: &str,
    focus_file: Option<&Path>,
    limit: usize,
) -> Result<(Vec<LspSymbolMatch>, bool), LspError> {
    let query_lower = query.to_lowercase();
    let mut symbols = Vec::new();
    for entry in WalkDir::new(base_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !is_supported_source_file(path) {
            continue;
        }
        let content = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(_) => continue,
        };
        let display = display_path(path, base_dir);
        for symbol in extract_symbols(&display, &content, &symbol_patterns_for_path(path)) {
            if !(symbol.name.to_lowercase().contains(&query_lower)
                || symbol
                    .signature
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query_lower))
            {
                continue;
            }
            symbols.push(symbol);
        }
    }
    symbols.sort_by(|left, right| {
        symbol_match_score(right, &query_lower, focus_file)
            .cmp(&symbol_match_score(left, &query_lower, focus_file))
            .then_with(|| left.file_path.cmp(&right.file_path))
            .then_with(|| left.line.cmp(&right.line))
    });
    let truncated = symbols.len() > limit;
    symbols.truncate(limit);
    Ok((symbols, truncated))
}

async fn find_references(
    base_dir: &Path,
    symbol: &str,
    focus_file: Option<&Path>,
    limit: usize,
) -> Result<(Vec<LspReferenceMatch>, bool), LspError> {
    let pattern = Regex::new(&format!(r"\b{}\b", regex::escape(symbol)))
        .map_err(|error| LspError::InvalidPath(error.to_string()))?;
    let mut references = Vec::new();
    for entry in WalkDir::new(base_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !is_supported_source_file(path) {
            continue;
        }
        let content = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(_) => continue,
        };
        let display = display_path(path, base_dir);
        for (index, line) in content.lines().enumerate() {
            if !pattern.is_match(line) {
                continue;
            }
            references.push(LspReferenceMatch {
                file_path: display.clone(),
                line: index + 1,
                snippet: condense_preview(line, 160),
            });
        }
    }
    references.sort_by(|left, right| {
        reference_score(right, symbol, focus_file)
            .cmp(&reference_score(left, symbol, focus_file))
            .then_with(|| left.file_path.cmp(&right.file_path))
            .then_with(|| left.line.cmp(&right.line))
    });
    let truncated = references.len() > limit;
    references.truncate(limit);
    Ok((references, truncated))
}

fn sort_definition_candidates(
    symbols: &mut [LspSymbolMatch],
    symbol: &str,
    focus_file: Option<&Path>,
) {
    let symbol_lower = symbol.to_lowercase();
    symbols.sort_by(|left, right| {
        symbol_match_score(right, &symbol_lower, focus_file)
            .cmp(&symbol_match_score(left, &symbol_lower, focus_file))
            .then_with(|| left.file_path.cmp(&right.file_path))
            .then_with(|| left.line.cmp(&right.line))
    });
}

fn symbol_match_score(item: &LspSymbolMatch, query_lower: &str, focus_file: Option<&Path>) -> u32 {
    let name_lower = item.name.to_lowercase();
    let signature_lower = item.signature.as_deref().unwrap_or_default().to_lowercase();
    let mut score = 0_u32;
    if let Some(path) = focus_file {
        let focus_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if item.file_path.ends_with(focus_name) {
            score += 200;
        }
    }
    if name_lower == query_lower {
        score += 150;
    } else if name_lower.starts_with(query_lower) {
        score += 100;
    } else if name_lower.contains(query_lower) {
        score += 60;
    }
    if signature_lower.contains(query_lower) {
        score += 20;
    }
    score + symbol_kind_bias(&item.kind)
}

fn symbol_kind_bias(kind: &str) -> u32 {
    match kind {
        "function" | "method" => 30,
        "struct" | "class" | "interface" | "trait" | "enum" => 25,
        "type" => 20,
        "const" | "var" => 10,
        _ => 0,
    }
}

fn reference_score(item: &LspReferenceMatch, symbol: &str, focus_file: Option<&Path>) -> u32 {
    let mut score = 0_u32;
    if let Some(path) = focus_file {
        let focus_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if item.file_path.ends_with(focus_name) {
            score += 200;
        }
    }

    let snippet = item.snippet.as_str();
    let line = snippet.trim();
    let symbol_patterns = [
        format!("{}(", symbol),
        format!("{}.{}", symbol, ""),
        format!("{}::", symbol),
        format!(".{}", symbol),
    ];
    if symbol_patterns.iter().any(|pattern| line.contains(pattern)) {
        score += 80;
    }
    if line.starts_with("fn ")
        || line.starts_with("def ")
        || line.starts_with("class ")
        || line.starts_with("struct ")
    {
        score += 10;
    }
    score
}

fn extract_symbols(
    file_path: &str,
    content: &str,
    patterns: &[SymbolPattern],
) -> Vec<LspSymbolMatch> {
    let mut symbols = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        for pattern in patterns {
            if let Some(captures) = pattern.regex.captures(trimmed) {
                let Some(name) = captures.name("name").map(|item| item.as_str().to_string()) else {
                    continue;
                };
                symbols.push(LspSymbolMatch {
                    name,
                    kind: pattern.kind.to_string(),
                    file_path: file_path.to_string(),
                    line: index + 1,
                    signature: Some(condense_preview(trimmed, 160)),
                });
                break;
            }
        }
    }
    symbols
}

struct SymbolPattern {
    kind: &'static str,
    regex: Regex,
}

fn symbol_patterns_for_path(path: &Path) -> Vec<SymbolPattern> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    match extension {
        "rs" => rust_patterns(),
        "py" => python_patterns(),
        "js" | "jsx" | "ts" | "tsx" => js_ts_patterns(),
        "go" => go_patterns(),
        "java" => java_patterns(),
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" => c_family_patterns(),
        "rb" => ruby_patterns(),
        "php" => php_patterns(),
        "cs" => csharp_patterns(),
        _ => generic_patterns(),
    }
}

fn rust_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "struct",
            r"^(?:pub\s+)?struct\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "enum",
            r"^(?:pub\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "trait",
            r"^(?:pub\s+)?trait\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "impl",
            r"^impl(?:<[^>]+>)?\s+(?P<name>[A-Za-z_][A-Za-z0-9_:<>]*)",
        ),
        (
            "function",
            r"^(?:pub\s+)?(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "const",
            r"^(?:pub\s+)?const\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "type",
            r"^(?:pub\s+)?type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
    ])
}

fn python_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        ("class", r"^class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"),
        (
            "function",
            r"^(?:async\s+)?def\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
    ])
}

fn js_ts_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "class",
            r"^(?:export\s+)?class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "function",
            r"^(?:export\s+)?(?:async\s+)?function\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "function",
            r"^(?:export\s+)?const\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\(",
        ),
        (
            "interface",
            r"^(?:export\s+)?interface\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "type",
            r"^(?:export\s+)?type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
    ])
}

fn go_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "type",
            r"^type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s+(?:struct|interface)",
        ),
        (
            "function",
            r"^func\s+(?:\([^)]+\)\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        ("const", r"^const\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"),
        ("var", r"^var\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"),
    ])
}

fn java_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "class",
            r"^(?:public\s+)?class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "interface",
            r"^(?:public\s+)?interface\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "enum",
            r"^(?:public\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "method",
            r"^(?:public|private|protected).*\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(",
        ),
    ])
}

fn c_family_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "struct",
            r"^(?:typedef\s+)?struct\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "enum",
            r"^(?:typedef\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "function",
            r"^[A-Za-z_][A-Za-z0-9_\s\*]+\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*\)\s*\{?$",
        ),
    ])
}

fn ruby_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        ("class", r"^class\s+(?P<name>[A-Za-z_][A-Za-z0-9_:]*)"),
        ("module", r"^module\s+(?P<name>[A-Za-z_][A-Za-z0-9_:]*)"),
        ("method", r"^def\s+(?P<name>[A-Za-z_][A-Za-z0-9_!?=]*)"),
    ])
}

fn php_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "class",
            r"^(?:final\s+|abstract\s+)?class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "interface",
            r"^interface\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        ("trait", r"^trait\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"),
        (
            "function",
            r"^(?:public|protected|private|static|\s)*function\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
    ])
}

fn csharp_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[
        (
            "class",
            r"^(?:public\s+)?class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "interface",
            r"^(?:public\s+)?interface\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "enum",
            r"^(?:public\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        ),
        (
            "method",
            r"^(?:public|private|protected|internal).*\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(",
        ),
    ])
}

fn generic_patterns() -> Vec<SymbolPattern> {
    compile_patterns(&[(
        "symbol",
        r"^(?:class|struct|enum|interface|trait|type|def|fn|func)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    )])
}

fn compile_patterns(defs: &[(&'static str, &'static str)]) -> Vec<SymbolPattern> {
    defs.iter()
        .filter_map(|(kind, pattern)| {
            Regex::new(pattern)
                .ok()
                .map(|regex| SymbolPattern { kind, regex })
        })
        .collect()
}

fn resolve_base_dir(path: Option<&str>) -> Result<PathBuf, LspError> {
    let cwd = std::env::current_dir().map_err(|error| LspError::InvalidPath(error.to_string()))?;
    let dir = match path {
        Some(raw) if !raw.trim().is_empty() => {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                candidate
            } else {
                cwd.join(candidate)
            }
        }
        _ => cwd,
    };
    Ok(dir)
}

fn resolve_path(raw: &str) -> Result<PathBuf, LspError> {
    if raw.trim().is_empty() {
        return Err(LspError::InvalidPath("path cannot be empty".to_string()));
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        let cwd =
            std::env::current_dir().map_err(|error| LspError::InvalidPath(error.to_string()))?;
        Ok(cwd.join(path))
    }
}

fn display_path(path: &Path, base_dir: &Path) -> String {
    path.strip_prefix(base_dir)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn is_supported_source_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default(),
        "rs" | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "h"
            | "cc"
            | "cpp"
            | "cxx"
            | "hpp"
            | "rb"
            | "php"
            | "cs"
    )
}

fn condense_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let head_len = max_chars.saturating_sub(20).max(20);
    let head: String = trimmed.chars().take(head_len).collect();
    format!("{}...", head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn document_symbol_extracts_rust_symbols() {
        let temp_dir = std::env::temp_dir().join(format!("lsp-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.rs");
        tokio::fs::write(&file_path, "pub struct Demo {}\nfn run() {}\n")
            .await
            .unwrap();

        let output = LspTool
            .call(LspArgs {
                action: LspAction::DocumentSymbol,
                query: None,
                file_path: Some(file_path.to_string_lossy().to_string()),
                symbol: None,
                path: None,
                limit: 10,
            })
            .await
            .unwrap();

        assert!(output.symbols.iter().any(|item| item.name == "Demo"));
        assert!(output.symbols.iter().any(|item| item.name == "run"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn workspace_symbol_prefers_same_file_and_exact_name() {
        let temp_dir = std::env::temp_dir().join(format!("lsp-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(temp_dir.join("src"))
            .await
            .unwrap();
        tokio::fs::write(temp_dir.join("src/main.rs"), "fn target() {}\n")
            .await
            .unwrap();
        tokio::fs::write(
            temp_dir.join("src/other.rs"),
            "fn target_helper() {}\nfn target() {}\n",
        )
        .await
        .unwrap();

        let output = LspTool
            .call(LspArgs {
                action: LspAction::WorkspaceSymbol,
                query: Some("target".to_string()),
                file_path: Some(temp_dir.join("src/main.rs").to_string_lossy().to_string()),
                symbol: None,
                path: Some(temp_dir.to_string_lossy().to_string()),
                limit: 10,
            })
            .await
            .unwrap();

        assert_eq!(
            output.symbols.first().map(|item| item.file_path.as_str()),
            Some("src/main.rs")
        );

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn go_to_definition_prefers_same_file_before_workspace_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("lsp-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(temp_dir.join("src"))
            .await
            .unwrap();
        let main = temp_dir.join("src/main.rs");
        tokio::fs::write(&main, "fn target() {}\nfn call() { target(); }\n")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.join("src/other.rs"), "fn target() {}\n")
            .await
            .unwrap();

        let output = LspTool
            .call(LspArgs {
                action: LspAction::GoToDefinition,
                query: None,
                file_path: Some(main.to_string_lossy().to_string()),
                symbol: Some("target".to_string()),
                path: Some(temp_dir.to_string_lossy().to_string()),
                limit: 10,
            })
            .await
            .unwrap();

        assert_eq!(
            output.symbols.first().map(|item| item.file_path.as_str()),
            Some("src/main.rs")
        );

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn find_references_sorts_same_file_first() {
        let temp_dir = std::env::temp_dir().join(format!("lsp-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(temp_dir.join("src"))
            .await
            .unwrap();
        let main = temp_dir.join("src/main.rs");
        tokio::fs::write(&main, "fn target() {}\nfn call() { target(); }\n")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.join("src/other.rs"), "fn helper() { target(); }\n")
            .await
            .unwrap();

        let output = LspTool
            .call(LspArgs {
                action: LspAction::FindReferences,
                query: None,
                file_path: Some(main.to_string_lossy().to_string()),
                symbol: Some("target".to_string()),
                path: Some(temp_dir.to_string_lossy().to_string()),
                limit: 10,
            })
            .await
            .unwrap();

        assert_eq!(
            output
                .references
                .first()
                .map(|item| item.file_path.as_str()),
            Some("src/main.rs")
        );

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
