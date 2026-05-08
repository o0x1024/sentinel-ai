pub fn build_terminal_session_fingerprint(
    execution_mode: crate::terminal::ExecutionMode,
    docker_image: &str,
    shell: &str,
    working_dir: &str,
) -> String {
    let mode = match execution_mode {
        crate::terminal::ExecutionMode::Docker => "docker",
        crate::terminal::ExecutionMode::Host => "host",
    };
    format!(
        "{}|{}|{}|{}",
        mode,
        docker_image.trim().to_lowercase(),
        shell.trim().to_lowercase(),
        working_dir.trim().replace('\\', "/")
    )
}

struct TerminalScreen {
    rows: Vec<Vec<char>>,
    row: usize,
    col: usize,
}

impl TerminalScreen {
    fn new() -> Self {
        Self {
            rows: vec![Vec::new()],
            row: 0,
            col: 0,
        }
    }

    fn ensure_row(&mut self) {
        while self.rows.len() <= self.row {
            self.rows.push(Vec::new());
        }
    }

    fn write_char(&mut self, ch: char) {
        self.ensure_row();
        let line = &mut self.rows[self.row];
        while line.len() < self.col {
            line.push(' ');
        }
        if self.col == line.len() {
            line.push(ch);
        } else {
            line[self.col] = ch;
        }
        self.col += 1;
    }

    fn newline(&mut self) {
        self.row += 1;
        self.col = 0;
        self.ensure_row();
    }

    fn erase_line(&mut self, mode: usize) {
        self.ensure_row();
        let line = &mut self.rows[self.row];
        match mode {
            1 => {
                let end = self.col.saturating_add(1).min(line.len());
                for ch in line.iter_mut().take(end) {
                    *ch = ' ';
                }
            }
            2 => line.clear(),
            _ => {
                if self.col < line.len() {
                    line.truncate(self.col);
                }
            }
        }
    }

    fn erase_display(&mut self, mode: usize) {
        match mode {
            2 | 3 => {
                self.rows = vec![Vec::new()];
                self.row = 0;
                self.col = 0;
            }
            _ => {
                self.erase_line(0);
                self.rows.truncate(self.row.saturating_add(1));
            }
        }
    }

    fn apply_csi(&mut self, raw_params: &str, final_char: char) {
        let normalized = raw_params.trim_start_matches('?');
        let params = parse_csi_params(normalized);
        let first = params.first().copied().flatten();
        let default_count = first.unwrap_or(1).max(1);

        match final_char {
            'A' => self.row = self.row.saturating_sub(default_count),
            'B' => {
                self.row = self.row.saturating_add(default_count);
                self.ensure_row();
            }
            'C' => self.col = self.col.saturating_add(default_count),
            'D' => self.col = self.col.saturating_sub(default_count),
            'G' => self.col = first.unwrap_or(1).saturating_sub(1),
            'H' | 'f' => {
                self.row = params
                    .first()
                    .and_then(|value| *value)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.col = params
                    .get(1)
                    .and_then(|value| *value)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.ensure_row();
            }
            'K' => self.erase_line(first.unwrap_or(0)),
            'J' => self.erase_display(first.unwrap_or(0)),
            _ => {}
        }
    }

    fn render(self) -> String {
        let mut lines = self
            .rows
            .into_iter()
            .map(|line| line.into_iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>();
        while matches!(lines.last(), Some(line) if line.trim().is_empty()) {
            lines.pop();
        }
        let normalized = lines.join("\n");
        let re_blank = regex::Regex::new(r"\n{3,}").unwrap();
        re_blank.replace_all(&normalized, "\n\n").trim().to_string()
    }
}

fn parse_csi_params(raw: &str) -> Vec<Option<usize>> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(';')
        .map(|part| {
            let digits = part
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>();
            if digits.is_empty() {
                None
            } else {
                digits.parse::<usize>().ok()
            }
        })
        .collect()
}

fn consume_osc<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(ch) = chars.next() {
        if ch == '\u{0007}' {
            break;
        }
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('\\')) {
                let _ = chars.next();
                break;
            }
        }
    }
}

fn consume_csi<I>(chars: &mut std::iter::Peekable<I>, screen: &mut TerminalScreen)
where
    I: Iterator<Item = char>,
{
    let mut params = String::new();
    while let Some(ch) = chars.next() {
        if ('@'..='~').contains(&ch) {
            screen.apply_csi(&params, ch);
            break;
        }
        params.push(ch);
    }
}

pub fn strip_ansi_codes(text: &str) -> String {
    let mut screen = TerminalScreen::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => match chars.next() {
                Some('[') => consume_csi(&mut chars, &mut screen),
                Some(']') => consume_osc(&mut chars),
                Some('=') | Some('>') | None => {}
                Some(_) => {}
            },
            '\r' => screen.col = 0,
            '\n' => screen.newline(),
            '\u{0008}' => screen.col = screen.col.saturating_sub(1),
            '\t' => {
                let next_tab = ((screen.col / 4) + 1) * 4;
                while screen.col < next_tab {
                    screen.write_char(' ');
                }
            }
            ch if ch.is_control() => {}
            ch => screen.write_char(ch),
        }
    }

    screen.render()
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

pub fn detect_prompt_state(rendered_output: &str) -> Option<serde_json::Value> {
    let option_re = regex::Regex::new(
        r"([●◉◼■○◯◌])\s*([A-Za-z][A-Za-z0-9_-]*)\s*/\s*([●◉◼■○◯◌])\s*([A-Za-z][A-Za-z0-9_-]*)",
    )
    .ok()?;
    let selected_re = regex::Regex::new(r"[●◉◼■]").ok()?;
    let lines = rendered_output.lines().collect::<Vec<_>>();

    for (index, line) in lines.iter().enumerate().rev() {
        let Some(captures) = option_re.captures(line) else {
            continue;
        };
        let first_marker = captures.get(1)?.as_str();
        let first_option = captures.get(2)?.as_str();
        let second_marker = captures.get(3)?.as_str();
        let second_option = captures.get(4)?.as_str();
        let selected = if selected_re.is_match(first_marker) {
            first_option
        } else if selected_re.is_match(second_marker) {
            second_option
        } else {
            continue;
        };
        let question = lines[..index]
            .iter()
            .rev()
            .map(|candidate| candidate.trim())
            .find(|candidate| !candidate.is_empty() && candidate.contains('?'))
            .map(str::to_string);

        return Some(serde_json::json!({
            "type": "select",
            "options": [first_option, second_option],
            "selected": selected,
            "question": question,
            "raw": line.trim(),
        }));
    }

    None
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
    fn strip_ansi_codes_removes_private_csi_sequences() {
        assert_eq!(
            strip_ansi_codes("\x1b[?25l\x1b[999D\x1b[44A\x1b[2B\x1b[2K\x1b[G○ Yes / ● No\x1b[?25h"),
            "○ Yes / ● No"
        );
    }

    #[test]
    fn strip_ansi_codes_renders_prompt_rewrite_on_same_line() {
        assert_eq!(
            strip_ansi_codes(
                "Install with npm and start now?\n● Yes / ○ No\x1b[999D\x1b[2K○ Yes / ● No"
            ),
            "Install with npm and start now?\n○ Yes / ● No"
        );
    }

    #[test]
    fn strip_ansi_codes_renders_prompt_rewrite_after_cursor_up() {
        assert_eq!(
            strip_ansi_codes("Install with npm and start now?\n● Yes / ○ No\nstatus\x1b[1A\x1b[999D\x1b[2K○ Yes / ● No"),
            "Install with npm and start now?\n○ Yes / ● No\nstatus"
        );
    }

    #[test]
    fn strip_ansi_codes_renders_clear_screen() {
        assert_eq!(
            strip_ansi_codes("old line\n\x1b[2J\x1b[Hnew line"),
            "new line"
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

    #[test]
    fn detect_prompt_state_reads_binary_selected_option() {
        let state = detect_prompt_state("Install with npm and start now?\n○ Yes / ● No")
            .expect("prompt state should be detected");

        assert_eq!(
            state.get("type").and_then(|value| value.as_str()),
            Some("select")
        );
        assert_eq!(
            state.get("selected").and_then(|value| value.as_str()),
            Some("No")
        );
        assert_eq!(
            state
                .get("options")
                .and_then(|value| value.as_array())
                .and_then(|options| options.first())
                .and_then(|value| value.as_str()),
            Some("Yes")
        );
    }
}
