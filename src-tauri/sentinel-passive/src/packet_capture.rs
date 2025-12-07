//! Packet capture module - provides network packet capture functionality
//!
//! Similar to Wireshark, captures raw network packets from network interfaces

use pnet::datalink::{self, Channel::Ethernet, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::arp::ArpPacket;
use pnet::packet::Packet;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
            Ok(_) => { error!("Unsupported channel type"); return; }
            Err(e) => { error!("Failed to create datalink channel: {}", e); return; }
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
                proto => (src, dst, format!("{:?}", proto), format!("IP Protocol: {:?}", proto)),
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
                proto => (src, dst, format!("{:?}", proto), format!("IPv6 Protocol: {:?}", proto)),
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
fn parse_dns_content(payload: &[u8], is_response: bool) -> Option<(String, ProtocolLayer)> {
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
