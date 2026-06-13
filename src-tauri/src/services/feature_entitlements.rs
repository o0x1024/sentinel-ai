use serde::{Deserialize, Serialize};

const FREE_TIER_ALLOWED_PLUGIN_IDS: &[&str] = &[
    "api_monitor",
    "intruder_dictionary_payload_generator",
    "intruder_hmac_request_signer",
    "intruder_sql_comment_tamper",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntitlements {
    pub tier: String,
    pub is_licensed: bool,
    pub has_local_license: bool,
    pub access_source: String,
    pub can_access_all_plugins: bool,
    pub can_access_bug_bounty: bool,
    pub can_access_bot_console: bool,
    pub can_manage_plugin_catalog: bool,
    pub can_add_plugins: bool,
    pub can_edit_plugins: bool,
    pub can_delete_plugins: bool,
    pub can_install_plugins: bool,
    pub can_review_plugins: bool,
    pub allowed_plugin_ids: Vec<String>,
}

pub fn get_free_tier_allowed_plugin_ids() -> Vec<String> {
    FREE_TIER_ALLOWED_PLUGIN_IDS
        .iter()
        .map(|plugin_id| (*plugin_id).to_string())
        .collect()
}

fn pro_entitlements(access_source: &str) -> AppEntitlements {
    AppEntitlements {
        tier: "pro".to_string(),
        is_licensed: true,
        has_local_license: access_source == "local_license",
        access_source: access_source.to_string(),
        can_access_all_plugins: true,
        can_access_bug_bounty: true,
        can_access_bot_console: true,
        can_manage_plugin_catalog: true,
        can_add_plugins: true,
        can_edit_plugins: true,
        can_delete_plugins: true,
        can_install_plugins: true,
        can_review_plugins: true,
        allowed_plugin_ids: get_free_tier_allowed_plugin_ids(),
    }
}

pub fn build_app_entitlements() -> AppEntitlements {
    let allowed_plugin_ids = get_free_tier_allowed_plugin_ids();

    if cfg!(debug_assertions) || !sentinel_license::is_enforcement_enabled() {
        return AppEntitlements {
            tier: "pro".to_string(),
            is_licensed: true,
            has_local_license: true,
            access_source: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "open".to_string()
            },
            can_access_all_plugins: true,
            can_access_bug_bounty: true,
            can_access_bot_console: true,
            can_manage_plugin_catalog: true,
            can_add_plugins: true,
            can_edit_plugins: true,
            can_delete_plugins: true,
            can_install_plugins: true,
            can_review_plugins: true,
            allowed_plugin_ids,
        };
    }

    if sentinel_license::has_valid_local_license() {
        return pro_entitlements("local_license");
    }

    AppEntitlements {
        tier: "free".to_string(),
        is_licensed: false,
        has_local_license: false,
        access_source: "free".to_string(),
        can_access_all_plugins: false,
        can_access_bug_bounty: false,
        can_access_bot_console: false,
        can_manage_plugin_catalog: false,
        can_add_plugins: false,
        can_edit_plugins: false,
        can_delete_plugins: false,
        can_install_plugins: false,
        can_review_plugins: false,
        allowed_plugin_ids,
    }
}

pub fn is_plugin_allowed_for_current_tier(plugin_id: &str) -> bool {
    let normalized_plugin_id = plugin_id.trim();
    if normalized_plugin_id.is_empty() {
        return false;
    }

    let entitlements = build_app_entitlements();
    entitlements.can_access_all_plugins
        || entitlements
            .allowed_plugin_ids
            .iter()
            .any(|allowed_id| allowed_id == normalized_plugin_id)
}

pub fn ensure_bug_bounty_access() -> Result<(), String> {
    if build_app_entitlements().can_access_bug_bounty {
        return Ok(());
    }

    if sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::BugBounty).is_err()
    {
        return Err("漏洞赏金功能需要完成本地 License 激活".to_string());
    }

    Err("漏洞赏金功能仅对付费版开放".to_string())
}

pub fn ensure_bot_console_access() -> Result<(), String> {
    if build_app_entitlements().can_access_bot_console {
        return Ok(());
    }

    if sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::BotConsole).is_err()
    {
        return Err("Bot 控制台需要完成本地 License 激活".to_string());
    }

    Err("Bot 控制台仅对付费版开放".to_string())
}

pub fn ensure_plugin_catalog_write_access() -> Result<(), String> {
    if build_app_entitlements().can_add_plugins {
        return Ok(());
    }

    if sentinel_license::ensure_feature_access(
        sentinel_license::LicensedFeature::PluginCatalogWrite,
    )
    .is_err()
    {
        return Err("新增、上传、安装或更新插件需要完成本地 License 激活".to_string());
    }

    Err("免费版不支持新增、上传、安装或更新插件".to_string())
}

pub fn ensure_plugin_delete_access() -> Result<(), String> {
    if build_app_entitlements().can_delete_plugins {
        return Ok(());
    }

    if sentinel_license::ensure_feature_access(
        sentinel_license::LicensedFeature::PluginCatalogDelete,
    )
    .is_err()
    {
        return Err("删除插件需要完成本地 License 激活".to_string());
    }

    Err("免费版不支持删除插件".to_string())
}

pub fn ensure_plugin_allowed_for_current_tier(plugin_id: &str) -> Result<(), String> {
    if is_plugin_allowed_for_current_tier(plugin_id) {
        return Ok(());
    }

    if sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::PluginCatalogRead)
        .is_err()
    {
        return Err("访问非白名单插件需要完成本地 License 激活".to_string());
    }

    Err(format!(
        "免费版仅允许使用固定插件：{}",
        get_free_tier_allowed_plugin_ids().join(", ")
    ))
}

pub fn filter_plugins_for_current_tier<T, F>(plugins: Vec<T>, get_plugin_id: F) -> Vec<T>
where
    F: Fn(&T) -> &str,
{
    if build_app_entitlements().can_access_all_plugins {
        return plugins;
    }

    plugins
        .into_iter()
        .filter(|plugin| is_plugin_allowed_for_current_tier(get_plugin_id(plugin)))
        .collect()
}
