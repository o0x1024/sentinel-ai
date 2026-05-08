use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::IpNet;

use super::protocol::{PortScanSocketRef, RequestedScanPort, ScanProtocol};
use super::{PortScanArgs, PortScanEngine, PortScanError, PortScanHostResult};

#[derive(Debug, Clone)]
pub(crate) struct SocketScanTask {
    pub host: String,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: ScanProtocol,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSocketTarget {
    pub host: String,
    pub ip: IpAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SocketStatus {
    Open,
    Closed,
    Filtered,
}

impl SocketStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Filtered => "filtered",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SocketOutcome {
    pub host: String,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: ScanProtocol,
    pub status: SocketStatus,
    pub reason: Option<String>,
    pub ttl: Option<u8>,
}

impl SocketOutcome {
    pub(crate) fn new(
        host: String,
        ip: IpAddr,
        port: u16,
        protocol: ScanProtocol,
        status: SocketStatus,
        reason: Option<String>,
        ttl: Option<u8>,
    ) -> Self {
        Self {
            host,
            ip,
            port,
            protocol,
            status,
            reason,
            ttl,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedHost {
    pub host: String,
    pub resolved_ips: Result<Vec<IpAddr>, String>,
}

#[derive(Debug)]
pub(crate) struct HostAccumulator {
    pub host: String,
    pub resolved_ips: BTreeSet<String>,
    pub open_ports: BTreeSet<RequestedScanPort>,
    pub closed_ports: BTreeSet<RequestedScanPort>,
    pub filtered_ports: BTreeSet<RequestedScanPort>,
    pub open_socket_count: usize,
    pub closed_socket_count: usize,
    pub filtered_socket_count: usize,
    pub error: Option<String>,
}

impl HostAccumulator {
    pub(crate) fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            resolved_ips: BTreeSet::new(),
            open_ports: BTreeSet::new(),
            closed_ports: BTreeSet::new(),
            filtered_ports: BTreeSet::new(),
            open_socket_count: 0,
            closed_socket_count: 0,
            filtered_socket_count: 0,
            error: None,
        }
    }

    pub(crate) fn into_result(
        self,
        include_closed: bool,
        include_filtered: bool,
    ) -> PortScanHostResult {
        let Self {
            host,
            resolved_ips,
            open_ports,
            closed_ports,
            filtered_ports,
            open_socket_count,
            closed_socket_count,
            filtered_socket_count,
            error,
        } = self;

        PortScanHostResult {
            host,
            resolved_ips: resolved_ips.into_iter().collect(),
            open_ports: open_ports
                .into_iter()
                .map(PortScanSocketRef::from)
                .collect(),
            closed_ports: include_closed
                .then(|| {
                    closed_ports
                        .into_iter()
                        .map(PortScanSocketRef::from)
                        .collect()
                })
                .unwrap_or_default(),
            filtered_ports: include_filtered
                .then(|| {
                    filtered_ports
                        .into_iter()
                        .map(PortScanSocketRef::from)
                        .collect()
                })
                .unwrap_or_default(),
            open_socket_count,
            closed_socket_count,
            filtered_socket_count,
            error,
        }
    }

    pub(crate) fn record_outcome(&mut self, outcome: &SocketOutcome) {
        let requested_port = RequestedScanPort {
            port: outcome.port,
            protocol: outcome.protocol,
        };

        match outcome.status {
            SocketStatus::Open => {
                self.open_ports.insert(requested_port);
                self.open_socket_count += 1;
            }
            SocketStatus::Closed => {
                self.closed_ports.insert(requested_port);
                self.closed_socket_count += 1;
            }
            SocketStatus::Filtered => {
                self.filtered_ports.insert(requested_port);
                self.filtered_socket_count += 1;
            }
        }
    }
}

pub(crate) fn normalize_input_targets(args: &PortScanArgs) -> Vec<String> {
    normalize_target_tokens(args.target.as_deref(), &args.targets)
}

pub(crate) fn normalize_excluded_targets(args: &PortScanArgs) -> Vec<String> {
    normalize_target_tokens(args.exclude_target.as_deref(), &args.exclude_targets)
}

pub(crate) fn normalize_requested_ports(
    args: &PortScanArgs,
) -> Result<Vec<RequestedScanPort>, PortScanError> {
    let mut ports = BTreeSet::new();
    for port in &args.ports {
        if *port == 0 {
            return Err(PortScanError::InvalidArgs(
                "port 0 is not valid for port scans".to_string(),
            ));
        }
        ports.insert(RequestedScanPort {
            port: *port,
            protocol: ScanProtocol::Tcp,
        });
    }

    if let Some(spec) = args.port_spec.as_deref() {
        for port in parse_port_spec(spec)? {
            ports.insert(port);
        }
    }

    if ports.is_empty() {
        return Err(PortScanError::InvalidArgs(
            "at least one port is required via ports or port_spec".to_string(),
        ));
    }

    Ok(ports.into_iter().collect())
}

pub(crate) fn normalize_excluded_ports(
    args: &PortScanArgs,
) -> Result<BTreeSet<RequestedScanPort>, PortScanError> {
    let mut ports = BTreeSet::new();
    for port in &args.exclude_ports {
        if *port == 0 {
            return Err(PortScanError::InvalidArgs(
                "excluded port 0 is not valid for port scans".to_string(),
            ));
        }
        ports.insert(RequestedScanPort {
            port: *port,
            protocol: ScanProtocol::Tcp,
        });
    }

    if let Some(spec) = args.exclude_port_spec.as_deref() {
        for port in parse_port_spec(spec)? {
            ports.insert(port);
        }
    }

    Ok(ports)
}

pub(crate) fn collect_socket_targets(
    resolution_results: Vec<ResolvedHost>,
    engine: PortScanEngine,
    host_accumulators: &mut BTreeMap<String, HostAccumulator>,
) -> Vec<ResolvedSocketTarget> {
    let mut targets = Vec::new();
    for resolved in resolution_results {
        if let Some(accumulator) = host_accumulators.get_mut(&resolved.host) {
            match resolved.resolved_ips {
                Ok(ips) => {
                    accumulator
                        .resolved_ips
                        .extend(ips.iter().map(std::string::ToString::to_string));

                    let mut queued_for_host = false;
                    let mut skipped_ipv6 = false;
                    for ip in ips {
                        if engine == PortScanEngine::RawSyn && matches!(ip, IpAddr::V6(_)) {
                            skipped_ipv6 = true;
                            continue;
                        }

                        queued_for_host = true;
                        targets.push(ResolvedSocketTarget {
                            host: resolved.host.clone(),
                            ip,
                        });
                    }

                    if engine == PortScanEngine::RawSyn
                        && !queued_for_host
                        && skipped_ipv6
                        && accumulator.error.is_none()
                    {
                        accumulator.error =
                            Some("raw_syn engine currently supports IPv4 targets only".to_string());
                    }
                }
                Err(error) => {
                    accumulator.error = Some(error);
                }
            }
        }
    }

    targets
}

pub(crate) fn build_socket_tasks(
    targets: &[ResolvedSocketTarget],
    requested_ports: &[RequestedScanPort],
) -> Vec<SocketScanTask> {
    let mut socket_tasks = Vec::with_capacity(targets.len().saturating_mul(requested_ports.len()));
    for target in targets {
        for port in requested_ports {
            socket_tasks.push(SocketScanTask {
                host: target.host.clone(),
                ip: target.ip,
                port: port.port,
                protocol: port.protocol,
            });
        }
    }

    socket_tasks
}

pub(crate) fn count_socket_attempts(
    target_count: usize,
    port_count: usize,
) -> Result<usize, PortScanError> {
    target_count
        .checked_mul(port_count)
        .ok_or_else(|| PortScanError::InvalidArgs("scan scope overflowed usize".to_string()))
}

fn split_list_tokens(input: &str) -> Vec<String> {
    input
        .split(|ch: char| ch == ',' || ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_target_tokens(single: Option<&str>, many: &[String]) -> Vec<String> {
    let mut merged = Vec::new();
    if let Some(target) = single.map(str::trim).filter(|value| !value.is_empty()) {
        merged.push(target.to_string());
    }
    merged.extend(many.iter().cloned());

    let mut dedup = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in merged {
        for token in split_list_tokens(&value) {
            if dedup.insert(token.clone()) {
                normalized.push(token);
            }
        }
    }
    normalized
}

fn parse_port_spec(spec: &str) -> Result<Vec<RequestedScanPort>, PortScanError> {
    let mut ports = BTreeSet::new();
    for token in split_list_tokens(spec) {
        let (protocol, raw_token) = split_protocol_token(&token)?;

        if let Some((start, end)) = raw_token.split_once('-') {
            let start = parse_single_port(start)?;
            let end = parse_single_port(end)?;
            if start > end {
                return Err(PortScanError::InvalidArgs(format!(
                    "invalid port range `{}`: start is greater than end",
                    token
                )));
            }
            for port in start..=end {
                ports.insert(RequestedScanPort { port, protocol });
            }
            continue;
        }

        ports.insert(RequestedScanPort {
            port: parse_single_port(raw_token)?,
            protocol,
        });
    }

    Ok(ports.into_iter().collect())
}

fn split_protocol_token(token: &str) -> Result<(ScanProtocol, &str), PortScanError> {
    if let Some((prefix, rest)) = token.split_once(':') {
        let protocol = ScanProtocol::from_prefix(prefix).ok_or_else(|| {
            PortScanError::InvalidArgs(format!(
                "unsupported protocol prefix `{}` in port token `{}`",
                prefix, token
            ))
        })?;
        let normalized = rest.trim();
        if normalized.is_empty() {
            return Err(PortScanError::InvalidArgs(format!(
                "missing port value in token `{}`",
                token
            )));
        }
        return Ok((protocol, normalized));
    }

    Ok((ScanProtocol::Tcp, token))
}

fn parse_single_port(raw: &str) -> Result<u16, PortScanError> {
    let port = raw
        .trim()
        .parse::<u16>()
        .map_err(|_| PortScanError::InvalidArgs(format!("invalid port `{}`", raw.trim())))?;
    if port == 0 {
        return Err(PortScanError::InvalidArgs(
            "port 0 is not valid for scans".to_string(),
        ));
    }
    Ok(port)
}

pub(crate) fn expand_target_specs(
    input_targets: &[String],
    max_hosts: usize,
) -> Result<Vec<String>, PortScanError> {
    if max_hosts == 0 {
        return Err(PortScanError::InvalidArgs(
            "max_hosts must be greater than 0".to_string(),
        ));
    }

    let mut expanded = Vec::new();
    let mut dedup = BTreeSet::new();
    for spec in input_targets {
        let hosts = expand_target_spec(spec, max_hosts)?;
        for host in hosts {
            if dedup.insert(host.clone()) {
                expanded.push(host);
                if expanded.len() > max_hosts {
                    return Err(PortScanError::InvalidArgs(format!(
                        "expanded host count exceeded max_hosts {}",
                        max_hosts
                    )));
                }
            }
        }
    }
    Ok(expanded)
}

pub(crate) fn expand_target_spec(
    spec: &str,
    remaining_budget: usize,
) -> Result<Vec<String>, PortScanError> {
    let normalized = normalize_host(spec);
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(ip) = normalized.parse::<IpAddr>() {
        return Ok(vec![ip.to_string()]);
    }

    if let Ok(net) = normalized.parse::<IpNet>() {
        return expand_cidr(net, remaining_budget);
    }

    if let Some((start, end)) = parse_ip_range(&normalized)? {
        return expand_ip_range(start, end, remaining_budget);
    }

    Ok(vec![normalized])
}

fn normalize_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim()
        .to_string()
}

fn parse_ip_range(spec: &str) -> Result<Option<(IpAddr, IpAddr)>, PortScanError> {
    let Some((raw_start, raw_end)) = spec.split_once('-') else {
        return Ok(None);
    };

    let raw_start = normalize_host(raw_start);
    let raw_end = normalize_host(raw_end);
    if raw_start.is_empty() || raw_end.is_empty() {
        return Ok(None);
    }

    let start = match raw_start.parse::<IpAddr>() {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let end = match raw_end.parse::<IpAddr>() {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    Ok(Some((start, end)))
}

fn expand_cidr(net: IpNet, remaining_budget: usize) -> Result<Vec<String>, PortScanError> {
    match net {
        IpNet::V4(v4) => {
            let prefix = v4.prefix_len() as u32;
            let host_bits = 32_u32.saturating_sub(prefix);
            let count = checked_host_count(host_bits, remaining_budget, &v4.to_string())?;
            let base = u32::from(v4.network());
            Ok((0..count)
                .map(|offset| Ipv4Addr::from(base + offset as u32).to_string())
                .collect())
        }
        IpNet::V6(v6) => {
            let prefix = v6.prefix_len() as u32;
            let host_bits = 128_u32.saturating_sub(prefix);
            let count = checked_host_count(host_bits, remaining_budget, &v6.to_string())?;
            let base = ipv6_to_u128(v6.network());
            Ok((0..count)
                .map(|offset| Ipv6Addr::from(base + offset as u128).to_string())
                .collect())
        }
    }
}

fn expand_ip_range(
    start: IpAddr,
    end: IpAddr,
    remaining_budget: usize,
) -> Result<Vec<String>, PortScanError> {
    match (start, end) {
        (IpAddr::V4(start), IpAddr::V4(end)) => {
            let start = u32::from(start);
            let end = u32::from(end);
            if start > end {
                return Err(PortScanError::InvalidArgs(
                    "invalid IPv4 range: start is greater than end".to_string(),
                ));
            }
            let count = end
                .checked_sub(start)
                .and_then(|delta| delta.checked_add(1))
                .map(|count| count as usize)
                .ok_or_else(|| {
                    PortScanError::InvalidArgs("IPv4 range size overflowed usize".to_string())
                })?;
            if count > remaining_budget {
                return Err(PortScanError::InvalidArgs(format!(
                    "IPv4 range {}-{} expands to {} hosts, exceeding remaining host budget {}",
                    Ipv4Addr::from(start),
                    Ipv4Addr::from(end),
                    count,
                    remaining_budget
                )));
            }
            Ok((start..=end)
                .map(|value| Ipv4Addr::from(value).to_string())
                .collect())
        }
        (IpAddr::V6(start), IpAddr::V6(end)) => {
            let start = ipv6_to_u128(start);
            let end = ipv6_to_u128(end);
            if start > end {
                return Err(PortScanError::InvalidArgs(
                    "invalid IPv6 range: start is greater than end".to_string(),
                ));
            }
            let count = end
                .checked_sub(start)
                .and_then(|delta| delta.checked_add(1))
                .ok_or_else(|| {
                    PortScanError::InvalidArgs("IPv6 range size overflowed u128".to_string())
                })?;
            if count > remaining_budget as u128 {
                return Err(PortScanError::InvalidArgs(format!(
                    "IPv6 range expands to {} hosts, exceeding remaining host budget {}",
                    count, remaining_budget
                )));
            }
            Ok((0..count as usize)
                .map(|offset| Ipv6Addr::from(start + offset as u128).to_string())
                .collect())
        }
        _ => Err(PortScanError::InvalidArgs(
            "mixed IP versions in range are not supported".to_string(),
        )),
    }
}

fn checked_host_count(
    host_bits: u32,
    remaining_budget: usize,
    label: &str,
) -> Result<usize, PortScanError> {
    if host_bits >= usize::BITS {
        return Err(PortScanError::InvalidArgs(format!(
            "target `{}` expands beyond supported host count; reduce the network scope",
            label
        )));
    }

    let count = 1usize.checked_shl(host_bits).ok_or_else(|| {
        PortScanError::InvalidArgs(format!(
            "target `{}` expands beyond supported host count",
            label
        ))
    })?;
    if count > remaining_budget {
        return Err(PortScanError::InvalidArgs(format!(
            "target `{}` expands to {} hosts, exceeding remaining host budget {}",
            label, count, remaining_budget
        )));
    }

    Ok(count)
}

fn ipv6_to_u128(value: Ipv6Addr) -> u128 {
    u128::from_be_bytes(value.octets())
}

pub(crate) async fn resolve_hosts(hosts: &[String]) -> Vec<ResolvedHost> {
    let mut results = Vec::with_capacity(hosts.len());
    for host in hosts {
        results.push(ResolvedHost {
            host: host.clone(),
            resolved_ips: resolve_host_ips(host).await,
        });
    }
    results
}

async fn resolve_host_ips(host: &str) -> Result<Vec<IpAddr>, String> {
    let normalized = normalize_host(host);
    if normalized.is_empty() {
        return Err("empty host".to_string());
    }

    if let Ok(ip) = normalized.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }

    let resolved = tokio::net::lookup_host((normalized.as_str(), 0))
        .await
        .map_err(|error| format!("DNS resolution failed for {}: {}", normalized, error))?;

    let mut ips = BTreeSet::new();
    for addr in resolved {
        ips.insert(addr.ip());
    }

    if ips.is_empty() {
        return Err(format!("no IPs resolved for {}", normalized));
    }

    Ok(ips.into_iter().collect())
}
