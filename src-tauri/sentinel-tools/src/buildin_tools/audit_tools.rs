use crate::buildin_tools::shell::{get_shell_config, ShellExecutionMode};
use crate::docker_sandbox::{DockerSandbox, DockerSandboxConfig};
use once_cell::sync::Lazy;
use regex::Regex;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;

// ── Cached regex patterns ───────────────────────────────────────────────────

static RE_HUNK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@").unwrap());

static RE_FN_RUST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:pub(?:\(\w+\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap()
});

static RE_FN_JS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:export\s+)?(?:async\s+)?function\s*\*?\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap()
});

static RE_FN_PY_RB: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap());

static RE_FN_GO: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"func\s+(?:\([^)]*\)\s+)?([A-Za-z_][A-Za-z0-9_]*)").unwrap());

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CodeSearchArgs {
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub file_glob: Option<String>,
    #[serde(default = "default_code_search_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub case_sensitive: bool,
}

fn default_code_search_max_results() -> usize {
    100
}

const CODE_SEARCH_TIMEOUT_SECS: u64 = 60;
const CODE_SEARCH_MAX_FILESIZE: &str = "2M";
const CODE_SEARCH_EXCLUDE_GLOBS: &[&str] = &[
    "!**/.git/**",
    "!**/node_modules/**",
    "!**/target/**",
    "!**/dist/**",
    "!**/build/**",
    "!**/.next/**",
    "!**/coverage/**",
];

#[derive(Debug, Clone, Serialize)]
pub struct CodeSearchMatch {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeSearchOutput {
    pub pattern: String,
    pub path: String,
    pub total_matches: usize,
    pub truncated: bool,
    pub matches: Vec<CodeSearchMatch>,
}

#[derive(Debug, thiserror::Error)]
pub enum CodeSearchError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("tool timeout after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, Clone)]
pub struct CodeSearchTool;

impl CodeSearchTool {
    pub const NAME: &'static str = "code_search";
    pub const DESCRIPTION: &'static str = "Search code with ripgrep and return structured file/line/column matches for audit evidence.";
}

impl Tool for CodeSearchTool {
    const NAME: &'static str = Self::NAME;
    type Args = CodeSearchArgs;
    type Output = CodeSearchOutput;
    type Error = CodeSearchError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CodeSearchArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.pattern.trim().is_empty() {
            return Err(CodeSearchError::InvalidArgs(
                "pattern is required".to_string(),
            ));
        }

        let runtime = load_audit_runtime().await;
        let path = resolve_effective_path(args.path.as_deref(), &runtime.mode);
        let max_results = args.max_results.clamp(1, 500);

        let mut command_args = vec![
            "--line-number".to_string(),
            "--column".to_string(),
            "--no-heading".to_string(),
            "--color".to_string(),
            "never".to_string(),
            "--max-filesize".to_string(),
            CODE_SEARCH_MAX_FILESIZE.to_string(),
        ];

        if args.case_sensitive {
            command_args.push("--case-sensitive".to_string());
        } else {
            command_args.push("--smart-case".to_string());
        }

        if let Some(file_glob) = args.file_glob.as_ref().filter(|v| !v.trim().is_empty()) {
            command_args.push("--glob".to_string());
            command_args.push(file_glob.clone());
        }

        for exclude_glob in CODE_SEARCH_EXCLUDE_GLOBS {
            command_args.push("--glob".to_string());
            command_args.push((*exclude_glob).to_string());
        }

        command_args.push(args.pattern.clone());
        command_args.push(path.clone());

        let output =
            run_command_for_code_search("rg", &command_args, CODE_SEARCH_TIMEOUT_SECS, &runtime)
                .await?;

        let mut matches = Vec::new();
        for line in output.stdout.lines() {
            if let Some(item) = parse_rg_line(line) {
                matches.push(item);
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        let total_matches = output
            .stdout
            .lines()
            .filter(|line| parse_rg_line(line).is_some())
            .count();
        let truncated = total_matches > matches.len();

        Ok(CodeSearchOutput {
            pattern: args.pattern,
            path,
            total_matches,
            truncated,
            matches,
        })
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GitDiffScopeArgs {
    #[serde(default)]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub target_ref: Option<String>,
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    #[serde(default = "default_git_diff_max_files")]
    pub max_files: usize,
}

fn default_git_diff_max_files() -> usize {
    200
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitDiffFile {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitDiffScopeOutput {
    pub repo_path: String,
    pub base_ref: String,
    pub target_ref: String,
    pub total_files: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub truncated: bool,
    pub files: Vec<GitDiffFile>,
}

#[derive(Debug, thiserror::Error)]
pub enum GitDiffScopeError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("tool timeout after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, Clone)]
pub struct GitDiffScopeTool;

impl GitDiffScopeTool {
    pub const NAME: &'static str = "git_diff_scope";
    pub const DESCRIPTION: &'static str = "Collect structured git diff scope (changed files, line stats, and hunk ranges) for code audit planning.";
}

impl Tool for GitDiffScopeTool {
    const NAME: &'static str = Self::NAME;
    type Args = GitDiffScopeArgs;
    type Output = GitDiffScopeOutput;
    type Error = GitDiffScopeError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GitDiffScopeArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let runtime = load_audit_runtime().await;
        let repo_path = resolve_effective_path(args.repo_path.as_deref(), &runtime.mode);
        let base_ref = args.base_ref.unwrap_or_else(|| "HEAD~1".to_string());
        let target_ref = args.target_ref.unwrap_or_else(|| "HEAD".to_string());
        let max_files = args.max_files.clamp(1, 1000);

        if base_ref.trim().is_empty() || target_ref.trim().is_empty() {
            return Err(GitDiffScopeError::InvalidArgs(
                "base_ref and target_ref must not be empty".to_string(),
            ));
        }

        let mut numstat_args = vec![
            "-C".to_string(),
            repo_path.clone(),
            "diff".to_string(),
            "--numstat".to_string(),
            "--no-color".to_string(),
            base_ref.clone(),
            target_ref.clone(),
        ];

        if let Some(paths) = args.paths.as_ref().filter(|items| !items.is_empty()) {
            numstat_args.push("--".to_string());
            for path in paths {
                numstat_args.push(path.clone());
            }
        }

        let numstat = run_command_for_git("git", &numstat_args, 20, &runtime).await?;

        let mut hunk_args = vec![
            "-C".to_string(),
            repo_path.clone(),
            "diff".to_string(),
            "--no-color".to_string(),
            "--unified=0".to_string(),
            base_ref.clone(),
            target_ref.clone(),
        ];

        if let Some(paths) = args.paths.as_ref().filter(|items| !items.is_empty()) {
            hunk_args.push("--".to_string());
            for path in paths {
                hunk_args.push(path.clone());
            }
        }

        let hunk_output = run_command_for_git("git", &hunk_args, 20, &runtime).await?;

        let file_stats = parse_numstat(&numstat.stdout);
        let hunk_map = parse_hunks(&hunk_output.stdout);

        let total_files = file_stats.len();
        let mut total_additions = 0usize;
        let mut total_deletions = 0usize;

        let mut files = Vec::new();
        for (path, (adds, dels)) in file_stats.iter().take(max_files) {
            total_additions += *adds;
            total_deletions += *dels;
            files.push(GitDiffFile {
                path: path.clone(),
                additions: *adds,
                deletions: *dels,
                hunks: hunk_map.get(path).cloned().unwrap_or_default(),
            });
        }

        let truncated = total_files > files.len();

        Ok(GitDiffScopeOutput {
            repo_path,
            base_ref,
            target_ref,
            total_files,
            total_additions,
            total_deletions,
            truncated,
            files,
        })
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GitCloneRepoArgs {
    pub repo_url: String,
    #[serde(default)]
    pub destination_parent: Option<String>,
    #[serde(default)]
    pub destination_name: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub depth: Option<u32>,
    #[serde(default)]
    pub refresh_if_exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitCloneRepoOutput {
    pub repo_url: String,
    pub local_path: String,
    pub cloned: bool,
    pub refreshed: bool,
    pub branch: Option<String>,
    pub head_commit: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GitCloneRepoError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("tool timeout after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, Clone)]
pub struct GitCloneRepoTool;

impl GitCloneRepoTool {
    pub const NAME: &'static str = "git_clone_repo";
    pub const DESCRIPTION: &'static str = "Clone a remote git repository to local workspace. Use this before audit tools when user provides only a repository URL without local path.";
}

impl Tool for GitCloneRepoTool {
    const NAME: &'static str = Self::NAME;
    type Args = GitCloneRepoArgs;
    type Output = GitCloneRepoOutput;
    type Error = GitCloneRepoError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GitCloneRepoArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo_url = args.repo_url.trim().to_string();
        if repo_url.is_empty() {
            return Err(GitCloneRepoError::InvalidArgs(
                "repo_url is required".to_string(),
            ));
        }

        let runtime = load_audit_runtime().await;
        let parent = resolve_clone_parent_path(args.destination_parent.as_deref(), &runtime.mode);
        ensure_clone_parent_exists(&parent, &runtime).await?;

        let repo_name = args
            .destination_name
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| infer_repo_name_from_url(&repo_url));
        let local_path = join_path_for_mode(&parent, &repo_name, &runtime.mode);

        let existing = is_git_repo(&local_path, &runtime).await;
        let mut cloned = false;
        let mut refreshed = false;

        if existing {
            if args.refresh_if_exists {
                let fetch_args = vec![
                    "-C".to_string(),
                    local_path.clone(),
                    "fetch".to_string(),
                    "--all".to_string(),
                    "--prune".to_string(),
                ];
                run_command_for_git_clone("git", &fetch_args, 90, &runtime).await?;

                if let Some(branch) = args.branch.as_ref().filter(|v| !v.trim().is_empty()) {
                    let checkout_args = vec![
                        "-C".to_string(),
                        local_path.clone(),
                        "checkout".to_string(),
                        branch.trim().to_string(),
                    ];
                    run_command_for_git_clone("git", &checkout_args, 60, &runtime).await?;

                    let pull_args = vec![
                        "-C".to_string(),
                        local_path.clone(),
                        "pull".to_string(),
                        "--ff-only".to_string(),
                        "origin".to_string(),
                        branch.trim().to_string(),
                    ];
                    run_command_for_git_clone("git", &pull_args, 120, &runtime).await?;
                }
                refreshed = true;
            }
        } else {
            let mut clone_args = vec!["clone".to_string()];
            if let Some(depth) = args.depth.filter(|v| *v > 0) {
                clone_args.push("--depth".to_string());
                clone_args.push(depth.to_string());
            }
            if let Some(branch) = args.branch.as_ref().filter(|v| !v.trim().is_empty()) {
                clone_args.push("--branch".to_string());
                clone_args.push(branch.trim().to_string());
            }
            clone_args.push(repo_url.clone());
            clone_args.push(local_path.clone());
            run_command_for_git_clone("git", &clone_args, 300, &runtime).await?;
            cloned = true;
        }

        let head_commit = get_repo_head_commit(&local_path, &runtime).await.ok();

        Ok(GitCloneRepoOutput {
            repo_url,
            local_path,
            cloned,
            refreshed,
            branch: args.branch,
            head_commit,
        })
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CallGraphLiteArgs {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub file_glob: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default = "default_call_graph_max_nodes")]
    pub max_nodes: usize,
}

fn default_call_graph_max_nodes() -> usize {
    120
}

#[derive(Debug, Clone, Serialize)]
pub struct CallGraphNode {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallGraphEdge {
    pub caller: String,
    pub callee: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallGraphLiteOutput {
    pub path: String,
    pub focused_symbol: Option<String>,
    pub nodes: Vec<CallGraphNode>,
    pub edges: Vec<CallGraphEdge>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CallGraphLiteError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("tool timeout after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, Clone)]
pub struct CallGraphLiteTool;

impl CallGraphLiteTool {
    pub const NAME: &'static str = "call_graph_lite";
    pub const DESCRIPTION: &'static str = "Build a lightweight call graph from function definitions and callsite heuristics for audit triage.";
}

impl Tool for CallGraphLiteTool {
    const NAME: &'static str = Self::NAME;
    type Args = CallGraphLiteArgs;
    type Output = CallGraphLiteOutput;
    type Error = CallGraphLiteError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CallGraphLiteArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let runtime = load_audit_runtime().await;
        let path = resolve_effective_path(args.path.as_deref(), &runtime.mode);
        let max_nodes = args.max_nodes.clamp(10, 500);
        // Match function definitions across multiple languages:
        // Rust: pub fn, async fn, pub(crate) fn, etc.
        // JS/TS: function, async function, export function
        // Python: def, async def
        // Go: func
        // Ruby: def
        // Java/C#/C++: public void foo, private static int bar, etc.
        let def_pattern = r"^\s*(?:(?:pub(?:\(\w+\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)|(?:export\s+)?(?:async\s+)?function\s*\*?\s+([A-Za-z_][A-Za-z0-9_]*)|(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)|func\s+(?:\([^)]*\)\s+)?([A-Za-z_][A-Za-z0-9_]*)|(?:(?:public|private|protected|internal|static|final|abstract|override|virtual|synchronized)\s+)*(?:[A-Za-z_][A-Za-z0-9_<>\[\]]*\s+)([A-Za-z_][A-Za-z0-9_]*)\s*\()";

        let mut defs_args = vec![
            "--line-number".to_string(),
            "--column".to_string(),
            "--no-heading".to_string(),
            "--color".to_string(),
            "never".to_string(),
            "--pcre2".to_string(),
            def_pattern.to_string(),
        ];
        if let Some(file_glob) = args.file_glob.as_ref().filter(|v| !v.trim().is_empty()) {
            defs_args.push("--glob".to_string());
            defs_args.push(file_glob.clone());
        }
        defs_args.push(path.clone());

        let defs_out = run_command_for_call_graph("rg", &defs_args, 20, &runtime).await?;
        let mut definitions = Vec::new();
        for line in defs_out.stdout.lines() {
            if let Some(parsed) = parse_rg_line(line) {
                if let Some(name) = extract_function_name(&parsed.text) {
                    definitions.push((name, parsed.file, parsed.line));
                }
            }
        }

        if definitions.is_empty() {
            return Ok(CallGraphLiteOutput {
                path,
                focused_symbol: args.symbol,
                nodes: vec![],
                edges: vec![],
                truncated: false,
            });
        }

        let mut focus_names = Vec::new();
        if let Some(symbol) = args.symbol.as_ref().filter(|v| !v.trim().is_empty()) {
            focus_names.push(symbol.trim().to_string());
        } else {
            for (name, _, _) in definitions.iter().take(25) {
                focus_names.push(name.clone());
            }
        }
        focus_names.sort();
        focus_names.dedup();

        let escaped_names: Vec<String> =
            focus_names.iter().map(|name| regex::escape(name)).collect();
        let call_pattern = format!(r"\b({})\s*\(", escaped_names.join("|"));
        let mut calls_args = vec![
            "--line-number".to_string(),
            "--column".to_string(),
            "--no-heading".to_string(),
            "--color".to_string(),
            "never".to_string(),
            "--pcre2".to_string(),
            call_pattern,
        ];
        if let Some(file_glob) = args.file_glob.as_ref().filter(|v| !v.trim().is_empty()) {
            calls_args.push("--glob".to_string());
            calls_args.push(file_glob.clone());
        }
        calls_args.push(path.clone());

        let calls_out = run_command_for_call_graph("rg", &calls_args, 20, &runtime).await?;

        let mut defs_by_file: HashMap<String, Vec<(String, usize)>> = HashMap::new();
        for (name, file, line) in &definitions {
            defs_by_file
                .entry(file.clone())
                .or_default()
                .push((name.clone(), *line));
        }
        for defs in defs_by_file.values_mut() {
            defs.sort_by_key(|(_, line)| *line);
        }

        let mut node_map: HashMap<String, CallGraphNode> = HashMap::new();
        for (name, file, line) in definitions.iter().take(max_nodes) {
            let id = format!("{}::{}", file, name);
            node_map.insert(
                id.clone(),
                CallGraphNode {
                    id,
                    name: name.clone(),
                    file: file.clone(),
                    line: *line,
                },
            );
        }

        let mut edge_map: HashMap<String, CallGraphEdge> = HashMap::new();
        for line in calls_out.stdout.lines() {
            let Some(parsed) = parse_rg_line(line) else {
                continue;
            };
            let Some(callee) = extract_called_name(&parsed.text, &focus_names) else {
                continue;
            };
            let Some(caller) = nearest_function_for_line(&defs_by_file, &parsed.file, parsed.line)
            else {
                continue;
            };
            if caller == callee {
                continue;
            }
            let key = format!("{}:{}:{}:{}", caller, callee, parsed.file, parsed.line);
            edge_map.entry(key).or_insert(CallGraphEdge {
                caller,
                callee,
                file: parsed.file,
                line: parsed.line,
            });
        }

        let mut nodes: Vec<CallGraphNode> = node_map.into_values().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let mut edges: Vec<CallGraphEdge> = edge_map.into_values().collect();
        edges.sort_by(|a, b| {
            a.caller
                .cmp(&b.caller)
                .then_with(|| a.callee.cmp(&b.callee))
                .then_with(|| a.file.cmp(&b.file))
                .then_with(|| a.line.cmp(&b.line))
        });

        Ok(CallGraphLiteOutput {
            path,
            focused_symbol: args.symbol,
            nodes,
            edges,
            truncated: definitions.len() > max_nodes,
        })
    }
}

pub(crate) fn resolve_effective_path(input: Option<&str>, mode: &ShellExecutionMode) -> String {
    if *mode == ShellExecutionMode::Docker {
        let default_path = "/workspace".to_string();
        let Some(raw) = input else {
            return default_path;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return default_path;
        }
        return trimmed.to_string();
    }

    let fallback = ".".to_string();
    let Some(raw) = input else {
        return fallback;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback;
    }

    let requested = PathBuf::from(trimmed);
    if requested.exists() {
        return trimmed.to_string();
    }

    // LLM often uses absolute "/workspace/xxx" paths copied from prompt context.
    // If runtime cwd already points to that workspace, remap to relative path.
    if let Ok(stripped) = requested.strip_prefix("/workspace") {
        let remapped = Path::new(".").join(stripped);
        if remapped.exists() {
            return remapped.to_string_lossy().to_string();
        }
    }

    fallback
}

fn resolve_clone_parent_path(input: Option<&str>, mode: &ShellExecutionMode) -> String {
    if *mode == ShellExecutionMode::Docker {
        let fallback = "/workspace".to_string();
        return input
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback);
    }

    input
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn infer_repo_name_from_url(repo_url: &str) -> String {
    let cleaned = repo_url.trim().trim_end_matches('/');
    let last = cleaned.rsplit(&['/', ':'][..]).next().unwrap_or("repo");
    let normalized = last.trim_end_matches(".git").trim();
    if normalized.is_empty() {
        "repo".to_string()
    } else {
        normalized.to_string()
    }
}

fn join_path_for_mode(parent: &str, name: &str, mode: &ShellExecutionMode) -> String {
    if *mode == ShellExecutionMode::Docker {
        let base = parent.trim_end_matches('/');
        if base.is_empty() {
            format!("/{}", name)
        } else {
            format!("{}/{}", base, name)
        }
    } else {
        Path::new(parent).join(name).to_string_lossy().to_string()
    }
}

async fn ensure_clone_parent_exists(
    parent: &str,
    runtime: &AuditRuntime,
) -> Result<(), GitCloneRepoError> {
    match runtime.mode {
        ShellExecutionMode::Docker => {
            let args = vec!["-p".to_string(), parent.to_string()];
            run_command_for_git_clone("mkdir", &args, 20, runtime).await?;
            Ok(())
        }
        _ => fs::create_dir_all(parent).await.map_err(|e| {
            GitCloneRepoError::CommandFailed(format!("failed to create parent dir: {}", e))
        }),
    }
}

async fn is_git_repo(path: &str, runtime: &AuditRuntime) -> bool {
    let args = vec![
        "-C".to_string(),
        path.to_string(),
        "rev-parse".to_string(),
        "--is-inside-work-tree".to_string(),
    ];
    run_audit_command("git", &args, 20, runtime, false)
        .await
        .is_ok()
}

async fn get_repo_head_commit(
    path: &str,
    runtime: &AuditRuntime,
) -> Result<String, GitCloneRepoError> {
    let args = vec![
        "-C".to_string(),
        path.to_string(),
        "rev-parse".to_string(),
        "HEAD".to_string(),
    ];
    let output = run_command_for_git_clone("git", &args, 20, runtime).await?;
    Ok(output.stdout.trim().to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct AuditRuntime {
    pub(crate) mode: ShellExecutionMode,
    pub(crate) docker_config: Option<DockerSandboxConfig>,
}

pub(crate) async fn load_audit_runtime() -> AuditRuntime {
    let shell_cfg = get_shell_config().await;
    AuditRuntime {
        mode: shell_cfg.default_execution_mode.clone(),
        docker_config: shell_cfg.docker_config.clone(),
    }
}

pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
}

pub(crate) fn shell_escape(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", arg.replace('\'', "'\"'\"'"))
}

pub(crate) fn build_shell_command(program: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_escape(program));
    for arg in args {
        parts.push(shell_escape(arg));
    }
    parts.join(" ")
}

pub(crate) async fn run_audit_command(
    program: &str,
    args: &[String],
    timeout_secs: u64,
    runtime: &AuditRuntime,
    allow_exit_code_1: bool,
) -> Result<CommandOutput, String> {
    match runtime.mode {
        ShellExecutionMode::Docker => {
            let docker_cfg = runtime.docker_config.clone().unwrap_or_default();
            // Non-interactive `bash -c` may miss user-level PATH entries (e.g. cargo bin).
            // Prepend common binary dirs and fail early with clear error when tool is unavailable.
            let path_prefix = "export PATH=\"$PATH:/usr/local/bin:/usr/bin:/bin:/usr/local/cargo/bin:/root/.cargo/bin\";";
            let precheck = format!("command -v {} >/dev/null 2>&1", shell_escape(program));
            let cmd = format!(
                "{} {} || {{ echo \"{} not found in PATH=$PATH\" >&2; exit 127; }}; cd /workspace 2>/dev/null || true; {}",
                path_prefix,
                precheck,
                program,
                build_shell_command(program, args)
            );
            let (stdout, stderr, exit_code) =
                execute_in_docker(&docker_cfg, &cmd, timeout_secs).await?;

            if exit_code == 1 && allow_exit_code_1 {
                return Ok(CommandOutput {
                    stdout: String::new(),
                });
            }

            if exit_code != 0 {
                return Err(if stderr.trim().is_empty() {
                    format!("exit code {} for command: {}", exit_code, cmd)
                } else {
                    format!(
                        "{} [mode=docker image={} container={}]",
                        stderr.trim(),
                        docker_cfg.image,
                        docker_cfg.container_name.as_deref().unwrap_or("<none>")
                    )
                });
            }

            Ok(CommandOutput { stdout })
        }
        ShellExecutionMode::Host => {
            let mut command = Command::new(program);
            command
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let timeout = Duration::from_secs(timeout_secs);
            let output = tokio::time::timeout(timeout, command.output())
                .await
                .map_err(|_| format!("tool timeout after {} seconds", timeout_secs))?
                .map_err(|e| e.to_string())?;

            if output.status.code() == Some(1) && allow_exit_code_1 {
                return Ok(CommandOutput {
                    stdout: String::new(),
                });
            }

            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }

            Ok(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            })
        }
    }
}

async fn execute_in_docker(
    docker_cfg: &DockerSandboxConfig,
    cmd: &str,
    timeout_secs: u64,
) -> Result<(String, String, i32), String> {
    let sandbox = DockerSandbox::new(docker_cfg.clone());
    sandbox
        .execute(cmd, timeout_secs)
        .await
        .map_err(|e| e.to_string())
}

async fn run_command_for_code_search(
    program: &str,
    args: &[String],
    timeout_secs: u64,
    runtime: &AuditRuntime,
) -> Result<CommandOutput, CodeSearchError> {
    match run_audit_command(program, args, timeout_secs, runtime, true).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if e.starts_with("tool timeout after ") {
                Err(CodeSearchError::Timeout(timeout_secs))
            } else {
                Err(CodeSearchError::CommandFailed(e))
            }
        }
    }
}

async fn run_command_for_git(
    program: &str,
    args: &[String],
    timeout_secs: u64,
    runtime: &AuditRuntime,
) -> Result<CommandOutput, GitDiffScopeError> {
    match run_audit_command(program, args, timeout_secs, runtime, false).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if e.starts_with("tool timeout after ") {
                Err(GitDiffScopeError::Timeout(timeout_secs))
            } else {
                Err(GitDiffScopeError::CommandFailed(e))
            }
        }
    }
}

async fn run_command_for_git_clone(
    program: &str,
    args: &[String],
    timeout_secs: u64,
    runtime: &AuditRuntime,
) -> Result<CommandOutput, GitCloneRepoError> {
    match run_audit_command(program, args, timeout_secs, runtime, false).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if e.starts_with("tool timeout after ") {
                Err(GitCloneRepoError::Timeout(timeout_secs))
            } else {
                Err(GitCloneRepoError::CommandFailed(e))
            }
        }
    }
}

pub(crate) fn parse_rg_line(line: &str) -> Option<CodeSearchMatch> {
    let mut parts = line.splitn(4, ':');
    let file = parts.next()?.to_string();
    let line_number = parts.next()?.parse::<usize>().ok()?;
    let column = parts.next()?.parse::<usize>().ok()?;
    let text = parts.next().unwrap_or_default().to_string();
    Some(CodeSearchMatch {
        file,
        line: line_number,
        column,
        text,
    })
}

fn parse_numstat(output: &str) -> Vec<(String, (usize, usize))> {
    let mut files = Vec::new();
    for line in output.lines() {
        let mut parts = line.splitn(3, '\t');
        let adds = parts.next().unwrap_or_default();
        let dels = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default();
        if path.is_empty() {
            continue;
        }

        let additions = adds.parse::<usize>().unwrap_or(0);
        let deletions = dels.parse::<usize>().unwrap_or(0);
        files.push((path.to_string(), (additions, deletions)));
    }
    files
}

fn parse_hunks(output: &str) -> HashMap<String, Vec<DiffHunk>> {
    let mut current_file: Option<String> = None;
    let mut hunks_by_file: HashMap<String, Vec<DiffHunk>> = HashMap::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            current_file = Some(rest.to_string());
            continue;
        }
        if let Some(caps) = RE_HUNK.captures(line) {
            if let Some(path) = current_file.clone() {
                let old_start = caps
                    .get(1)
                    .and_then(|v| v.as_str().parse::<usize>().ok())
                    .unwrap_or(0);
                let old_count = caps
                    .get(2)
                    .and_then(|v| v.as_str().parse::<usize>().ok())
                    .unwrap_or(1);
                let new_start = caps
                    .get(3)
                    .and_then(|v| v.as_str().parse::<usize>().ok())
                    .unwrap_or(0);
                let new_count = caps
                    .get(4)
                    .and_then(|v| v.as_str().parse::<usize>().ok())
                    .unwrap_or(1);
                hunks_by_file.entry(path).or_default().push(DiffHunk {
                    old_start,
                    old_count,
                    new_start,
                    new_count,
                });
            }
        }
    }

    hunks_by_file
}

fn extract_function_name(line: &str) -> Option<String> {
    // Use cached regexes for each language family
    for re in [&*RE_FN_RUST, &*RE_FN_JS, &*RE_FN_PY_RB, &*RE_FN_GO] {
        if let Some(caps) = re.captures(line) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

fn extract_called_name(line: &str, focus_names: &[String]) -> Option<String> {
    for name in focus_names {
        let needle = format!("{}(", name);
        if line.contains(&needle) {
            return Some(name.clone());
        }
    }
    None
}

fn nearest_function_for_line(
    defs_by_file: &HashMap<String, Vec<(String, usize)>>,
    file: &str,
    line: usize,
) -> Option<String> {
    let defs = defs_by_file.get(file)?;
    let mut result: Option<String> = None;
    for (name, def_line) in defs {
        if *def_line <= line {
            result = Some(name.clone());
        } else {
            break;
        }
    }
    result
}

async fn run_command_for_call_graph(
    program: &str,
    args: &[String],
    timeout_secs: u64,
    runtime: &AuditRuntime,
) -> Result<CommandOutput, CallGraphLiteError> {
    match run_audit_command(program, args, timeout_secs, runtime, true).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if e.starts_with("tool timeout after ") {
                Err(CallGraphLiteError::Timeout(timeout_secs))
            } else {
                Err(CallGraphLiteError::CommandFailed(e))
            }
        }
    }
}
