//! Packet capture module - provides network packet capture functionality
//!
//! Similar to Wireshark, captures raw network packets from network interfaces
//!
//! Windows Support: Requires Npcap to be installed (https://nmap.org/npcap/)
//! - Download and install Npcap in WinPcap API-compatible mode
//! - Packet.lib should be automatically found by pnet crate

use pcap_file::pcap::{PcapHeader, PcapPacket, PcapReader, PcapWriter};
use pcap_file::pcapng::{Block, PcapNgBlock, PcapNgReader, PcapNgWriter};
use pcap_file::pcapng::blocks::enhanced_packet::EnhancedPacketBlock;
use pcap_file::pcapng::blocks::interface_description::InterfaceDescriptionBlock;
use pcap_file::DataLink;
use pnet::datalink::{self, Channel::Ethernet, NetworkInterface};
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmpv6::Icmpv6Packet;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub description: Option<String>,
    pub mac: Option<String>,
    pub ipv4: Option<String>,
}

/// Protocol field with optional children for tree display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolField {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ProtocolField>>,
}

impl ProtocolField {
    fn new(name: &str, value: &str) -> Self {
        Self { name: name.to_string(), value: value.to_string(), children: None }
    }
    
    fn with_children(name: &str, value: &str, children: Vec<ProtocolField>) -> Self {
        Self { name: name.to_string(), value: value.to_string(), children: Some(children) }
    }
}

/// Protocol layer with field details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLayer {
    pub name: String,
    pub display: String,
    pub fields: Vec<ProtocolField>,
}

/// Captured packet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacket {
    pub id: u64,
    pub timestamp: i64,
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub length: usize,
    pub info: String,
    pub layers: Vec<ProtocolLayer>,
    pub raw: Vec<u8>,
}

/// Packet capture service
pub struct PacketCaptureService {
    running: Arc<AtomicBool>,
    packet_tx: Option<mpsc::Sender<CapturedPacket>>,
}

impl PacketCaptureService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            packet_tx: None,
        }
    }

    /// Get all available network interfaces
    pub fn get_interfaces() -> Vec<InterfaceInfo> {
        let all_interfaces = datalink::interfaces();
        debug!("Found {} total interfaces", all_interfaces.len());
        
        #[cfg(target_os = "windows")]
        if all_interfaces.is_empty() {
            warn!("No network interfaces found. On Windows, this usually means Npcap is not installed or not running in WinPcap-compatible mode.");
            warn!("Please install Npcap from https://nmap.org/npcap/ and enable 'WinPcap API-compatible Mode'");
        }
        
        all_interfaces
            .into_iter()
            .filter(|iface| {
                if iface.is_loopback() { return false; }
                let has_ip = !iface.ips.is_empty();
                let has_mac = iface.mac.is_some();
                has_ip || has_mac
            })
            .map(|iface| {
                let mac = iface.mac.map(|m| format!("{}", m));
                let ipv4 = iface.ips.iter()
                    .find(|ip| ip.is_ipv4())
                    .map(|ip| ip.ip().to_string());
                
                InterfaceInfo {
                    name: iface.name.clone(),
                    description: Some(iface.description.clone()),
                    mac,
                    ipv4,
                }
            })
            .collect()
    }

    /// Start packet capture on specified interface
    pub fn start_capture(
        &mut self,
        interface_name: &str,
    ) -> Result<mpsc::Receiver<CapturedPacket>, String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Capture already running".to_string());
        }

        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .find(|iface| iface.name == interface_name)
            .ok_or_else(|| format!("Interface {} not found", interface_name))?;

        info!("Starting packet capture on interface: {}", interface_name);

        let (tx, rx) = mpsc::channel::<CapturedPacket>(1000);
        self.packet_tx = Some(tx.clone());
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let iface_name = interface_name.to_string();
        
        std::thread::spawn(move || {
            Self::capture_loop(interface, iface_name, tx, running);
        });

        Ok(rx)
    }

    /// Stop packet capture
    pub fn stop_capture(&mut self) {
        info!("Stopping packet capture");
        self.running.store(false, Ordering::SeqCst);
        self.packet_tx = None;
    }

    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Main capture loop
    fn capture_loop(
        interface: NetworkInterface,
        iface_name: String,
        tx: mpsc::Sender<CapturedPacket>,
        running: Arc<AtomicBool>,
    ) {
        let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => { 
                error!("Unsupported channel type"); 
                return; 
            }
            Err(e) => { 
                error!("Failed to create datalink channel: {}", e);
                #[cfg(target_os = "windows")]
                error!("On Windows, ensure Npcap is installed with 'WinPcap API-compatible Mode' enabled");
                #[cfg(target_os = "windows")]
                error!("Also ensure you have Administrator privileges to capture packets");
                return; 
            }
        };

        let mut packet_id: u64 = 0;

        while running.load(Ordering::SeqCst) {
            match rx.next() {
                Ok(packet) => {
                    packet_id += 1;
                    if let Some(captured) = Self::parse_packet(packet_id, packet, &iface_name) {
                        if tx.blocking_send(captured).is_err() {
                            debug!("Channel closed, stopping capture");
                            break;
                        }
                    }
                }
                Err(e) => {
                    if running.load(Ordering::SeqCst) {
                        warn!("Error receiving packet: {}", e);
                    }
                }
            }
        }
        info!("Capture loop ended");
    }

    /// Parse raw packet data
    fn parse_packet(id: u64, data: &[u8], iface_name: &str) -> Option<CapturedPacket> {
        let ethernet = EthernetPacket::new(data)?;
        let timestamp = chrono::Utc::now().timestamp_millis();
        let mut layers = Vec::new();
        
        // Frame layer
        let frame_display = format!("Frame {}: {} bytes on wire, {} bytes captured on interface {}", 
            id, data.len() * 8, data.len(), iface_name);
        layers.push(ProtocolLayer {
            name: "Frame".to_string(),
            display: frame_display,
            fields: vec![
                ProtocolField::new("Interface", iface_name),
                ProtocolField::new("Encapsulation type", "Ethernet (1)"),
                ProtocolField::new("Frame Number", &id.to_string()),
                ProtocolField::new("Frame Length", &format!("{} bytes ({} bits)", data.len(), data.len() * 8)),
                ProtocolField::new("Capture Length", &format!("{} bytes", data.len())),
            ],
        });
        
        // Ethernet layer
        let src_mac = ethernet.get_source().to_string();
        let dst_mac = ethernet.get_destination().to_string();
        let src_vendor = get_mac_vendor(&src_mac);
        let dst_vendor = get_mac_vendor(&dst_mac);
        let ether_type = ethernet.get_ethertype();
        let ether_type_val: u16 = ether_type.0;
        
        let eth_display = format!("Ethernet II, Src: {} ({}), Dst: {} ({})", 
            src_vendor, src_mac, dst_vendor, dst_mac);
        layers.push(ProtocolLayer {
            name: "Ethernet".to_string(),
            display: eth_display,
            fields: vec![
                ProtocolField::new("Destination", &format!("{} ({})", dst_vendor, dst_mac)),
                ProtocolField::new("Source", &format!("{} ({})", src_vendor, src_mac)),
                ProtocolField::new("Type", &format!("{:?} (0x{:04x})", ether_type, ether_type_val)),
            ],
        });

        let (src, dst, protocol, info) = match ether_type {
            EtherTypes::Ipv4 => Self::parse_ipv4(ethernet.payload(), &mut layers),
            EtherTypes::Ipv6 => Self::parse_ipv6(ethernet.payload(), &mut layers),
            EtherTypes::Arp => Self::parse_arp(ethernet.payload(), &mut layers),
            _ => (src_mac.clone(), dst_mac.clone(), format!("{:?}", ether_type), "Unknown EtherType".to_string()),
        };

        Some(CapturedPacket { id, timestamp, src, dst, protocol, length: data.len(), info, layers, raw: data.to_vec() })
    }

    /// Parse IPv4 packet
    fn parse_ipv4(data: &[u8], layers: &mut Vec<ProtocolLayer>) -> (String, String, String, String) {
        if let Some(ipv4) = Ipv4Packet::new(data) {
            let src = ipv4.get_source().to_string();
            let dst = ipv4.get_destination().to_string();
            let version = ipv4.get_version();
            let ihl = ipv4.get_header_length();
            let dscp = ipv4.get_dscp();
            let ecn = ipv4.get_ecn();
            let total_len = ipv4.get_total_length();
            let ident = ipv4.get_identification();
            let flags = ipv4.get_flags();
            let frag_off = ipv4.get_fragment_offset();
            let ttl = ipv4.get_ttl();
            let next_proto = ipv4.get_next_level_protocol();
            let checksum = ipv4.get_checksum();
            
            let ip_display = format!("Internet Protocol Version 4, Src: {}, Dst: {}", src, dst);
            
            // Build flags children
            let flags_val = flags;
            let df = (flags_val & 0b010) != 0;
            let mf = (flags_val & 0b001) != 0;
            let flags_children = vec![
                ProtocolField::new(&format!("{}.. .... = Reserved bit", if flags_val & 0b100 != 0 { "1" } else { "0" }), "Not set"),
                ProtocolField::new(&format!(".{}. .... = Don't fragment", if df { "1" } else { "0" }), if df { "Set" } else { "Not set" }),
                ProtocolField::new(&format!("..{} .... = More fragments", if mf { "1" } else { "0" }), if mf { "Set" } else { "Not set" }),
            ];
            
            layers.push(ProtocolLayer {
                name: "IPv4".to_string(),
                display: ip_display,
                fields: vec![
                    ProtocolField::new(&format!("{:04b} .... = Version", version), &version.to_string()),
                    ProtocolField::new(&format!(".... {:04b} = Header Length", ihl), &format!("{} bytes ({})", ihl * 4, ihl)),
                    ProtocolField::with_children(
                        "Differentiated Services Field",
                        &format!("0x{:02x} (DSCP: 0x{:02x}, ECN: 0x{:02x})", (dscp << 2) | ecn, dscp, ecn),
                        vec![
                            ProtocolField::new(&format!("{:06b}.. = DSCP", dscp), &get_dscp_name(dscp)),
                            ProtocolField::new(&format!("......{:02b} = ECN", ecn), &get_ecn_name(ecn)),
                        ]
                    ),
                    ProtocolField::new("Total Length", &total_len.to_string()),
                    ProtocolField::new("Identification", &format!("0x{:04x} ({})", ident, ident)),
                    ProtocolField::with_children(
                        &format!("Flags: 0x{:02x}", flags_val),
                        &format!("{}{}", if df { ", Don't fragment" } else { "" }, if mf { ", More fragments" } else { "" }),
                        flags_children
                    ),
                    ProtocolField::new("Fragment Offset", &frag_off.to_string()),
                    ProtocolField::new("Time to Live", &ttl.to_string()),
                    ProtocolField::new("Protocol", &format!("{:?} ({})", next_proto, next_proto.0)),
                    ProtocolField::new("Header Checksum", &format!("0x{:04x}", checksum)),
                    ProtocolField::new("Source Address", &src),
                    ProtocolField::new("Destination Address", &dst),
                ],
            });

            match next_proto {
                IpNextHeaderProtocols::Tcp => Self::parse_tcp(ipv4.payload(), layers, &src, &dst),
                IpNextHeaderProtocols::Udp => Self::parse_udp(ipv4.payload(), layers, &src, &dst),
                IpNextHeaderProtocols::Icmp => Self::parse_icmp(ipv4.payload(), layers, &src, &dst),
                proto => {
                    let proto_name = get_ip_protocol_name(proto.0);
                    (src, dst, proto_name.clone(), format!("IP Protocol: {}", proto_name))
                }
            }
        } else {
            ("".to_string(), "".to_string(), "IPv4".to_string(), "Malformed IPv4".to_string())
        }
    }

    /// Parse IPv6 packet
    fn parse_ipv6(data: &[u8], layers: &mut Vec<ProtocolLayer>) -> (String, String, String, String) {
        if let Some(ipv6) = Ipv6Packet::new(data) {
            let src = ipv6.get_source().to_string();
            let dst = ipv6.get_destination().to_string();
            let hop = ipv6.get_hop_limit();
            let next_header = ipv6.get_next_header();
            let payload_len = ipv6.get_payload_length();
            
            let ip_display = format!("Internet Protocol Version 6, Src: {}, Dst: {}", src, dst);
            layers.push(ProtocolLayer {
                name: "IPv6".to_string(),
                display: ip_display,
                fields: vec![
                    ProtocolField::new("Version", "6"),
                    ProtocolField::new("Payload Length", &payload_len.to_string()),
                    ProtocolField::new("Next Header", &format!("{:?} ({})", next_header, next_header.0)),
                    ProtocolField::new("Hop Limit", &hop.to_string()),
                    ProtocolField::new("Source Address", &src),
                    ProtocolField::new("Destination Address", &dst),
                ],
            });

            match next_header {
                IpNextHeaderProtocols::Tcp => Self::parse_tcp(ipv6.payload(), layers, &src, &dst),
                IpNextHeaderProtocols::Udp => Self::parse_udp(ipv6.payload(), layers, &src, &dst),
                IpNextHeaderProtocols::Icmpv6 => Self::parse_icmpv6(ipv6.payload(), layers, &src, &dst),
                proto => {
                    let proto_name = get_ip_protocol_name(proto.0);
                    (src, dst, proto_name.clone(), format!("IPv6 Next Header: {}", proto_name))
                }
            }
        } else {
            ("".to_string(), "".to_string(), "IPv6".to_string(), "Malformed IPv6".to_string())
        }
    }

    /// Parse TCP packet
    fn parse_tcp(data: &[u8], layers: &mut Vec<ProtocolLayer>, src_ip: &str, dst_ip: &str) -> (String, String, String, String) {
        if let Some(tcp) = TcpPacket::new(data) {
            let src_port = tcp.get_source();
            let dst_port = tcp.get_destination();
            let seq = tcp.get_sequence();
            let ack = tcp.get_acknowledgement();
            let data_offset = tcp.get_data_offset();
            let flags = tcp.get_flags();
            let win = tcp.get_window();
            let checksum = tcp.get_checksum();
            let urgent = tcp.get_urgent_ptr();
            let payload = tcp.payload();
            let payload_len = payload.len();
            
            let flags_str = format_tcp_flags_short(flags as u16);
            let tcp_display = format!("Transmission Control Protocol, Src Port: {}, Dst Port: {}, Seq: {}, Ack: {}, Len: {}", 
                src_port, dst_port, seq, ack, payload_len);
            
            // Build flags children - detailed bit display (8-bit TCP flags)
            let cwr = flags & 0x80 != 0;
            let ece = flags & 0x40 != 0;
            let urg = flags & 0x20 != 0;
            let ack_flag = flags & 0x10 != 0;
            let psh = flags & 0x08 != 0;
            let rst = flags & 0x04 != 0;
            let syn = flags & 0x02 != 0;
            let fin = flags & 0x01 != 0;
            
            let flags_children = vec![
                ProtocolField::new(&format!("{}....... = Congestion Window Reduced (CWR)", if cwr { "1" } else { "0" }), if cwr { "Set" } else { "Not set" }),
                ProtocolField::new(&format!(".{}..... = ECN-Echo", if ece { "1" } else { "0" }), if ece { "Set" } else { "Not set" }),
                ProtocolField::new(&format!("..{}.... = Urgent", if urg { "1" } else { "0" }), if urg { "Set" } else { "Not set" }),
                ProtocolField::new(&format!("...{}... = Acknowledgment", if ack_flag { "1" } else { "0" }), if ack_flag { "Set" } else { "Not set" }),
                ProtocolField::new(&format!("....{}.. = Push", if psh { "1" } else { "0" }), if psh { "Set" } else { "Not set" }),
                ProtocolField::new(&format!(".....{}. = Reset", if rst { "1" } else { "0" }), if rst { "Set" } else { "Not set" }),
                ProtocolField::new(&format!("......{} = Syn", if syn { "1" } else { "0" }), if syn { "Set" } else { "Not set" }),
                ProtocolField::new(&format!(".......{} = Fin", if fin { "1" } else { "0" }), if fin { "Set" } else { "Not set" }),
            ];
            
            layers.push(ProtocolLayer {
                name: "TCP".to_string(),
                display: tcp_display,
                fields: vec![
                    ProtocolField::new("Source Port", &src_port.to_string()),
                    ProtocolField::new("Destination Port", &dst_port.to_string()),
                    ProtocolField::new("Sequence Number", &seq.to_string()),
                    ProtocolField::new("Acknowledgment Number", &ack.to_string()),
                    ProtocolField::new(&format!("{:04b} .... = Header Length", data_offset), &format!("{} bytes ({})", data_offset * 4, data_offset)),
                    ProtocolField::with_children(
                        &format!("Flags: 0x{:03x} ({})", flags, flags_str),
                        &flags_str,
                        flags_children
                    ),
                    ProtocolField::new("Window", &win.to_string()),
                    ProtocolField::new("Checksum", &format!("0x{:04x}", checksum)),
                    ProtocolField::new("Urgent Pointer", &urgent.to_string()),
                    ProtocolField::new("TCP Segment Len", &payload_len.to_string()),
                ],
            });

            // Try to parse HTTP content
            let (protocol, info) = if payload_len > 0 {
                if let Some((http_proto, http_info, http_layer)) = parse_http_content(payload) {
                    layers.push(http_layer);
                    (http_proto, http_info)
                } else {
                    let protocol = detect_app_protocol_tcp(src_port, dst_port);
                    let info = format!("{} → {} [{}] Seq={} Ack={} Win={} Len={}", 
                        src_port, dst_port, flags_str, seq, ack, win, payload_len);
                    (protocol, info)
                }
            } else {
                let protocol = detect_app_protocol_tcp(src_port, dst_port);
                let info = format!("{} → {} [{}] Seq={} Ack={} Win={} Len={}", 
                    src_port, dst_port, flags_str, seq, ack, win, payload_len);
                (protocol, info)
            };

            (format!("{}:{}", src_ip, src_port), format!("{}:{}", dst_ip, dst_port), protocol, info)
        } else {
            (src_ip.to_string(), dst_ip.to_string(), "TCP".to_string(), "Malformed TCP".to_string())
        }
    }

    /// Parse UDP packet
    fn parse_udp(data: &[u8], layers: &mut Vec<ProtocolLayer>, src_ip: &str, dst_ip: &str) -> (String, String, String, String) {
        if let Some(udp) = UdpPacket::new(data) {
            let src_port = udp.get_source();
            let dst_port = udp.get_destination();
            let length = udp.get_length();
            let checksum = udp.get_checksum();
            let payload = udp.payload();
            
            let udp_display = format!("User Datagram Protocol, Src Port: {}, Dst Port: {}", src_port, dst_port);
            layers.push(ProtocolLayer {
                name: "UDP".to_string(),
                display: udp_display,
                fields: vec![
                    ProtocolField::new("Source Port", &src_port.to_string()),
                    ProtocolField::new("Destination Port", &dst_port.to_string()),
                    ProtocolField::new("Length", &length.to_string()),
                    ProtocolField::new("Checksum", &format!("0x{:04x}", checksum)),
                    ProtocolField::new("UDP payload", &format!("{} bytes", payload.len())),
                ],
            });

            // Try to parse DNS if port 53
            let (protocol, info) = if src_port == 53 || dst_port == 53 {
                if let Some((dns_info, dns_layer)) = parse_dns_content(payload, src_port == 53) {
                    layers.push(dns_layer);
                    ("DNS".to_string(), dns_info)
                } else {
                    ("DNS".to_string(), format!("{} → {} Len={}", src_port, dst_port, length))
                }
            } else {
                let protocol = detect_app_protocol_udp(src_port, dst_port);
                let info = format!("{} → {} Len={}", src_port, dst_port, length);
                (protocol, info)
            };

            (format!("{}:{}", src_ip, src_port), format!("{}:{}", dst_ip, dst_port), protocol, info)
        } else {
            (src_ip.to_string(), dst_ip.to_string(), "UDP".to_string(), "Malformed UDP".to_string())
        }
    }

    /// Parse ICMP packet
    fn parse_icmp(data: &[u8], layers: &mut Vec<ProtocolLayer>, src_ip: &str, dst_ip: &str) -> (String, String, String, String) {
        if let Some(icmp) = IcmpPacket::new(data) {
            let icmp_type = icmp.get_icmp_type();
            let icmp_code = icmp.get_icmp_code();
            let checksum = icmp.get_checksum();
            
            let type_name = get_icmp_type_name(icmp_type.0);
            let icmp_display = format!("Internet Control Message Protocol ({})", type_name);
            
            layers.push(ProtocolLayer {
                name: "ICMP".to_string(),
                display: icmp_display,
                fields: vec![
                    ProtocolField::new("Type", &format!("{} ({})", icmp_type.0, type_name)),
                    ProtocolField::new("Code", &icmp_code.0.to_string()),
                    ProtocolField::new("Checksum", &format!("0x{:04x}", checksum)),
                ],
            });

            let info = format!("{} (type={}, code={})", type_name, icmp_type.0, icmp_code.0);
            (src_ip.to_string(), dst_ip.to_string(), "ICMP".to_string(), info)
        } else {
            (src_ip.to_string(), dst_ip.to_string(), "ICMP".to_string(), "Malformed ICMP".to_string())
        }
    }

    /// Parse ICMPv6 packet
    fn parse_icmpv6(data: &[u8], layers: &mut Vec<ProtocolLayer>, src_ip: &str, dst_ip: &str) -> (String, String, String, String) {
        if let Some(icmpv6) = Icmpv6Packet::new(data) {
            let icmpv6_type = icmpv6.get_icmpv6_type();
            let icmpv6_code = icmpv6.get_icmpv6_code();
            let checksum = icmpv6.get_checksum();
            
            let type_name = get_icmpv6_type_name(icmpv6_type.0);
            let icmpv6_display = format!("Internet Control Message Protocol v6 ({})", type_name);
            
            layers.push(ProtocolLayer {
                name: "ICMPv6".to_string(),
                display: icmpv6_display,
                fields: vec![
                    ProtocolField::new("Type", &format!("{} ({})", icmpv6_type.0, type_name)),
                    ProtocolField::new("Code", &icmpv6_code.0.to_string()),
                    ProtocolField::new("Checksum", &format!("0x{:04x}", checksum)),
                ],
            });

            let info = format!("{} (type={}, code={})", type_name, icmpv6_type.0, icmpv6_code.0);
            (src_ip.to_string(), dst_ip.to_string(), "ICMPv6".to_string(), info)
        } else {
            (src_ip.to_string(), dst_ip.to_string(), "ICMPv6".to_string(), "Malformed ICMPv6".to_string())
        }
    }

    /// Parse ARP packet
    fn parse_arp(data: &[u8], layers: &mut Vec<ProtocolLayer>) -> (String, String, String, String) {
        if let Some(arp) = ArpPacket::new(data) {
            let op = arp.get_operation();
            let sender_mac = arp.get_sender_hw_addr().to_string();
            let sender_ip = arp.get_sender_proto_addr().to_string();
            let target_mac = arp.get_target_hw_addr().to_string();
            let target_ip = arp.get_target_proto_addr().to_string();
            
            let op_val: u16 = op.0;
            let op_name = match op_val { 1 => "request", 2 => "reply", _ => "unknown" };
            let arp_display = format!("Address Resolution Protocol ({})", op_name);
            
            layers.push(ProtocolLayer {
                name: "ARP".to_string(),
                display: arp_display,
                fields: vec![
                    ProtocolField::new("Hardware type", "Ethernet (1)"),
                    ProtocolField::new("Protocol type", "IPv4 (0x0800)"),
                    ProtocolField::new("Hardware size", "6"),
                    ProtocolField::new("Protocol size", "4"),
                    ProtocolField::new("Opcode", &format!("{} ({})", op_name, op_val)),
                    ProtocolField::new("Sender MAC address", &sender_mac),
                    ProtocolField::new("Sender IP address", &sender_ip),
                    ProtocolField::new("Target MAC address", &target_mac),
                    ProtocolField::new("Target IP address", &target_ip),
                ],
            });

            let info = if op_name == "request" {
                format!("Who has {}? Tell {}", target_ip, sender_ip)
            } else {
                format!("{} is at {}", sender_ip, sender_mac)
            };
            
            (sender_ip, target_ip, "ARP".to_string(), info)
        } else {
            ("".to_string(), "".to_string(), "ARP".to_string(), "Malformed ARP".to_string())
        }
    }
}

impl Default for PacketCaptureService {
    fn default() -> Self { Self::new() }
}

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

/// PCAP file operations
pub struct PcapFileOps;

impl PcapFileOps {
    /// Read packets from pcap/pcapng file
    pub fn read_pcap_file(path: &Path) -> Result<Vec<CapturedPacket>, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut reader = BufReader::new(file);
        
        // Read magic number to detect format
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).map_err(|e| format!("Failed to read magic: {}", e))?;
        
        // Reset reader
        drop(reader);
        let file = File::open(path).map_err(|e| format!("Failed to reopen file: {}", e))?;
        let reader = BufReader::new(file);
        
        // Check format
        match &magic {
            [0xd4, 0xc3, 0xb2, 0xa1] | [0xa1, 0xb2, 0xc3, 0xd4] => {
                Self::read_pcap(reader)
            }
            [0x0a, 0x0d, 0x0d, 0x0a] => {
                Self::read_pcapng(reader)
            }
            _ => Err("Unknown file format, expected pcap or pcapng".to_string())
        }
    }

    /// Read classic pcap format
    fn read_pcap<R: Read>(reader: BufReader<R>) -> Result<Vec<CapturedPacket>, String> {
        let mut pcap_reader = PcapReader::new(reader).map_err(|e| format!("Failed to create pcap reader: {}", e))?;
        let mut packets = Vec::new();
        let mut id: u64 = 0;

        while let Some(pkt) = pcap_reader.next_packet() {
            match pkt {
                Ok(packet) => {
                    id += 1;
                    let ts_ms = packet.timestamp.as_secs() as i64 * 1000 
                        + packet.timestamp.subsec_nanos() as i64 / 1_000_000;
                    if let Some(mut captured) = PacketCaptureService::parse_packet(id, &packet.data, "pcap") {
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
        let mut pcapng_reader = PcapNgReader::new(reader).map_err(|e| format!("Failed to create pcapng reader: {}", e))?;
        let mut packets = Vec::new();
        let mut id: u64 = 0;

        while let Some(block) = pcapng_reader.next_block() {
            match block {
                Ok(block) => {
                    if let Block::EnhancedPacket(epb) = block {
                        id += 1;
                        let ts = epb.timestamp;
                        let ts_ms = (ts.as_secs() * 1000 + ts.subsec_millis() as u64) as i64;
                        if let Some(mut captured) = PacketCaptureService::parse_packet(id, &epb.data, "pcapng") {
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
            pcap_writer.write_packet(&pcap_packet)
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
        pcapng_writer.write_block(&idb.clone().into_block())
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
            pcapng_writer.write_block(&epb.into_block())
                .map_err(|e| format!("Failed to write packet block: {}", e))?;
        }

        info!("Wrote {} packets to pcapng file", packets.len());
        Ok(())
    }
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
    FileMagic { magic: &[0xFF, 0xD8, 0xFF], ext: "jpg", mime: "image/jpeg", has_end_marker: true },
    FileMagic { magic: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], ext: "png", mime: "image/png", has_end_marker: true },
    FileMagic { magic: b"GIF87a", ext: "gif", mime: "image/gif", has_end_marker: true },
    FileMagic { magic: b"GIF89a", ext: "gif", mime: "image/gif", has_end_marker: true },
    FileMagic { magic: b"RIFF", ext: "webp", mime: "image/webp", has_end_marker: false },
    FileMagic { magic: &[0x00, 0x00, 0x01, 0x00], ext: "ico", mime: "image/x-icon", has_end_marker: false },
    FileMagic { magic: b"BM", ext: "bmp", mime: "image/bmp", has_end_marker: false },
    FileMagic { magic: &[0x49, 0x49, 0x2A, 0x00], ext: "tiff", mime: "image/tiff", has_end_marker: false },
    FileMagic { magic: &[0x4D, 0x4D, 0x00, 0x2A], ext: "tiff", mime: "image/tiff", has_end_marker: false },
    // Documents
    FileMagic { magic: b"%PDF", ext: "pdf", mime: "application/pdf", has_end_marker: true },
    FileMagic { magic: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1], ext: "doc", mime: "application/msword", has_end_marker: false },
    FileMagic { magic: b"PK\x03\x04\x14\x00\x06\x00", ext: "docx", mime: "application/vnd.openxmlformats-officedocument.wordprocessingml.document", has_end_marker: false },
    FileMagic { magic: b"{\\rtf", ext: "rtf", mime: "application/rtf", has_end_marker: false },
    // Archives
    FileMagic { magic: &[0x50, 0x4B, 0x03, 0x04], ext: "zip", mime: "application/zip", has_end_marker: true },
    FileMagic { magic: &[0x50, 0x4B, 0x05, 0x06], ext: "zip", mime: "application/zip", has_end_marker: true },
    FileMagic { magic: &[0x1F, 0x8B], ext: "gz", mime: "application/gzip", has_end_marker: false },
    FileMagic { magic: &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07], ext: "rar", mime: "application/x-rar-compressed", has_end_marker: false },
    FileMagic { magic: &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C], ext: "7z", mime: "application/x-7z-compressed", has_end_marker: false },
    FileMagic { magic: &[0x75, 0x73, 0x74, 0x61, 0x72], ext: "tar", mime: "application/x-tar", has_end_marker: false },
    FileMagic { magic: &[0x42, 0x5A, 0x68], ext: "bz2", mime: "application/x-bzip2", has_end_marker: false },
    FileMagic { magic: &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00], ext: "xz", mime: "application/x-xz", has_end_marker: false },
    // Video
    FileMagic { magic: &[0x00, 0x00, 0x00, 0x1C, 0x66, 0x74, 0x79, 0x70], ext: "mp4", mime: "video/mp4", has_end_marker: false },
    FileMagic { magic: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70], ext: "mp4", mime: "video/mp4", has_end_marker: false },
    FileMagic { magic: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70], ext: "mp4", mime: "video/mp4", has_end_marker: false },
    FileMagic { magic: &[0x1A, 0x45, 0xDF, 0xA3], ext: "webm", mime: "video/webm", has_end_marker: false },
    FileMagic { magic: b"FLV", ext: "flv", mime: "video/x-flv", has_end_marker: false },
    FileMagic { magic: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70], ext: "mov", mime: "video/quicktime", has_end_marker: false },
    FileMagic { magic: &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11], ext: "wmv", mime: "video/x-ms-wmv", has_end_marker: false },
    FileMagic { magic: b"RIFF", ext: "avi", mime: "video/x-msvideo", has_end_marker: false },
    // Audio
    FileMagic { magic: &[0x49, 0x44, 0x33], ext: "mp3", mime: "audio/mpeg", has_end_marker: false },
    FileMagic { magic: &[0xFF, 0xFB], ext: "mp3", mime: "audio/mpeg", has_end_marker: false },
    FileMagic { magic: &[0xFF, 0xFA], ext: "mp3", mime: "audio/mpeg", has_end_marker: false },
    FileMagic { magic: &[0xFF, 0xF3], ext: "mp3", mime: "audio/mpeg", has_end_marker: false },
    FileMagic { magic: b"OggS", ext: "ogg", mime: "audio/ogg", has_end_marker: false },
    FileMagic { magic: b"fLaC", ext: "flac", mime: "audio/flac", has_end_marker: false },
    FileMagic { magic: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x41], ext: "m4a", mime: "audio/mp4", has_end_marker: false },
    // Executables and scripts
    FileMagic { magic: &[0x4D, 0x5A], ext: "exe", mime: "application/x-msdownload", has_end_marker: false },
    FileMagic { magic: &[0x7F, 0x45, 0x4C, 0x46], ext: "elf", mime: "application/x-executable", has_end_marker: false },
    FileMagic { magic: &[0xCE, 0xFA, 0xED, 0xFE], ext: "macho", mime: "application/x-mach-binary", has_end_marker: false },
    FileMagic { magic: &[0xCF, 0xFA, 0xED, 0xFE], ext: "macho64", mime: "application/x-mach-binary", has_end_marker: false },
    FileMagic { magic: b"#!", ext: "sh", mime: "text/x-shellscript", has_end_marker: false },
    FileMagic { magic: b"<?php", ext: "php", mime: "text/x-php", has_end_marker: false },
    FileMagic { magic: b"<?xml", ext: "xml", mime: "application/xml", has_end_marker: false },
    // Fonts
    FileMagic { magic: &[0x00, 0x01, 0x00, 0x00], ext: "ttf", mime: "font/ttf", has_end_marker: false },
    FileMagic { magic: b"wOFF", ext: "woff", mime: "font/woff", has_end_marker: false },
    FileMagic { magic: b"wOF2", ext: "woff2", mime: "font/woff2", has_end_marker: false },
    // Database
    FileMagic { magic: b"SQLite format 3", ext: "sqlite", mime: "application/x-sqlite3", has_end_marker: false },
    // CTF common: flags, keys, certificates
    FileMagic { magic: b"-----BEGIN", ext: "pem", mime: "application/x-pem-file", has_end_marker: true },
    FileMagic { magic: b"ssh-rsa ", ext: "pub", mime: "text/plain", has_end_marker: false },
    FileMagic { magic: b"ssh-ed25519 ", ext: "pub", mime: "text/plain", has_end_marker: false },
    // Java
    FileMagic { magic: &[0xCA, 0xFE, 0xBA, 0xBE], ext: "class", mime: "application/java-vm", has_end_marker: false },
    // Python compiled
    FileMagic { magic: &[0x03, 0xF3, 0x0D, 0x0A], ext: "pyc", mime: "application/x-python-code", has_end_marker: false },
    // Pcap (nested capture files)
    FileMagic { magic: &[0xD4, 0xC3, 0xB2, 0xA1], ext: "pcap", mime: "application/vnd.tcpdump.pcap", has_end_marker: false },
    FileMagic { magic: &[0xA1, 0xB2, 0xC3, 0xD4], ext: "pcap", mime: "application/vnd.tcpdump.pcap", has_end_marker: false },
    FileMagic { magic: &[0x0A, 0x0D, 0x0D, 0x0A], ext: "pcapng", mime: "application/x-pcapng", has_end_marker: false },
    // Disk images
    FileMagic { magic: b"QEMU", ext: "qcow", mime: "application/x-qemu-disk", has_end_marker: false },
    FileMagic { magic: b"conectix", ext: "vhd", mime: "application/x-vhd", has_end_marker: false },
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
                            &response_data, &content_type, &filename, &src, &dst, &packet_ids, stream_key, "HTTP"
                        ) { files.push(file); }
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
                                filename = name.trim_matches(|c| c == '"' || c == '\'' || c == ';').to_string();
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
                &response_data, &content_type, &filename, &src, &dst, &packet_ids, stream_key, "HTTP"
            ) { files.push(file); }
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
                    current_filename = text[start + 5..].lines().next().unwrap_or("").trim().to_string();
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
            let src_port: u16 = pkt.src.split(':').next_back().and_then(|p| p.parse().ok()).unwrap_or(0);
            let dst_port: u16 = pkt.dst.split(':').next_back().and_then(|p| p.parse().ok()).unwrap_or(0);
            
            if (src_port == 20 || dst_port == 20 || src_port > 1024 && dst_port > 1024) 
                && pkt.protocol == "TCP" && pkt.raw.len() > 60 {
                // Check if this looks like binary data
                let payload = if pkt.raw.len() > 54 { &pkt.raw[54..] } else { &pkt.raw };
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
                let filename = if current_filename.is_empty() { "ftp_transfer".to_string() } else { current_filename.clone() };
                if let Some(file) = Self::create_extracted_file(
                    &data_buffer, "", &filename, &src, &dst, &packet_ids, stream_key, "FTP"
                ) { files.push(file); }
                data_buffer.clear();
                packet_ids.clear();
            }
        }

        files
    }

    /// Extract email attachments from SMTP/POP3/IMAP
    fn extract_email_attachments(packets: &[&CapturedPacket], stream_key: &str) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        let mut email_data = String::new();
        let mut packet_ids = Vec::new();
        let mut src = String::new();
        let mut dst = String::new();

        for pkt in packets {
            let text = String::from_utf8_lossy(&pkt.raw);
            
            // Check for email protocols
            let is_email = pkt.protocol == "SMTP" || pkt.protocol == "POP3" || pkt.protocol == "IMAP"
                || text.contains("MAIL FROM:") || text.contains("DATA\r\n") 
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
            files.extend(Self::parse_mime_attachments(&email_data, &src, &dst, &packet_ids, stream_key));
        }

        files
    }

    /// Parse MIME attachments from email data
    fn parse_mime_attachments(data: &str, src: &str, dst: &str, packet_ids: &[u64], stream_key: &str) -> Vec<ExtractedFile> {
        let mut files = Vec::new();
        
        // Find boundary
        let boundary = data.lines()
            .find(|l| l.to_lowercase().contains("boundary="))
            .and_then(|l| {
                l.split("boundary=").nth(1).map(|b| b.trim_matches(|c| c == '"' || c == '\'' || c == ';'))
            });

        if let Some(boundary) = boundary {
            let boundary_marker = format!("--{}", boundary);
            let parts: Vec<&str> = data.split(&boundary_marker).collect();
            
            for part in parts {
                // Check for attachment
                if part.to_lowercase().contains("content-disposition: attachment") 
                    || part.to_lowercase().contains("content-transfer-encoding: base64") {
                    
                    // Get filename
                    let filename = part.lines()
                        .find(|l| l.to_lowercase().contains("filename="))
                        .and_then(|l| l.split("filename=").nth(1))
                        .map(|n| n.trim_matches(|c| c == '"' || c == '\'').to_string())
                        .unwrap_or_else(|| "attachment".to_string());
                    
                    // Get content type
                    let content_type = part.lines()
                        .find(|l| l.to_lowercase().starts_with("content-type:"))
                        .map(|l| l[13..].trim().split(';').next().unwrap_or("").to_string())
                        .unwrap_or_default();
                    
                    // Decode base64 content
                    if let Some(body_start) = part.find("\r\n\r\n").or_else(|| part.find("\n\n")) {
                        let body = &part[body_start..].trim();
                        let cleaned: String = body.chars().filter(|c| !c.is_whitespace()).collect();
                        if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cleaned) {
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
    fn extract_magic_from_stream(packets: &[&CapturedPacket], stream_key: &str) -> Vec<ExtractedFile> {
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
            if pkt.raw.len() < 50 { continue; }
            
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
        let base64_regex = regex::Regex::new(r"(?:[A-Za-z0-9+/]{4}){10,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?").ok();

        if let Some(re) = base64_regex {
            for pkt in packets {
                let text = String::from_utf8_lossy(&pkt.raw);
                
                for cap in re.find_iter(&text) {
                    let b64_str = cap.as_str();
                    if b64_str.len() < 100 { continue; } // Skip short strings
                    
                    if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64_str) {
                        if decoded.len() < 20 { continue; }
                        
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
            if pkt.protocol != "DNS" { continue; }
            
            // Look for suspicious DNS queries (long subdomains, TXT records)
            // Extract hex/base64 from subdomain labels
            for layer in &pkt.layers {
                if layer.name == "DNS" {
                    for field in &layer.fields {
                        if field.name == "Query Name" || field.name == "TXT Data" {
                            let value = &field.value;
                            // Check for encoded data (hex or base64 looking)
                            let cleaned: String = value.chars()
                                .filter(|c| c.is_ascii_hexdigit() || c.is_ascii_alphanumeric() || *c == '+' || *c == '/')
                                .collect();
                            
                            if cleaned.len() > 20 {
                                // Try hex decode
                                if let Ok(decoded) = hex::decode(&cleaned) {
                                    dns_data.extend_from_slice(&decoded);
                                }
                                // Try base64 decode
                                else if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cleaned) {
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
            if pkt.protocol != "ICMP" { continue; }
            
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
        if raw.len() < 54 { return &[]; }
        
        // Ethernet (14) + IP (20 min) + TCP/UDP (8-20)
        // Try to find actual payload start
        let eth_type = if raw.len() > 13 { ((raw[12] as u16) << 8) | raw[13] as u16 } else { 0 };
        
        let ip_start = 14;
        if eth_type != 0x0800 && eth_type != 0x86DD { return raw; } // Not IP
        
        let ip_header_len = if raw.len() > ip_start { ((raw[ip_start] & 0x0F) as usize) * 4 } else { 20 };
        let transport_start = ip_start + ip_header_len;
        
        if raw.len() <= transport_start + 8 { return &[]; }
        
        let proto = raw.get(ip_start + 9).copied().unwrap_or(0);
        let transport_header_len = match proto {
            6 => { // TCP
                if raw.len() > transport_start + 12 {
                    ((raw[transport_start + 12] >> 4) as usize) * 4
                } else { 20 }
            },
            17 => 8, // UDP
            _ => 8,
        };
        
        let payload_start = transport_start + transport_header_len;
        if payload_start < raw.len() { &raw[payload_start..] } else { &[] }
    }

    fn detect_file_at_offset(data: &[u8], offset: usize) -> Option<(&'static FileMagic, Vec<u8>)> {
        for magic in FILE_MAGICS {
            if offset + magic.magic.len() <= data.len()
                && &data[offset..offset + magic.magic.len()] == magic.magic {
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
                        for i in start + magic.magic.len()..search_end.saturating_sub(marker.len()) {
                            if i + marker.len() <= data.len() && &data[i..i + marker.len()] == *marker {
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
        if magic.ext == "bmp"
            && data.len() >= start + 6 {
                let size = u32::from_le_bytes([data[start + 2], data[start + 3], data[start + 4], data[start + 5]]) as usize;
                if size > 0 && size < max_size {
                    return (start + size).min(data.len());
                }
            }
        
        // Default: reasonable chunk
        (start + max_size.min(1024 * 512)).min(data.len())
    }

    fn create_extracted_file(
        data: &[u8], content_type: &str, filename: &str, src: &str, dst: &str,
        packet_ids: &[u64], stream_key: &str, source_type: &str
    ) -> Option<ExtractedFile> {
        if data.len() < 20 { return None; }
        
        // Try to detect actual type from magic
        let (detected_ext, detected_mime) = Self::detect_type_from_magic(data);
        
        let final_mime = if content_type.is_empty() || content_type.contains("octet-stream") {
            detected_mime.to_string()
        } else {
            content_type.to_string()
        };
        
        // Skip plain HTML if it's really HTML
        if final_mime.contains("text/html") && data.len() < 500 { return None; }
        
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
    pub fn get_file_packets<'a>(file: &ExtractedFile, packets: &'a [CapturedPacket]) -> Vec<&'a CapturedPacket> {
        packets.iter()
            .filter(|p| file.packet_ids.contains(&p.id))
            .collect()
    }

    /// Get all packets in the same stream as a file
    pub fn get_stream_packets<'a>(file: &ExtractedFile, packets: &'a [CapturedPacket]) -> Vec<&'a CapturedPacket> {
        packets.iter()
            .filter(|p| {
                let key = Self::stream_key(&p.src, &p.dst);
                key == file.stream_key
            })
            .collect()
    }

    /// Extract files and save to directory
    pub fn extract_and_save(packets: &[CapturedPacket], output_dir: &Path) -> Result<Vec<ExtractedFile>, String> {
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
        std::fs::write(path, &file.data)
            .map_err(|e| format!("Failed to write file: {}", e))
    }
}

/// Get MAC vendor name (simplified, just show prefix)
fn get_mac_vendor(mac: &str) -> String {
    let prefix = mac.split(':').take(3).collect::<Vec<_>>().join(":");
    // Common vendors
    match prefix.to_uppercase().as_str() {
        "00:1A:2B" | "00:50:56" | "00:0C:29" => "VMware".to_string(),
        "08:00:27" => "VirtualBox".to_string(),
        "52:54:00" => "QEMU".to_string(),
        "00:15:5D" => "Hyper-V".to_string(),
        "A0:36:BC" => "ASUSTek".to_string(),
        "84:87:FF" => "Shenzhen".to_string(),
        _ => {
            // Return first part of MAC as vendor hint
            prefix.replace(":", "").to_uppercase()
        }
    }
}

/// Get DSCP name
fn get_dscp_name(dscp: u8) -> String {
    match dscp {
        0 => "Default (0)".to_string(),
        8 => "CS1 (8)".to_string(),
        16 => "CS2 (16)".to_string(),
        24 => "CS3 (24)".to_string(),
        32 => "CS4 (32)".to_string(),
        40 => "CS5 (40)".to_string(),
        46 => "EF (46)".to_string(),
        48 => "CS6 (48)".to_string(),
        56 => "CS7 (56)".to_string(),
        _ => format!("Unknown ({})", dscp),
    }
}

/// Get ECN name
fn get_ecn_name(ecn: u8) -> String {
    match ecn {
        0 => "Not-ECT".to_string(),
        1 => "ECT(1)".to_string(),
        2 => "ECT(0)".to_string(),
        3 => "CE".to_string(),
        _ => format!("Unknown ({})", ecn),
    }
}

/// Get ICMP type name
fn get_icmp_type_name(t: u8) -> &'static str {
    match t {
        0 => "Echo Reply",
        3 => "Destination Unreachable",
        4 => "Source Quench",
        5 => "Redirect",
        8 => "Echo Request",
        9 => "Router Advertisement",
        10 => "Router Solicitation",
        11 => "Time Exceeded",
        12 => "Parameter Problem",
        13 => "Timestamp Request",
        14 => "Timestamp Reply",
        _ => "Unknown",
    }
}

/// Get ICMPv6 type name
fn get_icmpv6_type_name(t: u8) -> String {
    match t {
        1 => "Destination Unreachable".to_string(),
        2 => "Packet Too Big".to_string(),
        3 => "Time Exceeded".to_string(),
        4 => "Parameter Problem".to_string(),
        128 => "Echo Request".to_string(),
        129 => "Echo Reply".to_string(),
        130 => "Multicast Listener Query".to_string(),
        131 => "Multicast Listener Report".to_string(),
        132 => "Multicast Listener Done".to_string(),
        133 => "Router Solicitation".to_string(),
        134 => "Router Advertisement".to_string(),
        135 => "Neighbor Solicitation".to_string(),
        136 => "Neighbor Advertisement".to_string(),
        137 => "Redirect Message".to_string(),
        143 => "Multicast Listener Report v2".to_string(),
        _ => format!("Unknown ({})", t),
    }
}

/// Get standard IP protocol name from protocol number
fn get_ip_protocol_name(p: u8) -> String {
    match p {
        1 => "ICMP".to_string(),
        2 => "IGMP".to_string(),
        6 => "TCP".to_string(),
        17 => "UDP".to_string(),
        41 => "IPv6".to_string(),
        43 => "IPv6-Route".to_string(),
        44 => "IPv6-Frag".to_string(),
        50 => "ESP".to_string(),
        51 => "AH".to_string(),
        58 => "ICMPv6".to_string(),
        59 => "IPv6-NoNxt".to_string(),
        60 => "IPv6-Opts".to_string(),
        89 => "OSPF".to_string(),
        115 => "L2TP".to_string(),
        _ => format!("Protocol({})", p),
    }
}

/// Format TCP flags short string
fn format_tcp_flags_short(flags: u16) -> String {
    let mut parts = Vec::new();
    if flags & 0x02 != 0 { parts.push("SYN"); }
    if flags & 0x10 != 0 { parts.push("ACK"); }
    if flags & 0x01 != 0 { parts.push("FIN"); }
    if flags & 0x04 != 0 { parts.push("RST"); }
    if flags & 0x08 != 0 { parts.push("PSH"); }
    if flags & 0x20 != 0 { parts.push("URG"); }
    if parts.is_empty() { "".to_string() } else { parts.join(", ") }
}

/// Detect application protocol for TCP based on port
fn detect_app_protocol_tcp(src_port: u16, dst_port: u16) -> String {
    let port = if is_well_known_port(dst_port) { dst_port } else { src_port };
    match port {
        80 | 8080 | 8000 | 3000 => "HTTP".to_string(),
        443 | 8443 => "TLS".to_string(),
        21 => "FTP".to_string(),
        22 => "SSH".to_string(),
        23 => "Telnet".to_string(),
        25 | 587 | 465 => "SMTP".to_string(),
        110 | 995 => "POP3".to_string(),
        143 | 993 => "IMAP".to_string(),
        53 => "DNS".to_string(),
        3306 => "MySQL".to_string(),
        5432 => "PostgreSQL".to_string(),
        6379 => "Redis".to_string(),
        27017 => "MongoDB".to_string(),
        3389 => "RDP".to_string(),
        _ => "TCP".to_string(),
    }
}

/// Detect application protocol for UDP based on port
fn detect_app_protocol_udp(src_port: u16, dst_port: u16) -> String {
    let port = if is_well_known_port(dst_port) { dst_port } else { src_port };
    match port {
        53 => "DNS".to_string(),
        67 | 68 => "DHCP".to_string(),
        69 => "TFTP".to_string(),
        123 => "NTP".to_string(),
        137..=139 => "NetBIOS".to_string(),
        161 | 162 => "SNMP".to_string(),
        443 => "QUIC".to_string(),
        5353 => "mDNS".to_string(),
        5355 => "LLMNR".to_string(),
        _ => "UDP".to_string(),
    }
}

fn is_well_known_port(port: u16) -> bool {
    port < 1024 || matches!(port, 3306 | 5432 | 6379 | 27017 | 3389 | 8080 | 8443 | 3000 | 8000 | 5353 | 5355)
}

/// Parse HTTP content from TCP payload
fn parse_http_content(payload: &[u8]) -> Option<(String, String, ProtocolLayer)> {
    let text = String::from_utf8_lossy(payload);
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() { return None; }
    
    let first_line = lines[0];
    let mut fields = Vec::new();
    
    // HTTP request
    if first_line.starts_with("GET ") || first_line.starts_with("POST ") || 
       first_line.starts_with("PUT ") || first_line.starts_with("DELETE ") ||
       first_line.starts_with("HEAD ") || first_line.starts_with("OPTIONS ") {
        let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let method = parts[0];
            let uri = parts[1];
            let version = parts.get(2).unwrap_or(&"HTTP/1.1");
            
            fields.push(ProtocolField::new("Request Method", method));
            fields.push(ProtocolField::new("Request URI", uri));
            fields.push(ProtocolField::new("Request Version", version));
            
            // Extract headers
            let mut host = "";
            for line in &lines[1..] {
                if line.is_empty() { break; }
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim();
                    let val = v.trim();
                    if key.eq_ignore_ascii_case("host") { host = val; }
                    fields.push(ProtocolField::new(key, val));
                }
            }
            
            let display = format!("Hypertext Transfer Protocol ({} {})", method, uri);
            let info = format!("{} {} {}", method, uri, host);
            return Some(("HTTP".to_string(), info, ProtocolLayer { name: "HTTP".to_string(), display, fields }));
        }
    }
    
    // HTTP response
    if first_line.starts_with("HTTP/") {
        let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let version = parts[0];
            let status = parts[1];
            let reason = parts.get(2).unwrap_or(&"");
            
            fields.push(ProtocolField::new("Response Version", version));
            fields.push(ProtocolField::new("Status Code", status));
            fields.push(ProtocolField::new("Response Phrase", reason));
            
            for line in &lines[1..] {
                if line.is_empty() { break; }
                if let Some((k, v)) = line.split_once(':') {
                    fields.push(ProtocolField::new(k.trim(), v.trim()));
                }
            }
            
            let display = format!("Hypertext Transfer Protocol ({} {})", status, reason);
            let info = format!("{} {} {}", version, status, reason);
            return Some(("HTTP".to_string(), info, ProtocolLayer { name: "HTTP".to_string(), display, fields }));
        }
    }
    
    None
}

/// Parse DNS content from UDP payload
fn parse_dns_content(payload: &[u8], _is_response: bool) -> Option<(String, ProtocolLayer)> {
    if payload.len() < 12 { return None; }
    
    let id = u16::from_be_bytes([payload[0], payload[1]]);
    let flags = u16::from_be_bytes([payload[2], payload[3]]);
    let qr = (flags >> 15) & 1;
    let opcode = (flags >> 11) & 0xf;
    let rcode = flags & 0xf;
    let qdcount = u16::from_be_bytes([payload[4], payload[5]]);
    let ancount = u16::from_be_bytes([payload[6], payload[7]]);
    
    let mut fields = vec![
        ProtocolField::new("Transaction ID", &format!("0x{:04x}", id)),
        ProtocolField::with_children(
            &format!("Flags: 0x{:04x}", flags),
            if qr == 0 { "Standard query" } else { "Standard query response" },
            vec![
                ProtocolField::new(&format!("{} .... .... .... = Response", qr), if qr == 0 { "Message is a query" } else { "Message is a response" }),
                ProtocolField::new(&format!(".{:04b} ... .... .... = Opcode", opcode), &format!("{}", opcode)),
            ]
        ),
        ProtocolField::new("Questions", &qdcount.to_string()),
        ProtocolField::new("Answer RRs", &ancount.to_string()),
    ];
    
    // Parse query name
    let mut offset = 12;
    let mut domain_parts = Vec::new();
    while offset < payload.len() {
        let len = payload[offset] as usize;
        if len == 0 { offset += 1; break; }
        if offset + 1 + len > payload.len() { break; }
        if let Ok(part) = std::str::from_utf8(&payload[offset + 1..offset + 1 + len]) {
            domain_parts.push(part.to_string());
        }
        offset += 1 + len;
    }
    let domain = domain_parts.join(".");
    
    // Parse query type
    let mut qtype_str = "Unknown";
    if offset + 4 <= payload.len() {
        let qtype = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        qtype_str = match qtype { 1 => "A", 28 => "AAAA", 5 => "CNAME", 15 => "MX", 2 => "NS", _ => "Unknown" };
        fields.push(ProtocolField::new("Query Name", &domain));
        fields.push(ProtocolField::new("Query Type", &format!("{} ({})", qtype_str, qtype)));
        offset += 4;
    }
    
    // Parse answers
    let mut resolved = Vec::new();
    if qr == 1 && ancount > 0 {
        for _ in 0..ancount {
            if offset >= payload.len() { break; }
            // Skip name
            if payload[offset] & 0xc0 == 0xc0 { offset += 2; }
            else { while offset < payload.len() && payload[offset] != 0 { offset += payload[offset] as usize + 1; } offset += 1; }
            
            if offset + 10 > payload.len() { break; }
            let rtype = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
            let rdlen = u16::from_be_bytes([payload[offset + 8], payload[offset + 9]]) as usize;
            offset += 10;
            if offset + rdlen > payload.len() { break; }
            
            if rtype == 1 && rdlen == 4 {
                let ip = format!("{}.{}.{}.{}", payload[offset], payload[offset + 1], payload[offset + 2], payload[offset + 3]);
                resolved.push(ip);
            }
            offset += rdlen;
        }
        if !resolved.is_empty() {
            fields.push(ProtocolField::new("Resolved Addresses", &resolved.join(", ")));
        }
    }
    
    let display = if qr == 0 {
        format!("Domain Name System (query) - {}", domain)
    } else {
        format!("Domain Name System (response) - {}", domain)
    };
    
    let info = if qr == 0 {
        format!("Standard query {} {}", qtype_str, domain)
    } else if rcode == 0 {
        if !resolved.is_empty() {
            format!("Standard query response {} {}", domain, resolved.join(" "))
        } else {
            format!("Standard query response {}", domain)
        }
    } else {
        format!("Standard query response {} (error {})", domain, rcode)
    };
    
    Some((info, ProtocolLayer { name: "DNS".to_string(), display, fields }))
}
