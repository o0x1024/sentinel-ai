//! Readback tracking for large stored tool outputs.

use serde::{Deserialize, Serialize};

use crate::agents::context_engineering::checkpoint::ContextRunState;
use crate::agents::context_engineering::tool_digest::{condense_text, ToolDigest};

const MAX_TRACKED_ARTIFACTS: usize = 8;
const MAX_TRACKED_RANGES: usize = 24;
const DEFAULT_READBACK_CHUNK_LINES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ArtifactReadRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackedArtifact {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub storage_backend: String,
    pub source_tool: String,
    #[serde(default)]
    pub source_slot: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<usize>,
    #[serde(default)]
    pub total_lines: Option<usize>,
    #[serde(default)]
    pub read_ranges: Vec<ArtifactReadRange>,
    #[serde(default)]
    pub contiguous_read_through_line: usize,
    #[serde(default)]
    pub last_read_start_line: Option<usize>,
    #[serde(default)]
    pub last_read_end_line: Option<usize>,
    #[serde(default)]
    pub fully_read: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
struct StoredArtifactEvent {
    slot: Option<String>,
    path: String,
    storage_backend: String,
    size_bytes: Option<usize>,
    total_lines: Option<usize>,
}

#[derive(Debug, Clone)]
struct ArtifactReadEvent {
    artifact_id: String,
    start_line: usize,
    end_line: usize,
    total_lines: Option<usize>,
}

pub fn apply_tool_digest_artifact_updates(state: &mut ContextRunState, digest: &ToolDigest) {
    apply_tool_digest_to_tracked_artifacts(&mut state.tracked_artifacts, digest);
}

pub fn apply_tool_digest_to_tracked_artifacts(
    artifacts: &mut Vec<TrackedArtifact>,
    digest: &ToolDigest,
) {
    for stored in extract_stored_artifacts(digest) {
        upsert_stored_artifact(artifacts, digest, stored);
    }

    if let Some(read_event) = extract_artifact_read_event(digest) {
        apply_read_event(artifacts, digest, read_event);
    }

    sort_and_trim_artifacts(artifacts);
}

pub fn has_incomplete_host_artifacts(state: Option<&ContextRunState>) -> bool {
    state
        .map(|state| {
            state.tracked_artifacts.iter().any(|artifact| {
                artifact.storage_backend == "host"
                    && artifact.total_lines.unwrap_or(0) > 0
                    && !artifact.fully_read
            })
        })
        .unwrap_or(false)
}

pub fn render_artifact_readback_summary(artifacts: &[TrackedArtifact]) -> Option<String> {
    if artifacts.is_empty() {
        return None;
    }

    let mut sorted = artifacts.to_vec();
    sorted.sort_by(|left, right| {
        left.fully_read
            .cmp(&right.fully_read)
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
    });

    let mut body = String::from("Artifact Readback:\n");
    for artifact in sorted.iter().take(5) {
        body.push_str("- ");
        body.push_str(if artifact.fully_read {
            "[complete] "
        } else {
            "[pending] "
        });
        body.push_str(&condense_text(&artifact.artifact_id, 120));
        body.push_str(" | ");
        body.push_str(&artifact.storage_backend);
        if let Some(slot) = artifact.source_slot.as_ref() {
            body.push_str(" ");
            body.push_str(slot);
        }
        body.push_str(" via ");
        body.push_str(&artifact.source_tool);
        body.push_str(" | ");
        body.push_str(&render_coverage_summary(artifact));

        if let Some((next_start, next_limit)) = next_readback_chunk(artifact) {
            body.push_str(" | next: ");
            body.push_str(&build_next_step_hint(artifact, next_start, next_limit));
        }
        body.push('\n');
    }

    if sorted.len() > 5 {
        body.push_str("- ...<truncated>...\n");
    }

    Some(body.trim().to_string())
}

fn extract_stored_artifacts(digest: &ToolDigest) -> Vec<StoredArtifactEvent> {
    let Some(metadata) = digest.metadata.as_ref() else {
        return Vec::new();
    };
    let Some(items) = metadata
        .get("stored_artifacts")
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let path = item.get("path").and_then(|value| value.as_str())?;
            Some(StoredArtifactEvent {
                slot: item
                    .get("slot")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                path: path.to_string(),
                storage_backend: item
                    .get("storage_backend")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                size_bytes: item
                    .get("size")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize),
                total_lines: item
                    .get("lines")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize),
            })
        })
        .collect()
}

fn extract_artifact_read_event(digest: &ToolDigest) -> Option<ArtifactReadEvent> {
    let metadata = digest.metadata.as_ref()?;
    let read = metadata.get("artifact_read")?;
    let artifact_id = read.get("artifact_id").and_then(|value| value.as_str())?;
    let start_line = read
        .get("start_line")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let end_line = read
        .get("end_line")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    if start_line == 0 || end_line == 0 || end_line < start_line {
        return None;
    }

    Some(ArtifactReadEvent {
        artifact_id: artifact_id.to_string(),
        start_line,
        end_line,
        total_lines: read
            .get("total_lines")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize),
    })
}

fn upsert_stored_artifact(
    artifacts: &mut Vec<TrackedArtifact>,
    digest: &ToolDigest,
    event: StoredArtifactEvent,
) {
    let now_ms = digest.created_at_ms;
    if let Some(existing) = artifacts
        .iter_mut()
        .find(|artifact| artifact.artifact_id == event.path)
    {
        existing.artifact_kind = digest
            .artifact_kind
            .clone()
            .unwrap_or_else(|| "file".to_string());
        existing.storage_backend = event.storage_backend;
        existing.source_tool = digest.tool_name.clone();
        existing.source_slot = event.slot;
        existing.size_bytes = event.size_bytes.or(existing.size_bytes);
        existing.total_lines = event.total_lines.or(existing.total_lines);
        existing.updated_at_ms = now_ms;
        existing.fully_read = existing
            .total_lines
            .map(|total| existing.contiguous_read_through_line >= total)
            .unwrap_or(existing.fully_read);
        return;
    }

    artifacts.push(TrackedArtifact {
        artifact_id: event.path,
        artifact_kind: digest
            .artifact_kind
            .clone()
            .unwrap_or_else(|| "file".to_string()),
        storage_backend: event.storage_backend,
        source_tool: digest.tool_name.clone(),
        source_slot: event.slot,
        size_bytes: event.size_bytes,
        total_lines: event.total_lines,
        read_ranges: Vec::new(),
        contiguous_read_through_line: 0,
        last_read_start_line: None,
        last_read_end_line: None,
        fully_read: false,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    });
}

fn apply_read_event(
    artifacts: &mut Vec<TrackedArtifact>,
    digest: &ToolDigest,
    event: ArtifactReadEvent,
) {
    let now_ms = digest.created_at_ms;
    let artifact = if let Some(existing) = artifacts
        .iter_mut()
        .find(|artifact| artifact.artifact_id == event.artifact_id)
    {
        existing
    } else {
        artifacts.push(TrackedArtifact {
            artifact_id: event.artifact_id.clone(),
            artifact_kind: digest
                .artifact_kind
                .clone()
                .unwrap_or_else(|| "file".to_string()),
            storage_backend: "unknown".to_string(),
            source_tool: digest.tool_name.clone(),
            source_slot: None,
            size_bytes: None,
            total_lines: event.total_lines,
            read_ranges: Vec::new(),
            contiguous_read_through_line: 0,
            last_read_start_line: None,
            last_read_end_line: None,
            fully_read: false,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        });
        artifacts
            .last_mut()
            .expect("tracked artifact just inserted")
    };

    artifact.total_lines = event.total_lines.or(artifact.total_lines);
    artifact.last_read_start_line = Some(event.start_line);
    artifact.last_read_end_line = Some(event.end_line);
    artifact.updated_at_ms = now_ms;
    artifact.read_ranges.push(ArtifactReadRange {
        start_line: event.start_line,
        end_line: event.end_line,
    });
    merge_ranges(&mut artifact.read_ranges);
    if artifact.read_ranges.len() > MAX_TRACKED_RANGES {
        let keep_from = artifact.read_ranges.len() - MAX_TRACKED_RANGES;
        artifact.read_ranges = artifact.read_ranges.split_off(keep_from);
        merge_ranges(&mut artifact.read_ranges);
    }
    artifact.contiguous_read_through_line = contiguous_read_through_line(&artifact.read_ranges);
    artifact.fully_read = artifact
        .total_lines
        .map(|total| artifact.contiguous_read_through_line >= total)
        .unwrap_or(false);
}

fn sort_and_trim_artifacts(artifacts: &mut Vec<TrackedArtifact>) {
    artifacts.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
    if artifacts.len() > MAX_TRACKED_ARTIFACTS {
        artifacts.truncate(MAX_TRACKED_ARTIFACTS);
    }
}

fn merge_ranges(ranges: &mut Vec<ArtifactReadRange>) {
    ranges.sort_by(|left, right| {
        left.start_line
            .cmp(&right.start_line)
            .then_with(|| left.end_line.cmp(&right.end_line))
    });

    let mut merged: Vec<ArtifactReadRange> = Vec::new();
    for range in ranges.iter() {
        if let Some(last) = merged.last_mut() {
            if range.start_line <= last.end_line.saturating_add(1) {
                last.end_line = last.end_line.max(range.end_line);
                continue;
            }
        }
        merged.push(range.clone());
    }
    *ranges = merged;
}

fn contiguous_read_through_line(ranges: &[ArtifactReadRange]) -> usize {
    let mut contiguous_end = 0usize;
    for range in ranges {
        if range.start_line > contiguous_end.saturating_add(1) {
            break;
        }
        contiguous_end = contiguous_end.max(range.end_line);
    }
    contiguous_end
}

fn render_coverage_summary(artifact: &TrackedArtifact) -> String {
    match artifact.total_lines {
        Some(total_lines) if total_lines > 0 => format!(
            "{} / {} lines sequentially covered",
            artifact.contiguous_read_through_line, total_lines
        ),
        _ => format!("{} read range(s) recorded", artifact.read_ranges.len()),
    }
}

fn next_readback_chunk(artifact: &TrackedArtifact) -> Option<(usize, usize)> {
    if artifact.fully_read {
        return None;
    }
    let total_lines = artifact.total_lines?;
    if total_lines == 0 {
        return None;
    }
    let start_line = artifact.contiguous_read_through_line.saturating_add(1);
    if start_line > total_lines {
        return None;
    }
    let remaining = total_lines.saturating_sub(start_line).saturating_add(1);
    let limit = remaining.min(DEFAULT_READBACK_CHUNK_LINES);
    Some((start_line, limit))
}

fn build_next_step_hint(artifact: &TrackedArtifact, start_line: usize, limit: usize) -> String {
    match artifact.storage_backend.as_str() {
        "host" => format!(
            "file_read {{ \"file_path\": \"{}\", \"offset\": {}, \"limit\": {} }}",
            artifact.artifact_id, start_line, limit
        ),
        "container" => {
            let end_line = start_line.saturating_add(limit).saturating_sub(1);
            format!(
                "shell sed -n '{} ,{}p' {}",
                start_line, end_line, artifact.artifact_id
            )
            .replace(" ,", ",")
        }
        _ => format!(
            "read lines {}-{} from {}",
            start_line,
            start_line.saturating_add(limit).saturating_sub(1),
            artifact.artifact_id
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use serde_json::Value;

    use super::*;

    fn digest_with_metadata(tool_name: &str, created_at_ms: i64, metadata: Value) -> ToolDigest {
        ToolDigest {
            tool_name: tool_name.to_string(),
            status: "ok".to_string(),
            summary: String::new(),
            artifact_id: None,
            artifact_kind: Some("file".to_string()),
            preview_snippets: Vec::new(),
            metadata: Some(metadata),
            created_at_ms,
        }
    }

    #[test]
    fn tracks_stored_artifact_and_sequential_file_reads() {
        let mut state = ContextRunState::default();

        apply_tool_digest_artifact_updates(
            &mut state,
            &digest_with_metadata(
                "http_request",
                10,
                json!({
                    "stored_artifacts": [{
                        "slot": "body",
                        "path": "/tmp/http_response.txt",
                        "storage_backend": "host",
                        "size": 4096,
                        "lines": 380
                    }]
                }),
            ),
        );
        apply_tool_digest_artifact_updates(
            &mut state,
            &digest_with_metadata(
                "file_read",
                20,
                json!({
                    "artifact_read": {
                        "artifact_id": "/tmp/http_response.txt",
                        "start_line": 1,
                        "end_line": 200,
                        "total_lines": 380
                    }
                }),
            ),
        );
        apply_tool_digest_artifact_updates(
            &mut state,
            &digest_with_metadata(
                "file_read",
                30,
                json!({
                    "artifact_read": {
                        "artifact_id": "/tmp/http_response.txt",
                        "start_line": 201,
                        "end_line": 380,
                        "total_lines": 380
                    }
                }),
            ),
        );

        let artifact = state.tracked_artifacts.first().expect("tracked artifact");
        assert_eq!(artifact.contiguous_read_through_line, 380);
        assert!(artifact.fully_read);
        assert_eq!(artifact.storage_backend, "host");
    }

    #[test]
    fn render_summary_includes_next_step_for_pending_artifact() {
        let summary = render_artifact_readback_summary(&[TrackedArtifact {
            artifact_id: "/tmp/out.txt".to_string(),
            artifact_kind: "file".to_string(),
            storage_backend: "host".to_string(),
            source_tool: "http_request".to_string(),
            source_slot: Some("body".to_string()),
            size_bytes: Some(1024),
            total_lines: Some(350),
            read_ranges: vec![ArtifactReadRange {
                start_line: 1,
                end_line: 200,
            }],
            contiguous_read_through_line: 200,
            last_read_start_line: Some(1),
            last_read_end_line: Some(200),
            fully_read: false,
            created_at_ms: 1,
            updated_at_ms: 2,
        }])
        .expect("summary");

        assert!(summary.contains("[pending] /tmp/out.txt"));
        assert!(summary.contains("200 / 350 lines sequentially covered"));
        assert!(summary.contains("\"offset\": 201, \"limit\": 150"));
    }
}
