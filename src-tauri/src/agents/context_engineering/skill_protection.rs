//! Skill content protection during context trimming.
//!
//! Ensures that skill instructions survive system prompt and run-state
//! condensation by extracting them before trimming and re-injecting
//! them afterward within a dedicated budget.

use super::tool_digest::condense_text;

pub(crate) const SKILL_PROMPT_START: &str = "<!-- SKILL_PROMPT_START -->";
pub(crate) const SKILL_PROMPT_END: &str = "<!-- SKILL_PROMPT_END -->";

const SKILL_INSTRUCTIONS_HEADER: &str = "[Skill Instructions]\n";
pub(crate) const SKILL_INSTRUCTIONS_END: &str = "<!-- SKILL_INSTRUCTIONS_END -->";

/// Content above this marker is core (always kept); content below is
/// reference material that can be dropped under budget pressure.
const SKILL_DETAIL_SEPARATOR: &str = "<!-- SKILL_DETAIL -->";

const SKILL_BUDGET_RATIO: f64 = 0.20;
const SKILL_MIN_CHARS: usize = 400;

/// Extract the skill prompt block from the system prompt text.
/// Returns `(text_without_block, Some(inner_content))` or the
/// original text unchanged when no markers are found.
pub(crate) fn extract_skill_prompt(text: &str) -> (String, Option<String>) {
    let Some(start) = text.find(SKILL_PROMPT_START) else {
        return (text.to_string(), None);
    };
    let Some(end_rel) = text[start..].find(SKILL_PROMPT_END) else {
        return (text.to_string(), None);
    };
    let content_start = start + SKILL_PROMPT_START.len();
    let content_end = start + end_rel;
    let block_end = content_end + SKILL_PROMPT_END.len();

    let inner = text[content_start..content_end].trim().to_string();
    let mut without = String::with_capacity(text.len());
    without.push_str(&text[..start]);
    without.push_str(&text[block_end..]);

    if inner.is_empty() {
        (without, None)
    } else {
        (without, Some(inner))
    }
}

/// Extract the `[Skill Instructions]` block from run-state text.
pub(crate) fn extract_skill_instructions(text: &str) -> (String, Option<String>) {
    let Some(header_pos) = text.find(SKILL_INSTRUCTIONS_HEADER) else {
        return (text.to_string(), None);
    };
    let search_start = header_pos + SKILL_INSTRUCTIONS_HEADER.len();
    let Some(end_rel) = text[search_start..].find(SKILL_INSTRUCTIONS_END) else {
        return (text.to_string(), None);
    };
    let content_end = search_start + end_rel;
    let block_end = content_end + SKILL_INSTRUCTIONS_END.len();

    let inner = text[search_start..content_end].trim().to_string();
    let mut without = String::with_capacity(text.len());
    without.push_str(text[..header_pos].trim_end_matches('\n'));
    let remaining = text[block_end..].trim_start_matches('\n');
    if !remaining.is_empty() {
        if !without.is_empty() {
            without.push_str("\n\n");
        }
        without.push_str(remaining);
    }

    if inner.is_empty() {
        (without, None)
    } else {
        (without, Some(inner))
    }
}

/// Wrap raw skill prompt content with extraction markers.
pub(crate) fn wrap_skill_prompt(content: &str) -> String {
    format!(
        "\n\n{}\n{}\n{}",
        SKILL_PROMPT_START,
        content.trim(),
        SKILL_PROMPT_END
    )
}

/// Build a `[Skill Instructions]` block with an end marker so it can
/// be reliably extracted later.
pub(crate) fn build_protected_skill_instructions(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(
        "{}{}\n{}",
        SKILL_INSTRUCTIONS_HEADER, trimmed, SKILL_INSTRUCTIONS_END
    )
}

/// Character budget for skill content given a parent budget in tokens.
pub(crate) fn skill_char_budget(parent_budget_tokens: usize) -> usize {
    let budget = ((parent_budget_tokens as f64 * SKILL_BUDGET_RATIO) * 4.0).floor() as usize;
    budget.max(SKILL_MIN_CHARS)
}

/// Apply tiering: if a `<!-- SKILL_DETAIL -->` separator is present,
/// drop the reference portion first; condense the core only if it
/// still exceeds `max_chars`.
pub(crate) fn tier_skill_content(content: &str, max_chars: usize) -> String {
    let char_count = content.chars().count();
    if char_count <= max_chars {
        return content.to_string();
    }

    if let Some(sep_idx) = content.find(SKILL_DETAIL_SEPARATOR) {
        let core = content[..sep_idx].trim();
        let core_chars = core.chars().count();
        if core_chars <= max_chars {
            return core.to_string();
        }
        return condense_text(core, max_chars);
    }

    condense_text(content, max_chars)
}

/// Build a condensed skill reminder for post-compaction state digests.
/// Returns `None` when no skill content is available.
pub(crate) fn build_skill_reminder(
    skill_prompt: Option<&str>,
    runtime_context: Option<&str>,
) -> Option<String> {
    let combined = match (skill_prompt, runtime_context) {
        (Some(sp), Some(rc)) => format!("{}\n{}", sp.trim(), rc.trim()),
        (Some(sp), None) => sp.trim().to_string(),
        (None, Some(rc)) => rc.trim().to_string(),
        (None, None) => return None,
    };
    if combined.is_empty() {
        return None;
    }
    let core = if let Some(sep_idx) = combined.find(SKILL_DETAIL_SEPARATOR) {
        combined[..sep_idx].trim().to_string()
    } else {
        combined
    };
    let reminder = condense_text(&core, 600);
    Some(format!(
        "[ActiveSkillReminder]\nThe following skill directives are still active. Follow them:\n{}",
        reminder
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_skill_prompt_roundtrip() {
        let original = format!(
            "Before skill\n\n{}\nSkill rule 1\nSkill rule 2\n{}\nAfter skill",
            SKILL_PROMPT_START, SKILL_PROMPT_END
        );
        let (without, skill) = extract_skill_prompt(&original);
        assert_eq!(skill.as_deref(), Some("Skill rule 1\nSkill rule 2"));
        assert!(without.contains("Before skill"));
        assert!(without.contains("After skill"));
        assert!(!without.contains("SKILL_PROMPT_START"));
    }

    #[test]
    fn extract_skill_prompt_missing() {
        let text = "No skill markers here";
        let (without, skill) = extract_skill_prompt(text);
        assert!(skill.is_none());
        assert_eq!(without, text);
    }

    #[test]
    fn extract_skill_instructions_roundtrip() {
        let original = format!(
            "[SystemContext]\nexecution_id: abc\n\n[Skill Instructions]\nDo this\nDo that\n{}\n\n[Tasks]\ntask1",
            SKILL_INSTRUCTIONS_END
        );
        let (without, skill) = extract_skill_instructions(&original);
        assert_eq!(skill.as_deref(), Some("Do this\nDo that"));
        assert!(without.contains("[SystemContext]"));
        assert!(without.contains("[Tasks]"));
        assert!(!without.contains("Skill Instructions"));
    }

    #[test]
    fn extract_skill_instructions_missing() {
        let text = "[SystemContext]\ndata";
        let (without, skill) = extract_skill_instructions(text);
        assert!(skill.is_none());
        assert_eq!(without, text);
    }

    #[test]
    fn tier_content_under_budget_unchanged() {
        let content = "Rule 1\nRule 2";
        assert_eq!(tier_skill_content(content, 100), content);
    }

    #[test]
    fn tier_content_drops_detail_section() {
        let content = format!(
            "Core rule 1\nCore rule 2\n{}\nLong reference material {}",
            SKILL_DETAIL_SEPARATOR,
            "x".repeat(500)
        );
        let result = tier_skill_content(&content, 30);
        assert!(result.contains("Core rule"));
        assert!(!result.contains("reference material"));
    }

    #[test]
    fn tier_content_condenses_when_no_separator() {
        let long = "A".repeat(500);
        let result = tier_skill_content(&long, 100);
        assert!(result.chars().count() <= 110);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn build_reminder_none_when_empty() {
        assert!(build_skill_reminder(None, None).is_none());
        assert!(build_skill_reminder(Some(""), None).is_none());
    }

    #[test]
    fn build_reminder_includes_content() {
        let reminder = build_skill_reminder(Some("Always use TDD"), None);
        assert!(reminder.is_some());
        let text = reminder.unwrap();
        assert!(text.contains("ActiveSkillReminder"));
        assert!(text.contains("Always use TDD"));
    }

    #[test]
    fn wrap_and_extract_roundtrip() {
        let raw = "My skill rules here";
        let wrapped = format!("prefix{}", wrap_skill_prompt(raw));
        let (_, extracted) = extract_skill_prompt(&wrapped);
        assert_eq!(extracted.as_deref(), Some(raw));
    }

    #[test]
    fn skill_char_budget_respects_minimum() {
        assert!(skill_char_budget(100) >= SKILL_MIN_CHARS);
    }

    #[test]
    fn skill_char_budget_scales_with_parent() {
        let small = skill_char_budget(1000);
        let large = skill_char_budget(10000);
        assert!(large > small);
    }

    #[test]
    fn protected_instructions_empty_input() {
        assert_eq!(build_protected_skill_instructions(""), "");
        assert_eq!(build_protected_skill_instructions("  "), "");
    }

    #[test]
    fn protected_instructions_includes_end_marker() {
        let result = build_protected_skill_instructions("rule one");
        assert!(result.contains("[Skill Instructions]"));
        assert!(result.contains(SKILL_INSTRUCTIONS_END));
        assert!(result.contains("rule one"));
    }
}
