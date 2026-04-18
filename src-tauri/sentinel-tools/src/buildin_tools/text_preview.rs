pub fn condense_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let head_len = max_chars.saturating_sub(20).max(20);
    let head: String = trimmed.chars().take(head_len).collect();
    format!("{}...", head)
}
