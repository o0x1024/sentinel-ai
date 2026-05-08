use std::net::Ipv4Addr;

use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Flags, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::tcp::{ipv4_checksum, MutableTcpPacket, TcpPacket};
use pnet::packet::{MutablePacket, Packet};
use rsubdomain::EthTable;

const ETHERNET_HEADER_LEN: usize = 14;
const IPV4_HEADER_LEN: usize = 20;
const TCP_HEADER_LEN: usize = 20;
const RAW_FRAME_LEN: usize = ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + TCP_HEADER_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TcpResponse {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: u16,
    pub destination_port: u16,
    pub flags: u8,
    pub sequence: u32,
    pub acknowledgement: u32,
    pub ttl: u8,
}

pub(crate) fn build_tcp_frame(
    route: &EthTable,
    target_ip: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    sequence: u32,
    acknowledgement: u32,
    flags: u8,
    ttl: u8,
) -> [u8; RAW_FRAME_LEN] {
    let mut frame = [0u8; RAW_FRAME_LEN];

    let mut ethernet_packet =
        MutableEthernetPacket::new(&mut frame[..]).expect("ethernet frame buffer should fit");
    ethernet_packet.set_source(route.src_mac);
    ethernet_packet.set_destination(route.dst_mac);
    ethernet_packet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut())
        .expect("ipv4 payload should fit inside ethernet frame");
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4_packet.set_total_length((IPV4_HEADER_LEN + TCP_HEADER_LEN) as u16);
    ipv4_packet.set_identification(sequence as u16);
    ipv4_packet.set_ttl(ttl.max(1));
    ipv4_packet.set_flags(Ipv4Flags::DontFragment);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(route.src_ip);
    ipv4_packet.set_destination(target_ip);
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ip_checksum);

    let mut tcp_packet = MutableTcpPacket::new(ipv4_packet.payload_mut())
        .expect("tcp payload should fit inside ipv4 frame");
    tcp_packet.set_source(source_port);
    tcp_packet.set_destination(destination_port);
    tcp_packet.set_sequence(sequence);
    tcp_packet.set_acknowledgement(acknowledgement);
    tcp_packet.set_data_offset((TCP_HEADER_LEN / 4) as u8);
    tcp_packet.set_flags(flags);
    tcp_packet.set_window(64_240);
    tcp_packet.set_urgent_ptr(0);
    let tcp_checksum = ipv4_checksum(&tcp_packet.to_immutable(), &route.src_ip, &target_ip);
    tcp_packet.set_checksum(tcp_checksum);

    frame
}

pub(crate) fn parse_tcp_response(frame: &[u8]) -> Option<TcpResponse> {
    let ethernet = EthernetPacket::new(frame)?;
    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(ethernet.payload())?;
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Tcp {
        return None;
    }

    let tcp = TcpPacket::new(ipv4.payload())?;
    Some(TcpResponse {
        source_ip: ipv4.get_source(),
        destination_ip: ipv4.get_destination(),
        source_port: tcp.get_source(),
        destination_port: tcp.get_destination(),
        flags: tcp.get_flags(),
        sequence: tcp.get_sequence(),
        acknowledgement: tcp.get_acknowledgement(),
        ttl: ipv4.get_ttl(),
    })
}
