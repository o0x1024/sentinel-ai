use std::{env, fs, path::Path};

fn main() {
    generate_app_command_handler();
    tauri_build::build()
}

fn generate_app_command_handler() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let fragment_dir = Path::new(&manifest_dir).join("src/app/commands/fragments");
    let fragments = [
        "common.commands",
        "bounty.commands",
        "traffic.commands",
        "security.commands",
        "workflow.commands",
        "team_v4.commands",
    ];

    let mut commands = String::new();
    for fragment in fragments {
        let path = fragment_dir.join(fragment);
        println!("cargo:rerun-if-changed={}", path.display());
        commands.push_str(
            &fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display())),
        );
    }

    let generated = format!(
        r#"use tauri::generate_handler;

use crate::commands::{{
    self, ai, ai_conversation_binding_support, ai_execution_state_support, ai_turn_logs, aisettings,
    asset, config, database as db_commands, dictionary, llm_test_commands, memory_commands,
    packet_capture_commands, performance, proxifier_commands, rag_commands, scan_session_commands,
    scan_task_commands, tool_commands, traffic, window,
}};

pub fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {{
    generate_handler![
{commands}    ]
}}
"#
    );

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set");
    fs::write(
        Path::new(&out_dir).join("app_command_handler.rs"),
        generated,
    )
    .expect("failed to write generated app command handler");
}
