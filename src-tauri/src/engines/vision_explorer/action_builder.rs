//! Action building and validation
//!
//! Builds BrowserAction from VLM analysis results and provides action guards

use super::element_manager::ElementManager;
use super::route_tracker::RouteTracker;
use super::types::{
    BrowserAction, Coordinates, MouseButton, ScrollDirection, VisionExplorerConfig,
    VlmAnalysisResult, VlmNextAction,
};
use anyhow::{anyhow, Result};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Guard next action to prevent stale/interacted index usage
pub async fn guard_next_action(
    analysis: &mut VlmAnalysisResult,
    config: &VisionExplorerConfig,
    element_manager: &RwLock<ElementManager>,
    route_tracker: &RwLock<RouteTracker>,
) {
    let action_type = analysis.next_action.action_type.clone();

    // Text mode: block forbidden actions
    if !config.enable_multimodal {
        let forbidden = [
            "screenshot",
            "click_mouse",
            "move_mouse",
            "computer_click_mouse",
            "computer_screenshot",
        ];
        if forbidden.contains(&action_type.as_str()) {
            warn!(
                "Guard: '{}' is forbidden in text mode, converting to get_elements",
                action_type
            );
            analysis.next_action.action_type = "get_elements".to_string();
            analysis.next_action.element_index = None;
            analysis.next_action.value = None;
            analysis.next_action.reason = format!(
                "Guard: '{}' is not available in text mode (no visual capability), refreshing element list instead",
                action_type
            );
            return;
        }
    }

    let Some(index) = analysis.next_action.element_index else {
        let needs_index = matches!(
            action_type.as_str(),
            "click_by_index" | "fill_by_index" | "hover_by_index"
        );
        if needs_index {
            warn!(
                "Guard: {} requires element_index but none provided",
                action_type
            );
            analysis.next_action.action_type = "get_elements".to_string();
            analysis.next_action.reason = format!(
                "Guard: {} requires element_index, refreshing element list to get valid indices",
                action_type
            );
        }
        return;
    };

    let is_index_action = matches!(
        action_type.as_str(),
        "click_by_index" | "fill_by_index" | "hover_by_index"
    );
    if !is_index_action {
        return;
    }

    let (is_known, is_interacted, max_index, mut uninteracted, mut hover_candidates) = {
        let em = element_manager.read().await;
        let known = em.is_known_index(index);
        let interacted = em.is_interacted_by_index(index);
        let max_idx = em.total_elements() as u32;
        let mut u = em.get_uninteracted_indices();
        let mut h = em.get_hover_candidate_indices();
        u.sort_unstable();
        h.sort_unstable();
        (known, interacted, max_idx, u, h)
    };

    // Index not in current page mapping: force refresh
    if !is_known {
        warn!(
            "Guard: index {} not in current page mapping (max: {})",
            index, max_index
        );
        analysis.next_action.action_type = "get_elements".to_string();
        analysis.next_action.element_index = None;
        analysis.next_action.value = None;
        analysis.next_action.reason = format!(
            "Guard: element index {} is not in current page mapping (valid: 0-{}), refreshing annotated elements",
            index,
            max_index.saturating_sub(1)
        );
        return;
    }

    // Already interacted: handle based on action type
    if is_interacted {
        // For fill_by_index: allow re-filling even if "interacted" (e.g., form correction, re-visiting pages)
        // The VLM chose this specific index for a reason - don't switch to unrelated elements
        if action_type == "fill_by_index" {
            info!(
                "Guard: index {} was previously interacted, but allowing fill_by_index to re-fill (VLM chose this element specifically)",
                index
            );
            // Keep the original action unchanged
            return;
        }

        // For click_by_index and hover_by_index: try to switch to next uninteracted
        info!(
            "Guard: index {} already interacted, finding alternative for {}",
            index, action_type
        );

        if let Some(next_idx) = uninteracted.iter().find(|i| **i != index).cloned() {
            analysis.next_action.element_index = Some(next_idx);
            analysis.next_action.reason = format!(
                "Guard: index {} already interacted, switching to next uninteracted index {}",
                index, next_idx
            );
            return;
        }

        // Try hover candidates (only for non-fill actions)
        if let Some(hover_idx) = hover_candidates.first().cloned() {
            analysis.next_action.action_type = "hover_by_index".to_string();
            analysis.next_action.element_index = Some(hover_idx);
            analysis.next_action.value = None;
            analysis.next_action.reason = format!(
                "Guard: all current indices interacted, trying hover candidate index {} to reveal menus",
                hover_idx
            );
            return;
        }

        // Check for pending routes
        let pending_route = {
            let mut rt = route_tracker.write().await;
            rt.next_pending()
        };

        if let Some(route) = pending_route {
            analysis.next_action.action_type = "navigate".to_string();
            analysis.next_action.element_index = None;
            analysis.next_action.value = Some(route.clone());
            analysis.next_action.reason = format!(
                "Guard: all elements on current page interacted, navigating to pending route: {}",
                route
            );
            return;
        }

        // Last resort: scroll
        analysis.next_action.action_type = "scroll".to_string();
        analysis.next_action.element_index = None;
        analysis.next_action.value = Some("down".to_string());
        analysis.next_action.reason =
            "Guard: all current indices interacted, scrolling down to discover more elements"
                .to_string();
    }
}

/// Build BrowserAction from VLM analysis result
pub fn build_action_from_analysis(
    analysis: &VlmAnalysisResult,
    config: &VisionExplorerConfig,
) -> Result<BrowserAction> {
    let action = &analysis.next_action;

    match action.action_type.as_str() {
        "screenshot" => {
            if !config.enable_multimodal {
                warn!("Text mode: converting screenshot request to get_elements");
                Ok(BrowserAction::GetAnnotatedElements)
            } else {
                Ok(BrowserAction::Screenshot)
            }
        }

        "click_by_index" => {
            if let Some(index) = action.element_index {
                Ok(BrowserAction::ClickByIndex { index })
            } else if let Some(element_id) = &action.element_id {
                if let Ok(index) = element_id.parse::<u32>() {
                    Ok(BrowserAction::ClickByIndex { index })
                } else {
                    Err(anyhow!(
                        "click_by_index requires numeric element_index, got: {}",
                        element_id
                    ))
                }
            } else {
                Err(anyhow!("click_by_index requires element_index"))
            }
        }

        "annotate" | "annotate_elements" => Ok(BrowserAction::AnnotateElements),

        "get_elements" | "get_annotated_elements" => Ok(BrowserAction::GetAnnotatedElements),

        "set_auto_annotation" => {
            let enabled = action
                .value
                .as_deref()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true);
            Ok(BrowserAction::SetAutoAnnotation { enabled })
        }

        "fill_by_index" => {
            if let Some(index) = action.element_index {
                let value = action.value.clone().unwrap_or_default();
                Ok(BrowserAction::FillByIndex { index, value })
            } else if let Some(element_id) = &action.element_id {
                if let Ok(index) = element_id.parse::<u32>() {
                    let value = action.value.clone().unwrap_or_default();
                    Ok(BrowserAction::FillByIndex { index, value })
                } else {
                    Err(anyhow!("fill_by_index requires numeric element_index"))
                }
            } else {
                Err(anyhow!("fill_by_index requires element_index"))
            }
        }

        "hover_by_index" => {
            if let Some(index) = action.element_index {
                Ok(BrowserAction::HoverByIndex { index })
            } else if let Some(element_id) = &action.element_id {
                if let Ok(index) = element_id.parse::<u32>() {
                    Ok(BrowserAction::HoverByIndex { index })
                } else {
                    Err(anyhow!("hover_by_index requires numeric element_index"))
                }
            } else {
                Err(anyhow!("hover_by_index requires element_index"))
            }
        }

        "click" | "click_mouse" | "computer_click_mouse" => {
            if let Some(index) = action.element_index {
                return Ok(BrowserAction::ClickByIndex { index });
            }

            if let Some(element_id) = &action.element_id {
                if let Ok(index) = element_id.parse::<u32>() {
                    return Ok(BrowserAction::ClickByIndex { index });
                }
                if element_id.contains(',') {
                    let parts: Vec<&str> = element_id.split(',').collect();
                    if parts.len() == 2 {
                        let x: i32 = parts[0].trim().parse().unwrap_or(0);
                        let y: i32 = parts[1].trim().parse().unwrap_or(0);
                        return Ok(BrowserAction::ClickMouse {
                            coordinates: Some(Coordinates { x, y }),
                            button: MouseButton::Left,
                            click_count: 1,
                        });
                    }
                }
                Err(anyhow!(
                    "click requires numeric element_index or coordinate format (x,y), got: {}",
                    element_id
                ))
            } else {
                Ok(BrowserAction::ClickMouse {
                    coordinates: None,
                    button: MouseButton::Left,
                    click_count: 1,
                })
            }
        }

        "type" | "type_text" | "computer_type_text" | "fill" => {
            let value = action.value.clone().unwrap_or_default();

            if let Some(index) = action.element_index {
                return Ok(BrowserAction::FillByIndex { index, value });
            }

            if let Some(element_id) = &action.element_id {
                if let Ok(index) = element_id.parse::<u32>() {
                    return Ok(BrowserAction::FillByIndex { index, value });
                }
                Err(anyhow!(
                    "type/fill requires numeric element_index, got: {}",
                    element_id
                ))
            } else {
                Err(anyhow!("type/fill requires element_index"))
            }
        }

        "scroll" | "computer_scroll" => {
            let direction = action
                .value
                .as_deref()
                .map(|v| match v.to_lowercase().as_str() {
                    "up" => ScrollDirection::Up,
                    "left" => ScrollDirection::Left,
                    "right" => ScrollDirection::Right,
                    _ => ScrollDirection::Down,
                })
                .unwrap_or(ScrollDirection::Down);

            Ok(BrowserAction::Scroll {
                coordinates: None,
                direction,
                scroll_count: 3,
            })
        }

        "navigate" | "computer_navigate" => {
            let url = action.value.clone().unwrap_or(config.target_url.clone());
            Ok(BrowserAction::Navigate { url })
        }

        "wait" | "computer_wait" => {
            let duration_ms = action
                .value
                .as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500);
            Ok(BrowserAction::Wait { duration_ms })
        }

        "keys" | "type_keys" | "computer_type_keys" => {
            let keys = action
                .value
                .as_ref()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| vec!["Enter".to_string()]);
            Ok(BrowserAction::TypeKeys { keys })
        }

        "completed" | "done" | "set_exploration_status" | "set_status" => {
            Ok(BrowserAction::Screenshot)
        }

        "needs_help" => Ok(BrowserAction::Screenshot),

        _ => {
            warn!(
                "Unknown action type: {}, defaulting to screenshot",
                action.action_type
            );
            Ok(BrowserAction::Screenshot)
        }
    }
}
