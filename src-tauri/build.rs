use std::{env, path::PathBuf};

fn ensure_agent_browser_sidecar_exists() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing"));

    let target_triple = env::var("TAURI_ENV_TARGET_TRIPLE")
        .or_else(|_| env::var("TARGET"))
        .unwrap_or_else(|_| String::from("unknown-target"));

    let sidecar_file_name = if target_triple.contains("windows") {
        format!("sentinel-agent-browser-{}.exe", target_triple)
    } else {
        format!("sentinel-agent-browser-{}", target_triple)
    };

    let sidecar_path = manifest_dir.join("bin").join(sidecar_file_name);

    println!("cargo:rerun-if-changed={}", manifest_dir.join("bin").display());
    println!("cargo:rerun-if-env-changed=TAURI_ENV_TARGET_TRIPLE");

    if !sidecar_path.exists() {
        panic!(
            "agent-browser sidecar missing at {}. Run `npm run setup:agent-browser` in project root.",
            sidecar_path.display()
        );
    }
}

fn main() {
    ensure_agent_browser_sidecar_exists();
    tauri_build::build()
}
