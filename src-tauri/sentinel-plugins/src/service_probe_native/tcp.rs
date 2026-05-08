use super::common::{normalize_banner, normalize_protocol, MAX_BANNER_BYTES};
use super::probe::{ProbeEvidence, ProbeStageReporter};
use crate::service_probe_runtime::ServiceProbeTarget;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub(super) async fn probe_tcp_evidence(
    target: &ServiceProbeTarget,
    timeout_ms: u64,
    read_banner: bool,
    stage_reporter: Option<&ProbeStageReporter<'_>>,
) -> ProbeEvidence {
    if let Some(report) = stage_reporter {
        report(1, 4, "Opening TCP connection");
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
                protocol: protocol.clone(),
                probe_name: "tcp_banner".to_string(),
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
                protocol: protocol.clone(),
                probe_name: "tcp_banner".to_string(),
                success: false,
                available: false,
                banner: None,
                server_header: None,
                headers: BTreeMap::new(),
                status_code: None,
                error: Some("TCP connect timed out".to_string()),
            };
        }
    };

    if let Some(report) = stage_reporter {
        report(
            2,
            4,
            if read_banner {
                "Reading TCP banner"
            } else {
                "TCP connection established"
            },
        );
    }

    let banner = if read_banner {
        let mut buffer = [0_u8; MAX_BANNER_BYTES];
        match timeout(Duration::from_millis(timeout_ms), stream.read(&mut buffer)).await {
            Ok(Ok(size)) if size > 0 => normalize_banner(&buffer[..size]),
            Ok(Ok(_)) | Ok(Err(_)) | Err(_) => None,
        }
    } else {
        None
    };

    ProbeEvidence {
        success: true,
        available: true,
        host: target.host.clone(),
        port: target.port,
        protocol,
        probe_name: "tcp_banner".to_string(),
        banner,
        server_header: None,
        headers: BTreeMap::new(),
        status_code: None,
        error: None,
    }
}
