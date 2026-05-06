use crate::buildin_tools::{
    load_tool_search_runtime_context, ToolSearchAction, ToolSearchArgs, ToolSearchMatch,
    ToolSearchOutput, ToolSearchRuntimeContext, ToolSearchTool,
};
use crate::tool_server::{get_tool_server, ToolInfo};

fn tool_search_tokens(query_lower: &str) -> Vec<&str> {
    query_lower
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .collect()
}

fn query_mentions_any(tokens: &[&str], options: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| options.iter().any(|option| token == option))
}

fn score_code_tool_intent(
    query_lower: &str,
    tokens: &[&str],
    tool: &ToolInfo,
    runtime_context: Option<&ToolSearchRuntimeContext>,
) -> Option<(usize, String)> {
    let tool_id = tool.name.as_str();

    if tool_id == "lsp"
        && (query_lower.contains("go to definition")
            || query_lower.contains("find references")
            || query_lower.contains("workspace symbol")
            || query_lower.contains("document symbol")
            || query_mentions_any(
                tokens,
                &[
                    "definition",
                    "definitions",
                    "references",
                    "symbol",
                    "symbols",
                    "rename",
                    "navigation",
                ],
            ))
    {
        return Some((
            90,
            "recommended for symbol navigation and code-aware lookup".to_string(),
        ));
    }

    if tool_id == "glob"
        && (query_lower.contains("find file")
            || query_lower.contains("find files")
            || query_lower.contains("which file")
            || query_lower.contains("wildcard")
            || query_mentions_any(
                tokens,
                &[
                    "file",
                    "files",
                    "filename",
                    "filenames",
                    "path",
                    "paths",
                    "glob",
                    "wildcard",
                ],
            ))
    {
        return Some((
            80,
            "recommended for filename and path discovery".to_string(),
        ));
    }

    if tool_id == "grep"
        && (query_lower.contains("search code")
            || query_lower.contains("search content")
            || query_lower.contains("find string")
            || query_lower.contains("find usage")
            || query_mentions_any(
                tokens,
                &[
                    "grep", "regex", "search", "match", "matches", "usage", "usages", "string",
                    "content",
                ],
            ))
    {
        return Some((
            80,
            "recommended for content search and usage lookup".to_string(),
        ));
    }

    if tool_id == "file_read"
        && (query_lower.contains("read file")
            || query_lower.contains("open file")
            || query_lower.contains("show lines")
            || query_lower.contains("inspect file")
            || query_mentions_any(
                tokens,
                &["read", "open", "inspect", "snippet", "lines", "line"],
            ))
    {
        return Some((
            75,
            "recommended for reading file snippets with line control".to_string(),
        ));
    }

    if tool_id == "file_edit"
        && (query_lower.contains("edit file")
            || query_lower.contains("replace text")
            || query_lower.contains("patch file")
            || query_mentions_any(tokens, &["edit", "replace", "patch", "modify", "rewrite"]))
    {
        return Some((
            70,
            "recommended for targeted text replacement in existing files".to_string(),
        ));
    }

    if tool_id == "file_write"
        && (query_lower.contains("write file")
            || query_lower.contains("create file")
            || query_lower.contains("new file")
            || query_mentions_any(tokens, &["write", "create", "overwrite", "generate"]))
    {
        return Some((
            70,
            "recommended for full-file creation or overwrite".to_string(),
        ));
    }

    if tool_id == "file_read"
        && runtime_context
            .map(|context| context.prefer_file_read_backfill)
            .unwrap_or(false)
    {
        let reason = runtime_context
            .and_then(|context| context.reason.clone())
            .unwrap_or_else(|| {
                "recent file changes detected; prefer reading back the changed result".to_string()
            });
        return Some((65, reason));
    }

    None
}

pub(crate) fn score_tool_search_match(
    query_lower: &str,
    tool: &ToolInfo,
    runtime_context: Option<&ToolSearchRuntimeContext>,
) -> Option<(usize, String)> {
    let mut score = 0usize;
    let mut reasons = Vec::new();
    let tokens = tool_search_tokens(query_lower);

    if tool.name.to_lowercase() == query_lower {
        score += 100;
        reasons.push("exact tool id match".to_string());
    }

    if tool.name.to_lowercase().contains(query_lower) {
        score += 40;
        reasons.push("tool id contains query".to_string());
    }

    if tool.description.to_lowercase().contains(query_lower) {
        score += 25;
        reasons.push("description contains query".to_string());
    }

    if let Some(search_hint) = tool.search_hint.as_ref() {
        let lowered = search_hint.to_lowercase();
        if lowered.contains(query_lower) {
            score += 30;
            reasons.push("search hint contains query".to_string());
        }
    }

    for token in &tokens {
        if tool.name.to_lowercase().contains(token) {
            score += 15;
            reasons.push(format!("tool id matched token '{}'", token));
        }
        if tool.description.to_lowercase().contains(token) {
            score += 8;
            reasons.push(format!("description matched token '{}'", token));
        }
        if let Some(search_hint) = tool.search_hint.as_ref() {
            if search_hint.to_lowercase().contains(token) {
                score += 12;
                reasons.push(format!("search hint matched token '{}'", token));
            }
        }
    }

    for tag in &tool.tags {
        let lowered = tag.to_lowercase();
        if lowered == query_lower || tokens.iter().any(|token| *token == lowered) {
            score += 10;
            reasons.push(format!("tag '{}' matched", tag));
        }
    }

    if let Some((boost, reason)) =
        score_code_tool_intent(query_lower, &tokens, tool, runtime_context)
    {
        score += boost;
        reasons.push(reason);
    }

    if score == 0 {
        return None;
    }

    Some((score, reasons.join("; ")))
}

pub(crate) fn recommend_tool_bundle(
    query_lower: &str,
    available_tools: &std::collections::HashSet<String>,
    runtime_context: Option<&ToolSearchRuntimeContext>,
) -> (Vec<String>, Option<String>) {
    let tokens = tool_search_tokens(query_lower);
    let mut bundle = Vec::new();
    let mut reason = None;

    if query_lower.contains("go to definition")
        || query_lower.contains("find references")
        || query_lower.contains("workspace symbol")
        || query_lower.contains("document symbol")
        || query_mentions_any(
            &tokens,
            &[
                "definition",
                "definitions",
                "references",
                "reference",
                "symbol",
                "symbols",
                "navigation",
            ],
        )
    {
        bundle = vec!["lsp".to_string(), "file_read".to_string()];
        reason = Some("recommended bundle for symbol navigation and code inspection".to_string());
    } else if query_lower.contains("edit file")
        || query_lower.contains("patch file")
        || query_lower.contains("replace text")
        || query_mentions_any(&tokens, &["edit", "replace", "patch", "modify", "rewrite"])
    {
        bundle = vec!["file_read".to_string(), "file_edit".to_string()];
        reason = Some("recommended bundle for safe read-then-edit workflow".to_string());
    } else if query_lower.contains("write file")
        || query_lower.contains("create file")
        || query_lower.contains("new file")
        || query_mentions_any(&tokens, &["write", "create", "overwrite", "generate"])
    {
        bundle = vec!["file_read".to_string(), "file_write".to_string()];
        reason = Some("recommended bundle for safe read-before-write workflow".to_string());
    } else if query_lower.contains("find file")
        || query_lower.contains("find files")
        || query_lower.contains("filename")
        || query_lower.contains("wildcard")
        || query_mentions_any(
            &tokens,
            &[
                "file",
                "files",
                "filename",
                "filenames",
                "path",
                "paths",
                "glob",
                "wildcard",
            ],
        )
    {
        bundle = vec!["glob".to_string(), "file_read".to_string()];
        reason = Some("recommended bundle for locating files and then inspecting them".to_string());
    } else if query_lower.contains("find usage")
        || query_lower.contains("search code")
        || query_lower.contains("search content")
        || query_lower.contains("find string")
        || query_mentions_any(
            &tokens,
            &[
                "usage", "usages", "search", "regex", "string", "content", "match", "matches",
            ],
        )
    {
        bundle = vec!["grep".to_string(), "file_read".to_string()];
        reason =
            Some("recommended bundle for searching content and reading matched files".to_string());
    } else if runtime_context
        .map(|context| context.prefer_file_read_backfill)
        .unwrap_or(false)
    {
        bundle = vec!["file_read".to_string()];
        reason = runtime_context
            .and_then(|context| context.reason.clone())
            .or_else(|| {
                Some(
                    "recent file changes detected; recommended bundle prioritizes reading back the result"
                        .to_string(),
                )
            });
    }

    bundle.retain(|tool_id| available_tools.contains(tool_id));
    if bundle.is_empty() {
        (Vec::new(), None)
    } else {
        (bundle, reason)
    }
}

pub(crate) async fn run_tool_search(args: ToolSearchArgs) -> Result<ToolSearchOutput, String> {
    let tool_server = get_tool_server();
    let catalog = tool_server.list_tools().await;
    let runtime_context = load_tool_search_runtime_context(args.execution_id.as_deref()).await;
    let runtime_hint = build_runtime_hint(runtime_context.as_ref());

    match args.action {
        ToolSearchAction::Search => {
            let query = args
                .query
                .as_deref()
                .map(str::trim)
                .filter(|query| !query.is_empty())
                .ok_or_else(|| "query is required for tool_search action=search".to_string())?;

            let query_lower = query.to_lowercase();
            let available_tools = catalog
                .iter()
                .map(|tool| tool.name.clone())
                .collect::<std::collections::HashSet<_>>();
            let mut scored = Vec::new();
            for tool in catalog {
                if tool.name == ToolSearchTool::NAME {
                    continue;
                }
                if let Some((score, reason)) =
                    score_tool_search_match(&query_lower, &tool, runtime_context.as_ref())
                {
                    scored.push((score, tool, reason));
                }
            }

            scored.sort_by(|left, right| {
                right
                    .0
                    .cmp(&left.0)
                    .then_with(|| left.1.name.cmp(&right.1.name))
            });
            scored.truncate(args.max_results.max(1));

            let (recommended_tool_ids, recommendation_reason) =
                recommend_tool_bundle(&query_lower, &available_tools, runtime_context.as_ref());

            let matches = scored
                .into_iter()
                .map(|(_score, tool, reason)| ToolSearchMatch {
                    tool_id: tool.name,
                    description: tool.description,
                    reason,
                    category: Some(tool.category.to_string()),
                    source: Some(tool.source.to_string()),
                    search_hint: tool.search_hint,
                    exposure: Some(tool.exposure),
                    already_active: false,
                })
                .collect::<Vec<_>>();

            let message = if matches.is_empty() {
                "No matching tools found".to_string()
            } else if let Some(reason) = &recommendation_reason {
                format!(
                    "Found {} matching tools. Recommended bundle: {} ({})",
                    matches.len(),
                    recommended_tool_ids.join(", "),
                    reason
                )
            } else {
                format!("Found {} matching tools", matches.len())
            };

            Ok(ToolSearchOutput {
                action: "search".to_string(),
                matches,
                activated_tool_ids: Vec::new(),
                recommended_tool_ids,
                recommendation_reason,
                runtime_hint,
                requires_reload: false,
                message,
            })
        }
        ToolSearchAction::Activate => {
            let available = catalog
                .iter()
                .map(|tool| tool.name.clone())
                .collect::<std::collections::HashSet<_>>();
            let query_lower = args
                .query
                .as_deref()
                .map(str::trim)
                .filter(|query| !query.is_empty())
                .map(str::to_lowercase);
            let (recommended_tool_ids, recommendation_reason) = query_lower
                .as_deref()
                .map(|query| recommend_tool_bundle(query, &available, runtime_context.as_ref()))
                .unwrap_or_default();
            let requested_tool_ids = args
                .tool_ids
                .unwrap_or_else(|| recommended_tool_ids.clone())
                .into_iter()
                .filter(|tool_id| available.contains(tool_id))
                .collect::<Vec<_>>();

            if requested_tool_ids.is_empty() {
                return Err(
                    "tool_ids is required for tool_search action=activate when no recommended bundle is available"
                        .to_string(),
                );
            }

            let matches = requested_tool_ids
                .iter()
                .filter_map(|tool_id| {
                    catalog
                        .iter()
                        .find(|tool| &tool.name == tool_id)
                        .map(|tool| ToolSearchMatch {
                            tool_id: tool.name.clone(),
                            description: tool.description.clone(),
                            reason: recommendation_reason
                                .clone()
                                .unwrap_or_else(|| "explicit activation request".to_string()),
                            category: Some(tool.category.to_string()),
                            source: Some(tool.source.to_string()),
                            search_hint: tool.search_hint.clone(),
                            exposure: Some(tool.exposure),
                            already_active: false,
                        })
                })
                .collect::<Vec<_>>();

            Ok(ToolSearchOutput {
                action: "activate".to_string(),
                matches,
                activated_tool_ids: requested_tool_ids.clone(),
                recommended_tool_ids,
                recommendation_reason,
                runtime_hint,
                requires_reload: true,
                message: format!("Activated {} tools", requested_tool_ids.len()),
            })
        }
    }
}

fn build_runtime_hint(runtime_context: Option<&ToolSearchRuntimeContext>) -> Option<String> {
    runtime_context
        .filter(|context| context.prefer_file_read_backfill)
        .and_then(|context| context.reason.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str, description: &str, tags: &[&str]) -> ToolInfo {
        ToolInfo {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
            source: crate::dynamic_tool::ToolSource::Builtin,
            category: crate::dynamic_tool::ToolCategory::Utility,
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            search_hint: None,
            exposure: crate::dynamic_tool::ToolExposure::Deferred,
            execution_policy: Default::default(),
            enabled: true,
        }
    }

    #[test]
    fn tool_search_scoring_prefers_lsp_for_definition_queries() {
        let lsp = make_tool(
            "lsp",
            "Go to definition and find references",
            &["code", "symbol"],
        );
        let shell = make_tool("shell", "Run shell commands", &["system"]);

        let lsp_score = score_tool_search_match("go to definition for user service", &lsp, None)
            .map(|hit| hit.0)
            .unwrap_or(0);
        let shell_score =
            score_tool_search_match("go to definition for user service", &shell, None)
                .map(|hit| hit.0)
                .unwrap_or(0);

        assert!(lsp_score > shell_score);
    }

    #[test]
    fn tool_search_scoring_prefers_glob_for_find_file_queries() {
        let glob = make_tool("glob", "Find files by wildcard", &["file", "path"]);
        let file_read = make_tool(
            "file_read",
            "Read a file with line ranges",
            &["file", "read"],
        );

        let glob_score = score_tool_search_match("find file named config", &glob, None)
            .map(|hit| hit.0)
            .unwrap_or(0);
        let read_score = score_tool_search_match("find file named config", &file_read, None)
            .map(|hit| hit.0)
            .unwrap_or(0);

        assert!(glob_score > read_score);
    }

    #[test]
    fn tool_search_scoring_prefers_grep_for_usage_queries() {
        let grep = make_tool("grep", "Search content by regex", &["search", "regex"]);
        let glob = make_tool("glob", "Find files by wildcard", &["file", "path"]);

        let grep_score = score_tool_search_match("find usage of auth token", &grep, None)
            .map(|hit| hit.0)
            .unwrap_or(0);
        let glob_score = score_tool_search_match("find usage of auth token", &glob, None)
            .map(|hit| hit.0)
            .unwrap_or(0);

        assert!(grep_score > glob_score);
    }

    #[test]
    fn tool_search_bundle_recommends_lsp_and_file_read_for_definition_queries() {
        let available = ["lsp", "file_read", "grep"]
            .into_iter()
            .map(|tool| tool.to_string())
            .collect::<std::collections::HashSet<_>>();
        let (bundle, reason) =
            recommend_tool_bundle("go to definition for auth service", &available, None);

        assert_eq!(bundle, vec!["lsp".to_string(), "file_read".to_string()]);
        assert!(reason
            .as_deref()
            .unwrap_or_default()
            .contains("symbol navigation"));
    }

    #[test]
    fn tool_search_bundle_recommends_grep_and_file_read_for_usage_queries() {
        let available = ["grep", "file_read", "glob"]
            .into_iter()
            .map(|tool| tool.to_string())
            .collect::<std::collections::HashSet<_>>();
        let (bundle, reason) = recommend_tool_bundle("find usage of auth token", &available, None);

        assert_eq!(bundle, vec!["grep".to_string(), "file_read".to_string()]);
        assert!(reason
            .as_deref()
            .unwrap_or_default()
            .contains("searching content"));
    }

    #[test]
    fn tool_search_bundle_recommends_file_write_and_file_read_for_write_queries() {
        let available = ["file_write", "file_read", "grep"]
            .into_iter()
            .map(|tool| tool.to_string())
            .collect::<std::collections::HashSet<_>>();
        let (bundle, reason) =
            recommend_tool_bundle("create file for the new config", &available, None);

        assert_eq!(
            bundle,
            vec!["file_read".to_string(), "file_write".to_string()]
        );
        assert!(reason
            .as_deref()
            .unwrap_or_default()
            .contains("read-before-write"));
    }

    #[test]
    fn tool_search_runtime_context_prefers_file_read_when_recent_changes_exist() {
        let file_read = make_tool(
            "file_read",
            "Read a file with line ranges",
            &["file", "read"],
        );
        let file_write = make_tool(
            "file_write",
            "Create or overwrite files",
            &["file", "write"],
        );
        let runtime_context = ToolSearchRuntimeContext {
            prefer_file_read_backfill: true,
            reason: Some("recent file changes detected; prefer readback".to_string()),
        };

        let read_score =
            score_tool_search_match("what should I use next", &file_read, Some(&runtime_context))
                .map(|hit| hit.0)
                .unwrap_or(0);
        let write_score = score_tool_search_match(
            "what should I use next",
            &file_write,
            Some(&runtime_context),
        )
        .map(|hit| hit.0)
        .unwrap_or(0);

        assert!(read_score > write_score);
    }

    #[test]
    fn tool_search_runtime_hint_surfaces_bias_reason() {
        let runtime_context = ToolSearchRuntimeContext {
            prefer_file_read_backfill: true,
            reason: Some("recent file changes triggered readback bias".to_string()),
        };

        assert_eq!(
            build_runtime_hint(Some(&runtime_context)),
            Some("recent file changes triggered readback bias".to_string())
        );
    }
}
