//! Playwright MCP bridge
//!
//! Tool call conversion between BrowserAction and MCP tool calls

use super::types::{BrowserAction, Coordinates, MouseButton, ScrollDirection};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// Convert BrowserAction to tool call format
pub fn action_to_tool_call(action: &BrowserAction) -> (String, Value) {
    match action {
        BrowserAction::Screenshot => ("computer_screenshot".to_string(), json!({})),
        BrowserAction::MoveMouse { coordinates } => (
            "computer_move_mouse".to_string(),
            json!({ "coordinates": { "x": coordinates.x, "y": coordinates.y } }),
        ),
        BrowserAction::ClickMouse {
            coordinates,
            button,
            click_count,
        } => (
            "computer_click_mouse".to_string(),
            json!({
                "coordinates": coordinates.as_ref().map(|c| json!({ "x": c.x, "y": c.y })),
                "button": match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                },
                "click_count": click_count
            }),
        ),
        BrowserAction::Scroll {
            coordinates,
            direction,
            scroll_count,
        } => (
            "computer_scroll".to_string(),
            json!({
                "coordinates": coordinates.as_ref().map(|c| json!({ "x": c.x, "y": c.y })),
                "direction": match direction {
                    ScrollDirection::Up => "up",
                    ScrollDirection::Down => "down",
                    ScrollDirection::Left => "left",
                    ScrollDirection::Right => "right",
                },
                "scroll_count": scroll_count
            }),
        ),
        BrowserAction::TypeKeys { keys } => {
            ("computer_type_keys".to_string(), json!({ "keys": keys }))
        }
        BrowserAction::Wait { duration_ms } => (
            "computer_wait".to_string(),
            json!({ "duration_ms": duration_ms }),
        ),
        BrowserAction::Navigate { url } => ("computer_navigate".to_string(), json!({ "url": url })),
        BrowserAction::SelectOption { selector, value } => (
            "computer_select_option".to_string(),
            json!({ "selector": selector, "value": value }),
        ),
        BrowserAction::AnnotateElements => ("playwright_annotate".to_string(), json!({})),
        BrowserAction::ClickByIndex { index } => (
            "playwright_click_by_index".to_string(),
            json!({ "index": index }),
        ),
        BrowserAction::SetAutoAnnotation { enabled } => (
            "playwright_set_auto_annotation".to_string(),
            json!({ "enabled": enabled }),
        ),
        BrowserAction::GetAnnotatedElements => {
            ("playwright_get_annotated_elements".to_string(), json!({}))
        }
        BrowserAction::FillByIndex { index, value } => (
            "playwright_fill_by_index".to_string(),
            json!({ "index": index, "value": value }),
        ),
        BrowserAction::HoverByIndex { index } => {
            ("hover_by_index".to_string(), json!({ "index": index }))
        }
    }
}

/// Parse tool call to BrowserAction
pub fn parse_tool_call_to_action(tool_name: &str, params: &Value) -> Result<BrowserAction> {
    match tool_name {
        "computer_screenshot" => Ok(BrowserAction::Screenshot),

        "computer_move_mouse" => {
            let coords = params
                .get("coordinates")
                .ok_or_else(|| anyhow!("Missing coordinates"))?;
            Ok(BrowserAction::MoveMouse {
                coordinates: Coordinates {
                    x: coords.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    y: coords.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                },
            })
        }

        "computer_click_mouse" => {
            let coords = params.get("coordinates").map(|c| Coordinates {
                x: c.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: c.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            });

            let button = match params
                .get("button")
                .and_then(|b| b.as_str())
                .unwrap_or("left")
            {
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };

            let click_count = params
                .get("click_count")
                .and_then(|c| c.as_u64())
                .unwrap_or(1) as u32;

            Ok(BrowserAction::ClickMouse {
                coordinates: coords,
                button,
                click_count,
            })
        }

        "computer_scroll" => {
            let coords = params.get("coordinates").map(|c| Coordinates {
                x: c.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: c.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            });

            let direction = match params
                .get("direction")
                .and_then(|d| d.as_str())
                .unwrap_or("down")
            {
                "up" => ScrollDirection::Up,
                "left" => ScrollDirection::Left,
                "right" => ScrollDirection::Right,
                _ => ScrollDirection::Down,
            };

            let scroll_count = params
                .get("scroll_count")
                .and_then(|c| c.as_u64())
                .unwrap_or(3) as u32;

            Ok(BrowserAction::Scroll {
                coordinates: coords,
                direction,
                scroll_count,
            })
        }

        "computer_type_keys" => {
            let keys = params
                .get("keys")
                .and_then(|k| k.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Ok(BrowserAction::TypeKeys { keys })
        }

        "computer_wait" => {
            let duration_ms = params
                .get("duration_ms")
                .and_then(|d| d.as_u64())
                .unwrap_or(500);
            Ok(BrowserAction::Wait { duration_ms })
        }

        "computer_navigate" => {
            let url = params
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            Ok(BrowserAction::Navigate { url })
        }

        "playwright_annotate" | "annotate" | "annotate_elements" => {
            Ok(BrowserAction::AnnotateElements)
        }

        "playwright_click_by_index" | "click_by_index" => {
            let index = params.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
            Ok(BrowserAction::ClickByIndex { index })
        }

        "playwright_fill_by_index" | "fill_by_index" => {
            let index = params.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
            let value = params
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(BrowserAction::FillByIndex { index, value })
        }

        "playwright_set_auto_annotation" | "set_auto_annotation" => {
            let enabled = params
                .get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(true);
            Ok(BrowserAction::SetAutoAnnotation { enabled })
        }

        "playwright_get_annotated_elements" | "get_annotated_elements" | "get_elements" => {
            Ok(BrowserAction::GetAnnotatedElements)
        }

        "playwright_hover_by_index" | "hover_by_index" => {
            let index = params.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
            Ok(BrowserAction::HoverByIndex { index })
        }

        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}

/// Extract base64 data from JSON value
/// Supports multiple response formats
pub fn extract_base64_from_value(value: &Value) -> Option<String> {
    // Check screenshot_base64 field (MCP playwright format)
    if let Some(base64) = value.get("screenshot_base64").and_then(|v| v.as_str()) {
        if base64.len() > 100 {
            return Some(base64.to_string());
        }
    }

    // Direct string
    if let Some(s) = value.as_str() {
        if s.len() > 100
            && s.chars()
                .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
            return Some(s.to_string());
        }
        if let Some(base64) = extract_base64_from_raw_image_content(s) {
            return Some(base64);
        }
    }

    // { "content": [{ "type": "image", "data": "base64..." }] }
    if let Some(content) = value.get("content") {
        if let Some(arr) = content.as_array() {
            for item in arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("image") {
                    if let Some(data) = item.get("data").and_then(|d| d.as_str()) {
                        return Some(data.to_string());
                    }
                }
                if let Some(source) = item.get("source") {
                    if let Some(data) = source.get("data").and_then(|d| d.as_str()) {
                        return Some(data.to_string());
                    }
                }
            }
        }
        if let Some(data) = content.as_str() {
            if data.len() > 100 {
                return Some(data.to_string());
            }
        }
    }

    // Extract from output field
    if let Some(output) = value.get("output").and_then(|v| v.as_str()) {
        if let Ok(json) = serde_json::from_str::<Value>(output) {
            if let Some(base64) = json.get("screenshot_base64").and_then(|v| v.as_str()) {
                if base64.len() > 100 {
                    return Some(base64.to_string());
                }
            }
        }
        if let Some(base64) = extract_base64_from_raw_image_content(output) {
            return Some(base64);
        }
    }

    // Common field names
    for key in &["base64", "data", "image", "screenshot", "result"] {
        if let Some(val) = value.get(*key) {
            if let Some(s) = val.as_str() {
                if s.len() > 100 {
                    return Some(s.to_string());
                }
            }
            if let Some(found) = extract_base64_from_value(val) {
                return Some(found);
            }
        }
    }

    None
}

/// Extract base64 data from RawImageContent format string
fn extract_base64_from_raw_image_content(s: &str) -> Option<String> {
    let data_marker = "data: \"";
    let start_idx = s.find(data_marker)?;
    let data_start = start_idx + data_marker.len();

    let remaining = &s[data_start..];
    let end_idx = remaining.find('"')?;

    let base64_data = &remaining[..end_idx];
    if base64_data.len() > 100 {
        Some(base64_data.to_string())
    } else {
        None
    }
}

