use super::*;
use crate::buildin_tools::port_scan::sctp::{
    build_sctp_init_frame, parse_icmp_sctp_unreachable, parse_sctp_response, sctp_checksum,
    SCTP_CHUNK_TYPE_INIT_ACK, SCTP_NEXT_HEADER,
};
use crate::buildin_tools::port_scan::tcp::{build_tcp_frame, parse_tcp_response, TcpResponse};
use crate::buildin_tools::port_scan::udp::{
    build_udp_frame, parse_icmp_udp_unreachable, parse_udp_response, DNS_UDP_PROBE_PAYLOAD,
    GENERIC_UDP_PROBE_PAYLOAD,
};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Flags, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::tcp::{TcpFlags, TcpPacket};
use pnet::packet::udp::UdpPacket;
use pnet::packet::{MutablePacket, Packet};
use rsubdomain::PacketTransport;
use std::collections::{BTreeMap, BTreeSet};

const ETHERNET_HEADER_LEN: usize = 14;
const IPV4_HEADER_LEN: usize = 20;

fn sample_route() -> EthTable {
    EthTable {
        src_ip: Ipv4Addr::new(192, 168, 10, 20),
        device: "en0".to_string(),
        src_mac: pnet::datalink::MacAddr::new(0x10, 0x22, 0x33, 0x44, 0x55, 0x66),
        dst_mac: pnet::datalink::MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff),
        transport: PacketTransport::Ethernet,
    }
}

fn sample_probe() -> ScheduledProbe {
    let task = RoutedSynTask {
        task: SocketScanTask {
            host: "api.example.com".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 99)),
            port: 443,
            protocol: ScanProtocol::Tcp,
        },
        target_ip: Ipv4Addr::new(192, 168, 10, 99),
    };

    ScheduledProbe {
        pending: PendingSyn::new(task, 40_001),
        attempts_sent: 1,
        deadline: Instant::now() + Duration::from_secs(1),
    }
}

fn sample_space() -> RoutedSocketSpace {
    RoutedSocketSpace {
        protocol: ScanProtocol::Tcp,
        targets: vec![
            RoutedTarget {
                host: "api.example.com".to_string(),
                target_ip: Ipv4Addr::new(192, 168, 10, 99),
            },
            RoutedTarget {
                host: "www.example.com".to_string(),
                target_ip: Ipv4Addr::new(192, 168, 10, 100),
            },
        ],
        ports: vec![80, 443],
    }
}

#[test]
fn route_builder_groups_targets_by_route_and_protocol() {
    let shared_route = sample_route();
    let mut route_cache = BTreeMap::new();
    route_cache.insert(Ipv4Addr::new(192, 168, 10, 99), Ok(shared_route.clone()));
    route_cache.insert(Ipv4Addr::new(192, 168, 10, 100), Ok(shared_route.clone()));
    route_cache.insert(
        Ipv4Addr::new(192, 168, 10, 101),
        Err("no route to target".to_string()),
    );

    let targets = vec![
        ResolvedSocketTarget {
            host: "api.example.com".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 99)),
        },
        ResolvedSocketTarget {
            host: "www.example.com".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 100)),
        },
        ResolvedSocketTarget {
            host: "offline.example.com".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 101)),
        },
        ResolvedSocketTarget {
            host: "::1".to_string(),
            ip: "::1".parse().expect("ipv6 loopback"),
        },
    ];
    let requested_ports = vec![
        RequestedScanPort {
            port: 80,
            protocol: ScanProtocol::Tcp,
        },
        RequestedScanPort {
            port: 443,
            protocol: ScanProtocol::Tcp,
        },
        RequestedScanPort {
            port: 53,
            protocol: ScanProtocol::Udp,
        },
    ];

    let routed = build_routed_task_collection(targets, &requested_ports, route_cache);

    assert_eq!(routed.groups.len(), 2);
    assert_eq!(
        routed
            .host_errors
            .get("offline.example.com")
            .map(String::as_str),
        Some("raw_syn routing failed for 192.168.10.101: no route to target")
    );
    assert_eq!(
        routed.host_errors.get("::1").map(String::as_str),
        Some("raw_syn engine currently supports IPv4 targets only")
    );

    let tcp_group = routed
        .groups
        .iter()
        .find(|group| group.space.protocol == ScanProtocol::Tcp)
        .expect("tcp group should exist");
    assert_eq!(tcp_group.space.ports, vec![80, 443]);
    assert_eq!(tcp_group.space.targets.len(), 2);

    let udp_group = routed
        .groups
        .iter()
        .find(|group| group.space.protocol == ScanProtocol::Udp)
        .expect("udp group should exist");
    assert_eq!(udp_group.space.ports, vec![53]);
    assert_eq!(udp_group.space.targets.len(), 2);
}

fn build_sctp_reply_frame(
    route: &EthTable,
    source_ip: Ipv4Addr,
    destination_ip: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    verification_tag: u32,
    chunk_type: u8,
    chunk_flags: u8,
    initiate_tag: Option<u32>,
) -> Vec<u8> {
    let chunk_len = if initiate_tag.is_some() { 20 } else { 4 };
    let total_len = ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + 12 + chunk_len;
    let mut frame = vec![0u8; total_len];

    let mut ethernet =
        MutableEthernetPacket::new(&mut frame).expect("ethernet frame buffer should fit");
    ethernet.set_source(route.dst_mac);
    ethernet.set_destination(route.src_mac);
    ethernet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4 = MutableIpv4Packet::new(ethernet.payload_mut()).expect("ipv4 packet");
    ipv4.set_version(4);
    ipv4.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4.set_total_length((IPV4_HEADER_LEN + 12 + chunk_len) as u16);
    ipv4.set_ttl(64);
    ipv4.set_flags(Ipv4Flags::DontFragment);
    ipv4.set_next_level_protocol(SCTP_NEXT_HEADER);
    ipv4.set_source(source_ip);
    ipv4.set_destination(destination_ip);
    let payload = ipv4.payload_mut();
    payload[0..2].copy_from_slice(&source_port.to_be_bytes());
    payload[2..4].copy_from_slice(&destination_port.to_be_bytes());
    payload[4..8].copy_from_slice(&verification_tag.to_be_bytes());
    payload[8..12].fill(0);
    payload[12] = chunk_type;
    payload[13] = chunk_flags;
    payload[14..16].copy_from_slice(&(chunk_len as u16).to_be_bytes());
    if let Some(init_tag) = initiate_tag {
        payload[16..20].copy_from_slice(&init_tag.to_be_bytes());
        payload[20..24].copy_from_slice(&0x0000_8000u32.to_be_bytes());
        payload[24..26].copy_from_slice(&10u16.to_be_bytes());
        payload[26..28].copy_from_slice(&2048u16.to_be_bytes());
        payload[28..32].copy_from_slice(&0x1234_5678u32.to_be_bytes());
    }
    let checksum = sctp_checksum(payload);
    payload[8..12].copy_from_slice(&checksum.to_be_bytes());
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4.to_immutable());
    ipv4.set_checksum(ip_checksum);

    frame
}

#[test]
fn build_tcp_frame_sets_expected_syn_headers() {
    let route = sample_route();
    let packet = build_tcp_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        40_001,
        443,
        0x11223344,
        0,
        TcpFlags::SYN,
        61,
    );

    let ethernet = EthernetPacket::new(&packet).expect("ethernet packet");
    assert_eq!(ethernet.get_source(), route.src_mac);
    assert_eq!(ethernet.get_destination(), route.dst_mac);
    assert_eq!(ethernet.get_ethertype(), EtherTypes::Ipv4);

    let ipv4 = Ipv4Packet::new(ethernet.payload()).expect("ipv4 packet");
    assert_eq!(ipv4.get_source(), route.src_ip);
    assert_eq!(ipv4.get_destination(), Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(ipv4.get_ttl(), 61);
    assert_eq!(ipv4.get_next_level_protocol(), IpNextHeaderProtocols::Tcp);

    let tcp = TcpPacket::new(ipv4.payload()).expect("tcp packet");
    assert_eq!(tcp.get_source(), 40_001);
    assert_eq!(tcp.get_destination(), 443);
    assert_eq!(tcp.get_flags(), TcpFlags::SYN);
    assert_eq!(tcp.get_sequence(), 0x11223344);
}

#[test]
fn parse_tcp_response_extracts_syn_ack_fields() {
    let route = sample_route();
    let packet = build_tcp_frame(
        &EthTable {
            src_ip: Ipv4Addr::new(192, 168, 10, 99),
            device: route.device.clone(),
            src_mac: route.dst_mac,
            dst_mac: route.src_mac,
            transport: PacketTransport::Ethernet,
        },
        route.src_ip,
        443,
        40_001,
        0x55667788,
        0x11223345,
        TcpFlags::SYN | TcpFlags::ACK,
        64,
    );

    let response = parse_tcp_response(&packet).expect("tcp response");
    assert_eq!(response.source_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(response.destination_ip, route.src_ip);
    assert_eq!(response.source_port, 443);
    assert_eq!(response.destination_port, 40_001);
    assert_eq!(response.flags, TcpFlags::SYN | TcpFlags::ACK);
    assert_eq!(response.sequence, 0x55667788);
    assert_eq!(response.acknowledgement, 0x11223345);
    assert_eq!(response.ttl, 64);
}

#[test]
fn build_udp_frame_sets_expected_headers() {
    let route = sample_route();
    let packet = build_udp_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        40_002,
        53,
        DNS_UDP_PROBE_PAYLOAD,
        59,
    );

    let ethernet = EthernetPacket::new(&packet).expect("ethernet packet");
    assert_eq!(ethernet.get_source(), route.src_mac);
    assert_eq!(ethernet.get_destination(), route.dst_mac);
    assert_eq!(ethernet.get_ethertype(), EtherTypes::Ipv4);

    let ipv4 = Ipv4Packet::new(ethernet.payload()).expect("ipv4 packet");
    assert_eq!(ipv4.get_source(), route.src_ip);
    assert_eq!(ipv4.get_destination(), Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(ipv4.get_ttl(), 59);
    assert_eq!(ipv4.get_next_level_protocol(), IpNextHeaderProtocols::Udp);

    let udp = UdpPacket::new(ipv4.payload()).expect("udp packet");
    assert_eq!(udp.get_source(), 40_002);
    assert_eq!(udp.get_destination(), 53);
    assert_eq!(udp.payload(), DNS_UDP_PROBE_PAYLOAD);
}

#[test]
fn parse_udp_response_extracts_ports() {
    let route = sample_route();
    let packet = build_udp_frame(
        &EthTable {
            src_ip: Ipv4Addr::new(192, 168, 10, 99),
            device: route.device.clone(),
            src_mac: route.dst_mac,
            dst_mac: route.src_mac,
            transport: PacketTransport::Ethernet,
        },
        route.src_ip,
        53,
        40_003,
        DNS_UDP_PROBE_PAYLOAD,
        64,
    );

    let response = parse_udp_response(&packet).expect("udp response");
    assert_eq!(response.source_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(response.destination_ip, route.src_ip);
    assert_eq!(response.source_port, 53);
    assert_eq!(response.destination_port, 40_003);
    assert_eq!(response.ttl, 64);
}

#[test]
fn parse_icmp_udp_unreachable_extracts_embedded_ports() {
    let route = sample_route();
    let original_udp = build_udp_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        40_010,
        65000,
        GENERIC_UDP_PROBE_PAYLOAD,
        64,
    );
    let original_ipv4 = {
        let ethernet = EthernetPacket::new(&original_udp).expect("udp ethernet frame");
        ethernet.payload().to_vec()
    };

    let mut frame = vec![0u8; ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + 8 + original_ipv4.len()];
    let mut ethernet =
        MutableEthernetPacket::new(&mut frame).expect("icmp ethernet frame should fit");
    ethernet.set_source(route.dst_mac);
    ethernet.set_destination(route.src_mac);
    ethernet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4 = MutableIpv4Packet::new(ethernet.payload_mut()).expect("icmp ipv4 packet");
    ipv4.set_version(4);
    ipv4.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4.set_total_length((IPV4_HEADER_LEN + 8 + original_ipv4.len()) as u16);
    ipv4.set_ttl(64);
    ipv4.set_flags(Ipv4Flags::DontFragment);
    ipv4.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4.set_source(Ipv4Addr::new(192, 168, 10, 99));
    ipv4.set_destination(route.src_ip);
    ipv4.payload_mut()[0] = IcmpTypes::DestinationUnreachable.0;
    ipv4.payload_mut()[1] = 3;
    ipv4.payload_mut()[4..8].fill(0);
    ipv4.payload_mut()[8..].copy_from_slice(&original_ipv4);
    let checksum = pnet::packet::icmp::checksum(
        &IcmpPacket::new(ipv4.payload()).expect("icmp packet for checksum"),
    );
    ipv4.payload_mut()[2] = (checksum >> 8) as u8;
    ipv4.payload_mut()[3] = checksum as u8;
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4.to_immutable());
    ipv4.set_checksum(ip_checksum);

    let response = parse_icmp_udp_unreachable(&frame).expect("icmp unreachable response");
    assert_eq!(response.source_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(response.destination_ip, route.src_ip);
    assert_eq!(response.embedded_source_ip, route.src_ip);
    assert_eq!(
        response.embedded_destination_ip,
        Ipv4Addr::new(192, 168, 10, 99)
    );
    assert_eq!(response.embedded_source_port, 40_010);
    assert_eq!(response.embedded_destination_port, 65000);
    assert_eq!(response.code, 3);
    assert_eq!(response.ttl, 64);
}

#[test]
fn build_sctp_init_frame_sets_expected_headers() {
    let route = sample_route();
    let packet = build_sctp_init_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        40_020,
        2905,
        0x9e8d_5225,
        57,
    );

    let ethernet = EthernetPacket::new(&packet).expect("ethernet packet");
    assert_eq!(ethernet.get_source(), route.src_mac);
    assert_eq!(ethernet.get_destination(), route.dst_mac);
    assert_eq!(ethernet.get_ethertype(), EtherTypes::Ipv4);

    let ipv4 = Ipv4Packet::new(ethernet.payload()).expect("ipv4 packet");
    assert_eq!(ipv4.get_source(), route.src_ip);
    assert_eq!(ipv4.get_destination(), Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(ipv4.get_ttl(), 57);
    assert_eq!(ipv4.get_next_level_protocol(), SCTP_NEXT_HEADER);

    let payload = ipv4.payload();
    assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), 40_020);
    assert_eq!(u16::from_be_bytes([payload[2], payload[3]]), 2905);
    assert_eq!(
        u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
        0
    );
    assert_eq!(payload[12], 1);
    assert_eq!(
        u32::from_be_bytes([payload[16], payload[17], payload[18], payload[19]]),
        0x9e8d_5225
    );
}

#[test]
fn parse_sctp_response_extracts_init_ack_fields() {
    let route = sample_route();
    let packet = build_sctp_reply_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        route.src_ip,
        2905,
        40_021,
        0x9e8d_5225,
        SCTP_CHUNK_TYPE_INIT_ACK,
        0,
        Some(0x1020_3040),
    );

    let response = parse_sctp_response(&packet).expect("sctp response");
    assert_eq!(response.source_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(response.destination_ip, route.src_ip);
    assert_eq!(response.source_port, 2905);
    assert_eq!(response.destination_port, 40_021);
    assert_eq!(response.verification_tag, 0x9e8d_5225);
    assert_eq!(response.first_chunk_type, SCTP_CHUNK_TYPE_INIT_ACK);
    assert_eq!(response.first_chunk_initiate_tag, Some(0x1020_3040));
    assert_eq!(response.ttl, 64);
}

#[test]
fn parse_icmp_sctp_unreachable_extracts_embedded_ports() {
    let route = sample_route();
    let original_sctp = build_sctp_init_frame(
        &route,
        Ipv4Addr::new(192, 168, 10, 99),
        40_030,
        2905,
        0x9e8d_5225,
        64,
    );
    let original_ipv4 = {
        let ethernet = EthernetPacket::new(&original_sctp).expect("sctp ethernet frame");
        ethernet.payload().to_vec()
    };

    let mut frame = vec![0u8; ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + 8 + original_ipv4.len()];
    let mut ethernet =
        MutableEthernetPacket::new(&mut frame).expect("icmp ethernet frame should fit");
    ethernet.set_source(route.dst_mac);
    ethernet.set_destination(route.src_mac);
    ethernet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4 = MutableIpv4Packet::new(ethernet.payload_mut()).expect("icmp ipv4 packet");
    ipv4.set_version(4);
    ipv4.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4.set_total_length((IPV4_HEADER_LEN + 8 + original_ipv4.len()) as u16);
    ipv4.set_ttl(64);
    ipv4.set_flags(Ipv4Flags::DontFragment);
    ipv4.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4.set_source(Ipv4Addr::new(192, 168, 10, 99));
    ipv4.set_destination(route.src_ip);
    ipv4.payload_mut()[0] = IcmpTypes::DestinationUnreachable.0;
    ipv4.payload_mut()[1] = 2;
    ipv4.payload_mut()[4..8].fill(0);
    ipv4.payload_mut()[8..].copy_from_slice(&original_ipv4);
    let checksum = pnet::packet::icmp::checksum(
        &IcmpPacket::new(ipv4.payload()).expect("icmp packet for checksum"),
    );
    ipv4.payload_mut()[2] = (checksum >> 8) as u8;
    ipv4.payload_mut()[3] = checksum as u8;
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4.to_immutable());
    ipv4.set_checksum(ip_checksum);

    let response = parse_icmp_sctp_unreachable(&frame).expect("icmp sctp unreachable response");
    assert_eq!(response.source_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(response.destination_ip, route.src_ip);
    assert_eq!(response.embedded_source_ip, route.src_ip);
    assert_eq!(
        response.embedded_destination_ip,
        Ipv4Addr::new(192, 168, 10, 99)
    );
    assert_eq!(response.embedded_source_port, 40_030);
    assert_eq!(response.embedded_destination_port, 2905);
    assert_eq!(response.code, 2);
    assert_eq!(response.ttl, 64);
}

#[test]
fn capture_filter_accepts_only_local_port_window() {
    let filter = CaptureFilter {
        local_ip: Ipv4Addr::new(192, 168, 10, 20),
        source_port_start: 40_000,
        source_port_end: 40_255,
    };

    assert!(filter.matches_tcp(&TcpResponse {
        source_ip: Ipv4Addr::new(192, 168, 10, 99),
        destination_ip: Ipv4Addr::new(192, 168, 10, 20),
        source_port: 443,
        destination_port: 40_120,
        flags: TcpFlags::SYN | TcpFlags::ACK,
        sequence: 1,
        acknowledgement: 2,
        ttl: 64,
    }));
    assert!(!filter.matches_tcp(&TcpResponse {
        destination_port: 39_999,
        ..TcpResponse {
            source_ip: Ipv4Addr::new(192, 168, 10, 99),
            destination_ip: Ipv4Addr::new(192, 168, 10, 20),
            source_port: 443,
            destination_port: 40_120,
            flags: TcpFlags::SYN | TcpFlags::ACK,
            sequence: 1,
            acknowledgement: 2,
            ttl: 64,
        }
    }));
}

#[test]
fn response_match_requires_expected_acknowledgement() {
    let probe = sample_probe();
    let good = TcpResponse {
        source_ip: probe.pending.target_ip,
        destination_ip: Ipv4Addr::new(192, 168, 10, 20),
        source_port: probe.pending.task.port,
        destination_port: probe.pending.source_port,
        flags: TcpFlags::SYN | TcpFlags::ACK,
        sequence: 7,
        acknowledgement: probe.pending.expected_ack(),
        ttl: 64,
    };
    let bad = TcpResponse {
        acknowledgement: probe.pending.expected_ack().wrapping_add(10),
        ..good
    };

    assert!(response_matches_probe(
        &probe,
        &good,
        Ipv4Addr::new(192, 168, 10, 20)
    ));
    assert!(!response_matches_probe(
        &probe,
        &bad,
        Ipv4Addr::new(192, 168, 10, 20)
    ));
}

#[test]
fn scheduler_wait_timeout_honors_earliest_deadline() {
    let mut pending = BTreeMap::new();
    pending.insert(
        40_000,
        ScheduledProbe {
            pending: sample_probe().pending,
            attempts_sent: 1,
            deadline: Instant::now() + Duration::from_millis(5),
        },
    );

    let wait = scheduler_wait_timeout(&pending, Duration::from_millis(10));
    assert!(wait <= Duration::from_millis(10));
}

#[test]
fn routed_socket_space_maps_indices_without_prebuilt_task_list() {
    let space = sample_space();

    let first = space.task_at(0).expect("first task");
    assert_eq!(first.task.host, "api.example.com");
    assert_eq!(first.target_ip, Ipv4Addr::new(192, 168, 10, 99));
    assert_eq!(first.task.port, 80);
    assert_eq!(first.task.protocol, ScanProtocol::Tcp);

    let last = space.task_at(3).expect("last task");
    assert_eq!(last.task.host, "www.example.com");
    assert_eq!(last.target_ip, Ipv4Addr::new(192, 168, 10, 100));
    assert_eq!(last.task.port, 443);
    assert_eq!(last.task.protocol, ScanProtocol::Tcp);
}

#[test]
fn permuted_index_cursor_visits_each_index_once() {
    let mut cursor = PermutedIndexCursor::new(9, 0x1234_5678_9abc_def0, 1, 1, 0, None);
    let mut seen = BTreeSet::new();

    while let Some(index) = cursor.next_index() {
        assert!(seen.insert(index), "index {} was emitted twice", index);
    }

    assert_eq!(seen.len(), 9);
    assert_eq!(seen.first().copied(), Some(0));
    assert_eq!(seen.last().copied(), Some(8));
    assert!(cursor.is_exhausted());
}

#[test]
fn permuted_index_cursor_honors_shard_and_resume_window() {
    let mut cursor = PermutedIndexCursor::new(16, 0xfeed_beef, 2, 4, 3, Some(8));
    let mut emitted = Vec::new();

    while let Some(index) = cursor.next_index() {
        emitted.push(index);
    }

    assert_eq!(emitted.len(), 2);
    assert!(cursor.is_exhausted());
    assert_eq!(selected_task_count(16, 2, 4, 3, Some(8)), 2);
}
