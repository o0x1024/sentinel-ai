//! Skill CRUD commands

use std::fs;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_db::{Database, Skill, SkillDetail, SkillSummary};

use crate::skills::{parse_skill_markdown, scan_and_upsert_skills, skills_root, validate_skill_with_skills_ref};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub content: String,
    pub argument_hint: String,
    pub disable_model_invocation: bool,
    pub user_invocable: bool,
    pub allowed_tools: Vec<String>,
    pub model: String,
    pub context: String,
    pub agent: String,
    pub hooks: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub argument_hint: Option<String>,
    pub disable_model_invocation: Option<bool>,
    pub user_invocable: Option<bool>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    pub hooks: Option<serde_json::Value>,
}

fn sanitize_skill_dir_name(name: &str) -> String {
    name.trim()
        .replace(['/', '\\'], "_")
        .replace('\u{0000}', "")
}

fn contains_xml_tags(value: &str) -> bool {
    value.contains('<') || value.contains('>')
}

fn validate_skill_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Skill name is required".to_string());
    }
    if trimmed.len() > 64 {
        return Err("Skill name must be 64 characters or less".to_string());
    }
    if contains_xml_tags(trimmed) {
        return Err("Skill name must not contain XML tags".to_string());
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("anthropic") || lower.contains("claude") {
        return Err("Skill name must not contain reserved words".to_string());
    }
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err("Skill name must start with a lowercase letter or digit".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Skill name must use lowercase letters, numbers, and hyphens only".to_string());
    }
    if trimmed.ends_with('-') {
        return Err("Skill name must not end with a hyphen".to_string());
    }
    Ok(())
}

fn validate_skill_description(description: &str) -> Result<(), String> {
    let trimmed = description.trim();
    if trimmed.is_empty() {
        return Err("Skill description is required".to_string());
    }
    if trimmed.len() > 1024 {
        return Err("Skill description must be 1024 characters or less".to_string());
    }
    if contains_xml_tags(trimmed) {
        return Err("Skill description must not contain XML tags".to_string());
    }
    Ok(())
}

fn build_skill_markdown(name: &str, description: &str, content: &str) -> String {
    let mut md = String::new();
    md.push_str("---\n");
    md.push_str(&format!("name: {}\n", name));
    md.push_str(&format!("description: {}\n", description));
    md.push_str("---\n\n");
    md.push_str(content.trim());
    md.push('\n');
    md
}

fn write_skill_file(skill_dir: &PathBuf, name: &str, description: &str, content: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(skill_dir).map_err(|e| e.to_string())?;
    let skill_path = skill_dir.join("SKILL.md");
    let markdown = build_skill_markdown(name, description, content);
    fs::write(&skill_path, markdown).map_err(|e| e.to_string())?;
    Ok(skill_path)
}

fn normalize_source_path(root: &PathBuf, skill_path: &PathBuf) -> String {
    skill_path
        .strip_prefix(root)
        .unwrap_or(skill_path)
        .to_string_lossy()
        .to_string()
}

/// List all skills (summary only)
pub async fn list_skills(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<SkillSummary>, String> {
    db_service
        .list_skills_summary()
        .await
        .map_err(|e| e.to_string())
}

/// List all skills (full details)
pub async fn list_skills_full(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<Skill>, String> {
    db_service
        .list_all_skills()
        .await
        .map_err(|e| e.to_string())
}

/// Get skill detail (Level 2) by ID
pub async fn get_skill_detail(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<SkillDetail>, String> {
    db_service
        .get_skill_detail(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Get a single skill (Level 3) by ID
pub async fn get_skill(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<Skill>, String> {
    db_service
        .get_skill(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Get SKILL.md body for a skill
pub async fn get_skill_markdown(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    let skill = db_service
        .get_skill(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    let root = skills_root(&db_service);
    let skill_path = if !skill.source_path.is_empty() {
        root.join(&skill.source_path)
    } else {
        root.join(&skill.id).join("SKILL.md")
    };

    let content = fs::read_to_string(&skill_path).map_err(|e| e.to_string())?;
    let doc = parse_skill_markdown(&content).map_err(|e| e.to_string())?;
    Ok(doc.body)
}

/// Create a new skill
pub async fn create_skill(
    payload: CreateSkillRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Skill, String> {
    validate_skill_name(&payload.name)?;
    validate_skill_description(&payload.description)?;

    let root = skills_root(&db_service);
    let base_name = sanitize_skill_dir_name(&payload.name);
    if base_name != payload.name {
        return Err("Skill name contains invalid characters".to_string());
    }
    let dir_name = base_name.clone();
    let dir_exists = root.join(&dir_name).exists();
    let id_exists = db_service
        .get_skill(&dir_name)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    let name_exists = db_service
        .get_skill_by_name(&dir_name)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if dir_exists || id_exists || name_exists {
        return Err("Skill name already exists".to_string());
    }
    let skill_dir = root.join(&dir_name);
    let skill_name = dir_name.clone();
    let skill_path = write_skill_file(&skill_dir, &skill_name, &payload.description, &payload.content)?;

    if let Err(e) = validate_skill_with_skills_ref(&skill_dir) {
        tracing::warn!("skills-ref validation warning (create_skill): {}", e);
    }

    let source_path = normalize_source_path(&root, &skill_path);

    let db_payload = sentinel_db::CreateSkill {
        id: dir_name.clone(),
        name: skill_name,
        description: payload.description,
        source_path,
        argument_hint: payload.argument_hint,
        disable_model_invocation: payload.disable_model_invocation,
        user_invocable: payload.user_invocable,
        allowed_tools: payload.allowed_tools,
        model: payload.model,
        context: payload.context,
        agent: payload.agent,
        hooks: payload.hooks,
    };

    db_service
        .create_skill(&db_payload)
        .await
        .map_err(|e| e.to_string())
}

/// Update an existing skill
pub async fn update_skill(
    id: String,
    payload: UpdateSkillRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let existing = db_service
        .get_skill(&id)
        .await
        .map_err(|e| e.to_string())?;

    let existing = existing.ok_or_else(|| "Skill not found".to_string())?;
    let root = skills_root(&db_service);
    let skill_dir = root.join(&id);

    if let Some(ref name) = payload.name {
        if name != &id {
            return Err("Skill name must match its id".to_string());
        }
        validate_skill_name(name)?;
    }
    if let Some(ref description) = payload.description {
        validate_skill_description(description)?;
    }

    if payload.content.is_some() || payload.name.is_some() || payload.description.is_some() {
        let name = existing.name.clone();
        let description = payload
            .description
            .clone()
            .unwrap_or_else(|| existing.description.clone());
        let content = if let Some(content) = payload.content.clone() {
            content
        } else {
            let skill_path = skill_dir.join("SKILL.md");
            if skill_path.exists() {
                let doc = parse_skill_markdown(
                    &fs::read_to_string(&skill_path).map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?;
                doc.body
            } else {
                String::new()
            }
        };

        let _ = write_skill_file(&skill_dir, &name, &description, &content)?;

        if let Err(e) = validate_skill_with_skills_ref(&skill_dir) {
            tracing::warn!("skills-ref validation warning (update_skill): {}", e);
        }
    }

    let skill_path = skill_dir.join("SKILL.md");
    let source_path = if skill_path.exists() {
        normalize_source_path(&root, &skill_path)
    } else {
        existing.source_path.clone()
    };

    let name_update = if let Some(name) = payload.name.clone() {
        if name == id {
            Some(name)
        } else {
            None
        }
    } else {
        None
    };

    let db_payload = sentinel_db::UpdateSkill {
        name: name_update,
        description: payload.description,
        source_path: Some(source_path),
        argument_hint: payload.argument_hint,
        disable_model_invocation: payload.disable_model_invocation,
        user_invocable: payload.user_invocable,
        allowed_tools: payload.allowed_tools,
        model: payload.model,
        context: payload.context,
        agent: payload.agent,
        hooks: payload.hooks,
    };

    db_service
        .update_skill(&id, &db_payload)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a skill
pub async fn delete_skill(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let root = skills_root(&db_service);
    let canonical_root = std::fs::canonicalize(&root).map_err(|e| e.to_string())?;

    let skill = db_service
        .get_skill(&id)
        .await
        .map_err(|e| e.to_string())?;

    // Try to remove on-disk skill directory first so index refresh won't re-import it.
    if let Some(skill) = skill {
        let skill_dir_from_id = root.join(&id);
        if skill_dir_from_id.exists() {
            let canonical_skill_dir =
                std::fs::canonicalize(&skill_dir_from_id).map_err(|e| e.to_string())?;
            if !canonical_skill_dir.starts_with(&canonical_root) || canonical_skill_dir == canonical_root {
                return Err("Refusing to delete path outside skills root".to_string());
            }
            std::fs::remove_dir_all(&canonical_skill_dir).map_err(|e| e.to_string())?;
        } else if !skill.source_path.trim().is_empty() {
            let source = root.join(&skill.source_path);
            if source.exists() {
                let candidate_dir = if source.is_file() {
                    source.parent().unwrap_or(&source).to_path_buf()
                } else {
                    source
                };
                let canonical_candidate = std::fs::canonicalize(&candidate_dir).map_err(|e| e.to_string())?;
                if canonical_candidate.starts_with(&canonical_root) && canonical_candidate != canonical_root {
                    std::fs::remove_dir_all(&canonical_candidate).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    db_service.delete_skill(&id).await.map_err(|e| e.to_string())
}

/// Scan skills directory and refresh DB index
pub async fn refresh_skills_index(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<usize, String> {
    scan_and_upsert_skills(&db_service)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct SkillFileEntry {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct SkillCandidate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillInstallHistory {
    pub id: String,
    pub timestamp: String,
    pub source_type: String,
    pub source: String,
    pub skills: Vec<String>,
    pub status: String,
    pub message: Option<String>,
}

fn resolve_skill_dir(root: &Path, id: &str) -> Result<PathBuf, String> {
    let skill_dir = root.join(id);
    if !skill_dir.exists() {
        return Err("Skill directory not found".to_string());
    }
    let canonical = std::fs::canonicalize(&skill_dir).map_err(|e| e.to_string())?;
    Ok(canonical)
}

fn skills_import_root(root: &Path) -> PathBuf {
    root.join(".imports")
}

fn skills_install_history_path(root: &Path) -> PathBuf {
    root.join("install_history.json")
}

fn load_install_history(root: &Path) -> Vec<SkillInstallHistory> {
    let path = skills_install_history_path(root);
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(records) = serde_json::from_str::<Vec<SkillInstallHistory>>(&content) {
            return records;
        }
    }
    vec![]
}

fn save_install_history(root: &Path, records: &[SkillInstallHistory]) -> Result<(), String> {
    let path = skills_install_history_path(root);
    let content = serde_json::to_string_pretty(records).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn append_install_history(root: &Path, record: SkillInstallHistory) -> Result<(), String> {
    let mut records = load_install_history(root);
    records.insert(0, record);
    if records.len() > 200 {
        records.truncate(200);
    }
    save_install_history(root, &records)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn discover_skills_in_dir(dir: &Path) -> Result<Vec<SkillCandidate>, String> {
    let mut candidates = Vec::new();
    for entry in WalkDir::new(dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "SKILL.md" {
            continue;
        }
        let skill_dir = entry.path().parent().unwrap_or(dir);
        let dir_name = skill_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        let doc = parse_skill_markdown(&content).map_err(|e| e.to_string())?;
        candidates.push(SkillCandidate {
            id: dir_name,
            name: doc.frontmatter.name,
            description: doc.frontmatter.description,
            path: skill_dir.to_string_lossy().to_string(),
        });
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    candidates.dedup_by(|a, b| a.id == b.id);
    Ok(candidates)
}

fn resolve_skill_source_path(source_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(source_path);
    if !path.exists() {
        return Err("Source path not found".to_string());
    }
    if path.is_file() {
        return Ok(path.parent().unwrap_or(path).to_path_buf());
    }
    Ok(path.to_path_buf())
}

fn validate_relative_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Path is required".to_string());
    }
    let rel = Path::new(path);
    if rel.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }
    for comp in rel.components() {
        match comp {
            Component::ParentDir => return Err("Parent directory paths are not allowed".to_string()),
            Component::CurDir => return Err("Current directory paths are not allowed".to_string()),
            _ => {}
        }
    }
    Ok(rel.to_path_buf())
}

fn resolve_skill_file_for_write(skill_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let rel = validate_relative_path(relative_path)?;
    let target = skill_dir.join(&rel);
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid file path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let canonical_root = std::fs::canonicalize(skill_dir).map_err(|e| e.to_string())?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("Path escapes skill directory".to_string());
    }
    Ok(target)
}

/// List files under a skill directory (excluding directories)
pub async fn list_skill_files(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<SkillFileEntry>, String> {
    let root = skills_root(&db_service);
    let skill_dir = resolve_skill_dir(&root, &id)?;
    let mut files = Vec::new();

    for entry in WalkDir::new(&skill_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            let rel = path
                .strip_prefix(&skill_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(SkillFileEntry { path: rel, size });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Read a skill file content
pub async fn read_skill_file(
    id: String,
    path: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    let root = skills_root(&db_service);
    let skill_dir = resolve_skill_dir(&root, &id)?;
    let rel = validate_relative_path(&path)?;
    let target = skill_dir.join(&rel);
    
    // Security check: ensure the file is within the skill directory
    let canonical_root = std::fs::canonicalize(&skill_dir).map_err(|e| e.to_string())?;
    let canonical_file = std::fs::canonicalize(&target).map_err(|e| e.to_string())?;
    
    if !canonical_file.starts_with(&canonical_root) {
        return Err("Path escapes skill directory".to_string());
    }
    
    let content = std::fs::read_to_string(&canonical_file).map_err(|e| e.to_string())?;
    Ok(content)
}

/// Save a skill file (create/update)
pub async fn save_skill_file(
    id: String,
    path: String,
    content: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let root = skills_root(&db_service);
    let skill_dir = resolve_skill_dir(&root, &id)?;
    if content.as_bytes().len() > 500 * 1024 {
        return Err("File too large (max 500KB)".to_string());
    }
    let target = resolve_skill_file_for_write(&skill_dir, &path)?;
    std::fs::write(&target, content).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Delete a skill file
pub async fn delete_skill_file(
    id: String,
    path: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let root = skills_root(&db_service);
    let skill_dir = resolve_skill_dir(&root, &id)?;
    let rel = validate_relative_path(&path)?;
    let target = skill_dir.join(&rel);
    let canonical_root = std::fs::canonicalize(&skill_dir).map_err(|e| e.to_string())?;
    let canonical_file = std::fs::canonicalize(&target).map_err(|e| e.to_string())?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err("Path escapes skill directory".to_string());
    }
    std::fs::remove_file(&canonical_file).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Import a file into a skill directory (copy from path)
pub async fn import_skill_file(
    id: String,
    source_path: String,
    target_path: Option<String>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    let root = skills_root(&db_service);
    let skill_dir = resolve_skill_dir(&root, &id)?;
    let source = Path::new(&source_path);
    if !source.exists() || !source.is_file() {
        return Err("Source file not found".to_string());
    }
    let metadata = std::fs::metadata(source).map_err(|e| e.to_string())?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err("File too large (max 2MB)".to_string());
    }
    let file_name = source
        .file_name()
        .ok_or_else(|| "Invalid source file name".to_string())?
        .to_string_lossy()
        .to_string();
    let relative = target_path.unwrap_or(file_name);
    let target = resolve_skill_file_for_write(&skill_dir, &relative)?;
    std::fs::copy(source, &target).map_err(|e| e.to_string())?;
    Ok(relative)
}

/// Discover skills from a local file or directory
pub async fn discover_skills_from_path(
    source_path: String,
) -> Result<Vec<SkillCandidate>, String> {
    let resolved = resolve_skill_source_path(&source_path)?;
    discover_skills_in_dir(&resolved)
}

/// Clone a git repo to a temp directory and discover skills
pub async fn discover_skills_from_git(
    url: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(String, Vec<SkillCandidate>), String> {
    let root = skills_root(&db_service);
    let import_root = skills_import_root(&root);
    fs::create_dir_all(&import_root).map_err(|e| e.to_string())?;
    let temp_dir = import_root.join(format!("git-{}", Uuid::new_v4()));

    let status = std::process::Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&url)
        .arg(&temp_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            let candidates = discover_skills_in_dir(&temp_dir)?;
            Ok((temp_dir.to_string_lossy().to_string(), candidates))
        }
        Ok(s) => Err(format!("git clone failed (exit {})", s)),
        Err(e) => Err(format!("git clone failed: {}", e)),
    }
}

/// Install selected skills from a local directory
pub async fn install_skills_from_path(
    source_path: String,
    skill_ids: Vec<String>,
    source_type: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<String>, String> {
    let root = skills_root(&db_service);
    let resolved = resolve_skill_source_path(&source_path)?;
    let candidates = discover_skills_in_dir(&resolved)?;

    if candidates.is_empty() {
        return Err("No skills found in source".to_string());
    }

    let target_ids = if skill_ids.is_empty() && candidates.len() == 1 {
        vec![candidates[0].id.clone()]
    } else {
        skill_ids
    };

    if target_ids.is_empty() {
        return Err("No skills selected".to_string());
    }

    let mut installed = Vec::new();
    for skill_id in target_ids.iter() {
        let candidate = candidates
            .iter()
            .find(|c| &c.id == skill_id)
            .ok_or_else(|| format!("Skill {} not found in source", skill_id))?;

        let src_dir = Path::new(&candidate.path);
        let dest_dir = root.join(skill_id);
        if dest_dir.exists() {
            fs::remove_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        }
        copy_dir_all(src_dir, &dest_dir)?;
        installed.push(skill_id.clone());
    }

    let _ = scan_and_upsert_skills(&db_service).await;

    let record = SkillInstallHistory {
        id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        source_type,
        source: source_path,
        skills: installed.clone(),
        status: "success".to_string(),
        message: None,
    };
    let _ = append_install_history(&root, record);

    Ok(installed)
}

/// Read install history
pub async fn list_skill_install_history(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<SkillInstallHistory>, String> {
    let root = skills_root(&db_service);
    Ok(load_install_history(&root))
}

/// Delete a record from install history
pub async fn delete_skill_install_history(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let root = skills_root(&db_service);
    let mut records = load_install_history(&root);
    let before = records.len();
    records.retain(|r| r.id != id);
    if records.len() == before {
        return Ok(false);
    }
    save_install_history(&root, &records)?;
    Ok(true)
}
