#[derive(Debug, Clone, Default)]
pub struct HypothesisTracker {
    focus_hint: Option<String>,
}

impl HypothesisTracker {
    pub fn observe_text(&mut self, text: &str) {
        if let Some(focus_hint) = extract_focus_hint(text) {
            self.focus_hint = Some(focus_hint);
        }
    }

    pub fn focus_hint(&self) -> Option<&str> {
        self.focus_hint.as_deref()
    }

    pub fn clear(&mut self) {
        self.focus_hint = None;
    }
}

fn extract_focus_hint(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut best: Option<String> = None;
    for segment in split_segments(trimmed) {
        if !looks_like_hypothesis_segment(segment) {
            continue;
        }

        let compact = segment.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.len() >= 12 {
            best = Some(limit_len(&compact, 240));
        }
    }

    best.or_else(|| extract_path_or_symbol_focus(trimmed))
}

fn split_segments(text: &str) -> Vec<&str> {
    text.split(|ch| matches!(ch, '\n' | '\r' | '。' | '！' | '？' | ';'))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn looks_like_hypothesis_segment(segment: &str) -> bool {
    let lower = segment.to_lowercase();
    let markers = [
        "可以确定",
        "根因",
        "问题在",
        "问题出在",
        "问题就是",
        "应该",
        "必须",
        "最佳方案",
        "唯一方案",
        "root cause",
        "the issue is",
        "the problem is",
        "should",
        "must",
        "best approach",
    ];

    markers.iter().any(|marker| lower.contains(marker))
        && (contains_path_like_token(segment) || contains_symbol_like_token(segment))
}

fn contains_path_like_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let trimmed = token.trim_matches(|ch: char| "()[]{}'\",:".contains(ch));
        trimmed.contains('/') || trimmed.ends_with(".rs") || trimmed.ends_with(".ts")
    })
}

fn contains_symbol_like_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let trimmed = token.trim_matches(|ch: char| "()[]{}'\",:".contains(ch));
        trimmed.contains("::") || trimmed.contains('_') || trimmed.ends_with("()")
    })
}

fn extract_path_or_symbol_focus(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|token| token.trim_matches(|ch: char| "()[]{}'\",:".contains(ch)))
        .find(|token| {
            !token.is_empty()
                && (token.contains('/') || token.contains("::") || token.ends_with(".rs"))
        })
        .map(|token| limit_len(token, 240))
}

fn limit_len(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::HypothesisTracker;

    #[test]
    fn tracker_extracts_focus_from_claim_with_path() {
        let mut tracker = HypothesisTracker::default();
        tracker
            .observe_text("可以确定问题出在 src-tauri/src/agents/tenth_man.rs，这里应该优先检查。");
        assert_eq!(
            tracker.focus_hint(),
            Some("可以确定问题出在 src-tauri/src/agents/tenth_man.rs，这里应该优先检查。")
        );
    }

    #[test]
    fn tracker_ignores_plain_short_text() {
        let mut tracker = HypothesisTracker::default();
        tracker.observe_text("先看一下这里");
        assert_eq!(tracker.focus_hint(), None);
    }
}
