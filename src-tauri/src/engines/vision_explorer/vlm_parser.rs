//! VLM response parsing and action validation
//!
//! Parses VLM JSON responses and validates/fixes actions for text mode

use super::types::{VlmAnalysisResult, VlmNextAction};
use anyhow::{anyhow, Result};
use serde_json::Value;
use tracing::{debug, info, warn};

/// Parse VLM response JSON and extract analysis result
pub fn parse_vlm_response(
    response: &str,
    consecutive_screenshots: u32,
    enable_multimodal: bool,
) -> Result<VlmAnalysisResult> {
    let json_str = extract_json_from_response(response)?;

    debug!("Extracted JSON from VLM response: {}", json_str);

    let parsed: Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "Failed to parse VLM JSON response: {}. Raw JSON: {}",
                e, json_str
            );
            return Err(anyhow!("{}", e));
        }
    };

    let page_analysis = parsed
        .get("page_analysis")
        .and_then(|v| v.as_str())
        .unwrap_or("No analysis provided")
        .to_string();

    let mut next_action = parsed
        .get("next_action")
        .map(|v| VlmNextAction {
            action_type: v
                .get("type")
                .or_else(|| v.get("action_type"))
                .and_then(|t| t.as_str())
                .unwrap_or("screenshot")
                .to_string(),
            element_id: v
                .get("element_id")
                .or_else(|| v.get("selector"))
                .and_then(|e| e.as_str())
                .map(String::from),
            element_index: v
                .get("element_index")
                .or_else(|| v.get("index"))
                .and_then(|e| e.as_u64())
                .map(|n| n as u32),
            value: v.get("value").and_then(|v| v.as_str()).map(String::from),
            reason: v
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("No reason provided")
                .to_string(),
        })
        .unwrap_or(VlmNextAction {
            action_type: "screenshot".to_string(),
            element_id: None,
            element_index: None,
            value: None,
            reason: "Default action".to_string(),
        });

    // Text mode validation
    if !enable_multimodal {
        next_action = validate_text_mode_action(next_action, consecutive_screenshots);
    } else {
        // Detect screenshot loop
        if next_action.action_type == "screenshot" && consecutive_screenshots >= 3 {
            warn!(
                "Detected screenshot loop ({} consecutive), forcing needs_help action",
                consecutive_screenshots
            );
            next_action = VlmNextAction {
                action_type: "needs_help".to_string(),
                element_id: None,
                element_index: None,
                value: None,
                reason: format!(
                    "Stuck in screenshot loop ({} consecutive screenshots). Page state may not be captured correctly.",
                    consecutive_screenshots
                ),
            };
        }
    }

    let estimated_apis: Vec<String> = parsed
        .get("estimated_apis")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let exploration_progress = parsed
        .get("exploration_progress")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;

    let is_exploration_complete = parsed
        .get("is_exploration_complete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || next_action.action_type == "completed"
        || next_action.action_type == "done";

    let completion_reason = parsed
        .get("completion_reason")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(VlmAnalysisResult {
        page_analysis,
        next_action,
        estimated_apis,
        exploration_progress,
        is_exploration_complete,
        completion_reason,
    })
}

/// Validate and fix actions for text mode
pub fn validate_text_mode_action(
    mut action: VlmNextAction,
    consecutive_get_elements: u32,
) -> VlmNextAction {
    let action_type = action.action_type.clone();

    // Forbidden actions in text mode (coordinate-based)
    let forbidden_visual = [
        "screenshot",
        "click_mouse",
        "move_mouse",
        "computer_click_mouse",
        "computer_screenshot",
        "computer_move_mouse",
        "annotate",
        "annotate_elements",
    ];

    if forbidden_visual.contains(&action_type.as_str()) {
        warn!(
            "Text mode: converting forbidden action '{}' to get_elements",
            action_type
        );
        action.action_type = "get_elements".to_string();
        action.element_index = None;
        action.value = None;
        action.reason = format!(
            "Action '{}' requires visual/coordinate capability which is unavailable in text mode. \
             Use index-based actions: click_by_index, fill_by_index, hover_by_index.",
            action_type
        );
        return action;
    }

    // Detect get_elements loop
    if action_type == "get_elements" && consecutive_get_elements >= 2 {
        warn!(
            "Text mode: detected get_elements loop ({} consecutive)",
            consecutive_get_elements
        );

        if consecutive_get_elements == 2 {
            action.action_type = "scroll".to_string();
            action.element_index = None;
            action.value = Some("down".to_string());
            action.reason = format!(
                "Loop detected: {} consecutive get_elements calls. Scrolling down to reveal more content. \
                 If stuck, try: click_by_index on uninteracted elements, or navigate to pending routes.",
                consecutive_get_elements
            );
        } else {
            action.action_type = "navigate".to_string();
            action.element_index = None;
            action.reason = format!(
                "Stuck in get_elements loop ({} consecutive). Consider navigating to a pending route or \
                 clicking on specific elements by index.",
                consecutive_get_elements
            );
        }
        return action;
    }

    // Validate index-based actions have valid index
    let needs_index = matches!(
        action_type.as_str(),
        "click_by_index" | "fill_by_index" | "hover_by_index"
    );
    if needs_index && action.element_index.is_none() {
        let parsed_idx = action.element_id.as_ref().and_then(|id| {
            id.parse::<u32>().ok().or_else(|| {
                id.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .ok()
            })
        });

        if let Some(idx) = parsed_idx {
            action.element_index = Some(idx);
            info!("Text mode: extracted index {} from element_id", idx);
        } else {
            warn!(
                "Text mode: {} missing element_index, converting to get_elements",
                action_type
            );
            action.action_type = "get_elements".to_string();
            action.reason = format!(
                "Action '{}' requires element_index (integer) but none was provided. \
                 Check the Page Elements section for valid indices (e.g., [5] button \"Submit\").",
                action_type
            );
        }
    }

    // Validate fill_by_index has value
    if action.action_type == "fill_by_index" {
        if action.value.is_none() || action.value.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
            info!("Text mode: fill_by_index with empty value - this will clear the field");
            if action.value.is_none() {
                action.value = Some(String::new());
            }
        }
    }

    // Validate scroll has direction
    if action.action_type == "scroll" && action.value.is_none() {
        action.value = Some("down".to_string());
    }

    action
}

/// Extract JSON from VLM response
pub fn extract_json_from_response(response: &str) -> Result<String> {
    // Try to find JSON block
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return Ok(response[start..=end].to_string());
            }
        }
    }

    // Try to find JSON in code block
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return Ok(response[json_start..json_start + end].trim().to_string());
        }
    }

    // Try to find plain code block
    if let Some(start) = response.find("```") {
        let code_start = response[start + 3..]
            .find('\n')
            .map(|i| start + 4 + i)
            .unwrap_or(start + 3);
        if let Some(end) = response[code_start..].find("```") {
            return Ok(response[code_start..code_start + end].trim().to_string());
        }
    }

    Err(anyhow!("No JSON found in response"))
}

