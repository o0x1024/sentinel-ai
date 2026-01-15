//! Setup utilities for agent-browser
//!
//! Handles installation of Playwright browsers and other dependencies

use anyhow::Result;
use std::process::Command;
use tracing::{info, warn};

/// Check if Playwright browsers are installed
pub fn check_playwright_installed() -> bool {
    // Try to find chromium browser
    let home = dirs::home_dir().unwrap_or_default();
    
    #[cfg(target_os = "macos")]
    let browser_path = home.join("Library/Caches/ms-playwright/chromium-*/chrome-mac/Chromium.app");
    
    #[cfg(target_os = "linux")]
    let browser_path = home.join(".cache/ms-playwright/chromium-*/chrome-linux/chrome");
    
    #[cfg(target_os = "windows")]
    let browser_path = home.join("AppData/Local/ms-playwright/chromium-*/chrome-win/chrome.exe");
    
    // Check if any chromium installation exists
    if let Ok(entries) = glob::glob(&browser_path.to_string_lossy()) {
        for entry in entries.flatten() {
            if entry.exists() {
                return true;
            }
        }
    }
    
    false
}

/// Install Playwright browsers
pub fn install_playwright_browsers() -> Result<()> {
    info!("Installing Playwright browsers...");
    
    // Check if npx is available
    let npx_check = Command::new("npx")
        .arg("--version")
        .output();
    
    if npx_check.is_err() {
        warn!("npx not found. Please install Node.js to use browser automation features.");
        anyhow::bail!("Node.js is required for browser automation. Please install from https://nodejs.org/");
    }
    
    // Install chromium browser
    let output = Command::new("npx")
        .arg("playwright")
        .arg("install")
        .arg("chromium")
        .output()?;
    
    if output.status.success() {
        info!("Playwright chromium browser installed successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to install Playwright browsers: {}", stderr)
    }
}

/// Setup agent-browser environment
pub async fn setup_environment() -> Result<()> {
    // Check if browsers are installed
    if !check_playwright_installed() {
        info!("Playwright browsers not found, attempting to install...");
        match install_playwright_browsers() {
            Ok(_) => info!("Browser setup completed successfully"),
            Err(e) => {
                warn!("Failed to auto-install browsers: {}. Browser features may not work.", e);
                warn!("Please run: npx playwright install chromium");
            }
        }
    } else {
        info!("Playwright browsers already installed");
    }
    
    Ok(())
}
