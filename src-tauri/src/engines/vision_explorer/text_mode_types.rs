//! Text mode related types
//!
//! Types for text-based page analysis without screenshots

use serde::{Deserialize, Serialize};

/// Page state info from JS script
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageStateInfo {
    #[serde(default)]
    pub has_loading: bool,
    #[serde(default)]
    pub has_empty_state: bool,
    #[serde(default)]
    pub has_pagination: bool,
    #[serde(default)]
    pub scroll_position: f64,
    #[serde(default)]
    pub scroll_height: f64,
    #[serde(default)]
    pub client_height: f64,
    #[serde(default)]
    pub can_scroll_more: bool,
}

/// Visible table info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleTableInfo {
    #[serde(default)]
    pub headers: String,
    #[serde(default)]
    pub rows: usize,
}

/// Visible list info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleListInfo {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub preview: String,
}

/// Visible text and regions info for text mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleTextInfo {
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub nav_items: Vec<String>,
    #[serde(default)]
    pub main_headings: Vec<String>,
    #[serde(default)]
    pub main_content_preview: String,
    #[serde(default)]
    pub sidebar_items: Vec<String>,
    #[serde(default)]
    pub footer_text: String,
    #[serde(default)]
    pub visible_tables: Vec<VisibleTableInfo>,
    #[serde(default)]
    pub visible_lists: Vec<VisibleListInfo>,
    #[serde(default)]
    pub form_labels: Vec<String>,
    #[serde(default)]
    pub alerts_and_messages: Vec<String>,
    #[serde(default)]
    pub page_state: PageStateInfo,
}

/// Form field state info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormFieldState {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub tag: String,
    #[serde(default, rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default, rename = "validationError")]
    pub validation_error: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Lazy load detection info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LazyLoadInfo {
    #[serde(default)]
    pub has_infinite_scroll: bool,
    #[serde(default)]
    pub has_load_more_button: bool,
    #[serde(default)]
    pub has_pagination: bool,
}

/// Enhanced element attributes from JS
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnhancedElementAttributes {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub rect: ElementRect,
    #[serde(default)]
    pub computed_styles: ComputedStyleInfo,
    #[serde(default)]
    pub derived_state: Vec<String>,
    #[serde(default)]
    pub is_occluded: bool,
    #[serde(default)]
    pub inferred_label: Option<String>,
}

/// Element bounding rect
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElementRect {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
}

/// Computed style info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputedStyleInfo {
    #[serde(default)]
    pub color_semantic: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub opacity: f64,
    #[serde(default)]
    pub is_bold: bool,
}

