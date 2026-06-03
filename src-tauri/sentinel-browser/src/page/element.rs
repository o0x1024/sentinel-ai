use crate::adapter::traits::{Point, Rect};

/// Strategy for locating an element on the page
#[derive(Debug, Clone)]
pub enum ElementSelector {
    /// By accessibility tree ref_id (preferred — stable and AI-friendly)
    RefId(String),
    /// By CSS selector (fallback)
    Css(String),
    /// By XPath
    XPath(String),
    /// By visible text content
    Text(String),
    /// By exact viewport coordinates
    Coordinates(Point),
}

/// Resolved element with all info needed for interaction
#[derive(Debug, Clone)]
pub struct ResolvedElement {
    pub selector: ElementSelector,
    pub bounds: Rect,
    pub is_visible: bool,
    pub is_in_viewport: bool,
    pub is_interactable: bool,
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
}

impl ResolvedElement {
    /// Get a randomized click point within the element bounds (not always center)
    pub fn random_click_point(&self) -> Point {
        let mut rng = rand::thread_rng();
        use rand::Rng;

        let padding = 0.2; // stay 20% away from edges
        let x_range = self.bounds.width * (1.0 - 2.0 * padding);
        let y_range = self.bounds.height * (1.0 - 2.0 * padding);

        Point {
            x: self.bounds.x + self.bounds.width * padding + rng.gen::<f64>() * x_range,
            y: self.bounds.y + self.bounds.height * padding + rng.gen::<f64>() * y_range,
        }
    }

    pub fn center(&self) -> Point {
        self.bounds.center()
    }

    /// Approximate target "size" for Fitts' Law calculation
    pub fn target_size(&self) -> f64 {
        self.bounds.width.min(self.bounds.height)
    }
}
