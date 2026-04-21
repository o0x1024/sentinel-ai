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

const MISSING_REVISION_TOKEN: &str = "missing";

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FileWriteArgs {
    /// File path to create or overwrite.
    pub file_path: String,
    /// New file contents.
    pub content: String,
    /// Allow overwriting an existing file.
    #[serde(default)]
    pub overwrite: bool,
    /// Optional revision token from a prior file_read result.
    #[serde(default)]
    pub expected_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileWriteOutput {
    pub file_path: String,
    pub runtime: FileRuntimeMetadata,
    pub operation: String,
    pub created: bool,
    pub bytes_written: usize,
    pub content_hash: String,
    #[serde(default)]
    pub previous_preview: Option<String>,
    pub content_preview: String,
    pub change_summary: TextChangeSummary,
    #[serde(default)]
    pub stored_artifacts: Vec<StoredOutputArtifact>,
}

#[derive(Debug, thiserror::Error)]
pub enum FileWriteError {
    #[error("invalid file path: {0}")]
    InvalidPath(String),
    #[error("path is a directory: {0}")]
    IsDirectory(String),
    #[error("target file already exists and overwrite=false")]
    AlreadyExists,
    #[error("file revision mismatch: expected {expected}, actual {actual}")]
    RevisionMismatch { expected: String, actual: String },
    #[error("failed to write file: {0}")]
    WriteFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct FileWriteTool;

impl FileWriteTool {
    pub const NAME: &'static str = "file_write";
    pub const DESCRIPTION: &'static str = concat!(
        "Create a new text file or overwrite an existing one when explicitly allowed. ",
        "Use this for full-file generation or deliberate rewrites. ",
        "Set overwrite=true if the target file already exists. ",
        "Use expected_hash to guard against stale writes; pass expected_hash=\"missing\" to require create-if-absent."
    );
}

impl Tool for FileWriteTool {
    const NAME: &'static str = Self::NAME;
    type Args = FileWriteArgs;
    type Output = FileWriteOutput;
    type Error = FileWriteError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(FileWriteArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut previous_preview = None;
        let mut previous_content = String::new();
        let mut current_hash = None;
        let expects_missing = args.expected_hash.as_deref() == Some(MISSING_REVISION_TOKEN);
        let path_kind = get_path_kind(&args.file_path)
            .await
            .map_err(FileWriteError::WriteFailed)?;
        if matches!(path_kind, FilePathKind::Directory) {
            return Err(FileWriteError::IsDirectory(args.file_path));
        }
        if matches!(path_kind, FilePathKind::File) {
            let existing_bytes = read_path_bytes(&args.file_path)
                .await
                .map_err(FileWriteError::WriteFailed)?;
            let existing = String::from_utf8(existing_bytes.clone())
                .map_err(|error| FileWriteError::WriteFailed(error.to_string()))?;
            current_hash = Some(hash_bytes(&existing_bytes));
            if expects_missing {
                return Err(FileWriteError::RevisionMismatch {
                    expected: MISSING_REVISION_TOKEN.to_string(),
                    actual: current_hash
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                });
            }
            if !args.overwrite {
                return Err(FileWriteError::AlreadyExists);
            }
            previous_content = existing.clone();
            previous_preview = Some(condense_text_preview(&existing, 120));
        }

        if let Some(expected_hash) = args.expected_hash.as_deref() {
            match current_hash.as_deref() {
                Some(actual_hash) if actual_hash == expected_hash => {}
                Some(actual_hash) => {
                    return Err(FileWriteError::RevisionMismatch {
                        expected: expected_hash.to_string(),
                        actual: actual_hash.to_string(),
                    });
                }
                None if expected_hash == MISSING_REVISION_TOKEN => {}
                None => {
                    return Err(FileWriteError::RevisionMismatch {
                        expected: expected_hash.to_string(),
                        actual: "missing".to_string(),
                    });
                }
            }
        }

        let created = matches!(path_kind, FilePathKind::Missing);
        let operation = if created { "create" } else { "overwrite" }.to_string();
        let content_preview = condense_text_preview(&args.content, 120);
        let change_summary = summarize_text_change(&previous_content, &args.content);
        write_path_bytes(&args.file_path, args.content.as_bytes(), created)
            .await
            .map_err(FileWriteError::WriteFailed)?;
        let content_hash = hash_text(&args.content);
        let stored_artifacts = vec![build_host_file_artifact(
            "file",
            &args.file_path,
            args.content.len(),
            count_text_lines(&args.content),
        )];

        Ok(FileWriteOutput {
            file_path: args.file_path,
            runtime: current_runtime_metadata(),
            operation,
            created,
            bytes_written: args.content.len(),
            content_hash,
            previous_preview,
            content_preview,
            change_summary,
            stored_artifacts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_write_creates_new_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-write-tool-{}", uuid::Uuid::new_v4()));
        let file_path = temp_dir.join("nested").join("sample.txt");

        let output = FileWriteTool
            .call(FileWriteArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: "hello\n".to_string(),
                overwrite: false,
                expected_hash: None,
            })
            .await
            .unwrap();

        let written = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(output.created);
        assert_eq!(output.operation, "create");
        assert_eq!(written, "hello\n");
        assert_eq!(output.content_hash.len(), 64);
        assert_eq!(output.previous_preview, None);
        assert_eq!(output.content_preview, "hello");
        assert_eq!(output.change_summary.first_changed_line, Some(1));
        assert_eq!(output.change_summary.added_line_count, 1);
        assert_eq!(output.change_summary.before_preview, None);
        assert_eq!(
            output.change_summary.after_preview.as_deref(),
            Some("hello")
        );
        assert_eq!(output.stored_artifacts.len(), 1);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn file_write_rejects_stale_expected_hash() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-write-tool-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "original\n").await.unwrap();

        let error = FileWriteTool
            .call(FileWriteArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: "updated\n".to_string(),
                overwrite: true,
                expected_hash: Some("deadbeef".to_string()),
            })
            .await
            .expect_err("stale hash should be rejected");

        assert!(matches!(error, FileWriteError::RevisionMismatch { .. }));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn file_write_supports_create_if_absent_revision_token() {
        let temp_dir =
            std::env::temp_dir().join(format!("file-write-tool-{}", uuid::Uuid::new_v4()));
        let file_path = temp_dir.join("nested").join("sample.txt");

        let output = FileWriteTool
            .call(FileWriteArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: "created\n".to_string(),
                overwrite: false,
                expected_hash: Some(MISSING_REVISION_TOKEN.to_string()),
            })
            .await
            .unwrap();

        assert!(output.created);

        let error = FileWriteTool
            .call(FileWriteArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: "again\n".to_string(),
                overwrite: false,
                expected_hash: Some(MISSING_REVISION_TOKEN.to_string()),
            })
            .await
            .expect_err("existing file should fail create-if-absent guard");

        assert!(matches!(error, FileWriteError::RevisionMismatch { .. }));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
