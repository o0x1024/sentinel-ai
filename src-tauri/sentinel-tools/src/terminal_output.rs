pub fn build_terminal_session_fingerprint(
    execution_mode: crate::terminal::ExecutionMode,
    docker_image: &str,
    shell: &str,
) -> String {
    let mode = match execution_mode {
        crate::terminal::ExecutionMode::Docker => "docker",
        crate::terminal::ExecutionMode::Host => "host",
    };
    format!(
        "{}|{}|{}",
        mode,
        docker_image.trim().to_lowercase(),
        shell.trim().to_lowercase()
    )
}

pub fn strip_ansi_codes(text: &str) -> String {
    let re = regex::Regex::new(
        r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\][0-9;]*[^\x07]*\x07|\x1b[=>]|\x1b\][0-9];[^\x07]*\x07",
    )
    .unwrap();
    let without_ansi = re.replace_all(text, "").to_string();

    let mut rendered_lines: Vec<String> = Vec::new();
    let mut line: Vec<char> = Vec::new();
    let mut cursor = 0usize;

    let flush_line =
        |rendered_lines: &mut Vec<String>, line: &mut Vec<char>, cursor: &mut usize| {
            let rendered = line.iter().collect::<String>();
            rendered_lines.push(rendered.trim_end().to_string());
            line.clear();
            *cursor = 0;
        };

    for ch in without_ansi.chars() {
        match ch {
            '\r' => cursor = 0,
            '\n' => flush_line(&mut rendered_lines, &mut line, &mut cursor),
            '\u{0008}' => {
                cursor = cursor.saturating_sub(1);
            }
            '\t' => {
                while line.len() <= cursor {
                    line.push(' ');
                }
                line[cursor] = '\t';
                cursor += 1;
            }
            ch if ch.is_control() => {}
            ch => {
                while line.len() < cursor {
                    line.push(' ');
                }
                if cursor == line.len() {
                    line.push(ch);
                } else {
                    line[cursor] = ch;
                }
                cursor += 1;
            }
        }
    }

    if !line.is_empty() {
        flush_line(&mut rendered_lines, &mut line, &mut cursor);
    }

    let normalized = rendered_lines.join("\n");
    let re_blank = regex::Regex::new(r"\n{3,}").unwrap();
    let cleaned = re_blank.replace_all(&normalized, "\n\n").to_string();

    cleaned.trim().to_string()
}

fn strip_interactive_command_echo(output: &str, command: &str) -> String {
    if output.is_empty() || command.trim().is_empty() {
        return output.to_string();
    }

    let output_lines = output.lines().collect::<Vec<_>>();
    let command_lines = command.lines().collect::<Vec<_>>();
    if output_lines.len() < command_lines.len() {
        return output.to_string();
    }

    let mut matched = 0usize;
    while matched < command_lines.len() {
        let output_line = output_lines[matched].trim_end();
        let command_line = command_lines[matched].trim_end();
        let normalized_output = output_line
            .strip_prefix("$ ")
            .or_else(|| output_line.strip_prefix("# "))
            .or_else(|| output_line.strip_prefix("% "))
            .or_else(|| output_line.strip_prefix("> "))
            .unwrap_or(output_line)
            .trim_end();
        if normalized_output != command_line {
            break;
        }
        matched += 1;
    }

    if matched == 0 {
        return output.to_string();
    }

    output_lines[matched..].join("\n").trim_start().to_string()
}

fn trim_trailing_shell_prompt(output: &str) -> String {
    let mut lines = output.lines().collect::<Vec<_>>();
    while matches!(lines.last(), Some(line) if crate::terminal::is_shell_prompt_line(line)) {
        lines.pop();
    }
    lines.join("\n").trim_end().to_string()
}

pub fn sanitize_interactive_output(raw_output: &str, command: &str) -> String {
    let rendered = strip_ansi_codes(raw_output);
    let without_echo = strip_interactive_command_echo(&rendered, command);
    trim_trailing_shell_prompt(&without_echo)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
