//! 元素管理器
//!
//! 跨页面管理所有可交互元素，生成元素指纹用于去重和覆盖率计算

use super::types::AnnotatedElement;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

/// 元素指纹信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElementFingerprint {
    /// 指纹哈希
    pub fingerprint: String,
    /// 元素类型
    pub element_type: String,
    /// 元素文本（截断）
    pub text: String,
    /// 所在页面
    pub page_url: String,
    /// 原始索引
    pub original_index: u32,
    /// 是否是悬停候选
    pub is_hover_candidate: bool,
}

/// 动态组件信息（模态框、弹窗等）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicComponent {
    /// 组件类型
    pub component_type: String,
    /// 选择器
    pub selector: String,
    /// 触发元素指纹
    pub trigger_fingerprint: Option<String>,
    /// 是否已探索
    pub explored: bool,
}

/// 元素管理器
#[derive(Debug, Clone)]
pub struct ElementManager {
    /// 所有唯一元素 (fingerprint -> ElementFingerprint)
    all_elements: HashMap<String, ElementFingerprint>,
    /// 已交互的元素指纹
    interacted_elements: HashSet<String>,
    /// 当前页面的元素映射 (index -> fingerprint)
    current_page_mapping: HashMap<u32, String>,
    /// 需要悬停探测的元素指纹
    hover_candidates: Vec<String>,
    /// 动态发现的组件
    dynamic_components: Vec<DynamicComponent>,
    /// 当前页面 URL
    current_page_url: String,
}

impl ElementManager {
    /// 创建新的元素管理器
    pub fn new() -> Self {
        Self {
            all_elements: HashMap::new(),
            interacted_elements: HashSet::new(),
            current_page_mapping: HashMap::new(),
            hover_candidates: Vec::new(),
            dynamic_components: Vec::new(),
            current_page_url: String::new(),
        }
    }

    /// Generate element fingerprint using stable identifiers
    /// Priority: id > data-testid > name+type > selector+text
    fn generate_fingerprint(element: &AnnotatedElement, page_url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Include normalized page path (without query/hash)
        let normalized_url = Self::normalize_url_for_fingerprint(page_url);
        normalized_url.hash(&mut hasher);

        // Priority 1: Use stable id attribute (skip auto-generated ids)
        if let Some(id) = element.attributes.get("id") {
            if Self::is_stable_id(id) {
                id.hash(&mut hasher);
                element.element_type.hash(&mut hasher);
                return format!("{:016x}", hasher.finish());
            }
        }

        // Priority 2: Use data-testid (common in React/Vue apps)
        if let Some(testid) = element
            .attributes
            .get("data-testid")
            .or_else(|| element.attributes.get("data-test-id"))
            .or_else(|| element.attributes.get("data-cy"))
        {
            testid.hash(&mut hasher);
            element.element_type.hash(&mut hasher);
            return format!("{:016x}", hasher.finish());
        }

        // Priority 3: Use name + type for form elements
        if let Some(name) = element.attributes.get("name") {
            if !name.is_empty() {
                name.hash(&mut hasher);
                element.element_type.hash(&mut hasher);
                if let Some(input_type) = element.attributes.get("type") {
                    input_type.hash(&mut hasher);
                }
                return format!("{:016x}", hasher.finish());
            }
        }

        // Priority 4: Use href for links (normalized)
        if let Some(href) = element.attributes.get("href") {
            if !href.is_empty() && !href.starts_with("javascript:") && !href.starts_with("#") {
                Self::normalize_href(href).hash(&mut hasher);
                element.element_type.hash(&mut hasher);
                element
                    .text
                    .chars()
                    .take(15)
                    .collect::<String>()
                    .hash(&mut hasher);
                return format!("{:016x}", hasher.finish());
            }
        }

        // Priority 5: Use aria-label for semantic elements
        if let Some(aria_label) = element.attributes.get("aria-label") {
            if !aria_label.is_empty() {
                aria_label.hash(&mut hasher);
                element.element_type.hash(&mut hasher);
                return format!("{:016x}", hasher.finish());
            }
        }

        // Fallback: Use type + text + role (avoid selector which may be unstable)
        element.element_type.hash(&mut hasher);
        element
            .text
            .chars()
            .take(30)
            .collect::<String>()
            .to_lowercase()
            .hash(&mut hasher);
        if let Some(role) = element.attributes.get("role") {
            role.hash(&mut hasher);
        }
        // Include tag_name for disambiguation
        element.tag_name.to_lowercase().hash(&mut hasher);

        format!("{:016x}", hasher.finish())
    }

    /// Check if an id is stable (not auto-generated by frameworks)
    fn is_stable_id(id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        // Skip common auto-generated id patterns
        let auto_patterns = [
            "react-",
            "vue-",
            "ng-",
            "ember-",
            "__",
            "rc-",
            "ant-",
            "el-",
            "arco-",
            "semi-",
            "chakra-",
            "radix-",
            "headlessui-",
            "mui-",
            "mantine-",
        ];
        let id_lower = id.to_lowercase();
        if auto_patterns.iter().any(|p| id_lower.starts_with(p)) {
            return false;
        }
        // Skip ids that look like random hashes (e.g., "a1b2c3d4")
        if id.len() >= 6
            && id.chars().all(|c| c.is_alphanumeric())
            && id.chars().filter(|c| c.is_numeric()).count() >= 3
        {
            return false;
        }
        true
    }

    /// Normalize href for fingerprinting (remove query params and hash)
    fn normalize_href(href: &str) -> String {
        href.split('?')
            .next()
            .unwrap_or(href)
            .split('#')
            .next()
            .unwrap_or(href)
            .to_string()
    }

    /// Normalize URL for fingerprinting (remove query params and hash)
    fn normalize_url_for_fingerprint(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or(""),
                parsed.path()
            )
        } else {
            // Relative path, just strip query/hash
            url.split('?')
                .next()
                .unwrap_or(url)
                .split('#')
                .next()
                .unwrap_or(url)
                .to_string()
        }
    }

    /// Check if element is a hover candidate (dropdown, menu, etc.)
    fn is_hover_candidate(element: &AnnotatedElement) -> bool {
        // Check aria attributes
        if element.attributes.get("aria-haspopup").is_some() {
            return true;
        }
        if let Some(expanded) = element.attributes.get("aria-expanded") {
            // Only if it's expandable (false means can be expanded)
            if expanded == "false" {
                return true;
            }
        }

        // Check class names
        if let Some(class) = element.attributes.get("class") {
            let class_lower = class.to_lowercase();
            let hover_classes = [
                "dropdown",
                "menu",
                "nav",
                "submenu",
                "popover",
                "tooltip",
                "trigger",
                "toggle",
                "expand",
                "collapse",
                "accordion",
                "tab",
                "select",
                "combobox",
                "autocomplete",
            ];
            if hover_classes.iter().any(|c| class_lower.contains(c)) {
                return true;
            }
        }

        // Check role
        if let Some(role) = element.attributes.get("role") {
            let role_lower = role.to_lowercase();
            let hover_roles = [
                "menuitem", "menu", "listbox", "combobox", "tab", "tablist", "tree", "treeitem",
                "option", "menubar",
            ];
            if hover_roles.iter().any(|r| role_lower == *r) {
                return true;
            }
        }

        // Check for arrow/expand indicators in text
        let text_lower = element.text.to_lowercase();
        let expand_indicators = ["▼", "▾", "↓", "▶", "›", "»", "more", "expand"];
        if expand_indicators.iter().any(|i| text_lower.contains(i)) {
            return true;
        }

        false
    }

    /// Detect dynamic components from element attributes
    pub fn detect_dynamic_component(element: &AnnotatedElement) -> Option<String> {
        // Check for modal/dialog triggers
        if let Some(target) = element
            .attributes
            .get("data-bs-target")
            .or_else(|| element.attributes.get("data-target"))
            .or_else(|| element.attributes.get("data-modal"))
        {
            if target.contains("modal") || target.contains("dialog") {
                return Some("modal".to_string());
            }
        }

        // Check for drawer/sidebar triggers
        if let Some(class) = element.attributes.get("class") {
            let class_lower = class.to_lowercase();
            if class_lower.contains("drawer") || class_lower.contains("sidebar") {
                return Some("drawer".to_string());
            }
        }

        // Check for popover/tooltip
        if element.attributes.contains_key("data-popover")
            || element.attributes.contains_key("data-tooltip")
        {
            return Some("popover".to_string());
        }

        // Check for tab panels
        if let Some(role) = element.attributes.get("role") {
            if role == "tab" {
                return Some("tab".to_string());
            }
        }

        None
    }

    /// Update elements for current page
    pub fn update_page_elements(&mut self, elements: &[AnnotatedElement], page_url: &str) -> usize {
        self.current_page_url = page_url.to_string();
        self.current_page_mapping.clear();
        self.hover_candidates.clear();

        let mut new_count = 0;

        for element in elements {
            let fingerprint = Self::generate_fingerprint(element, page_url);

            // Save current page mapping
            self.current_page_mapping
                .insert(element.index, fingerprint.clone());

            // Check for new elements
            if !self.all_elements.contains_key(&fingerprint) {
                new_count += 1;

                let is_hover = Self::is_hover_candidate(element);

                let info = ElementFingerprint {
                    fingerprint: fingerprint.clone(),
                    element_type: element.element_type.clone(),
                    text: element.text.chars().take(50).collect(),
                    page_url: page_url.to_string(),
                    original_index: element.index,
                    is_hover_candidate: is_hover,
                };

                if is_hover {
                    self.hover_candidates.push(fingerprint.clone());
                }

                self.all_elements.insert(fingerprint.clone(), info);

                // Detect dynamic components
                if let Some(component_type) = Self::detect_dynamic_component(element) {
                    self.add_dynamic_component(DynamicComponent {
                        component_type,
                        selector: element.selector.clone(),
                        trigger_fingerprint: Some(fingerprint),
                        explored: false,
                    });
                }
            }
        }

        if new_count > 0 {
            info!(
                "Discovered {} new elements on page: {}",
                new_count, page_url
            );
        }

        new_count
    }

    /// 标记元素已交互（通过当前页面索引）
    pub fn mark_interacted_by_index(&mut self, index: u32) -> bool {
        if let Some(fingerprint) = self.current_page_mapping.get(&index).cloned() {
            self.interacted_elements.insert(fingerprint);
            true
        } else {
            debug!("Element index {} not found in current page mapping", index);
            false
        }
    }

    /// 标记元素已交互（通过指纹）
    pub fn mark_interacted(&mut self, fingerprint: &str) {
        self.interacted_elements.insert(fingerprint.to_string());
    }

    /// 检查元素是否已交互（通过当前页面索引）
    pub fn is_interacted_by_index(&self, index: u32) -> bool {
        if let Some(fingerprint) = self.current_page_mapping.get(&index) {
            self.interacted_elements.contains(fingerprint)
        } else {
            false
        }
    }

    /// 检查索引是否属于当前页面映射（防止模型使用陈旧索引）
    pub fn is_known_index(&self, index: u32) -> bool {
        self.current_page_mapping.contains_key(&index)
    }

    /// 获取当前页面未交互的元素索引
    pub fn get_uninteracted_indices(&self) -> Vec<u32> {
        self.current_page_mapping
            .iter()
            .filter(|(_, fp)| !self.interacted_elements.contains(*fp))
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// 获取当前页面的悬停候选元素索引
    pub fn get_hover_candidate_indices(&self) -> Vec<u32> {
        self.current_page_mapping
            .iter()
            .filter(|(_, fp)| {
                if let Some(info) = self.all_elements.get(*fp) {
                    info.is_hover_candidate && !self.interacted_elements.contains(*fp)
                } else {
                    false
                }
            })
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// 获取指纹对应的元素索引（当前页面）
    pub fn get_index_by_fingerprint(&self, fingerprint: &str) -> Option<u32> {
        self.current_page_mapping
            .iter()
            .find(|(_, fp)| *fp == fingerprint)
            .map(|(idx, _)| *idx)
    }

    /// 计算元素覆盖率
    pub fn coverage_percentage(&self) -> f32 {
        if self.all_elements.is_empty() {
            return 100.0;
        }
        (self.interacted_elements.len() as f32 / self.all_elements.len() as f32) * 100.0
    }

    /// 获取统计信息
    pub fn stats(&self) -> ElementStats {
        ElementStats {
            total: self.all_elements.len(),
            interacted: self.interacted_elements.len(),
            hover_candidates: self.hover_candidates.len(),
            coverage: self.coverage_percentage(),
        }
    }

    /// 获取未交互元素列表（用于报告）
    pub fn get_uninteracted_elements(&self, limit: usize) -> Vec<ElementFingerprint> {
        self.all_elements
            .iter()
            .filter(|(fp, _)| !self.interacted_elements.contains(*fp))
            .take(limit)
            .map(|(_, info)| info.clone())
            .collect()
    }

    /// 添加动态组件
    pub fn add_dynamic_component(&mut self, component: DynamicComponent) {
        // 检查是否已存在
        let exists = self.dynamic_components.iter().any(|c| {
            c.selector == component.selector && c.component_type == component.component_type
        });

        if !exists {
            info!(
                "Discovered dynamic component: {} ({})",
                component.component_type, component.selector
            );
            self.dynamic_components.push(component);
        }
    }

    /// 标记动态组件已探索
    pub fn mark_component_explored(&mut self, selector: &str) {
        for comp in &mut self.dynamic_components {
            if comp.selector == selector {
                comp.explored = true;
                break;
            }
        }
    }

    /// 获取未探索的动态组件
    pub fn get_unexplored_components(&self) -> Vec<&DynamicComponent> {
        self.dynamic_components
            .iter()
            .filter(|c| !c.explored)
            .collect()
    }

    /// 计算组件覆盖率
    pub fn component_coverage_percentage(&self) -> f32 {
        if self.dynamic_components.is_empty() {
            return 100.0;
        }
        let explored = self
            .dynamic_components
            .iter()
            .filter(|c| c.explored)
            .count();
        (explored as f32 / self.dynamic_components.len() as f32) * 100.0
    }

    /// 获取所有元素数量
    pub fn total_elements(&self) -> usize {
        self.all_elements.len()
    }

    /// 获取已交互元素数量
    pub fn interacted_count(&self) -> usize {
        self.interacted_elements.len()
    }
}

impl Default for ElementManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 元素统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElementStats {
    pub total: usize,
    pub interacted: usize,
    pub hover_candidates: usize,
    pub coverage: f32,
}

impl Default for ElementStats {
    fn default() -> Self {
        Self {
            total: 0,
            interacted: 0,
            hover_candidates: 0,
            coverage: 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::BoundingBox;
    use super::*;
    use std::collections::HashMap;

    fn make_element(
        index: u32,
        element_type: &str,
        text: &str,
        attributes: HashMap<String, String>,
    ) -> AnnotatedElement {
        AnnotatedElement {
            index,
            element_type: element_type.to_string(),
            tag_name: "DIV".to_string(),
            text: text.to_string(),
            selector: format!("#element-{}", index),
            bounding_box: BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            attributes,
            enhanced_attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_element_manager_basic() {
        let mut manager = ElementManager::new();

        let elements = vec![
            make_element(0, "button", "Submit", HashMap::new()),
            make_element(1, "link", "Home", HashMap::new()),
        ];

        let new_count = manager.update_page_elements(&elements, "https://example.com/");
        assert_eq!(new_count, 2);

        // 重复添加不应增加
        let new_count2 = manager.update_page_elements(&elements, "https://example.com/");
        assert_eq!(new_count2, 0);
    }

    #[test]
    fn test_hover_candidate_detection() {
        let mut manager = ElementManager::new();

        let mut attrs = HashMap::new();
        attrs.insert("aria-haspopup".to_string(), "true".to_string());

        let elements = vec![
            make_element(0, "button", "Menu", attrs),
            make_element(1, "button", "Submit", HashMap::new()),
        ];

        manager.update_page_elements(&elements, "https://example.com/");

        let hover_indices = manager.get_hover_candidate_indices();
        assert_eq!(hover_indices.len(), 1);
        assert_eq!(hover_indices[0], 0);
    }

    #[test]
    fn test_coverage() {
        let mut manager = ElementManager::new();

        let elements = vec![
            make_element(0, "button", "A", HashMap::new()),
            make_element(1, "button", "B", HashMap::new()),
        ];

        manager.update_page_elements(&elements, "https://example.com/");
        assert_eq!(manager.coverage_percentage(), 0.0);

        manager.mark_interacted_by_index(0);
        assert_eq!(manager.coverage_percentage(), 50.0);

        manager.mark_interacted_by_index(1);
        assert_eq!(manager.coverage_percentage(), 100.0);
    }
}
