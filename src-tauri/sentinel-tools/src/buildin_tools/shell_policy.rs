use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommandSemantic {
    ReadOnly,
    Mutating,
    Dangerous(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellCommandAnalysis {
    pub semantic: ShellCommandSemantic,
    pub classification_code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestedAllowRule {
    pub rule: String,
    pub reason_key: String,
}

const CODE_READ_ONLY_SIMPLE_BASE: &str = "read_only.simple_base";
const CODE_READ_ONLY_TEST_COMMAND: &str = "read_only.test_command";
const CODE_READ_ONLY_COMMAND_EXISTS: &str = "read_only.command_exists";
const CODE_READ_ONLY_FIND_SCAN: &str = "read_only.find_scan";
const CODE_READ_ONLY_GIT_INSPECTION: &str = "read_only.git_inspection";
const CODE_MUTATING_WRITE_REDIRECTION: &str = "mutating.write_redirection";
const CODE_MUTATING_UNCLASSIFIED: &str = "mutating.unclassified";
const CODE_DANGEROUS_PRIVILEGE_ESCALATION: &str = "dangerous.privilege_escalation";
const CODE_DANGEROUS_USER_CONTEXT_SWITCH: &str = "dangerous.user_context_switch";

const REASON_KEEP_ORIGINAL_READ_ONLY: &str = "tools.shell.allowRuleReasons.keepOriginalReadOnly";
const REASON_NARROW_READ_ONLY_BASE: &str = "tools.shell.allowRuleReasons.narrowReadOnlyBase";
const REASON_NARROW_TEST_COMMAND: &str = "tools.shell.allowRuleReasons.narrowTestCommand";
const REASON_NARROW_COMMAND_EXISTS: &str = "tools.shell.allowRuleReasons.narrowCommandExists";
const REASON_NARROW_FIND: &str = "tools.shell.allowRuleReasons.narrowFind";
const REASON_NARROW_GIT_INSPECTION: &str = "tools.shell.allowRuleReasons.narrowGitInspection";
const REASON_NARROW_GIT_SUBCOMMAND: &str = "tools.shell.allowRuleReasons.narrowGitSubcommand";
const REASON_NARROW_GIT_REMOTE_SHOW: &str = "tools.shell.allowRuleReasons.narrowGitRemoteShow";
const REASON_NARROW_GIT_REMOTE_GET_URL: &str =
    "tools.shell.allowRuleReasons.narrowGitRemoteGetUrl";
const REASON_NARROW_GIT_STASH_LIST: &str = "tools.shell.allowRuleReasons.narrowGitStashList";
const REASON_NARROW_GIT_STASH_SHOW: &str = "tools.shell.allowRuleReasons.narrowGitStashShow";
const REASON_EXACT_MUTATING_SUBCOMMAND: &str =
    "tools.shell.allowRuleReasons.exactMutatingSubcommand";

const SIMPLE_READ_ONLY_COMMANDS: &[&str] = &[
    "ag",
    "ack",
    "cat",
    "date",
    "du",
    "echo",
    "env",
    "file",
    "grep",
    "head",
    "id",
    "less",
    "ls",
    "more",
    "printenv",
    "ps",
    "pwd",
    "printf",
    "realpath",
    "readlink",
    "rg",
    "stat",
    "tail",
    "tree",
    "uname",
    "wc",
    "whereis",
    "which",
    "whoami",
];

struct DangerousBaseCommand {
    base: &'static str,
    reason_key: &'static str,
    classification_code: &'static str,
}

const DANGEROUS_BASE_COMMANDS: &[DangerousBaseCommand] = &[
    DangerousBaseCommand {
        base: "sudo",
        reason_key: "tools.shell.semanticReasons.dangerousPrivilegeEscalation",
        classification_code: CODE_DANGEROUS_PRIVILEGE_ESCALATION,
    },
    DangerousBaseCommand {
        base: "su",
        reason_key: "tools.shell.semanticReasons.dangerousUserContextSwitch",
        classification_code: CODE_DANGEROUS_USER_CONTEXT_SWITCH,
    },
    DangerousBaseCommand {
        base: "doas",
        reason_key: "tools.shell.semanticReasons.dangerousPrivilegeEscalation",
        classification_code: CODE_DANGEROUS_PRIVILEGE_ESCALATION,
    },
    DangerousBaseCommand {
        base: "pkexec",
        reason_key: "tools.shell.semanticReasons.dangerousPrivilegeEscalation",
        classification_code: CODE_DANGEROUS_PRIVILEGE_ESCALATION,
    },
];

const FIND_MUTATING_TOKENS: &[&str] = &[
    "-delete",
    "-exec",
    "-execdir",
    "-ok",
    "-okdir",
    "-fprint",
    "-fprint0",
    "-fprintf",
    "-fls",
    "-ls",
];

const GIT_BRANCH_MUTATING_FLAGS: &[&str] = &[
    "-c",
    "-C",
    "-d",
    "-D",
    "-m",
    "-M",
    "-u",
    "--copy",
    "--delete",
    "--move",
    "--set-upstream-to",
    "--unset-upstream",
];

const GIT_TAG_MUTATING_FLAGS: &[&str] = &["-a", "-d", "-f", "-m", "-s", "-u", "--delete"];

const GIT_READ_ONLY_SUBCOMMANDS: &[&str] = &[
    "blame",
    "describe",
    "diff",
    "grep",
    "log",
    "rev-parse",
    "show",
    "status",
];

pub fn split_policy_commands(command: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = command.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];

        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }

        if ch == '\\' && !in_single {
            current.push(ch);
            escaped = true;
            index += 1;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            current.push(ch);
            index += 1;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            current.push(ch);
            index += 1;
            continue;
        }

        if !in_single && !in_double {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth = paren_depth.saturating_sub(1),
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                _ => {}
            }

            let at_top_level = paren_depth == 0 && brace_depth == 0 && bracket_depth == 0;
            if at_top_level {
                let next = chars.get(index + 1).copied();
                let is_separator = match (ch, next) {
                    ('&', Some('&')) | ('|', Some('|')) => true,
                    (';', _) | ('|', _) => true,
                    ('&', Some('>')) => false,
                    ('&', _) => true,
                    _ => false,
                };

                if is_separator {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        commands.push(trimmed.to_string());
                    }
                    current.clear();

                    if matches!((ch, next), ('&', Some('&')) | ('|', Some('|'))) {
                        index += 2;
                    } else {
                        index += 1;
                    }
                    continue;
                }
            }
        }

        current.push(ch);
        index += 1;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        commands.push(trimmed.to_string());
    }

    if commands.is_empty() {
        vec![command.trim().to_string()]
    } else {
        commands
    }
}

pub fn classify_shell_command(command: &str) -> ShellCommandSemantic {
    analyze_shell_command(command).semantic
}

pub fn analyze_shell_command(command: &str) -> ShellCommandAnalysis {
    let mut first_read_only_code = CODE_READ_ONLY_SIMPLE_BASE;
    let mut first_mutating_code = CODE_MUTATING_UNCLASSIFIED;
    let mut saw_read_only = false;
    let mut saw_mutating = false;

    for subcommand in split_policy_commands(command) {
        let analysis = analyze_subcommand(&subcommand);
        match analysis.semantic {
            ShellCommandSemantic::ReadOnly => {
                if !saw_read_only {
                    saw_read_only = true;
                    first_read_only_code = analysis.classification_code;
                }
            }
            ShellCommandSemantic::Mutating => {
                if !saw_mutating {
                    saw_mutating = true;
                    first_mutating_code = analysis.classification_code;
                }
            }
            ShellCommandSemantic::Dangerous(_) => return analysis,
        }
    }

    if saw_mutating {
        ShellCommandAnalysis {
            semantic: ShellCommandSemantic::Mutating,
            classification_code: first_mutating_code,
        }
    } else {
        ShellCommandAnalysis {
            semantic: ShellCommandSemantic::ReadOnly,
            classification_code: if saw_read_only {
                first_read_only_code
            } else {
                CODE_READ_ONLY_SIMPLE_BASE
            },
        }
    }
}

pub fn suggest_allow_rules(command: &str) -> Vec<String> {
    suggest_allow_rule_details(command)
        .into_iter()
        .map(|item| item.rule)
        .collect()
}

pub fn suggest_allow_rule_details(command: &str) -> Vec<SuggestedAllowRule> {
    let semantic = classify_shell_command(command);
    if matches!(semantic, ShellCommandSemantic::Dangerous(_)) {
        return Vec::new();
    }

    let rules = split_policy_commands(command)
        .into_iter()
        .map(|subcommand| match semantic {
            ShellCommandSemantic::ReadOnly => build_read_only_rule(&subcommand),
            ShellCommandSemantic::Mutating => SuggestedAllowRule {
                rule: subcommand.trim().to_string(),
                reason_key: REASON_EXACT_MUTATING_SUBCOMMAND.to_string(),
            },
            ShellCommandSemantic::Dangerous(_) => SuggestedAllowRule {
                rule: String::new(),
                reason_key: String::new(),
            },
        })
        .filter(|item| !item.rule.is_empty())
        .collect::<Vec<_>>();

    dedupe_rules(rules)
}

fn analyze_subcommand(command: &str) -> ShellCommandAnalysis {
    if has_write_redirection(command) {
        return ShellCommandAnalysis {
            semantic: ShellCommandSemantic::Mutating,
            classification_code: CODE_MUTATING_WRITE_REDIRECTION,
        };
    }

    let tokens = tokenize_command(command);
    let Some(start_index) = first_command_token_index(&tokens) else {
        return ShellCommandAnalysis {
            semantic: ShellCommandSemantic::Mutating,
            classification_code: CODE_MUTATING_UNCLASSIFIED,
        };
    };

    let tokens = &tokens[start_index..];
    let base = tokens[0].as_str();

    if let Some(dangerous) = DANGEROUS_BASE_COMMANDS.iter().find(|entry| base == entry.base) {
        return ShellCommandAnalysis {
            semantic: ShellCommandSemantic::Dangerous(dangerous.reason_key),
            classification_code: dangerous.classification_code,
        };
    }

    if SIMPLE_READ_ONLY_COMMANDS.contains(&base) {
        return ShellCommandAnalysis {
            semantic: ShellCommandSemantic::ReadOnly,
            classification_code: CODE_READ_ONLY_SIMPLE_BASE,
        };
    }

    match base {
        "[" | "test" => ShellCommandAnalysis {
            semantic: ShellCommandSemantic::ReadOnly,
            classification_code: CODE_READ_ONLY_TEST_COMMAND,
        },
        "command" if matches!(tokens.get(1).map(String::as_str), Some("-v")) => {
            ShellCommandAnalysis {
                semantic: ShellCommandSemantic::ReadOnly,
                classification_code: CODE_READ_ONLY_COMMAND_EXISTS,
            }
        }
        "find" if is_read_only_find(tokens) => ShellCommandAnalysis {
            semantic: ShellCommandSemantic::ReadOnly,
            classification_code: CODE_READ_ONLY_FIND_SCAN,
        },
        "git" if is_read_only_git(tokens) => ShellCommandAnalysis {
            semantic: ShellCommandSemantic::ReadOnly,
            classification_code: CODE_READ_ONLY_GIT_INSPECTION,
        },
        _ => ShellCommandAnalysis {
            semantic: ShellCommandSemantic::Mutating,
            classification_code: CODE_MUTATING_UNCLASSIFIED,
        },
    }
}

fn build_read_only_rule(command: &str) -> SuggestedAllowRule {
    let tokens = tokenize_command(command);
    let Some(start_index) = first_command_token_index(&tokens) else {
        return SuggestedAllowRule {
            rule: command.trim().to_string(),
            reason_key: REASON_KEEP_ORIGINAL_READ_ONLY.to_string(),
        };
    };

    let tokens = &tokens[start_index..];
    let Some(base) = tokens.first().map(String::as_str) else {
        return SuggestedAllowRule {
            rule: command.trim().to_string(),
            reason_key: REASON_KEEP_ORIGINAL_READ_ONLY.to_string(),
        };
    };

    if SIMPLE_READ_ONLY_COMMANDS.contains(&base) {
        return SuggestedAllowRule {
            rule: base.to_string(),
            reason_key: REASON_NARROW_READ_ONLY_BASE.to_string(),
        };
    }

    match base {
        "[" | "test" => SuggestedAllowRule {
            rule: base.to_string(),
            reason_key: REASON_NARROW_TEST_COMMAND.to_string(),
        },
        "command" if matches!(tokens.get(1).map(String::as_str), Some("-v")) => {
            SuggestedAllowRule {
                rule: "command -v".to_string(),
                reason_key: REASON_NARROW_COMMAND_EXISTS.to_string(),
            }
        }
        "find" => SuggestedAllowRule {
            rule: "find".to_string(),
            reason_key: REASON_NARROW_FIND.to_string(),
        },
        "git" => build_git_read_only_rule(tokens),
        _ => SuggestedAllowRule {
            rule: command.trim().to_string(),
            reason_key: REASON_KEEP_ORIGINAL_READ_ONLY.to_string(),
        },
    }
}

fn build_git_read_only_rule(tokens: &[String]) -> SuggestedAllowRule {
    let Some(subcommand) = tokens.get(1).map(String::as_str) else {
        return SuggestedAllowRule {
            rule: "git".to_string(),
            reason_key: REASON_NARROW_GIT_INSPECTION.to_string(),
        };
    };

    if GIT_READ_ONLY_SUBCOMMANDS.contains(&subcommand) {
        return SuggestedAllowRule {
            rule: format!("git {}", subcommand),
            reason_key: REASON_NARROW_GIT_SUBCOMMAND.to_string(),
        };
    }

    match subcommand {
        "remote" => match tokens.get(2).map(String::as_str) {
            Some("show") => SuggestedAllowRule {
                rule: "git remote show".to_string(),
                reason_key: REASON_NARROW_GIT_REMOTE_SHOW.to_string(),
            },
            Some("get-url") => SuggestedAllowRule {
                rule: "git remote get-url".to_string(),
                reason_key: REASON_NARROW_GIT_REMOTE_GET_URL.to_string(),
            },
            _ => SuggestedAllowRule {
                rule: "git".to_string(),
                reason_key: REASON_NARROW_GIT_INSPECTION.to_string(),
            },
        },
        "stash" => match tokens.get(2).map(String::as_str) {
            Some("list") => SuggestedAllowRule {
                rule: "git stash list".to_string(),
                reason_key: REASON_NARROW_GIT_STASH_LIST.to_string(),
            },
            Some("show") => SuggestedAllowRule {
                rule: "git stash show".to_string(),
                reason_key: REASON_NARROW_GIT_STASH_SHOW.to_string(),
            },
            _ => SuggestedAllowRule {
                rule: "git".to_string(),
                reason_key: REASON_NARROW_GIT_INSPECTION.to_string(),
            },
        },
        "branch" | "tag" | "ls-files" | "ls-tree" => SuggestedAllowRule {
            rule: format!("git {}", subcommand),
            reason_key: REASON_NARROW_GIT_SUBCOMMAND.to_string(),
        },
        _ => SuggestedAllowRule {
            rule: "git".to_string(),
            reason_key: REASON_NARROW_GIT_INSPECTION.to_string(),
        },
    }
}

fn first_command_token_index(tokens: &[String]) -> Option<usize> {
    tokens.iter().position(|token| {
        let trimmed = token.trim();
        !trimmed.is_empty()
            && !(trimmed.contains('=') && !trimmed.starts_with('=') && !trimmed.starts_with('-'))
    })
}

fn tokenize_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = command.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in chars {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single {
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }

        if !in_single && !in_double && ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn has_write_redirection(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single {
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }

        if in_single || in_double {
            continue;
        }

        if ch == '>' {
            let prev = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
            if prev != Some('-') {
                return true;
            }
        }
    }

    false
}

fn is_read_only_find(tokens: &[String]) -> bool {
    !tokens
        .iter()
        .skip(1)
        .any(|token| FIND_MUTATING_TOKENS.contains(&token.as_str()))
}

fn is_read_only_git(tokens: &[String]) -> bool {
    let Some(subcommand) = tokens.get(1).map(String::as_str) else {
        return false;
    };

    if GIT_READ_ONLY_SUBCOMMANDS.contains(&subcommand) {
        return true;
    }

    match subcommand {
        "branch" => !tokens
            .iter()
            .skip(2)
            .any(|token| GIT_BRANCH_MUTATING_FLAGS.contains(&token.as_str())),
        "ls-files" | "ls-tree" => true,
        "remote" => matches!(
            tokens.get(2).map(String::as_str),
            None | Some("-v") | Some("show") | Some("get-url")
        ),
        "stash" => matches!(tokens.get(2).map(String::as_str), Some("list") | Some("show")),
        "tag" => {
            !tokens
                .iter()
                .skip(2)
                .any(|token| GIT_TAG_MUTATING_FLAGS.contains(&token.as_str()))
        }
        _ => false,
    }
}

fn dedupe_rules(rules: Vec<SuggestedAllowRule>) -> Vec<SuggestedAllowRule> {
    let mut deduped = Vec::new();
    for rule in rules {
        if !deduped.iter().any(|existing: &SuggestedAllowRule| existing.rule == rule.rule) {
            deduped.push(rule);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_shell_command, classify_shell_command, split_policy_commands,
        suggest_allow_rule_details, suggest_allow_rules, ShellCommandSemantic,
    };

    #[test]
    fn splits_compound_commands_without_touching_quoted_segments() {
        assert_eq!(
            split_policy_commands("git status && npm test; echo done"),
            vec!["git status", "npm test", "echo done"]
        );
        assert_eq!(
            split_policy_commands("python -c \"print('a && b')\" | jq ."),
            vec!["python -c \"print('a && b')\"", "jq ."]
        );
    }

    #[test]
    fn classifies_read_only_commands() {
        assert_eq!(
            classify_shell_command("git status && rg TODO src"),
            ShellCommandSemantic::ReadOnly
        );
        assert_eq!(
            classify_shell_command("find src -name '*.rs'"),
            ShellCommandSemantic::ReadOnly
        );
    }

    #[test]
    fn classifies_mutating_and_dangerous_commands() {
        assert_eq!(
            classify_shell_command("echo test > /tmp/out.txt"),
            ShellCommandSemantic::Mutating
        );
        assert_eq!(
            classify_shell_command("find . -delete"),
            ShellCommandSemantic::Mutating
        );
        assert_eq!(
            classify_shell_command("sudo ls /root"),
            ShellCommandSemantic::Dangerous(
                "tools.shell.semanticReasons.dangerousPrivilegeEscalation"
            )
        );
    }

    #[test]
    fn returns_machine_readable_classification_codes() {
        assert_eq!(
            analyze_shell_command("git status").classification_code,
            "read_only.git_inspection"
        );
        assert_eq!(
            analyze_shell_command("echo hi > /tmp/x").classification_code,
            "mutating.write_redirection"
        );
        assert_eq!(
            analyze_shell_command("sudo ls /root").classification_code,
            "dangerous.privilege_escalation"
        );
    }

    #[test]
    fn suggests_allow_rules_for_read_only_and_mutating_commands() {
        assert_eq!(
            suggest_allow_rules("git status && rg TODO src"),
            vec!["git status", "rg"]
        );
        assert_eq!(
            suggest_allow_rules("npm install && npm run build"),
            vec!["npm install", "npm run build"]
        );
        assert!(suggest_allow_rules("sudo ls /root").is_empty());
        let details = suggest_allow_rule_details("git status && rg TODO src");
        assert_eq!(details[0].rule, "git status");
        assert_eq!(
            details[0].reason_key,
            "tools.shell.allowRuleReasons.narrowGitSubcommand"
        );
    }
}
