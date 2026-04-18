use glob::{glob_with, MatchOptions};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GlobArgs {
    /// Glob pattern, for example `src/**/*.rs`
    pub pattern: String,
    /// Base directory for the pattern. Defaults to the current working directory.
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of files to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    200
}

#[derive(Debug, Clone, Serialize)]
pub struct GlobOutput {
    pub pattern: String,
    pub base_path: String,
    pub filenames: Vec<String>,
    pub num_files: usize,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GlobError {
    #[error("invalid base path: {0}")]
    InvalidBasePath(String),
    #[error("invalid glob pattern: {0}")]
    InvalidPattern(String),
}

#[derive(Debug, Clone, Default)]
pub struct GlobTool;

impl GlobTool {
    pub const NAME: &'static str = "glob";
    pub const DESCRIPTION: &'static str = concat!(
        "Find files by wildcard path pattern inside the current workspace. ",
        "Use this to discover candidate files before reading or editing them. ",
        "Prefer this over shell for filename discovery because results are structured and bounded."
    );
}

impl Tool for GlobTool {
    const NAME: &'static str = Self::NAME;
    type Args = GlobArgs;
    type Output = GlobOutput;
    type Error = GlobError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GlobArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_dir = resolve_base_dir(args.path.as_deref())?;
        let joined_pattern = base_dir.join(&args.pattern);
        let pattern = joined_pattern.to_string_lossy().to_string();
        let options = MatchOptions {
            case_sensitive: true,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };

        let mut filenames = Vec::new();
        let mut truncated = false;
        let limit = args.limit.max(1).min(2000);

        let entries = glob_with(&pattern, options)
            .map_err(|error| GlobError::InvalidPattern(error.to_string()))?;
        for entry in entries {
            let path = match entry {
                Ok(path) => path,
                Err(_) => continue,
            };
            if !path.is_file() {
                continue;
            }
            if filenames.len() >= limit {
                truncated = true;
                break;
            }
            filenames.push(to_relative_or_absolute(&base_dir, &path));
        }

        filenames.sort();
        let num_files = filenames.len();

        Ok(GlobOutput {
            pattern: args.pattern,
            base_path: base_dir.to_string_lossy().to_string(),
            filenames,
            num_files,
            truncated,
        })
    }
}

fn resolve_base_dir(path: Option<&str>) -> Result<PathBuf, GlobError> {
    let cwd =
        std::env::current_dir().map_err(|error| GlobError::InvalidBasePath(error.to_string()))?;
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

fn to_relative_or_absolute(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn glob_returns_relative_matches() {
        let temp_dir = std::env::temp_dir().join(format!("glob-tool-{}", uuid::Uuid::new_v4()));
        let nested = temp_dir.join("src");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(nested.join("main.rs"), "fn main() {}\n")
            .await
            .unwrap();
        tokio::fs::write(nested.join("lib.rs"), "pub fn demo() {}\n")
            .await
            .unwrap();

        let output = GlobTool
            .call(GlobArgs {
                pattern: "src/*.rs".to_string(),
                path: Some(temp_dir.to_string_lossy().to_string()),
                limit: 10,
            })
            .await
            .unwrap();

        assert_eq!(output.num_files, 2);
        assert!(output.filenames.iter().any(|path| path == "src/main.rs"));
        assert!(output.filenames.iter().any(|path| path == "src/lib.rs"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
