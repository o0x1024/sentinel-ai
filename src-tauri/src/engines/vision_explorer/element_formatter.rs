//! Element formatting for prompts
//!
//! Formats annotated elements into readable strings for VLM prompts

use super::types::AnnotatedElement;

/// Safely truncate a string to a maximum number of characters
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// Format elements grouped by category for better comprehension
/// Groups: navigation, form_inputs, actions, links, other
pub fn format_elements_grouped(elements: &[AnnotatedElement], limit: usize) -> String {
    // Categorize elements
    let mut nav_items: Vec<&AnnotatedElement> = Vec::new();
    let mut form_inputs: Vec<&AnnotatedElement> = Vec::new();
    let mut actions: Vec<&AnnotatedElement> = Vec::new();
    let mut links: Vec<&AnnotatedElement> = Vec::new();
    let mut other: Vec<&AnnotatedElement> = Vec::new();

    for e in elements.iter().take(limit) {
        let is_nav = e
            .attributes
            .get("role")
            .map(|r| r.contains("nav") || r == "menuitem" || r == "menu")
            .unwrap_or(false)
            || e.attributes
                .get("class")
                .map(|c| {
                    let cl = c.to_lowercase();
                    cl.contains("nav") || cl.contains("menu") || cl.contains("sidebar")
                })
                .unwrap_or(false);

        if is_nav {
            nav_items.push(e);
        } else if e.element_type == "input"
            || e.element_type == "textarea"
            || e.element_type == "select"
        {
            form_inputs.push(e);
        } else if e.element_type == "button" || e.element_type == "submit" {
            actions.push(e);
        } else if e.element_type == "link" {
            links.push(e);
        } else {
            other.push(e);
        }
    }

    let mut output = String::new();

    // Navigation section
    if !nav_items.is_empty() {
        output.push_str(&format!("### Navigation ({} items)\n", nav_items.len()));
        for e in nav_items.iter().take(20) {
            output.push_str(&format_element_line(e));
        }
        if nav_items.len() > 20 {
            output.push_str(&format!(
                "... +{} more navigation items\n",
                nav_items.len() - 20
            ));
        }
        output.push('\n');
    }

    // Form inputs section
    if !form_inputs.is_empty() {
        output.push_str(&format!("### Form Inputs ({} items)\n", form_inputs.len()));
        for e in form_inputs.iter().take(15) {
            output.push_str(&format_element_line(e));
        }
        if form_inputs.len() > 15 {
            output.push_str(&format!("... +{} more inputs\n", form_inputs.len() - 15));
        }
        output.push('\n');
    }

    // Action buttons section
    if !actions.is_empty() {
        output.push_str(&format!("### Actions ({} items)\n", actions.len()));
        for e in actions.iter().take(15) {
            output.push_str(&format_element_line(e));
        }
        if actions.len() > 15 {
            output.push_str(&format!("... +{} more actions\n", actions.len() - 15));
        }
        output.push('\n');
    }

    // Links section
    if !links.is_empty() {
        output.push_str(&format!("### Links ({} items)\n", links.len()));
        for e in links.iter().take(25) {
            output.push_str(&format_element_line(e));
        }
        if links.len() > 25 {
            output.push_str(&format!("... +{} more links\n", links.len() - 25));
        }
        output.push('\n');
    }

    // Other elements section
    if !other.is_empty() {
        output.push_str(&format!("### Other ({} items)\n", other.len()));
        for e in other.iter().take(10) {
            output.push_str(&format_element_line(e));
        }
        if other.len() > 10 {
            output.push_str(&format!("... +{} more elements\n", other.len() - 10));
        }
    }

    output
}

/// Format a single element line with key attributes
pub fn format_element_line(e: &AnnotatedElement) -> String {
    let text = truncate_str(&e.text, 25).replace('\n', " ");
    let mut attrs = Vec::new();

    // href for links
    if let Some(href) = e.attributes.get("href") {
        if !href.is_empty() && !href.starts_with("javascript:") {
            let path = if href.starts_with("http") {
                href.split('/').skip(3).collect::<Vec<_>>().join("/")
            } else {
                href.to_string()
            };
            attrs.push(format!("→{}", truncate_str(&path, 25)));
        }
    }

    // name/placeholder for inputs
    if let Some(name) = e.attributes.get("name") {
        if !name.is_empty() {
            attrs.push(format!("n:{}", truncate_str(name, 12)));
        }
    }
    if let Some(ph) = e.attributes.get("placeholder") {
        if !ph.is_empty() && text.is_empty() {
            attrs.push(format!("ph:{}", truncate_str(ph, 12)));
        }
    }

    // Current value for inputs
    if let Some(value) = e.attributes.get("value") {
        if !value.is_empty() {
            let display_val = if e.element_type == "password" {
                "***".to_string()
            } else {
                truncate_str(value, 15)
            };
            attrs.push(format!("v:{}", display_val));
        }
    }

    // Input type
    if let Some(input_type) = e.attributes.get("type") {
        if !input_type.is_empty() && input_type != "text" && input_type != "submit" {
            attrs.push(format!("t:{}", input_type));
        }
    }

    // Required indicator
    if e.attributes.get("required").is_some() {
        attrs.push("*".to_string());
    }

    // Disabled state
    if e.attributes.get("disabled").is_some()
        || e.attributes
            .get("aria-disabled")
            .map(|v| v == "true")
            .unwrap_or(false)
    {
        attrs.push("⊘".to_string());
    }

    // Expandable indicator
    if e.attributes.get("aria-haspopup").is_some()
        || e.attributes
            .get("aria-expanded")
            .map(|v| v == "false")
            .unwrap_or(false)
    {
        attrs.push("▼".to_string());
    }

    // Already expanded indicator
    if e.attributes
        .get("aria-expanded")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        attrs.push("▲".to_string());
    }

    let attrs_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" [{}]", attrs.join(" "))
    };
    format!("[{}] {} \"{}\"{}\n", e.index, e.element_type, text, attrs_str)
}

/// Format elements as compact CSV (legacy format)
pub fn format_elements_as_csv(elements: &[AnnotatedElement], limit: usize) -> String {
    let mut lines = Vec::with_capacity(limit);

    for e in elements.iter().take(limit) {
        let text = truncate_str(&e.text, 25)
            .replace(',', ";")
            .replace('\n', " ");

        let mut extras = Vec::new();

        // href for links
        if let Some(href) = e.attributes.get("href") {
            if !href.is_empty() && !href.starts_with("javascript:") {
                let path = if href.starts_with("http") {
                    href.split('/').skip(3).collect::<Vec<_>>().join("/")
                } else {
                    href.to_string()
                };
                extras.push(format!("→{}", truncate_str(&path, 30)));
            }
        }

        // name/placeholder for inputs
        if let Some(name) = e.attributes.get("name") {
            if !name.is_empty() {
                extras.push(format!("n:{}", truncate_str(name, 15)));
            }
        }
        if let Some(ph) = e.attributes.get("placeholder") {
            if !ph.is_empty() && text.is_empty() {
                extras.push(format!("ph:{}", truncate_str(ph, 15)));
            }
        }

        // Input type
        if let Some(input_type) = e.attributes.get("type") {
            if !input_type.is_empty() && input_type != "text" && input_type != "submit" {
                extras.push(format!("t:{}", input_type));
            }
        }

        // Expandable indicators
        if e.attributes.get("aria-haspopup").is_some()
            || e.attributes
                .get("aria-expanded")
                .map(|v| v == "false")
                .unwrap_or(false)
        {
            extras.push("▼".to_string());
        }

        // Role (only meaningful ones)
        if let Some(role) = e.attributes.get("role") {
            let meaningful_roles = ["menu", "menuitem", "tab", "button", "link", "listbox"];
            if meaningful_roles.contains(&role.as_str()) {
                extras.push(format!("r:{}", role));
            }
        }

        let extra_str = if extras.is_empty() {
            String::new()
        } else {
            extras.join(" ").replace(',', ";")
        };

        let line = format!("{},{},{},{}", e.index, e.element_type, text, extra_str);
        lines.push(line);
    }

    lines.join("\n")
}

