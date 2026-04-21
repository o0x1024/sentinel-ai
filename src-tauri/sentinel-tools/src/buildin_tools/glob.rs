use glob::Pattern;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::buildin_tools::file_runtime::{
    current_runtime_metadata, list_files_under, FileRuntimeMetadata,
};

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
    pub runtime: FileRuntimeMetadata,
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
        let listing = list_files_under(args.path.as_deref())
            .await
            .map_err(GlobError::InvalidBasePath)?;
        let pattern = Pattern::new(&args.pattern)
            .map_err(|error| GlobError::InvalidPattern(error.to_string()))?;

        let mut filenames = Vec::new();
        let mut truncated = false;
        let limit = args.limit.max(1).min(2000);

        for entry in &listing.files {
            if !(pattern.matches(&entry.display_path)
                || pattern.matches_path(std::path::Path::new(&entry.logical_path)))
            {
                continue;
            }
            if filenames.len() >= limit {
                truncated = true;
                break;
            }
            filenames.push(entry.display_path.clone());
        }

        filenames.sort();
        let num_files = filenames.len();

        Ok(GlobOutput {
            pattern: args.pattern,
            base_path: listing.base_path,
            runtime: current_runtime_metadata(),
            filenames,
            num_files,
            truncated,
        })
    }
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
