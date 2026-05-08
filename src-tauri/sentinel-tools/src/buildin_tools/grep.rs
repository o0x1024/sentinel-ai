use glob::Pattern;
use regex::RegexBuilder;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::buildin_tools::file_runtime::{
    current_runtime_metadata, list_files_under, read_path_bytes, FileRuntimeMetadata,
};

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
}

#[derive(Debug, thiserror::Error)]
pub enum GrepError {
    #[error("invalid base path: {0}")]
    InvalidBasePath(String),
    #[error("invalid regex pattern: {0}")]
    InvalidPattern(String),
}

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
            output_mode: match args.output_mode {
                GrepOutputMode::Content => "content".to_string(),
                GrepOutputMode::FilesWithMatches => "files_with_matches".to_string(),
                GrepOutputMode::Count => "count".to_string(),
            },
            filenames,
            content,
            num_matches: num_matches.saturating_sub(args.offset),
            applied_limit,
            scanned_files,
            truncated,
        })
    }
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
        assert_eq!(output.scanned_files, 1);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
