use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};

const ADMIN_INDEX: &str = include_str!("admin_ui_vue_dist/index.html");
const ADMIN_JS: &str = include_str!("admin_ui_vue_dist/assets/admin.js");
const ADMIN_CSS: &str = include_str!("admin_ui_vue_dist/assets/admin.css");

pub async fn admin_page() -> Html<&'static str> {
    Html(ADMIN_INDEX)
}

pub async fn admin_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "admin.js" => (
            [(
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            ADMIN_JS,
        )
            .into_response(),
        "admin.css" => (
            [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
            ADMIN_CSS,
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
