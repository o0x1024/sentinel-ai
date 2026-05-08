use futures::stream::{self, StreamExt};
use sentinel_plugins::MonitorProgressContext;
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use super::{emit_port_scan_progress, progress_emit_every, scan_progress_step, ScanEngineResult};
use crate::buildin_tools::port_scan::protocol::ScanProtocol;
use crate::buildin_tools::port_scan::targeting::{SocketOutcome, SocketScanTask, SocketStatus};

pub(crate) async fn scan_connect_sockets(
    socket_tasks: Vec<SocketScanTask>,
    timeout_ms: u64,
    concurrency: usize,
    tries: u8,
    monitor_progress: Option<&MonitorProgressContext>,
) -> ScanEngineResult {
    if socket_tasks.is_empty() {
        return ScanEngineResult {
            socket_outcomes: Vec::new(),
            host_errors: BTreeMap::new(),
        };
    }

    let total = socket_tasks.len();
    let emit_every = progress_emit_every(total);
    let mut completed = 0usize;
    let mut found_open = 0usize;
    let mut socket_outcomes = Vec::new();

    let mut task_stream = stream::iter(
        socket_tasks
            .into_iter()
            .map(|task| async move { try_connect_socket(task, timeout_ms, tries).await }),
    )
    .buffer_unordered(concurrency);

    while let Some(result) = task_stream.next().await {
        completed += 1;
        if matches!(result.status, SocketStatus::Open) {
            found_open += 1;
        }
        socket_outcomes.push(result);

        if completed % emit_every == 0 || completed == total {
            emit_port_scan_progress(
                monitor_progress,
                2 + scan_progress_step(completed, total),
                5,
                Some(format!(
                    "Connect scan progress: {}/{} sockets checked, {} open",
                    completed, total, found_open
                )),
                None,
                Some("scanning_ports".to_string()),
                Some("Scanning ports".to_string()),
                Some(false),
            );
        }
    }

    ScanEngineResult {
        socket_outcomes,
        host_errors: BTreeMap::new(),
    }
}

async fn try_connect_socket(task: SocketScanTask, timeout_ms: u64, tries: u8) -> SocketOutcome {
    debug_assert_eq!(task.protocol, ScanProtocol::Tcp);
    let socket = SocketAddr::new(task.ip, task.port);
    for attempt in 0..tries.max(1) {
        let connect_result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::net::TcpStream::connect(socket),
        )
        .await;

        if let Ok(Ok(stream)) = connect_result {
            drop(stream);
            return SocketOutcome::new(
                task.host,
                task.ip,
                task.port,
                ScanProtocol::Tcp,
                SocketStatus::Open,
                Some("connect".to_string()),
                None,
            );
        }

        if attempt + 1 < tries.max(1) {
            continue;
        }

        return match connect_result {
            Ok(Err(error)) => socket_outcome_from_connect_error(task, error.kind()),
            Err(_) => SocketOutcome::new(
                task.host,
                task.ip,
                task.port,
                ScanProtocol::Tcp,
                SocketStatus::Filtered,
                Some("timeout".to_string()),
                None,
            ),
            Ok(Ok(_)) => unreachable!("successful connect should have returned above"),
        };
    }

    unreachable!("connect retry loop should always return a final outcome")
}

fn socket_outcome_from_connect_error(task: SocketScanTask, kind: ErrorKind) -> SocketOutcome {
    let (status, reason) = match kind {
        ErrorKind::ConnectionRefused => (SocketStatus::Closed, "connection_refused"),
        ErrorKind::ConnectionReset => (SocketStatus::Closed, "connection_reset"),
        ErrorKind::ConnectionAborted => (SocketStatus::Closed, "connection_aborted"),
        ErrorKind::TimedOut => (SocketStatus::Filtered, "timeout"),
        _ => (SocketStatus::Filtered, "connect_failed"),
    };

    SocketOutcome::new(
        task.host,
        task.ip,
        task.port,
        ScanProtocol::Tcp,
        status,
        Some(reason.to_string()),
        None,
    )
}
