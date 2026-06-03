use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Serialize;

use crate::services::system_agents::BrowserShellFrame;

const MAX_RECENT_INPUTS: usize = 3;
const MAX_TERMINAL_TEXT_CHARS: usize = 6000;

#[derive(Debug, Clone, Serialize)]
pub struct BrowserShellReadFramesSummary {
    pub session_id: String,
    pub inspected_frame_count: usize,
    pub recent_inputs: Vec<String>,
    pub terminal_text: String,
    pub terminal_text_truncated: bool,
    pub prompt_detected: bool,
    pub last_received_at: Option<DateTime<Utc>>,
}

pub fn summarize_browser_shell_frames(
    session_id: &str,
    frames: &[BrowserShellFrame],
) -> BrowserShellReadFramesSummary {
    let prompt_regex = Regex::new(r"(^|\n)[^\n]{0,120}[$#%>] $").expect("valid prompt regex");
    let mut recent_inputs = Vec::<String>::new();
    let mut output_chunks = Vec::<String>::new();
    let mut last_received_at = None;

    for frame in frames {
        last_received_at = Some(frame.received_at);
        let text = decode_frame_text(frame);
        if text.is_empty() {
            continue;
        }

        if frame.direction == "out" {
            let normalized_input = normalize_input_chunk(&text);
            if normalized_input.is_empty() {
                continue;
            }
            if recent_inputs.last() == Some(&normalized_input) {
                continue;
            }
            recent_inputs.push(normalized_input);
            if recent_inputs.len() > MAX_RECENT_INPUTS {
                recent_inputs.remove(0);
            }
            continue;
        }

        let normalized_output = normalize_output_chunk(&text);
        if normalized_output.is_empty() {
            continue;
        }
        if should_drop_as_echo(&normalized_output, recent_inputs.last()) {
            continue;
        }
        if output_chunks.last() == Some(&normalized_output) {
            continue;
        }
        output_chunks.push(normalized_output);
    }

    let mut terminal_text = output_chunks.join("");
    let mut terminal_text_truncated = false;
    if terminal_text.chars().count() > MAX_TERMINAL_TEXT_CHARS {
        terminal_text = terminal_text
            .chars()
            .rev()
            .take(MAX_TERMINAL_TEXT_CHARS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        terminal_text_truncated = true;
    }

    BrowserShellReadFramesSummary {
        session_id: session_id.trim().to_string(),
        inspected_frame_count: frames.len(),
        recent_inputs,
        prompt_detected: prompt_regex.is_match(&terminal_text),
        terminal_text,
        terminal_text_truncated,
        last_received_at,
    }
}

fn decode_frame_text(frame: &BrowserShellFrame) -> String {
    if let Some(payload_base64) = frame.payload_base64.as_deref() {
        let trimmed = payload_base64.trim();
        if !trimmed.is_empty() {
            if let Ok(bytes) = BASE64_STANDARD.decode(trimmed) {
                return String::from_utf8_lossy(&bytes).to_string();
            }
        }
    }

    frame.text_preview.clone().unwrap_or_default()
}

fn normalize_input_chunk(raw: &str) -> String {
    normalize_terminal_text(raw)
        .trim()
        .trim_end_matches(['\n', '\r'])
        .to_string()
}

fn normalize_output_chunk(raw: &str) -> String {
    let normalized = normalize_terminal_text(raw);
    if normalized.trim().is_empty() {
        return String::new();
    }
    normalized
}

fn normalize_terminal_text(raw: &str) -> String {
    let ansi_regex = Regex::new(r"\x1B\[[0-9;?]*[ -/]*[@-~]").expect("valid ansi regex");
    let without_ansi = ansi_regex.replace_all(raw, "");
    without_ansi
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "")
}

fn should_drop_as_echo(output: &str, latest_input: Option<&String>) -> bool {
    let Some(latest_input) = latest_input else {
        return false;
    };
    let output_trimmed = output.trim();
    if output_trimmed.is_empty() || output_trimmed.len() > 512 {
        return false;
    }
    output_trimmed == latest_input.trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::system_agents::BrowserShellFrame;

    fn frame(
        direction: &str,
        text_preview: Option<&str>,
        payload_base64: Option<&str>,
    ) -> BrowserShellFrame {
        BrowserShellFrame {
            id: "frame-1".to_string(),
            session_id: "session-1".to_string(),
            direction: direction.to_string(),
            frame_type: "text".to_string(),
            text_preview: text_preview.map(ToOwned::to_owned),
            payload_base64: payload_base64.map(ToOwned::to_owned),
            received_at: Utc::now(),
        }
    }

    #[test]
    fn summarize_frames_omits_base64_and_frame_metadata() {
        let payload = BASE64_STANDARD.encode("ls\r\n");
        let frames = vec![
            frame("out", Some("ls"), Some(&payload)),
            frame("in", Some("ls"), Some(&payload)),
            frame("in", Some("file-a\nfile-b\nprompt$ "), None),
        ];

        let summary = summarize_browser_shell_frames("session-1", &frames);

        assert_eq!(summary.recent_inputs, vec!["ls"]);
        assert_eq!(summary.terminal_text, "file-a\nfile-b\nprompt$ ");
        assert!(summary.prompt_detected);
        assert_eq!(summary.inspected_frame_count, 3);
    }

    #[test]
    fn summarize_frames_keeps_recent_tail_when_output_is_large() {
        let large = "x".repeat(MAX_TERMINAL_TEXT_CHARS + 200);
        let frames = vec![frame("in", Some(&large), None)];

        let summary = summarize_browser_shell_frames("session-1", &frames);

        assert!(summary.terminal_text_truncated);
        assert_eq!(
            summary.terminal_text.chars().count(),
            MAX_TERMINAL_TEXT_CHARS
        );
    }
}
