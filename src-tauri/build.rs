use std::{env, path::PathBuf, process::Command};

fn ensure_agent_browser_dist() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing"));
    let agent_browser_dir = manifest_dir.join("agent-browser");
    let daemon_js = agent_browser_dir.join("dist").join("daemon.js");

    println!(
        "cargo:rerun-if-changed={}",
        agent_browser_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        agent_browser_dir.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        agent_browser_dir.join("tsconfig.json").display()
    );
    println!("cargo:rerun-if-env-changed=SKIP_AGENT_BROWSER_BUILD");

    if daemon_js.exists() || env::var_os("SKIP_AGENT_BROWSER_BUILD").is_some() {
        return;
    }

    let npm_cmd = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };

    println!(
        "cargo:warning=agent-browser dist missing, running `npm run build` in {}",
        agent_browser_dir.display()
    );

    let status = Command::new(npm_cmd)
        .args(["run", "build"])
        .current_dir(&agent_browser_dir)
        .status()
        .unwrap_or_else(|err| {
            panic!(
                "Failed to execute `{}` for agent-browser build: {}",
                npm_cmd, err
            )
        });

    if !status.success() {
        panic!(
            "agent-browser build failed (exit code: {:?}). Run `npm run build` in {}",
            status.code(),
            agent_browser_dir.display()
        );
    }
}

fn main() {
    ensure_agent_browser_dist();
    tauri_build::build()
}
