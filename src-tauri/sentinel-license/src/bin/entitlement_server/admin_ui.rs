use axum::response::Html;

const ADMIN_UI_TEMPLATE: &str = include_str!("admin_ui.html");
const ADMIN_UI_CSS_PARTS: [&str; 5] = [
    include_str!("admin_ui_base.css"),
    include_str!("admin_ui_layout.css"),
    include_str!("admin_ui_forms.css"),
    include_str!("admin_ui_data.css"),
    include_str!("admin_ui_overlays.css"),
];
const ADMIN_UI_SIDEBAR: &str = include_str!("admin_ui_sidebar.html");
const ADMIN_UI_OVERVIEW_PANEL: &str = include_str!("admin_ui_overview_panel.html");
const ADMIN_UI_CUSTOMERS_PANEL: &str = include_str!("admin_ui_customers_panel.html");
const ADMIN_UI_AUDIT_PANEL: &str = include_str!("admin_ui_audit_panel.html");
const ADMIN_UI_DEVICES_PANEL: &str = include_str!("admin_ui_devices_panel.html");
const ADMIN_UI_ADMINS_PANEL: &str = include_str!("admin_ui_admins_panel.html");
const ADMIN_UI_CONFIRM_DIALOG: &str = include_str!("admin_ui_confirm_dialog.html");
const ADMIN_UI_JS_PARTS: [&str; 7] = [
    include_str!("admin_ui_shared.js"),
    include_str!("admin_ui_overview.js"),
    include_str!("admin_ui_customers.js"),
    include_str!("admin_ui_audit.js"),
    include_str!("admin_ui_devices.js"),
    include_str!("admin_ui_admins.js"),
    include_str!("admin_ui_bootstrap.js"),
];

pub async fn admin_page() -> Html<String> {
    let admin_ui_css = ADMIN_UI_CSS_PARTS.join("\n\n");
    let admin_ui_js = ADMIN_UI_JS_PARTS.join("\n\n");
    let html = ADMIN_UI_TEMPLATE
        .replace("__ADMIN_UI_CSS__", &admin_ui_css)
        .replace("__ADMIN_UI_SIDEBAR__", ADMIN_UI_SIDEBAR)
        .replace("__ADMIN_UI_OVERVIEW_PANEL__", ADMIN_UI_OVERVIEW_PANEL)
        .replace("__ADMIN_UI_CUSTOMERS_PANEL__", ADMIN_UI_CUSTOMERS_PANEL)
        .replace("__ADMIN_UI_AUDIT_PANEL__", ADMIN_UI_AUDIT_PANEL)
        .replace("__ADMIN_UI_DEVICES_PANEL__", ADMIN_UI_DEVICES_PANEL)
        .replace("__ADMIN_UI_ADMINS_PANEL__", ADMIN_UI_ADMINS_PANEL)
        .replace("__ADMIN_UI_CONFIRM_DIALOG__", ADMIN_UI_CONFIRM_DIALOG)
        .replace("__ADMIN_UI_JS__", &admin_ui_js);
    Html(html)
}
