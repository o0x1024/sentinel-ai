use glob::Pattern;
use regex::RegexBuilder;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    /// Result mode.
    #[serde(default = "default_output_mode")]
    pub output_mode: GrepOutputMode,
    /// Maximum number of matches or files to return.
    #[serde(default = "default_head_limit")]
    pub head_limit: usize,
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
    pub output_mode: String,
    pub filenames: Vec<String>,
    pub content: Vec<GrepContentMatch>,
    pub num_matches: usize,
    pub applied_limit: usize,
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
        let base_dir = resolve_base_dir(args.path.as_deref())?;
        let regex = RegexBuilder::new(&args.pattern)
            .multi_line(false)
            .build()
            .map_err(|error| GrepError::InvalidPattern(error.to_string()))?;
        let glob = args
            .glob
            .as_deref()
            .and_then(|value| Pattern::new(value).ok());
        let applied_limit = args.head_limit.max(1).min(1000);

        let mut filenames = Vec::new();
        let mut content = Vec::new();
        let mut num_matches = 0_usize;
        let mut emitted = 0_usize;
        let mut truncated = false;

        'files: for entry in WalkDir::new(&base_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = to_relative_or_absolute(&base_dir, path);
            if let Some(pattern) = &glob {
                if !(pattern.matches(&relative) || pattern.matches_path(path)) {
                    continue;
                }
            }

            let bytes = match tokio::fs::read(path).await {
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
            base_path: base_dir.to_string_lossy().to_string(),
            output_mode: match args.output_mode {
                GrepOutputMode::Content => "content".to_string(),
                GrepOutputMode::FilesWithMatches => "files_with_matches".to_string(),
                GrepOutputMode::Count => "count".to_string(),
            },
            filenames,
            content,
            num_matches: num_matches.saturating_sub(args.offset),
            applied_limit,
            truncated,
        })
    }
}

fn resolve_base_dir(path: Option<&str>) -> Result<PathBuf, GrepError> {
    let cwd =
        std::env::current_dir().map_err(|error| GrepError::InvalidBasePath(error.to_string()))?;
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

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).any(|byte| *byte == 0)
}

fn to_relative_or_absolute(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
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
                output_mode: GrepOutputMode::Content,
                head_limit: 10,
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
}
