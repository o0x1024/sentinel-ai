use crate::engines::vision_explorer_v2::core::{Agent, Event, SuggestedAction};
use anyhow::Result;
use async_trait::async_trait;

/// Specialized agent for solving complex navigation patterns.
/// Handles: Hidden Sidebars (Hamburger), Accordions, Nested Menus.
#[derive(Clone)]
pub struct NavigationPatternSolver {
    id: String,
}

impl NavigationPatternSolver {
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Analyze a list of suggested actions and detect navigation patterns
    pub fn detect_patterns(&self, actions: &[SuggestedAction]) -> Vec<SuggestedAction> {
        let mut enhanced_actions = actions.to_vec();

        // 1. Detect Hamburger Menus
        // Look for icons that likely toggle a sidebar (often top-left or top-right)
        for action in enhanced_actions.iter_mut() {
            if self.is_hamburger_menu(&action.description) {
                action.description =
                    format!("[PRIORITY] Expand Navigation: {}", action.description);
                action.confidence += 0.2; // Boost confidence
            }
        }

        // 2. Detect Accordions / Collapsed Items
        // Look for terms like "Expand", "Show", "More", ">", "+"
        for action in enhanced_actions.iter_mut() {
            if self.is_collapsed_item(&action.description) {
                action.description = format!("[EXPLORATION] Expand Item: {}", action.description);
                // We keep confidence stable but mark it for the Planner
            }
        }

        enhanced_actions
    }

    fn is_hamburger_menu(&self, text: &str) -> bool {
        let t = text.to_lowercase();
        t.contains("menu")
            || t.contains("hamburger")
            || t.contains("nav toggle")
            || t.contains("sidebar")
    }

    fn is_collapsed_item(&self, text: &str) -> bool {
        let t = text.to_lowercase();
        t.contains("expand")
            || t.contains("collapse")
            || t.contains("show more")
            || t.contains("arrow")
            || t.contains("chevron")
    }
}

#[async_trait]
impl Agent for NavigationPatternSolver {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn handle_event(&self, _event: &Event) -> Result<()> {
        // This agent is mostly reactive/called directly by Planner or Analyst
        // but could listen for 'Stuck' events to suggest workarounds.
        Ok(())
    }
}
