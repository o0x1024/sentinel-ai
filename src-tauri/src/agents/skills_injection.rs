//! Claude Code-aligned skill catalog and model-driven body injection.
//!
//! - Catalog (`<skills_instructions>`) is injected once per conversation as a system-reminder.
//! - Skill bodies (`<skill>`) are returned inline in the `skills` tool result.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sentinel_llm::ChatMessage;

pub const SKILLS_INSTRUCTIONS_OPEN_TAG: &str = "<skills_instructions>";
pub const SKILLS_INSTRUCTIONS_CLOSE_TAG: &str = "</skills_instructions>";
pub const SKILL_OPEN_TAG: &str = "<skill>";
pub const SKILL_CLOSE_TAG: &str = "</skill>";
pub const TOOL_MENTION_SIGIL: char = '$';

const SKILL_BUDGET_CONTEXT_PERCENT: f64 = 0.01;
const CHARS_PER_TOKEN: usize = 4;
pub const DEFAULT_SKILL_CHAR_BUDGET: usize = 8_000;
const MAX_LISTING_DESC_CHARS: usize = 250;
const MIN_DESC_LENGTH: usize = 20;

const SKILLS_INTRO: &str =
    "The following skills are available for use with the skills tool. Each provides specialized capabilities and domain knowledge.";

const SKILLS_HOW_TO_USE: &str = r###"- Discovery: Skills available this session are listed above (name + description only).
- Invocation: When the task clearly matches a skill's description, call the `skills` tool with the skill name immediately. SKILL.md instructions are returned in the tool result — follow them exactly.
- Helper files: SKILL.md may reference helper files (e.g. `references/attack_vectors.md`). These are NOT auto-loaded. Load them on demand with `skills(skill="<name>", action="read_file", file="<relative_path>")` when the instructions say to use them.
- Blocking requirement: When a skill matches the user's request, invoke the relevant skill BEFORE generating any other response about the task.
- Multiple skills: If multiple skills apply, invoke them in sequence (one tool call each).
- Already loaded: If you see a `<skill>` tag in the current conversation, the skill is already loaded — follow it directly instead of invoking again.
- Do not use file_read, glob, or shell to read SKILL.md or skill helper files — use the skills tool instead.
- Safety: If a skill can't be applied cleanly, describe the blocker and proceed with the best alternative."###;

const SKILL_PATH_PREFIX: &str = "skill://";
const SKILL_FILENAME: &str = "SKILL.md";

/// Enabled skill metadata used for catalog rendering and mention resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_md_path: PathBuf,
}

/// Loaded skill body ready for conversation injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInjection {
    pub name: String,
    pub path: String,
    pub contents: String,
}

/// Same-turn dedup for host skill prompts (Codex `InjectedHostSkillPrompts`).
#[derive(Debug, Default)]
pub struct InjectedSkillPaths {
    paths: HashSet<String>,
}

impl InjectedSkillPaths {
    pub fn insert_path(&mut self, path: impl Into<String>) {
        let path = path.into();
        self.paths.insert(normalize_skill_path(&path).to_string());
        self.paths.insert(path);
    }

    pub fn contains_path(&self, path: &str) -> bool {
        self.paths.contains(path) || self.paths.contains(normalize_skill_path(path))
    }
}

pub fn history_contains_skills_catalog(messages: &[ChatMessage]) -> bool {
    messages.iter().any(|msg| {
        msg.content.contains(SKILLS_INSTRUCTIONS_OPEN_TAG)
            || msg.content.contains("<available_skills>")
    })
}

pub fn wrap_skills_instructions(body: &str) -> String {
    format!(
        "{open}\n{body}\n{close}",
        open = SKILLS_INSTRUCTIONS_OPEN_TAG,
        body = body.trim(),
        close = SKILLS_INSTRUCTIONS_CLOSE_TAG
    )
}

/// Tracks which skill names have already been sent in the catalog (Claude Code `sentSkillNames`).
#[derive(Debug, Default)]
pub struct SkillCatalogState {
    pub sent_skill_names: HashSet<String>,
    pub is_initial: bool,
}

impl SkillCatalogState {
    pub fn recover_from_history(&mut self, messages: &[ChatMessage]) {
        if !self.sent_skill_names.is_empty() {
            return;
        }
        for msg in messages {
            if !msg.content.contains(SKILLS_INSTRUCTIONS_OPEN_TAG) {
                continue;
            }
            for line in msg.content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("- ") {
                    if let Some((name, _)) = rest.split_once(':') {
                        self.sent_skill_names.insert(name.trim().to_string());
                    }
                }
            }
        }
    }

    pub fn filter_unsent<'a>(&self, entries: &'a [SkillEntry]) -> Vec<&'a SkillEntry> {
        entries
            .iter()
            .filter(|entry| !self.sent_skill_names.contains(&entry.name))
            .collect()
    }

    pub fn mark_sent(&mut self, entries: &[SkillEntry]) {
        for entry in entries {
            self.sent_skill_names.insert(entry.name.clone());
        }
    }
}

pub fn get_skill_char_budget(context_window_tokens: Option<usize>) -> usize {
    match context_window_tokens {
        Some(tokens) if tokens > 0 => {
            ((tokens as f64) * (CHARS_PER_TOKEN as f64) * SKILL_BUDGET_CONTEXT_PERCENT) as usize
        }
        _ => DEFAULT_SKILL_CHAR_BUDGET,
    }
}

pub fn format_entries_within_budget(
    entries: &[SkillEntry],
    context_window_tokens: Option<usize>,
) -> Vec<String> {
    if entries.is_empty() {
        return Vec::new();
    }

    let budget = get_skill_char_budget(context_window_tokens);
    let full_lines: Vec<String> = entries
        .iter()
        .map(|entry| format!("- {}: {}", entry.name, entry.description))
        .collect();
    let full_total: usize = full_lines.iter().map(|line| line.len()).sum::<usize>()
        + full_lines.len().saturating_sub(1);

    if full_total <= budget {
        return full_lines;
    }

    let name_overhead: usize = entries
        .iter()
        .map(|entry| entry.name.len() + 4)
        .sum::<usize>()
        + entries.len().saturating_sub(1);
    let available_for_descs = budget.saturating_sub(name_overhead);
    let max_desc_len = if entries.is_empty() {
        0
    } else {
        available_for_descs / entries.len()
    };

    if max_desc_len < MIN_DESC_LENGTH {
        return entries
            .iter()
            .map(|entry| format!("- {}", entry.name))
            .collect();
    }

    entries
        .iter()
        .map(|entry| {
            format!(
                "- {}: {}",
                entry.name,
                truncate_to_char_budget(&entry.description, max_desc_len)
            )
        })
        .collect()
}

fn truncate_to_char_budget(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut trimmed: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    trimmed.push('\u{2026}');
    trimmed
}

pub fn render_skills_instructions_body(entries: &[SkillEntry]) -> String {
    render_skills_instructions_with_lines(&format_entries_within_budget(entries, None))
}

pub fn render_skills_instructions_with_lines(skill_lines: &[String]) -> String {
    let mut lines = vec!["## Skills".to_string(), SKILLS_INTRO.to_string()];
    lines.push("### Available skills".to_string());
    lines.extend(skill_lines.iter().cloned());
    lines.push("### How to use skills".to_string());
    lines.push(SKILLS_HOW_TO_USE.to_string());
    format!("\n{}\n", lines.join("\n"))
}

pub fn format_skill_body_message(injection: &SkillInjection) -> String {
    format!(
        "{open}\n<name>{name}</name>\n<path>{path}</path>\n{contents}\n{close}",
        open = SKILL_OPEN_TAG,
        name = injection.name,
        path = injection.path,
        contents = injection.contents,
        close = SKILL_CLOSE_TAG
    )
}

/// Format a skill invoke tool result with the full body inline (Codex-style direct return).
pub fn format_skill_inline_tool_result(skill_id: &str, skill_name: &str, body: &str) -> String {
    format!(
        "Skill loaded: {skill_name}\n\n{skill_block}\n\nReferenced files have been inlined above. Do not search for them in the workspace.",
        skill_block = format_skill_body_message(&SkillInjection {
            name: skill_name.to_string(),
            path: format!("{SKILL_PATH_PREFIX}{skill_id}/{SKILL_FILENAME}"),
            contents: body.to_string(),
        }),
    )
}

/// Extract the raw skill body from an inline tool result `content` field.
pub fn extract_skill_body_from_inline_tool_result(content: &str) -> String {
    const PATH_CLOSE: &str = "</path>\n";
    const SKILL_CLOSE: &str = "\n</skill>";
    if let Some(start) = content.find(PATH_CLOSE) {
        let body_start = start + PATH_CLOSE.len();
        if let Some(end) = content[body_start..].find(SKILL_CLOSE) {
            return content[body_start..body_start + end].to_string();
        }
    }
    content.to_string()
}

pub fn load_skill_entries_from_root(skills_root: &Path) -> Vec<SkillEntry> {
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(skills_root) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let skill_id = entry.file_name().to_string_lossy().to_string();
        let skill_md = entry.path().join(SKILL_FILENAME);
        if !skill_md.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&skill_md) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let doc = match crate::skills::parse_skill_markdown(&content) {
            Ok(doc) => doc,
            Err(_) => continue,
        };
        if doc.frontmatter.disable_model_invocation.unwrap_or(false) {
            continue;
        }
        if doc.frontmatter.description.trim().is_empty()
            && doc
                .frontmatter
                .when_to_use
                .as_ref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        {
            continue;
        }
        let name = doc.frontmatter.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let description = build_description(&doc.frontmatter.description, doc.frontmatter.when_to_use.as_deref());
        entries.push(SkillEntry {
            id: skill_id,
            name,
            description,
            skill_md_path: skill_md,
        });
    }

    entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    entries
}

pub fn rank_skill_entries_by_task(entries: &mut [SkillEntry], task: Option<&str>) {
    let task_text = task.unwrap_or_default().to_lowercase();
    entries.sort_by(|left, right| {
        let score_a = crate::agents::tool_router::score_skill_match(
            &task_text,
            &left.name,
            &left.description,
        );
        let score_b = crate::agents::tool_router::score_skill_match(
            &task_text,
            &right.name,
            &right.description,
        );
        score_b
            .cmp(&score_a)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

pub fn build_skills_catalog_message(
    mut entries: Vec<SkillEntry>,
    task: Option<&str>,
    total_count: usize,
    context_window_tokens: Option<usize>,
) -> Option<String> {
    if entries.is_empty() && total_count == 0 {
        return None;
    }
    rank_skill_entries_by_task(&mut entries, task);
    let skill_lines = format_entries_within_budget(&entries, context_window_tokens);
    let mut body = render_skills_instructions_with_lines(&skill_lines);
    if total_count > entries.len() {
        body.push_str(&format!(
            "\n... {} more skills omitted for brevity.\n",
            total_count - entries.len()
        ));
    }
    Some(wrap_skills_instructions(&body))
}

/// Inject skill catalog into history with dedup (Claude Code `skill_listing` pattern).
pub fn inject_skills_catalog(
    history: &mut Vec<ChatMessage>,
    all_entries: &[SkillEntry],
    state: &mut SkillCatalogState,
    task: Option<&str>,
    context_window_tokens: Option<usize>,
) {
    if all_entries.is_empty() {
        return;
    }

    state.recover_from_history(history);
    let unsent: Vec<SkillEntry> = state
        .filter_unsent(all_entries)
        .into_iter()
        .cloned()
        .collect();
    if unsent.is_empty() {
        return;
    }

    state.is_initial = state.sent_skill_names.is_empty();
    let total_count = all_entries.len();
    let Some(catalog) = build_skills_catalog_message(
        unsent.clone(),
        task,
        total_count,
        context_window_tokens,
    ) else {
        return;
    };

    let prefix = if state.is_initial {
        String::new()
    } else {
        "The following new skills are available:\n\n".to_string()
    };
    history.push(ChatMessage::user(format!(
        "<system-reminder>\n{prefix}{catalog}\n</system-reminder>"
    )));
    state.mark_sent(&unsent);
}

/// Format invoked skills for post-compaction re-injection (unified `<skill>` tags).
pub fn format_invoked_skills_for_compaction(
    skills: &[(String, String, String)],
) -> String {
    let skills_content = skills
        .iter()
        .map(|(skill_id, skill_name, body)| {
            format_skill_body_message(&SkillInjection {
                name: skill_name.clone(),
                path: format!("skill://{skill_id}/SKILL.md"),
                contents: body.clone(),
            })
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "<system-reminder>\nThe following skills were invoked in this session. Continue to follow these guidelines:\n\n{skills_content}\n</system-reminder>"
    )
}

fn build_description(description: &str, when_to_use: Option<&str>) -> String {
    let description = description.trim();
    let description = if let Some(when) = when_to_use.filter(|value| !value.trim().is_empty()) {
        format!("{description} - {}", when.trim())
    } else {
        description.to_string()
    };
    if description.chars().count() > MAX_LISTING_DESC_CHARS {
        let mut trimmed = description
            .chars()
            .take(MAX_LISTING_DESC_CHARS.saturating_sub(1))
            .collect::<String>();
        trimmed.push('\u{2026}');
        trimmed
    } else {
        description
    }
}

/// Collect explicitly mentioned skills from user text inputs (Codex-compatible).
pub fn collect_explicit_skill_mentions(texts: &[&str], skills: &[SkillEntry]) -> Vec<SkillEntry> {
    let name_counts = build_skill_name_counts(skills);
    let mut selected = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    for text in texts {
        let mentions = extract_tool_mentions(text);
        select_skills_from_mentions(skills, &name_counts, &mentions, &mut seen_names, &mut seen_paths, &mut selected);
    }

    selected
}

fn build_skill_name_counts(skills: &[SkillEntry]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for skill in skills {
        *counts.entry(skill.name.clone()).or_insert(0) += 1;
        if skill.id != skill.name {
            *counts.entry(skill.id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

struct ToolMentions<'a> {
    names: HashSet<&'a str>,
    paths: HashSet<&'a str>,
    plain_names: HashSet<&'a str>,
}

fn extract_tool_mentions(text: &str) -> ToolMentions<'_> {
    extract_tool_mentions_with_sigil(text, TOOL_MENTION_SIGIL)
}

fn extract_tool_mentions_with_sigil(text: &str, sigil: char) -> ToolMentions<'_> {
    let text_bytes = text.as_bytes();
    let mut mentioned_names: HashSet<&str> = HashSet::new();
    let mut mentioned_paths: HashSet<&str> = HashSet::new();
    let mut plain_names: HashSet<&str> = HashSet::new();

    let mut index = 0;
    while index < text_bytes.len() {
        let byte = text_bytes[index];
        if byte == b'[' {
            if let Some((name, path, end_index)) =
                parse_linked_tool_mention(text, text_bytes, index, sigil)
            {
                mentioned_names.insert(name);
                mentioned_paths.insert(path);
                index = end_index;
                continue;
            }
        }

        if byte != sigil as u8 {
            index += 1;
            continue;
        }

        let name_start = index + 1;
        let Some(first_name_byte) = text_bytes.get(name_start) else {
            index += 1;
            continue;
        };
        if !is_mention_name_char(*first_name_byte) {
            index += 1;
            continue;
        }

        let mut name_end = name_start + 1;
        while name_end < text_bytes.len() {
            let Some(next_byte) = text_bytes.get(name_end) else {
                break;
            };
            if !is_mention_name_char(*next_byte) {
                break;
            }
            name_end += 1;
        }

        let name = &text[name_start..name_end];
        mentioned_names.insert(name);
        plain_names.insert(name);
        index = name_end;
    }

    ToolMentions {
        names: mentioned_names,
        paths: mentioned_paths,
        plain_names,
    }
}

fn select_skills_from_mentions(
    skills: &[SkillEntry],
    name_counts: &HashMap<String, usize>,
    mentions: &ToolMentions<'_>,
    seen_names: &mut HashSet<String>,
    seen_paths: &mut HashSet<String>,
    selected: &mut Vec<SkillEntry>,
) {
    if mentions.names.is_empty() && mentions.paths.is_empty() {
        return;
    }

    let mention_skill_paths: HashSet<&str> = mentions
        .paths
        .iter()
        .copied()
        .filter(|path| is_skill_path(path))
        .map(normalize_skill_path)
        .collect();

    for skill in skills {
        let path_str = skill.skill_md_path.to_string_lossy();
        if seen_paths.contains(path_str.as_ref()) {
            continue;
        }
        if mention_skill_paths.contains(path_str.as_ref())
            || mention_skill_paths.contains(skill.id.as_str())
        {
            seen_paths.insert(path_str.into_owned());
            seen_names.insert(skill.name.clone());
            selected.push(skill.clone());
        }
    }

    for skill in skills {
        if seen_paths.contains(skill.skill_md_path.to_string_lossy().as_ref()) {
            continue;
        }
        for candidate in [&skill.name, &skill.id] {
            if !mentions.plain_names.contains(candidate.as_str()) {
                continue;
            }
            let count = name_counts.get(candidate.as_str()).copied().unwrap_or(0);
            if count != 1 {
                continue;
            }
            if seen_names.insert(skill.name.clone()) {
                seen_paths.insert(skill.skill_md_path.to_string_lossy().into_owned());
                selected.push(skill.clone());
            }
            break;
        }
    }
}

pub fn resolve_skill_injections(entries: &[SkillEntry]) -> Vec<SkillInjection> {
    let mut injections = Vec::with_capacity(entries.len());
    for entry in entries {
        let Ok(content) = std::fs::read_to_string(&entry.skill_md_path) else {
            continue;
        };
        let Ok(doc) = crate::skills::parse_skill_markdown(&content) else {
            continue;
        };
        injections.push(SkillInjection {
            name: entry.name.clone(),
            path: entry.skill_md_path.to_string_lossy().into_owned(),
            contents: doc.body,
        });
    }
    injections
}

fn parse_linked_tool_mention<'a>(
    text: &'a str,
    text_bytes: &[u8],
    start: usize,
    sigil: char,
) -> Option<(&'a str, &'a str, usize)> {
    let sigil_index = start + 1;
    if text_bytes.get(sigil_index) != Some(&(sigil as u8)) {
        return None;
    }

    let name_start = sigil_index + 1;
    let first_name_byte = text_bytes.get(name_start)?;
    if !is_mention_name_char(*first_name_byte) {
        return None;
    }

    let mut name_end = name_start + 1;
    while name_end < text_bytes.len() {
        let Some(next_byte) = text_bytes.get(name_end) else {
            break;
        };
        if !is_mention_name_char(*next_byte) {
            break;
        }
        name_end += 1;
    }

    if text_bytes.get(name_end) != Some(&b']') {
        return None;
    }

    let mut path_start = name_end + 1;
    while path_start < text_bytes.len() {
        let Some(next_byte) = text_bytes.get(path_start) else {
            break;
        };
        if !next_byte.is_ascii_whitespace() {
            break;
        }
        path_start += 1;
    }
    if text_bytes.get(path_start) != Some(&b'(') {
        return None;
    }

    let mut path_end = path_start + 1;
    while path_end < text_bytes.len() {
        let Some(next_byte) = text_bytes.get(path_end) else {
            break;
        };
        if *next_byte == b')' {
            break;
        }
        path_end += 1;
    }
    if text_bytes.get(path_end) != Some(&b')') {
        return None;
    }

    let path = text[path_start + 1..path_end].trim();
    Some((&text[name_start..name_end], path, path_end + 1))
}

fn is_mention_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

fn is_skill_path(path: &str) -> bool {
    path.starts_with(SKILL_PATH_PREFIX)
        || path.rsplit(['/', '\\']).next().unwrap_or(path).eq_ignore_ascii_case(SKILL_FILENAME)
}

fn normalize_skill_path(path: &str) -> &str {
    path.strip_prefix(SKILL_PATH_PREFIX).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_dollar_mention() {
        let mentions = extract_tool_mentions("Use $code-audit for this task");
        assert!(mentions.plain_names.contains("code-audit"));
    }

    #[test]
    fn extract_linked_skill_path() {
        let mentions = extract_tool_mentions("see [$audit](skill://audit/SKILL.md)");
        assert!(mentions.names.contains("audit"));
        assert!(mentions.paths.contains("skill://audit/SKILL.md"));
    }

    #[test]
    fn unambiguous_plain_name_selects_skill() {
        let skills = vec![SkillEntry {
            id: "penetration-tester".to_string(),
            name: "penetration-tester".to_string(),
            description: "pentest".to_string(),
            skill_md_path: PathBuf::from("/tmp/skills/penetration-tester/SKILL.md"),
        }];
        let selected = collect_explicit_skill_mentions(&["$penetration-tester go"], &skills);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "penetration-tester");
    }

    #[test]
    fn ambiguous_plain_name_is_ignored() {
        let skills = vec![
            SkillEntry {
                id: "audit-a".to_string(),
                name: "audit".to_string(),
                description: "a".to_string(),
                skill_md_path: PathBuf::from("/tmp/skills/audit-a/SKILL.md"),
            },
            SkillEntry {
                id: "audit-b".to_string(),
                name: "audit".to_string(),
                description: "b".to_string(),
                skill_md_path: PathBuf::from("/tmp/skills/audit-b/SKILL.md"),
            },
        ];
        let selected = collect_explicit_skill_mentions(&["use audit please"], &skills);
        assert!(selected.is_empty());
    }

    #[test]
    fn history_detects_skills_catalog() {
        let messages = vec![ChatMessage::new(
            "developer",
            "<skills_instructions>\n## Skills\n</skills_instructions>",
        )];
        assert!(history_contains_skills_catalog(&messages));
    }

    #[test]
    fn budget_truncation_fits_more_than_fixed_cap() {
        let entries: Vec<SkillEntry> = (0..12)
            .map(|idx| SkillEntry {
                id: format!("skill-{idx}"),
                name: format!("skill-{idx}"),
                description: format!("desc-{idx}"),
                skill_md_path: PathBuf::from(format!("/tmp/skills/skill-{idx}/SKILL.md")),
            })
            .collect();
        let lines = format_entries_within_budget(&entries, Some(200_000));
        assert!(lines.len() > 8, "budget should allow more than legacy 8-skill cap");
    }

    #[test]
    fn catalog_state_skips_already_sent_skills() {
        let entries = vec![
            SkillEntry {
                id: "a".to_string(),
                name: "alpha".to_string(),
                description: "first".to_string(),
                skill_md_path: PathBuf::from("/tmp/a/SKILL.md"),
            },
            SkillEntry {
                id: "b".to_string(),
                name: "beta".to_string(),
                description: "second".to_string(),
                skill_md_path: PathBuf::from("/tmp/b/SKILL.md"),
            },
        ];
        let mut state = SkillCatalogState::default();
        state.mark_sent(&entries[..1].to_vec());
        let unsent = state.filter_unsent(&entries);
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].name, "beta");
    }

    #[test]
    fn format_skill_body_uses_codex_tags() {
        let message = format_skill_body_message(&SkillInjection {
            name: "audit".to_string(),
            path: "/tmp/skills/audit/SKILL.md".to_string(),
            contents: "rule one".to_string(),
        });
        assert!(message.contains("<skill>"));
        assert!(message.contains("<path>/tmp/skills/audit/SKILL.md</path>"));
        assert!(!message.contains("<id>"));
    }

    #[test]
    fn format_skill_inline_tool_result_wraps_body() {
        let content = format_skill_inline_tool_result("audit", "Code Audit", "rule one");
        assert!(content.starts_with("Skill loaded: Code Audit"));
        assert!(content.contains("<skill>"));
        assert!(content.contains("rule one"));
        assert!(content.contains("Do not search for them in the workspace"));
    }

    #[test]
    fn extracts_skill_body_from_inline_tool_result() {
        let content = format_skill_inline_tool_result("audit", "Code Audit", "rule one");
        assert_eq!(
            super::extract_skill_body_from_inline_tool_result(&content),
            "rule one"
        );
    }
}
