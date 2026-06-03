use crate::plugin_context::PluginContext;
use crate::request_scheduler::{PluginFetchPolicyKind, PluginRequestScheduleRequest};

pub fn fetch_policy_kind_for_context(
    plugin_ctx: &PluginContext,
) -> Result<Option<PluginFetchPolicyKind>, String> {
    let execution_context = plugin_ctx.execution_context().unwrap_or_default();
    match execution_context.as_str() {
        "bounty_workflow" => return Ok(Some(PluginFetchPolicyKind::BountyFetch)),
        "monitor_task" => return Ok(Some(PluginFetchPolicyKind::MonitorFetch)),
        "agent_tool" => return Ok(Some(PluginFetchPolicyKind::AgentFetch)),
        "traffic_active_probe" => return Ok(Some(PluginFetchPolicyKind::TrafficActiveProbe)),
        "plugin_test" => return Ok(Some(PluginFetchPolicyKind::PluginTestFetch)),
        "traffic_scan" => {
            return Err(
                "traffic_scan plugins cannot actively fetch without activeProbe".to_string(),
            )
        }
        "intruder_processor" => {
            return Err("intruder_processor plugins cannot actively fetch".to_string())
        }
        "" => {}
        _ => {}
    }

    match plugin_ctx.plugin_main_category().as_deref() {
        Some("bounty") => Ok(Some(PluginFetchPolicyKind::BountyFetch)),
        Some("agent") => Ok(Some(PluginFetchPolicyKind::AgentFetch)),
        Some("traffic") => {
            Err("traffic plugins cannot actively fetch without activeProbe".to_string())
        }
        Some("intruder") => Err("intruder plugins cannot actively fetch".to_string()),
        _ => Ok(None),
    }
}

pub fn build_plugin_request_schedule(
    plugin_ctx: &PluginContext,
    kind: PluginFetchPolicyKind,
    request_id: &str,
    method: &str,
    url: &str,
) -> Result<PluginRequestScheduleRequest, String> {
    let host = normalize_fetch_host(url)?;
    let plugin_id = plugin_ctx
        .plugin_id()
        .unwrap_or_else(|| "unknown-plugin".to_string());
    let execution_context = plugin_ctx
        .execution_context()
        .unwrap_or_else(|| "unknown".to_string());
    let run_id = plugin_ctx
        .run_id()
        .unwrap_or_else(|| format!("plugin-run:{plugin_id}"));

    Ok(PluginRequestScheduleRequest {
        kind,
        request_id: request_id.to_string(),
        run_id,
        plugin_id,
        execution_context,
        method: method.to_string(),
        url: url.to_string(),
        host,
    })
}

fn normalize_fetch_host(url: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|error| format!("Invalid fetch URL '{}': {}", url, error))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("Fetch URL '{}' has no host", url))?;
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Ok(format!("{host}{port}"))
}
