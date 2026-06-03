use super::common::{normalize_banner, normalize_protocol, MAX_BANNER_BYTES};
use super::probe::{ProbeEvidence, ProbeStageReporter};
use crate::service_probe_runtime::ServiceProbeTarget;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub(super) async fn probe_postgres_evidence(
    target: &ServiceProbeTarget,
    timeout_ms: u64,
    stage_reporter: Option<&ProbeStageReporter<'_>>,
) -> ProbeEvidence {
    if let Some(report) = stage_reporter {
        report(1, 4, "Opening PostgreSQL connection");
    }

    let protocol = normalize_protocol(&target.protocol, target.port);
    let connect_host = target.connect_ip.as_deref().unwrap_or(target.host.as_str());
    let connect_result = timeout(
        Duration::from_millis(timeout_ms),
        TcpStream::connect((connect_host, target.port)),
    )
    .await;

    let mut stream = match connect_result {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => {
            return ProbeEvidence {
                host: target.host.clone(),
                port: target.port,
                protocol,
                probe_name: "postgres_ssl_request".to_string(),
                success: false,
                available: false,
                banner: None,
                server_header: None,
                headers: BTreeMap::new(),
                status_code: None,
                error: Some(error.to_string()),
            };
        }
        Err(_) => {
            return ProbeEvidence {
                host: target.host.clone(),
                port: target.port,
                protocol,
                probe_name: "postgres_ssl_request".to_string(),
                success: false,
                available: false,
                banner: None,
                server_header: None,
                headers: BTreeMap::new(),
                status_code: None,
                error: Some("PostgreSQL connect timed out".to_string()),
            };
        }
    };

    if let Some(report) = stage_reporter {
        report(2, 4, "Sending PostgreSQL SSL probe");
    }

    let ssl_request: [u8; 8] = [0, 0, 0, 8, 4, 210, 22, 47];
    let _ = timeout(
        Duration::from_millis(timeout_ms),
        stream.write_all(&ssl_request),
    )
    .await;

    let mut buffer = [0_u8; MAX_BANNER_BYTES];
    let banner = match timeout(Duration::from_millis(timeout_ms), stream.read(&mut buffer)).await {
        Ok(Ok(size)) if size > 0 => {
            if size == 1 {
                match buffer[0] {
                    b'S' => Some("PostgreSQL SSL supported".to_string()),
                    b'N' => Some("PostgreSQL SSL not supported".to_string()),
                    _ => normalize_banner(&buffer[..size]),
                }
            } else {
                normalize_banner(&buffer[..size])
            }
        }
        _ => None,
    };

    ProbeEvidence {
        success: true,
        available: true,
        host: target.host.clone(),
        port: target.port,
        protocol,
        probe_name: "postgres_ssl_request".to_string(),
        banner,
        server_header: None,
        headers: BTreeMap::new(),
        status_code: None,
        error: None,
    }
}
