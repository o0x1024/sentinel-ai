//! Advanced audit tools providing deeper code analysis capabilities.
//!
//! - `read_file`          — Read file content with line ranges or list directories
//! - `project_overview`   — Detect languages, frameworks, entry points, and dependencies
//! - `audit_coverage`     — Per-session audit coverage tracking
//! - `dependency_audit`   — Scan dependency manifests for known vulnerabilities
//! - `cross_file_taint`   — Cross-file source-to-sink taint heuristic analysis
//! - `audit_report`       — Export audit findings to Markdown or SARIF format

use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

use super::audit_tools::{
    load_audit_runtime, parse_rg_line, resolve_effective_path, run_audit_command, shell_escape,
    AuditRuntime,
};
use super::shell::ShellExecutionMode;

// ============================================================================
// ReadFileTool
// ============================================================================

const READ_FILE_DEFAULT_LIMIT: usize = 200;
const READ_FILE_MAX_LIMIT: usize = 2000;
const READ_FILE_MAX_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadFileArgs {
    /// File or directory path to read
    pub path: String,
    /// Start line number (1-indexed). Default: 1
    #[serde(default)]
    pub offset: Option<usize>,
    /// Maximum lines to return. Default: 200, max: 2000
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadFileOutput {
    pub path: String,
    pub is_directory: bool,
    /// Directory entries (only when path is a directory)
    pub entries: Vec<DirEntry>,
    /// File content with line numbers prepended (only when path is a file)
    pub content: Option<String>,
    /// Total line count of the file
    pub total_lines: Option<usize>,
    /// First line returned (1-indexed)
    pub offset: usize,
    pub returned_lines: usize,
    pub truncated: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFileError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
}

#[derive(Debug, Clone)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub const NAME: &'static str = "read_file";
    pub const DESCRIPTION: &'static str =
        "Read file content with optional line range, or list directory entries. \
         Essential for reading source code, config files, and exploring project structure during code audit.";
}

impl Tool for ReadFileTool {
    const NAME: &'static str = Self::NAME;
    type Args = ReadFileArgs;
    type Output = ReadFileOutput;
    type Error = ReadFileError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ReadFileArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.path.trim().to_string();
        if path.is_empty() {
            return Err(ReadFileError::InvalidArgs("path is required".to_string()));
        }
        let limit = args
            .limit
            .unwrap_or(READ_FILE_DEFAULT_LIMIT)
            .clamp(1, READ_FILE_MAX_LIMIT);
        let offset = args.offset.unwrap_or(1).max(1);
        let runtime = load_audit_runtime().await;
        match runtime.mode {
            ShellExecutionMode::Docker => read_file_docker(&path, offset, limit, &runtime).await,
            ShellExecutionMode::Host => read_file_host(&path, offset, limit).await,
        }
    }
}

async fn is_dir_docker(path: &str, runtime: &AuditRuntime) -> bool {
    let script = format!("[ -d {} ] && echo DIR || echo FILE", shell_escape(path));
    let args = vec!["-c".to_string(), script];
    run_audit_command("bash", &args, 5, runtime, false)
        .await
        .map(|o| o.stdout.trim() == "DIR")
        .unwrap_or(false)
}

async fn path_exists_docker(path: &str, runtime: &AuditRuntime) -> bool {
    let script = format!(
        "[ -e {} ] && echo EXISTS || echo MISSING",
        shell_escape(path)
    );
    let args = vec!["-c".to_string(), script];
    run_audit_command("bash", &args, 5, runtime, false)
        .await
        .map(|o| o.stdout.trim() == "EXISTS")
        .unwrap_or(false)
}

async fn workspace_top_entries_docker(runtime: &AuditRuntime) -> Vec<String> {
    let args = vec!["-1".to_string(), "/workspace".to_string()];
    let out = run_audit_command("ls", &args, 8, runtime, true).await;
    match out {
        Ok(v) => v
            .stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .take(20)
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn find_same_name_candidates_docker(path: &str, runtime: &AuditRuntime) -> Vec<String> {
    let Some(file_name) = Path::new(path).file_name().and_then(|v| v.to_str()) else {
        return Vec::new();
    };
    if file_name.trim().is_empty() {
        return Vec::new();
    }

    let script = format!(
        "find /workspace -type f -name {} 2>/dev/null | head -n 20",
        shell_escape(file_name)
    );
    let args = vec!["-lc".to_string(), script];
    let out = run_audit_command("bash", &args, 20, runtime, true).await;
    match out {
        Ok(v) => v
            .stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn read_file_docker(
    path: &str,
    offset: usize,
    limit: usize,
    runtime: &AuditRuntime,
) -> Result<ReadFileOutput, ReadFileError> {
    if !path_exists_docker(path, runtime).await {
        let workspace_entries = workspace_top_entries_docker(runtime).await;
        let candidates = find_same_name_candidates_docker(path, runtime).await;
        let mut hints = Vec::new();
        hints.push("path not found in docker workspace".to_string());
        if !workspace_entries.is_empty() {
            hints.push(format!(
                "top-level /workspace entries: {}",
                workspace_entries.join(", ")
            ));
        }
        if !candidates.is_empty() {
            hints.push(format!(
                "same-name file candidates: {}",
                candidates.join(", ")
            ));
        }
        hints.push(
            "tip: call read_file with path='/workspace' first to discover mounted project roots"
                .to_string(),
        );
        return Err(ReadFileError::Io(format!(
            "{} ({})",
            path,
            hints.join("; ")
        )));
    }

    if is_dir_docker(path, runtime).await {
        let args = vec![
            "--color=never".to_string(),
            "-la".to_string(),
            path.to_string(),
        ];
        let out = run_audit_command("ls", &args, 10, runtime, false)
            .await
            .map_err(ReadFileError::CommandFailed)?;
        let entries = parse_ls_output(&out.stdout);
        return Ok(ReadFileOutput {
            path: path.to_string(),
            is_directory: true,
            entries,
            content: None,
            total_lines: None,
            offset: 1,
            returned_lines: 0,
            truncated: false,
            size_bytes: None,
        });
    }

    // Count total lines
    let total_lines = {
        let wc_args = vec!["-l".to_string(), path.to_string()];
        run_audit_command("wc", &wc_args, 10, runtime, false)
            .await
            .ok()
            .and_then(|o| o.stdout.split_whitespace().next()?.parse::<usize>().ok())
    };

    let end = offset + limit - 1;
    let sed_script = format!("{},{}p", offset, end);
    let sed_args = vec!["-n".to_string(), sed_script, path.to_string()];
    let out = run_audit_command("sed", &sed_args, 30, runtime, false)
        .await
        .map_err(ReadFileError::CommandFailed)?;

    let lines: Vec<&str> = out.stdout.lines().collect();
    let returned = lines.len();
    let truncated = total_lines
        .map(|t| t > offset + returned.saturating_sub(1))
        .unwrap_or(false);
    let numbered: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:6}|{}", offset + i, line))
        .collect();

    Ok(ReadFileOutput {
        path: path.to_string(),
        is_directory: false,
        entries: vec![],
        content: Some(numbered.join("\n")),
        total_lines,
        offset,
        returned_lines: returned,
        truncated,
        size_bytes: None,
    })
}

async fn read_file_host(
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<ReadFileOutput, ReadFileError> {
    // Remap /workspace prefix when path doesn't exist locally
    let effective = {
        let p = Path::new(path);
        if p.exists() {
            path.to_string()
        } else if let Ok(stripped) = p.strip_prefix("/workspace") {
            let alt = Path::new(".").join(stripped);
            if alt.exists() {
                alt.to_string_lossy().to_string()
            } else {
                return Err(ReadFileError::Io(format!("path not found: {}", path)));
            }
        } else {
            return Err(ReadFileError::Io(format!("path not found: {}", path)));
        }
    };

    let meta = tokio::fs::metadata(&effective)
        .await
        .map_err(|e| ReadFileError::Io(e.to_string()))?;

    if meta.is_dir() {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&effective)
            .await
            .map_err(|e| ReadFileError::Io(e.to_string()))?;
        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| ReadFileError::Io(e.to_string()))?
        {
            let m = entry.metadata().await.ok();
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: m.as_ref().map(|x| x.is_dir()).unwrap_or(false),
                size_bytes: m.as_ref().map(|x| x.len()),
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        return Ok(ReadFileOutput {
            path: effective,
            is_directory: true,
            entries,
            content: None,
            total_lines: None,
            offset: 1,
            returned_lines: 0,
            truncated: false,
            size_bytes: Some(meta.len()),
        });
    }

    if meta.len() > READ_FILE_MAX_BYTES {
        return Err(ReadFileError::InvalidArgs(format!(
            "file too large ({} bytes, max {})",
            meta.len(),
            READ_FILE_MAX_BYTES
        )));
    }

    let raw = tokio::fs::read_to_string(&effective)
        .await
        .map_err(|e| ReadFileError::Io(format!("failed to read {}: {}", effective, e)))?;

    let all: Vec<&str> = raw.lines().collect();
    let total = all.len();
    let start = (offset - 1).min(total);
    let end = (start + limit).min(total);
    let slice = &all[start..end];
    let returned = slice.len();
    let truncated = end < total;
    let numbered: Vec<String> = slice
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:6}|{}", offset + i, line))
        .collect();

    Ok(ReadFileOutput {
        path: effective,
        is_directory: false,
        entries: vec![],
        content: Some(numbered.join("\n")),
        total_lines: Some(total),
        offset,
        returned_lines: returned,
        truncated,
        size_bytes: Some(meta.len()),
    })
}

fn parse_ls_output(output: &str) -> Vec<DirEntry> {
    let mut entries = Vec::new();
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }
        let is_dir = parts[0].starts_with('d');
        let size = parts[4].parse::<u64>().ok();
        let name = parts[8..].join(" ");
        if name == "." || name == ".." {
            continue;
        }
        entries.push(DirEntry {
            name,
            is_dir,
            size_bytes: size,
        });
    }
    entries
}

// ============================================================================
// ProjectOverviewTool
// ============================================================================

const PROJECT_MAX_DEPTH: usize = 4;
const PROJECT_MAX_FILES: usize = 8000;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ProjectOverviewArgs {
    #[serde(default)]
    pub path: Option<String>,
    /// Max directory depth to scan. Default: 4
    #[serde(default)]
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LangStat {
    pub language: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntryPointInfo {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestInfo {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectOverviewOutput {
    pub path: String,
    pub total_files: usize,
    pub languages: Vec<LangStat>,
    pub frameworks: Vec<String>,
    pub entry_points: Vec<EntryPointInfo>,
    pub dependency_manifests: Vec<ManifestInfo>,
    /// Top-level directory listing (2 levels deep)
    pub structure: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectOverviewError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
}

#[derive(Debug, Clone)]
pub struct ProjectOverviewTool;

impl ProjectOverviewTool {
    pub const NAME: &'static str = "project_overview";
    pub const DESCRIPTION: &'static str =
        "Scan a project directory to detect programming languages, frameworks, entry points, \
         and dependency manifests. Use this at the start of a code audit to establish global context \
         before diving into specific files.";
}

impl Tool for ProjectOverviewTool {
    const NAME: &'static str = Self::NAME;
    type Args = ProjectOverviewArgs;
    type Output = ProjectOverviewOutput;
    type Error = ProjectOverviewError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ProjectOverviewArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let runtime = load_audit_runtime().await;
        let path = resolve_effective_path(args.path.as_deref(), &runtime.mode);
        let max_depth = args.max_depth.unwrap_or(PROJECT_MAX_DEPTH).clamp(1, 8);

        match runtime.mode {
            ShellExecutionMode::Docker => project_overview_docker(&path, max_depth, &runtime).await,
            ShellExecutionMode::Host => project_overview_host(&path, max_depth).await,
        }
    }
}

async fn project_overview_docker(
    path: &str,
    max_depth: usize,
    runtime: &AuditRuntime,
) -> Result<ProjectOverviewOutput, ProjectOverviewError> {
    // find all files up to max_depth, print extension
    let find_args = vec![
        path.to_string(),
        "-maxdepth".to_string(),
        max_depth.to_string(),
        "-type".to_string(),
        "f".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/.git/*".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/node_modules/*".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/target/*".to_string(),
    ];
    let out = run_audit_command("find", &find_args, 30, runtime, false)
        .await
        .map_err(ProjectOverviewError::CommandFailed)?;
    let files: Vec<&str> = out.stdout.lines().collect();
    build_overview_from_file_list(path, &files)
}

async fn project_overview_host(
    path: &str,
    max_depth: usize,
) -> Result<ProjectOverviewOutput, ProjectOverviewError> {
    use walkdir::WalkDir;
    let mut files: Vec<String> = Vec::new();
    let walker = WalkDir::new(path)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | "node_modules" | "target" | "dist" | "build" | ".next" | "coverage"
            )
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            files.push(entry.path().to_string_lossy().to_string());
            if files.len() >= PROJECT_MAX_FILES {
                break;
            }
        }
    }
    let borrowed: Vec<&str> = files.iter().map(String::as_str).collect();
    build_overview_from_file_list(path, &borrowed)
}

fn build_overview_from_file_list(
    root: &str,
    files: &[&str],
) -> Result<ProjectOverviewOutput, ProjectOverviewError> {
    let truncated = files.len() >= PROJECT_MAX_FILES;
    let total_files = files.len();

    // Language detection by extension
    let ext_to_lang: &[(&str, &str)] = &[
        ("rs", "Rust"),
        ("go", "Go"),
        ("py", "Python"),
        ("js", "JavaScript"),
        ("ts", "TypeScript"),
        ("jsx", "JavaScript"),
        ("tsx", "TypeScript"),
        ("java", "Java"),
        ("kt", "Kotlin"),
        ("php", "PHP"),
        ("rb", "Ruby"),
        ("cs", "C#"),
        ("cpp", "C++"),
        ("c", "C"),
        ("h", "C/C++"),
        ("swift", "Swift"),
        ("scala", "Scala"),
        ("sh", "Shell"),
        ("bash", "Shell"),
        ("html", "HTML"),
        ("css", "CSS"),
        ("scss", "SCSS"),
        ("vue", "Vue"),
        ("svelte", "Svelte"),
        ("sql", "SQL"),
        ("yaml", "YAML"),
        ("yml", "YAML"),
        ("json", "JSON"),
        ("toml", "TOML"),
        ("tf", "Terraform"),
        ("proto", "Protobuf"),
    ];
    let mut lang_counts: HashMap<&str, usize> = HashMap::new();
    for file in files {
        let ext = Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if let Some((_, lang)) = ext_to_lang.iter().find(|(e, _)| *e == ext) {
            *lang_counts.entry(lang).or_insert(0) += 1;
        }
    }
    let mut languages: Vec<LangStat> = lang_counts
        .into_iter()
        .map(|(language, file_count)| LangStat {
            language: language.to_string(),
            file_count,
        })
        .collect();
    languages.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    languages.truncate(10);

    // Framework / manifest detection
    let manifest_patterns: &[(&str, &str)] = &[
        ("package.json", "npm"),
        ("Cargo.toml", "cargo"),
        ("requirements.txt", "pip"),
        ("go.mod", "go"),
        ("pom.xml", "maven"),
        ("build.gradle", "gradle"),
        ("build.gradle.kts", "gradle"),
        ("composer.json", "composer"),
        ("Gemfile", "bundler"),
        ("pyproject.toml", "pyproject"),
        ("poetry.lock", "poetry"),
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
        ("Pipfile", "pipenv"),
    ];
    let mut dependency_manifests: Vec<ManifestInfo> = Vec::new();
    let mut manifests_found: Vec<&str> = Vec::new();
    for file in files {
        let fname = Path::new(file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if let Some((_, kind)) = manifest_patterns.iter().find(|(pat, _)| *pat == fname) {
            dependency_manifests.push(ManifestInfo {
                path: file.to_string(),
                kind: kind.to_string(),
            });
            manifests_found.push(kind);
        }
    }

    // Framework detection from file patterns
    let mut frameworks: Vec<String> = Vec::new();
    let file_set: std::collections::HashSet<String> =
        files.iter().map(|f| f.to_lowercase()).collect();
    let has_file = |name: &str| file_set.iter().any(|f| f.ends_with(name));
    let has_content_pattern = |pat: &str| files.iter().any(|f| f.to_lowercase().contains(pat));

    if has_file("next.config.js") || has_file("next.config.ts") {
        frameworks.push("Next.js".to_string());
    }
    if has_content_pattern("/nuxt.config") {
        frameworks.push("Nuxt.js".to_string());
    }
    if file_set.iter().any(|f| f.ends_with(".vue")) {
        frameworks.push("Vue.js".to_string());
    }
    if has_file("angular.json") {
        frameworks.push("Angular".to_string());
    }
    if has_content_pattern("react")
        && file_set
            .iter()
            .any(|f| f.ends_with(".jsx") || f.ends_with(".tsx"))
    {
        frameworks.push("React".to_string());
    }
    if has_file("manage.py") || has_content_pattern("django") {
        frameworks.push("Django".to_string());
    }
    if has_content_pattern("flask") {
        frameworks.push("Flask".to_string());
    }
    if has_content_pattern("fastapi") {
        frameworks.push("FastAPI".to_string());
    }
    if has_content_pattern("spring")
        || has_file("application.properties")
        || has_file("application.yml")
    {
        frameworks.push("Spring".to_string());
    }
    if manifests_found.contains(&"cargo") {
        frameworks.push("Rust/Cargo".to_string());
    }
    if manifests_found.contains(&"go") {
        frameworks.push("Go Modules".to_string());
    }
    if has_content_pattern("express") {
        frameworks.push("Express.js".to_string());
    }
    if has_content_pattern("laravel") {
        frameworks.push("Laravel".to_string());
    }
    if has_content_pattern("rails") || has_file("config/routes.rb") {
        frameworks.push("Ruby on Rails".to_string());
    }
    frameworks.dedup();

    // Entry point detection
    let entry_patterns: &[(&str, &str)] = &[
        ("main.rs", "main"),
        ("lib.rs", "library"),
        ("main.go", "main"),
        ("main.py", "main"),
        ("app.py", "app"),
        ("index.js", "index"),
        ("index.ts", "index"),
        ("app.js", "app"),
        ("app.ts", "app"),
        ("server.js", "server"),
        ("server.ts", "server"),
        ("manage.py", "django-manage"),
        ("wsgi.py", "wsgi"),
        ("asgi.py", "asgi"),
        ("cmd/main.go", "go-main"),
        ("src/main.rs", "main"),
        ("src/main.ts", "main"),
        ("src/main.js", "main"),
        ("src/index.ts", "index"),
        ("src/index.js", "index"),
        ("src/app.ts", "app"),
        ("src/app.js", "app"),
    ];
    let mut entry_points: Vec<EntryPointInfo> = Vec::new();
    for file in files {
        let lower = file.to_lowercase();
        for (pat, kind) in entry_patterns {
            if lower.ends_with(pat) {
                entry_points.push(EntryPointInfo {
                    path: file.to_string(),
                    kind: kind.to_string(),
                });
                break;
            }
        }
    }
    entry_points.truncate(20);

    // Top-level structure (up to 2 levels)
    let mut seen_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut structure: Vec<String> = Vec::new();
    for file in files.iter().take(2000) {
        let rel = file.trim_start_matches(root).trim_start_matches('/');
        let parts: Vec<&str> = rel.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let dir_entry = format!("{}/", parts[0]);
            if seen_dirs.insert(dir_entry.clone()) {
                structure.push(dir_entry);
            }
            if parts.len() >= 3 {
                let sub = format!("  {}/", parts[1]);
                let key = format!("{}/{}", parts[0], parts[1]);
                if seen_dirs.insert(key) {
                    structure.push(sub);
                }
            }
        } else if parts.len() == 1 && !parts[0].is_empty() {
            let key = format!("__file__{}", parts[0]);
            if seen_dirs.insert(key) {
                structure.push(parts[0].to_string());
            }
        }
    }
    structure.sort();
    structure.dedup();

    Ok(ProjectOverviewOutput {
        path: root.to_string(),
        total_files,
        languages,
        frameworks,
        entry_points,
        dependency_manifests,
        structure,
        truncated,
    })
}

// ============================================================================
// AuditCoverageTool
// ============================================================================

#[derive(Default)]
struct CoverageSession {
    audited: std::collections::HashSet<String>,
    todo: std::collections::HashSet<String>,
    notes: HashMap<String, String>,
}

static AUDIT_COVERAGE: Lazy<RwLock<HashMap<String, CoverageSession>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AuditCoverageArgs {
    /// Session identifier, typically the conversation_id
    pub session_id: String,
    /// Operation: "mark_audited" | "mark_todo" | "list" | "summary" | "reset"
    pub operation: String,
    /// File or module paths for mark operations
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    /// Optional note attached to each path
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditCoverageOutput {
    pub session_id: String,
    pub operation: String,
    pub audited: Vec<String>,
    pub todo: Vec<String>,
    pub audited_count: usize,
    pub todo_count: usize,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditCoverageError {
    #[error("invalid operation: {0}")]
    InvalidOp(String),
}

#[derive(Debug, Clone)]
pub struct AuditCoverageTool;

impl AuditCoverageTool {
    pub const NAME: &'static str = "audit_coverage";
    pub const DESCRIPTION: &'static str =
        "Track audit coverage per session. Mark files/modules as audited or pending, \
         list coverage status, and reset state. Helps ensure no code paths are missed \
         during a systematic code audit.";
}

impl Tool for AuditCoverageTool {
    const NAME: &'static str = Self::NAME;
    type Args = AuditCoverageArgs;
    type Output = AuditCoverageOutput;
    type Error = AuditCoverageError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AuditCoverageArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let op = args.operation.trim().to_lowercase();
        let session = args.session_id.clone();
        let paths = args.paths.unwrap_or_default();
        let note = args.note.unwrap_or_default();

        match op.as_str() {
            "mark_audited" => {
                let mut state = AUDIT_COVERAGE.write().await;
                let s = state.entry(session.clone()).or_default();
                for p in &paths {
                    s.audited.insert(p.clone());
                    s.todo.remove(p);
                    if !note.is_empty() {
                        s.notes.insert(p.clone(), note.clone());
                    }
                }
                let (audited, todo) = snapshot(s);
                Ok(AuditCoverageOutput {
                    session_id: session,
                    operation: "mark_audited".to_string(),
                    audited_count: audited.len(),
                    todo_count: todo.len(),
                    audited,
                    todo,
                    message: format!("Marked {} path(s) as audited.", paths.len()),
                })
            }
            "mark_todo" => {
                let mut state = AUDIT_COVERAGE.write().await;
                let s = state.entry(session.clone()).or_default();
                for p in &paths {
                    if !s.audited.contains(p) {
                        s.todo.insert(p.clone());
                    }
                    if !note.is_empty() {
                        s.notes.insert(p.clone(), note.clone());
                    }
                }
                let (audited, todo) = snapshot(s);
                Ok(AuditCoverageOutput {
                    session_id: session,
                    operation: "mark_todo".to_string(),
                    audited_count: audited.len(),
                    todo_count: todo.len(),
                    audited,
                    todo,
                    message: format!("Queued {} path(s) for audit.", paths.len()),
                })
            }
            "list" | "summary" => {
                let state = AUDIT_COVERAGE.read().await;
                let (audited, todo) = if let Some(s) = state.get(&session) {
                    snapshot(s)
                } else {
                    (vec![], vec![])
                };
                let msg = format!(
                    "Coverage: {}/{} audited.",
                    audited.len(),
                    audited.len() + todo.len()
                );
                Ok(AuditCoverageOutput {
                    session_id: session,
                    operation: "list".to_string(),
                    audited_count: audited.len(),
                    todo_count: todo.len(),
                    audited,
                    todo,
                    message: msg,
                })
            }
            "reset" => {
                AUDIT_COVERAGE.write().await.remove(&session);
                Ok(AuditCoverageOutput {
                    session_id: session,
                    operation: "reset".to_string(),
                    audited_count: 0,
                    todo_count: 0,
                    audited: vec![],
                    todo: vec![],
                    message: "Coverage state reset.".to_string(),
                })
            }
            _ => Err(AuditCoverageError::InvalidOp(format!(
                "unknown operation '{}'; expected: mark_audited | mark_todo | list | summary | reset",
                op
            ))),
        }
    }
}

fn snapshot(s: &CoverageSession) -> (Vec<String>, Vec<String>) {
    let mut audited: Vec<String> = s.audited.iter().cloned().collect();
    audited.sort();
    let mut todo: Vec<String> = s.todo.iter().cloned().collect();
    todo.sort();
    (audited, todo)
}

// ============================================================================
// DependencyAuditTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DependencyAuditArgs {
    /// Root path to scan for dependency manifests. Default: workspace root
    #[serde(default)]
    pub path: Option<String>,
    /// Run security scanner tools if available (npm audit, cargo audit, etc.)
    #[serde(default = "default_run_scanners")]
    pub run_scanners: bool,
}

fn default_run_scanners() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct DepInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VulnInfo {
    pub package: String,
    pub installed_version: String,
    pub severity: String,
    pub title: String,
    pub cve: Option<String>,
    pub fixed_in: Option<String>,
    pub advisory_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestScan {
    pub kind: String,
    pub path: String,
    pub dependencies: Vec<DepInfo>,
    pub vulnerabilities: Vec<VulnInfo>,
    /// "scanner" | "static_only" | "scanner_unavailable" | "error"
    pub scan_method: String,
    pub scanner_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyAuditOutput {
    pub path: String,
    pub manifests: Vec<ManifestScan>,
    pub total_dependencies: usize,
    pub total_vulnerabilities: usize,
    pub critical_or_high_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DependencyAuditError {
    #[error("command failed: {0}")]
    CommandFailed(String),
}

#[derive(Debug, Clone)]
pub struct DependencyAuditTool;

impl DependencyAuditTool {
    pub const NAME: &'static str = "dependency_audit";
    pub const DESCRIPTION: &'static str =
        "Scan dependency manifests (package.json, Cargo.toml, requirements.txt, go.mod, pom.xml) \
         for known vulnerabilities. Runs npm audit, cargo audit, or pip-audit when available. \
         Returns structured vulnerability data with severity, CVE IDs, and fix versions.";
}

impl Tool for DependencyAuditTool {
    const NAME: &'static str = Self::NAME;
    type Args = DependencyAuditArgs;
    type Output = DependencyAuditOutput;
    type Error = DependencyAuditError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(DependencyAuditArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let runtime = load_audit_runtime().await;
        let path = resolve_effective_path(args.path.as_deref(), &runtime.mode);
        let run_scanners = args.run_scanners;

        // Find manifest files using code_search approach
        let manifest_names = [
            "package.json",
            "Cargo.toml",
            "requirements.txt",
            "go.mod",
            "pom.xml",
            "build.gradle",
            "composer.json",
            "Gemfile",
            "pyproject.toml",
        ];

        let mut manifests: Vec<ManifestScan> = Vec::new();

        for manifest_name in &manifest_names {
            let found = find_manifest_files(&path, manifest_name, &runtime).await;
            for manifest_path in found {
                let kind = manifest_kind(manifest_name);
                let content = read_manifest_content(&manifest_path, &runtime).await;
                let deps = parse_dependencies(kind, &content);
                let (vulns, method, note) = if run_scanners {
                    run_security_scanner(kind, &manifest_path, &runtime).await
                } else {
                    (
                        vec![],
                        "static_only".to_string(),
                        Some("Scanner skipped (run_scanners=false)".to_string()),
                    )
                };
                manifests.push(ManifestScan {
                    kind: kind.to_string(),
                    path: manifest_path,
                    dependencies: deps,
                    vulnerabilities: vulns,
                    scan_method: method,
                    scanner_note: note,
                });
            }
        }

        let total_dependencies: usize = manifests.iter().map(|m| m.dependencies.len()).sum();
        let total_vulnerabilities: usize = manifests.iter().map(|m| m.vulnerabilities.len()).sum();
        let critical_or_high_count: usize = manifests
            .iter()
            .flat_map(|m| &m.vulnerabilities)
            .filter(|v| matches!(v.severity.to_lowercase().as_str(), "critical" | "high"))
            .count();

        Ok(DependencyAuditOutput {
            path,
            manifests,
            total_dependencies,
            total_vulnerabilities,
            critical_or_high_count,
        })
    }
}

async fn find_manifest_files(root: &str, name: &str, runtime: &AuditRuntime) -> Vec<String> {
    // Use find command to locate manifest files (skip .git, node_modules, target)
    let find_args = vec![
        root.to_string(),
        "-maxdepth".to_string(),
        "6".to_string(),
        "-name".to_string(),
        name.to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/.git/*".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/node_modules/*".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/target/*".to_string(),
        "!".to_string(),
        "-path".to_string(),
        "*/.cargo/*".to_string(),
    ];
    match run_audit_command("find", &find_args, 20, runtime, true).await {
        Ok(out) => out
            .stdout
            .lines()
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => vec![],
    }
}

async fn read_manifest_content(path: &str, runtime: &AuditRuntime) -> String {
    let args = vec![path.to_string()];
    run_audit_command("cat", &args, 10, runtime, false)
        .await
        .map(|o| o.stdout)
        .unwrap_or_default()
}

fn manifest_kind(name: &str) -> &'static str {
    match name {
        "package.json" => "npm",
        "Cargo.toml" => "cargo",
        "requirements.txt" => "pip",
        "go.mod" => "go",
        "pom.xml" => "maven",
        "build.gradle" | "build.gradle.kts" => "gradle",
        "composer.json" => "composer",
        "Gemfile" => "bundler",
        "pyproject.toml" => "pyproject",
        _ => "unknown",
    }
}

fn parse_dependencies(kind: &str, content: &str) -> Vec<DepInfo> {
    match kind {
        "npm" => parse_npm_deps(content),
        "cargo" => parse_cargo_deps(content),
        "pip" => parse_pip_deps(content),
        "go" => parse_go_deps(content),
        _ => vec![],
    }
}

fn parse_npm_deps(content: &str) -> Vec<DepInfo> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(content) else {
        return vec![];
    };
    let mut deps = Vec::new();
    for section in &["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = json.get(section).and_then(|v| v.as_object()) {
            for (name, ver) in obj {
                deps.push(DepInfo {
                    name: name.clone(),
                    version: ver.as_str().unwrap_or("*").to_string(),
                });
            }
        }
    }
    deps
}

fn parse_cargo_deps(content: &str) -> Vec<DepInfo> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    // Track table-style deps like [dependencies.serde]
    let mut table_dep_name: Option<String> = None;

    static RE_INLINE_VERSION: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap());

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for [dependencies.name] style
        if let Some(rest) = trimmed.strip_prefix("[dependencies.") {
            if let Some(name) = rest.strip_suffix(']') {
                table_dep_name = Some(name.to_string());
                in_deps = false;
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix("[dev-dependencies.") {
            if let Some(name) = rest.strip_suffix(']') {
                table_dep_name = Some(name.to_string());
                in_deps = false;
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix("[build-dependencies.") {
            if let Some(name) = rest.strip_suffix(']') {
                table_dep_name = Some(name.to_string());
                in_deps = false;
                continue;
            }
        }

        // Standard dependency sections
        if trimmed.starts_with("[dependencies]")
            || trimmed.starts_with("[dev-dependencies]")
            || trimmed.starts_with("[build-dependencies]")
        {
            in_deps = true;
            table_dep_name = None;
            continue;
        }

        // Any other section header terminates
        if trimmed.starts_with('[') {
            in_deps = false;
            table_dep_name = None;
            continue;
        }

        // Handle table-style dep: extract version = "..."
        if let Some(ref dep_name) = table_dep_name {
            if trimmed.starts_with("version") {
                if let Some(caps) = RE_INLINE_VERSION.captures(trimmed) {
                    deps.push(DepInfo {
                        name: dep_name.clone(),
                        version: caps[1].to_string(),
                    });
                }
                table_dep_name = None;
            }
            continue;
        }

        if in_deps && !trimmed.starts_with('#') && trimmed.contains('=') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let rhs = parts[1].trim();

                if name.is_empty() {
                    continue;
                }

                let ver = if rhs.starts_with('{') {
                    // Inline table: serde = { version = "1.0", features = ["derive"] }
                    RE_INLINE_VERSION
                        .captures(rhs)
                        .map(|caps| caps[1].to_string())
                        .unwrap_or_else(|| "*".to_string())
                } else {
                    // Simple: serde = "1.0"
                    rhs.trim_matches('"').trim_matches('\'').to_string()
                };

                deps.push(DepInfo { name, version: ver });
            }
        }
    }
    deps
}

fn parse_pip_deps(content: &str) -> Vec<DepInfo> {
    content
        .lines()
        .filter(|l| !l.trim().starts_with('#') && !l.trim().is_empty())
        .map(|line| {
            let (name, ver) = if let Some(idx) = line.find("==") {
                (&line[..idx], &line[idx + 2..])
            } else if let Some(idx) = line.find(">=") {
                (&line[..idx], &line[idx..])
            } else {
                (line, "")
            };
            DepInfo {
                name: name.trim().to_string(),
                version: ver.trim().to_string(),
            }
        })
        .filter(|d| !d.name.is_empty())
        .collect()
}

fn parse_go_deps(content: &str) -> Vec<DepInfo> {
    let mut deps = Vec::new();
    let mut in_require = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "require (" {
            in_require = true;
            continue;
        }
        if trimmed == ")" {
            in_require = false;
        }
        if in_require || trimmed.starts_with("require ") {
            let effective = trimmed.strip_prefix("require ").unwrap_or(trimmed);
            let parts: Vec<&str> = effective.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DepInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                });
            }
        }
    }
    deps
}

async fn run_security_scanner(
    kind: &str,
    manifest_path: &str,
    runtime: &AuditRuntime,
) -> (Vec<VulnInfo>, String, Option<String>) {
    let manifest_dir = Path::new(manifest_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or(".");

    match kind {
        "npm" => run_npm_audit(manifest_dir, runtime).await,
        "cargo" => run_cargo_audit(manifest_dir, runtime).await,
        "pip" | "pyproject" => run_pip_audit(manifest_dir, runtime).await,
        _ => (
            vec![],
            "scanner_unavailable".to_string(),
            Some(format!("No automated scanner for {}", kind)),
        ),
    }
}

async fn run_npm_audit(
    dir: &str,
    runtime: &AuditRuntime,
) -> (Vec<VulnInfo>, String, Option<String>) {
    let script = format!("cd {} && npm audit --json 2>/dev/null", shell_escape(dir));
    let args = vec!["-c".to_string(), script];
    match run_audit_command("bash", &args, 60, runtime, true).await {
        Ok(out) if !out.stdout.trim().is_empty() => {
            let vulns = parse_npm_audit_json(&out.stdout);
            (vulns, "scanner".to_string(), None)
        }
        Ok(_) => (
            vec![],
            "scanner".to_string(),
            Some("npm audit returned no output".to_string()),
        ),
        Err(e) => (vec![], "scanner_unavailable".to_string(), Some(e)),
    }
}

fn parse_npm_audit_json(json_str: &str) -> Vec<VulnInfo> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return vec![];
    };
    let mut vulns = Vec::new();
    // npm audit v7+ format: { vulnerabilities: { pkgName: { severity, via, fixAvailable } } }
    if let Some(obj) = json.get("vulnerabilities").and_then(|v| v.as_object()) {
        for (pkg, info) in obj {
            let severity = info
                .get("severity")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();
            let title = info
                .get("via")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.get("title").or(v.get("name")))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            let cve = info
                .get("via")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.get("cwe"))
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|c| c.as_str())
                .map(str::to_string);
            let fixed_in = if info
                .get("fixAvailable")
                .and_then(|f| f.as_bool())
                .unwrap_or(false)
            {
                Some("available".to_string())
            } else {
                None
            };
            vulns.push(VulnInfo {
                package: pkg.clone(),
                installed_version: info
                    .get("range")
                    .and_then(|r| r.as_str())
                    .unwrap_or("*")
                    .to_string(),
                severity,
                title,
                cve,
                fixed_in,
                advisory_url: None,
            });
        }
    }
    vulns
}

async fn run_cargo_audit(
    dir: &str,
    runtime: &AuditRuntime,
) -> (Vec<VulnInfo>, String, Option<String>) {
    let script = format!("cd {} && cargo audit --json 2>/dev/null", shell_escape(dir));
    let args = vec!["-c".to_string(), script];
    match run_audit_command("bash", &args, 120, runtime, true).await {
        Ok(out) if !out.stdout.trim().is_empty() => {
            let vulns = parse_cargo_audit_json(&out.stdout);
            (vulns, "scanner".to_string(), None)
        }
        Ok(_) => (
            vec![],
            "scanner_unavailable".to_string(),
            Some("cargo audit not installed or no vulnerabilities".to_string()),
        ),
        Err(e) => (vec![], "scanner_unavailable".to_string(), Some(e)),
    }
}

fn parse_cargo_audit_json(json_str: &str) -> Vec<VulnInfo> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return vec![];
    };
    let mut vulns = Vec::new();
    if let Some(vulns_arr) = json
        .get("vulnerabilities")
        .and_then(|v| v.get("list"))
        .and_then(|l| l.as_array())
    {
        for entry in vulns_arr {
            let advisory = entry.get("advisory").cloned().unwrap_or_default();
            let pkg = entry.get("package").cloned().unwrap_or_default();
            vulns.push(VulnInfo {
                package: pkg
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                installed_version: pkg
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                severity: advisory
                    .get("cvss")
                    .and_then(|c| c.get("score"))
                    .and_then(|s| s.as_f64())
                    .map(cvss_score_to_severity)
                    .unwrap_or_else(|| "unknown".to_string()),
                title: advisory
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                cve: advisory
                    .get("aliases")
                    .and_then(|a| a.as_array())
                    .and_then(|a| a.first())
                    .and_then(|c| c.as_str())
                    .map(str::to_string),
                fixed_in: advisory
                    .get("patched_versions")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                advisory_url: advisory
                    .get("url")
                    .and_then(|u| u.as_str())
                    .map(str::to_string),
            });
        }
    }
    vulns
}

fn cvss_score_to_severity(score: f64) -> String {
    if score >= 9.0 {
        "critical"
    } else if score >= 7.0 {
        "high"
    } else if score >= 4.0 {
        "medium"
    } else {
        "low"
    }
    .to_string()
}

async fn run_pip_audit(
    dir: &str,
    runtime: &AuditRuntime,
) -> (Vec<VulnInfo>, String, Option<String>) {
    let script = format!(
        "cd {} && pip-audit --format json 2>/dev/null",
        shell_escape(dir)
    );
    let args = vec!["-c".to_string(), script];
    match run_audit_command("bash", &args, 60, runtime, true).await {
        Ok(out) if !out.stdout.trim().is_empty() => {
            let vulns = parse_pip_audit_json(&out.stdout);
            (vulns, "scanner".to_string(), None)
        }
        Ok(_) => (
            vec![],
            "scanner_unavailable".to_string(),
            Some("pip-audit not installed or no vulnerabilities".to_string()),
        ),
        Err(e) => (vec![], "scanner_unavailable".to_string(), Some(e)),
    }
}

fn parse_pip_audit_json(json_str: &str) -> Vec<VulnInfo> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return vec![];
    };
    let mut vulns = Vec::new();
    if let Some(arr) = json.as_array() {
        for entry in arr {
            let pkg = entry
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let ver = entry
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            if let Some(vulns_arr) = entry.get("vulns").and_then(|v| v.as_array()) {
                for v in vulns_arr {
                    vulns.push(VulnInfo {
                        package: pkg.to_string(),
                        installed_version: ver.to_string(),
                        severity: "unknown".to_string(),
                        title: v
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("unknown")
                            .chars()
                            .take(120)
                            .collect(),
                        cve: v.get("id").and_then(|i| i.as_str()).map(str::to_string),
                        fixed_in: v
                            .get("fix_versions")
                            .and_then(|fv| fv.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        advisory_url: v.get("link").and_then(|l| l.as_str()).map(str::to_string),
                    });
                }
            }
        }
    }
    vulns
}

// ============================================================================
// CrossFileTaintTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CrossFileTaintArgs {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub file_glob: Option<String>,
    /// Source patterns (user-controlled input). Defaults to common HTTP input patterns.
    #[serde(default)]
    pub source_patterns: Option<Vec<String>>,
    /// Sink patterns (dangerous operations). Defaults to common injection sinks.
    #[serde(default)]
    pub sink_patterns: Option<Vec<String>>,
    /// Max cross-file traces to return. Default: 40
    #[serde(default = "default_cross_max")]
    pub max_traces: usize,
}

fn default_cross_max() -> usize {
    40
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossFilePoint {
    pub file: String,
    pub line: usize,
    pub text: String,
    pub function: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossFileTrace {
    pub source: CrossFilePoint,
    /// Intermediate call site connecting source file to sink file
    pub cross_call_site: Option<CrossFilePoint>,
    pub sink: CrossFilePoint,
    pub hops: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossFileTaintOutput {
    pub path: String,
    pub total_sources: usize,
    pub total_sinks: usize,
    pub cross_file_traces: Vec<CrossFileTrace>,
    /// Same-file traces count (handled by other methods)
    pub same_file_trace_count: usize,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CrossFileTaintError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("timeout after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, Clone)]
pub struct CrossFileTaintTool;

impl CrossFileTaintTool {
    pub const NAME: &'static str = "cross_file_taint";
    pub const DESCRIPTION: &'static str =
        "Find cross-file source-to-sink taint traces using function call heuristics. \
         Identifies user-controlled data (source) in one file flowing into dangerous \
         operations (sink) in another file via function call chains. Complements \
         other analysis methods which only cover same-file traces.";
}

impl Tool for CrossFileTaintTool {
    const NAME: &'static str = Self::NAME;
    type Args = CrossFileTaintArgs;
    type Output = CrossFileTaintOutput;
    type Error = CrossFileTaintError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CrossFileTaintArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let runtime = load_audit_runtime().await;
        let path = resolve_effective_path(args.path.as_deref(), &runtime.mode);
        let max_traces = args.max_traces.clamp(1, 200);
        let source_patterns = args
            .source_patterns
            .unwrap_or_else(default_cross_source_patterns);
        let sink_patterns = args
            .sink_patterns
            .unwrap_or_else(default_cross_sink_patterns);

        if source_patterns.is_empty() || sink_patterns.is_empty() {
            return Err(CrossFileTaintError::InvalidArgs(
                "source_patterns and sink_patterns must not be empty".to_string(),
            ));
        }

        // Step 1: collect all source + sink points
        let sources = collect_points(&path, args.file_glob.as_deref(), &source_patterns, &runtime)
            .await
            .map_err(|e| CrossFileTaintError::CommandFailed(e))?;
        let sinks = collect_points(&path, args.file_glob.as_deref(), &sink_patterns, &runtime)
            .await
            .map_err(|e| CrossFileTaintError::CommandFailed(e))?;

        // Step 2: group by file
        let mut sources_by_file: HashMap<String, Vec<&(String, usize, String)>> = HashMap::new();
        for pt in &sources {
            sources_by_file.entry(pt.0.clone()).or_default().push(pt);
        }
        let mut sinks_by_file: HashMap<String, Vec<&(String, usize, String)>> = HashMap::new();
        for pt in &sinks {
            sinks_by_file.entry(pt.0.clone()).or_default().push(pt);
        }

        let same_file_trace_count = sources_by_file
            .keys()
            .filter(|f| sinks_by_file.contains_key(*f))
            .count();

        // Step 3: extract tainted identifiers from source lines
        // and search for them in sink files
        let mut cross_traces: Vec<CrossFileTrace> = Vec::new();

        'outer: for (src_file, src_points) in &sources_by_file {
            for (sink_file, sink_points) in &sinks_by_file {
                if src_file == sink_file {
                    continue; // same-file handled separately
                }
                // Find if source function is called in sink file, or vice versa
                for src_pt in src_points {
                    // Extract identifier from source text (the param/var name after the dot)
                    let Some(ident) = extract_tainted_ident(&src_pt.2) else {
                        continue;
                    };
                    // Search for this identifier in the sink file
                    let search_pattern = format!(r"\b{}\b", regex::escape(&ident));
                    let rg_args = vec![
                        "--line-number".to_string(),
                        "--column".to_string(),
                        "--no-heading".to_string(),
                        "--color".to_string(),
                        "never".to_string(),
                        "--pcre2".to_string(),
                        search_pattern,
                        sink_file.clone(),
                    ];
                    let Ok(rg_out) = run_audit_command("rg", &rg_args, 15, &runtime, true).await
                    else {
                        continue;
                    };

                    for rg_line in rg_out.stdout.lines() {
                        let Some(hit) = parse_rg_line(rg_line) else {
                            continue;
                        };
                        // Check if this hit is near a sink point
                        for sink_pt in sink_points {
                            let dist = (hit.line as isize - sink_pt.1 as isize).unsigned_abs();
                            if dist <= 50 {
                                let confidence =
                                    ((50 - dist.min(50)) as f64 / 50.0 * 0.8 + 0.1).clamp(0.1, 0.9);
                                cross_traces.push(CrossFileTrace {
                                    source: CrossFilePoint {
                                        file: src_file.clone(),
                                        line: src_pt.1,
                                        text: src_pt.2.clone(),
                                        function: None,
                                    },
                                    cross_call_site: Some(CrossFilePoint {
                                        file: sink_file.clone(),
                                        line: hit.line,
                                        text: hit.text.clone(),
                                        function: None,
                                    }),
                                    sink: CrossFilePoint {
                                        file: sink_file.clone(),
                                        line: sink_pt.1,
                                        text: sink_pt.2.clone(),
                                        function: None,
                                    },
                                    hops: 2,
                                    confidence,
                                });
                                if cross_traces.len() >= max_traces {
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        cross_traces.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let truncated = cross_traces.len() >= max_traces;

        Ok(CrossFileTaintOutput {
            path,
            total_sources: sources.len(),
            total_sinks: sinks.len(),
            cross_file_traces: cross_traces,
            same_file_trace_count,
            truncated,
        })
    }
}

async fn collect_points(
    path: &str,
    file_glob: Option<&str>,
    patterns: &[String],
    runtime: &AuditRuntime,
) -> Result<Vec<(String, usize, String)>, String> {
    let mut points = Vec::new();
    for pattern in patterns {
        let mut args = vec![
            "--line-number".to_string(),
            "--column".to_string(),
            "--no-heading".to_string(),
            "--color".to_string(),
            "never".to_string(),
            "--pcre2".to_string(),
            pattern.clone(),
        ];
        if let Some(glob) = file_glob.filter(|v| !v.trim().is_empty()) {
            args.push("--glob".to_string());
            args.push(glob.to_string());
        }
        args.push(path.to_string());
        let out = run_audit_command("rg", &args, 20, runtime, true).await?;
        for line in out.stdout.lines() {
            if let Some(m) = parse_rg_line(line) {
                points.push((m.file, m.line, m.text));
            }
        }
    }
    Ok(points)
}

fn extract_tainted_ident(text: &str) -> Option<String> {
    // Extract last identifier segment after common input access patterns
    // e.g., "req.body.username" → "username"
    // e.g., "ctx.query.search" → "search"
    // e.g., "request.getParameter("foo")" → "foo"
    let text = text.trim();

    static RE_GET_PARAM: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r#"get\w*\(\s*["'](\w+)["']"#).unwrap());
    static RE_DOT_FIELD: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r"(?:req|request|ctx|context|params|query|body|input|args?)\.\w+\.(\w+)")
            .unwrap()
    });
    static RE_DOT_SINGLE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"(?:req|request|ctx|context)\.\w+\b").unwrap());

    // Named parameter in function call: getParameter("name"), get("key"), etc.
    if let Some(cap) = RE_GET_PARAM.captures(text) {
        if let Some(m) = cap.get(1) {
            return Some(m.as_str().to_string());
        }
    }

    // Dot notation: req.body.fieldName → fieldName
    if let Some(cap) = RE_DOT_FIELD.captures(text) {
        if let Some(m) = cap.get(1) {
            let ident = m.as_str();
            // Avoid overly generic names
            if ident.len() > 2 && !matches!(ident, "get" | "set" | "map" | "has" | "len") {
                return Some(ident.to_string());
            }
        }
    }

    // req.params.id or similar single level
    if let Some(cap) = RE_DOT_SINGLE.captures(text) {
        let matched = cap.get(0)?.as_str();
        let parts: Vec<&str> = matched.splitn(3, '.').collect();
        if parts.len() >= 2 {
            let ident = parts.last().unwrap_or(&"");
            if ident.len() > 2 {
                return Some(ident.to_string());
            }
        }
    }

    None
}

fn default_cross_source_patterns() -> Vec<String> {
    vec![
        // JavaScript/TypeScript (Express, Koa)
        r"req\.params\.\w+".to_string(),
        r"req\.query\.\w+".to_string(),
        r"req\.body\.\w+".to_string(),
        r"ctx\.query\.\w+".to_string(),
        r"ctx\.params\.\w+".to_string(),
        r"ctx\.request\.body".to_string(),
        // Java (Servlet, Spring)
        r"request\.getParameter\(".to_string(),
        r"request\.getHeader\(".to_string(),
        r"@RequestParam".to_string(),
        r"@PathVariable".to_string(),
        r"@RequestBody".to_string(),
        // Python (Flask, Django)
        r"request\.form\[".to_string(),
        r"request\.args\.get\(".to_string(),
        r"request\.json".to_string(),
        r"request\.data".to_string(),
        r"request\.GET\.get".to_string(),
        r"request\.POST\.get".to_string(),
        // PHP
        r"\$_GET\[".to_string(),
        r"\$_POST\[".to_string(),
        r"\$_REQUEST\[".to_string(),
        r"\$_COOKIE\[".to_string(),
        r"\$_SERVER\[".to_string(),
        // Go (net/http, Gin)
        r"c\.Param\(".to_string(),
        r"c\.Query\(".to_string(),
        r"c\.PostForm\(".to_string(),
        r"r\.FormValue\(".to_string(),
        r"r\.URL\.Query".to_string(),
        // Ruby (Rails)
        r"params\[:\w+\]".to_string(),
        // Rust (Actix, Axum)
        r"web::Query".to_string(),
        r"web::Json".to_string(),
        r"web::Path".to_string(),
        // C/C++ input functions
        r"gets\(".to_string(),
        r"scanf\(".to_string(),
        r"fgets\(".to_string(),
        r"getenv\(".to_string(),
        r"argv".to_string(),
    ]
}

fn default_cross_sink_patterns() -> Vec<String> {
    vec![
        // SQL / Database
        r"execute\s*\(".to_string(),
        r"query\s*\(".to_string(),
        r"exec\s*\(".to_string(),
        r"rawQuery\s*\(".to_string(),
        r"db\.run\(".to_string(),
        r"\.raw\s*\(".to_string(),
        r"createConnection\(".to_string(),
        r"cursor\.execute".to_string(),
        r"\.exec\(".to_string(),
        // Eval / Code Injection
        r"eval\s*\(".to_string(),
        r"Function\s*\(".to_string(),
        // XSS
        r"innerHTML\s*=".to_string(),
        r"outerHTML\s*=".to_string(),
        r"document\.write".to_string(),
        r"dangerouslySetInnerHTML".to_string(),
        r"render_template_string".to_string(),
        // Command Injection
        r"subprocess\.".to_string(),
        r"os\.system\(".to_string(),
        r"os\.popen\(".to_string(),
        r"child_process".to_string(),
        r"Command::new".to_string(),
        r"exec\.Command".to_string(),
        r"system\(".to_string(),
        r"shell_exec\(".to_string(),
        r"Runtime\.exec".to_string(),
        // File system
        r"open\s*\(".to_string(),
        r"readFile\(".to_string(),
        r"writeFile\(".to_string(),
        r"fopen\(".to_string(),
        r"std::fs".to_string(),
        // SSRF
        r"fetch\(".to_string(),
        r"requests\.get".to_string(),
        r"http\.Get".to_string(),
        r"curl_exec".to_string(),
        // Deserialization
        r"pickle\.loads".to_string(),
        r"yaml\.load\(".to_string(),
        r"unserialize\(".to_string(),
        // Redirect
        r"redirect\(".to_string(),
        r"sendRedirect".to_string(),
        // C/C++ memory-unsafe
        r"strcpy\(".to_string(),
        r"strcat\(".to_string(),
        r"sprintf\(".to_string(),
        r"memcpy\(".to_string(),
    ]
}

// ============================================================================
// AuditReportTool
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AuditReportArgs {
    /// Report title. Default: "Security Audit Report"
    #[serde(default)]
    pub title: Option<String>,
    /// Output format: "markdown" | "sarif". Default: "markdown"
    #[serde(default)]
    pub format: Option<String>,
    /// Target system or repository name
    #[serde(default)]
    pub target: Option<String>,
    /// Findings to include (same schema as audit_finding_upsert)
    pub findings: Vec<serde_json::Value>,
    /// Auditor name or team name
    #[serde(default)]
    pub auditor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReportOutput {
    pub format: String,
    pub title: String,
    pub report: String,
    pub finding_count: usize,
    pub severity_summary: HashMap<String, usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditReportError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
}

#[derive(Debug, Clone)]
pub struct AuditReportTool;

impl AuditReportTool {
    pub const NAME: &'static str = "audit_report";
    pub const DESCRIPTION: &'static str =
        "Export audit findings to a formatted report (Markdown or SARIF 2.1). \
         Pass the findings array to generate a structured security report suitable \
         for sharing with developers or ingesting into CI/CD tooling.";
}

impl Tool for AuditReportTool {
    const NAME: &'static str = Self::NAME;
    type Args = AuditReportArgs;
    type Output = AuditReportOutput;
    type Error = AuditReportError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AuditReportArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.findings.is_empty() {
            return Err(AuditReportError::InvalidArgs(
                "findings must not be empty".to_string(),
            ));
        }
        let format = args.format.as_deref().unwrap_or("markdown").to_lowercase();
        let title = args
            .title
            .as_deref()
            .unwrap_or("Security Audit Report")
            .to_string();
        let target = args.target.as_deref().unwrap_or("Unknown Target");
        let auditor = args.auditor.as_deref().unwrap_or("Sentinel AI");

        let mut severity_summary: HashMap<String, usize> = HashMap::new();
        for f in &args.findings {
            let sev = f
                .get("severity")
                .and_then(|s| s.as_str())
                .unwrap_or("info")
                .to_lowercase();
            *severity_summary.entry(sev).or_insert(0) += 1;
        }

        let report = match format.as_str() {
            "sarif" => render_sarif(&title, &args.findings),
            _ => render_markdown(&title, target, auditor, &args.findings, &severity_summary),
        };

        Ok(AuditReportOutput {
            format,
            title,
            report,
            finding_count: args.findings.len(),
            severity_summary,
        })
    }
}

fn render_markdown(
    title: &str,
    target: &str,
    auditor: &str,
    findings: &[serde_json::Value],
    severity_summary: &HashMap<String, usize>,
) -> String {
    use chrono::Utc;
    let mut out = String::new();
    let date = Utc::now().format("%Y-%m-%d").to_string();

    out.push_str(&format!("# {}\n\n", title));
    out.push_str(&format!("**Target:** {}\n", target));
    out.push_str(&format!("**Auditor:** {}\n", auditor));
    out.push_str(&format!("**Date:** {}\n\n", date));

    // Summary table
    out.push_str("## Summary\n\n");
    out.push_str("| Severity | Count |\n|---|---|\n");
    for sev in &["critical", "high", "medium", "low", "info"] {
        let count = severity_summary.get(*sev).copied().unwrap_or(0);
        if count > 0 {
            out.push_str(&format!("| {} | {} |\n", sev.to_uppercase(), count));
        }
    }
    out.push_str(&format!("| **Total** | **{}** |\n\n", findings.len()));

    // Quality gate
    let total = findings.len() as f64;
    let with_evidence_count = findings
        .iter()
        .filter(|f| {
            f.get("evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .any(|v| v.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false))
                })
                .unwrap_or(false)
        })
        .count() as f64;
    let uncertain_count = findings
        .iter()
        .filter(|f| {
            let verification = f
                .get("verification_status")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let lifecycle = f
                .get("lifecycle_stage")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            verification == "needs_more_evidence"
                || lifecycle == "candidate"
                || lifecycle == "triaged"
                || lifecycle == "verified"
        })
        .count() as f64;
    let false_positive_count = findings
        .iter()
        .filter(|f| {
            let status = f.get("status").and_then(|v| v.as_str()).unwrap_or_default();
            let lifecycle = f
                .get("lifecycle_stage")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            status == "false_positive" || lifecycle == "rejected"
        })
        .count() as f64;

    let evidence_rate = if total > 0.0 {
        with_evidence_count / total
    } else {
        0.0
    };
    let uncertain_rate = if total > 0.0 {
        uncertain_count / total
    } else {
        0.0
    };
    let false_positive_rate = if total > 0.0 {
        false_positive_count / total
    } else {
        0.0
    };

    let min_evidence_rate = 0.70;
    let max_uncertain_rate = 0.30;
    let max_false_positive_rate = 0.20;
    let gate_passed = total == 0.0
        || (evidence_rate >= min_evidence_rate
            && uncertain_rate <= max_uncertain_rate
            && false_positive_rate <= max_false_positive_rate);

    out.push_str("## Quality Gate\n\n");
    out.push_str(&format!(
        "- Gate Status: **{}**\n",
        if gate_passed { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!(
        "- Evidence Rate: **{:.1}%** (threshold: >= {:.1}%)\n",
        evidence_rate * 100.0,
        min_evidence_rate * 100.0
    ));
    out.push_str(&format!(
        "- Uncertain Rate: **{:.1}%** (threshold: <= {:.1}%)\n",
        uncertain_rate * 100.0,
        max_uncertain_rate * 100.0
    ));
    out.push_str(&format!(
        "- False Positive/Rejection Rate: **{:.1}%** (threshold: <= {:.1}%)\n\n",
        false_positive_rate * 100.0,
        max_false_positive_rate * 100.0
    ));

    // Findings
    out.push_str("## Findings\n\n");
    for (i, f) in findings.iter().enumerate() {
        let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let title_f = f
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let severity = f.get("severity").and_then(|v| v.as_str()).unwrap_or("info");
        let cwe = f.get("cwe").and_then(|v| v.as_str()).unwrap_or("-");
        let confidence = f
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|c| format!("{:.0}%", c * 100.0))
            .unwrap_or_else(|| "-".to_string());
        let description = f.get("description").and_then(|v| v.as_str()).unwrap_or("-");
        let fix = f.get("fix").and_then(|v| v.as_str()).unwrap_or("-");

        out.push_str(&format!(
            "### {index}. [{id}] {title}\n\n",
            index = i + 1,
            id = id,
            title = title_f
        ));
        out.push_str(&format!(
            "| Field | Value |\n|---|---|\n\
             | Severity | `{severity}` |\n\
             | CWE | {cwe} |\n\
             | Confidence | {confidence} |\n\n",
        ));
        out.push_str(&format!("**Description**\n\n{}\n\n", description));
        out.push_str(&format!("**Remediation**\n\n{}\n\n", fix));

        if let Some(files) = f.get("files").and_then(|v| v.as_array()) {
            if !files.is_empty() {
                out.push_str("**Affected Files**\n\n");
                for file in files {
                    if let Some(s) = file.as_str() {
                        out.push_str(&format!("- `{}`\n", s));
                    }
                }
                out.push('\n');
            }
        }

        if let Some(evidence) = f.get("evidence").and_then(|v| v.as_array()) {
            if !evidence.is_empty() {
                out.push_str("**Evidence**\n\n```\n");
                for e in evidence {
                    if let Some(s) = e.as_str() {
                        out.push_str(s);
                        out.push('\n');
                    }
                }
                out.push_str("```\n\n");
            }
        }

        out.push_str("---\n\n");
    }

    out
}

fn render_sarif(title: &str, findings: &[serde_json::Value]) -> String {
    let rules: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("AUDIT-001");
            let name = f
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Security Finding");
            let cwe = f.get("cwe").and_then(|v| v.as_str()).unwrap_or("");
            serde_json::json!({
                "id": id,
                "name": name,
                "shortDescription": { "text": name },
                "help": {
                    "text": f.get("fix").and_then(|v| v.as_str()).unwrap_or("See description")
                },
                "properties": {
                    "tags": if cwe.is_empty() { vec![] } else { vec![cwe] }
                }
            })
        })
        .collect();

    let results: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            let rule_id = f.get("id").and_then(|v| v.as_str()).unwrap_or("AUDIT-001");
            let severity = f
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("warning");
            let sarif_level = match severity.to_lowercase().as_str() {
                "critical" | "high" => "error",
                "medium" => "warning",
                _ => "note",
            };
            let message = f
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Security finding detected");

            let locations: Vec<serde_json::Value> = f
                .get("files")
                .and_then(|v| v.as_array())
                .map(|files| {
                    files
                        .iter()
                        .filter_map(|file| file.as_str())
                        .map(|path| {
                            let (file_path, line) = if let Some(idx) = path.rfind(':') {
                                let (p, l) = path.split_at(idx);
                                let line = l.trim_start_matches(':').parse::<u64>().unwrap_or(1);
                                (p, line)
                            } else {
                                (path, 1u64)
                            };
                            serde_json::json!({
                                "physicalLocation": {
                                    "artifactLocation": { "uri": file_path },
                                    "region": { "startLine": line }
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            serde_json::json!({
                "ruleId": rule_id,
                "level": sarif_level,
                "message": { "text": message },
                "locations": locations
            })
        })
        .collect();

    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Sentinel AI",
                    "informationUri": "https://sentinel-ai.dev",
                    "rules": rules
                }
            },
            "results": results,
            "properties": { "reportTitle": title }
        }]
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}
