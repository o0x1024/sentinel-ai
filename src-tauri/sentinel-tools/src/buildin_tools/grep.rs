use glob::Pattern;
use regex::RegexBuilder;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::buildin_tools::file_runtime::{
    current_runtime_metadata, execute_in_runtime, list_files_under, read_path_bytes,
    resolve_runtime_path, FileRuntimeMetadata,
};

// ── types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrepOutputMode {
    Content,
    FilesWithMatches,
    Count,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GrepArgs {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Base directory for the search. Defaults to the current working directory.
    #[serde(default)]
    pub path: Option<String>,
    /// Optional glob filter, for example `*.rs` or `src/**/*.ts`.
    #[serde(default)]
    pub glob: Option<String>,
    /// Additional include glob filters. If any include filter is present, only matching files are searched.
    #[serde(default)]
    pub include_globs: Vec<String>,
    /// Exclude glob filters. Matching files are skipped after include filters are applied.
    #[serde(default)]
    pub exclude_globs: Vec<String>,
    /// Match regex case-insensitively.
    #[serde(default)]
    pub case_insensitive: bool,
    /// Result mode.
    #[serde(default = "default_output_mode")]
    pub output_mode: GrepOutputMode,
    /// Maximum number of matches or files to return.
    #[serde(default = "default_head_limit")]
    pub head_limit: usize,
    /// Maximum number of matching candidate files to inspect.
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    /// Number of matches to skip before collecting output.
    #[serde(default)]
    pub offset: usize,
}

fn default_output_mode() -> GrepOutputMode {
    GrepOutputMode::Content
}

fn default_head_limit() -> usize {
    50
}

fn default_max_files() -> usize {
    10000
}

#[derive(Debug, Clone, Serialize)]
pub struct GrepContentMatch {
    pub file_path: String,
    pub line_number: usize,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrepOutput {
    pub pattern: String,
    pub base_path: String,
    pub runtime: FileRuntimeMetadata,
    pub output_mode: String,
    pub filenames: Vec<String>,
    pub content: Vec<GrepContentMatch>,
    pub num_matches: usize,
    pub applied_limit: usize,
    pub scanned_files: usize,
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GrepError {
    #[error("invalid base path: {0}")]
    InvalidBasePath(String),
    #[error("invalid regex pattern: {0}")]
    InvalidPattern(String),
}

// ── tool definition ────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct GrepTool;

impl GrepTool {
    pub const NAME: &'static str = "grep";
    pub const DESCRIPTION: &'static str = concat!(
        "Search text content in files using a regex pattern. ",
        "Use this to find symbols, strings, config keys, or repeated snippets across the workspace. ",
        "Supports case-insensitive matching, include/exclude globs, files-with-matches, count, and bounded result limits. ",
        "Prefer this over shell for bounded, structured search results."
    );
}

impl Tool for GrepTool {
    const NAME: &'static str = Self::NAME;
    type Args = GrepArgs;
    type Output = GrepOutput;
    type Error = GrepError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GrepArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match rg_grep(&args).await {
            Ok(output) => Ok(output),
            Err(RgError::NotAvailable) => {
                tracing::debug!("ripgrep not available, using fallback grep");
                let mut output = fallback_grep(args).await?;
                output.warning = Some(
                    "ripgrep (rg) is not installed; using slow fallback. \
                     Install for 10-100x faster search: \
                     brew install ripgrep / apt install ripgrep / cargo install ripgrep"
                        .to_string(),
                );
                Ok(output)
            }
            Err(RgError::Failed(reason)) => {
                tracing::debug!(reason, "ripgrep failed, using fallback grep");
                fallback_grep(args).await
            }
        }
    }
}

// ── ripgrep fast path ──────────────────────────────────────────────

enum RgError {
    NotAvailable,
    Failed(String),
}

async fn rg_grep(args: &GrepArgs) -> Result<GrepOutput, RgError> {
    let base_path = resolve_search_base(args.path.as_deref())
        .await
        .map_err(RgError::Failed)?;

    let rg_args = build_rg_args(args, &base_path);
    let result = execute_in_runtime("rg", &rg_args).await.map_err(|e| {
        if is_not_found_error(&e) {
            RgError::NotAvailable
        } else {
            RgError::Failed(e)
        }
    })?;

    // rg exit codes: 0 = matches found, 1 = no matches, 2 = error
    if !result.success {
        let stderr = String::from_utf8_lossy(&result.stderr);
        if result.stdout.is_empty() && stderr.trim().is_empty() {
            return Ok(make_empty_output(args, &base_path));
        }
        if is_not_found_error(&stderr) {
            return Err(RgError::NotAvailable);
        }
        return Err(RgError::Failed(stderr.into_owned()));
    }

    parse_rg_json(args, &base_path, &result.stdout).map_err(RgError::Failed)
}

async fn resolve_search_base(path: Option<&str>) -> Result<String, String> {
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => resolve_runtime_path(p).await,
        None => Ok(current_runtime_metadata().working_dir),
    }
}

fn build_rg_args(args: &GrepArgs, base_path: &str) -> Vec<String> {
    let mut rg: Vec<String> = vec!["--json".into()];

    if args.case_insensitive {
        rg.push("-i".into());
    }

    if let Some(g) = &args.glob {
        rg.extend(["--glob".into(), g.clone()]);
    }
    for g in &args.include_globs {
        rg.extend(["--glob".into(), g.clone()]);
    }
    for g in &args.exclude_globs {
        rg.extend(["--glob".into(), format!("!{}", g)]);
    }

    rg.push("--".into());
    rg.push(args.pattern.clone());
    rg.push(base_path.into());
    rg
}

fn parse_rg_json(args: &GrepArgs, base_path: &str, stdout: &[u8]) -> Result<GrepOutput, String> {
    let text = String::from_utf8_lossy(stdout);
    let applied_limit = args.head_limit.max(1).min(1000);
    let base_prefix = format!("{}/", base_path.strip_suffix('/').unwrap_or(base_path));

    let mut filenames = Vec::new();
    let mut content = Vec::new();
    let mut num_matches = 0_usize;
    let mut emitted = 0_usize;
    let mut scanned_files = 0_usize;
    let mut truncated = false;
    let mut seen_files = HashSet::new();

    for line in text.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match v.get("type").and_then(|t| t.as_str()) {
            Some("begin") => {
                scanned_files += 1;
            }
            Some("match") => {
                let Some(data) = v.get("data") else {
                    continue;
                };

                let raw_path = data
                    .pointer("/path/text")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                let rel_path = strip_base_prefix(raw_path, &base_prefix);
                let line_number = data
                    .get("line_number")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                let line_text = data
                    .pointer("/lines/text")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .trim_end_matches('\n')
                    .to_string();

                seen_files.insert(rel_path.clone());
                num_matches += 1;

                if num_matches <= args.offset {
                    continue;
                }

                match args.output_mode {
                    GrepOutputMode::Content => {
                        if emitted >= applied_limit {
                            truncated = true;
                            break;
                        }
                        content.push(GrepContentMatch {
                            file_path: rel_path,
                            line_number,
                            line: line_text,
                        });
                        emitted += 1;
                    }
                    GrepOutputMode::FilesWithMatches => {
                        if !filenames.contains(&rel_path) {
                            if emitted >= applied_limit {
                                truncated = true;
                                break;
                            }
                            filenames.push(rel_path);
                            emitted += 1;
                        }
                    }
                    GrepOutputMode::Count => {}
                }
            }
            _ => {}
        }
    }

    Ok(GrepOutput {
        pattern: args.pattern.clone(),
        base_path: base_path.to_string(),
        runtime: current_runtime_metadata(),
        output_mode: output_mode_str(&args.output_mode),
        filenames,
        content,
        num_matches: num_matches.saturating_sub(args.offset),
        applied_limit,
        scanned_files,
        truncated,
        warning: None,
    })
}

fn strip_base_prefix(path: &str, base_prefix: &str) -> String {
    path.strip_prefix(base_prefix)
        .or_else(|| path.strip_prefix("./"))
        .unwrap_or(path)
        .to_string()
}

fn is_not_found_error(msg: &str) -> bool {
    msg.contains("not found")
        || msg.contains("No such file or directory")
        || msg.contains("executable file not found")
}

fn make_empty_output(args: &GrepArgs, base_path: &str) -> GrepOutput {
    GrepOutput {
        pattern: args.pattern.clone(),
        base_path: base_path.to_string(),
        runtime: current_runtime_metadata(),
        output_mode: output_mode_str(&args.output_mode),
        filenames: Vec::new(),
        content: Vec::new(),
        num_matches: 0,
        applied_limit: args.head_limit.max(1).min(1000),
        scanned_files: 0,
        truncated: false,
        warning: None,
    }
}

fn output_mode_str(mode: &GrepOutputMode) -> String {
    match mode {
        GrepOutputMode::Content => "content".to_string(),
        GrepOutputMode::FilesWithMatches => "files_with_matches".to_string(),
        GrepOutputMode::Count => "count".to_string(),
    }
}

// ── pure-Rust fallback ─────────────────────────────────────────────

async fn fallback_grep(args: GrepArgs) -> Result<GrepOutput, GrepError> {
    let listing = list_files_under(args.path.as_deref())
        .await
        .map_err(GrepError::InvalidBasePath)?;
    let regex = RegexBuilder::new(&args.pattern)
        .multi_line(false)
        .case_insensitive(args.case_insensitive)
        .build()
        .map_err(|error| GrepError::InvalidPattern(error.to_string()))?;
    let include_patterns = compile_patterns(
        args.glob
            .as_deref()
            .into_iter()
            .chain(args.include_globs.iter().map(String::as_str)),
    )?;
    let exclude_patterns = compile_patterns(args.exclude_globs.iter().map(String::as_str))?;
    let applied_limit = args.head_limit.max(1).min(1000);
    let max_files = args.max_files.max(1).min(100000);

    let mut filenames = Vec::new();
    let mut content = Vec::new();
    let mut num_matches = 0_usize;
    let mut emitted = 0_usize;
    let mut scanned_files = 0_usize;
    let mut truncated = false;

    'files: for entry in &listing.files {
        let relative = entry.display_path.clone();
        if !include_patterns.is_empty()
            && !matches_any_pattern(&include_patterns, &relative, &entry.logical_path)
        {
            continue;
        }
        if matches_any_pattern(&exclude_patterns, &relative, &entry.logical_path) {
            continue;
        }
        if scanned_files >= max_files {
            truncated = true;
            break;
        }
        scanned_files += 1;

        let bytes = match read_path_bytes(&entry.logical_path).await {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if looks_binary(&bytes) {
            continue;
        }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let mut file_recorded = false;
        for (index, line) in text.lines().enumerate() {
            if !regex.is_match(line) {
                continue;
            }
            num_matches += 1;
            if num_matches <= args.offset {
                continue;
            }

            match args.output_mode {
                GrepOutputMode::Content => {
                    if emitted >= applied_limit {
                        truncated = true;
                        break 'files;
                    }
                    content.push(GrepContentMatch {
                        file_path: relative.clone(),
                        line_number: index + 1,
                        line: line.to_string(),
                    });
                    emitted += 1;
                }
                GrepOutputMode::FilesWithMatches => {
                    if !file_recorded {
                        if emitted >= applied_limit {
                            truncated = true;
                            break 'files;
                        }
                        filenames.push(relative.clone());
                        emitted += 1;
                        file_recorded = true;
                    }
                }
                GrepOutputMode::Count => {}
            }
        }
    }

    if matches!(args.output_mode, GrepOutputMode::Count) {
        filenames.clear();
        content.clear();
    }

    Ok(GrepOutput {
        pattern: args.pattern,
        base_path: listing.base_path,
        runtime: current_runtime_metadata(),
        output_mode: output_mode_str(&args.output_mode),
        filenames,
        content,
        num_matches: num_matches.saturating_sub(args.offset),
        applied_limit,
        scanned_files,
        truncated,
        warning: None,
    })
}

fn compile_patterns<'a>(values: impl Iterator<Item = &'a str>) -> Result<Vec<Pattern>, GrepError> {
    values
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Pattern::new(value).map_err(|error| {
                GrepError::InvalidPattern(format!("invalid glob '{}': {}", value, error))
            })
        })
        .collect()
}

fn matches_any_pattern(patterns: &[Pattern], display_path: &str, logical_path: &str) -> bool {
    patterns.iter().any(|pattern| {
        pattern.matches(display_path) || pattern.matches_path(std::path::Path::new(logical_path))
    })
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).any(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn grep_finds_content_matches() {
        let temp_dir = std::env::temp_dir().join(format!("grep-tool-{}", uuid::Uuid::new_v4()));
        let nested = temp_dir.join("src");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(
            nested.join("main.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .await
        .unwrap();
        tokio::fs::write(nested.join("lib.rs"), "pub fn helper() {}\n")
            .await
            .unwrap();

        let output = GrepTool
            .call(GrepArgs {
                pattern: "println!".to_string(),
                path: Some(temp_dir.to_string_lossy().to_string()),
                glob: Some("src/*.rs".to_string()),
                include_globs: vec![],
                exclude_globs: vec![],
                case_insensitive: false,
                output_mode: GrepOutputMode::Content,
                head_limit: 10,
                max_files: 100,
                offset: 0,
            })
            .await
            .unwrap();

        assert_eq!(output.num_matches, 1);
        assert_eq!(output.content.len(), 1);
        assert_eq!(output.content[0].file_path, "src/main.rs");
        assert_eq!(output.content[0].line_number, 2);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn grep_supports_case_insensitive_include_and_exclude_globs() {
        let temp_dir = std::env::temp_dir().join(format!("grep-tool-{}", uuid::Uuid::new_v4()));
        let nested = temp_dir.join("src");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(nested.join("main.c"), "int main() { return AEAD_OK; }\n")
            .await
            .unwrap();
        tokio::fs::write(nested.join("skip.c"), "int skip() { return aead_skip; }\n")
            .await
            .unwrap();
        tokio::fs::write(nested.join("readme.txt"), "aead docs\n")
            .await
            .unwrap();

        let output = GrepTool
            .call(GrepArgs {
                pattern: "aead".to_string(),
                path: Some(temp_dir.to_string_lossy().to_string()),
                glob: None,
                include_globs: vec!["src/*.c".to_string()],
                exclude_globs: vec!["src/skip.c".to_string()],
                case_insensitive: true,
                output_mode: GrepOutputMode::FilesWithMatches,
                head_limit: 10,
                max_files: 100,
                offset: 0,
            })
            .await
            .unwrap();

        assert_eq!(output.filenames, vec!["src/main.c"]);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn fallback_grep_finds_content_matches() {
        let temp_dir =
            std::env::temp_dir().join(format!("grep-fallback-{}", uuid::Uuid::new_v4()));
        let nested = temp_dir.join("src");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(
            nested.join("main.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .await
        .unwrap();
        tokio::fs::write(nested.join("lib.rs"), "pub fn helper() {}\n")
            .await
            .unwrap();

        let output = fallback_grep(GrepArgs {
            pattern: "println!".to_string(),
            path: Some(temp_dir.to_string_lossy().to_string()),
            glob: Some("src/*.rs".to_string()),
            include_globs: vec![],
            exclude_globs: vec![],
            case_insensitive: false,
            output_mode: GrepOutputMode::Content,
            head_limit: 10,
            max_files: 100,
            offset: 0,
        })
        .await
        .unwrap();

        assert_eq!(output.num_matches, 1);
        assert_eq!(output.content.len(), 1);
        assert_eq!(output.content[0].file_path, "src/main.rs");
        assert_eq!(output.content[0].line_number, 2);
        assert_eq!(output.scanned_files, 2); // both .rs files pass the glob and are opened

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
