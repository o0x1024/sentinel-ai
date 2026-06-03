//! Sentinel AI - Security Analysis Platform

pub mod agents;
pub mod analyzers;
pub mod app;
pub mod commands;
pub mod engines;
pub mod events;
pub mod generators;
pub mod managers;
pub mod memory;
pub mod models;
pub mod services;
pub mod skills;
pub mod tools;
pub mod trackers;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}

pub(crate) use app::lifecycle::update_proxy_menu_text;
pub use commands::traffic::TrafficAnalysisState;
