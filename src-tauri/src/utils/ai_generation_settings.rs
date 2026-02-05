use sentinel_db::Database;
use sentinel_llm::LlmConfig;

/// Apply persisted AI generation settings (temperature/max_tokens) to an LLM config.
pub async fn apply_generation_settings_from_db(
    db: &dyn Database,
    mut config: LlmConfig,
) -> LlmConfig {
    let temperature = db
        .get_config("ai", "temperature")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.trim().parse::<f32>().ok());

    if let Some(temp) = temperature {
        config = config.with_temperature(temp);
    }

    let max_tokens = db
        .get_config("ai", "max_tokens")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.trim().parse::<u32>().ok());

    if let Some(tokens) = max_tokens {
        config = config.with_max_tokens(tokens);
    }

    config
}
