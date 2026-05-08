use std::collections::{BTreeMap, VecDeque};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pnet::datalink::{
    self, Channel::Ethernet, Config as DatalinkConfig, DataLinkReceiver, DataLinkSender,
};
use pnet::packet::tcp::TcpFlags;
use rsubdomain::EthTable;
use sentinel_plugins::MonitorProgressContext;

use super::{
    emit_port_scan_progress, progress_emit_every, scan_progress_step, PortScanError,
    ScanEngineResult,
};
use crate::buildin_tools::port_scan::protocol::{RequestedScanPort, ScanProtocol};
use crate::buildin_tools::port_scan::raw_syn_routing::{
    route_raw_targets, RoutedTaskCollection, RoutedTaskGroup,
};
use crate::buildin_tools::port_scan::sctp::{
    build_sctp_init_frame, parse_icmp_sctp_unreachable, parse_sctp_response, SctpIcmpUnreachable,
    SctpResponse, SCTP_CHUNK_TYPE_ABORT, SCTP_CHUNK_TYPE_INIT_ACK,
};
use crate::buildin_tools::port_scan::targeting::{
    ResolvedSocketTarget, SocketOutcome, SocketScanTask, SocketStatus,
};
use crate::buildin_tools::port_scan::tcp::{build_tcp_frame, parse_tcp_response, TcpResponse};
use crate::buildin_tools::port_scan::udp::{
    build_udp_frame, parse_icmp_udp_unreachable, parse_udp_response, udp_probe_payload,
    UdpIcmpUnreachable, UdpResponse,
};

#[cfg(test)]
use crate::buildin_tools::port_scan::raw_syn_routing::build_routed_task_collection;

const DEFAULT_RAW_SOURCE_PORT_FIRST: u16 = 40_000;
const RAW_MAX_IN_FLIGHT: usize = 20_000;
const RAW_READ_TIMEOUT_MS: u64 = 50;
const RAW_SCHEDULER_POLL_MS: u64 = 10;

#[derive(Debug, Clone)]
pub(crate) struct RawSynScanConfig {
    pub timeout_ms: u64,
    pub concurrency: usize,
    pub tries: u8,
    pub ttl: u8,
    pub device: Option<String>,
    pub source_port_first: Option<u16>,
    pub source_port_last: Option<u16>,
    pub max_rate: Option<f64>,
    pub seed: Option<u64>,
    pub shard_index: usize,
    pub shard_count: usize,
    pub resume_index: usize,
    pub resume_count: Option<usize>,
    pub monitor_progress: Option<MonitorProgressContext>,
}

pub(crate) async fn scan_raw_syn_sockets(
    resolved_targets: Vec<ResolvedSocketTarget>,
    requested_ports: Vec<RequestedScanPort>,
    config: RawSynScanConfig,
) -> Result<ScanEngineResult, PortScanError> {
    if resolved_targets.is_empty() || requested_ports.is_empty() {
        return Ok(ScanEngineResult::default());
    }

    let routed =
        route_raw_targets(resolved_targets, &requested_ports, config.device.as_deref()).await?;
    if routed.groups.is_empty() {
        return Ok(ScanEngineResult {
            socket_outcomes: Vec::new(),
            host_errors: routed.host_errors,
        });
    }

    tokio::task::spawn_blocking(move || scan_raw_syn_groups_blocking(routed, config))
        .await
        .map_err(|error| {
            PortScanError::ExecutionFailed(format!("raw_syn worker join failed: {}", error))
        })?
}

fn scan_raw_syn_groups_blocking(
    routed: RoutedTaskCollection,
    config: RawSynScanConfig,
) -> Result<ScanEngineResult, PortScanError> {
    validate_raw_syn_config(&config)?;
    let RawSynScanConfig {
        timeout_ms,
        concurrency,
        tries,
        ttl,
        device: _,
        source_port_first,
        source_port_last,
        max_rate,
        seed,
        shard_index,
        shard_count,
        resume_index,
        resume_count,
        monitor_progress,
    } = config;
    let cursor_plan = CursorPlan {
        seed,
        shard_index,
        shard_count,
        resume_index,
        resume_count,
    };
    let (source_port_first, source_port_last) =
        raw_source_port_bounds(source_port_first, source_port_last)?;
    let total_tasks = routed
        .groups
        .iter()
        .map(|group| group.space.selected_task_count(cursor_plan))
        .sum::<usize>();
    if total_tasks == 0 {
        return Ok(ScanEngineResult {
            socket_outcomes: Vec::new(),
            host_errors: routed.host_errors,
        });
    }

    let max_in_flight = concurrency.clamp(
        1,
        RAW_MAX_IN_FLIGHT.min(max_source_port_capacity(
            source_port_first,
            source_port_last,
        )),
    );
    let progress = Arc::new(Mutex::new(RawSynProgress::new(
        total_tasks,
        monitor_progress,
    )));
    let rate_limiter = max_rate
        .map(RawRateLimiter::new)
        .transpose()?
        .map(|limiter| Arc::new(Mutex::new(limiter)));
    let mut socket_outcomes = Vec::new();
    let mut handles = Vec::new();

    for group in routed.groups {
        let progress = Arc::clone(&progress);
        let rate_limiter = rate_limiter.clone();
        let scheduler = SchedulerConfig {
            timeout_ms,
            tries,
            ttl,
            max_in_flight,
            source_port_first: Some(source_port_first),
            source_port_last: Some(source_port_last),
            cursor_plan,
        };
        handles.push(std::thread::spawn(move || {
            scan_raw_syn_group_blocking(group, scheduler, progress, rate_limiter)
        }));
    }

    for handle in handles {
        let group_outcomes = handle.join().map_err(|_| {
            PortScanError::ExecutionFailed("raw_syn group thread panicked".to_string())
        })??;
        socket_outcomes.extend(group_outcomes);
    }

    Ok(ScanEngineResult {
        socket_outcomes,
        host_errors: routed.host_errors,
    })
}

fn validate_raw_syn_config(config: &RawSynScanConfig) -> Result<(), PortScanError> {
    if config.shard_count == 0 {
        return Err(PortScanError::InvalidArgs(
            "raw_syn shard_count must be greater than 0".to_string(),
        ));
    }
    if config.shard_index == 0 || config.shard_index > config.shard_count {
        return Err(PortScanError::InvalidArgs(format!(
            "raw_syn shard_index must be within 1..={} but got {}",
            config.shard_count, config.shard_index
        )));
    }
    if let Some(max_rate) = config.max_rate {
        if !max_rate.is_finite() || max_rate <= 0.0 {
            return Err(PortScanError::InvalidArgs(
                "raw_syn max_rate must be a finite number greater than 0".to_string(),
            ));
        }
    }
    if let Some(first) = config.source_port_first {
        if first == 0 {
            return Err(PortScanError::InvalidArgs(
                "raw_syn source_port_first must be greater than 0".to_string(),
            ));
        }
    }
    if let Some(last) = config.source_port_last {
        if last == 0 {
            return Err(PortScanError::InvalidArgs(
                "raw_syn source_port_last must be greater than 0".to_string(),
            ));
        }
    }
    if let (Some(first), Some(last)) = (config.source_port_first, config.source_port_last) {
        if first > last {
            return Err(PortScanError::InvalidArgs(format!(
                "raw_syn source port range is invalid: {} is greater than {}",
                first, last
            )));
        }
    }
    Ok(())
}

fn scan_raw_syn_group_blocking(
    group: RoutedTaskGroup,
    scheduler: SchedulerConfig,
    progress: Arc<Mutex<RawSynProgress>>,
    rate_limiter: Option<Arc<Mutex<RawRateLimiter>>>,
) -> Result<Vec<SocketOutcome>, PortScanError> {
    let selected_task_count = group.space.selected_task_count(scheduler.cursor_plan);
    if selected_task_count == 0 {
        return Ok(Vec::new());
    }

    let interface = datalink::interfaces()
        .into_iter()
        .find(|interface| interface.name == group.route.device)
        .ok_or_else(|| {
            PortScanError::ExecutionFailed(format!(
                "raw_syn interface {} is no longer available",
                group.route.device
            ))
        })?;
    let config = DatalinkConfig {
        read_timeout: Some(Duration::from_millis(RAW_READ_TIMEOUT_MS)),
        write_timeout: Some(Duration::from_millis(scheduler.timeout_ms.max(100))),
        read_buffer_size: 65_536,
        write_buffer_size: 65_536,
        promiscuous: false,
        ..Default::default()
    };
    let (mut tx, rx) = match datalink::channel(&interface, config) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            return Err(PortScanError::ExecutionFailed(format!(
                "raw_syn interface {} returned an unsupported datalink channel type",
                interface.name
            )));
        }
        Err(error) => {
            return Err(PortScanError::ExecutionFailed(format!(
                "raw_syn could not open datalink channel on {}: {}",
                interface.name, error
            )));
        }
    };

    let reserved_source_ports =
        reserved_source_port_count(&group.space, &scheduler, selected_task_count)?;
    let capture = CaptureThread::spawn(
        rx,
        &group.route.device,
        CaptureFilter {
            local_ip: group.route.src_ip,
            source_port_start: scheduler
                .source_port_first
                .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST),
            source_port_end: capture_source_port_end(
                scheduler
                    .source_port_first
                    .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST),
                reserved_source_ports,
            )?,
        },
        group.space.protocol,
    );

    let group_result = match group.space.protocol {
        ScanProtocol::Tcp => drive_tcp_sender_scheduler(
            tx.as_mut(),
            &capture.events,
            &group.route,
            group.space,
            scheduler,
            &progress,
            rate_limiter.as_ref(),
        ),
        ScanProtocol::Udp => drive_udp_sender_scheduler(
            tx.as_mut(),
            &capture.events,
            &group.route,
            group.space,
            scheduler,
            selected_task_count,
            &progress,
            rate_limiter.as_ref(),
        ),
        ScanProtocol::Sctp => drive_sctp_sender_scheduler(
            tx.as_mut(),
            &capture.events,
            &group.route,
            group.space,
            scheduler,
            &progress,
            rate_limiter.as_ref(),
        ),
    };
    let shutdown_result = capture.shutdown();
    let group_outcomes = group_result?;
    shutdown_result?;
    Ok(group_outcomes)
}

#[derive(Debug, Clone, Copy)]
struct CursorPlan {
    seed: Option<u64>,
    shard_index: usize,
    shard_count: usize,
    resume_index: usize,
    resume_count: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct SchedulerConfig {
    timeout_ms: u64,
    tries: u8,
    ttl: u8,
    max_in_flight: usize,
    source_port_first: Option<u16>,
    source_port_last: Option<u16>,
    cursor_plan: CursorPlan,
}

fn drive_tcp_sender_scheduler(
    tx: &mut dyn DataLinkSender,
    capture_rx: &Receiver<CaptureEvent>,
    route: &EthTable,
    space: RoutedSocketSpace,
    scheduler: SchedulerConfig,
    progress: &Arc<Mutex<RawSynProgress>>,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<Vec<SocketOutcome>, PortScanError> {
    if space.is_empty() {
        return Ok(Vec::new());
    }

    let mut task_cursor = RoutedTaskCursor::new(route, space, scheduler.cursor_plan);
    let source_port_first = scheduler
        .source_port_first
        .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST);
    let mut available_ports = available_source_ports(source_port_first, scheduler.max_in_flight);
    let mut pending = BTreeMap::<u16, ScheduledProbe>::new();
    let poll_ceiling = Duration::from_millis(RAW_SCHEDULER_POLL_MS);
    let retry_window = Duration::from_millis(scheduler.timeout_ms.max(100));
    let max_tries = scheduler.tries.max(1);
    let mut socket_outcomes = Vec::new();

    while !task_cursor.is_exhausted() || !pending.is_empty() {
        fill_in_flight_window(
            tx,
            route,
            &mut task_cursor,
            &mut available_ports,
            &mut pending,
            scheduler.max_in_flight,
            retry_window,
            scheduler.ttl,
            rate_limiter,
        )?;

        let wait_timeout = scheduler_wait_timeout(&pending, poll_ceiling);
        match capture_rx.recv_timeout(wait_timeout) {
            Ok(CaptureEvent::TcpResponse(response)) => {
                if let Some((resolved_probe, outcome)) =
                    resolve_capture_response(tx, route, &mut pending, response, scheduler.ttl)?
                {
                    available_ports.push_back(resolved_probe.pending.source_port);
                    let current_target = resolved_probe.pending.task.host.clone();
                    let open = matches!(outcome.status, SocketStatus::Open);
                    record_progress(progress, open, &current_target)?;
                    socket_outcomes.push(outcome);
                }
            }
            Ok(
                CaptureEvent::UdpResponse(_)
                | CaptureEvent::UdpClosed(_)
                | CaptureEvent::SctpResponse(_)
                | CaptureEvent::SctpClosed(_),
            ) => {}
            Ok(CaptureEvent::Error(error)) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture failed on interface {}: {}",
                    route.device, error
                )));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture channel disconnected on interface {}",
                    route.device
                )));
            }
        }

        reap_expired_probes(
            tx,
            route,
            &mut pending,
            &mut available_ports,
            retry_window,
            max_tries,
            scheduler.ttl,
            rate_limiter,
            progress,
            &mut socket_outcomes,
        )?;
    }

    Ok(socket_outcomes)
}

fn fill_in_flight_window(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    task_cursor: &mut RoutedTaskCursor,
    available_ports: &mut VecDeque<u16>,
    pending: &mut BTreeMap<u16, ScheduledProbe>,
    max_in_flight: usize,
    retry_window: Duration,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    while pending.len() < max_in_flight {
        let Some(source_port) = available_ports.pop_front() else {
            break;
        };
        let Some(task) = task_cursor.next_task() else {
            available_ports.push_front(source_port);
            break;
        };

        let pending_probe = PendingSyn::new(task, source_port);
        transmit_syn(tx, route, &pending_probe, ttl, rate_limiter)?;
        pending.insert(
            source_port,
            ScheduledProbe {
                pending: pending_probe,
                attempts_sent: 1,
                deadline: Instant::now() + retry_window,
            },
        );
    }

    Ok(())
}

fn resolve_capture_response(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &mut BTreeMap<u16, ScheduledProbe>,
    response: TcpResponse,
    ttl: u8,
) -> Result<Option<(ScheduledProbe, SocketOutcome)>, PortScanError> {
    let Some(existing) = pending.get(&response.destination_port) else {
        return Ok(None);
    };
    if !response_matches_probe(existing, &response, route.src_ip) {
        return Ok(None);
    }

    let flags = response.flags;
    if flags & (TcpFlags::SYN | TcpFlags::ACK) == (TcpFlags::SYN | TcpFlags::ACK) {
        let resolved = pending
            .remove(&response.destination_port)
            .expect("matched pending probe should still exist");
        let outcome = SocketOutcome::new(
            resolved.pending.task.host.clone(),
            IpAddr::V4(resolved.pending.target_ip),
            resolved.pending.task.port,
            ScanProtocol::Tcp,
            SocketStatus::Open,
            Some("syn_ack".to_string()),
            Some(response.ttl),
        );

        let rst = build_tcp_frame(
            route,
            resolved.pending.target_ip,
            resolved.pending.source_port,
            resolved.pending.task.port,
            response.acknowledgement,
            response.sequence.wrapping_add(1),
            TcpFlags::RST | TcpFlags::ACK,
            ttl,
        );
        let _ = send_frame(tx, &rst);

        return Ok(Some((resolved, outcome)));
    }

    if flags & TcpFlags::RST != 0 {
        let resolved = pending
            .remove(&response.destination_port)
            .expect("matched pending probe should still exist");
        return Ok(Some((
            resolved.clone(),
            SocketOutcome::new(
                resolved.pending.task.host.clone(),
                IpAddr::V4(resolved.pending.target_ip),
                resolved.pending.task.port,
                ScanProtocol::Tcp,
                SocketStatus::Closed,
                Some("rst".to_string()),
                Some(response.ttl),
            ),
        )));
    }

    Ok(None)
}

fn reap_expired_probes(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &mut BTreeMap<u16, ScheduledProbe>,
    available_ports: &mut VecDeque<u16>,
    retry_window: Duration,
    max_tries: u8,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
    progress: &Arc<Mutex<RawSynProgress>>,
    socket_outcomes: &mut Vec<SocketOutcome>,
) -> Result<(), PortScanError> {
    let now = Instant::now();
    let expired_ports = pending
        .iter()
        .filter_map(|(source_port, probe)| (probe.deadline <= now).then_some(*source_port))
        .collect::<Vec<_>>();

    for source_port in expired_ports {
        let Some(mut probe) = pending.remove(&source_port) else {
            continue;
        };
        if probe.attempts_sent >= max_tries {
            available_ports.push_back(probe.pending.source_port);
            let current_target = probe.pending.task.host.clone();
            record_progress(progress, false, &current_target)?;
            socket_outcomes.push(SocketOutcome::new(
                probe.pending.task.host.clone(),
                IpAddr::V4(probe.pending.target_ip),
                probe.pending.task.port,
                ScanProtocol::Tcp,
                SocketStatus::Filtered,
                Some("no_response".to_string()),
                None,
            ));
            continue;
        }

        transmit_syn(tx, route, &probe.pending, ttl, rate_limiter)?;
        probe.attempts_sent += 1;
        probe.deadline = Instant::now() + retry_window;
        pending.insert(source_port, probe);
    }

    Ok(())
}

fn drive_udp_sender_scheduler(
    tx: &mut dyn DataLinkSender,
    capture_rx: &Receiver<CaptureEvent>,
    route: &EthTable,
    space: RoutedSocketSpace,
    scheduler: SchedulerConfig,
    selected_task_count: usize,
    progress: &Arc<Mutex<RawSynProgress>>,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<Vec<SocketOutcome>, PortScanError> {
    if space.is_empty() {
        return Ok(Vec::new());
    }

    let mut task_cursor = RoutedTaskCursor::new(route, space, scheduler.cursor_plan);
    let source_port_first = scheduler
        .source_port_first
        .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST);
    let mut available_ports = available_source_ports(source_port_first, selected_task_count);
    let mut pending = BTreeMap::<u16, ScheduledUdpProbe>::new();
    let poll_ceiling = Duration::from_millis(RAW_SCHEDULER_POLL_MS);
    let retry_window = Duration::from_millis(scheduler.timeout_ms.max(100));
    let max_tries = scheduler.tries.max(1);
    let mut socket_outcomes = Vec::new();

    while !task_cursor.is_exhausted() || !pending.is_empty() {
        fill_udp_in_flight_window(
            tx,
            route,
            &mut task_cursor,
            &mut available_ports,
            &mut pending,
            scheduler.max_in_flight,
            retry_window,
            scheduler.ttl,
            rate_limiter,
        )?;

        let wait_timeout = scheduler_wait_timeout_udp(&pending, poll_ceiling);
        match capture_rx.recv_timeout(wait_timeout) {
            Ok(CaptureEvent::UdpResponse(response)) => {
                if let Some((resolved_probe, open_socket)) =
                    resolve_udp_capture_response(&mut pending, response)
                {
                    let current_target = resolved_probe.pending.task.host.clone();
                    record_progress(progress, true, &current_target)?;
                    socket_outcomes.push(open_socket);
                }
            }
            Ok(CaptureEvent::UdpClosed(response)) => {
                if let Some((resolved_probe, outcome)) =
                    resolve_udp_capture_close(&mut pending, response)
                {
                    let current_target = resolved_probe.pending.task.host.clone();
                    record_progress(progress, false, &current_target)?;
                    socket_outcomes.push(outcome);
                }
            }
            Ok(
                CaptureEvent::TcpResponse(_)
                | CaptureEvent::SctpResponse(_)
                | CaptureEvent::SctpClosed(_),
            ) => {}
            Ok(CaptureEvent::Error(error)) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture failed on interface {}: {}",
                    route.device, error
                )));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture channel disconnected on interface {}",
                    route.device
                )));
            }
        }

        reap_expired_udp_probes(
            tx,
            route,
            &mut pending,
            retry_window,
            max_tries,
            scheduler.ttl,
            rate_limiter,
            progress,
            &mut socket_outcomes,
        )?;
    }

    Ok(socket_outcomes)
}

fn fill_udp_in_flight_window(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    task_cursor: &mut RoutedTaskCursor,
    available_ports: &mut VecDeque<u16>,
    pending: &mut BTreeMap<u16, ScheduledUdpProbe>,
    max_in_flight: usize,
    retry_window: Duration,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    while pending.len() < max_in_flight {
        let Some(source_port) = available_ports.pop_front() else {
            break;
        };
        let Some(task) = task_cursor.next_task() else {
            available_ports.push_front(source_port);
            break;
        };

        let pending_probe = PendingUdpProbe::new(task, source_port);
        transmit_udp_probe(tx, route, &pending_probe, ttl, rate_limiter)?;
        pending.insert(
            source_port,
            ScheduledUdpProbe {
                pending: pending_probe,
                attempts_sent: 1,
                deadline: Instant::now() + retry_window,
            },
        );
    }

    Ok(())
}

fn resolve_udp_capture_response(
    pending: &mut BTreeMap<u16, ScheduledUdpProbe>,
    response: UdpResponse,
) -> Option<(ScheduledUdpProbe, SocketOutcome)> {
    let existing = pending.get(&response.destination_port)?;
    if !udp_response_matches_probe(existing, &response) {
        return None;
    }

    let resolved = pending.remove(&response.destination_port)?;
    let open_socket = SocketOutcome::new(
        resolved.pending.task.host.clone(),
        IpAddr::V4(resolved.pending.target_ip),
        resolved.pending.task.port,
        ScanProtocol::Udp,
        SocketStatus::Open,
        Some("udp_response".to_string()),
        Some(response.ttl),
    );
    Some((resolved, open_socket))
}

fn resolve_udp_capture_close(
    pending: &mut BTreeMap<u16, ScheduledUdpProbe>,
    response: UdpIcmpUnreachable,
) -> Option<(ScheduledUdpProbe, SocketOutcome)> {
    let existing = pending.get(&response.embedded_source_port)?;
    if !udp_close_matches_probe(existing, &response) {
        return None;
    }
    let resolved = pending.remove(&response.embedded_source_port)?;
    let outcome = SocketOutcome::new(
        resolved.pending.task.host.clone(),
        IpAddr::V4(resolved.pending.target_ip),
        resolved.pending.task.port,
        ScanProtocol::Udp,
        SocketStatus::Closed,
        Some("icmp_port_unreachable".to_string()),
        Some(response.ttl),
    );
    Some((resolved, outcome))
}

fn reap_expired_udp_probes(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &mut BTreeMap<u16, ScheduledUdpProbe>,
    retry_window: Duration,
    max_tries: u8,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
    progress: &Arc<Mutex<RawSynProgress>>,
    socket_outcomes: &mut Vec<SocketOutcome>,
) -> Result<(), PortScanError> {
    let now = Instant::now();
    let expired_ports = pending
        .iter()
        .filter_map(|(source_port, probe)| (probe.deadline <= now).then_some(*source_port))
        .collect::<Vec<_>>();

    for source_port in expired_ports {
        let Some(mut probe) = pending.remove(&source_port) else {
            continue;
        };
        if probe.attempts_sent >= max_tries {
            let current_target = probe.pending.task.host.clone();
            record_progress(progress, false, &current_target)?;
            socket_outcomes.push(SocketOutcome::new(
                probe.pending.task.host.clone(),
                IpAddr::V4(probe.pending.target_ip),
                probe.pending.task.port,
                ScanProtocol::Udp,
                SocketStatus::Filtered,
                Some("no_response".to_string()),
                None,
            ));
            continue;
        }

        transmit_udp_probe(tx, route, &probe.pending, ttl, rate_limiter)?;
        probe.attempts_sent += 1;
        probe.deadline = Instant::now() + retry_window;
        pending.insert(source_port, probe);
    }

    Ok(())
}

fn drive_sctp_sender_scheduler(
    tx: &mut dyn DataLinkSender,
    capture_rx: &Receiver<CaptureEvent>,
    route: &EthTable,
    space: RoutedSocketSpace,
    scheduler: SchedulerConfig,
    progress: &Arc<Mutex<RawSynProgress>>,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<Vec<SocketOutcome>, PortScanError> {
    if space.is_empty() {
        return Ok(Vec::new());
    }

    let mut task_cursor = RoutedTaskCursor::new(route, space, scheduler.cursor_plan);
    let source_port_first = scheduler
        .source_port_first
        .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST);
    let mut available_ports = available_source_ports(source_port_first, scheduler.max_in_flight);
    let mut pending = BTreeMap::<u16, ScheduledSctpProbe>::new();
    let poll_ceiling = Duration::from_millis(RAW_SCHEDULER_POLL_MS);
    let retry_window = Duration::from_millis(scheduler.timeout_ms.max(100));
    let max_tries = scheduler.tries.max(1);
    let mut socket_outcomes = Vec::new();

    while !task_cursor.is_exhausted() || !pending.is_empty() {
        fill_sctp_in_flight_window(
            tx,
            route,
            &mut task_cursor,
            &mut available_ports,
            &mut pending,
            scheduler.max_in_flight,
            retry_window,
            scheduler.ttl,
            rate_limiter,
        )?;

        let wait_timeout = scheduler_wait_timeout_sctp(&pending, poll_ceiling);
        match capture_rx.recv_timeout(wait_timeout) {
            Ok(CaptureEvent::SctpResponse(response)) => {
                if let Some((resolved_probe, outcome)) =
                    resolve_sctp_capture_response(&mut pending, response)
                {
                    available_ports.push_back(resolved_probe.pending.source_port);
                    let current_target = resolved_probe.pending.task.host.clone();
                    let open = matches!(outcome.status, SocketStatus::Open);
                    record_progress(progress, open, &current_target)?;
                    socket_outcomes.push(outcome);
                }
            }
            Ok(CaptureEvent::SctpClosed(response)) => {
                if let Some((resolved_probe, outcome)) =
                    resolve_sctp_capture_close(&mut pending, response)
                {
                    available_ports.push_back(resolved_probe.pending.source_port);
                    let current_target = resolved_probe.pending.task.host.clone();
                    record_progress(progress, false, &current_target)?;
                    socket_outcomes.push(outcome);
                }
            }
            Ok(
                CaptureEvent::TcpResponse(_)
                | CaptureEvent::UdpResponse(_)
                | CaptureEvent::UdpClosed(_),
            ) => {}
            Ok(CaptureEvent::Error(error)) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture failed on interface {}: {}",
                    route.device, error
                )));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(PortScanError::ExecutionFailed(format!(
                    "raw_syn capture channel disconnected on interface {}",
                    route.device
                )));
            }
        }

        reap_expired_sctp_probes(
            tx,
            route,
            &mut pending,
            &mut available_ports,
            retry_window,
            max_tries,
            scheduler.ttl,
            rate_limiter,
            progress,
            &mut socket_outcomes,
        )?;
    }

    Ok(socket_outcomes)
}

fn fill_sctp_in_flight_window(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    task_cursor: &mut RoutedTaskCursor,
    available_ports: &mut VecDeque<u16>,
    pending: &mut BTreeMap<u16, ScheduledSctpProbe>,
    max_in_flight: usize,
    retry_window: Duration,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    while pending.len() < max_in_flight {
        let Some(source_port) = available_ports.pop_front() else {
            break;
        };
        let Some(task) = task_cursor.next_task() else {
            available_ports.push_front(source_port);
            break;
        };

        let pending_probe = PendingSctpProbe::new(task, source_port);
        transmit_sctp_probe(tx, route, &pending_probe, ttl, rate_limiter)?;
        pending.insert(
            source_port,
            ScheduledSctpProbe {
                pending: pending_probe,
                attempts_sent: 1,
                deadline: Instant::now() + retry_window,
            },
        );
    }

    Ok(())
}

fn resolve_sctp_capture_response(
    pending: &mut BTreeMap<u16, ScheduledSctpProbe>,
    response: SctpResponse,
) -> Option<(ScheduledSctpProbe, SocketOutcome)> {
    let existing = pending.get(&response.destination_port)?;
    if !sctp_response_matches_probe(existing, &response) {
        return None;
    }

    match response.first_chunk_type {
        SCTP_CHUNK_TYPE_INIT_ACK => {
            let resolved = pending.remove(&response.destination_port)?;
            let open_socket = SocketOutcome::new(
                resolved.pending.task.host.clone(),
                IpAddr::V4(resolved.pending.target_ip),
                resolved.pending.task.port,
                ScanProtocol::Sctp,
                SocketStatus::Open,
                Some("init_ack".to_string()),
                Some(response.ttl),
            );
            Some((resolved, open_socket))
        }
        SCTP_CHUNK_TYPE_ABORT => {
            let resolved = pending.remove(&response.destination_port)?;
            Some((
                resolved.clone(),
                SocketOutcome::new(
                    resolved.pending.task.host.clone(),
                    IpAddr::V4(resolved.pending.target_ip),
                    resolved.pending.task.port,
                    ScanProtocol::Sctp,
                    SocketStatus::Closed,
                    Some("abort".to_string()),
                    Some(response.ttl),
                ),
            ))
        }
        _ => None,
    }
}

fn resolve_sctp_capture_close(
    pending: &mut BTreeMap<u16, ScheduledSctpProbe>,
    response: SctpIcmpUnreachable,
) -> Option<(ScheduledSctpProbe, SocketOutcome)> {
    let existing = pending.get(&response.embedded_source_port)?;
    if !sctp_close_matches_probe(existing, &response) {
        return None;
    }
    let resolved = pending.remove(&response.embedded_source_port)?;
    let reason = match response.code {
        2 => "icmp_protocol_unreachable",
        3 => "icmp_port_unreachable",
        _ => "icmp_unreachable",
    };
    let outcome = SocketOutcome::new(
        resolved.pending.task.host.clone(),
        IpAddr::V4(resolved.pending.target_ip),
        resolved.pending.task.port,
        ScanProtocol::Sctp,
        SocketStatus::Closed,
        Some(reason.to_string()),
        Some(response.ttl),
    );
    Some((resolved, outcome))
}

fn reap_expired_sctp_probes(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &mut BTreeMap<u16, ScheduledSctpProbe>,
    available_ports: &mut VecDeque<u16>,
    retry_window: Duration,
    max_tries: u8,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
    progress: &Arc<Mutex<RawSynProgress>>,
    socket_outcomes: &mut Vec<SocketOutcome>,
) -> Result<(), PortScanError> {
    let now = Instant::now();
    let expired_ports = pending
        .iter()
        .filter_map(|(source_port, probe)| (probe.deadline <= now).then_some(*source_port))
        .collect::<Vec<_>>();

    for source_port in expired_ports {
        let Some(mut probe) = pending.remove(&source_port) else {
            continue;
        };
        if probe.attempts_sent >= max_tries {
            available_ports.push_back(probe.pending.source_port);
            let current_target = probe.pending.task.host.clone();
            record_progress(progress, false, &current_target)?;
            socket_outcomes.push(SocketOutcome::new(
                probe.pending.task.host.clone(),
                IpAddr::V4(probe.pending.target_ip),
                probe.pending.task.port,
                ScanProtocol::Sctp,
                SocketStatus::Filtered,
                Some("no_response".to_string()),
                None,
            ));
            continue;
        }

        transmit_sctp_probe(tx, route, &probe.pending, ttl, rate_limiter)?;
        probe.attempts_sent += 1;
        probe.deadline = Instant::now() + retry_window;
        pending.insert(source_port, probe);
    }

    Ok(())
}

fn transmit_syn(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &PendingSyn,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    wait_for_send_slot(rate_limiter)?;
    let packet = build_tcp_frame(
        route,
        pending.target_ip,
        pending.source_port,
        pending.task.port,
        pending.sequence,
        0,
        TcpFlags::SYN,
        ttl,
    );
    send_frame(tx, &packet)
}

fn transmit_udp_probe(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &PendingUdpProbe,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    wait_for_send_slot(rate_limiter)?;
    let packet = build_udp_frame(
        route,
        pending.target_ip,
        pending.source_port,
        pending.task.port,
        pending.payload,
        ttl,
    );
    send_frame(tx, &packet)
}

fn transmit_sctp_probe(
    tx: &mut dyn DataLinkSender,
    route: &EthTable,
    pending: &PendingSctpProbe,
    ttl: u8,
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    wait_for_send_slot(rate_limiter)?;
    let packet = build_sctp_init_frame(
        route,
        pending.target_ip,
        pending.source_port,
        pending.task.port,
        pending.verification_tag,
        ttl,
    );
    send_frame(tx, &packet)
}

fn scheduler_wait_timeout(
    pending: &BTreeMap<u16, ScheduledProbe>,
    poll_ceiling: Duration,
) -> Duration {
    let Some(next_deadline) = pending.values().map(|probe| probe.deadline).min() else {
        return poll_ceiling;
    };

    let now = Instant::now();
    if next_deadline <= now {
        return Duration::from_millis(0);
    }

    next_deadline
        .saturating_duration_since(now)
        .min(poll_ceiling)
}

fn scheduler_wait_timeout_sctp(
    pending: &BTreeMap<u16, ScheduledSctpProbe>,
    poll_ceiling: Duration,
) -> Duration {
    let Some(next_deadline) = pending.values().map(|probe| probe.deadline).min() else {
        return poll_ceiling;
    };

    let now = Instant::now();
    if next_deadline <= now {
        return Duration::from_millis(0);
    }

    next_deadline
        .saturating_duration_since(now)
        .min(poll_ceiling)
}

fn scheduler_wait_timeout_udp(
    pending: &BTreeMap<u16, ScheduledUdpProbe>,
    poll_ceiling: Duration,
) -> Duration {
    let Some(next_deadline) = pending.values().map(|probe| probe.deadline).min() else {
        return poll_ceiling;
    };

    let now = Instant::now();
    if next_deadline <= now {
        return Duration::from_millis(0);
    }

    next_deadline
        .saturating_duration_since(now)
        .min(poll_ceiling)
}

fn available_source_ports(source_port_first: u16, max_in_flight: usize) -> VecDeque<u16> {
    (0..max_in_flight)
        .map(|offset| source_port_first + offset as u16)
        .collect()
}

fn max_source_port_capacity(source_port_first: u16, source_port_last: u16) -> usize {
    (source_port_last - source_port_first) as usize + 1
}

fn raw_source_port_bounds(
    source_port_first: Option<u16>,
    source_port_last: Option<u16>,
) -> Result<(u16, u16), PortScanError> {
    let first = source_port_first.unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST);
    let last = source_port_last.unwrap_or(u16::MAX);
    if first == 0 || last == 0 || first > last {
        return Err(PortScanError::InvalidArgs(format!(
            "raw_syn source port range is invalid: {}-{}",
            first, last
        )));
    }
    Ok((first, last))
}

fn reserved_source_port_count(
    space: &RoutedSocketSpace,
    scheduler: &SchedulerConfig,
    selected_task_count: usize,
) -> Result<usize, PortScanError> {
    match space.protocol {
        ScanProtocol::Tcp | ScanProtocol::Sctp => {
            Ok(scheduler.max_in_flight.min(selected_task_count.max(1)))
        }
        ScanProtocol::Udp => {
            let first = scheduler
                .source_port_first
                .unwrap_or(DEFAULT_RAW_SOURCE_PORT_FIRST);
            let last = scheduler.source_port_last.unwrap_or(u16::MAX);
            let capacity = max_source_port_capacity(first, last);
            if selected_task_count > capacity {
                return Err(PortScanError::InvalidArgs(format!(
                    "raw_syn UDP scan space on {} requires {} source ports but configured range {}-{} only provides {}",
                    space.protocol.label(),
                    selected_task_count,
                    first,
                    last,
                    capacity
                )));
            }
            Ok(selected_task_count.max(1))
        }
    }
}

fn capture_source_port_end(
    source_port_first: u16,
    reserved_source_ports: usize,
) -> Result<u16, PortScanError> {
    source_port_first
        .checked_add(reserved_source_ports.checked_sub(1).ok_or_else(|| {
            PortScanError::ExecutionFailed(
                "raw_syn reserved source port window underflowed".to_string(),
            )
        })? as u16)
        .ok_or_else(|| {
            PortScanError::InvalidArgs("raw_syn reserved source port window overflowed".to_string())
        })
}

#[derive(Debug, Clone)]
pub(super) struct RoutedSocketSpace {
    pub(super) protocol: ScanProtocol,
    pub(super) targets: Vec<RoutedTarget>,
    pub(super) ports: Vec<u16>,
}

impl RoutedSocketSpace {
    fn is_empty(&self) -> bool {
        self.targets.is_empty() || self.ports.is_empty()
    }

    fn total_tasks(&self) -> usize {
        self.targets.len().saturating_mul(self.ports.len())
    }

    fn selected_task_count(&self, plan: CursorPlan) -> usize {
        selected_task_count(
            self.total_tasks(),
            plan.shard_index,
            plan.shard_count,
            plan.resume_index,
            plan.resume_count,
        )
    }

    fn task_at(&self, index: usize) -> Option<RoutedSynTask> {
        if self.is_empty() || index >= self.total_tasks() {
            return None;
        }

        let port_count = self.ports.len();
        let target_index = index / port_count;
        let port_index = index % port_count;
        let target = self.targets.get(target_index)?;
        let port = *self.ports.get(port_index)?;
        Some(RoutedSynTask {
            task: SocketScanTask {
                host: target.host.clone(),
                ip: IpAddr::V4(target.target_ip),
                port,
                protocol: self.protocol,
            },
            target_ip: target.target_ip,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct RoutedTarget {
    pub(super) host: String,
    pub(super) target_ip: Ipv4Addr,
}

#[derive(Debug, Clone)]
struct RoutedTaskCursor {
    space: RoutedSocketSpace,
    order: PermutedIndexCursor,
}

impl RoutedTaskCursor {
    fn new(route: &EthTable, space: RoutedSocketSpace, plan: CursorPlan) -> Self {
        let route_seed = task_space_seed(route, &space);
        let seed = match plan.seed {
            Some(user_seed) => mix64(user_seed ^ route_seed),
            None => route_seed,
        };
        let order = PermutedIndexCursor::new(
            space.total_tasks(),
            seed,
            plan.shard_index,
            plan.shard_count,
            plan.resume_index,
            plan.resume_count,
        );
        Self { space, order }
    }

    fn is_exhausted(&self) -> bool {
        self.order.is_exhausted()
    }

    fn next_task(&mut self) -> Option<RoutedSynTask> {
        let index = self.order.next_index()?;
        self.space.task_at(index)
    }
}

#[derive(Debug, Clone)]
struct PermutedIndexCursor {
    range: usize,
    start: usize,
    step: usize,
    next_position: usize,
    position_step: usize,
    end_position: usize,
}

impl PermutedIndexCursor {
    fn new(
        range: usize,
        seed: u64,
        shard_index: usize,
        shard_count: usize,
        resume_index: usize,
        resume_count: Option<usize>,
    ) -> Self {
        let end_position = scan_end_position(range, resume_index, resume_count);
        if range <= 1 {
            return Self {
                range,
                start: 0,
                step: 1,
                next_position: resume_index.min(end_position),
                position_step: shard_count.max(1),
                end_position,
            };
        }

        let start = (seed % range as u64) as usize;
        let mut step = ((mix64(seed) % range as u64) as usize).max(1);
        while gcd(step, range) != 1 {
            step += 1;
            if step == range {
                step = 1;
            }
        }

        Self {
            range,
            start,
            step,
            next_position: resume_index.saturating_add(shard_index.saturating_sub(1)),
            position_step: shard_count.max(1),
            end_position,
        }
    }

    fn is_exhausted(&self) -> bool {
        self.next_position >= self.end_position
    }

    fn next_index(&mut self) -> Option<usize> {
        if self.is_exhausted() {
            return None;
        }

        let position = self.next_position;
        self.next_position = self.next_position.saturating_add(self.position_step);
        let index = if self.range <= 1 {
            0
        } else {
            ((self.start as u128 + position as u128 * self.step as u128) % self.range as u128)
                as usize
        };
        Some(index)
    }
}

fn scan_end_position(range: usize, resume_index: usize, resume_count: Option<usize>) -> usize {
    let resume_end = resume_count
        .map(|count| resume_index.saturating_add(count))
        .unwrap_or(range);
    range.min(resume_end)
}

fn selected_task_count(
    range: usize,
    shard_index: usize,
    shard_count: usize,
    resume_index: usize,
    resume_count: Option<usize>,
) -> usize {
    let end_position = scan_end_position(range, resume_index, resume_count);
    let first_position = resume_index.saturating_add(shard_index.saturating_sub(1));
    if first_position >= end_position || shard_count == 0 {
        return 0;
    }

    1 + (end_position - 1 - first_position) / shard_count
}

#[derive(Debug, Clone)]
struct RoutedSynTask {
    task: SocketScanTask,
    target_ip: Ipv4Addr,
}

#[derive(Debug, Clone)]
struct PendingSyn {
    task: SocketScanTask,
    target_ip: Ipv4Addr,
    source_port: u16,
    sequence: u32,
}

impl PendingSyn {
    fn new(task: RoutedSynTask, source_port: u16) -> Self {
        let sequence = initial_probe_token(&task, source_port);
        Self {
            task: task.task,
            target_ip: task.target_ip,
            source_port,
            sequence,
        }
    }

    fn expected_ack(&self) -> u32 {
        self.sequence.wrapping_add(1)
    }
}

#[derive(Debug, Clone)]
struct ScheduledProbe {
    pending: PendingSyn,
    attempts_sent: u8,
    deadline: Instant,
}

#[derive(Debug, Clone)]
struct PendingUdpProbe {
    task: SocketScanTask,
    target_ip: Ipv4Addr,
    source_port: u16,
    payload: &'static [u8],
}

impl PendingUdpProbe {
    fn new(task: RoutedSynTask, source_port: u16) -> Self {
        let payload = udp_probe_payload(task.task.port);
        Self {
            payload,
            task: task.task,
            target_ip: task.target_ip,
            source_port,
        }
    }
}

#[derive(Debug, Clone)]
struct ScheduledUdpProbe {
    pending: PendingUdpProbe,
    attempts_sent: u8,
    deadline: Instant,
}

#[derive(Debug, Clone)]
struct PendingSctpProbe {
    task: SocketScanTask,
    target_ip: Ipv4Addr,
    source_port: u16,
    verification_tag: u32,
}

impl PendingSctpProbe {
    fn new(task: RoutedSynTask, source_port: u16) -> Self {
        let verification_tag = initial_probe_token(&task, source_port);
        Self {
            task: task.task,
            target_ip: task.target_ip,
            source_port,
            verification_tag,
        }
    }
}

#[derive(Debug, Clone)]
struct ScheduledSctpProbe {
    pending: PendingSctpProbe,
    attempts_sent: u8,
    deadline: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CaptureFilter {
    local_ip: Ipv4Addr,
    source_port_start: u16,
    source_port_end: u16,
}

impl CaptureFilter {
    fn matches_tcp(&self, response: &TcpResponse) -> bool {
        response.destination_ip == self.local_ip
            && response.destination_port >= self.source_port_start
            && response.destination_port <= self.source_port_end
    }

    fn matches_udp(&self, response: &UdpResponse) -> bool {
        response.destination_ip == self.local_ip
            && response.destination_port >= self.source_port_start
            && response.destination_port <= self.source_port_end
    }

    fn matches_udp_unreachable(&self, response: &UdpIcmpUnreachable) -> bool {
        response.embedded_source_ip == self.local_ip
            && response.embedded_source_port >= self.source_port_start
            && response.embedded_source_port <= self.source_port_end
    }

    fn matches_sctp(&self, response: &SctpResponse) -> bool {
        response.destination_ip == self.local_ip
            && response.destination_port >= self.source_port_start
            && response.destination_port <= self.source_port_end
    }

    fn matches_sctp_unreachable(&self, response: &SctpIcmpUnreachable) -> bool {
        response.embedded_source_ip == self.local_ip
            && response.embedded_source_port >= self.source_port_start
            && response.embedded_source_port <= self.source_port_end
    }
}

enum CaptureEvent {
    TcpResponse(TcpResponse),
    UdpResponse(UdpResponse),
    UdpClosed(UdpIcmpUnreachable),
    SctpResponse(SctpResponse),
    SctpClosed(SctpIcmpUnreachable),
    Error(String),
}

struct CaptureThread {
    events: Receiver<CaptureEvent>,
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CaptureThread {
    fn spawn(
        mut rx: Box<dyn DataLinkReceiver>,
        device_name: &str,
        filter: CaptureFilter,
        protocol: ScanProtocol,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<CaptureEvent>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_signal = Arc::clone(&stop_flag);
        let device_name = device_name.to_string();

        let handle = std::thread::spawn(move || {
            while !stop_signal.load(Ordering::SeqCst) {
                match rx.next() {
                    Ok(frame) => match protocol {
                        ScanProtocol::Tcp => {
                            let Some(response) = parse_tcp_response(frame) else {
                                continue;
                            };
                            if filter.matches_tcp(&response) {
                                let _ = event_tx.send(CaptureEvent::TcpResponse(response));
                            }
                        }
                        ScanProtocol::Udp => {
                            if let Some(response) = parse_udp_response(frame) {
                                if filter.matches_udp(&response) {
                                    let _ = event_tx.send(CaptureEvent::UdpResponse(response));
                                }
                                continue;
                            }
                            let Some(response) = parse_icmp_udp_unreachable(frame) else {
                                continue;
                            };
                            if filter.matches_udp_unreachable(&response) {
                                let _ = event_tx.send(CaptureEvent::UdpClosed(response));
                            }
                        }
                        ScanProtocol::Sctp => {
                            if let Some(response) = parse_sctp_response(frame) {
                                if filter.matches_sctp(&response) {
                                    let _ = event_tx.send(CaptureEvent::SctpResponse(response));
                                }
                                continue;
                            }
                            let Some(response) = parse_icmp_sctp_unreachable(frame) else {
                                continue;
                            };
                            if filter.matches_sctp_unreachable(&response) {
                                let _ = event_tx.send(CaptureEvent::SctpClosed(response));
                            }
                        }
                    },
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                        ) =>
                    {
                        continue;
                    }
                    Err(error) => {
                        let _ = event_tx.send(CaptureEvent::Error(format!(
                            "{} receive error: {}",
                            device_name, error
                        )));
                        break;
                    }
                }
            }
        });

        Self {
            events: event_rx,
            stop_flag,
            handle: Some(handle),
        }
    }

    fn shutdown(mut self) -> Result<(), PortScanError> {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| {
                PortScanError::ExecutionFailed("raw_syn capture thread panicked".to_string())
            })?;
        }
        Ok(())
    }
}

struct RawSynProgress {
    total_tasks: usize,
    emit_every: usize,
    completed: usize,
    found_open: usize,
    monitor_progress: Option<MonitorProgressContext>,
}

impl RawSynProgress {
    fn new(total_tasks: usize, monitor_progress: Option<MonitorProgressContext>) -> Self {
        Self {
            total_tasks,
            emit_every: progress_emit_every(total_tasks.max(1)),
            completed: 0,
            found_open: 0,
            monitor_progress,
        }
    }

    fn record_completion(&mut self, open: bool, current_target: &str) {
        self.completed += 1;
        if open {
            self.found_open += 1;
        }

        if self.completed % self.emit_every != 0 && self.completed != self.total_tasks {
            return;
        }

        emit_port_scan_progress(
            self.monitor_progress.as_ref(),
            2 + scan_progress_step(self.completed, self.total_tasks),
            5,
            Some(format!(
                "Raw SYN scan progress: {}/{} sockets settled, {} open",
                self.completed, self.total_tasks, self.found_open
            )),
            Some(current_target.to_string()),
            Some("scanning_ports".to_string()),
            Some("Scanning ports".to_string()),
            Some(false),
        );
    }
}

fn response_matches_probe(
    probe: &ScheduledProbe,
    response: &TcpResponse,
    local_ip: Ipv4Addr,
) -> bool {
    response.destination_ip == local_ip
        && response.source_ip == probe.pending.target_ip
        && response.source_port == probe.pending.task.port
        && response.destination_port == probe.pending.source_port
        && response.acknowledgement == probe.pending.expected_ack()
}

fn udp_response_matches_probe(probe: &ScheduledUdpProbe, response: &UdpResponse) -> bool {
    response.source_ip == probe.pending.target_ip
        && response.source_port == probe.pending.task.port
        && response.destination_port == probe.pending.source_port
}

fn udp_close_matches_probe(probe: &ScheduledUdpProbe, response: &UdpIcmpUnreachable) -> bool {
    response.code == 3
        && response.embedded_destination_ip == probe.pending.target_ip
        && response.embedded_source_port == probe.pending.source_port
        && response.embedded_destination_port == probe.pending.task.port
}

fn sctp_response_matches_probe(probe: &ScheduledSctpProbe, response: &SctpResponse) -> bool {
    response.source_ip == probe.pending.target_ip
        && response.source_port == probe.pending.task.port
        && response.destination_port == probe.pending.source_port
        && response.verification_tag == probe.pending.verification_tag
}

fn sctp_close_matches_probe(probe: &ScheduledSctpProbe, response: &SctpIcmpUnreachable) -> bool {
    (response.code == 2 || response.code == 3)
        && response.source_ip == probe.pending.target_ip
        && response.embedded_destination_ip == probe.pending.target_ip
        && response.embedded_source_port == probe.pending.source_port
        && response.embedded_destination_port == probe.pending.task.port
}

fn send_frame(tx: &mut dyn DataLinkSender, packet: &[u8]) -> Result<(), PortScanError> {
    match tx.send_to(packet, None) {
        Some(Ok(())) => Ok(()),
        Some(Err(error)) => Err(PortScanError::ExecutionFailed(format!(
            "raw_syn send failed: {}",
            error
        ))),
        None => Err(PortScanError::ExecutionFailed(
            "raw_syn sender returned no transmit handle".to_string(),
        )),
    }
}

#[derive(Debug)]
struct RawRateLimiter {
    max_rate: f64,
    started_at: Instant,
    reserved_packets: u64,
}

impl RawRateLimiter {
    fn new(max_rate: f64) -> Result<Self, PortScanError> {
        if !max_rate.is_finite() || max_rate <= 0.0 {
            return Err(PortScanError::InvalidArgs(
                "raw_syn max_rate must be a finite number greater than 0".to_string(),
            ));
        }

        Ok(Self {
            max_rate,
            started_at: Instant::now(),
            reserved_packets: 0,
        })
    }

    fn reserve_delay(&mut self) -> Duration {
        let target_packet = self.reserved_packets.saturating_add(1);
        self.reserved_packets = target_packet;
        let scheduled = Duration::from_secs_f64(target_packet as f64 / self.max_rate);
        scheduled.saturating_sub(self.started_at.elapsed())
    }
}

fn wait_for_send_slot(
    rate_limiter: Option<&Arc<Mutex<RawRateLimiter>>>,
) -> Result<(), PortScanError> {
    let Some(rate_limiter) = rate_limiter else {
        return Ok(());
    };

    let delay = {
        let mut limiter = rate_limiter.lock().map_err(|_| {
            PortScanError::ExecutionFailed("raw_syn rate limiter mutex was poisoned".to_string())
        })?;
        limiter.reserve_delay()
    };
    if !delay.is_zero() {
        std::thread::sleep(delay);
    }
    Ok(())
}

fn record_progress(
    progress: &Arc<Mutex<RawSynProgress>>,
    open: bool,
    current_target: &str,
) -> Result<(), PortScanError> {
    let mut progress = progress.lock().map_err(|_| {
        PortScanError::ExecutionFailed("raw_syn progress mutex was poisoned".to_string())
    })?;
    progress.record_completion(open, current_target);
    Ok(())
}

fn task_space_seed(route: &EthTable, space: &RoutedSocketSpace) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325u64;
    for byte in route.device.bytes() {
        seed = fold_seed(seed, byte);
    }
    for byte in route.src_ip.octets() {
        seed = fold_seed(seed, byte);
    }
    for byte in space.protocol.label().bytes() {
        seed = fold_seed(seed, byte);
    }
    for target in &space.targets {
        for byte in target.target_ip.octets() {
            seed = fold_seed(seed, byte);
        }
        for byte in target.host.bytes() {
            seed = fold_seed(seed, byte);
        }
    }
    for port in &space.ports {
        for byte in port.to_be_bytes() {
            seed = fold_seed(seed, byte);
        }
    }
    seed
}

fn fold_seed(seed: u64, value: u8) -> u64 {
    seed.wrapping_mul(0x1000_0000_01b3) ^ value as u64
}

fn mix64(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

fn gcd(mut left: usize, mut right: usize) -> usize {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn initial_probe_token(task: &RoutedSynTask, source_port: u16) -> u32 {
    let host_hash = task.task.host.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(16777619).wrapping_add(byte as u32)
    });
    u32::from(task.target_ip)
        .wrapping_mul(1_103_515_245)
        .wrapping_add((task.task.port as u32) << 16)
        .wrapping_add(source_port as u32)
        .wrapping_add(host_hash)
}

#[cfg(test)]
#[path = "raw_syn_engine_tests.rs"]
mod raw_syn_engine_tests;
