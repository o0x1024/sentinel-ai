use super::*;
use crate::dynamic_tool::ToolCategory;
use crate::terminal_output::{sanitize_interactive_output, strip_ansi_codes};
use serde_json::json;

#[test]
fn strip_ansi_codes_renders_carriage_returns() {
    assert_eq!(
        strip_ansi_codes("progress 1\rprogress 2\nok"),
        "progress 2\nok"
    );
}

#[test]
fn sanitize_interactive_output_removes_echo_and_prompt() {
    let raw = "cat <<'EOF'\r\n> hello\r\n> EOF\r\nhello\r\n$ ";
    assert_eq!(
        sanitize_interactive_output(raw, "cat <<'EOF'\nhello\nEOF"),
        "hello"
    );
}

#[tokio::test]
async fn test_tool_server_init() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    assert!(server.tool_count().await >= 4);

    // Check builtin tools exist
    assert!(server.get_tool("http_request").await.is_some());
    assert!(server.get_tool("shell").await.is_some());
    assert!(server.get_tool("interactive_shell").await.is_none());
    assert!(server.get_tool("exec_command").await.is_none());
    assert!(server.get_tool("write_stdin").await.is_none());
    assert!(server.get_tool("tasks").await.is_some());
    assert!(server.get_tool("web_search").await.is_some());
    assert!(server.get_tool("port_scan").await.is_some());
    assert!(server.get_tool("subdomain_brute").await.is_some());
    assert!(server.get_tool("tool_search").await.is_some());
    assert!(server.get_tool("browser").await.is_none());
    assert!(server.get_tool("browser_shell").await.is_some());
    assert!(server.get_tool("route_discovery").await.is_some());

    let port_scan = server
        .get_tool("port_scan")
        .await
        .expect("port_scan should exist");
    let subdomain_brute = server
        .get_tool("subdomain_brute")
        .await
        .expect("subdomain_brute should exist");
    let file_read = server
        .get_tool("file_read")
        .await
        .expect("file_read should exist");
    let shell = server.get_tool("shell").await.expect("shell should exist");
    let tasks = server.get_tool("tasks").await.expect("tasks should exist");
    let skills = server
        .get_tool("skills")
        .await
        .expect("skills should exist");
    let skill_creator = server
        .get_tool("skill_creator")
        .await
        .expect("skill_creator should exist");
    let search_exploit = server
        .get_tool("search_exploit")
        .await
        .expect("search_exploit should exist");
    assert_eq!(port_scan.category, ToolCategory::SecurityRecon);
    assert_eq!(subdomain_brute.category, ToolCategory::SecurityRecon);
    assert_eq!(file_read.category, ToolCategory::FileCode);
    assert_eq!(shell.category, ToolCategory::Terminal);
    assert_eq!(tasks.category, ToolCategory::Collaboration);
    assert_eq!(skills.category, ToolCategory::KnowledgeExtension);
    assert_eq!(skill_creator.category, ToolCategory::KnowledgeExtension);
    assert_eq!(search_exploit.category, ToolCategory::VulnerabilityResearch);

    let skills_schema = skills.input_schema.to_string();
    assert!(
        skills_schema.contains("\"skill\""),
        "skills schema should expose skill parameter: {}",
        skills_schema
    );
    assert!(
        !skills_schema.contains("\"read_file\""),
        "skills schema must not expose read_file action: {}",
        skills_schema
    );
    assert!(
        !skills_schema.contains("\"list\""),
        "skills schema must not expose list action: {}",
        skills_schema
    );

    let skill_creator_schema = skill_creator.input_schema.to_string();
    assert!(
        skill_creator_schema.contains("\"create\"")
            && skill_creator_schema.contains("\"validate\""),
        "skill_creator schema should expose authoring actions: {}",
        skill_creator_schema
    );
}

#[tokio::test]
async fn shell_short_command_completes_without_session_id() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let result = server
        .execute(
            "shell",
            json!({
                "command": "printf short-ok",
                "execution_mode": "host"
            }),
        )
        .await;

    assert!(result.success, "short shell command should succeed");
    let output = result.output.expect("shell command should return output");
    assert_eq!(
        output.get("completed").and_then(|value| value.as_bool()),
        Some(true),
        "unexpected short command output: {}",
        output
    );
    assert!(
        output
            .get("stdout")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .contains("short-ok"),
        "unexpected short command output: {}",
        output
    );
    assert!(output.get("session_id").is_none());
}

#[tokio::test]
async fn shell_waiting_command_returns_session_and_can_cancel() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let first = server
        .execute(
            "shell",
            json!({
                "command": "cat",
                "execution_mode": "host",
                "yield_time_ms": 100
            }),
        )
        .await;

    assert!(
        first.success,
        "shell session start should succeed: {:?}",
        first.error
    );
    let first_output = first.output.expect("shell session should return output");
    let session_id = first_output
        .get("session_id")
        .and_then(|value| value.as_str())
        .expect("shell session should return session_id")
        .to_string();
    assert_eq!(
        first_output.get("status").and_then(|value| value.as_str()),
        Some("input_waiting"),
        "unexpected shell session output: {}",
        first_output
    );

    let second = server
        .execute(
            "shell",
            json!({
                "session_id": session_id.clone(),
                "action": "cancel"
            }),
        )
        .await;

    assert!(second.success, "shell session cancel should succeed");
    let second_output = second.output.expect("cancel should return output");
    assert_eq!(
        second_output.get("status").and_then(|value| value.as_str()),
        Some("cancelled"),
        "unexpected cancel output: {}",
        second_output
    );
}

#[tokio::test]
async fn shell_interactive_command_without_yield_time_stays_one_shot() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let result = server
        .execute(
            "shell",
            json!({
                "command": "cat",
                "execution_mode": "host"
            }),
        )
        .await;

    assert!(
        result.success,
        "interactive one-shot shell call should return a structured shell result"
    );
    let output = result
        .output
        .expect("interactive one-shot shell call should return output");
    assert_eq!(
        output.get("success").and_then(|value| value.as_bool()),
        Some(false),
        "unexpected interactive one-shot output: {}",
        output
    );
    assert_eq!(
        output
            .get("interaction_required")
            .and_then(|value| value.as_bool()),
        Some(true),
        "unexpected interactive one-shot output: {}",
        output
    );
    assert!(
        output.get("session_id").is_none(),
        "unexpected session output: {}",
        output
    );
}

#[tokio::test]
async fn shell_poll_does_not_write_to_waiting_session() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let first = server
        .execute(
            "shell",
            json!({
                "command": "cat",
                "execution_mode": "host",
                "yield_time_ms": 100
            }),
        )
        .await;

    assert!(first.success, "shell session start should succeed");
    let first_output = first.output.expect("shell session should return output");
    let session_id = first_output
        .get("session_id")
        .and_then(|value| value.as_str())
        .expect("shell session should return session_id")
        .to_string();

    let poll = server
        .execute(
            "shell",
            json!({
                "session_id": session_id.clone(),
                "action": "poll",
                "yield_time_ms": 250
            }),
        )
        .await;
    assert!(poll.success, "shell poll should succeed");
    let poll_output = poll.output.expect("poll should return output");
    assert_eq!(
        poll_output.get("action").and_then(|value| value.as_str()),
        Some("poll")
    );
    assert_eq!(
        poll_output.get("stdout").and_then(|value| value.as_str()),
        Some("")
    );

    let write = server
        .execute(
            "shell",
            json!({
                "session_id": session_id.clone(),
                "action": "write",
                "chars": "poll-check\n",
                "yield_time_ms": 250
            }),
        )
        .await;
    assert!(write.success, "shell write should succeed");
    let write_output = write.output.expect("write should return output");
    assert!(
        write_output
            .get("stdout")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .contains("poll-check"),
        "unexpected write output: {}",
        write_output
    );

    let cancel = server
        .execute(
            "shell",
            json!({
                "session_id": session_id,
                "action": "cancel"
            }),
        )
        .await;
    assert!(cancel.success, "shell cancel should succeed");
}

#[tokio::test]
async fn shell_key_actions_handle_vite_style_multiselect_prompt() {
    use crate::buildin_tools::shell::{get_shell_config, set_shell_config};

    let original_shell_config = get_shell_config().await;
    let mut shell_config = original_shell_config.clone();
    shell_config.allowed_commands.push("node -e".to_string());
    set_shell_config(shell_config).await;

    let server = ToolServer::new();
    server.init_builtin_tools().await;
    let script = r#"node -e "const options=['Cancel operation','Remove existing files and continue','Ignore files and continue']; let selected=0; const render=()=>process.stdout.write('\x1b[999D\x1b[2KCurrent directory is not empty. Please choose how to proceed:\n' + options.map((option,index)=>(index===selected?'\u25cf ':'\u25cb ') + option).join('\n')); render(); process.stdin.setRawMode(true); process.stdin.resume(); process.stdin.on('data', (buf) => { const s=buf.toString('utf8'); for (const ch of s.match(/\x1b\[[AB]|\r/g) || []) { if (ch === '\x1b[B') selected = Math.min(options.length - 1, selected + 1); if (ch === '\x1b[A') selected = Math.max(0, selected - 1); if (ch === '\r') { process.stdout.write('\nconfirmed=' + options[selected] + '\n'); process.exit(0); } } });""#;

    let first = server
        .execute(
            "shell",
            json!({
                "command": script,
                "execution_mode": "host",
                "yield_time_ms": 300
            }),
        )
        .await;

    assert!(
        first.success,
        "shell prompt command should start: {:?}",
        first.error
    );
    let first_output = first.output.expect("shell prompt should return output");
    let session_id = first_output
        .get("session_id")
        .and_then(|value| value.as_str())
        .expect("shell prompt should return session_id")
        .to_string();

    let key = server
        .execute(
            "shell",
            json!({
                "session_id": session_id.clone(),
                "action": "key",
                "key": "ArrowDown",
                "repeat": 2,
                "yield_time_ms": 250
            }),
        )
        .await;

    assert!(key.success, "shell key action should succeed");
    let key_output = key.output.expect("key should return output");
    assert_eq!(
        key_output.get("action").and_then(|value| value.as_str()),
        Some("key"),
        "unexpected key output: {}",
        key_output
    );

    let submit = server
        .execute(
            "shell",
            json!({
                "session_id": session_id,
                "action": "submit",
                "yield_time_ms": 1000
            }),
        )
        .await;

    assert!(submit.success, "shell prompt submit should succeed");
    let submit_output = submit.output.expect("submit should return output");
    assert!(
        submit_output
            .get("stdout")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .contains("confirmed=Ignore files and continue"),
        "unexpected submit output: {}",
        submit_output
    );
    assert_eq!(
        submit_output.get("status").and_then(|value| value.as_str()),
        Some("completed"),
        "unexpected submit status: {}",
        submit_output
    );

    set_shell_config(original_shell_config).await;
}

#[tokio::test]
async fn shell_select_option_is_not_supported() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let first = server
        .execute(
            "shell",
            json!({
                "command": "cat",
                "execution_mode": "host",
                "yield_time_ms": 100
            }),
        )
        .await;
    assert!(first.success, "shell session start should succeed");
    let first_output = first.output.expect("shell session should return output");
    let session_id = first_output
        .get("session_id")
        .and_then(|value| value.as_str())
        .expect("shell session should return session_id")
        .to_string();

    let select = server
        .execute(
            "shell",
            json!({
                "session_id": session_id.clone(),
                "action": "select_option",
                "target_option": "Yes"
            }),
        )
        .await;

    assert!(!select.success, "select_option should be rejected");
    assert!(
        select
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("\"key\""),
        "unexpected select_option error: {:?}",
        select.error
    );

    let cancel = server
        .execute(
            "shell",
            json!({
                "session_id": session_id,
                "action": "cancel"
            }),
        )
        .await;
    assert!(cancel.success, "shell cancel should succeed");
}

#[tokio::test]
async fn list_tools_by_source_preserves_builtin_categories() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let builtin_tools = server.list_tools_by_source("builtin").await;
    let browser_shell = builtin_tools
        .iter()
        .find(|tool| tool.name == "browser_shell")
        .expect("browser_shell tool should exist");
    let route_discovery = builtin_tools
        .iter()
        .find(|tool| tool.name == "route_discovery")
        .expect("route_discovery tool should exist");

    assert_eq!(browser_shell.category, ToolCategory::WebNetwork);
    assert_eq!(route_discovery.category, ToolCategory::SecurityRecon);
}

#[tokio::test]
async fn shell_schema_exposes_prompt_capable_session_fields() {
    let server = ToolServer::new();
    server.init_builtin_tools().await;

    let shell = server.get_tool("shell").await.expect("shell should exist");
    let properties = shell
        .input_schema
        .get("properties")
        .and_then(|value| value.as_object())
        .expect("shell schema should have properties");

    assert!(properties.contains_key("yield_time_ms"));
    assert!(properties.contains_key("session_id"));
    assert!(properties.contains_key("process_id"));
    assert!(properties.contains_key("chars"));
    assert!(properties.contains_key("key"));

    let chars_description = properties
        .get("chars")
        .and_then(|value| value.get("description"))
        .and_then(|value| value.as_str())
        .expect("chars should describe raw input semantics");
    assert!(chars_description.contains("Raw characters"));
    assert!(!chars_description.contains("select_option"));

    let action_description = properties
        .get("action")
        .and_then(|value| value.get("description"))
        .and_then(|value| value.as_str())
        .expect("action should describe prompt semantics");
    assert!(action_description.contains("TTY Enter"));
    assert!(action_description.contains("explicit terminal key"));
    assert!(!action_description.contains("select_option"));
}
