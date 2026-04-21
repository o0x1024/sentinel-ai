use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::buildin_tools::file_context::hash_bytes;
use crate::buildin_tools::file_runtime::{
    current_runtime_metadata, get_path_kind, read_path_bytes, FilePathKind, FileRuntimeMetadata,
};

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
    pub runtime: FileRuntimeMetadata,
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
        if matches!(
            get_path_kind(&args.file_path)
                .await
                .map_err(FileReadError::ReadFailed)?,
            FilePathKind::Directory
        ) {
            return Err(FileReadError::IsDirectory(args.file_path));
        }

        let file_bytes = read_path_bytes(&args.file_path)
            .await
            .map_err(FileReadError::ReadFailed)?;
        if looks_binary(&file_bytes[..file_bytes.len().min(1024)]) {
            return Err(FileReadError::BinaryFile);
        }

        let start_line = args.offset.max(1);
        let line_limit = args.limit.max(1).min(2000);
        let read_result = read_file_range(&file_bytes, start_line, line_limit)?;
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
            runtime: current_runtime_metadata(),
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

fn read_file_range(
    file_bytes: &[u8],
    start_line: usize,
    line_limit: usize,
) -> Result<FileReadRangeResult, FileReadError> {
    let cursor = std::io::Cursor::new(file_bytes);
    let mut reader = BufReader::new(cursor);
    let mut line_buffer = String::new();
    let mut selected_lines = Vec::new();
    let mut total_lines = 0usize;
    let mut content_bytes = Vec::new();

    loop {
        line_buffer.clear();
        let bytes_read = reader.read_line(&mut line_buffer).map_err(|error| {
            FileReadError::ReadFailed(format!("failed to parse text lines: {}", error))
        })?;
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
