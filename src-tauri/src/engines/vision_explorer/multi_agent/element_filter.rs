//! Element filtering and intelligent context management
//!
//! Provides smart element filtering to optimize LLM context usage:
//! - Region-based folding (collapse non-active regions)
//! - Deduplication of repeated patterns
//! - Modal isolation (force focus on active overlays)
//! - Priority ranking for exploration

use crate::engines::vision_explorer::types::AnnotatedElement;
use std::collections::HashMap;
use tracing::{debug, info};

/// Element region classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementRegion {
    Header,
    Navigation,
    Sidebar,
    MainContent,
    Footer,
    Modal,
    Unknown,
}

/// Filter configuration for element context optimization
#[derive(Debug, Clone)]
pub struct ElementFilterConfig {
    /// Max elements to include in prompt
    pub max_elements: usize,
    /// Max elements per non-active region (for folding)
    pub max_per_folded_region: usize,
    /// Enable modal isolation (hide non-modal elements when modal is active)
    pub enable_modal_isolation: bool,
    /// Enable pattern deduplication (collapse similar elements)
    pub enable_pattern_dedup: bool,
    /// Max similar items before folding
    pub similar_items_threshold: usize,
    /// Viewport dimensions for region detection
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for ElementFilterConfig {
    fn default() -> Self {
        Self {
            max_elements: 100,
            max_per_folded_region: 5,
            enable_modal_isolation: true,
            enable_pattern_dedup: true,
            similar_items_threshold: 5,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }
}

/// Result of element filtering
#[derive(Debug, Clone)]
pub struct FilteredElements {
    /// Elements to include in prompt
    pub elements: Vec<AnnotatedElement>,
    /// Summary of folded regions
    pub folded_regions: Vec<FoldedRegionSummary>,
    /// Whether modal is active (forces isolation)
    pub modal_active: bool,
    /// Modal close button index if found
    pub modal_close_index: Option<u32>,
    /// Total elements before filtering
    pub total_before: usize,
    /// Total elements after filtering
    pub total_after: usize,
}

/// Summary of a folded region
#[derive(Debug, Clone)]
pub struct FoldedRegionSummary {
    pub region: ElementRegion,
    pub total_count: usize,
    pub sample_texts: Vec<String>,
    pub index_range: (u32, u32),
}

/// Smart element filter for context optimization
pub struct ElementFilter {
    config: ElementFilterConfig,
}

impl ElementFilter {
    pub fn new(config: ElementFilterConfig) -> Self {
        Self { config }
    }

    /// Filter elements for optimal LLM context
    pub fn filter(&self, elements: &[AnnotatedElement]) -> FilteredElements {
        let total_before = elements.len();

        // Step 1: Check for active modal
        let (modal_elements, modal_close_idx) = self.detect_modal_elements(elements);

        if self.config.enable_modal_isolation && !modal_elements.is_empty() {
            // Modal is active - ONLY show modal elements
            let modal_count = modal_elements.len();
            info!(
                "Modal isolation: focusing on {} modal elements, hiding {} background elements",
                modal_count,
                total_before - modal_count
            );

            return FilteredElements {
                elements: modal_elements,
                folded_regions: vec![FoldedRegionSummary {
                    region: ElementRegion::MainContent,
                    total_count: total_before - modal_count,
                    sample_texts: vec![
                        "[Background elements hidden due to active modal]".to_string()
                    ],
                    index_range: (0, 0),
                }],
                modal_active: true,
                modal_close_index: modal_close_idx,
                total_before,
                total_after: modal_count,
            };
        }

        // Step 2: Classify elements by region
        let classified = self.classify_by_region(elements);

        // Step 3: Apply pattern deduplication
        let deduplicated = if self.config.enable_pattern_dedup {
            self.deduplicate_patterns(&classified)
        } else {
            classified
        };

        // Step 4: Fold non-priority regions
        let (filtered, folded) = self.fold_regions(&deduplicated);

        let total_after = filtered.len();
        debug!(
            "Element filter: {} -> {} elements ({} folded regions)",
            total_before,
            total_after,
            folded.len()
        );

        FilteredElements {
            elements: filtered,
            folded_regions: folded,
            modal_active: false,
            modal_close_index: None,
            total_before,
            total_after,
        }
    }

    /// Detect modal elements and close button
    fn detect_modal_elements(
        &self,
        elements: &[AnnotatedElement],
    ) -> (Vec<AnnotatedElement>, Option<u32>) {
        let mut modal_elements = Vec::new();
        let mut close_button_idx: Option<u32> = None;

        for el in elements {
            let is_in_modal = self.is_modal_element(el);

            if is_in_modal {
                modal_elements.push(el.clone());

                // Check if this is a close button
                if close_button_idx.is_none() && self.is_close_button(el) {
                    close_button_idx = Some(el.index);
                }
            }
        }

        (modal_elements, close_button_idx)
    }

    /// Check if element belongs to an active modal
    fn is_modal_element(&self, el: &AnnotatedElement) -> bool {
        // Check aria-modal
        if el
            .attributes
            .get("aria-modal")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            return true;
        }

        // Check role
        if el
            .attributes
            .get("role")
            .map(|v| v == "dialog" || v == "alertdialog")
            .unwrap_or(false)
        {
            return true;
        }

        // Check class names for modal indicators
        if let Some(class) = el.attributes.get("class") {
            let class_lower = class.to_lowercase();
            if class_lower.contains("modal") && !class_lower.contains("modal-backdrop") {
                return true;
            }
            if class_lower.contains("dialog") || class_lower.contains("drawer") {
                return true;
            }
        }

        // Check enhanced attributes for occlusion layer
        if let Some(enhanced) = &el.enhanced_attributes {
            if enhanced.is_in_modal {
                return true;
            }
        }

        false
    }

    /// Check if element is a close button
    fn is_close_button(&self, el: &AnnotatedElement) -> bool {
        let text_lower = el.text.to_lowercase();
        let class = el
            .attributes
            .get("class")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let aria_label = el
            .attributes
            .get("aria-label")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // Text patterns
        if text_lower.contains("close")
            || text_lower.contains("cancel")
            || text_lower == "x"
            || text_lower == "×"
            || text_lower.contains("关闭")
            || text_lower.contains("取消")
        {
            return true;
        }

        // Class patterns
        if class.contains("close") || class.contains("dismiss") {
            return true;
        }

        // Aria label
        if aria_label.contains("close") || aria_label.contains("dismiss") {
            return true;
        }

        false
    }

    /// Classify elements by viewport region
    fn classify_by_region(
        &self,
        elements: &[AnnotatedElement],
    ) -> HashMap<ElementRegion, Vec<AnnotatedElement>> {
        let mut classified: HashMap<ElementRegion, Vec<AnnotatedElement>> = HashMap::new();

        let vw = self.config.viewport_width as f64;
        let vh = self.config.viewport_height as f64;

        let header_threshold = vh * 0.12;
        let sidebar_threshold = vw * 0.22;
        let footer_threshold = vh * 0.88;

        for el in elements {
            let region =
                self.classify_element(el, header_threshold, sidebar_threshold, footer_threshold);
            classified.entry(region).or_default().push(el.clone());
        }

        classified
    }

    /// Classify single element to region
    /// Enhanced: Uses DOM semantic analysis (tags, class, id, data attributes) 
    /// before falling back to position-based classification
    /// Supports non-standard layouts including right sidebars and bottom navigation
    fn classify_element(
        &self,
        el: &AnnotatedElement,
        header_threshold: f64,
        sidebar_threshold: f64,
        footer_threshold: f64,
    ) -> ElementRegion {
        // Check for modal first
        if self.is_modal_element(el) {
            return ElementRegion::Modal;
        }

        // ==== Step 1: Check semantic HTML5 tags ====
        let tag_lower = el.tag_name.to_lowercase();
        match tag_lower.as_str() {
            "nav" => return ElementRegion::Navigation,
            "header" => return ElementRegion::Header,
            "footer" => return ElementRegion::Footer,
            "aside" => return ElementRegion::Sidebar,
            "main" => return ElementRegion::MainContent,
            _ => {}
        }

        // ==== Step 2: Check ARIA role attributes (highest semantic priority) ====
        if let Some(role) = el.attributes.get("role") {
            let role_lower = role.to_lowercase();
            match role_lower.as_str() {
                "navigation" | "menubar" | "menu" | "menuitem" => return ElementRegion::Navigation,
                "main" => return ElementRegion::MainContent,
                "banner" => return ElementRegion::Header,
                "contentinfo" => return ElementRegion::Footer,
                "complementary" => return ElementRegion::Sidebar,
                _ => {}
            }
        }

        // ==== Step 3: Check class names for semantic patterns ====
        if let Some(class) = el.attributes.get("class") {
            let class_lower = class.to_lowercase();
            
            // Navigation patterns (including non-standard positions)
            let nav_patterns = [
                "nav", "menu", "navigation", "navbar", "menubar", "topbar",
                "sidebar", "sidenav", "side-nav", "sidebar-nav", "left-nav", "right-nav",
                "bottom-nav", "footer-nav", "dock", "tab-bar", "tabbar",
                "breadcrumb", "pagination", "stepper",
            ];
            for pattern in nav_patterns {
                if class_lower.contains(pattern) {
                    return ElementRegion::Navigation;
                }
            }

            // Header patterns
            let header_patterns = [
                "header", "top-bar", "topbar", "masthead", "site-header", "page-header",
                "app-header", "layout-header", "banner",
            ];
            for pattern in header_patterns {
                if class_lower.contains(pattern) && !class_lower.contains("nav") {
                    return ElementRegion::Header;
                }
            }

            // Footer patterns 
            let footer_patterns = [
                "footer", "bottom-bar", "bottombar", "site-footer", "page-footer",
                "app-footer", "layout-footer", "copyright",
            ];
            for pattern in footer_patterns {
                if class_lower.contains(pattern) && !class_lower.contains("nav") {
                    return ElementRegion::Footer;
                }
            }

            // Sidebar patterns (including right sidebar)
            let sidebar_patterns = [
                "sidebar", "aside", "side-panel", "drawer", "offcanvas", "off-canvas",
                "leftpanel", "rightpanel", "left-panel", "right-panel",
            ];
            for pattern in sidebar_patterns {
                if class_lower.contains(pattern) {
                    return ElementRegion::Sidebar;
                }
            }

            // Main content patterns
            let main_patterns = [
                "main-content", "content-area", "page-content", "app-content",
                "main-area", "content-wrapper", "layout-content",
            ];
            for pattern in main_patterns {
                if class_lower.contains(pattern) {
                    return ElementRegion::MainContent;
                }
            }
        }

        // ==== Step 4: Check id attribute for semantic patterns ====
        if let Some(id) = el.attributes.get("id") {
            let id_lower = id.to_lowercase();
            
            if id_lower.contains("nav") || id_lower.contains("menu") {
                return ElementRegion::Navigation;
            }
            if id_lower.contains("header") || id_lower.contains("masthead") {
                return ElementRegion::Header;
            }
            if id_lower.contains("footer") {
                return ElementRegion::Footer;
            }
            if id_lower.contains("sidebar") || id_lower.contains("aside") {
                return ElementRegion::Sidebar;
            }
            if id_lower.contains("main") || id_lower.contains("content") {
                return ElementRegion::MainContent;
            }
        }

        // ==== Step 5: Check data attributes ====
        let data_attrs_nav = ["data-nav", "data-menu", "data-navigation"];
        for attr in data_attrs_nav {
            if el.attributes.get(attr).is_some() {
                return ElementRegion::Navigation;
            }
        }

        // ==== Step 6: Position-based classification (fallback) ====
        let y = el.bounding_box.y;
        let x = el.bounding_box.x;
        let vw = self.config.viewport_width as f64;
        let right_sidebar_threshold = vw * 0.78; // Right 22% of viewport

        // Check for fixed/sticky positioning indicators in class
        let is_fixed = el.attributes.get("class")
            .map(|c| c.to_lowercase())
            .map(|c| c.contains("fixed") || c.contains("sticky"))
            .unwrap_or(false);

        if y < header_threshold {
            // Top area - header unless it looks like navigation
            if el.element_type == "link" || el.element_type == "button" {
                if let Some(class) = el.attributes.get("class") {
                    let class_lower = class.to_lowercase();
                    if class_lower.contains("nav") || class_lower.contains("menu") {
                        return ElementRegion::Navigation;
                    }
                }
            }
            ElementRegion::Header
        } else if y > footer_threshold {
            // Bottom area - but check for fixed bottom navigation
            if is_fixed || el.element_type == "link" || el.element_type == "button" {
                if let Some(class) = el.attributes.get("class") {
                    let class_lower = class.to_lowercase();
                    if class_lower.contains("nav") || class_lower.contains("menu") || class_lower.contains("tab") {
                        return ElementRegion::Navigation;
                    }
                }
            }
            ElementRegion::Footer
        } else if x < sidebar_threshold {
            // Left area - sidebar or navigation
            if el.element_type == "link"
                || el.attributes
                    .get("class")
                    .map(|c| {
                        let cl = c.to_lowercase();
                        cl.contains("nav") || cl.contains("menu") || cl.contains("item")
                    })
                    .unwrap_or(false)
            {
                ElementRegion::Navigation
            } else {
                ElementRegion::Sidebar
            }
        } else if x > right_sidebar_threshold {
            // Right area - right sidebar (common in dashboards and documentation sites)
            if el.element_type == "link"
                || el.attributes
                    .get("class")
                    .map(|c| {
                        let cl = c.to_lowercase();
                        cl.contains("nav") || cl.contains("menu") || cl.contains("toc")
                    })
                    .unwrap_or(false)
            {
                ElementRegion::Navigation
            } else {
                ElementRegion::Sidebar
            }
        } else {
            ElementRegion::MainContent
        }
    }

    /// Deduplicate patterns (e.g., table rows, list items)
    fn deduplicate_patterns(
        &self,
        classified: &HashMap<ElementRegion, Vec<AnnotatedElement>>,
    ) -> HashMap<ElementRegion, Vec<AnnotatedElement>> {
        let mut result = HashMap::new();

        for (region, elements) in classified {
            if elements.len() <= self.config.similar_items_threshold {
                result.insert(*region, elements.clone());
                continue;
            }

            // Group by pattern signature
            let patterns = self.detect_patterns(elements);
            let mut deduped = Vec::new();

            for (pattern, group) in patterns {
                let group_len = group.len();
                if group_len <= self.config.similar_items_threshold {
                    deduped.extend(group);
                } else {
                    // Keep first few, mark rest as folded
                    deduped.extend(group.into_iter().take(self.config.similar_items_threshold));
                    debug!(
                        "Pattern dedup: folded {} similar items with pattern '{}'",
                        group_len - self.config.similar_items_threshold,
                        pattern
                    );
                }
            }

            result.insert(*region, deduped);
        }

        result
    }

    /// Detect repeating patterns in elements
    fn detect_patterns(
        &self,
        elements: &[AnnotatedElement],
    ) -> HashMap<String, Vec<AnnotatedElement>> {
        let mut patterns: HashMap<String, Vec<AnnotatedElement>> = HashMap::new();

        for el in elements {
            let sig = self.element_pattern_signature(el);
            patterns.entry(sig).or_default().push(el.clone());
        }

        patterns
    }

    /// Generate pattern signature for element
    fn element_pattern_signature(&self, el: &AnnotatedElement) -> String {
        // Signature based on tag, type, and class prefix
        let class_prefix = el
            .attributes
            .get("class")
            .map(|c| c.split_whitespace().next().unwrap_or(""))
            .unwrap_or("");

        format!("{}:{}:{}", el.tag_name, el.element_type, class_prefix)
    }

    /// Fold non-priority regions
    fn fold_regions(
        &self,
        classified: &HashMap<ElementRegion, Vec<AnnotatedElement>>,
    ) -> (Vec<AnnotatedElement>, Vec<FoldedRegionSummary>) {
        let mut result = Vec::new();
        let mut folded = Vec::new();

        // Priority order: MainContent > Modal > Navigation > Header > Sidebar > Footer
        let priority_order = [
            ElementRegion::Modal,
            ElementRegion::MainContent,
            ElementRegion::Navigation,
            ElementRegion::Header,
            ElementRegion::Sidebar,
            ElementRegion::Footer,
        ];

        let mut remaining_budget = self.config.max_elements;

        for region in priority_order {
            if let Some(elements) = classified.get(&region) {
                let is_priority =
                    matches!(region, ElementRegion::MainContent | ElementRegion::Modal);

                if is_priority {
                    // Priority regions get full allocation
                    let take = elements.len().min(remaining_budget);
                    result.extend(elements.iter().take(take).cloned());
                    remaining_budget = remaining_budget.saturating_sub(take);

                    if elements.len() > take {
                        folded.push(self.create_fold_summary(region, elements, take));
                    }
                } else {
                    // Non-priority regions get limited allocation
                    let take = elements
                        .len()
                        .min(self.config.max_per_folded_region)
                        .min(remaining_budget);
                    result.extend(elements.iter().take(take).cloned());
                    remaining_budget = remaining_budget.saturating_sub(take);

                    if elements.len() > take {
                        folded.push(self.create_fold_summary(region, elements, take));
                    }
                }
            }
        }

        // Handle Unknown region
        if let Some(elements) = classified.get(&ElementRegion::Unknown) {
            let take = elements.len().min(remaining_budget);
            result.extend(elements.iter().take(take).cloned());
            if elements.len() > take {
                folded.push(self.create_fold_summary(ElementRegion::Unknown, elements, take));
            }
        }

        (result, folded)
    }

    /// Create fold summary for region
    fn create_fold_summary(
        &self,
        region: ElementRegion,
        elements: &[AnnotatedElement],
        shown_count: usize,
    ) -> FoldedRegionSummary {
        let sample_texts: Vec<String> = elements
            .iter()
            .skip(shown_count)
            .take(3)
            .map(|e| truncate(&e.text, 30))
            .filter(|s| !s.is_empty())
            .collect();

        let indices: Vec<u32> = elements.iter().map(|e| e.index).collect();
        let min_idx = indices.iter().min().copied().unwrap_or(0);
        let max_idx = indices.iter().max().copied().unwrap_or(0);

        FoldedRegionSummary {
            region,
            total_count: elements.len() - shown_count,
            sample_texts,
            index_range: (min_idx, max_idx),
        }
    }
}

/// Format filtered elements for prompt
pub fn format_filtered_for_prompt(filtered: &FilteredElements) -> String {
    let mut output = String::new();

    // Modal warning
    if filtered.modal_active {
        output.push_str("⚠️ ACTIVE MODAL DETECTED - Only showing modal elements\n");
        if let Some(close_idx) = filtered.modal_close_index {
            output.push_str(&format!("→ Close button at index [{}]\n", close_idx));
        }
        output.push_str(
            "→ IMPORTANT: Interact with modal first before accessing background elements!\n\n",
        );
    }

    // Elements
    output.push_str(&format!(
        "## Interactive Elements ({} shown)\n",
        filtered.elements.len()
    ));
    for el in &filtered.elements {
        let text = truncate(&el.text, 35);
        let mut extras = Vec::new();

        if let Some(href) = el.attributes.get("href") {
            if !href.is_empty() && !href.starts_with("javascript:") {
                extras.push(format!("→{}", truncate(href, 30)));
            }
        }

        if let Some(placeholder) = el.attributes.get("placeholder") {
            if !placeholder.is_empty() && text.is_empty() {
                extras.push(format!("ph:{}", truncate(placeholder, 20)));
            }
        }

        let extras_str = if extras.is_empty() {
            String::new()
        } else {
            format!(" [{}]", extras.join(" "))
        };

        output.push_str(&format!(
            "[{}] {} \"{}\"{}",
            el.index, el.element_type, text, extras_str
        ));

        output.push('\n');
    }

    // Folded regions summary
    if !filtered.folded_regions.is_empty() {
        output.push_str("\n## Folded Regions (use indices to access)\n");
        for fold in &filtered.folded_regions {
            let region_name = format!("{:?}", fold.region);
            let samples = if fold.sample_texts.is_empty() {
                String::new()
            } else {
                format!(" e.g.: {}", fold.sample_texts.join(", "))
            };
            output.push_str(&format!(
                "- {} ({} more elements, indices {}-{}){}\n",
                region_name, fold.total_count, fold.index_range.0, fold.index_range.1, samples
            ));
        }
    }

    output
}

/// Truncate string
fn truncate(s: &str, max_len: usize) -> String {
    let s = s.trim().replace('\n', " ");
    if s.chars().count() <= max_len {
        s
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::vision_explorer::types::BoundingBox;

    fn make_element(index: u32, el_type: &str, text: &str, y: f64) -> AnnotatedElement {
        AnnotatedElement {
            index,
            element_type: el_type.to_string(),
            tag_name: "div".to_string(),
            text: text.to_string(),
            selector: format!("#el-{}", index),
            bounding_box: BoundingBox {
                x: 100.0,
                y,
                width: 100.0,
                height: 30.0,
            },
            attributes: HashMap::new(),
            enhanced_attributes: None,
        }
    }

    #[test]
    fn test_region_classification() {
        let filter = ElementFilter::new(ElementFilterConfig::default());

        let elements = vec![
            make_element(0, "link", "Home", 10.0),       // Header
            make_element(1, "link", "Products", 20.0),   // Header
            make_element(2, "button", "Submit", 400.0),  // Main
            make_element(3, "input", "Search", 450.0),   // Main
            make_element(4, "link", "Copyright", 700.0), // Footer
        ];

        let result = filter.filter(&elements);
        assert!(result.elements.len() <= 100);
        assert!(!result.modal_active);
    }
}
