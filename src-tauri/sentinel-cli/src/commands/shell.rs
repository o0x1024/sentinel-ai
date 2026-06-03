use crate::args::{ShellAction, ShellCommand, ShellRunArgs};
use crate::output::print_json;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize)]
struct ShellRunReport {
    command: String,
    cwd: Option<String>,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    completed: bool,
    execution_time_ms: u64,
}

pub async fn run(command: ShellCommand) -> Result<()> {
    match command.action {
        ShellAction::Run(args) => run_command(args).await,
    }
}

async fn run_command(args: ShellRunArgs) -> Result<()> {
    let mut cmd = build_shell_command(&args.command, args.cwd.clone());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let started = std::time::Instant::now();
    let output = timeout(Duration::from_secs(args.timeout_secs), cmd.output())
        .await
        .context("shell command timed out")?
        .context("failed to execute shell command")?;

    print_json(&ShellRunReport {
        command: args.command,
        cwd: args.cwd.map(|value| value.display().to_string()),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
        completed: true,
        execution_time_ms: started.elapsed().as_millis() as u64,
    })
}

fn build_shell_command(command: &str, cwd: Option<PathBuf>) -> Command {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut value = Command::new("cmd");
        value.arg("/C").arg(command);
        value
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut value = if std::path::Path::new("/bin/bash").exists() {
            Command::new("/bin/bash")
        } else {
            Command::new("/bin/sh")
        };
        value.arg("-lc").arg(command);
        value
    };

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd
}
