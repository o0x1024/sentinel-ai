use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TextChangeSummary {
    pub previous_line_count: usize,
    pub current_line_count: usize,
    pub removed_line_count: usize,
    pub added_line_count: usize,
    pub changed_line_count: usize,
    pub first_changed_line: Option<usize>,
    #[serde(default)]
    pub before_preview: Option<String>,
    #[serde(default)]
    pub after_preview: Option<String>,
}

pub fn summarize_text_change(previous: &str, current: &str) -> TextChangeSummary {
    let previous_lines = collect_lines(previous);
    let current_lines = collect_lines(current);

    let shared_prefix = previous_lines
        .iter()
        .zip(current_lines.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let max_shared_suffix = previous_lines
        .len()
        .saturating_sub(shared_prefix)
        .min(current_lines.len().saturating_sub(shared_prefix));
    let shared_suffix = previous_lines[shared_prefix..]
        .iter()
        .rev()
        .zip(current_lines[shared_prefix..].iter().rev())
        .take(max_shared_suffix)
        .take_while(|(left, right)| left == right)
        .count();

    let previous_changed_end = previous_lines.len().saturating_sub(shared_suffix);
    let current_changed_end = current_lines.len().saturating_sub(shared_suffix);
    let previous_changed = &previous_lines[shared_prefix..previous_changed_end];
    let current_changed = &current_lines[shared_prefix..current_changed_end];

    let first_changed_line = if previous_changed.is_empty() && current_changed.is_empty() {
        None
    } else {
        Some(shared_prefix + 1)
    };

    TextChangeSummary {
        previous_line_count: previous_lines.len(),
        current_line_count: current_lines.len(),
        removed_line_count: previous_changed.len(),
        added_line_count: current_changed.len(),
        changed_line_count: previous_changed.len().max(current_changed.len()),
        first_changed_line,
        before_preview: build_changed_preview(previous_changed),
        after_preview: build_changed_preview(current_changed),
    }
}

fn collect_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.lines().collect()
}

fn build_changed_preview(lines: &[&str]) -> Option<String> {
    if lines.is_empty() {
        return None;
    }

    let joined = lines.join("\n");
    Some(condense_preview(joined.trim(), 160))
}

fn condense_preview(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let head_len = max_chars.saturating_sub(20).max(20);
    let head: String = text.chars().take(head_len).collect();
    format!("{}...", head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_text_change_tracks_replaced_block() {
        let summary = summarize_text_change(
            "alpha\nbeta\ngamma\nomega\n",
            "alpha\nbeta changed\ngamma changed\nomega\n",
        );

        assert_eq!(summary.previous_line_count, 4);
        assert_eq!(summary.current_line_count, 4);
        assert_eq!(summary.removed_line_count, 2);
        assert_eq!(summary.added_line_count, 2);
        assert_eq!(summary.changed_line_count, 2);
        assert_eq!(summary.first_changed_line, Some(2));
        assert_eq!(summary.before_preview.as_deref(), Some("beta\ngamma"));
        assert_eq!(
            summary.after_preview.as_deref(),
            Some("beta changed\ngamma changed")
        );
    }

    #[test]
    fn summarize_text_change_tracks_new_file_content() {
        let summary = summarize_text_change("", "first\nsecond\n");

        assert_eq!(summary.previous_line_count, 0);
        assert_eq!(summary.current_line_count, 2);
        assert_eq!(summary.removed_line_count, 0);
        assert_eq!(summary.added_line_count, 2);
        assert_eq!(summary.changed_line_count, 2);
        assert_eq!(summary.first_changed_line, Some(1));
        assert_eq!(summary.before_preview, None);
        assert_eq!(summary.after_preview.as_deref(), Some("first\nsecond"));
    }
}
