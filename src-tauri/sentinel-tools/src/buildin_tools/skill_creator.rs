//! Skill creator tool for authoring local Sentinel skills.

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_SKILL_ID_CHARS: usize = 64;
const MAX_DESCRIPTION_CHARS: usize = 200;
const MAX_FILE_BYTES: usize = 200 * 1024;
const MAX_FILE_LINES: usize = 2_000;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillCreatorAction {
    /// Create a new skill directory containing SKILL.md and optional files.
    Create,
    /// Update an existing skill's SKILL.md and optional files.
    Update,
    /// Validate an existing skill directory without writing files.
    Validate,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SkillCreatorFile {
    /// Relative path inside the skill directory, for example references/api.md or scripts/build.py.
    pub path: String,
    /// UTF-8 file content.
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SkillCreatorArgs {
    pub action: SkillCreatorAction,
    /// Directory id and frontmatter name. Use lowercase letters, numbers, and hyphens.
    pub skill_id: String,
    /// Clear trigger description. Sentinel caps this at 200 characters to keep skill discovery precise.
    #[serde(default)]
    pub description: Option<String>,
    /// Markdown body for SKILL.md after YAML frontmatter.
    #[serde(default)]
    pub body: Option<String>,
    /// Optional software dependencies to include in SKILL.md frontmatter.
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    /// Optional bundled resource or script files.
    #[serde(default)]
    pub files: Vec<SkillCreatorFile>,
    /// Allow create/update to replace existing files.
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillCreatorValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillCreatorOutput {
    pub success: bool,
    pub action: String,
    pub skill_id: String,
    pub skill_dir: String,
    pub skill_md_path: String,
    pub written_files: Vec<String>,
    pub validation: SkillCreatorValidation,
    pub next_steps: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillCreatorError {
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("Skill already exists: {0}")]
    AlreadyExists(String),
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("Path escapes skill directory")]
    PathEscape,
    #[error("File too large: {0}")]
    FileTooLarge(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("YAML error: {0}")]
    Yaml(String),
}

#[derive(Debug, Clone, Serialize)]
struct SkillFrontmatter<'a> {
    name: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<&'a Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillCreatorTool;

impl SkillCreatorTool {
    pub const NAME: &'static str = "skill_creator";
    pub const DESCRIPTION: &'static str = concat!(
        "Create, update, and validate local Sentinel skills using the Agent Skills progressive-disclosure structure. ",
        "Use this instead of generic file tools when authoring a reusable workflow skill. ",
        "It writes SKILL.md with YAML name/description metadata, optional dependencies, and optional references/scripts files; ",
        "it validates focused descriptions, path safety, required files, and the repository rule that files over 2000 lines must be split."
    );

    fn skills_root() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("skills")
    }

    fn validate_skill_id(skill_id: &str) -> Result<(), SkillCreatorError> {
        let trimmed = skill_id.trim();
        if trimmed.is_empty() {
            return Err(SkillCreatorError::InvalidArgs(
                "skill_id is required".to_string(),
            ));
        }
        if trimmed != skill_id {
            return Err(SkillCreatorError::InvalidArgs(
                "skill_id must not contain leading or trailing whitespace".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_SKILL_ID_CHARS {
            return Err(SkillCreatorError::InvalidArgs(format!(
                "skill_id must be {MAX_SKILL_ID_CHARS} characters or less"
            )));
        }
        let first = trimmed.chars().next().unwrap_or_default();
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(SkillCreatorError::InvalidArgs(
                "skill_id must start with a lowercase letter or digit".to_string(),
            ));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(SkillCreatorError::InvalidArgs(
                "skill_id must use lowercase letters, numbers, and hyphens only".to_string(),
            ));
        }
        if trimmed.ends_with('-') {
            return Err(SkillCreatorError::InvalidArgs(
                "skill_id must not end with a hyphen".to_string(),
            ));
        }
        Ok(())
    }

    fn require_description<'a>(
        action: &SkillCreatorAction,
        description: &'a Option<String>,
    ) -> Result<Option<&'a str>, SkillCreatorError> {
        match action {
            SkillCreatorAction::Create => {
                let value = description.as_deref().ok_or_else(|| {
                    SkillCreatorError::InvalidArgs(
                        "description is required when creating a skill".to_string(),
                    )
                })?;
                Self::validate_description(value)?;
                Ok(Some(value))
            }
            SkillCreatorAction::Update => {
                if let Some(value) = description.as_deref() {
                    Self::validate_description(value)?;
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            SkillCreatorAction::Validate => Ok(None),
        }
    }

    fn validate_description(description: &str) -> Result<(), SkillCreatorError> {
        let trimmed = description.trim();
        if trimmed.is_empty() {
            return Err(SkillCreatorError::InvalidArgs(
                "description is required".to_string(),
            ));
        }
        if trimmed != description {
            return Err(SkillCreatorError::InvalidArgs(
                "description must not contain leading or trailing whitespace".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(SkillCreatorError::InvalidArgs(format!(
                "description must be {MAX_DESCRIPTION_CHARS} characters or less"
            )));
        }
        if trimmed.contains('<') || trimmed.contains('>') {
            return Err(SkillCreatorError::InvalidArgs(
                "description must not contain XML tags".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_body(
        action: &SkillCreatorAction,
        body: &Option<String>,
    ) -> Result<(), SkillCreatorError> {
        if matches!(action, SkillCreatorAction::Create) {
            let value = body.as_deref().ok_or_else(|| {
                SkillCreatorError::InvalidArgs("body is required when creating a skill".to_string())
            })?;
            if value.trim().is_empty() {
                return Err(SkillCreatorError::InvalidArgs(
                    "body must not be empty".to_string(),
                ));
            }
            Self::validate_file_size("SKILL.md body", value)?;
        }
        if let Some(value) = body {
            Self::validate_file_size("SKILL.md body", value)?;
        }
        Ok(())
    }

    fn validate_file_size(path: &str, content: &str) -> Result<(), SkillCreatorError> {
        if content.len() > MAX_FILE_BYTES {
            return Err(SkillCreatorError::FileTooLarge(path.to_string()));
        }
        if content.lines().count() > MAX_FILE_LINES {
            return Err(SkillCreatorError::InvalidArgs(format!(
                "{path} exceeds {MAX_FILE_LINES} lines; split it by function"
            )));
        }
        Ok(())
    }

    fn resolve_write_path(
        skill_dir: &Path,
        relative_path: &str,
    ) -> Result<PathBuf, SkillCreatorError> {
        let rel = Self::sanitize_relative_path(relative_path)?;
        let target = skill_dir.join(&rel);
        let parent = target.parent().ok_or_else(|| {
            SkillCreatorError::InvalidArgs(format!("invalid file path: {relative_path}"))
        })?;
        let canonical_root = if skill_dir.exists() {
            fs::canonicalize(skill_dir).map_err(|e| SkillCreatorError::Io(e.to_string()))?
        } else {
            let parent = skill_dir.parent().ok_or_else(|| {
                SkillCreatorError::InvalidArgs("skill directory has no parent".to_string())
            })?;
            fs::create_dir_all(parent).map_err(|e| SkillCreatorError::Io(e.to_string()))?;
            fs::canonicalize(parent)
                .map_err(|e| SkillCreatorError::Io(e.to_string()))?
                .join(skill_dir.file_name().unwrap_or_default())
        };
        let canonical_parent = if parent.exists() {
            fs::canonicalize(parent).map_err(|e| SkillCreatorError::Io(e.to_string()))?
        } else {
            let existing_parent = parent
                .ancestors()
                .find(|candidate| candidate.exists())
                .ok_or_else(|| SkillCreatorError::Io("no existing parent directory".to_string()))?;
            fs::canonicalize(existing_parent).map_err(|e| SkillCreatorError::Io(e.to_string()))?
        };
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(SkillCreatorError::PathEscape);
        }
        Ok(target)
    }

    fn sanitize_relative_path(relative_path: &str) -> Result<PathBuf, SkillCreatorError> {
        let path = relative_path.trim();
        if path.is_empty() {
            return Err(SkillCreatorError::InvalidArgs(
                "file path is required".to_string(),
            ));
        }
        if path == "SKILL.md" {
            return Err(SkillCreatorError::InvalidArgs(
                "SKILL.md is generated from description/body; do not pass it in files".to_string(),
            ));
        }
        let rel = PathBuf::from(path);
        if rel.is_absolute() {
            return Err(SkillCreatorError::PathEscape);
        }
        if rel.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(SkillCreatorError::PathEscape);
        }
        Ok(rel)
    }

    fn build_skill_markdown(
        skill_id: &str,
        description: &str,
        body: &str,
        dependencies: Option<&Vec<String>>,
    ) -> Result<String, SkillCreatorError> {
        let frontmatter = SkillFrontmatter {
            name: skill_id,
            description,
            dependencies,
        };
        let yaml = serde_yaml::to_string(&frontmatter)
            .map_err(|e| SkillCreatorError::Yaml(e.to_string()))?;
        Ok(format!("---\n{}---\n\n{}\n", yaml, body.trim()))
    }

    fn read_existing_frontmatter(
        skill_md_path: &Path,
    ) -> Result<(String, String), SkillCreatorError> {
        let content =
            fs::read_to_string(skill_md_path).map_err(|e| SkillCreatorError::Io(e.to_string()))?;
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(SkillCreatorError::InvalidArgs(
                "SKILL.md missing YAML frontmatter".to_string(),
            ));
        }
        let mut lines = trimmed.lines();
        if lines.next().unwrap_or_default().trim() != "---" {
            return Err(SkillCreatorError::InvalidArgs(
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
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml_lines.join("\n"))
            .map_err(|e| SkillCreatorError::Yaml(e.to_string()))?;
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Ok((name, description))
    }

    fn validate_skill_dir(skill_dir: &Path, skill_id: &str) -> SkillCreatorValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let skill_md_path = skill_dir.join("SKILL.md");

        if !skill_md_path.exists() {
            errors.push("SKILL.md is required".to_string());
        } else {
            match Self::read_existing_frontmatter(&skill_md_path) {
                Ok((name, description)) => {
                    if name != skill_id {
                        errors.push("frontmatter name must match skill_id".to_string());
                    }
                    if let Err(error) = Self::validate_description(&description) {
                        errors.push(error.to_string());
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }

        if skill_dir.exists() {
            for entry in walkdir::WalkDir::new(skill_dir)
                .max_depth(4)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                match fs::metadata(path) {
                    Ok(metadata) if metadata.len() > MAX_FILE_BYTES as u64 => errors.push(format!(
                        "{} exceeds {} bytes",
                        path.strip_prefix(skill_dir)
                            .unwrap_or(path)
                            .to_string_lossy(),
                        MAX_FILE_BYTES
                    )),
                    Ok(_) => {}
                    Err(error) => errors.push(error.to_string()),
                }
                match fs::read_to_string(path) {
                    Ok(content) if content.lines().count() > MAX_FILE_LINES => {
                        errors.push(format!(
                            "{} exceeds {} lines; split it by function",
                            path.strip_prefix(skill_dir)
                                .unwrap_or(path)
                                .to_string_lossy(),
                            MAX_FILE_LINES
                        ))
                    }
                    Ok(content) => {
                        if path == skill_md_path && content.lines().count() > 500 {
                            warnings.push(
                                "SKILL.md is over 500 lines; move details into references/"
                                    .to_string(),
                            );
                        }
                        if content.contains("BEGIN OPENSSH PRIVATE KEY")
                            || content.contains("BEGIN RSA PRIVATE KEY")
                            || content.contains("api_key =")
                            || content.contains("API_KEY=")
                        {
                            warnings.push(format!(
                                "{} may contain hardcoded secrets",
                                path.strip_prefix(skill_dir)
                                    .unwrap_or(path)
                                    .to_string_lossy()
                            ));
                        }
                    }
                    Err(_) => warnings.push(format!(
                        "{} is not UTF-8 text; skill resources should be reviewable",
                        path.strip_prefix(skill_dir)
                            .unwrap_or(path)
                            .to_string_lossy()
                    )),
                }
            }
        }

        SkillCreatorValidation {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    async fn call_with_root(
        &self,
        args: SkillCreatorArgs,
        root: PathBuf,
    ) -> Result<SkillCreatorOutput, SkillCreatorError> {
        Self::validate_skill_id(&args.skill_id)?;
        let description = Self::require_description(&args.action, &args.description)?;
        Self::validate_body(&args.action, &args.body)?;
        for file in &args.files {
            Self::sanitize_relative_path(&file.path)?;
            Self::validate_file_size(&file.path, &file.content)?;
        }

        let skill_dir = root.join(&args.skill_id);
        let skill_md_path = skill_dir.join("SKILL.md");
        let mut written_files = Vec::new();

        match args.action {
            SkillCreatorAction::Create => {
                if skill_dir.exists() && !args.overwrite {
                    return Err(SkillCreatorError::AlreadyExists(args.skill_id));
                }
                fs::create_dir_all(&skill_dir).map_err(|e| SkillCreatorError::Io(e.to_string()))?;
                let skill_md = Self::build_skill_markdown(
                    &args.skill_id,
                    description.unwrap_or_default(),
                    args.body.as_deref().unwrap_or_default(),
                    args.dependencies.as_ref(),
                )?;
                fs::write(&skill_md_path, skill_md)
                    .map_err(|e| SkillCreatorError::Io(e.to_string()))?;
                written_files.push(skill_md_path.to_string_lossy().to_string());
                Self::write_extra_files(
                    &skill_dir,
                    &args.files,
                    args.overwrite,
                    &mut written_files,
                )?;
            }
            SkillCreatorAction::Update => {
                if !skill_md_path.exists() {
                    return Err(SkillCreatorError::NotFound(args.skill_id));
                }
                if description.is_some() || args.body.is_some() || args.dependencies.is_some() {
                    let (_, existing_description) =
                        Self::read_existing_frontmatter(&skill_md_path)?;
                    let existing_content = fs::read_to_string(&skill_md_path)
                        .map_err(|e| SkillCreatorError::Io(e.to_string()))?;
                    let existing_body = existing_content
                        .split_once("\n---")
                        .map(|(_, body)| body.trim_start_matches(['\n', '\r']))
                        .unwrap_or_default();
                    let next_description = description.unwrap_or(existing_description.as_str());
                    let next_body = args.body.as_deref().unwrap_or(existing_body);
                    let skill_md = Self::build_skill_markdown(
                        &args.skill_id,
                        next_description,
                        next_body,
                        args.dependencies.as_ref(),
                    )?;
                    fs::write(&skill_md_path, skill_md)
                        .map_err(|e| SkillCreatorError::Io(e.to_string()))?;
                    written_files.push(skill_md_path.to_string_lossy().to_string());
                }
                Self::write_extra_files(
                    &skill_dir,
                    &args.files,
                    args.overwrite,
                    &mut written_files,
                )?;
            }
            SkillCreatorAction::Validate => {
                if !skill_dir.exists() {
                    return Err(SkillCreatorError::NotFound(args.skill_id));
                }
            }
        }

        let validation = Self::validate_skill_dir(&skill_dir, &args.skill_id);
        let next_steps = if validation.valid {
            vec![
                format!(
                    "Invoke the skill with the skills tool: skills(skill=\"{}\") to verify it loads correctly.",
                    args.skill_id
                ),
                "Test with prompts that should and should not trigger this skill; refine description if activation is imprecise.".to_string(),
            ]
        } else {
            vec!["Fix validation errors, then run skill_creator action=validate again.".to_string()]
        };

        Ok(SkillCreatorOutput {
            success: validation.valid,
            action: format!("{:?}", args.action).to_lowercase(),
            skill_id: args.skill_id,
            skill_dir: skill_dir.to_string_lossy().to_string(),
            skill_md_path: skill_md_path.to_string_lossy().to_string(),
            written_files,
            validation,
            next_steps,
        })
    }

    fn write_extra_files(
        skill_dir: &Path,
        files: &[SkillCreatorFile],
        overwrite: bool,
        written_files: &mut Vec<String>,
    ) -> Result<(), SkillCreatorError> {
        for file in files {
            let target = Self::resolve_write_path(skill_dir, &file.path)?;
            if target.exists() && !overwrite {
                return Err(SkillCreatorError::AlreadyExists(file.path.clone()));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| SkillCreatorError::Io(e.to_string()))?;
            }
            fs::write(&target, &file.content).map_err(|e| SkillCreatorError::Io(e.to_string()))?;
            written_files.push(target.to_string_lossy().to_string());
        }
        Ok(())
    }
}

impl Tool for SkillCreatorTool {
    const NAME: &'static str = Self::NAME;
    type Args = SkillCreatorArgs;
    type Output = SkillCreatorOutput;
    type Error = SkillCreatorError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SkillCreatorArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.call_with_root(args, Self::skills_root()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("sentinel-skill-creator-{}", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn creates_skill_with_reference_file() {
        let root = temp_root();
        let tool = SkillCreatorTool;
        let output = tool
            .call_with_root(
                SkillCreatorArgs {
                    action: SkillCreatorAction::Create,
                    skill_id: "incident-notes".to_string(),
                    description: Some(
                        "Draft repeatable incident notes with evidence and follow-up sections."
                            .to_string(),
                    ),
                    body: Some(
                        "# Incident Notes\n\nUse references/template.md when drafting notes."
                            .to_string(),
                    ),
                    dependencies: None,
                    files: vec![SkillCreatorFile {
                        path: "references/template.md".to_string(),
                        content: "# Template\n\n- Evidence\n- Follow-up\n".to_string(),
                    }],
                    overwrite: false,
                },
                root.clone(),
            )
            .await
            .expect("skill should be created");

        assert!(output.success, "{output:?}");
        assert!(root.join("incident-notes").join("SKILL.md").exists());
        assert!(root
            .join("incident-notes")
            .join("references/template.md")
            .exists());
        let skill_md = fs::read_to_string(root.join("incident-notes").join("SKILL.md")).unwrap();
        assert!(skill_md.contains("name: incident-notes"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn rejects_path_escape() {
        let root = temp_root();
        let tool = SkillCreatorTool;
        let result = tool
            .call_with_root(
                SkillCreatorArgs {
                    action: SkillCreatorAction::Create,
                    skill_id: "unsafe-path".to_string(),
                    description: Some(
                        "Reject file paths that escape the skill directory.".to_string(),
                    ),
                    body: Some("# Unsafe Path\n".to_string()),
                    dependencies: None,
                    files: vec![SkillCreatorFile {
                        path: "../outside.md".to_string(),
                        content: "no".to_string(),
                    }],
                    overwrite: false,
                },
                root.clone(),
            )
            .await;

        assert!(matches!(result, Err(SkillCreatorError::PathEscape)));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn validates_line_limit() {
        let root = temp_root();
        let tool = SkillCreatorTool;
        let long_body = std::iter::repeat("line")
            .take(MAX_FILE_LINES + 1)
            .collect::<Vec<_>>()
            .join("\n");
        let result = tool
            .call_with_root(
                SkillCreatorArgs {
                    action: SkillCreatorAction::Create,
                    skill_id: "large-skill".to_string(),
                    description: Some(
                        "Reject oversized skill files before writing them.".to_string(),
                    ),
                    body: Some(long_body),
                    dependencies: None,
                    files: Vec::new(),
                    overwrite: false,
                },
                root.clone(),
            )
            .await;

        assert!(matches!(result, Err(SkillCreatorError::InvalidArgs(_))));
        let _ = fs::remove_dir_all(root);
    }
}
