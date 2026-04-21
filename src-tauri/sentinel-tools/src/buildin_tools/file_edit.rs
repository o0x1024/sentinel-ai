use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::buildin_tools::file_context::{
    build_host_file_artifact, count_text_lines, hash_bytes, hash_text,
};
use crate::buildin_tools::file_runtime::{
    current_runtime_metadata, get_path_kind, read_path_bytes, write_path_bytes, FilePathKind,
    FileRuntimeMetadata,
};
use crate::buildin_tools::text_change::{summarize_text_change, TextChangeSummary};
use crate::buildin_tools::text_preview::condense_preview as condense_text_preview;
use crate::output_storage::StoredOutputArtifact;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FileEditArgs {
    /// Existing file to edit.
    pub file_path: String,
    /// Exact string to replace.
    pub old_string: String,
    /// Replacement text.
    pub new_string: String,
    /// Replace all matches instead of exactly one.
    #[serde(default)]
    pub replace_all: bool,
    /// Optional revision token from a prior file_read result.
    #[serde(default)]
    pub expected_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEditOutput {
    pub file_path: String,
    pub runtime: FileRuntimeMetadata,
    pub replacements: usize,
    pub bytes_written: usize,
    pub content_hash: String,
    pub before_preview: String,
    pub after_preview: String,
    pub change_summary: TextChangeSummary,
    #[serde(default)]
    pub stored_artifacts: Vec<StoredOutputArtifact>,
}

#[derive(Debug, thiserror::Error)]
pub enum FileEditError {
    #[error("invalid file path: {0}")]
    InvalidPath(String),
    #[error("target file does not exist: {0}")]
    MissingFile(String),
    #[error("path is a directory: {0}")]
    IsDirectory(String),
    #[error("old_string cannot be empty")]
    EmptyOldString,
    #[error("expected exactly one match, found {0}")]
    AmbiguousMatch(usize),
    #[error("target text not found")]
    NotFound,
    #[error("file revision mismatch: expected {expected}, actual {actual}")]
    RevisionMismatch { expected: String, actual: String },
    #[error("binary file is not supported")]
    BinaryFile,
    #[error("failed to edit file: {0}")]
    EditFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct FileEditTool;

impl FileEditTool {
    pub const NAME: &'static str = "file_edit";
    pub const DESCRIPTION: &'static str = concat!(
        "Modify an existing text file by exact string replacement. ",
        "Use this for targeted edits when you already know the current snippet to replace. ",
        "By default it requires exactly one match to avoid accidental broad changes."
    );
}

impl Tool for FileEditTool {
    const NAME: &'static str = Self::NAME;
    type Args = FileEditArgs;
    type Output = FileEditOutput;
    type Error = FileEditError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FileEditArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.old_string.is_empty() {
            return Err(FileEditError::EmptyOldString);
        }

        match get_path_kind(&args.file_path)
            .await
            .map_err(FileEditError::EditFailed)?
        {
            FilePathKind::Missing => {
                return Err(FileEditError::MissingFile(args.file_path.clone()))
            }
            FilePathKind::Directory => return Err(FileEditError::IsDirectory(args.file_path)),
            FilePathKind::File => {}
        }

        let bytes = read_path_bytes(&args.file_path)
            .await
            .map_err(FileEditError::EditFailed)?;
        if looks_binary(&bytes) {
            return Err(FileEditError::BinaryFile);
        }
        let current_hash = hash_bytes(&bytes);
        if let Some(expected_hash) = args.expected_hash.as_deref() {
            if current_hash != expected_hash {
                return Err(FileEditError::RevisionMismatch {
                    expected: expected_hash.to_string(),
                    actual: current_hash.clone(),
                });
            }
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| FileEditError::EditFailed(error.to_string()))?;

        let matches = text.matches(&args.old_string).count();
        if matches == 0 {
            return Err(FileEditError::NotFound);
        }
        if !args.replace_all && matches != 1 {
            return Err(FileEditError::AmbiguousMatch(matches));
        }

        let updated = if args.replace_all {
            text.replace(&args.old_string, &args.new_string)
        } else {
            text.replacen(&args.old_string, &args.new_string, 1)
        };
        let before_preview = preview_around_marker(&text, &args.old_string);
        let after_preview = preview_around_marker(&updated, &args.new_string);
        let change_summary = summarize_text_change(&text, &updated);
        write_path_bytes(&args.file_path, updated.as_bytes(), false)
            .await
            .map_err(FileEditError::EditFailed)?;
        let content_hash = hash_text(&updated);
        let stored_artifacts = vec![build_host_file_artifact(
            "file",
            &args.file_path,
            updated.len(),
            count_text_lines(&updated),
        )];

        Ok(FileEditOutput {
            file_path: args.file_path,
            runtime: current_runtime_metadata(),
            replacements: if args.replace_all { matches } else { 1 },
            bytes_written: updated.len(),
            content_hash,
            before_preview,
            after_preview,
            change_summary,
            stored_artifacts,
        })
    }
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).any(|byte| *byte == 0)
}

fn preview_around_marker(text: &str, marker: &str) -> String {
    if marker.is_empty() {
        return condense_text_preview(text, 120);
    }

    let Some(start) = text.find(marker) else {
        return condense_text_preview(text, 120);
    };
    let end = start.saturating_add(marker.len());
    let preview_start = text[..start]
        .char_indices()
        .rev()
        .nth(40)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let preview_end = text[end..]
        .char_indices()
        .nth(40)
        .map(|(idx, _)| end + idx)
        .unwrap_or(text.len());
    condense_text_preview(&text[preview_start..preview_end], 120)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_edit_replaces_single_match() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-edit-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "hello world\n").await.unwrap();

        let output = FileEditTool
            .call(FileEditArgs {
                file_path: file_path.to_string_lossy().to_string(),
                old_string: "world".to_string(),
                new_string: "sentinel".to_string(),
                replace_all: false,
                expected_hash: None,
            })
            .await
            .unwrap();

        let updated = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(output.replacements, 1);
        assert_eq!(updated, "hello sentinel\n");
        assert_eq!(output.content_hash.len(), 64);
        assert!(output.before_preview.contains("world"));
        assert!(output.after_preview.contains("sentinel"));
        assert_eq!(output.change_summary.first_changed_line, Some(1));
        assert_eq!(output.change_summary.changed_line_count, 1);
        assert_eq!(
            output.change_summary.after_preview.as_deref(),
            Some("hello sentinel")
        );
        assert_eq!(output.stored_artifacts.len(), 1);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn file_edit_rejects_stale_expected_hash() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-edit-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "hello world\n").await.unwrap();

        let error = FileEditTool
            .call(FileEditArgs {
                file_path: file_path.to_string_lossy().to_string(),
                old_string: "world".to_string(),
                new_string: "sentinel".to_string(),
                replace_all: false,
                expected_hash: Some("deadbeef".to_string()),
            })
            .await
            .expect_err("stale hash should be rejected");

        assert!(matches!(error, FileEditError::RevisionMismatch { .. }));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
