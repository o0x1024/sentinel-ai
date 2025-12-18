//! Login page detection logic
//!
//! Detects login pages and extracts login form fields

use super::types::{LoginField, PageState};

/// Check if a URL looks like a login route
pub fn is_login_like_route(url: &str) -> bool {
    let lower = url.to_lowercase();
    ["login", "signin", "sign-in", "auth", "authenticate", "sso"]
        .iter()
        .any(|k| lower.contains(k))
}

/// Detect login page and extract login fields
pub fn detect_login_page(page_state: &PageState) -> Option<Vec<LoginField>> {
    let url_lower = page_state.url.to_lowercase();
    let title_lower = page_state.title.to_lowercase();

    let url_indicators = ["login", "signin", "sign-in", "auth", "authenticate", "sso"];
    let is_url_login = url_indicators.iter().any(|ind| url_lower.contains(ind));

    let title_indicators = ["登录", "login", "signin", "sign in", "登入", "认证"];
    let is_title_login = title_indicators.iter().any(|ind| title_lower.contains(ind));

    // If the page clearly shows "logged-in" indicators, do NOT treat it as login page
    if has_logged_in_indicators(page_state) {
        return None;
    }

    // Filter visible input elements
    let inputs: Vec<_> = page_state
        .interactable_elements
        .iter()
        .filter(|e| {
            let tag = e.tag.to_lowercase();
            let type_attr = e
                .element_type
                .as_ref()
                .or_else(|| e.attributes.get("type"))
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "text".to_string());

            tag == "input"
                && !["hidden", "submit", "button", "image", "reset"].contains(&type_attr.as_str())
        })
        .collect();

    let has_password = inputs.iter().any(|e| {
        e.element_type
            .as_ref()
            .or_else(|| e.attributes.get("type"))
            .map(|s| s.to_lowercase() == "password")
            .unwrap_or(false)
    });

    let has_login_action = has_login_action_indicators(page_state);

    // Stricter login page detection
    let is_login_page = (has_password && has_login_action)
        || ((is_url_login || is_title_login) && !inputs.is_empty() && has_login_action);

    if !is_login_page {
        return None;
    }

    // Build field list
    let mut fields = Vec::new();
    let mut has_username = false;
    let mut has_password_field = false;

    for input in inputs {
        let type_attr = input
            .element_type
            .as_ref()
            .or_else(|| input.attributes.get("type"))
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "text".to_string());

        let name_attr = input
            .attributes
            .get("name")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let id_attr = input.id.to_lowercase();

        let placeholder_attr = input
            .attributes
            .get("placeholder")
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| input.text.to_lowercase());

        let combined_text = format!("{} {} {}", name_attr, id_attr, placeholder_attr);

        if type_attr == "password" {
            fields.push(LoginField {
                id: "password".to_string(),
                label: "密码".to_string(),
                field_type: "password".to_string(),
                required: true,
                placeholder: Some(
                    input
                        .attributes
                        .get("placeholder")
                        .cloned()
                        .unwrap_or("请输入密码".to_string()),
                ),
            });
            has_password_field = true;
        } else if !has_username
            && (type_attr == "email"
                || combined_text.contains("user")
                || combined_text.contains("name")
                || combined_text.contains("login")
                || combined_text.contains("email")
                || combined_text.contains("phone")
                || combined_text.contains("account")
                || combined_text.contains("账号")
                || combined_text.contains("用户")
                || combined_text.contains("邮箱")
                || combined_text.contains("手机"))
        {
            fields.push(LoginField {
                id: "username".to_string(),
                label: "账号/邮箱/手机号".to_string(),
                field_type: "text".to_string(),
                required: true,
                placeholder: Some(
                    input
                        .attributes
                        .get("placeholder")
                        .cloned()
                        .unwrap_or("请输入账号".to_string()),
                ),
            });
            has_username = true;
        } else if combined_text.contains("code")
            || combined_text.contains("verif")
            || combined_text.contains("captcha")
            || combined_text.contains("otp")
            || combined_text.contains("验证码")
        {
            fields.push(LoginField {
                id: "verification_code".to_string(),
                label: "验证码".to_string(),
                field_type: "text".to_string(),
                required: false,
                placeholder: Some(
                    input
                        .attributes
                        .get("placeholder")
                        .cloned()
                        .unwrap_or("请输入验证码".to_string()),
                ),
            });
        } else {
            // Other unknown fields
            let mut field_id = input
                .attributes
                .get("name")
                .cloned()
                .unwrap_or_else(|| input.id.clone());

            if field_id.starts_with("element_") {
                field_id = format!("field_{}", fields.len());
            }

            let label = input
                .attributes
                .get("placeholder")
                .cloned()
                .unwrap_or_else(|| "输入框".to_string());

            fields.push(LoginField {
                id: field_id,
                label,
                field_type: type_attr,
                required: false,
                placeholder: input.attributes.get("placeholder").cloned(),
            });
        }
    }

    // Fallback: add standard fields if URL strongly indicates login page
    if (!has_username || !has_password_field)
        && (is_url_login || is_title_login)
        && fields.is_empty()
    {
        return Some(vec![
            LoginField {
                id: "username".to_string(),
                label: "账号".to_string(),
                field_type: "text".to_string(),
                required: true,
                placeholder: Some("请输入账号".to_string()),
            },
            LoginField {
                id: "password".to_string(),
                label: "密码".to_string(),
                field_type: "password".to_string(),
                required: true,
                placeholder: Some("请输入密码".to_string()),
            },
        ]);
    }

    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Check if page has logged-in indicators
pub fn has_logged_in_indicators(page_state: &PageState) -> bool {
    let indicators = [
        "logout",
        "log out",
        "sign out",
        "退出",
        "注销",
        "登出",
        "个人中心",
        "工作台",
        "控制台",
        "dashboard",
    ];

    let haystacks: Vec<String> = page_state
        .interactable_elements
        .iter()
        .flat_map(|e| {
            let mut v = Vec::with_capacity(4);
            v.push(e.text.to_lowercase());
            if let Some(t) = &e.element_type {
                v.push(t.to_lowercase());
            }
            if let Some(vv) = e.attributes.get("aria-label") {
                v.push(vv.to_lowercase());
            }
            if let Some(vv) = e.attributes.get("title") {
                v.push(vv.to_lowercase());
            }
            v
        })
        .chain(page_state.annotated_elements.iter().flat_map(|e| {
            let mut v = Vec::with_capacity(4);
            v.push(e.text.to_lowercase());
            v.push(e.element_type.to_lowercase());
            if let Some(vv) = e.attributes.get("aria-label") {
                v.push(vv.to_lowercase());
            }
            if let Some(vv) = e.attributes.get("title") {
                v.push(vv.to_lowercase());
            }
            v
        }))
        .collect();

    let has_text_indicators = indicators.iter().any(|k| {
        let kk = k.to_lowercase();
        haystacks.iter().any(|s| s.contains(&kk))
    });

    // Check URL for backend patterns
    let url_lower = page_state.url.to_lowercase();
    let backend_url_patterns = [
        "/admin",
        "/dashboard",
        "/console",
        "/manage",
        "/backend",
        "/system",
        "/settings",
        "/threat",
        "/vuln",
        "/permission",
    ];
    let has_backend_url = backend_url_patterns.iter().any(|p| url_lower.contains(p));

    has_text_indicators || has_backend_url
}

/// Check if page has login action indicators (submit buttons etc)
pub fn has_login_action_indicators(page_state: &PageState) -> bool {
    let keywords = [
        "登录",
        "login",
        "sign in",
        "signin",
        "submit",
        "立即登录",
    ];

    let mut candidates: Vec<String> = Vec::new();

    for e in &page_state.interactable_elements {
        let tag = e.tag.to_lowercase();
        let t = e.text.to_lowercase();
        let aria = e
            .attributes
            .get("aria-label")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let title = e
            .attributes
            .get("title")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let ty = e
            .element_type
            .as_ref()
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let value = e
            .attributes
            .get("value")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if tag == "button" || (tag == "input" && ["submit", "button"].contains(&ty.as_str())) {
            candidates.push(format!("{} {} {} {}", t, aria, title, value));
        }
    }

    for e in &page_state.annotated_elements {
        let t = e.text.to_lowercase();
        let ty = e.element_type.to_lowercase();
        let aria = e
            .attributes
            .get("aria-label")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let title = e
            .attributes
            .get("title")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if ty.contains("button") || ty.contains("submit") {
            candidates.push(format!("{} {} {}", t, aria, title));
        }
    }

    if candidates.is_empty() {
        return false;
    }

    keywords.iter().any(|k| {
        let kk = k.to_lowercase();
        candidates.iter().any(|s| s.contains(&kk))
    })
}

