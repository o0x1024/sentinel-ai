use sentinel_db::{Database, DatabaseService};

pub async fn resolve_ui_language(db: &DatabaseService) -> String {
    match db.get_config("ui", "language").await {
        Ok(Some(lang)) if lang.to_lowercase().starts_with("zh") => "zh".to_string(),
        Ok(Some(_)) => "en".to_string(),
        _ => "en".to_string(),
    }
}

pub fn is_chinese_ui_language(language: &str) -> bool {
    language.to_lowercase().starts_with("zh")
}

pub fn output_language_instruction(language: &str) -> &'static str {
    if is_chinese_ui_language(language) {
        "Return all natural-language values in Simplified Chinese. Keep JSON keys in English."
    } else {
        "Return all natural-language values in English. Keep JSON keys in English."
    }
}
