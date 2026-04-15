use super::playwright_driver_script::PLAYWRIGHT_DRIVER_SCRIPT;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use uuid::Uuid;

struct BrowserDriverIo {
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

pub struct BrowserDriverHandle {
    child: Mutex<Child>,
    io: Mutex<BrowserDriverIo>,
}

impl BrowserDriverHandle {
    pub async fn spawn(headless: bool) -> Result<Self> {
        let script_path = ensure_driver_script().await?;
        let node_binary = std::env::var("SENTINEL_BROWSER_NODE").unwrap_or_else(|_| "node".into());
        let mut child = Command::new(&node_binary)
            .arg(script_path)
            .arg(if headless { "--headless" } else { "--headed" })
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start Playwright driver with binary '{}'; set SENTINEL_BROWSER_NODE or install node+playwright",
                    node_binary
                )
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture browser driver stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture browser driver stdout"))?;

        Ok(Self {
            child: Mutex::new(child),
            io: Mutex::new(BrowserDriverIo {
                stdin,
                stdout: BufReader::new(stdout).lines(),
            }),
        })
    }

    pub async fn send(&self, command: Value) -> Result<Value> {
        let mut io = self.io.lock().await;
        let payload = serde_json::to_string(&command)?;
        io.stdin.write_all(payload.as_bytes()).await?;
        io.stdin.write_all(b"\n").await?;
        io.stdin.flush().await?;

        let line = io
            .stdout
            .next_line()
            .await?
            .ok_or_else(|| anyhow!("browser driver closed without response"))?;
        let response: Value = serde_json::from_str(&line)
            .with_context(|| format!("invalid browser driver response: {}", line))?;
        if response
            .get("ok")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            Ok(response.get("data").cloned().unwrap_or(Value::Null))
        } else {
            Err(anyhow!(
                "{}",
                response
                    .get("error")
                    .and_then(|value| value.as_str())
                    .unwrap_or("browser driver request failed")
            ))
        }
    }

    pub async fn terminate(&self) {
        let mut child = self.child.lock().await;
        let _ = child.start_kill();
    }
}

async fn ensure_driver_script() -> Result<PathBuf> {
    let base_dir = std::env::temp_dir().join("sentinel-browser-driver");
    fs::create_dir_all(&base_dir).await?;
    let script_path = base_dir.join("playwright_driver.mjs");
    if !Path::new(&script_path).exists() {
        fs::write(&script_path, PLAYWRIGHT_DRIVER_SCRIPT).await?;
    }
    Ok(script_path)
}

pub fn command_payload(
    action: &str,
    headless: bool,
    fields: serde_json::Map<String, Value>,
) -> Value {
    let mut payload = serde_json::Map::new();
    payload.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
    payload.insert("action".to_string(), Value::String(action.to_string()));
    payload.insert("headless".to_string(), Value::Bool(headless));
    payload.extend(fields);
    Value::Object(payload)
}

pub type SharedBrowserDriver = Arc<BrowserDriverHandle>;
