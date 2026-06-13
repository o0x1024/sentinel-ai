//! Claude Code-style Skill tool — invoke + on-demand read_file for helper files.

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};

const SKILL_FILENAME: &str = "SKILL.md";
const SKILL_OPEN_TAG: &str = "<skill>";
const SKILL_CLOSE_TAG: &str = "</skill>";
const MAX_FILE_BYTES: usize = 200 * 1024;
const MAX_PATH_DEPTH: usize = 3;
const SKILL_DIR_VAR: &str = "${SENTINEL_SKILL_DIR}";

type SkillsForkExecutorFuture =
    Pin<Box<dyn Future<Output = Result<SkillsForkResult, String>> + Send>>;
pub type SkillsForkExecutorFn =
    Arc<dyn Fn(SkillsForkRequest) -> SkillsForkExecutorFuture + Send + Sync>;

static SKILLS_FORK_EXECUTOR: OnceLock<SkillsForkExecutorFn> = OnceLock::new();
static SKILLS_PARENT_EXECUTION_ID: RwLock<Option<String>> = RwLock::new(None);

/// Request to run a skill in an isolated sub-agent (Claude Code `context: fork`).
#[derive(Debug, Clone)]
pub struct SkillsForkRequest {
    pub parent_execution_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub task: String,
    pub allowed_tools: Option<Vec<String>>,
    pub model_override: Option<String>,
    pub effort: Option<String>,
    pub agent_role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillsForkResult {
    pub success: bool,
    pub result: String,
}

/// Register the host executor that runs forked skills as sub-agents.
pub fn set_skills_fork_executor(executor: SkillsForkExecutorFn) {
    let _ = SKILLS_FORK_EXECUTOR.set(executor);
}

/// Bind the current agent execution id so forked skills can inherit parent context.
pub fn set_skills_parent_execution_id(execution_id: Option<String>) {
    if let Ok(mut slot) = SKILLS_PARENT_EXECUTION_ID.write() {
        *slot = execution_id;
    }
}

fn current_parent_execution_id() -> Option<String> {
    SKILLS_PARENT_EXECUTION_ID
        .read()
        .ok()
        .and_then(|slot| slot.clone())
}

fn get_fork_executor() -> Result<&'static SkillsForkExecutorFn, SkillsToolError> {
    SKILLS_FORK_EXECUTOR
        .get()
        .ok_or(SkillsToolError::ForkNotConfigured)
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SkillsToolArgs {
    /// Skill name or directory id (e.g. "code-audit" or frontmatter name).
    pub skill: String,
    /// Action to perform: "invoke" (default) loads SKILL.md instructions;
    /// "read_file" reads a single helper file from the skill directory.
    #[serde(default)]
    pub action: Option<String>,
    /// Relative path of the helper file to read (only for action="read_file").
    #[serde(default)]
    pub file: Option<String>,
    /// Optional arguments passed to the skill (substituted into the skill body).
    #[serde(default)]
    pub args: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillsToolOutput {
    pub action: String,
    pub skill: SkillSummary,
    /// Full skill instructions returned inline in the tool result.
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Relative paths of @-referenced helper files expanded into the skill body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referenced_files: Option<Vec<String>>,
    /// Non-fatal issues while expanding helper files (e.g. missing references).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillsToolError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("Path escapes skill directory")]
    PathEscape,
    #[error("File too large (max 200KB)")]
    FileTooLarge,
    #[error("Depth exceeds limit (max 3)")]
    DepthExceeded,
    #[error("Invalid UTF-8 content")]
    InvalidUtf8,
    #[error("YAML parse error: {0}")]
    Yaml(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Skill fork requires agent execution context")]
    NoExecutionContext,
    #[error("Skill fork executor not configured")]
    ForkNotConfigured,
    #[error("Fork execution failed: {0}")]
    ForkFailed(String),
}

#[derive(Debug, Clone, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default, alias = "when-to-use")]
    when_to_use: Option<String>,
    #[serde(default, alias = "allowed-tools")]
    allowed_tools: Option<Vec<String>>,
    model: Option<String>,
    effort: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default, alias = "disable-model-invocation")]
    disable_model_invocation: Option<bool>,
}

#[derive(Debug, Clone)]
struct SkillDocument {
    frontmatter: SkillFrontmatter,
    body: String,
}

#[derive(Debug, Clone)]
struct ResolvedSkill {
    id: String,
    dir: PathBuf,
    doc: SkillDocument,
}

#[derive(Debug, Clone)]
pub struct SkillsTool;

impl SkillsTool {
    pub const NAME: &'static str = "skills";
    pub const DESCRIPTION: &'static str =
        "Load and interact with skills. Two actions are supported:\n\
- invoke (default): loads the skill's SKILL.md instructions. Helper files referenced \
in the instructions are NOT auto-loaded — use read_file to load them on demand.\n\
- read_file: reads a single helper file from the skill directory (e.g. references/attack_vectors.md).\n\
Available skills are listed in <system-reminder> messages in the conversation. \
If you see a <skill> tag in the current turn, the skill is already loaded — follow it directly. \
Do not use file_read, glob, or shell to access skill files.";

    fn skills_root() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("skills")
    }

    fn parse_skill_markdown(content: &str) -> Result<SkillDocument, SkillsToolError> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(SkillsToolError::Yaml(
                "SKILL.md missing YAML frontmatter".to_string(),
            ));
        }

        let mut lines = trimmed.lines();
        let first = lines.next().unwrap_or_default();
        if first.trim() != "---" {
            return Err(SkillsToolError::Yaml(
                "SKILL.md frontmatter start must be '---'".to_string(),
            ));
        }

        let mut yaml_lines = Vec::new();
        for line in lines.by_ref() {
            if line.trim() == "---" {
                break;
            }
            yaml_lines.push(line);
        }

        let yaml_str = yaml_lines.join("\n");
        let frontmatter: SkillFrontmatter =
            serde_yaml::from_str(&yaml_str).map_err(|e| SkillsToolError::Yaml(e.to_string()))?;

        let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        Ok(SkillDocument { frontmatter, body })
    }

    fn resolve_skill(skill_ref: &str) -> Result<ResolvedSkill, SkillsToolError> {
        let root = Self::skills_root();
        let direct = root.join(skill_ref);
        if direct.is_dir() && direct.join(SKILL_FILENAME).exists() {
            return Self::load_skill_from_dir(skill_ref, &direct);
        }

        let Ok(read_dir) = fs::read_dir(&root) else {
            return Err(SkillsToolError::NotFound(skill_ref.to_string()));
        };

        let mut matches = Vec::new();
        for entry in read_dir.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            if id == skill_ref {
                let skill_dir = entry.path();
                return Self::load_skill_from_dir(&id, &skill_dir);
            }
            let skill_md = entry.path().join(SKILL_FILENAME);
            if !skill_md.exists() {
                continue;
            }
            let content = fs::read_to_string(&skill_md)
                .map_err(|e| SkillsToolError::Io(e.to_string()))?;
            let doc = Self::parse_skill_markdown(&content)?;
            if doc.frontmatter.name.eq_ignore_ascii_case(skill_ref) {
                matches.push((id, entry.path(), doc));
            }
        }

        match matches.len() {
            0 => Err(SkillsToolError::NotFound(skill_ref.to_string())),
            1 => {
                let (id, dir, doc) = matches.into_iter().next().unwrap();
                Ok(ResolvedSkill {
                    id,
                    dir: fs::canonicalize(&dir)
                        .map_err(|e| SkillsToolError::NotFound(e.to_string()))?,
                    doc,
                })
            }
            _ => Err(SkillsToolError::InvalidArgs(format!(
                "Ambiguous skill name '{skill_ref}' — use the skill directory id instead"
            ))),
        }
    }

    fn load_skill_from_dir(id: &str, skill_dir: &Path) -> Result<ResolvedSkill, SkillsToolError> {
        let skill_md = skill_dir.join(SKILL_FILENAME);
        let content =
            fs::read_to_string(&skill_md).map_err(|e| SkillsToolError::NotFound(e.to_string()))?;
        let doc = Self::parse_skill_markdown(&content)?;
        Ok(ResolvedSkill {
            id: id.to_string(),
            dir: fs::canonicalize(skill_dir)
                .map_err(|e| SkillsToolError::NotFound(e.to_string()))?,
            doc,
        })
    }

    fn substitute_skill_dir(content: &str, skill_dir: &Path) -> String {
        let dir = skill_dir.to_string_lossy();
        content
            .replace(SKILL_DIR_VAR, &dir)
            .replace("${CLAUDE_SKILL_DIR}", &dir)
    }

    fn substitute_args(content: &str, args: Option<&str>) -> String {
        let Some(args) = args.filter(|value| !value.trim().is_empty()) else {
            return content.to_string();
        };
        content.replace("${ARGS}", args).replace("$ARGUMENTS", args)
    }

    fn build_skill_content(resolved: &ResolvedSkill, args: Option<&str>) -> String {
        let skill_dir = resolved.dir.to_string_lossy();
        let mut content = format!(
            "Base directory for this skill: {skill_dir}\n\n{}",
            resolved.doc.body
        );
        content = Self::substitute_skill_dir(&content, &resolved.dir);
        Self::substitute_args(&content, args)
    }

    fn expand_with_referenced_files(
        content: &str,
        skill_dir: &Path,
    ) -> Result<(String, Vec<String>, Vec<String>), SkillsToolError> {
        let mentions = collect_skill_file_references(content);
        if mentions.is_empty() {
            return Ok((content.to_string(), Vec::new(), Vec::new()));
        }

        let mut expanded = content.to_string();
        let mut seen = HashSet::new();
        let mut referenced_files = Vec::new();
        let mut warnings = Vec::new();

        for mention in mentions {
            if !seen.insert(mention.clone()) {
                continue;
            }
            let Some((rel_display, file_content)) =
                read_skill_scoped_at_mention(skill_dir, &mention)?
            else {
                let warning = format!("Referenced file not found: {mention}");
                tracing::warn!("Skill helper file skipped: {warning}");
                warnings.push(warning);
                continue;
            };
            referenced_files.push(rel_display.clone());
            expanded.push_str(&format!(
                "\n\n<file path=\"{rel_display}\">\n{file_content}\n</file>"
            ));
        }

        Ok((expanded, referenced_files, warnings))
    }

    fn format_skill_invoke_content(skill_id: &str, skill_name: &str, body: &str) -> String {
        format!(
            "Skill loaded: {skill_name}\n\n{open}\n<name>{skill_name}</name>\n<path>skill://{skill_id}/{skill_filename}</path>\n{body}\n{close}",
            open = SKILL_OPEN_TAG,
            close = SKILL_CLOSE_TAG,
            skill_filename = SKILL_FILENAME,
        )
    }

    async fn handle_read_file(
        &self,
        skill_ref: &str,
        file_path: Option<&str>,
    ) -> Result<SkillsToolOutput, SkillsToolError> {
        let file_path = file_path
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .ok_or_else(|| {
                SkillsToolError::InvalidArgs(
                    "file parameter is required for action=\"read_file\"".to_string(),
                )
            })?;

        let resolved = Self::resolve_skill(skill_ref)?;
        let Some((rel_display, file_content)) =
            read_skill_scoped_at_mention(&resolved.dir, file_path)?
        else {
            return Err(SkillsToolError::NotFound(format!(
                "Helper file not found: {file_path}"
            )));
        };

        let skill_summary = SkillSummary {
            id: resolved.id.clone(),
            name: resolved.doc.frontmatter.name.clone(),
            description: resolved.doc.frontmatter.description.clone(),
            when_to_use: resolved.doc.frontmatter.when_to_use.clone(),
        };

        Ok(SkillsToolOutput {
            action: "read_file".to_string(),
            skill: skill_summary,
            content: format!(
                "<file path=\"{rel_display}\">\n{file_content}\n</file>"
            ),
            allowed_tools: None,
            model_override: None,
            effort: None,
            referenced_files: Some(vec![rel_display]),
            warnings: None,
        })
    }

    /// Resolve a skill identifier to its directory id (for enablement checks).
    pub fn resolve_skill_id(skill_ref: &str) -> Result<String, SkillsToolError> {
        Ok(Self::resolve_skill(skill_ref)?.id)
    }
}

impl Tool for SkillsTool {
    const NAME: &'static str = Self::NAME;
    type Args = SkillsToolArgs;
    type Output = SkillsToolOutput;
    type Error = SkillsToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SkillsToolArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let skill_ref = args.skill.trim();
        if skill_ref.is_empty() {
            return Err(SkillsToolError::InvalidArgs(
                "skill name is required".to_string(),
            ));
        }

        let action = args.action.as_deref().unwrap_or("invoke");

        if action == "read_file" {
            return self.handle_read_file(skill_ref, args.file.as_deref()).await;
        }

        let resolved = Self::resolve_skill(skill_ref)?;
        if resolved.doc.frontmatter.disable_model_invocation.unwrap_or(false) {
            return Err(SkillsToolError::InvalidArgs(format!(
                "Skill '{skill_ref}' is not available for model invocation"
            )));
        }
        let content = Self::build_skill_content(&resolved, args.args.as_deref());

        let frontmatter = &resolved.doc.frontmatter;
        let name = frontmatter.name.clone();
        let skill_summary = SkillSummary {
            id: resolved.id.clone(),
            name: name.clone(),
            description: frontmatter.description.clone(),
            when_to_use: frontmatter.when_to_use.clone(),
        };

        if frontmatter
            .context
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("fork"))
        {
            let (fork_content, referenced_files, warnings) =
                Self::expand_with_referenced_files(&content, &resolved.dir)?;
            let referenced_files = (!referenced_files.is_empty()).then_some(referenced_files);
            let warnings = (!warnings.is_empty()).then_some(warnings);
            let parent_execution_id = current_parent_execution_id().ok_or(SkillsToolError::NoExecutionContext)?;
            let executor = get_fork_executor()?;
            let fork_result = executor(SkillsForkRequest {
                parent_execution_id,
                skill_id: resolved.id.clone(),
                skill_name: name.clone(),
                task: fork_content,
                allowed_tools: frontmatter.allowed_tools.clone(),
                model_override: frontmatter.model.clone(),
                effort: frontmatter.effort.clone(),
                agent_role: frontmatter.agent.clone(),
            })
            .await
            .map_err(SkillsToolError::ForkFailed)?;

            return Ok(SkillsToolOutput {
                action: "fork".to_string(),
                skill: skill_summary,
                content: format!(
                    "Skill \"{name}\" completed (forked execution).\n\nResult:\n{}",
                    fork_result.result
                ),
                allowed_tools: None,
                model_override: None,
                effort: None,
                referenced_files,
                warnings,
            });
        }

        Ok(SkillsToolOutput {
            action: "invoke".to_string(),
            skill: skill_summary,
            content: Self::format_skill_invoke_content(&resolved.id, &name, &content),
            allowed_tools: frontmatter.allowed_tools.clone(),
            model_override: frontmatter.model.clone(),
            effort: frontmatter.effort.clone(),
            referenced_files: None,
            warnings: None,
        })
    }
}

/// Extract `@`-mentioned file paths from text (Claude Code-compatible).
pub fn extract_at_mentioned_files(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    let quoted_re = regex::Regex::new(r#"(?:^|\s)@"([^"]+)""#).expect("valid regex");
    for cap in quoted_re.captures_iter(content) {
        if let Some(path) = cap.get(1).map(|m| m.as_str().to_string()) {
            if !path.ends_with(" (agent)") && seen.insert(path.clone()) {
                results.push(path);
            }
        }
    }

    let regular_re = regex::Regex::new(r#"(?:^|\s)@([^\s]+)"#).expect("valid regex");
    for cap in regular_re.captures_iter(content) {
        if let Some(path) = cap.get(1).map(|m| m.as_str().to_string()) {
            if path.starts_with('"') || path.ends_with(" (agent)") {
                continue;
            }
            if seen.insert(path.clone()) {
                results.push(path);
            }
        }
    }

    results
}

fn is_skill_scoped_file_ref(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty()
        || path.contains("://")
        || path.contains('\n')
        || path.contains('{')
        || path.contains('}')
        || path.contains('@')
    {
        return false;
    }

    // Real skill helper paths are scoped (e.g. references/foo.md, scripts/bar.py).
    // Prose like `.js`, `TDES.js`, or `config.json` must not match.
    if !path.contains('/') {
        return false;
    }

    const EXTENSIONS: [&str; 12] = [
        "md", "txt", "json", "yaml", "yml", "toml", "csv", "sh", "py", "rs", "ts", "js",
    ];
    let lower = path.to_lowercase();
    EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

/// Extract relative file paths wrapped in backticks, e.g. `references/attack_vectors.md`.
pub fn extract_backtick_referenced_files(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let backtick_re = regex::Regex::new(r"`([^`\n]+)`").expect("valid regex");

    for cap in backtick_re.captures_iter(content) {
        let Some(path) = cap.get(1).map(|m| m.as_str().trim().to_string()) else {
            continue;
        };
        if !is_skill_scoped_file_ref(&path) || !seen.insert(path.clone()) {
            continue;
        }
        results.push(path);
    }

    results
}

fn trim_skill_file_mention(path: &str) -> String {
    path.trim()
        .trim_end_matches(['.', ',', ';', ':', ')', ']', '"', '\''])
        .to_string()
}

pub fn collect_skill_file_references(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for path in extract_at_mentioned_files(content)
        .into_iter()
        .chain(extract_backtick_referenced_files(content))
    {
        let path = trim_skill_file_mention(&path);
        if path.is_empty() || !is_skill_scoped_file_ref(&path) || !seen.insert(path.clone()) {
            continue;
        }
        results.push(path);
    }

    results
}

fn parse_at_mention_path(mention: &str) -> (String, Option<usize>, Option<usize>) {
    let re = regex::Regex::new(r"^([^#]+)(?:#L(\d+)(?:-(\d+))?)?").expect("valid regex");
    if let Some(cap) = re.captures(mention) {
        let filename = cap
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let line_start = cap.get(2).and_then(|m| m.as_str().parse().ok());
        let line_end = cap.get(3).and_then(|m| m.as_str().parse().ok());
        (filename, line_start, line_end)
    } else {
        (mention.to_string(), None, None)
    }
}

fn read_skill_scoped_at_mention(
    skill_dir: &Path,
    mention: &str,
) -> Result<Option<(String, String)>, SkillsToolError> {
    let (raw_path, line_start, line_end) = parse_at_mention_path(mention);
    if raw_path.is_empty() {
        return Ok(None);
    }

    let candidate = if Path::new(&raw_path).is_absolute() {
        PathBuf::from(&raw_path)
    } else {
        skill_dir.join(&raw_path)
    };

    let relative = candidate.strip_prefix(skill_dir).unwrap_or(&candidate);
    if !check_depth(relative) {
        return Err(SkillsToolError::DepthExceeded);
    }

    let Ok(canonical_file) = fs::canonicalize(&candidate) else {
        return Ok(None);
    };
    let canonical_skill =
        fs::canonicalize(skill_dir).map_err(|e| SkillsToolError::Io(e.to_string()))?;
    if !canonical_file.starts_with(&canonical_skill) {
        return Ok(None);
    }

    let metadata = fs::metadata(&canonical_file)
        .map_err(|e| SkillsToolError::Io(e.to_string()))?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(SkillsToolError::FileTooLarge);
    }

    let bytes = fs::read(&canonical_file).map_err(|e| SkillsToolError::Io(e.to_string()))?;
    let mut text = String::from_utf8(bytes).map_err(|_| SkillsToolError::InvalidUtf8)?;

    if line_start.is_some() || line_end.is_some() {
        let lines: Vec<&str> = text.lines().collect();
        let start = line_start.unwrap_or(1).saturating_sub(1);
        let end = line_end.unwrap_or(start + 1).min(lines.len());
        if start < lines.len() {
            text = lines[start..end].join("\n");
        }
    }

    let rel_display = canonical_file
        .strip_prefix(&canonical_skill)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| raw_path.clone());

    Ok(Some((rel_display, text)))
}

fn check_depth(path: &Path) -> bool {
    path.components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count()
        <= MAX_PATH_DEPTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_at_mentions_from_skill_text() {
        let text = "Read @references/patterns.md and @\"docs/guide with spaces.md\" first.";
        let mentions = extract_at_mentioned_files(text);
        assert!(mentions.contains(&"references/patterns.md".to_string()));
        assert!(mentions.contains(&"docs/guide with spaces.md".to_string()));
    }

    #[test]
    fn extract_skips_agent_mentions() {
        let text = r#"Use @"code-reviewer (agent)" for review."#;
        let mentions = extract_at_mentioned_files(text);
        assert!(mentions.is_empty());
    }

    #[test]
    fn extract_backtick_reference_paths_from_skill_body() {
        let text = "Read `references/attack_vectors.md` and `references/cvss_scoring.md` first.";
        let mentions = extract_backtick_referenced_files(text);
        assert!(mentions.contains(&"references/attack_vectors.md".to_string()));
        assert!(mentions.contains(&"references/cvss_scoring.md".to_string()));
    }

    #[test]
    fn collect_skill_file_references_merges_at_and_backtick_paths() {
        let text = "Use @references/patterns.md and `references/tool_setup.md`.";
        let mentions = collect_skill_file_references(text);
        assert_eq!(
            mentions,
            vec![
                "references/patterns.md".to_string(),
                "references/tool_setup.md".to_string()
            ]
        );
    }

    #[test]
    fn extract_backtick_reference_skips_non_file_literals() {
        let text = r#"{"email": "a@b.com"}` and `not-a-file`"#;
        let mentions = extract_backtick_referenced_files(text);
        assert!(mentions.is_empty());
    }

    #[test]
    fn extract_backtick_reference_skips_extension_only_literals() {
        let text = "Download all referenced `.js` files and inspect `TDES.js`.";
        let mentions = extract_backtick_referenced_files(text);
        assert!(mentions.is_empty());
    }

    #[test]
    fn is_skill_scoped_file_ref_requires_path_separator() {
        assert!(!is_skill_scoped_file_ref(".js"));
        assert!(!is_skill_scoped_file_ref("TDES.js"));
        assert!(!is_skill_scoped_file_ref("config.json"));
        assert!(is_skill_scoped_file_ref("references/attack_vectors.md"));
        assert!(is_skill_scoped_file_ref("scripts/sql_injection_test.py"));
    }

    #[test]
    fn collect_skill_file_references_skips_at_mentions_without_path_separator() {
        let text = "Use @agent-reviewer and @references/patterns.md.";
        let mentions = collect_skill_file_references(text);
        assert_eq!(mentions, vec!["references/patterns.md".to_string()]);
    }
}
