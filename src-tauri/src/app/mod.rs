mod commands;
pub(crate) mod lifecycle;
mod state;

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;

fn archive_old_logs(logs_dir: &Path) {
    if let Ok(entries) = fs::read_dir(logs_dir) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // 匹配 sentinel-ai.log.YYYY-MM-DD 这种由 tracing-appender 滚动生成的文件
                    if file_name.starts_with("sentinel-ai.log.")
                        && !file_name.contains(&today)
                        && !file_name.ends_with(".gz")
                    {
                        let mut archive_path = path.clone();
                        archive_path.set_extension("gz");

                        if let Ok(log_file) = fs::File::open(&path) {
                            if let Ok(archive_file) = fs::File::create(&archive_path) {
                                let mut reader = BufReader::new(log_file);
                                let mut encoder =
                                    GzEncoder::new(archive_file, Compression::default());
                                if std::io::copy(&mut reader, &mut encoder).is_ok() {
                                    let _ = fs::remove_file(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run() {
    // Install rustls CryptoProvider (required for rustls 0.23+)
    // Must be called before any rustls usage
    let _ = rustls::crypto::ring::default_provider().install_default();

    let context = tauri::generate_context!();

    // Configure log directory:
    // - Debug: project working directory `logs/`
    // - Release: system data directory `<data_dir>/sentinel-ai/logs`
    #[cfg(debug_assertions)]
    let logs_dir = {
        let path = PathBuf::from("logs");
        let _ = fs::create_dir_all(&path);
        path
    };

    #[cfg(not(debug_assertions))]
    let logs_dir = {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("logs");
        let _ = fs::create_dir_all(&path);
        path
    };
    let logs_dir_path = logs_dir.clone();
    let logs_dir = logs_dir.to_string_lossy().to_string();

    // 启动前清理旧日志进行压缩
    archive_old_logs(&logs_dir_path);

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "sentinel-ai.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("sentinel_ai=info".parse().unwrap())
        .add_directive("sentinel_plugins=info".parse().unwrap())
        .add_directive("sentinel_workflow=info".parse().unwrap())
        .add_directive("sentinel_traffic=info".parse().unwrap())
        .add_directive("sentinel_rag=info".parse().unwrap())
        .add_directive("sentinel_llm=info".parse().unwrap())
        .add_directive("hudsucker=off".parse().unwrap())
        .add_directive(
            "rig::agent::prompt_request::streaming=warn"
                .parse()
                .unwrap(),
        );

    // let rig_debug = std::env::var("SENTINEL_RIG_DEBUG")
    //     .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
    //     .unwrap_or(false);
    // if rig_debug {
    //     env_filter = env_filter
    //         .add_directive("sentinel_llm=debug".parse().unwrap())
    //         .add_directive("rig=debug".parse().unwrap())
    //         .add_directive(
    //             "rig::agent::prompt_request::streaming=debug"
    //                 .parse()
    //                 .unwrap(),
    //         );
    // }

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_line_number(true)
        .with_ansi(false)
        .init();

    std::mem::forget(_guard);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(lifecycle::handle_window_event)
        .setup(state::setup_app)
        .invoke_handler(self::commands::handler())
        .run(context)
        .expect("Failed to start Tauri application");
}
