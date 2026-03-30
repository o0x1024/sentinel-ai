use crate::service_probe_runtime::{ServiceProbeRule, ServiceProbeTarget};
use std::collections::BTreeMap;

use super::common::{invalid_target_result, is_http_target, normalize_protocol};
use super::http::probe_http_evidence;
use super::mysql::probe_mysql_evidence;
use super::postgres::probe_postgres_evidence;
use super::redis::probe_redis_evidence;
use super::smtp::probe_smtp_evidence;
use super::tcp::probe_tcp_evidence;

#[derive(Debug, Clone)]
pub(crate) struct ProbeEvidence {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub probe_name: String,
    pub success: bool,
    pub available: bool,
    pub banner: Option<String>,
    pub server_header: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

pub(crate) type ProbeStageReporter<'a> = dyn Fn(u32, u32, &str) + Send + Sync + 'a;

pub(crate) async fn probe_target(
    target: ServiceProbeTarget,
    rules: &[ServiceProbeRule],
    timeout_ms: u64,
    follow_http_redirects: bool,
    read_banner: bool,
    stage_reporter: Option<&ProbeStageReporter<'_>>,
) -> Result<ProbeEvidence, crate::service_probe_runtime::ServiceProbeResult> {
    let _ = rules;
    let normalized_target = ServiceProbeTarget {
        host: target.host.trim().to_string(),
        port: target.port,
        protocol: normalize_protocol(&target.protocol, target.port),
    };

    if normalized_target.host.is_empty() || normalized_target.port == 0 {
        return Err(invalid_target_result(
            normalized_target,
            "Invalid service target",
        ));
    }

    let evidence = if is_http_target(normalized_target.port, &normalized_target.protocol) {
        probe_http_evidence(
            &normalized_target,
            timeout_ms,
            follow_http_redirects,
            stage_reporter,
        )
        .await
    } else if normalized_target.port == 6379 || normalized_target.protocol == "redis" {
        probe_redis_evidence(&normalized_target, timeout_ms, stage_reporter).await
    } else if normalized_target.port == 3306 || normalized_target.protocol == "mysql" {
        probe_mysql_evidence(&normalized_target, timeout_ms, stage_reporter).await
    } else if normalized_target.port == 5432
        || normalized_target.protocol == "postgres"
        || normalized_target.protocol == "postgresql"
    {
        probe_postgres_evidence(&normalized_target, timeout_ms, stage_reporter).await
    } else if matches!(normalized_target.port, 25 | 587) || normalized_target.protocol == "smtp" {
        probe_smtp_evidence(&normalized_target, timeout_ms, stage_reporter).await
    } else {
        probe_tcp_evidence(&normalized_target, timeout_ms, read_banner, stage_reporter).await
    };

    Ok(evidence)
}
