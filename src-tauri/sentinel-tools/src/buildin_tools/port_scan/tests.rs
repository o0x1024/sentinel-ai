use super::*;
use crate::buildin_tools::port_scan::protocol::RequestedScanPort;
use crate::tool_server::ToolServer;
use serde_json::json;
use std::env;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

struct LocalTcpBannerServer {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl LocalTcpBannerServer {
    async fn spawn(banner: &[u8]) -> Self {
        Self::spawn_with_ports(&[0], banner).await
    }

    async fn spawn_with_ports(candidate_ports: &[u16], banner: &[u8]) -> Self {
        let mut listener = None;
        for port in candidate_ports {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, *port)).await {
                Ok(bound) => {
                    listener = Some(bound);
                    break;
                }
                Err(_) => continue,
            }
        }

        let listener = listener.expect("local test listener should bind");
        let addr = listener
            .local_addr()
            .expect("local test listener should have an address");
        let payload = banner.to_vec();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_result = listener.accept() => {
                        let Ok((mut stream, _)) = accept_result else {
                            break;
                        };

                        if !payload.is_empty() {
                            let _ = stream.write_all(&payload).await;
                        }
                        let _ = stream.shutdown().await;
                    }
                }
            }
        });

        Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
            task,
        }
    }

    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let _ = self.task.await;
    }
}

struct LocalHttpHeadServer {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl LocalHttpHeadServer {
    async fn spawn_on_common_port(server_header: &str) -> Self {
        const CANDIDATE_PORTS: &[u16] = &[8081, 8000, 8888, 9000, 8080];

        let mut listener = None;
        for port in CANDIDATE_PORTS {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, *port)).await {
                Ok(bound) => {
                    listener = Some(bound);
                    break;
                }
                Err(_) => continue,
            }
        }

        let listener = listener.expect("one common HTTP test port should be available");
        let addr = listener
            .local_addr()
            .expect("http test listener should have an address");
        let response = format!(
            "HTTP/1.1 200 OK\r\nServer: {server_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .into_bytes();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_result = listener.accept() => {
                        let Ok((mut stream, _)) = accept_result else {
                            break;
                        };
                        let _ = stream.write_all(&response).await;
                        let _ = stream.shutdown().await;
                    }
                }
            }
        });

        Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
            task,
        }
    }

    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let _ = self.task.await;
    }
}

fn reserve_then_release_local_port() -> u16 {
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("temporary local listener should bind");
    listener
        .local_addr()
        .expect("temporary local listener should have an address")
        .port()
}

fn test_args(port: u16) -> PortScanArgs {
    PortScanArgs {
        engine: PortScanEngine::Connect,
        targets: Vec::new(),
        target: Some("127.0.0.1".to_string()),
        exclude_targets: Vec::new(),
        exclude_target: None,
        port_spec: None,
        ports: vec![port],
        exclude_port_spec: None,
        exclude_ports: Vec::new(),
        timeout_ms: 300,
        concurrency: 16,
        tries: 1,
        device: None,
        raw_ttl: 64,
        raw_source_port_first: None,
        raw_source_port_last: None,
        max_rate: None,
        seed: None,
        shard_index: 1,
        shard_count: 1,
        resume_index: 0,
        resume_count: None,
        service_probe: false,
        read_banner: true,
        follow_http_redirects: true,
        probe_timeout_ms: 500,
        probe_concurrency: 8,
        include_closed: false,
        include_filtered: false,
        max_hosts: 8,
        max_socket_attempts: 128,
        monitor_progress: None,
    }
}

#[derive(Debug, Clone)]
struct RawSynLiveConfig {
    protocol: ScanProtocol,
    device: String,
    target: String,
    open_port: u16,
    closed_port: u16,
    timeout_ms: u64,
    concurrency: usize,
    tries: u8,
}

impl RawSynLiveConfig {
    fn tcp_from_env() -> Self {
        Self::from_env("SENTINEL_PORT_SCAN_RAW_SYN", ScanProtocol::Tcp)
    }

    fn udp_from_env() -> Self {
        Self::from_env("SENTINEL_PORT_SCAN_RAW_UDP", ScanProtocol::Udp)
    }

    fn sctp_from_env() -> Self {
        Self::from_env("SENTINEL_PORT_SCAN_RAW_SCTP", ScanProtocol::Sctp)
    }

    fn from_env(env_prefix: &'static str, protocol: ScanProtocol) -> Self {
        Self {
            protocol,
            device: required_env(&format!("{}_DEVICE", env_prefix)),
            target: required_env(&format!("{}_TARGET", env_prefix)),
            open_port: required_env_parsed(&format!("{}_OPEN_PORT", env_prefix)),
            closed_port: required_env_parsed(&format!("{}_CLOSED_PORT", env_prefix)),
            timeout_ms: optional_env_parsed(&format!("{}_TIMEOUT_MS", env_prefix), 1_500),
            concurrency: optional_env_parsed(&format!("{}_CONCURRENCY", env_prefix), 256),
            tries: optional_env_parsed(&format!("{}_TRIES", env_prefix), 2),
        }
    }
}

impl RawSynLiveConfig {
    fn requested_ports(&self) -> Vec<PortScanSocketRef> {
        vec![
            PortScanSocketRef {
                port: self.open_port,
                protocol: self.protocol.label().to_string(),
            },
            PortScanSocketRef {
                port: self.closed_port,
                protocol: self.protocol.label().to_string(),
            },
        ]
    }

    fn into_args(self) -> PortScanArgs {
        let (ports, port_spec) = match self.protocol {
            ScanProtocol::Tcp => (vec![self.open_port, self.closed_port], None),
            ScanProtocol::Udp => (
                Vec::new(),
                Some(format!("U:{},U:{}", self.open_port, self.closed_port)),
            ),
            ScanProtocol::Sctp => (
                Vec::new(),
                Some(format!("S:{},S:{}", self.open_port, self.closed_port)),
            ),
        };

        PortScanArgs {
            engine: PortScanEngine::RawSyn,
            targets: Vec::new(),
            target: Some(self.target),
            exclude_targets: Vec::new(),
            exclude_target: None,
            port_spec,
            ports,
            exclude_port_spec: None,
            exclude_ports: Vec::new(),
            timeout_ms: self.timeout_ms,
            concurrency: self.concurrency,
            tries: self.tries,
            device: Some(self.device),
            raw_ttl: 64,
            raw_source_port_first: None,
            raw_source_port_last: None,
            max_rate: None,
            seed: None,
            shard_index: 1,
            shard_count: 1,
            resume_index: 0,
            resume_count: None,
            service_probe: false,
            read_banner: true,
            follow_http_redirects: true,
            probe_timeout_ms: 1_000,
            probe_concurrency: 8,
            include_closed: true,
            include_filtered: false,
            max_hosts: 8,
            max_socket_attempts: 512,
            monitor_progress: None,
        }
    }
}

fn required_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} must be set for raw_syn live smoke tests", key))
}

fn required_env_parsed<T>(key: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = required_env(key);
    raw.parse::<T>()
        .unwrap_or_else(|error| panic!("{} must be a valid value: {}", key, error))
}

fn optional_env_parsed<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw
            .parse::<T>()
            .unwrap_or_else(|error| panic!("{} must be a valid value: {}", key, error)),
        Err(_) => default,
    }
}

fn assert_raw_live_smoke_result(result: &PortScanOutput, config: &RawSynLiveConfig) {
    assert_eq!(result.data.summary.total_requested_ports, 2);
    assert_eq!(result.data.summary.total_open_ports, 1);
    assert_eq!(result.data.summary.total_closed_ports, 1);
    assert_eq!(result.data.summary.total_filtered_ports, 0);
    assert_eq!(result.data.requested_ports, config.requested_ports());
    assert!(
        result.data.ports.iter().any(|entry| {
            entry.ip == config.target
                && entry.port == config.open_port
                && entry.protocol == config.protocol.label()
                && entry.status == "open"
        }),
        "expected raw_syn to report {} {}:{} as open",
        config.protocol.label(),
        config.target,
        config.open_port
    );
    assert!(
        result.data.ports.iter().any(|entry| {
            entry.ip == config.target
                && entry.port == config.closed_port
                && entry.protocol == config.protocol.label()
                && entry.status == "closed"
        }),
        "expected raw_syn to report {} {}:{} as closed",
        config.protocol.label(),
        config.target,
        config.closed_port
    );
    assert!(
        result.data.results.iter().all(|host| host.error.is_none()),
        "unexpected host errors in raw_syn live smoke result for {}: {:?}",
        config.protocol.label(),
        result.data.results
    );
}

#[test]
fn parses_masscan_style_port_ranges() {
    let args = PortScanArgs {
        engine: PortScanEngine::Connect,
        targets: Vec::new(),
        target: None,
        exclude_targets: Vec::new(),
        exclude_target: None,
        port_spec: Some("80,443,8000-8002".to_string()),
        ports: Vec::new(),
        exclude_port_spec: None,
        exclude_ports: Vec::new(),
        timeout_ms: default_connect_timeout_ms(),
        concurrency: default_scan_concurrency(),
        tries: default_connect_tries(),
        device: None,
        raw_ttl: default_raw_ttl(),
        raw_source_port_first: None,
        raw_source_port_last: None,
        max_rate: None,
        seed: None,
        shard_index: 1,
        shard_count: 1,
        resume_index: 0,
        resume_count: None,
        service_probe: false,
        read_banner: default_read_banner(),
        follow_http_redirects: default_follow_http_redirects(),
        probe_timeout_ms: default_probe_timeout_ms(),
        probe_concurrency: default_probe_concurrency(),
        include_closed: false,
        include_filtered: false,
        max_hosts: default_max_hosts(),
        max_socket_attempts: default_max_socket_attempts(),
        monitor_progress: None,
    };

    assert_eq!(
        normalize_requested_ports(&args).expect("port spec should parse"),
        vec![
            RequestedScanPort {
                port: 80,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 443,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 8000,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 8001,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 8002,
                protocol: ScanProtocol::Tcp,
            },
        ]
    );
}

#[test]
fn parses_protocol_prefixed_port_tokens() {
    let args = PortScanArgs {
        port_spec: Some("80,U:53,udp:123,S:2905".to_string()),
        ports: Vec::new(),
        ..test_args(80)
    };

    assert_eq!(
        normalize_requested_ports(&args).expect("protocol-prefixed spec should parse"),
        vec![
            RequestedScanPort {
                port: 53,
                protocol: ScanProtocol::Udp,
            },
            RequestedScanPort {
                port: 80,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 123,
                protocol: ScanProtocol::Udp,
            },
            RequestedScanPort {
                port: 2905,
                protocol: ScanProtocol::Sctp,
            },
        ]
    );
}

#[test]
fn parses_excluded_port_ranges() {
    let args = PortScanArgs {
        exclude_port_spec: Some("22,443,8000-8001".to_string()),
        ..test_args(80)
    };

    assert_eq!(
        normalize_excluded_ports(&args)
            .expect("exclude port spec should parse")
            .into_iter()
            .collect::<Vec<_>>(),
        vec![
            RequestedScanPort {
                port: 22,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 443,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 8000,
                protocol: ScanProtocol::Tcp,
            },
            RequestedScanPort {
                port: 8001,
                protocol: ScanProtocol::Tcp,
            },
        ]
    );
}

#[test]
fn normalizes_excluded_targets() {
    let args = PortScanArgs {
        exclude_target: Some("10.0.0.1".to_string()),
        exclude_targets: vec!["10.0.0.2,10.0.0.3".to_string()],
        ..test_args(80)
    };

    assert_eq!(
        normalize_excluded_targets(&args),
        vec![
            "10.0.0.1".to_string(),
            "10.0.0.2".to_string(),
            "10.0.0.3".to_string()
        ]
    );
}

#[test]
fn expands_ipv4_cidr_targets() {
    let hosts = targeting::expand_target_spec("192.168.1.0/30", 4).expect("CIDR should expand");
    assert_eq!(
        hosts,
        vec![
            "192.168.1.0".to_string(),
            "192.168.1.1".to_string(),
            "192.168.1.2".to_string(),
            "192.168.1.3".to_string()
        ]
    );
}

#[test]
fn expands_ipv4_range_targets() {
    let hosts = targeting::expand_target_spec("10.0.0.1-10.0.0.3", 3).expect("range should expand");
    assert_eq!(
        hosts,
        vec![
            "10.0.0.1".to_string(),
            "10.0.0.2".to_string(),
            "10.0.0.3".to_string()
        ]
    );
}

#[test]
fn keeps_domain_targets_as_is() {
    let hosts =
        targeting::expand_target_spec("api.example.com", 10).expect("domain should pass through");
    assert_eq!(hosts, vec!["api.example.com".to_string()]);
}

#[test]
fn rejects_over_budget_cidr() {
    let error = targeting::expand_target_spec("10.0.0.0/24", 32).expect_err("budget should reject");
    assert!(error
        .to_string()
        .contains("exceeding remaining host budget"));
}

#[tokio::test]
async fn port_scan_detects_open_local_port() {
    let server = LocalTcpBannerServer::spawn(&[]).await;
    let args = test_args(server.addr.port());
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert!(result.success);
    assert_eq!(result.data.summary.total_open_ports, 1);
    assert_eq!(result.data.summary.total_closed_ports, 0);
    assert_eq!(result.data.summary.total_filtered_ports, 0);
    assert_eq!(result.data.summary.successful_scans, 1);
    assert_eq!(result.data.results.len(), 1);
    assert_eq!(result.data.results[0].host, "127.0.0.1");
    assert_eq!(
        result.data.results[0].open_ports,
        vec![PortScanSocketRef {
            port: server.addr.port(),
            protocol: "tcp".to_string(),
        }]
    );
    assert_eq!(result.data.ports.len(), 1);
    assert_eq!(result.data.ports[0].port, server.addr.port());
    assert_eq!(result.data.ports[0].ip, "127.0.0.1");
    assert_eq!(result.data.ports[0].protocol, "tcp");
    assert_eq!(result.data.ports[0].status, "open");
    assert_eq!(result.data.ports[0].reason.as_deref(), Some("connect"));
    assert!(result.data.ports[0].ttl.is_none());
    assert!(result.data.ports[0].service_name.is_none());

    server.shutdown().await;
}

#[tokio::test]
async fn port_scan_reports_closed_local_port() {
    let closed_port = reserve_then_release_local_port();
    let args = test_args(closed_port);
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert!(result.success);
    assert_eq!(result.data.summary.total_open_ports, 0);
    assert_eq!(result.data.summary.total_closed_ports, 1);
    assert_eq!(result.data.summary.total_filtered_ports, 0);
    assert!(result.data.results[0].closed_ports.is_empty());
    assert!(result.data.ports.is_empty());
}

#[tokio::test]
async fn port_scan_includes_closed_local_port_when_requested() {
    let closed_port = reserve_then_release_local_port();
    let mut args = test_args(closed_port);
    args.include_closed = true;
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert_eq!(
        result.data.results[0].closed_ports,
        vec![PortScanSocketRef {
            port: closed_port,
            protocol: "tcp".to_string(),
        }]
    );
    assert_eq!(result.data.ports.len(), 1);
    assert_eq!(result.data.ports[0].port, closed_port);
    assert_eq!(result.data.ports[0].status, "closed");
    assert_eq!(
        result.data.ports[0].reason.as_deref(),
        Some("connection_refused")
    );
    assert!(result.data.ports[0].ttl.is_none());
}

#[tokio::test]
async fn port_scan_merges_service_probe_banner_fields() {
    let server = LocalTcpBannerServer::spawn_with_ports(&[2222], b"SSH-2.0-OpenSSH_9.7\r\n").await;
    let mut args = test_args(server.addr.port());
    args.service_probe = true;
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert!(result.success);
    assert_eq!(result.data.summary.total_open_ports, 1);
    assert_eq!(result.data.summary.total_closed_ports, 0);
    assert_eq!(result.data.summary.service_probed_ports, 1);
    assert_eq!(
        result.data.ports[0].banner.as_deref(),
        Some("SSH-2.0-OpenSSH_9.7")
    );
    assert_eq!(result.data.ports[0].service_name.as_deref(), Some("ssh"));
    assert!(result.data.ports[0].error.is_none());

    server.shutdown().await;
}

#[tokio::test]
async fn port_scan_skips_service_probe_for_non_common_ports() {
    let server = LocalTcpBannerServer::spawn(b"SSH-2.0-OpenSSH_9.7\r\n").await;
    let mut args = test_args(server.addr.port());
    args.service_probe = true;
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert_eq!(result.data.summary.service_probed_ports, 0);
    assert!(result.data.ports[0].service_name.is_none());
    assert!(result.data.ports[0].banner.is_none());

    server.shutdown().await;
}

#[tokio::test]
async fn port_scan_keeps_open_result_when_common_http_probe_fails() {
    let server = LocalTcpBannerServer::spawn_with_ports(&[8081, 8000, 8888], b"NOTHTTP\r\n").await;
    let mut args = test_args(server.addr.port());
    args.service_probe = true;
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert!(result.success);
    assert_eq!(result.data.summary.total_open_ports, 1);
    assert_eq!(result.data.summary.service_probed_ports, 0);
    assert!(result.data.summary.service_probe_error.is_none());
    assert!(result.data.ports[0].service_name.is_none());
    assert!(result.data.ports[0].server_header.is_none());
    assert!(result.data.ports[0].error.is_some());

    server.shutdown().await;
}

#[tokio::test]
async fn port_scan_respects_excluded_ports() {
    let server = LocalTcpBannerServer::spawn(&[]).await;
    let closed_port = reserve_then_release_local_port();
    let mut args = test_args(server.addr.port());
    args.ports = vec![server.addr.port(), closed_port];
    args.exclude_port_spec = Some(server.addr.port().to_string());
    let tool = PortScanTool;

    let result = tool.call(args).await.expect("port scan should succeed");

    assert_eq!(result.data.summary.total_requested_ports, 1);
    assert_eq!(result.data.summary.total_open_ports, 0);
    assert_eq!(result.data.summary.total_closed_ports, 1);
    assert!(result.data.ports.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn port_scan_rejects_fully_excluded_targets() {
    let mut args = test_args(80);
    args.exclude_target = Some("127.0.0.1".to_string());
    let tool = PortScanTool;

    let error = tool
        .call(args)
        .await
        .expect_err("fully excluded target set should fail");

    assert!(
        error
            .to_string()
            .contains("no scanable hosts remained after normalization"),
        "unexpected error: {}",
        error
    );
}

#[tokio::test]
async fn connect_engine_rejects_udp_requested_ports() {
    let tool = PortScanTool;
    let error = tool
        .call(PortScanArgs {
            engine: PortScanEngine::Connect,
            port_spec: Some("U:53".to_string()),
            ports: Vec::new(),
            ..test_args(80)
        })
        .await
        .expect_err("connect engine should reject UDP ports");

    assert!(error
        .to_string()
        .contains("connect engine only supports TCP ports"));
}

#[tokio::test]
async fn tool_server_executes_port_scan() {
    let server = LocalTcpBannerServer::spawn(&[]).await;
    let tool_server = ToolServer::new();
    tool_server.init_builtin_tools().await;

    let result = tool_server
        .execute(
            "port_scan",
            json!({
                "engine": "connect",
                "target": "127.0.0.1",
                "ports": [server.addr.port()],
                "timeout_ms": 300,
                "concurrency": 16,
                "tries": 1,
                "service_probe": false,
                "max_hosts": 8,
                "max_socket_attempts": 128
            }),
        )
        .await;

    assert!(
        result.success,
        "tool server execution should succeed: {:?}",
        result.error
    );
    let output = result.output.expect("tool server should return output");
    assert_eq!(
        output
            .get("data")
            .and_then(|value| value.get("summary"))
            .and_then(|value| value.get("total_open_ports"))
            .and_then(|value| value.as_u64()),
        Some(1)
    );

    server.shutdown().await;
}

#[tokio::test]
async fn tool_server_port_scan_defaults_probe_common_http() {
    let server = LocalHttpHeadServer::spawn_on_common_port("TestHTTP/1.0").await;
    let tool_server = ToolServer::new();
    tool_server.init_builtin_tools().await;

    let result = tool_server
        .execute(
            "port_scan",
            json!({
                "target": "127.0.0.1",
                "ports": [server.addr.port()],
                "timeout_ms": 300,
                "concurrency": 16,
                "tries": 1,
                "max_hosts": 8,
                "max_socket_attempts": 128
            }),
        )
        .await;

    assert!(result.success, "tool server execution should succeed");
    let output = result.output.expect("tool server should return output");
    assert_eq!(
        output
            .get("data")
            .and_then(|value| value.get("summary"))
            .and_then(|value| value.get("service_probed_ports"))
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        output
            .get("data")
            .and_then(|value| value.get("ports"))
            .and_then(|value| value.as_array())
            .and_then(|ports| ports.first())
            .and_then(|value| value.get("service_name"))
            .and_then(|value| value.as_str()),
        Some("http")
    );

    server.shutdown().await;
}

#[test]
fn raw_syn_builds_ipv6_only_hosts_as_errors() {
    let mut accumulators = BTreeMap::new();
    accumulators.insert("::1".to_string(), HostAccumulator::new("::1"));
    let targets = collect_socket_targets(
        vec![targeting::ResolvedHost {
            host: "::1".to_string(),
            resolved_ips: Ok(vec!["::1".parse().expect("ipv6 loopback")]),
        }],
        PortScanEngine::RawSyn,
        &mut accumulators,
    );

    assert!(targets.is_empty());
    assert_eq!(
        accumulators
            .get("::1")
            .and_then(|entry| entry.error.as_deref()),
        Some("raw_syn engine currently supports IPv4 targets only")
    );
}

#[tokio::test]
#[ignore = "requires NET_RAW privileges plus SENTINEL_PORT_SCAN_RAW_SYN_DEVICE/TARGET/OPEN_PORT/CLOSED_PORT"]
async fn raw_syn_live_smoke_detects_open_and_closed_ports() {
    let config = RawSynLiveConfig::tcp_from_env();
    let tool = PortScanTool;
    let result = tool
        .call(config.clone().into_args())
        .await
        .expect("raw_syn live smoke scan should complete");

    assert_raw_live_smoke_result(&result, &config);
}

#[tokio::test]
#[ignore = "requires NET_RAW privileges plus SENTINEL_PORT_SCAN_RAW_UDP_DEVICE/TARGET/OPEN_PORT/CLOSED_PORT"]
async fn raw_udp_live_smoke_detects_open_and_closed_ports() {
    let config = RawSynLiveConfig::udp_from_env();
    let tool = PortScanTool;
    let result = tool
        .call(config.clone().into_args())
        .await
        .expect("raw_udp live smoke scan should complete");

    assert_raw_live_smoke_result(&result, &config);
}

#[tokio::test]
#[ignore = "requires NET_RAW privileges plus SENTINEL_PORT_SCAN_RAW_SCTP_DEVICE/TARGET/OPEN_PORT/CLOSED_PORT"]
async fn raw_sctp_live_smoke_detects_open_and_closed_ports() {
    let config = RawSynLiveConfig::sctp_from_env();
    let tool = PortScanTool;
    let result = tool
        .call(config.clone().into_args())
        .await
        .expect("raw_sctp live smoke scan should complete");

    assert_raw_live_smoke_result(&result, &config);
}
