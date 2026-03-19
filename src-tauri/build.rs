use std::{env, fs, io, path::Path, path::PathBuf, process::Command};

fn copy_dir_recursive(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn stage_local_agent_browser_bundle(
    local_agent_browser_dir: &Path,
    generated_bundle_dir: &Path,
) -> io::Result<()> {
    let local_dist_dir = local_agent_browser_dir.join("dist");
    if !local_dist_dir.exists() {
        return Ok(());
    }

    if generated_bundle_dir.exists() {
        fs::remove_dir_all(generated_bundle_dir)?;
    }

    fs::create_dir_all(generated_bundle_dir.join("node_modules"))?;
    copy_dir_recursive(&local_dist_dir, &generated_bundle_dir.join("dist"))?;

    let local_package_json = local_agent_browser_dir.join("package.json");
    if local_package_json.exists() {
        fs::copy(local_package_json, generated_bundle_dir.join("package.json"))?;
    }

    let local_node_modules = local_agent_browser_dir.join("node_modules");
    if local_node_modules.exists() {
        copy_dir_recursive(&local_node_modules, &generated_bundle_dir.join("node_modules"))?;
    } else {
        fs::write(
            generated_bundle_dir.join("node_modules").join("placeholder.txt"),
            "local-dev-fallback\n",
        )?;
    }

    fs::write(
        generated_bundle_dir.join("UPSTREAM_COMMIT"),
        "repo=local\nref=local\ncommit=local-dev-fallback\n",
    )?;

    Ok(())
}

fn ensure_agent_browser_dist() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing"));
    let generated_bundle_dir = manifest_dir.join("agent-browser-bundle");
    let generated_daemon_js = generated_bundle_dir.join("dist").join("daemon.js");
    let local_agent_browser_dir = manifest_dir.join("agent-browser");
    let local_daemon_js = local_agent_browser_dir.join("dist").join("daemon.js");
    let profile = env::var("PROFILE").unwrap_or_default();

    println!(
        "cargo:rerun-if-changed={}",
        local_agent_browser_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        local_agent_browser_dir.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        local_agent_browser_dir.join("tsconfig.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        generated_bundle_dir.join("dist").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        generated_bundle_dir.join("package.json").display()
    );
    println!("cargo:rerun-if-env-changed=SKIP_AGENT_BROWSER_BUILD");

    if generated_daemon_js.exists() || env::var_os("SKIP_AGENT_BROWSER_BUILD").is_some() {
        return;
    }

    if profile == "release" {
        panic!(
            "agent-browser bundle missing at {}. Run `npm run prepare:agent-browser` before building release artifacts.",
            generated_bundle_dir.display()
        );
    }

    if local_daemon_js.exists() {
        stage_local_agent_browser_bundle(&local_agent_browser_dir, &generated_bundle_dir)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to stage local agent-browser bundle into {}: {}",
                    generated_bundle_dir.display(),
                    err
                )
            });
        return;
    }

    let npm_cmd = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };

    println!(
        "cargo:warning=agent-browser dist missing, running `npm run build` in {}",
        local_agent_browser_dir.display()
    );

    let status = Command::new(npm_cmd)
        .args(["run", "build"])
        .current_dir(&local_agent_browser_dir)
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
            local_agent_browser_dir.display()
        );
    }
}

fn main() {
    ensure_agent_browser_dist();
    tauri_build::build()
}
