use super::common::{is_tls_port, normalize_protocol};
use super::probe::{ProbeEvidence, ProbeStageReporter};
use crate::service_probe_runtime::ServiceProbeTarget;
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub(super) async fn probe_http_evidence(
    target: &ServiceProbeTarget,
    timeout_ms: u64,
    follow_http_redirects: bool,
    stage_reporter: Option<&ProbeStageReporter<'_>>,
) -> ProbeEvidence {
    if let Some(report) = stage_reporter {
        report(1, 4, "Preparing HTTP request");
    }

    let requested_protocol = normalize_protocol(&target.protocol, target.port);
    let protocol = if matches!(requested_protocol.as_str(), "http" | "https") {
        requested_protocol
    } else if is_tls_port(target.port) {
        "https".to_string()
    } else {
        "http".to_string()
    };
    let url = format!("{protocol}://{}:{}/", target.host, target.port);

    let mut client_builder = reqwest::Client::builder().redirect(if follow_http_redirects {
        reqwest::redirect::Policy::limited(5)
    } else {
        reqwest::redirect::Policy::none()
    });
    if let Some(connect_ip) = target.connect_ip.as_deref() {
        if let Ok(connect_ip) = connect_ip.parse::<IpAddr>() {
            client_builder = client_builder.resolve(
                target.host.as_str(),
                SocketAddr::new(connect_ip, target.port),
            );
        }
    }

    let client = match client_builder.build() {
        Ok(client) => client,
        Err(error) => {
            return ProbeEvidence {
                host: target.host.clone(),
                port: target.port,
                protocol,
                probe_name: "http_head".to_string(),
                success: false,
                available: false,
                banner: None,
                server_header: None,
                headers: BTreeMap::new(),
                status_code: None,
                error: Some(format!("Failed creating HTTP client: {error}")),
            };
        }
    };

    match client
        .head(url)
        .timeout(Duration::from_millis(timeout_ms))
        .send()
        .await
    {
        Ok(response) => {
            if let Some(report) = stage_reporter {
                report(2, 4, "Parsing HTTP response headers");
            }
            let mut headers = BTreeMap::new();
            for (name, value) in response.headers().iter() {
                if let Ok(value) = value.to_str() {
                    headers.insert(name.as_str().to_ascii_lowercase(), value.to_string());
                }
            }
            let server_header = response
                .headers()
                .get(reqwest::header::SERVER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            ProbeEvidence {
                success: true,
                available: true,
                host: target.host.clone(),
                port: target.port,
                protocol,
                probe_name: "http_head".to_string(),
                banner: None,
                server_header,
                headers,
                status_code: Some(response.status().as_u16()),
                error: None,
            }
        }
        Err(error) => ProbeEvidence {
            success: false,
            available: false,
            host: target.host.clone(),
            port: target.port,
            protocol,
            probe_name: "http_head".to_string(),
            banner: None,
            server_header: None,
            headers: BTreeMap::new(),
            status_code: None,
            error: Some(error.to_string()),
        },
    }
}
