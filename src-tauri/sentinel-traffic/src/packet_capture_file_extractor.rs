use crate::packet_capture::CapturedPacket;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// Extracted file from traffic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFile {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub src: String,
    pub dst: String,
    pub data: Vec<u8>,
    pub packet_ids: Vec<u64>,
    pub stream_key: String,
    pub source_type: String, // "HTTP" or "TCP"
}

/// File magic signatures for CTF-style traffic analysis
struct FileMagic {
    magic: &'static [u8],
    ext: &'static str,
    mime: &'static str,
    has_end_marker: bool,
}

const FILE_MAGICS: &[FileMagic] = &[
    // Images
    FileMagic {
        magic: &[0xFF, 0xD8, 0xFF],
        ext: "jpg",
        mime: "image/jpeg",
        has_end_marker: true,
    },
    FileMagic {
        magic: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        ext: "png",
        mime: "image/png",
        has_end_marker: true,
    },
    FileMagic {
        magic: b"GIF87a",
        ext: "gif",
        mime: "image/gif",
        has_end_marker: true,
    },
    FileMagic {
        magic: b"GIF89a",
        ext: "gif",
        mime: "image/gif",
        has_end_marker: true,
    },
    FileMagic {
        magic: b"RIFF",
        ext: "webp",
        mime: "image/webp",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x00, 0x00, 0x01, 0x00],
        ext: "ico",
        mime: "image/x-icon",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"BM",
        ext: "bmp",
        mime: "image/bmp",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x49, 0x49, 0x2A, 0x00],
        ext: "tiff",
        mime: "image/tiff",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x4D, 0x4D, 0x00, 0x2A],
        ext: "tiff",
        mime: "image/tiff",
        has_end_marker: false,
    },
    // Documents
    FileMagic {
        magic: b"%PDF",
        ext: "pdf",
        mime: "application/pdf",
        has_end_marker: true,
    },
    FileMagic {
        magic: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        ext: "doc",
        mime: "application/msword",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"PK\x03\x04\x14\x00\x06\x00",
        ext: "docx",
        mime: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"{\\rtf",
        ext: "rtf",
        mime: "application/rtf",
        has_end_marker: false,
    },
    // Archives
    FileMagic {
        magic: &[0x50, 0x4B, 0x03, 0x04],
        ext: "zip",
        mime: "application/zip",
        has_end_marker: true,
    },
    FileMagic {
        magic: &[0x50, 0x4B, 0x05, 0x06],
        ext: "zip",
        mime: "application/zip",
        has_end_marker: true,
    },
    FileMagic {
        magic: &[0x1F, 0x8B],
        ext: "gz",
        mime: "application/gzip",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07],
        ext: "rar",
        mime: "application/x-rar-compressed",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
        ext: "7z",
        mime: "application/x-7z-compressed",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x75, 0x73, 0x74, 0x61, 0x72],
        ext: "tar",
        mime: "application/x-tar",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x42, 0x5A, 0x68],
        ext: "bz2",
        mime: "application/x-bzip2",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00],
        ext: "xz",
        mime: "application/x-xz",
        has_end_marker: false,
    },
    // Video
    FileMagic {
        magic: &[0x00, 0x00, 0x00, 0x1C, 0x66, 0x74, 0x79, 0x70],
        ext: "mp4",
        mime: "video/mp4",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70],
        ext: "mp4",
        mime: "video/mp4",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70],
        ext: "mp4",
        mime: "video/mp4",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x1A, 0x45, 0xDF, 0xA3],
        ext: "webm",
        mime: "video/webm",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"FLV",
        ext: "flv",
        mime: "video/x-flv",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70],
        ext: "mov",
        mime: "video/quicktime",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11],
        ext: "wmv",
        mime: "video/x-ms-wmv",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"RIFF",
        ext: "avi",
        mime: "video/x-msvideo",
        has_end_marker: false,
    },
    // Audio
    FileMagic {
        magic: &[0x49, 0x44, 0x33],
        ext: "mp3",
        mime: "audio/mpeg",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xFF, 0xFB],
        ext: "mp3",
        mime: "audio/mpeg",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xFF, 0xFA],
        ext: "mp3",
        mime: "audio/mpeg",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xFF, 0xF3],
        ext: "mp3",
        mime: "audio/mpeg",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"OggS",
        ext: "ogg",
        mime: "audio/ogg",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"fLaC",
        ext: "flac",
        mime: "audio/flac",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[
            0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x41,
        ],
        ext: "m4a",
        mime: "audio/mp4",
        has_end_marker: false,
    },
    // Executables and scripts
    FileMagic {
        magic: &[0x4D, 0x5A],
        ext: "exe",
        mime: "application/x-msdownload",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x7F, 0x45, 0x4C, 0x46],
        ext: "elf",
        mime: "application/x-executable",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xCE, 0xFA, 0xED, 0xFE],
        ext: "macho",
        mime: "application/x-mach-binary",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xCF, 0xFA, 0xED, 0xFE],
        ext: "macho64",
        mime: "application/x-mach-binary",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"#!",
        ext: "sh",
        mime: "text/x-shellscript",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"<?php",
        ext: "php",
        mime: "text/x-php",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"<?xml",
        ext: "xml",
        mime: "application/xml",
        has_end_marker: false,
    },
    // Fonts
    FileMagic {
        magic: &[0x00, 0x01, 0x00, 0x00],
        ext: "ttf",
        mime: "font/ttf",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"wOFF",
        ext: "woff",
        mime: "font/woff",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"wOF2",
        ext: "woff2",
        mime: "font/woff2",
        has_end_marker: false,
    },
    // Database
    FileMagic {
        magic: b"SQLite format 3",
        ext: "sqlite",
        mime: "application/x-sqlite3",
        has_end_marker: false,
    },
    // CTF common: flags, keys, certificates
    FileMagic {
        magic: b"-----BEGIN",
        ext: "pem",
        mime: "application/x-pem-file",
        has_end_marker: true,
    },
    FileMagic {
        magic: b"ssh-rsa ",
        ext: "pub",
        mime: "text/plain",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"ssh-ed25519 ",
        ext: "pub",
        mime: "text/plain",
        has_end_marker: false,
    },
    // Java
    FileMagic {
        magic: &[0xCA, 0xFE, 0xBA, 0xBE],
        ext: "class",
        mime: "application/java-vm",
        has_end_marker: false,
    },
    // Python compiled
    FileMagic {
        magic: &[0x03, 0xF3, 0x0D, 0x0A],
        ext: "pyc",
        mime: "application/x-python-code",
        has_end_marker: false,
    },
    // Pcap (nested capture files)
    FileMagic {
        magic: &[0xD4, 0xC3, 0xB2, 0xA1],
        ext: "pcap",
        mime: "application/vnd.tcpdump.pcap",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0xA1, 0xB2, 0xC3, 0xD4],
        ext: "pcap",
        mime: "application/vnd.tcpdump.pcap",
        has_end_marker: false,
    },
    FileMagic {
        magic: &[0x0A, 0x0D, 0x0D, 0x0A],
        ext: "pcapng",
        mime: "application/x-pcapng",
        has_end_marker: false,
    },
    // Disk images
    FileMagic {
        magic: b"QEMU",
        ext: "qcow",
        mime: "application/x-qemu-disk",
        has_end_marker: false,
    },
    FileMagic {
        magic: b"conectix",
        ext: "vhd",
        mime: "application/x-vhd",
        has_end_marker: false,
    },
];

/// CTF-style file extraction from all traffic protocols
pub struct FileExtractor;

impl FileExtractor {
    /// Extract files from all captured packets using multiple methods
    pub fn extract_files(packets: &[CapturedPacket]) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut file_counter = 0u64;

        // 1. Group packets by stream (TCP/UDP)
        let mut streams: HashMap<String, Vec<&CapturedPacket>> = HashMap::new();
        for pkt in packets {
            let key = Self::stream_key(&pkt.src, &pkt.dst);
            streams.entry(key).or_default().push(pkt);
        }

        // 2. Process each stream with multiple extraction methods
        for (stream_key, stream_packets) in &streams {
            // Sort by packet id
            let mut sorted: Vec<_> = stream_packets.to_vec();
            sorted.sort_by_key(|p| p.id);

            // HTTP extraction
            files.extend(Self::extract_http_files(&sorted, stream_key));

            // FTP data extraction
            files.extend(Self::extract_ftp_files(&sorted, stream_key));

            // SMTP/Email attachment extraction
            files.extend(Self::extract_email_attachments(&sorted, stream_key));

            // Raw magic-based extraction from all protocols
            files.extend(Self::extract_magic_from_stream(&sorted, stream_key));
        }

        // 3. Scan all packets for magic bytes (catches files in any protocol)
        files.extend(Self::extract_magic_from_all_packets(packets));

        // 4. Extract Base64 encoded data
        files.extend(Self::extract_base64_data(packets));

        // 5. Extract DNS tunnel data
        files.extend(Self::extract_dns_tunnel_data(packets));

        // 6. Extract ICMP tunnel data
        files.extend(Self::extract_icmp_data(packets));

        // Assign IDs and deduplicate
        for f in &mut files {
            file_counter += 1;
            f.id = format!("file_{}", file_counter);
        }
        files = Self::deduplicate_files(files);

        info!("Extracted {} files from traffic (CTF mode)", files.len());
        files
    }

    fn stream_key(src: &str, dst: &str) -> String {
        let mut parts = [src, dst];
        parts.sort();
        format!("{}-{}", parts[0], parts[1])
    }

    /// Extract files from HTTP responses
    fn extract_http_files(packets: &[&CapturedPacket], stream_key: &str) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut response_data = Vec::new();
        let mut content_type = String::new();
        let mut filename = String::new();
        let mut src = String::new();
        let mut dst = String::new();
        let mut in_response = false;
        let mut packet_ids = Vec::new();

        for pkt in packets {
            let text = String::from_utf8_lossy(&pkt.raw);

            // Detect HTTP response
            if text.contains("HTTP/1.") || text.contains("HTTP/2") {
                if text.contains(" 200 ") || text.contains(" 206 ") || text.contains(" 304 ") {
                    // Save previous response
                    if !response_data.is_empty() {
                        if let Some(file) = Self::create_extracted_file(
                            &response_data,
                            &content_type,
                            &filename,
                            &src,
                            &dst,
                            &packet_ids,
                            stream_key,
                            "HTTP",
                        ) {
                            files.push(file);
                        }
                    }

                    response_data.clear();
                    content_type.clear();
                    filename.clear();
                    packet_ids.clear();

                    in_response = true;
                    src = pkt.src.clone();
                    dst = pkt.dst.clone();
                    packet_ids.push(pkt.id);

                    // Parse headers
                    for line in text.lines() {
                        let line_lower = line.to_lowercase();
                        if line_lower.starts_with("content-type:") {
                            content_type = line[13..].trim().to_string();
                        } else if line_lower.starts_with("content-disposition:") {
                            if let Some(fn_idx) = line.to_lowercase().find("filename=") {
                                let name = &line[fn_idx + 9..];
                                filename = name
                                    .trim_matches(|c| c == '"' || c == '\'' || c == ';')
                                    .to_string();
                            }
                        }
                    }

                    // Extract body
                    if let Some(body_start) = text.find("\r\n\r\n") {
                        if body_start + 4 < pkt.raw.len() {
                            response_data.extend_from_slice(&pkt.raw[body_start + 4..]);
                        }
                    }
                }
            } else if in_response && pkt.src == src {
                packet_ids.push(pkt.id);
                response_data.extend_from_slice(&pkt.raw);
            }
        }

        // Save last response
        if !response_data.is_empty() {
            if let Some(file) = Self::create_extracted_file(
                &response_data,
                &content_type,
                &filename,
                &src,
                &dst,
                &packet_ids,
                stream_key,
                "HTTP",
            ) {
                files.push(file);
            }
        }

        files
    }

    /// Extract files from FTP data transfers
    fn extract_ftp_files(packets: &[&CapturedPacket], stream_key: &str) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut data_buffer = Vec::new();
        let mut packet_ids = Vec::new();
        let mut current_filename = String::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            let text = String::from_utf8_lossy(&pkt.raw);

            // Detect FTP RETR command (file download)
            if text.contains("RETR ") {
                if let Some(start) = text.find("RETR ") {
                    current_filename = text[start + 5..]
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                }
            }

            // Detect FTP 150/226 responses (transfer start/complete)
            if text.contains("150 ") || text.contains("125 ") {
                data_buffer.clear();
                packet_ids.clear();
                src = pkt.src.clone();
                dst = pkt.dst.clone();
            }

            // FTP data port (usually high port, binary data)
            let src_port: u16 = pkt
                .src
                .split(':')
                .next_back()
                .and_then(|p| p.parse().ok())
                .unwrap_or(0);
            let dst_port: u16 = pkt
                .dst
                .split(':')
                .next_back()
                .and_then(|p| p.parse().ok())
                .unwrap_or(0);

            if (src_port == 20 || dst_port == 20 || src_port > 1024 && dst_port > 1024)
                && pkt.protocol == "TCP"
                && pkt.raw.len() > 60
            {
                // Check if this looks like binary data
                let payload = if pkt.raw.len() > 54 {
                    &pkt.raw[54..]
                } else {
                    &pkt.raw
                };
                if !payload.is_empty() && !text.starts_with("220 ") && !text.starts_with("USER ") {
                    data_buffer.extend_from_slice(payload);
                    packet_ids.push(pkt.id);
                    if src.is_empty() {
                        src = pkt.src.clone();
                        dst = pkt.dst.clone();
                    }
                }
            }

            // Save on FTP transfer complete
            if text.contains("226 ") && !data_buffer.is_empty() {
                let filename = if current_filename.is_empty() {
                    "ftp_transfer".to_string()
                } else {
                    current_filename.clone()
                };
                if let Some(file) = Self::create_extracted_file(
                    &data_buffer,
                    "",
                    &filename,
                    &src,
                    &dst,
                    &packet_ids,
                    stream_key,
                    "FTP",
                ) {
                    files.push(file);
                }
                data_buffer.clear();
                packet_ids.clear();
            }
        }

        files
    }

    /// Extract email attachments from SMTP/POP3/IMAP
    fn extract_email_attachments(
        packets: &[&CapturedPacket],
        stream_key: &str,
    ) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut email_data = String::new();
        let mut packet_ids = Vec::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            let text = String::from_utf8_lossy(&pkt.raw);

            // Check for email protocols
            let is_email = pkt.protocol == "SMTP"
                || pkt.protocol == "POP3"
                || pkt.protocol == "IMAP"
                || text.contains("MAIL FROM:")
                || text.contains("DATA\r\n")
                || text.contains("Content-Transfer-Encoding:");

            if is_email {
                email_data.push_str(&text);
                packet_ids.push(pkt.id);
                if src.is_empty() {
                    src = pkt.src.clone();
                    dst = pkt.dst.clone();
                }
            }
        }

        // Parse MIME attachments
        if !email_data.is_empty() {
            files.extend(Self::parse_mime_attachments(
                &email_data,
                &src,
                &dst,
                &packet_ids,
                stream_key,
            ));
        }

        files
    }

    /// Parse MIME attachments from email data
    fn parse_mime_attachments(
        data: &str,
        src: &str,
        dst: &str,
        packet_ids: &[u64],
        stream_key: &str,
    ) -> Vec<ExtractedFile> {
        let mut files = Vec::new();

        // Find boundary
        let boundary = data
            .lines()
            .find(|l| l.to_lowercase().contains("boundary="))
            .and_then(|l| {
                l.split("boundary=")
                    .nth(1)
                    .map(|b| b.trim_matches(|c| c == '"' || c == '\'' || c == ';'))
            });

        if let Some(boundary) = boundary {
            let boundary_marker = format!("--{}", boundary);
            let parts: Vec<&str> = data.split(&boundary_marker).collect();

            for part in parts {
                // Check for attachment
                if part
                    .to_lowercase()
                    .contains("content-disposition: attachment")
                    || part
                        .to_lowercase()
                        .contains("content-transfer-encoding: base64")
                {
                    // Get filename
                    let filename = part
                        .lines()
                        .find(|l| l.to_lowercase().contains("filename="))
                        .and_then(|l| l.split("filename=").nth(1))
                        .map(|n| n.trim_matches(|c| c == '"' || c == '\'').to_string())
                        .unwrap_or_else(|| "attachment".to_string());

                    // Get content type
                    let content_type = part
                        .lines()
                        .find(|l| l.to_lowercase().starts_with("content-type:"))
                        .map(|l| l[13..].trim().split(';').next().unwrap_or("").to_string())
                        .unwrap_or_default();

                    // Decode base64 content
                    if let Some(body_start) = part.find("\r\n\r\n").or_else(|| part.find("\n\n")) {
                        let body = &part[body_start..].trim();
                        let cleaned: String = body.chars().filter(|c| !c.is_whitespace()).collect();
                        if let Ok(decoded) = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            &cleaned,
                        ) {
                            if decoded.len() > 10 {
                                files.push(ExtractedFile {
                                    id: String::new(),
                                    filename,
                                    content_type,
                                    size: decoded.len(),
                                    src: src.to_string(),
                                    dst: dst.to_string(),
                                    data: decoded,
                                    packet_ids: packet_ids.to_vec(),
                                    stream_key: stream_key.to_string(),
                                    source_type: "EMAIL".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Extract files from TCP/UDP stream using magic bytes
    fn extract_magic_from_stream(
        packets: &[&CapturedPacket],
        stream_key: &str,
    ) -> Vec<ExtractedFile> {
        let mut files = Vec::new();

        // Reassemble stream data
        let mut stream_data = Vec::new();
        let mut packet_ids = Vec::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            // Extract payload (skip headers)
            let payload = Self::extract_payload(&pkt.raw);
            if !payload.is_empty() {
                stream_data.extend_from_slice(payload);
                packet_ids.push(pkt.id);
                if src.is_empty() {
                    src = pkt.src.clone();
                    dst = pkt.dst.clone();
                }
            }
        }

        if stream_data.len() < 10 {
            return files;
        }

        // Scan for file magic
        let mut offset = 0;
        while offset < stream_data.len().saturating_sub(8) {
            if let Some((magic, file_data)) = Self::detect_file_at_offset(&stream_data, offset) {
                if file_data.len() >= 20 {
                    files.push(ExtractedFile {
                        id: String::new(),
                        filename: Self::generate_filename_with_ext(magic.ext),
                        content_type: magic.mime.to_string(),
                        size: file_data.len(),
                        src: src.clone(),
                        dst: dst.clone(),
                        data: file_data.clone(),
                        packet_ids: packet_ids.clone(),
                        stream_key: stream_key.to_string(),
                        source_type: "STREAM".to_string(),
                    });
                    offset += file_data.len();
                } else {
                    offset += 1;
                }
            } else {
                offset += 1;
            }
        }

        files
    }

    /// Extract files from all packets by scanning raw data
    fn extract_magic_from_all_packets(packets: &[CapturedPacket]) -> Vec<ExtractedFile> {
        let mut files = Vec::new();

        for pkt in packets {
            // Skip small packets
            if pkt.raw.len() < 50 {
                continue;
            }

            // Scan entire raw packet for file signatures
            let mut offset = 0;
            while offset < pkt.raw.len().saturating_sub(8) {
                if let Some((magic, file_data)) = Self::detect_file_at_offset(&pkt.raw, offset) {
                    if file_data.len() >= 20 && file_data.len() < pkt.raw.len() - 10 {
                        files.push(ExtractedFile {
                            id: String::new(),
                            filename: Self::generate_filename_with_ext(magic.ext),
                            content_type: magic.mime.to_string(),
                            size: file_data.len(),
                            src: pkt.src.clone(),
                            dst: pkt.dst.clone(),
                            data: file_data.clone(),
                            packet_ids: vec![pkt.id],
                            stream_key: Self::stream_key(&pkt.src, &pkt.dst),
                            source_type: pkt.protocol.clone(),
                        });
                        offset += file_data.len();
                    } else {
                        offset += 1;
                    }
                } else {
                    offset += 1;
                }
            }
        }

        files
    }

    /// Extract Base64 encoded data from packets
    fn extract_base64_data(packets: &[CapturedPacket]) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let base64_regex = regex::Regex::new(
            r"(?:[A-Za-z0-9+/]{4}){10,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?",
        )
        .ok();

        if let Some(re) = base64_regex {
            for pkt in packets {
                let text = String::from_utf8_lossy(&pkt.raw);

                for cap in re.find_iter(&text) {
                    let b64_str = cap.as_str();
                    if b64_str.len() < 100 {
                        continue;
                    } // Skip short strings

                    if let Ok(decoded) =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64_str)
                    {
                        if decoded.len() < 20 {
                            continue;
                        }

                        // Check if decoded data has file magic
                        let (ext, mime) = Self::detect_type_from_magic(&decoded);

                        files.push(ExtractedFile {
                            id: String::new(),
                            filename: Self::generate_filename_with_ext(ext),
                            content_type: mime.to_string(),
                            size: decoded.len(),
                            src: pkt.src.clone(),
                            dst: pkt.dst.clone(),
                            data: decoded,
                            packet_ids: vec![pkt.id],
                            stream_key: Self::stream_key(&pkt.src, &pkt.dst),
                            source_type: "BASE64".to_string(),
                        });
                    }
                }
            }
        }

        files
    }

    /// Extract data from DNS tunnel (data in TXT records or subdomains)
    fn extract_dns_tunnel_data(packets: &[CapturedPacket]) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut dns_data = Vec::new();
        let mut packet_ids = Vec::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            if pkt.protocol != "DNS" {
                continue;
            }

            // Look for suspicious DNS queries (long subdomains, TXT records)
            // Extract hex/base64 from subdomain labels
            for layer in &pkt.layers {
                if layer.name == "DNS" {
                    for field in &layer.fields {
                        if field.name == "Query Name" || field.name == "TXT Data" {
                            let value = &field.value;
                            // Check for encoded data (hex or base64 looking)
                            let cleaned: String = value
                                .chars()
                                .filter(|c| {
                                    c.is_ascii_hexdigit()
                                        || c.is_ascii_alphanumeric()
                                        || *c == '+'
                                        || *c == '/'
                                })
                                .collect();

                            if cleaned.len() > 20 {
                                // Try hex decode
                                if let Ok(decoded) = hex::decode(&cleaned) {
                                    dns_data.extend_from_slice(&decoded);
                                }
                                // Try base64 decode
                                else if let Ok(decoded) = base64::Engine::decode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &cleaned,
                                ) {
                                    dns_data.extend_from_slice(&decoded);
                                }

                                packet_ids.push(pkt.id);
                                if src.is_empty() {
                                    src = pkt.src.clone();
                                    dst = pkt.dst.clone();
                                }
                            }
                        }
                    }
                }
            }
        }

        if dns_data.len() > 20 {
            let (ext, mime) = Self::detect_type_from_magic(&dns_data);
            files.push(ExtractedFile {
                id: String::new(),
                filename: format!("dns_exfil.{}", ext),
                content_type: mime.to_string(),
                size: dns_data.len(),
                src,
                dst,
                data: dns_data,
                packet_ids,
                stream_key: String::new(),
                source_type: "DNS_TUNNEL".to_string(),
            });
        }

        files
    }

    /// Extract data from ICMP packets (ping tunnel)
    fn extract_icmp_data(packets: &[CapturedPacket]) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut icmp_data = Vec::new();
        let mut packet_ids = Vec::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            if pkt.protocol != "ICMP" {
                continue;
            }

            // ICMP data portion starts after IP header (20 bytes) + ICMP header (8 bytes)
            if pkt.raw.len() > 42 {
                // Skip Ethernet (14) + IP (20) + ICMP header (8)
                let icmp_payload = &pkt.raw[42..];

                // Check if it's not just padding/zeros
                if icmp_payload.iter().any(|&b| b != 0) {
                    icmp_data.extend_from_slice(icmp_payload);
                    packet_ids.push(pkt.id);
                    if src.is_empty() {
                        src = pkt.src.clone();
                        dst = pkt.dst.clone();
                    }
                }
            }
        }

        if icmp_data.len() > 20 {
            let (ext, mime) = Self::detect_type_from_magic(&icmp_data);
            files.push(ExtractedFile {
                id: String::new(),
                filename: format!("icmp_data.{}", ext),
                content_type: mime.to_string(),
                size: icmp_data.len(),
                src,
                dst,
                data: icmp_data,
                packet_ids,
                stream_key: String::new(),
                source_type: "ICMP_TUNNEL".to_string(),
            });
        }

        files
    }

    /// Extract payload from raw packet (skip headers)
    fn extract_payload(raw: &[u8]) -> &[u8] {
        if raw.len() < 54 {
            return &[];
        }

        // Ethernet (14) + IP (20 min) + TCP/UDP (8-20)
        // Try to find actual payload start
        let eth_type = if raw.len() > 13 {
            ((raw[12] as u16) << 8) | raw[13] as u16
        } else {
            0
        };

        let ip_start = 14;
        if eth_type != 0x0800 && eth_type != 0x86DD {
            return raw;
        } // Not IP

        let ip_header_len = if raw.len() > ip_start {
            ((raw[ip_start] & 0x0F) as usize) * 4
        } else {
            20
        };
        let transport_start = ip_start + ip_header_len;

        if raw.len() <= transport_start + 8 {
            return &[];
        }

        let proto = raw.get(ip_start + 9).copied().unwrap_or(0);
        let transport_header_len = match proto {
            6 => {
                // TCP
                if raw.len() > transport_start + 12 {
                    ((raw[transport_start + 12] >> 4) as usize) * 4
                } else {
                    20
                }
            }
            17 => 8, // UDP
            _ => 8,
        };

        let payload_start = transport_start + transport_header_len;
        if payload_start < raw.len() {
            &raw[payload_start..]
        } else {
            &[]
        }
    }

    fn detect_file_at_offset(data: &[u8], offset: usize) -> Option<(&'static FileMagic, Vec<u8>)> {
        for magic in FILE_MAGICS {
            if offset + magic.magic.len() <= data.len()
                && &data[offset..offset + magic.magic.len()] == magic.magic
            {
                let end = Self::estimate_file_end(data, offset, magic);
                if end > offset {
                    return Some((magic, data[offset..end].to_vec()));
                }
            }
        }
        None
    }

    fn detect_type_from_magic(data: &[u8]) -> (&'static str, &'static str) {
        for magic in FILE_MAGICS {
            if data.len() >= magic.magic.len() && &data[..magic.magic.len()] == magic.magic {
                return (magic.ext, magic.mime);
            }
        }
        ("bin", "application/octet-stream")
    }

    fn estimate_file_end(data: &[u8], start: usize, magic: &FileMagic) -> usize {
        let remaining = data.len() - start;
        let max_size = remaining.min(50 * 1024 * 1024);
        let search_end = start + max_size;

        if magic.has_end_marker {
            match magic.ext {
                "jpg" => {
                    for i in start + magic.magic.len()..search_end.saturating_sub(1) {
                        if data[i] == 0xFF && data[i + 1] == 0xD9 {
                            return i + 2;
                        }
                    }
                }
                "png" => {
                    let iend = b"IEND";
                    for i in start + magic.magic.len()..search_end.saturating_sub(7) {
                        if i + 4 <= data.len() && &data[i..i + 4] == iend {
                            return (i + 8).min(data.len());
                        }
                    }
                }
                "gif" => {
                    for i in start + magic.magic.len()..search_end {
                        if data[i] == 0x3B {
                            return i + 1;
                        }
                    }
                }
                "zip" => {
                    let eocd = &[0x50, 0x4B, 0x05, 0x06];
                    for i in (start + magic.magic.len()..search_end.saturating_sub(21)).rev() {
                        if i + 4 <= data.len() && &data[i..i + 4] == eocd {
                            return (i + 22).min(data.len());
                        }
                    }
                }
                "pdf" => {
                    let eof = b"%%EOF";
                    for i in (start + magic.magic.len()..search_end.saturating_sub(4)).rev() {
                        if i + 5 <= data.len() && &data[i..i + 5] == eof {
                            return (i + 5).min(data.len());
                        }
                    }
                }
                "pem" => {
                    let end_markers = [b"-----END ".as_slice()];
                    for marker in &end_markers {
                        for i in start + magic.magic.len()..search_end.saturating_sub(marker.len())
                        {
                            if i + marker.len() <= data.len()
                                && &data[i..i + marker.len()] == *marker
                            {
                                // Find the closing -----
                                for j in i + marker.len()..search_end.min(i + 50) {
                                    if j + 5 <= data.len() && &data[j..j + 5] == b"-----" {
                                        return (j + 5).min(data.len());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // For files without end markers, try to estimate size from header or use reasonable default
        if magic.ext == "bmp" && data.len() >= start + 6 {
            let size = u32::from_le_bytes([
                data[start + 2],
                data[start + 3],
                data[start + 4],
                data[start + 5],
            ]) as usize;
            if size > 0 && size < max_size {
                return (start + size).min(data.len());
            }
        }

        // Default: reasonable chunk
        (start + max_size.min(1024 * 512)).min(data.len())
    }

    fn create_extracted_file(
        data: &[u8],
        content_type: &str,
        filename: &str,
        src: &str,
        dst: &str,
        packet_ids: &[u64],
        stream_key: &str,
        source_type: &str,
    ) -> Option<ExtractedFile> {
        if data.len() < 20 {
            return None;
        }

        // Try to detect actual type from magic
        let (detected_ext, detected_mime) = Self::detect_type_from_magic(data);

        let final_mime = if content_type.is_empty() || content_type.contains("octet-stream") {
            detected_mime.to_string()
        } else {
            content_type.to_string()
        };

        // Skip plain HTML if it's really HTML
        if final_mime.contains("text/html") && data.len() < 500 {
            return None;
        }

        let final_filename = if filename.is_empty() {
            Self::generate_filename_with_ext(detected_ext)
        } else {
            filename.to_string()
        };

        Some(ExtractedFile {
            id: String::new(),
            filename: final_filename,
            content_type: final_mime,
            size: data.len(),
            src: src.to_string(),
            dst: dst.to_string(),
            data: data.to_vec(),
            packet_ids: packet_ids.to_vec(),
            stream_key: stream_key.to_string(),
            source_type: source_type.to_string(),
        })
    }

    #[allow(dead_code)]
    fn generate_filename_from_type(content_type: &str) -> String {
        let ext = match content_type.split(';').next().unwrap_or("").trim() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "application/pdf" => "pdf",
            "application/zip" => "zip",
            "application/x-gzip" | "application/gzip" => "gz",
            "application/x-tar" => "tar",
            "application/x-rar-compressed" => "rar",
            "application/x-7z-compressed" => "7z",
            "application/javascript" | "text/javascript" => "js",
            "text/css" => "css",
            "application/json" => "json",
            "application/xml" | "text/xml" => "xml",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "audio/mpeg" => "mp3",
            "audio/wav" => "wav",
            "application/octet-stream" => "bin",
            _ => "bin",
        };
        Self::generate_filename_with_ext(ext)
    }

    fn generate_filename_with_ext(ext: &str) -> String {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let rand_suffix: u32 = rand::random::<u32>() % 10000;
        format!("file_{}_{}.{}", timestamp, rand_suffix, ext)
    }

    fn deduplicate_files(files: Vec<ExtractedFile>) -> Vec<ExtractedFile> {
        let mut seen_hashes = std::collections::HashSet::new();
        let mut result = Vec::new();

        for file in files {
            // Simple hash: first 100 bytes + size
            let hash_data: Vec<u8> = file.data.iter().take(100).copied().collect();
            let hash_key = format!("{:?}_{}", hash_data, file.size);

            if seen_hashes.insert(hash_key) {
                result.push(file);
            }
        }
        result
    }

    /// Get packets related to a specific file
    pub fn get_file_packets<'a>(
        file: &ExtractedFile,
        packets: &'a [CapturedPacket],
    ) -> Vec<&'a CapturedPacket> {
        packets
            .iter()
            .filter(|p| file.packet_ids.contains(&p.id))
            .collect()
    }

    /// Get all packets in the same stream as a file
    pub fn get_stream_packets<'a>(
        file: &ExtractedFile,
        packets: &'a [CapturedPacket],
    ) -> Vec<&'a CapturedPacket> {
        packets
            .iter()
            .filter(|p| {
                let key = Self::stream_key(&p.src, &p.dst);
                key == file.stream_key
            })
            .collect()
    }

    /// Extract files and save to directory
    pub fn extract_and_save(
        packets: &[CapturedPacket],
        output_dir: &Path,
    ) -> Result<Vec<ExtractedFile>, String> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        let files = Self::extract_files(packets);

        for file in &files {
            let file_path = output_dir.join(&file.filename);
            std::fs::write(&file_path, &file.data)
                .map_err(|e| format!("Failed to write file {}: {}", file.filename, e))?;
        }

        Ok(files)
    }

    /// Save a single file to path
    pub fn save_file(file: &ExtractedFile, path: &Path) -> Result<(), String> {
        std::fs::write(path, &file.data).map_err(|e| format!("Failed to write file: {}", e))
    }
}
