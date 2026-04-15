use crate::arena::{ChallengeInfo, HintResponse};

pub fn contest_system_prompt() -> String {
    [
        "You are an unattended contest-solving agent.",
        "You are solving exactly one already-started challenge.",
        "The runner controls challenge lifecycle externally: listing, starting, hint requests, flag submission, and stopping instances are NOT your job.",
        "You may only use the provided tools to inspect the target and gather evidence.",
        "Use browser for JavaScript-rendered pages, authenticated UI flows, uploads, or when rendered content differs from raw HTTP.",
        "Use http_request for precise single requests and shell for multi-step workflows that need cookies, redirects, multipart forms, loops, or chained commands.",
        "When authentication or session state matters, prefer shell with curl/wget cookie jars over repeating stateless http_request calls.",
        "Browser session state, raw http_request state, and shell cookie-jar state are isolated from each other unless you explicitly recreate the session.",
        "If a login succeeds in browser, continue authenticated navigation in browser unless you have explicitly extracted and replayed the same session in another tool.",
        "When you identify a product, framework, CVE, or exploit family, use search_exploit early instead of guessing payloads from memory.",
        "If you are stuck, looping, or uncertain about your current line of attack, call tenth_man_review for an adversarial critique before continuing.",
        "Your goal is to find a valid flag in the format flag{...}.",
        "Do not ask for user input.",
        "Do not describe hypothetical steps without taking them when a tool can verify them.",
        "Avoid repeating the exact same failed request or command.",
        "On login pages, do not brute-force many credential guesses without evidence. First inspect frontend JavaScript, hidden routes, network requests, localStorage, cookies, and post-login redirects.",
        "When you finish a reasoning turn, your final response MUST be a single JSON object with one of these statuses:",
        r#"{"status":"continue","reason":"what to try next"}"#,
        r#"{"status":"need_hint","reason":"why hint is needed"}"#,
        r#"{"status":"candidate_flag","flag":"flag{...}","reason":"where it was found"}"#,
        r#"{"status":"done","reason":"challenge is solved or no more work remains after a correct submission"}"#,
        r#"{"status":"give_up","reason":"why you cannot make progress within budget"}"#,
        "Do not include pseudo-code objects such as {url}, {id}, or JavaScript snippets in the final answer.",
        "Return JSON only in the final answer for each turn.",
    ]
    .join("\n")
}

pub fn challenge_turn_prompt(
    execution_id: &str,
    challenge: &ChallengeInfo,
    entrypoints: &[String],
    hint: Option<&HintResponse>,
    hint_allowed: bool,
    step_index: usize,
    max_steps: usize,
    context_summary: Option<&str>,
    last_feedback: Option<&str>,
) -> String {
    let mut sections = vec![
        format!("execution_id: {}", execution_id),
        format!("step: {}/{}", step_index, max_steps),
        format!("challenge_code: {}", challenge.code),
        format!("title: {}", challenge.title),
        format!("difficulty: {}", challenge.difficulty),
        format!("level: {}", challenge.level),
        format!("total_score: {}", challenge.total_score),
        format!(
            "flag_progress: {}/{}",
            challenge.flag_got_count, challenge.flag_count
        ),
        format!("hint_allowed: {}", hint_allowed),
        "entrypoints:".to_string(),
    ];

    for entrypoint in entrypoints {
        sections.push(format!("- {}", entrypoint));
    }

    sections.push("description:".to_string());
    sections.push(challenge.description.clone());

    if let Some(hint) = hint.and_then(|value| value.hint_content.as_deref()) {
        sections.push("hint:".to_string());
        sections.push(hint.to_string());
    }

    if let Some(summary) = context_summary.filter(|value| !value.trim().is_empty()) {
        sections.push("compressed_context_summary:".to_string());
        sections.push(summary.to_string());
    }

    if let Some(last_feedback) = last_feedback.filter(|value| !value.trim().is_empty()) {
        sections.push("runner_feedback:".to_string());
        sections.push(last_feedback.to_string());
    }

    sections.push(
        "Use tools to verify reality. For JavaScript-heavy pages, prefer browser. For login flows, redirects, file uploads, or cookie-backed workflows, prefer browser or shell with curl and a cookie jar. Keep in mind that browser auth state is separate from http_request and shell state unless you explicitly recreate it. When calling tenth_man_review, use the execution_id above. Do not emit natural language at the end; emit the JSON control signal only."
            .to_string(),
    );

    sections.join("\n")
}
