//! Packet capture commands - Tauri commands for network packet capture
//!
//! Windows Support: Requires Npcap to be installed (https://nmap.org/npcap/)
//! - Download and install Npcap in WinPcap API-compatible mode

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;
use tracing::{error, info};

use sentinel_traffic::{CapturedPacket, FileExtractor, InterfaceInfo, PacketCaptureService, PcapFileOps};

/// Packet capture state
pub struct PacketCaptureState {
    service: Arc<RwLock<PacketCaptureService>>,
}

impl Default for PacketCaptureState {
    fn default() -> Self {
        Self {
            service: Arc::new(RwLock::new(PacketCaptureService::new())),
        }
    }
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

    match service.start_capture(&interface_name) {
        Ok(mut rx) => {
            let app_handle = app.clone();
            
            // Spawn task to forward packets to frontend
            tokio::spawn(async move {
                while let Some(packet) = rx.recv().await {
                    if let Err(e) = app_handle.emit("packet-captured", &packet) {
                        error!("Failed to emit packet event: {}", e);
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
pub async fn is_capture_running(
    state: State<'_, PacketCaptureState>,
) -> Result<bool, String> {
    let service = state.service.read().await;
    Ok(service.is_running())
}

/// Open pcap/pcapng file and return packets
#[tauri::command]
pub async fn open_pcap_file(
    file_path: String,
) -> Result<Vec<CapturedPacket>, String> {
    info!("Opening pcap file: {}", file_path);
    let path = PathBuf::from(&file_path);
    
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    PcapFileOps::read_pcap_file(&path)
}

/// Save packets to pcap file
#[tauri::command]
pub async fn save_pcap_file(
    file_path: String,
    packets: Vec<CapturedPacket>,
) -> Result<(), String> {
    info!("Saving {} packets to pcap file: {}", packets.len(), file_path);
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
static EXTRACTED_FILES_CACHE: std::sync::LazyLock<tokio::sync::RwLock<Vec<sentinel_traffic::ExtractedFile>>> = 
    std::sync::LazyLock::new(|| tokio::sync::RwLock::new(Vec::new()));

#[tauri::command]
pub async fn extract_files_preview(
    packets: Vec<CapturedPacket>,
) -> Result<Vec<ExtractedFileInfo>, String> {
    info!("Extracting files preview from {} packets", packets.len());
    
    let files = FileExtractor::extract_files(&packets);
    
    // Cache extracted files for later download
    {
        let mut cache = EXTRACTED_FILES_CACHE.write().await;
        *cache = files.clone();
    }
    
    Ok(files.into_iter().map(|f| ExtractedFileInfo {
        id: f.id,
        filename: f.filename,
        content_type: f.content_type,
        size: f.size,
        src: f.src,
        dst: f.dst,
        packet_ids: f.packet_ids,
        stream_key: f.stream_key,
        source_type: f.source_type,
    }).collect())
}

/// Extract and save files from packets to directory
#[tauri::command]
pub async fn extract_files_to_dir(
    packets: Vec<CapturedPacket>,
    output_dir: String,
) -> Result<Vec<ExtractedFileInfo>, String> {
    info!("Extracting files from {} packets to: {}", packets.len(), output_dir);
    let path = PathBuf::from(&output_dir);
    
    let files = FileExtractor::extract_and_save(&packets, &path)?;
    
    Ok(files.into_iter().map(|f| ExtractedFileInfo {
        id: f.id,
        filename: f.filename,
        content_type: f.content_type,
        size: f.size,
        src: f.src,
        dst: f.dst,
        packet_ids: f.packet_ids,
        stream_key: f.stream_key,
        source_type: f.source_type,
    }).collect())
}

/// Save a single extracted file by ID
#[tauri::command]
pub async fn save_extracted_file(
    file_id: String,
    save_path: String,
) -> Result<(), String> {
    info!("Saving extracted file {} to {}", file_id, save_path);
    
    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache.iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    
    let path = PathBuf::from(&save_path);
    FileExtractor::save_file(file, &path)
}

/// Get packets related to a specific extracted file
#[tauri::command]
pub async fn get_file_related_packets(
    file_id: String,
    packets: Vec<CapturedPacket>,
) -> Result<Vec<CapturedPacket>, String> {
    info!("Getting packets related to file {}", file_id);
    
    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache.iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    
    let related: Vec<CapturedPacket> = packets.into_iter()
        .filter(|p| file.packet_ids.contains(&p.id))
        .collect();
    
    Ok(related)
}

/// Get all packets in the same stream as a file
#[tauri::command]
pub async fn get_file_stream_packets(
    file_id: String,
    packets: Vec<CapturedPacket>,
) -> Result<Vec<CapturedPacket>, String> {
    info!("Getting stream packets for file {}", file_id);
    
    let cache = EXTRACTED_FILES_CACHE.read().await;
    let file = cache.iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    
    let stream_key = file.stream_key.clone();
    
    let stream_packets: Vec<CapturedPacket> = packets.into_iter()
        .filter(|p| {
            let key = format_stream_key(&p.src, &p.dst);
            key == stream_key
        })
        .collect();
    
    Ok(stream_packets)
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
    std::fs::create_dir_all(&dir_path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    
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

