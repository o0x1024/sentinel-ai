use std::net::Ipv4Addr;

use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Flags, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::udp::{ipv4_checksum as udp_ipv4_checksum, MutableUdpPacket, UdpPacket};
use pnet::packet::{MutablePacket, Packet};
use rsubdomain::EthTable;

const ETHERNET_HEADER_LEN: usize = 14;
const IPV4_HEADER_LEN: usize = 20;
const UDP_HEADER_LEN: usize = 8;

pub(crate) const GENERIC_UDP_PROBE_PAYLOAD: &[u8] = b"\x00";
pub(crate) const DNS_UDP_PROBE_PAYLOAD: &[u8] = &[
    0x13, 0x37, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'v', b'e', b'r',
    b's', b'i', b'o', b'n', 0x04, b'b', b'i', b'n', b'd', 0x00, 0x00, 0x10, 0x00, 0x03,
];
pub(crate) const NTP_UDP_PROBE_PAYLOAD: &[u8] = &[
    0x1b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UdpResponse {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: u16,
    pub destination_port: u16,
    pub ttl: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UdpIcmpUnreachable {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub embedded_source_ip: Ipv4Addr,
    pub embedded_destination_ip: Ipv4Addr,
    pub embedded_source_port: u16,
    pub embedded_destination_port: u16,
    pub code: u8,
    pub ttl: u8,
}

pub(crate) fn build_udp_frame(
    route: &EthTable,
    target_ip: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    payload: &[u8],
    ttl: u8,
) -> Vec<u8> {
    let udp_packet_len = UDP_HEADER_LEN + payload.len();
    let total_len = ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + udp_packet_len;
    let mut frame = vec![0u8; total_len];

    let mut ethernet_packet =
        MutableEthernetPacket::new(&mut frame[..]).expect("ethernet frame buffer should fit");
    ethernet_packet.set_source(route.src_mac);
    ethernet_packet.set_destination(route.dst_mac);
    ethernet_packet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut())
        .expect("ipv4 payload should fit inside ethernet frame");
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4_packet.set_total_length((IPV4_HEADER_LEN + udp_packet_len) as u16);
    ipv4_packet.set_identification(destination_port ^ source_port);
    ipv4_packet.set_ttl(ttl.max(1));
    ipv4_packet.set_flags(Ipv4Flags::DontFragment);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Udp);
    ipv4_packet.set_source(route.src_ip);
    ipv4_packet.set_destination(target_ip);
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ip_checksum);

    let mut udp_packet = MutableUdpPacket::new(ipv4_packet.payload_mut())
        .expect("udp payload should fit inside ipv4 frame");
    udp_packet.set_source(source_port);
    udp_packet.set_destination(destination_port);
    udp_packet.set_length(udp_packet_len as u16);
    udp_packet.set_payload(payload);
    let udp_checksum = udp_ipv4_checksum(&udp_packet.to_immutable(), &route.src_ip, &target_ip);
    udp_packet.set_checksum(udp_checksum);

    frame
}

pub(crate) fn parse_udp_response(frame: &[u8]) -> Option<UdpResponse> {
    let ethernet = EthernetPacket::new(frame)?;
    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(ethernet.payload())?;
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
        return None;
    }

    let udp = UdpPacket::new(ipv4.payload())?;
    Some(UdpResponse {
        source_ip: ipv4.get_source(),
        destination_ip: ipv4.get_destination(),
        source_port: udp.get_source(),
        destination_port: udp.get_destination(),
        ttl: ipv4.get_ttl(),
    })
}

pub(crate) fn parse_icmp_udp_unreachable(frame: &[u8]) -> Option<UdpIcmpUnreachable> {
    let ethernet = EthernetPacket::new(frame)?;
    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(ethernet.payload())?;
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Icmp {
        return None;
    }

    let icmp = IcmpPacket::new(ipv4.payload())?;
    if icmp.get_icmp_type() != IcmpTypes::DestinationUnreachable {
        return None;
    }
    let code = icmp.get_icmp_code().0;
    if code != 3 {
        return None;
    }

    let icmp_payload = icmp.payload();
    if icmp_payload.len() < 4 {
        return None;
    }

    let embedded_ip = Ipv4Packet::new(&icmp_payload[4..])?;
    if embedded_ip.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
        return None;
    }

    let embedded_udp = UdpPacket::new(embedded_ip.payload())?;
    Some(UdpIcmpUnreachable {
        source_ip: ipv4.get_source(),
        destination_ip: ipv4.get_destination(),
        embedded_source_ip: embedded_ip.get_source(),
        embedded_destination_ip: embedded_ip.get_destination(),
        embedded_source_port: embedded_udp.get_source(),
        embedded_destination_port: embedded_udp.get_destination(),
        code,
        ttl: ipv4.get_ttl(),
    })
}

pub(crate) fn udp_probe_payload(port: u16) -> &'static [u8] {
    match port {
        53 => DNS_UDP_PROBE_PAYLOAD,
        123 => NTP_UDP_PROBE_PAYLOAD,
        _ => GENERIC_UDP_PROBE_PAYLOAD,
    }
}
