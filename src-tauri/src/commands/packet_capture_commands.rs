//! Packet capture commands - Tauri commands for network packet capture
//!
//! Windows Support: Requires Npcap to be installed (https://nmap.org/npcap/)
//! - Download and install Npcap in WinPcap API-compatible mode

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing::{error, info};

use sentinel_traffic::{
    CapturedPacket, FileExtractor, InterfaceInfo, PacketCaptureService, PcapFileOps,
};

/// Packet capture state
pub struct PacketCaptureState {
    service: Arc<RwLock<PacketCaptureService>>,
    packet_cache: Arc<RwLock<Vec<CapturedPacket>>>,
}

const PACKET_BATCH_SIZE: usize = 100;
const PACKET_BATCH_FLUSH_INTERVAL_MS: u64 = 50;
const MAX_PACKET_CACHE_SIZE: usize = 10000;
const TRIMMED_PACKET_CACHE_SIZE: usize = 8000;

impl Default for PacketCaptureState {
    fn default() -> Self {
        Self {
            service: Arc::new(RwLock::new(PacketCaptureService::new())),
            packet_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSummary {
    pub id: u64,
    pub timestamp: i64,
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub length: usize,
    pub info: String,
}

impl From<&CapturedPacket> for PacketSummary {
    fn from(packet: &CapturedPacket) -> Self {
        Self {
            id: packet.id,
            timestamp: packet.timestamp,
            src: packet.src.clone(),
            dst: packet.dst.clone(),
            protocol: packet.protocol.clone(),
            length: packet.length,
            info: packet.info.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedPacketFilterRequest {
    pub search_text: String,
    pub protocols: Vec<String>,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: String,
    pub dst_port: String,
    pub contains_string: String,
    pub contains_hex: String,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub tcp_flags: Vec<String>,
}

fn trim_packet_cache(cache: &mut Vec<CapturedPacket>) {
    if cache.len() <= MAX_PACKET_CACHE_SIZE {
        return;
    }

    let remove_count = cache.len().saturating_sub(TRIMMED_PACKET_CACHE_SIZE);
    cache.drain(0..remove_count);
}

fn match_port(addr: &str, port_filter: &str) -> bool {
    let port = addr
        .split(':')
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);

    if let Some((min, max)) = port_filter.split_once('-') {
        let min = min.parse::<u16>().unwrap_or(0);
        let max = max.parse::<u16>().unwrap_or(0);
        return port >= min && port <= max;
    }

    port == port_filter.parse::<u16>().unwrap_or(0)
}

fn packet_matches_advanced_filter(
    packet: &CapturedPacket,
    filter: &AdvancedPacketFilterRequest,
) -> bool {
    if !filter.search_text.is_empty() {
        let search_text = filter.search_text.to_lowercase();
        let packet_text = format!(
            "{} {} {} {}",
            packet.src, packet.dst, packet.protocol, packet.info
        )
        .to_lowercase();

        if !packet_text.contains(&search_text) {
            return false;
        }
    }

    if !filter.protocols.is_empty()
        && !filter
            .protocols
            .iter()
            .any(|protocol| protocol == &packet.protocol)
    {
        return false;
    }

    if !filter.src_ip.is_empty() && !packet.src.starts_with(&filter.src_ip) {
        return false;
    }

    if !filter.dst_ip.is_empty() && !packet.dst.starts_with(&filter.dst_ip) {
        return false;
    }

    if !filter.src_port.is_empty() && !match_port(&packet.src, &filter.src_port) {
        return false;
    }

    if !filter.dst_port.is_empty() && !match_port(&packet.dst, &filter.dst_port) {
        return false;
    }

    if let Some(min_length) = filter.min_length {
        if packet.length < min_length {
            return false;
        }
    }

    if let Some(max_length) = filter.max_length {
        if packet.length > max_length {
            return false;
        }
    }

    if !filter.contains_string.is_empty() {
        let ascii = packet
            .raw
            .iter()
            .map(|byte| {
                if (32..=126).contains(byte) {
                    char::from(*byte)
                } else {
                    '\0'
                }
            })
            .collect::<String>()
            .to_lowercase();

        if !ascii.contains(&filter.contains_string.to_lowercase()) {
            return false;
        }
    }

    if !filter.contains_hex.is_empty() {
        let packet_hex = packet
            .raw
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        let filter_hex = filter
            .contains_hex
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>()
            .to_lowercase();

        if !packet_hex.contains(&filter_hex) {
            return false;
        }
    }

    if !filter.tcp_flags.is_empty() {
        let flags = packet.info.to_lowercase();
        if !filter
            .tcp_flags
            .iter()
            .map(|flag| flag.to_lowercase())
            .any(|flag| flags.contains(&flag))
        {
            return false;
        }
    }

    true
}

/// Response wrapper
#[derive(Serialize, Deserialize)]
pub struct CaptureResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CaptureResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Get available network interfaces
#[tauri::command]
pub async fn get_network_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    info!("Getting network interfaces");
    Ok(PacketCaptureService::get_interfaces())
}

/// Start packet capture on specified interface
#[tauri::command]
pub async fn start_packet_capture(
    app: AppHandle,
    state: State<'_, PacketCaptureState>,
    interface_name: String,
) -> Result<CaptureResponse<()>, String> {
    info!("Starting packet capture on interface: {}", interface_name);

    let mut service = state.service.write().await;

    if service.is_running() {
        return Ok(CaptureResponse::err("Capture already running"));
    }

    {
        let mut cache = state.packet_cache.write().await;
        cache.clear();
    }

    match service.start_capture(&interface_name) {
        Ok(mut rx) => {
            let app_handle = app.clone();
            let packet_cache = state.packet_cache.clone();

            // Spawn task to forward packets to frontend
            tokio::spawn(async move {
                let mut batch = Vec::with_capacity(PACKET_BATCH_SIZE);
                let mut flush_timer =
                    time::interval(Duration::from_millis(PACKET_BATCH_FLUSH_INTERVAL_MS));
                flush_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        maybe_packet = rx.recv() => {
                            match maybe_packet {
                                Some(packet) => {
                                    let summary = PacketSummary::from(&packet);

                                    {
                                        let mut cache = packet_cache.write().await;
                                        cache.push(packet);
                                        trim_packet_cache(&mut cache);
                                    }

                                    batch.push(summary);

                                    if batch.len() >= PACKET_BATCH_SIZE {
                                        if let Err(e) = app_handle.emit("packet-captured-batch", &batch) {
                                            error!("Failed to emit packet batch event: {}", e);
                                        }
                                        batch.clear();
                                    }
                                }
                                None => {
                                    if !batch.is_empty() {
                                        if let Err(e) = app_handle.emit("packet-captured-batch", &batch) {
                                            error!("Failed to emit packet batch event: {}", e);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        _ = flush_timer.tick() => {
                            if !batch.is_empty() {
                                if let Err(e) = app_handle.emit("packet-captured-batch", &batch) {
                                    error!("Failed to emit packet batch event: {}", e);
                                }
                                batch.clear();
                            }
                        }
                    }
                }
                info!("Packet forwarding task ended");
            });

            Ok(CaptureResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to start capture: {}", e);
            Ok(CaptureResponse::err(e))
        }
    }
}

/// Stop packet capture
#[tauri::command]
pub async fn stop_packet_capture(
    state: State<'_, PacketCaptureState>,
) -> Result<CaptureResponse<()>, String> {
    info!("Stopping packet capture");

    let mut service = state.service.write().await;
    service.stop_capture();

    Ok(CaptureResponse::ok(()))
}

/// Check if capture is running
#[tauri::command]
pub async fn is_capture_running(state: State<'_, PacketCaptureState>) -> Result<bool, String> {
    let service = state.service.read().await;
    Ok(service.is_running())
}

#[tauri::command]
pub async fn clear_packet_capture_cache(
    state: State<'_, PacketCaptureState>,
) -> Result<(), String> {
    {
        let mut cache = state.packet_cache.write().await;
        cache.clear();
    }
    {
        let mut extracted = EXTRACTED_FILES_CACHE.write().await;
        extracted.clear();
    }
    Ok(())
}

/// Open pcap/pcapng file and return packets
#[tauri::command]
pub async fn open_pcap_file(
    state: State<'_, PacketCaptureState>,
    file_path: String,
) -> Result<Vec<PacketSummary>, String> {
    info!("Opening pcap file: {}", file_path);
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let packets = PcapFileOps::read_pcap_file(&path)?;
    let summaries = packets.iter().map(PacketSummary::from).collect();

    let mut cache = state.packet_cache.write().await;
    *cache = packets;

    Ok(summaries)
}

/// Save packets to pcap file
#[tauri::command]
pub async fn save_pcap_file(
    state: State<'_, PacketCaptureState>,
    file_path: String,
) -> Result<(), String> {
    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    info!(
        "Saving {} packets to pcap file: {}",
        packets.len(),
        file_path
    );
    let path = PathBuf::from(&file_path);

    // Determine format by extension
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("pcap");

    if ext == "pcapng" {
        PcapFileOps::write_pcapng_file(&path, &packets)
    } else {
        PcapFileOps::write_pcap_file(&path, &packets)
    }
}

/// Extract file info from packets (preview without saving)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFileInfo {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub src: String,
    pub dst: String,
    pub packet_ids: Vec<u64>,
    pub stream_key: String,
    pub source_type: String,
}

/// Cached extracted files for download
static EXTRACTED_FILES_CACHE: std::sync::LazyLock<
    tokio::sync::RwLock<Vec<sentinel_traffic::ExtractedFile>>,
> = std::sync::LazyLock::new(|| tokio::sync::RwLock::new(Vec::new()));

#[tauri::command]
pub async fn extract_files_preview(
    state: State<'_, PacketCaptureState>,
) -> Result<Vec<ExtractedFileInfo>, String> {
    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    info!("Extracting files preview from {} packets", packets.len());

    let files = FileExtractor::extract_files(&packets);

    // Cache extracted files for later download
    {
        let mut cache = EXTRACTED_FILES_CACHE.write().await;
        *cache = files.clone();
    }

    Ok(files
        .into_iter()
        .map(|f| ExtractedFileInfo {
            id: f.id,
            filename: f.filename,
            content_type: f.content_type,
            size: f.size,
            src: f.src,
            dst: f.dst,
            packet_ids: f.packet_ids,
            stream_key: f.stream_key,
            source_type: f.source_type,
        })
        .collect())
}

/// Extract and save files from packets to directory
#[tauri::command]
pub async fn extract_files_to_dir(
    state: State<'_, PacketCaptureState>,
    output_dir: String,
) -> Result<Vec<ExtractedFileInfo>, String> {
    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    info!(
        "Extracting files from {} packets to: {}",
        packets.len(),
        output_dir
    );
    let path = PathBuf::from(&output_dir);

    let files = FileExtractor::extract_and_save(&packets, &path)?;

    Ok(files
        .into_iter()
        .map(|f| ExtractedFileInfo {
            id: f.id,
            filename: f.filename,
            content_type: f.content_type,
            size: f.size,
            src: f.src,
            dst: f.dst,
            packet_ids: f.packet_ids,
            stream_key: f.stream_key,
            source_type: f.source_type,
        })
        .collect())
}

/// Save a single extracted file by ID
#[tauri::command]
pub async fn save_extracted_file(file_id: String, save_path: String) -> Result<(), String> {
    info!("Saving extracted file {} to {}", file_id, save_path);

    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache
        .iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;

    let path = PathBuf::from(&save_path);
    FileExtractor::save_file(file, &path)
}

/// Get packets related to a specific extracted file
#[tauri::command]
pub async fn get_file_related_packets(
    state: State<'_, PacketCaptureState>,
    file_id: String,
) -> Result<Vec<CapturedPacket>, String> {
    info!("Getting packets related to file {}", file_id);

    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache
        .iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;

    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    let related: Vec<CapturedPacket> = packets
        .into_iter()
        .filter(|p| file.packet_ids.contains(&p.id))
        .collect();

    Ok(related)
}

/// Get all packets in the same stream as a file
#[tauri::command]
pub async fn get_file_stream_packets(
    state: State<'_, PacketCaptureState>,
    file_id: String,
) -> Result<Vec<CapturedPacket>, String> {
    info!("Getting stream packets for file {}", file_id);

    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache
        .iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;

    let stream_key = file.stream_key.clone();

    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    let stream_packets: Vec<CapturedPacket> = packets
        .into_iter()
        .filter(|p| {
            let key = format_stream_key(&p.src, &p.dst);
            key == stream_key
        })
        .collect();

    Ok(stream_packets)
}

#[tauri::command]
pub async fn get_packet_details(
    state: State<'_, PacketCaptureState>,
    packet_id: u64,
) -> Result<CapturedPacket, String> {
    let cache = state.packet_cache.read().await;
    cache
        .iter()
        .find(|packet| packet.id == packet_id)
        .cloned()
        .ok_or_else(|| format!("Packet not found: {}", packet_id))
}

#[tauri::command]
pub async fn get_packet_stream_packets(
    state: State<'_, PacketCaptureState>,
    packet_id: u64,
    stream_type: String,
) -> Result<Vec<CapturedPacket>, String> {
    let packets = {
        let cache = state.packet_cache.read().await;
        cache.clone()
    };

    let packet = packets
        .iter()
        .find(|item| item.id == packet_id)
        .ok_or_else(|| format!("Packet not found: {}", packet_id))?;

    let src_ip = packet.src.split(':').next().unwrap_or_default().to_string();
    let dst_ip = packet.dst.split(':').next().unwrap_or_default().to_string();
    let src_port = packet.src.split(':').nth(1).unwrap_or_default().to_string();
    let dst_port = packet.dst.split(':').nth(1).unwrap_or_default().to_string();

    let protocol_filter: &[&str] = match stream_type.as_str() {
        "tcp" => &["TCP", "HTTP", "HTTPS", "TLS"],
        "udp" => &["UDP", "DNS", "DHCP", "NTP", "QUIC", "mDNS", "LLMNR"],
        "http" => &["HTTP", "HTTPS"],
        other => return Err(format!("Unsupported stream type: {}", other)),
    };

    let stream_packets = packets
        .into_iter()
        .filter(|candidate| {
            if !protocol_filter.iter().any(|proto| {
                candidate.protocol.contains(proto) || proto.contains(&candidate.protocol)
            }) {
                return false;
            }

            let candidate_src_ip = candidate.src.split(':').next().unwrap_or_default();
            let candidate_dst_ip = candidate.dst.split(':').next().unwrap_or_default();
            let candidate_src_port = candidate.src.split(':').nth(1).unwrap_or_default();
            let candidate_dst_port = candidate.dst.split(':').nth(1).unwrap_or_default();

            let forward_match = candidate_src_ip == src_ip
                && candidate_dst_ip == dst_ip
                && candidate_src_port == src_port
                && candidate_dst_port == dst_port;
            let reverse_match = candidate_src_ip == dst_ip
                && candidate_dst_ip == src_ip
                && candidate_src_port == dst_port
                && candidate_dst_port == src_port;

            forward_match || reverse_match
        })
        .collect();

    Ok(stream_packets)
}

#[tauri::command]
pub async fn match_packet_advanced_filter(
    state: State<'_, PacketCaptureState>,
    packet_ids: Vec<u64>,
    filter: AdvancedPacketFilterRequest,
) -> Result<Vec<u64>, String> {
    let packets = state.packet_cache.read().await;
    let packet_id_set: std::collections::HashSet<u64> = packet_ids.into_iter().collect();

    let matches = packets
        .iter()
        .filter(|packet| packet_id_set.contains(&packet.id))
        .filter(|packet| packet_matches_advanced_filter(packet, &filter))
        .map(|packet| packet.id)
        .collect();

    Ok(matches)
}

fn format_stream_key(src: &str, dst: &str) -> String {
    let mut parts = [src, dst];
    parts.sort();
    format!("{}-{}", parts[0], parts[1])
}

/// Save selected files by IDs to directory
#[tauri::command]
pub async fn save_selected_files(
    file_ids: Vec<String>,
    output_dir: String,
) -> Result<Vec<String>, String> {
    info!("Saving {} selected files to {}", file_ids.len(), output_dir);

    let dir_path = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&dir_path).map_err(|e| format!("Failed to create directory: {}", e))?;

    let cache = EXTRACTED_FILES_CACHE.read().await;
    let mut saved_files = Vec::new();

    for file_id in file_ids {
        if let Some(file) = cache.iter().find(|f| f.id == file_id) {
            let file_path = dir_path.join(&file.filename);
            std::fs::write(&file_path, &file.data)
                .map_err(|e| format!("Failed to save {}: {}", file.filename, e))?;
            saved_files.push(file.filename.clone());
        }
    }

    Ok(saved_files)
}
