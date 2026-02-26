use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use sentinel_db::{Database, DatabaseService};

// 0 = unknown, 1 = available, 2 = unavailable
static SKILLS_REF_STATUS: AtomicU8 = AtomicU8::new(0);
static SKILLS_REF_UNAVAILABLE_LOGGED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub when_to_use: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkillDocument {
    pub frontmatter: SkillFrontmatter,
    pub body: String,
}

pub fn skills_root(db: &DatabaseService) -> PathBuf {
    db.get_skills_root_dir()
}

pub fn read_skill_markdown(path: &Path) -> Result<SkillDocument> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read skill file: {}", path.display()))?;
    parse_skill_markdown(&content)
}

pub fn parse_skill_markdown(content: &str) -> Result<SkillDocument> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        anyhow::bail!("SKILL.md missing YAML frontmatter");
    }

    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or_default();
    if first.trim() != "---" {
        anyhow::bail!("SKILL.md frontmatter start must be '---'");
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
        serde_yaml::from_str(&yaml_str).context("Failed to parse SKILL.md YAML frontmatter")?;

    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();

    Ok(SkillDocument { frontmatter, body })
}

pub fn validate_skill_with_skills_ref(skill_dir: &Path) -> Result<()> {
    if SKILLS_REF_STATUS.load(Ordering::Relaxed) == 2 {
        return Ok(());
    }

    let result = std::process::Command::new("skills-ref")
        .arg("validate")
        .arg(skill_dir)
        .output();

    match result {
        Ok(output) => {
            SKILLS_REF_STATUS.store(1, Ordering::Relaxed);
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                anyhow::bail!(
                    "skills-ref validate failed: {}\n{}",
                    stderr.trim(),
                    stdout.trim()
                );
            }
            Ok(())
        }
        Err(e) => {
            // If skills-ref is not installed, skip validation instead of blocking scans.
            if e.kind() == std::io::ErrorKind::NotFound {
                SKILLS_REF_STATUS.store(2, Ordering::Relaxed);
                if !SKILLS_REF_UNAVAILABLE_LOGGED.swap(true, Ordering::Relaxed) {
                    tracing::debug!("skills-ref not available, skipping validation");
                }
                return Ok(());
            }
            tracing::warn!("skills-ref invocation failed, skipping validation: {}", e);
            Ok(())
        }
    }
}

pub fn resolve_skill_file_path(
    root: &Path,
    skill_id: &str,
    relative_path: &str,
) -> Result<PathBuf> {
    let skill_dir = root.join(skill_id);
    let candidate = skill_dir.join(relative_path);
    let canonical_root = fs::canonicalize(&skill_dir)
        .with_context(|| format!("Skill directory not found: {}", skill_dir.display()))?;
    let canonical_file = fs::canonicalize(&candidate)
        .with_context(|| format!("Skill file not found: {}", candidate.display()))?;

    if !canonical_file.starts_with(&canonical_root) {
        anyhow::bail!("Path escapes skill directory");
    }

    Ok(canonical_file)
}

pub async fn scan_and_upsert_skills(db_service: &DatabaseService) -> Result<usize> {
    let root = skills_root(db_service);
    fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create skills root: {}", root.display()))?;

    // Install built-in skills (Phase 4: CPG audit workflow)
    install_builtin_skills(&root);

    let mut count = 0usize;
    let entries = fs::read_dir(&root)
        .with_context(|| format!("Failed to read skills root: {}", root.display()))?;

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir_path = entry.path();
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let skill_md = dir_path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let content = match fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read SKILL.md {}: {}", skill_md.display(), e);
                continue;
            }
        };

        let doc = match parse_skill_markdown(&content) {
            Ok(doc) => doc,
            Err(e) => {
                tracing::warn!("Invalid SKILL.md {}: {}", skill_md.display(), e);
                continue;
            }
        };

        if let Err(e) = validate_skill_with_skills_ref(&dir_path) {
            tracing::warn!("skills-ref validation warning (scan): {}", e);
        }

        if doc.frontmatter.name != dir_name {
            tracing::warn!(
                "Skill name mismatch: dir='{}' frontmatter='{}' (continuing)",
                dir_name,
                doc.frontmatter.name
            );
        }

        let source_path = skill_md
            .strip_prefix(&root)
            .unwrap_or(&skill_md)
            .to_string_lossy()
            .to_string();

        let existing = db_service.get_skill(&dir_name).await?;
        if let Some(existing) = existing {
            let update = sentinel_db::UpdateSkill {
                name: Some(dir_name.clone()),
                description: Some(doc.frontmatter.description.clone()),
                source_path: Some(source_path),
                argument_hint: Some(existing.argument_hint),
                disable_model_invocation: Some(existing.disable_model_invocation),
                user_invocable: Some(existing.user_invocable),
                allowed_tools: Some(existing.allowed_tools),
                model: Some(existing.model),
                context: Some(existing.context),
                agent: Some(existing.agent),
                hooks: existing.hooks,
            };
            db_service.update_skill(&dir_name, &update).await?;
        } else {
            let create = sentinel_db::CreateSkill {
                id: dir_name.clone(),
                name: dir_name.clone(),
                description: doc.frontmatter.description.clone(),
                source_path,
                argument_hint: String::new(),
                disable_model_invocation: false,
                user_invocable: true,
                allowed_tools: vec![],
                model: String::new(),
                context: String::new(),
                agent: String::new(),
                hooks: Some(serde_json::Value::Object(Default::default())),
            };
            db_service.create_skill(&create).await?;
        }

        count += 1;
    }

    Ok(count)
}

// ── Built-in Skills ─────────────────────────────────────────────────────────

/// Embedded built-in skills that ship with the binary.
const BUILTIN_SKILLS: &[(&str, &str)] = &[(
    "code-audit",
    include_str!("../../skills/code-audit/SKILL.md"),
)];

/// Install built-in skills to the skills directory if not already present.
fn install_builtin_skills(skills_root: &Path) {
    for (skill_id, content) in BUILTIN_SKILLS {
        let skill_dir = skills_root.join(skill_id);
        let skill_md = skill_dir.join("SKILL.md");

        // Only write if the skill doesn't exist yet or content has changed
        let should_write = if skill_md.exists() {
            match fs::read_to_string(&skill_md) {
                Ok(existing) => existing.trim() != content.trim(),
                Err(_) => true,
            }
        } else {
            true
        };

        if should_write {
            if let Err(e) = fs::create_dir_all(&skill_dir) {
                tracing::warn!("Failed to create built-in skill dir '{}': {}", skill_id, e);
                continue;
            }
            if let Err(e) = fs::write(&skill_md, content) {
                tracing::warn!("Failed to write built-in skill '{}': {}", skill_id, e);
                continue;
            }
            tracing::info!("Installed built-in skill: {}", skill_id);
        }
    }
}
