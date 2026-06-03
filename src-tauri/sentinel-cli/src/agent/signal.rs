use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSignalStatus {
    Continue,
    NeedHint,
    CandidateFlag,
    Done,
    GiveUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSignal {
    pub status: AgentSignalStatus,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub flag: Option<String>,
}

pub fn parse_agent_signal(raw: &str) -> Result<AgentSignal> {
    let candidates = extract_json_candidates(raw);
    if candidates.is_empty() {
        return Err(anyhow!("agent response did not contain a JSON object"));
    }

    let mut last_error = None;
    for candidate in candidates.iter().rev() {
        match serde_json::from_str::<AgentSignal>(candidate) {
            Ok(signal) => return validate_signal(signal),
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    Err(anyhow!(
        "failed to parse agent signal JSON: {}",
        last_error.unwrap_or_else(|| "no valid signal object found".to_string())
    ))
}

pub fn extract_candidate_flags(raw: &str) -> Vec<String> {
    let regex = Regex::new(r"flag\{[^}\r\n]+\}").expect("valid flag regex");
    regex
        .captures_iter(raw)
        .filter_map(|capture| capture.get(0).map(|item| item.as_str().to_string()))
        .collect()
}

fn validate_signal(signal: AgentSignal) -> Result<AgentSignal> {
    match signal.status {
        AgentSignalStatus::CandidateFlag => {
            if signal.flag.as_deref().unwrap_or_default().trim().is_empty() {
                return Err(anyhow!("candidate_flag signal missing flag"));
            }
        }
        AgentSignalStatus::Continue
        | AgentSignalStatus::NeedHint
        | AgentSignalStatus::Done
        | AgentSignalStatus::GiveUp => {}
    }
    Ok(signal)
}

fn extract_json_candidates(raw: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        candidates.push(trimmed.to_string());
    }

    if let Some(fence_start) = trimmed.find("```json").or_else(|| trimmed.find("```")) {
        let after_start = &trimmed[fence_start..];
        if let Some(first_line_end) = after_start.find('\n') {
            let body = &after_start[first_line_end + 1..];
            if let Some(fence_end) = body.find("```") {
                let fenced = body[..fence_end].trim();
                if fenced.starts_with('{') && fenced.ends_with('}') {
                    candidates.push(fenced.to_string());
                }
                candidates.extend(extract_all_json_objects(fenced));
            }
        }
    }

    candidates.extend(extract_all_json_objects(trimmed));
    dedup_candidates(candidates)
}

fn extract_all_json_objects(raw: &str) -> Vec<String> {
    let mut start_indices = Vec::new();
    let mut candidates = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in raw.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start_indices.push(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_indices.pop() {
                        candidates.push(raw[start..=index].trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    candidates
}

fn dedup_candidates(candidates: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for candidate in candidates {
        if !deduped.iter().any(|existing| existing == &candidate) {
            deduped.push(candidate);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::{parse_agent_signal, AgentSignalStatus};

    #[test]
    fn parses_last_valid_signal_when_response_contains_invalid_object_examples() {
        let raw = r#"
The frontend sends JSON.stringify({url}), which is not valid JSON by itself.
I will request a hint now.
{"status":"need_hint","reason":"import endpoint keeps rejecting the request body"}
"#;

        let signal = parse_agent_signal(raw).expect("signal should parse");
        assert!(matches!(signal.status, AgentSignalStatus::NeedHint));
        assert_eq!(
            signal.reason.as_deref(),
            Some("import endpoint keeps rejecting the request body")
        );
    }

    #[test]
    fn parses_fenced_signal_block() {
        let raw = r#"
I found a likely flag.
```json
{"status":"candidate_flag","flag":"flag{abc}","reason":"found in response body"}
```
"#;

        let signal = parse_agent_signal(raw).expect("signal should parse");
        assert!(matches!(signal.status, AgentSignalStatus::CandidateFlag));
        assert_eq!(signal.flag.as_deref(), Some("flag{abc}"));
    }
}
