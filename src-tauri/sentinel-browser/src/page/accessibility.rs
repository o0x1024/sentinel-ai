use serde::{Deserialize, Serialize};

use crate::adapter::traits::Rect;

/// A node in the accessibility tree — the primary page representation for AI agents.
/// ~90% smaller than raw HTML while preserving all interactive semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yNode {
    /// Stable element reference ID (e.g. "e1", "e2", ...)
    pub ref_id: String,
    /// ARIA role (button, link, textbox, heading, etc.)
    pub role: String,
    /// Accessible name (visible text / aria-label)
    pub name: String,
    /// Current value (for inputs, sliders, etc.)
    pub value: Option<String>,
    /// State flags (focused, disabled, checked, expanded, selected, required, ...)
    pub state: Vec<String>,
    /// Bounding rectangle in viewport coordinates
    pub bounds: Option<Rect>,
    /// Nesting level for indentation
    pub level: Option<u32>,
    /// Child nodes
    pub children: Vec<A11yNode>,
    /// Whether this node is new since the last snapshot (for tree diffing)
    pub is_new: bool,
}

/// Complete accessibility tree for a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityTree {
    pub url: String,
    pub title: String,
    pub root: A11yNode,
    pub snapshot_id: u64,
}

impl AccessibilityTree {
    /// Render the tree as a token-efficient text representation for LLM consumption
    pub fn to_text(&self) -> String {
        let mut output = format!("Page: {} ({})\n\n", self.title, self.url);
        self.render_node(&self.root, 0, &mut output);
        output
    }

    fn render_node(&self, node: &A11yNode, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        let new_marker = if node.is_new { "*" } else { "" };

        let state_str = if node.state.is_empty() {
            String::new()
        } else {
            format!(" ({})", node.state.join(", "))
        };

        let value_str = match &node.value {
            Some(v) if !v.is_empty() => format!(" value={:?}", v),
            _ => String::new(),
        };

        // Only render nodes that have meaningful content
        if !node.name.is_empty() || !node.role.is_empty() {
            output.push_str(&format!(
                "{}{}[{}] {} {:?}{}{}\n",
                indent, new_marker, node.ref_id, node.role, node.name, value_str, state_str
            ));
        }

        for child in &node.children {
            self.render_node(child, depth + 1, output);
        }
    }

    /// Compute diff between two snapshots, marking new/changed nodes
    pub fn diff(previous: &AccessibilityTree, current: &mut AccessibilityTree) {
        Self::diff_nodes(&previous.root, &mut current.root);
    }

    fn diff_nodes(prev: &A11yNode, curr: &mut A11yNode) {
        // Simple heuristic: match by role+name
        let prev_children_keys: Vec<String> = prev
            .children
            .iter()
            .map(|c| format!("{}:{}", c.role, c.name))
            .collect();

        for child in curr.children.iter_mut() {
            let key = format!("{}:{}", child.role, child.name);
            if !prev_children_keys.contains(&key) {
                child.is_new = true;
                Self::mark_all_new(child);
            } else if let Some(prev_child) = prev
                .children
                .iter()
                .find(|c| format!("{}:{}", c.role, c.name) == key)
            {
                Self::diff_nodes(prev_child, child);
            }
        }
    }

    fn mark_all_new(node: &mut A11yNode) {
        node.is_new = true;
        for child in node.children.iter_mut() {
            Self::mark_all_new(child);
        }
    }
}
