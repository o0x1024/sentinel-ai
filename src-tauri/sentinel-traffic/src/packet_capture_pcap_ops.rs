use crate::packet_capture::{CapturedPacket, PacketCaptureService};
use pcap_file::pcap::{PcapHeader, PcapPacket, PcapReader, PcapWriter};
use pcap_file::pcapng::blocks::enhanced_packet::EnhancedPacketBlock;
use pcap_file::pcapng::blocks::interface_description::InterfaceDescriptionBlock;
use pcap_file::pcapng::{Block, PcapNgBlock, PcapNgReader, PcapNgWriter};
use pcap_file::DataLink;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;
use std::time::Duration;
use tracing::{info, warn};

/// PCAP file operations
pub struct PcapFileOps;

impl PcapFileOps {
    /// Read packets from pcap/pcapng file
    pub fn read_pcap_file(path: &Path) -> Result<Vec<CapturedPacket>, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut reader = BufReader::new(file);

        // Read magic number to detect format
        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|e| format!("Failed to read magic: {}", e))?;

        // Reset reader
        drop(reader);
        let file = File::open(path).map_err(|e| format!("Failed to reopen file: {}", e))?;
        let reader = BufReader::new(file);

        // Check format
        match &magic {
            [0xd4, 0xc3, 0xb2, 0xa1] | [0xa1, 0xb2, 0xc3, 0xd4] => Self::read_pcap(reader),
            [0x0a, 0x0d, 0x0d, 0x0a] => Self::read_pcapng(reader),
            _ => Err("Unknown file format, expected pcap or pcapng".to_string()),
        }
    }

    /// Read classic pcap format
    fn read_pcap<R: Read>(reader: BufReader<R>) -> Result<Vec<CapturedPacket>, String> {
        let mut pcap_reader =
            PcapReader::new(reader).map_err(|e| format!("Failed to create pcap reader: {}", e))?;
        let mut packets = Vec::new();
        let mut id: u64 = 0;

        while let Some(pkt) = pcap_reader.next_packet() {
            match pkt {
                Ok(packet) => {
                    id += 1;
                    let ts_ms = packet.timestamp.as_secs() as i64 * 1000
                        + packet.timestamp.subsec_nanos() as i64 / 1_000_000;
                    if let Some(mut captured) =
                        PacketCaptureService::parse_packet(id, &packet.data, "pcap")
                    {
                        captured.timestamp = ts_ms;
                        packets.push(captured);
                    }
                }
                Err(e) => {
                    warn!("Error reading packet: {}", e);
                    break;
                }
            }
        }

        info!("Read {} packets from pcap file", packets.len());
        Ok(packets)
    }

    /// Read pcapng format
    fn read_pcapng<R: Read>(reader: BufReader<R>) -> Result<Vec<CapturedPacket>, String> {
        let mut pcapng_reader = PcapNgReader::new(reader)
            .map_err(|e| format!("Failed to create pcapng reader: {}", e))?;
        let mut packets = Vec::new();
        let mut id: u64 = 0;

        while let Some(block) = pcapng_reader.next_block() {
            match block {
                Ok(block) => {
                    if let Block::EnhancedPacket(epb) = block {
                        id += 1;
                        let ts = epb.timestamp;
                        let ts_ms = (ts.as_secs() * 1000 + ts.subsec_millis() as u64) as i64;
                        if let Some(mut captured) =
                            PacketCaptureService::parse_packet(id, &epb.data, "pcapng")
                        {
                            captured.timestamp = ts_ms;
                            packets.push(captured);
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading block: {}", e);
                    break;
                }
            }
        }

        info!("Read {} packets from pcapng file", packets.len());
        Ok(packets)
    }

    /// Write packets to pcap file
    pub fn write_pcap_file(path: &Path, packets: &[CapturedPacket]) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        let writer = BufWriter::new(file);

        let header = PcapHeader {
            datalink: DataLink::ETHERNET,
            ..Default::default()
        };

        let mut pcap_writer = PcapWriter::with_header(writer, header)
            .map_err(|e| format!("Failed to create pcap writer: {}", e))?;

        for pkt in packets {
            let ts_secs = (pkt.timestamp / 1000) as u32;
            let ts_usecs = ((pkt.timestamp % 1000) * 1000) as u32;
            let ts = std::time::Duration::new(ts_secs as u64, ts_usecs * 1000);

            let pcap_packet = PcapPacket::new(ts, pkt.raw.len() as u32, &pkt.raw);
            pcap_writer
                .write_packet(&pcap_packet)
                .map_err(|e| format!("Failed to write packet: {}", e))?;
        }

        info!("Wrote {} packets to pcap file", packets.len());
        Ok(())
    }

    /// Write packets to pcapng file
    pub fn write_pcapng_file(path: &Path, packets: &[CapturedPacket]) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        let writer = BufWriter::new(file);

        let mut pcapng_writer = PcapNgWriter::new(writer)
            .map_err(|e| format!("Failed to create pcapng writer: {}", e))?;

        // Write interface description block
        let idb = InterfaceDescriptionBlock {
            linktype: DataLink::ETHERNET,
            snaplen: 65535,
            options: vec![],
        };
        pcapng_writer
            .write_block(&idb.clone().into_block())
            .map_err(|e| format!("Failed to write interface block: {}", e))?;

        // Write packets
        for pkt in packets {
            let ts_secs = (pkt.timestamp / 1000) as u64;
            let ts_nanos = ((pkt.timestamp % 1000) * 1_000_000) as u32;
            let ts = Duration::new(ts_secs, ts_nanos);
            let epb = EnhancedPacketBlock {
                interface_id: 0,
                timestamp: ts,
                original_len: pkt.raw.len() as u32,
                data: pkt.raw.clone().into(),
                options: vec![],
            };
            pcapng_writer
                .write_block(&epb.into_block())
                .map_err(|e| format!("Failed to write packet block: {}", e))?;
        }

        info!("Wrote {} packets to pcapng file", packets.len());
        Ok(())
    }
}
