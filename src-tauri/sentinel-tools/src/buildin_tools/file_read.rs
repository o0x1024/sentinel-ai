use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use crate::buildin_tools::file_context::hash_bytes;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FileReadArgs {
    /// File path to read.
    pub file_path: String,
    /// Starting line number, 1-based. Defaults to 1.
    #[serde(default = "default_start_line")]
    pub offset: usize,
    /// Maximum number of lines to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_start_line() -> usize {
    1
}

fn default_limit() -> usize {
    200
}

#[derive(Debug, Clone, Serialize)]
pub struct FileReadOutput {
    pub file_path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub total_lines: usize,
    pub truncated: bool,
    pub content_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FileReadError {
    #[error("invalid file path: {0}")]
    InvalidPath(String),
    #[error("file is a directory: {0}")]
    IsDirectory(String),
    #[error("binary file is not supported")]
    BinaryFile,
    #[error("failed to read file: {0}")]
    ReadFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct FileReadTool;

impl FileReadTool {
    pub const NAME: &'static str = "file_read";
    pub const DESCRIPTION: &'static str = concat!(
        "Read a text file with line-range controls. ",
        "Use this for code or config inspection when you need exact lines instead of shell output. ",
        "The result is bounded, preserves line order, and rejects binary files."
    );
}

impl Tool for FileReadTool {
    const NAME: &'static str = Self::NAME;
    type Args = FileReadArgs;
    type Output = FileReadOutput;
    type Error = FileReadError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FileReadArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&args.file_path)?;
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| FileReadError::ReadFailed(error.to_string()))?;
        if metadata.is_dir() {
            return Err(FileReadError::IsDirectory(args.file_path));
        }

        let mut sample_file = tokio::fs::File::open(&path)
            .await
            .map_err(|error| FileReadError::ReadFailed(error.to_string()))?;
        let mut sample = [0u8; 1024];
        let sample_len = sample_file
            .read(&mut sample)
            .await
            .map_err(|error| FileReadError::ReadFailed(error.to_string()))?;
        if looks_binary(&sample[..sample_len]) {
            return Err(FileReadError::BinaryFile);
        }

        let start_line = args.offset.max(1);
        let line_limit = args.limit.max(1).min(2000);
        let read_result = read_file_range(&path, start_line, line_limit).await?;
        let start_index = start_line.saturating_sub(1).min(read_result.total_lines);
        let end_index = start_index
            .saturating_add(read_result.selected_lines.len())
            .min(read_result.total_lines);
        let total_lines = read_result.total_lines;
        let truncated = end_index < total_lines;
        let content = read_result.selected_lines.join("\n");
        let end_line = if end_index == 0 { 0 } else { end_index };

        Ok(FileReadOutput {
            file_path: args.file_path,
            content,
            start_line,
            end_line,
            total_lines,
            truncated,
            content_hash: read_result.content_hash,
        })
    }
}

struct FileReadRangeResult {
    selected_lines: Vec<String>,
    total_lines: usize,
    content_hash: String,
}

async fn read_file_range(
    path: &Path,
    start_line: usize,
    line_limit: usize,
) -> Result<FileReadRangeResult, FileReadError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|error| FileReadError::ReadFailed(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut line_buffer = String::new();
    let mut selected_lines = Vec::new();
    let mut total_lines = 0usize;
    let mut content_bytes = Vec::new();

    loop {
        line_buffer.clear();
        let bytes_read = reader
            .read_line(&mut line_buffer)
            .await
            .map_err(|error| FileReadError::ReadFailed(error.to_string()))?;
        if bytes_read == 0 {
            break;
        }

        total_lines += 1;
        content_bytes.extend_from_slice(line_buffer.as_bytes());

        if total_lines >= start_line && selected_lines.len() < line_limit {
            selected_lines.push(trim_line_ending(&line_buffer).to_string());
        }
    }

    Ok(FileReadRangeResult {
        selected_lines,
        total_lines,
        content_hash: hash_bytes(&content_bytes),
    })
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

fn resolve_path(raw: &str) -> Result<PathBuf, FileReadError> {
    if raw.trim().is_empty() {
        return Err(FileReadError::InvalidPath(
            "path cannot be empty".to_string(),
        ));
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        let cwd = std::env::current_dir()
            .map_err(|error| FileReadError::InvalidPath(error.to_string()))?;
        Ok(cwd.join(path))
    }
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).any(|byte| *byte == 0)
}

#[allow(dead_code)]
fn _to_display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_read_respects_offset_and_limit() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-read-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "a\nb\nc\nd\n").await.unwrap();

        let output = FileReadTool
            .call(FileReadArgs {
                file_path: file_path.to_string_lossy().to_string(),
                offset: 2,
                limit: 2,
            })
            .await
            .unwrap();

        assert_eq!(output.start_line, 2);
        assert_eq!(output.end_line, 3);
        assert_eq!(output.total_lines, 4);
        assert_eq!(output.content, "b\nc");
        assert!(output.truncated);
        assert_eq!(output.content_hash.len(), 64);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
