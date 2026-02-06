//! Skills tool for Claude-style progressive disclosure

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::docker_sandbox::{DockerSandbox, DockerSandboxConfig};
use crate::output_storage::{get_host_context_dir, CONTAINER_CONTEXT_DIR};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillsAction {
    /// List all available skills (id, name, description)
    List,
    /// Load SKILL.md content for a skill
    Load,
    /// Read a referenced file inside the skill directory
    ReadFile,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SkillsToolArgs {
    /// Action to perform: list, load, or read_file
    pub action: SkillsAction,
    /// Skill identifier (directory name under skills root)
    pub skill_id: Option<String>,
    /// Relative path inside the skill directory (for read_file)
    pub path: Option<String>,
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
    pub skills: Option<Vec<SkillSummary>>,
    pub skill: Option<SkillSummary>,
    pub content: Option<String>,
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_hint: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillsToolError {
    #[error("Skill file not found: {0}")]
    NotFound(String),
    #[error("Invalid action arguments: {0}")]
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
}

#[derive(Debug, Clone, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    when_to_use: Option<String>,
}

#[derive(Debug, Clone)]
struct SkillDocument {
    frontmatter: SkillFrontmatter,
    body: String,
}

#[derive(Debug, Clone)]
pub struct SkillsTool;

impl SkillsTool {
    pub const NAME: &'static str = "skills";
    pub const DESCRIPTION: &'static str =
        "Claude-style skills tool. Use action=list to see all skills, action=load to read SKILL.md, and action=read_file to read referenced files inside a skill.";

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

    fn resolve_skill_dir(skill_id: &str) -> Result<PathBuf, SkillsToolError> {
        let root = Self::skills_root();
        let skill_dir = root.join(skill_id);
        let canonical = fs::canonicalize(&skill_dir)
            .map_err(|e| SkillsToolError::NotFound(e.to_string()))?;
        Ok(canonical)
    }

    fn check_depth(path: &Path) -> bool {
        path.components()
            .filter(|c| matches!(c, std::path::Component::Normal(_)))
            .count()
            <= 3
    }

    fn normalize_rel_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    fn runtime_host_path(skill_id: &str, rel_path: &str) -> PathBuf {
        get_host_context_dir()
            .join("skills")
            .join(skill_id)
            .join(rel_path)
    }

    fn runtime_container_path(skill_id: &str, rel_path: &str) -> String {
        format!("{}/skills/{}/{}", CONTAINER_CONTEXT_DIR, skill_id, rel_path)
    }

    async fn stage_runtime_file(
        skill_id: &str,
        rel_path: &str,
        bytes: &[u8],
    ) -> Result<(String, Option<String>), SkillsToolError> {
        let rel_path = Self::normalize_rel_path(rel_path);
        let host_target = Self::runtime_host_path(skill_id, &rel_path);
        if let Some(parent) = host_target.parent() {
            fs::create_dir_all(parent).map_err(|e| SkillsToolError::Io(e.to_string()))?;
        }
        fs::write(&host_target, bytes).map_err(|e| SkillsToolError::Io(e.to_string()))?;

        let container_target = Self::runtime_container_path(skill_id, &rel_path);
        let host_target_str = host_target.to_string_lossy().to_string();

        let container_path = if DockerSandbox::is_docker_available().await {
            let sandbox = DockerSandbox::new(DockerSandboxConfig::default());
            match sandbox
                .copy_file_to_container(&host_target_str, &container_target)
                .await
            {
                Ok(_) => Some(container_target),
                Err(e) => {
                    tracing::warn!(
                        "Failed to stage skill file into container ({}): {}",
                        host_target_str,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok((host_target_str, container_path))
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
        match args.action {
            SkillsAction::List => {
                let root = Self::skills_root();
                let mut skills = Vec::new();
                for entry in WalkDir::new(&root)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    if entry.file_name() != "SKILL.md" {
                        continue;
                    }
                    let skill_dir = entry.path().parent().unwrap_or(&root);
                    let id = skill_dir
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let content =
                        fs::read_to_string(entry.path()).map_err(|e| SkillsToolError::Io(e.to_string()))?;
                    let doc = Self::parse_skill_markdown(&content)?;
                    skills.push(SkillSummary {
                        id,
                        name: doc.frontmatter.name,
                        description: doc.frontmatter.description,
                        when_to_use: doc.frontmatter.when_to_use,
                    });
                }
                skills.sort_by(|a, b| a.id.cmp(&b.id));
                Ok(SkillsToolOutput {
                    action: "list".to_string(),
                    skills: Some(skills),
                    skill: None,
                    content: None,
                    path: None,
                    host_path: None,
                    container_path: None,
                    runtime_hint: None,
                })
            }
            SkillsAction::Load => {
                let skill_id = args
                    .skill_id
                    .ok_or_else(|| SkillsToolError::InvalidArgs("skill_id is required".to_string()))?;
                let skill_dir = Self::resolve_skill_dir(&skill_id)?;
                let skill_md = skill_dir.join("SKILL.md");
                let bytes = fs::read(&skill_md).map_err(|e| SkillsToolError::NotFound(e.to_string()))?;
                let content = String::from_utf8(bytes.clone()).map_err(|_| SkillsToolError::InvalidUtf8)?;
                let doc = Self::parse_skill_markdown(&content)?;
                let (host_path, container_path) =
                    Self::stage_runtime_file(&skill_id, "SKILL.md", &bytes).await?;
                let runtime_hint = container_path
                    .as_ref()
                    .map(|p| format!("Docker mode: use this path directly -> {}", p))
                    .or_else(|| Some(format!("Host mode fallback path -> {}", host_path)));
                Ok(SkillsToolOutput {
                    action: "load".to_string(),
                    skills: None,
                    skill: Some(SkillSummary {
                        id: skill_id,
                        name: doc.frontmatter.name,
                        description: doc.frontmatter.description,
                        when_to_use: doc.frontmatter.when_to_use,
                    }),
                    content: Some(doc.body),
                    path: Some("SKILL.md".to_string()),
                    host_path: Some(host_path),
                    container_path,
                    runtime_hint,
                })
            }
            SkillsAction::ReadFile => {
                let skill_id = args
                    .skill_id
                    .ok_or_else(|| SkillsToolError::InvalidArgs("skill_id is required".to_string()))?;
                let path = args
                    .path
                    .ok_or_else(|| SkillsToolError::InvalidArgs("path is required".to_string()))?;

                if !Self::check_depth(Path::new(&path)) {
                    return Err(SkillsToolError::DepthExceeded);
                }

                let skill_dir = Self::resolve_skill_dir(&skill_id)?;
                let candidate = skill_dir.join(&path);
                let canonical_file =
                    fs::canonicalize(&candidate).map_err(|e| SkillsToolError::NotFound(e.to_string()))?;
                if !canonical_file.starts_with(&skill_dir) {
                    return Err(SkillsToolError::PathEscape);
                }
                let metadata =
                    fs::metadata(&canonical_file).map_err(|e| SkillsToolError::Io(e.to_string()))?;
                if metadata.len() > 200 * 1024 {
                    return Err(SkillsToolError::FileTooLarge);
                }
                let bytes = fs::read(&canonical_file).map_err(|e| SkillsToolError::Io(e.to_string()))?;
                let content = String::from_utf8(bytes.clone()).map_err(|_| SkillsToolError::InvalidUtf8)?;
                let rel_path = Self::normalize_rel_path(&path);
                let (host_path, container_path) =
                    Self::stage_runtime_file(&skill_id, &rel_path, &bytes).await?;
                let runtime_hint = container_path
                    .as_ref()
                    .map(|p| format!("Docker mode: execute/read this file via {}", p))
                    .or_else(|| Some(format!("Host mode fallback path -> {}", host_path)));

                Ok(SkillsToolOutput {
                    action: "read_file".to_string(),
                    skills: None,
                    skill: Some(SkillSummary {
                        id: skill_id,
                        name: String::new(),
                        description: String::new(),
                        when_to_use: None,
                    }),
                    content: Some(content),
                    path: Some(rel_path),
                    host_path: Some(host_path),
                    container_path,
                    runtime_hint,
                })
            }
        }
    }
}
