use sha2::{Digest, Sha256};

use crate::output_storage::StoredOutputArtifact;

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

pub fn count_text_lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.lines().count()
    }
}

pub fn build_host_file_artifact(
    slot: impl Into<String>,
    file_path: &str,
    size: usize,
    lines: usize,
) -> StoredOutputArtifact {
    StoredOutputArtifact {
        slot: slot.into(),
        path: file_path.to_string(),
        storage_backend: "host".to_string(),
        size,
        lines,
    }
}
