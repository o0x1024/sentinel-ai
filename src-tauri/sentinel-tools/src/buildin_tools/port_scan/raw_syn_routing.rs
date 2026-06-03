use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr};

use futures::stream::{self, StreamExt};
use rsubdomain::device::{auto_get_devices_for_dns, get_device_by_name_for_dns};
use rsubdomain::{EthTable, PacketTransport};

use super::protocol::{RequestedScanPort, ScanProtocol};
use super::raw_syn_engine::{RoutedSocketSpace, RoutedTarget};
use super::targeting::ResolvedSocketTarget;
use super::PortScanError;

const RAW_ROUTE_RESOLUTION_CONCURRENCY: usize = 16;

#[derive(Debug, Clone)]
pub(super) struct RoutedTaskCollection {
    pub(super) groups: Vec<RoutedTaskGroup>,
    pub(super) host_errors: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(super) struct RoutedTaskGroup {
    pub(super) route: EthTable,
    pub(super) space: RoutedSocketSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RouteKey {
    device: String,
    src_ip: Ipv4Addr,
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
}

impl From<&EthTable> for RouteKey {
    fn from(value: &EthTable) -> Self {
        Self {
            device: value.device.clone(),
            src_ip: value.src_ip,
            src_mac: [
                value.src_mac.0,
                value.src_mac.1,
                value.src_mac.2,
                value.src_mac.3,
                value.src_mac.4,
                value.src_mac.5,
            ],
            dst_mac: [
                value.dst_mac.0,
                value.dst_mac.1,
                value.dst_mac.2,
                value.dst_mac.3,
                value.dst_mac.4,
                value.dst_mac.5,
            ],
        }
    }
}

pub(super) async fn route_raw_targets(
    targets: Vec<ResolvedSocketTarget>,
    requested_ports: &[RequestedScanPort],
    device: Option<&str>,
) -> Result<RoutedTaskCollection, PortScanError> {
    let unique_ipv4_targets = collect_unique_ipv4_targets(&targets);
    let route_cache = resolve_routes_for_targets(unique_ipv4_targets, device).await?;
    Ok(build_routed_task_collection(
        targets,
        requested_ports,
        route_cache,
    ))
}

pub(super) fn build_routed_task_collection(
    targets: Vec<ResolvedSocketTarget>,
    requested_ports: &[RequestedScanPort],
    route_cache: BTreeMap<Ipv4Addr, Result<EthTable, String>>,
) -> RoutedTaskCollection {
    let mut grouped = BTreeMap::<(RouteKey, ScanProtocol), RoutedTaskGroup>::new();
    let mut host_errors = BTreeMap::<String, String>::new();
    let mut hosts_with_routed_tasks = BTreeSet::<String>::new();
    let protocol_ports = build_protocol_ports(requested_ports);

    for target in targets {
        let IpAddr::V4(target_ip) = target.ip else {
            host_errors.entry(target.host.clone()).or_insert_with(|| {
                "raw_syn engine currently supports IPv4 targets only".to_string()
            });
            continue;
        };

        let Some(route) = route_cache.get(&target_ip) else {
            host_errors.entry(target.host.clone()).or_insert_with(|| {
                format!(
                    "raw_syn routing failed for {}: missing route result",
                    target_ip
                )
            });
            continue;
        };

        match route {
            Ok(route) => {
                hosts_with_routed_tasks.insert(target.host.clone());
                for (protocol, ports) in &protocol_ports {
                    let key = (RouteKey::from(route), *protocol);
                    grouped
                        .entry(key)
                        .or_insert_with(|| RoutedTaskGroup {
                            route: route.clone(),
                            space: RoutedSocketSpace {
                                protocol: *protocol,
                                targets: Vec::new(),
                                ports: ports.clone(),
                            },
                        })
                        .space
                        .targets
                        .push(RoutedTarget {
                            host: target.host.clone(),
                            target_ip,
                        });
                }
            }
            Err(error) => {
                host_errors.entry(target.host.clone()).or_insert_with(|| {
                    format!("raw_syn routing failed for {}: {}", target_ip, error)
                });
            }
        }
    }

    for host in hosts_with_routed_tasks {
        host_errors.remove(&host);
    }

    RoutedTaskCollection {
        groups: grouped.into_values().collect(),
        host_errors,
    }
}

fn collect_unique_ipv4_targets(targets: &[ResolvedSocketTarget]) -> BTreeSet<Ipv4Addr> {
    targets
        .iter()
        .filter_map(|target| match target.ip {
            IpAddr::V4(ip) => Some(ip),
            IpAddr::V6(_) => None,
        })
        .collect()
}

fn build_protocol_ports(requested_ports: &[RequestedScanPort]) -> BTreeMap<ScanProtocol, Vec<u16>> {
    let mut protocol_ports = BTreeMap::<ScanProtocol, Vec<u16>>::new();
    for requested_port in requested_ports {
        protocol_ports
            .entry(requested_port.protocol)
            .or_default()
            .push(requested_port.port);
    }
    protocol_ports
}

async fn resolve_routes_for_targets(
    unique_targets: BTreeSet<Ipv4Addr>,
    device: Option<&str>,
) -> Result<BTreeMap<Ipv4Addr, Result<EthTable, String>>, PortScanError> {
    let concurrency = unique_targets
        .len()
        .clamp(1, RAW_ROUTE_RESOLUTION_CONCURRENCY);
    let requested_device = device.map(str::to_string);

    let resolved = stream::iter(unique_targets.into_iter().map(|target_ip| {
        let requested_device = requested_device.clone();
        async move {
            let route = resolve_route_for_target(target_ip, requested_device.as_deref()).await;
            (target_ip, route)
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;

    Ok(resolved.into_iter().collect())
}

async fn resolve_route_for_target(
    target_ip: Ipv4Addr,
    device: Option<&str>,
) -> Result<EthTable, String> {
    let probe_targets = vec![target_ip.to_string()];
    match device {
        Some(device_name) => {
            let device_name = device_name.to_string();
            tokio::task::spawn_blocking(move || {
                get_device_by_name_for_dns(&device_name, &probe_targets, PacketTransport::Ethernet)
            })
            .await
            .map_err(|error| format!("route worker join failed for {}: {}", target_ip, error))?
        }
        None => tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                auto_get_devices_for_dns(&probe_targets, PacketTransport::Ethernet).await
            })
        })
        .await
        .map_err(|error| format!("route worker join failed for {}: {}", target_ip, error))?,
    }
}
