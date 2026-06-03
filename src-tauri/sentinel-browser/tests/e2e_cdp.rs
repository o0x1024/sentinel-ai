//! End-to-end test: launches Chrome, navigates, captures page state & screenshot.
//!
//! Run with: cargo test -p sentinel-browser --test e2e_cdp -- --nocapture
//!
//! Prerequisites: Chrome/Chromium installed at a standard path.
//! The test launches Chrome with a temporary profile and remote debugging port.

use sentinel_browser::adapter::traits::*;
use sentinel_browser::adapter::cdp_adapter::CdpBackend;
use sentinel_browser::lifecycle::discovery::get_browser_version;

use std::time::Duration;

/// Port where Chrome should be running for E2E tests.
/// Start Chrome manually before running:
///   chrome --headless=new --remote-debugging-port=19555 \
///     --remote-debugging-address=127.0.0.1 --no-first-run \
///     --disable-extensions --user-data-dir=/tmp/sentinel-e2e about:blank
const TEST_PORT: u16 = 19555;

#[tokio::test]
async fn test_launch_navigate_screenshot() {
    // Connect to an existing Chrome instance
    let version = match get_browser_version(TEST_PORT).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "SKIP: No Chrome on port {}. Start with:\n  \
                 \"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome\" \
                 --headless=new --remote-debugging-port={} \
                 --remote-debugging-address=127.0.0.1 --no-first-run \
                 --disable-extensions --user-data-dir=/tmp/sentinel-e2e about:blank\n  Error: {}",
                TEST_PORT, TEST_PORT, e
            );
            return;
        }
    };

    let ws_url = version.web_socket_debugger_url;
    println!("Connected to {} at {}", version.browser, ws_url);

    // 3. Connect backend
    let mut backend = CdpBackend::new();
    backend.connect(&ws_url).await.expect("Failed to connect");
    assert!(backend.is_connected());

    // 4. List tabs
    let tabs = backend.list_tabs().await.expect("Failed to list tabs");
    println!("Open tabs: {}", tabs.len());
    assert!(!tabs.is_empty());

    let tab = &tabs[0].id;

    // 5. Navigate to a test page
    let nav_result = backend
        .navigate(tab, "data:text/html,<html><body><h1>Hello Sentinel</h1><button id='btn'>Click Me</button></body></html>")
        .await
        .expect("Failed to navigate");
    println!("Navigation took {}ms", nav_result.load_time_ms);

    // 6. Evaluate JavaScript
    let title = backend
        .evaluate_js(tab, "document.title")
        .await
        .expect("Failed to evaluate JS");
    println!("Page title: {:?}", title);

    let heading = backend
        .evaluate_js(tab, "document.querySelector('h1').textContent")
        .await
        .expect("Failed to get heading");
    assert_eq!(heading.as_str().unwrap(), "Hello Sentinel");

    // 7. Get accessibility tree
    let a11y_tree = backend
        .get_accessibility_tree(tab)
        .await
        .expect("Failed to get a11y tree");
    let tree_text = a11y_tree.to_text();
    println!("Accessibility tree:\n{}", tree_text);

    // 8. Take screenshot
    let screenshot = backend
        .capture_screenshot(tab, ScreenshotOpts {
            full_page: false,
            format: ImageFormat::Png,
            quality: None,
            clip: None,
        })
        .await
        .expect("Failed to capture screenshot");
    println!("Screenshot size: {} bytes", screenshot.len());
    assert!(screenshot.len() > 100); // Valid PNG should be much larger

    // Verify PNG magic bytes
    assert_eq!(&screenshot[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    // 9. Click the button via Input.dispatchMouseEvent
    let coords = backend
        .evaluate_js(tab, r#"(() => {
            const btn = document.querySelector('#btn');
            const rect = btn.getBoundingClientRect();
            return [rect.left + rect.width/2, rect.top + rect.height/2];
        })()"#)
        .await
        .expect("Failed to get button coords");

    let arr = coords.as_array().unwrap();
    let x = arr[0].as_f64().unwrap();
    let y = arr[1].as_f64().unwrap();

    backend
        .dispatch_mouse_event(tab, MouseEvent {
            kind: MouseEventKind::Pressed,
            x,
            y,
            button: MouseButton::Left,
        })
        .await
        .expect("Failed mousePressed");

    backend
        .dispatch_mouse_event(tab, MouseEvent {
            kind: MouseEventKind::Released,
            x,
            y,
            button: MouseButton::Left,
        })
        .await
        .expect("Failed mouseReleased");

    println!("Button clicked at ({}, {})", x, y);

    // 10. Create and close a new tab
    let new_tab = backend
        .create_tab(Some("about:blank"))
        .await
        .expect("Failed to create tab");
    println!("Created tab: {:?}", new_tab);

    let tabs_after = backend.list_tabs().await.expect("Failed to list tabs");
    assert!(tabs_after.len() >= 2);

    backend
        .close_tab(&new_tab)
        .await
        .expect("Failed to close tab");

    // 11. Disconnect
    backend.disconnect().await.expect("Failed to disconnect");
    assert!(!backend.is_connected());

    println!("\n✓ E2E test passed: launch → navigate → JS → a11y → screenshot → click → tabs → close");
}
