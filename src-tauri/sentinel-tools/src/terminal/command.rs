//! Command normalization and detection for interactive shell
//!
//! Handles long-running commands by:
//! 1. Auto-adding limits to known continuous-output commands
//! 2. Detecting command completion via shell prompt

use regex::Regex;
use std::sync::LazyLock;

/// Wait strategy for command output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaitStrategy {
    /// Auto-detect completion (default): prompt detection + idle timeout
    #[default]
    Auto,
    /// Wait for shell prompt to appear
    Prompt,
    /// Fixed timeout only
    Timeout,
    /// Wait for specific number of output lines
    Lines,
}

impl WaitStrategy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "prompt" => Self::Prompt,
            "timeout" => Self::Timeout,
            "lines" => Self::Lines,
            _ => Self::Auto,
        }
    }
}

/// Shell prompt patterns for completion detection
static PROMPT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Basic prompts: $, #, >, %
        Regex::new(r"[$#>%]\s*$").unwrap(),
        // user@host:path$ format
        Regex::new(r"\w+@[\w\-\.]+:[^\n]*[$#>%]\s*$").unwrap(),
        // (venv) $ format
        Regex::new(r"\([^)]+\)\s*[$#>%]\s*$").unwrap(),
        // Docker container prompt
        Regex::new(r"\(sandbox[^)]*\)[^\n]*[$#>%]\s*$").unwrap(),
        // root@container:/path#
        Regex::new(r"root@[a-f0-9]+:[^\n]*#\s*$").unwrap(),
    ]
});

/// Check if output ends with a shell prompt (command completed)
pub fn detect_shell_prompt(output: &str) -> bool {
    // Get last few lines for prompt detection
    let last_part: String = output
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    PROMPT_PATTERNS.iter().any(|p| p.is_match(&last_part))
}

/// Known long-running commands that need normalization
struct CommandNormalizer {
    command: &'static str,
    normalizer: fn(&[&str]) -> Option<String>,
}

/// Normalize long-running commands to have bounded output
pub fn normalize_command(cmd: &str) -> (String, bool) {
    let cmd_trimmed = cmd.trim();
    let parts: Vec<&str> = cmd_trimmed.split_whitespace().collect();

    if parts.is_empty() {
        return (cmd.to_string(), false);
    }

    let base_cmd = parts[0].split('/').last().unwrap_or(parts[0]);

    let normalizers: &[CommandNormalizer] = &[
        CommandNormalizer {
            command: "ping",
            normalizer: normalize_ping,
        },
        CommandNormalizer {
            command: "top",
            normalizer: normalize_top,
        },
        CommandNormalizer {
            command: "htop",
            normalizer: normalize_htop,
        },
        CommandNormalizer {
            command: "watch",
            normalizer: normalize_watch,
        },
        CommandNormalizer {
            command: "tail",
            normalizer: normalize_tail,
        },
        CommandNormalizer {
            command: "tcpdump",
            normalizer: normalize_tcpdump,
        },
        CommandNormalizer {
            command: "tshark",
            normalizer: normalize_tshark,
        },
    ];

    for n in normalizers {
        if base_cmd == n.command {
            if let Some(normalized) = (n.normalizer)(&parts) {
                return (normalized, true);
            }
        }
    }

    (cmd.to_string(), false)
}

/// Normalize ping: add -c 4 if no count specified
fn normalize_ping(parts: &[&str]) -> Option<String> {
    // Check if already has count flag
    if parts.contains(&"-c") || parts.contains(&"-n") || parts.contains(&"-w") {
        return None;
    }

    // Find the target (last non-flag argument)
    let mut new_parts = parts.to_vec();

    // Insert -c 4 after 'ping'
    if new_parts.len() > 1 {
        new_parts.insert(1, "-c");
        new_parts.insert(2, "4");
    }

    Some(new_parts.join(" "))
}

/// Normalize top: run once and exit
fn normalize_top(parts: &[&str]) -> Option<String> {
    // Check if already has iteration limit
    if parts.contains(&"-n") || parts.contains(&"-l") || parts.contains(&"-b") {
        return None;
    }

    // macOS uses -l, Linux uses -b -n
    #[cfg(target_os = "macos")]
    {
        Some(format!("{} -l 1", parts[0]))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(format!("{} -b -n 1", parts[0]))
    }
}

/// Normalize htop: not suitable for non-interactive, suggest top
fn normalize_htop(_parts: &[&str]) -> Option<String> {
    // htop is interactive-only, replace with top
    #[cfg(target_os = "macos")]
    {
        Some("top -l 1".to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some("top -b -n 1".to_string())
    }
}

/// Normalize watch: execute the command once instead
fn normalize_watch(parts: &[&str]) -> Option<String> {
    if parts.len() <= 1 {
        return None;
    }

    // Skip watch flags and get the actual command
    let mut i = 1;
    while i < parts.len() {
        let part = parts[i];
        if part.starts_with('-') {
            // Skip flag and its value if needed
            if matches!(part, "-n" | "-d" | "-t" | "-p" | "-g") {
                i += 1; // Skip the flag
                if i < parts.len() && !parts[i].starts_with('-') {
                    i += 1; // Skip the value
                }
            } else {
                i += 1;
            }
        } else {
            break;
        }
    }

    if i < parts.len() {
        Some(parts[i..].join(" "))
    } else {
        None
    }
}

/// Normalize tail: replace -f/-F with -n 50
fn normalize_tail(parts: &[&str]) -> Option<String> {
    if !parts.contains(&"-f") && !parts.contains(&"-F") {
        return None;
    }

    let new_cmd = parts
        .iter()
        .map(|&p| {
            if p == "-f" || p == "-F" {
                "-n"
            } else {
                p
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Add line count if not present
    if !parts.contains(&"-n") {
        Some(new_cmd.replace("-n", "-n 50"))
    } else {
        Some(new_cmd)
    }
}

/// Normalize tcpdump: add -c 10 if no count
fn normalize_tcpdump(parts: &[&str]) -> Option<String> {
    if parts.contains(&"-c") {
        return None;
    }

    let mut new_parts = parts.to_vec();
    new_parts.insert(1, "-c");
    new_parts.insert(2, "10");
    Some(new_parts.join(" "))
}

/// Normalize tshark: add -c 10 if no count
fn normalize_tshark(parts: &[&str]) -> Option<String> {
    if parts.contains(&"-c") {
        return None;
    }

    let mut new_parts = parts.to_vec();
    new_parts.insert(1, "-c");
    new_parts.insert(2, "10");
    Some(new_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ping() {
        let (cmd, modified) = normalize_command("ping baidu.com");
        assert!(modified);
        assert!(cmd.contains("-c 4"));

        let (cmd, modified) = normalize_command("ping -c 10 baidu.com");
        assert!(!modified);
        assert!(!cmd.contains("-c 4"));
    }

    #[test]
    fn test_normalize_tail() {
        let (cmd, modified) = normalize_command("tail -f /var/log/syslog");
        assert!(modified);
        assert!(cmd.contains("-n 50"));
        assert!(!cmd.contains("-f"));
    }

    #[test]
    fn test_normalize_watch() {
        let (cmd, modified) = normalize_command("watch -n 1 df -h");
        assert!(modified);
        assert_eq!(cmd, "df -h");
    }

    #[test]
    fn test_detect_prompt() {
        assert!(detect_shell_prompt("output\n$ "));
        assert!(detect_shell_prompt("output\nroot@abc123:/workspace# "));
        assert!(detect_shell_prompt("(sandbox) user@host:~$ "));
        assert!(!detect_shell_prompt("still running..."));
    }
}
