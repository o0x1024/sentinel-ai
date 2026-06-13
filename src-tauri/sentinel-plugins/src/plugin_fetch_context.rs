use crate::plugin_context::PluginContext;

/// Returns `Ok(())` when the plugin context may issue a normal (non-activeProbe) fetch.
pub fn check_fetch_allowed(plugin_ctx: &PluginContext) -> Result<(), String> {
    let execution_context = plugin_ctx.execution_context().unwrap_or_default();
    match execution_context.as_str() {
        "traffic_scan" => {
            return Err(
                "traffic_scan plugins cannot actively fetch without activeProbe".to_string(),
            );
        }
        "intruder_processor" => {
            return Err("intruder_processor plugins cannot actively fetch".to_string());
        }
        _ => {}
    }

    match plugin_ctx.plugin_main_category().as_deref() {
        Some("traffic") => Err("traffic plugins cannot actively fetch without activeProbe".to_string()),
        Some("intruder") => Err("intruder plugins cannot actively fetch".to_string()),
        _ => Ok(()),
    }
}
