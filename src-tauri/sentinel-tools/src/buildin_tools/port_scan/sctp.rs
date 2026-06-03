use std::net::Ipv4Addr;

use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::{Ipv4Flags, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{MutablePacket, Packet};
use rsubdomain::EthTable;

const CRC32C_TABLE: [u32; 256] = [
    0x00000000, 0xF26B8303, 0xE13B70F7, 0x1350F3F4, 0xC79A971F, 0x35F1141C, 0x26A1E7E8, 0xD4CA64EB,
    0x8AD958CF, 0x78B2DBCC, 0x6BE22838, 0x9989AB3B, 0x4D43CFD0, 0xBF284CD3, 0xAC78BF27, 0x5E133C24,
    0x105EC76F, 0xE235446C, 0xF165B798, 0x030E349B, 0xD7C45070, 0x25AFD373, 0x36FF2087, 0xC494A384,
    0x9A879FA0, 0x68EC1CA3, 0x7BBCEF57, 0x89D76C54, 0x5D1D08BF, 0xAF768BBC, 0xBC267848, 0x4E4DFB4B,
    0x20BD8EDE, 0xD2D60DDD, 0xC186FE29, 0x33ED7D2A, 0xE72719C1, 0x154C9AC2, 0x061C6936, 0xF477EA35,
    0xAA64D611, 0x580F5512, 0x4B5FA6E6, 0xB93425E5, 0x6DFE410E, 0x9F95C20D, 0x8CC531F9, 0x7EAEB2FA,
    0x30E349B1, 0xC288CAB2, 0xD1D83946, 0x23B3BA45, 0xF779DEAE, 0x05125DAD, 0x1642AE59, 0xE4292D5A,
    0xBA3A117E, 0x4851927D, 0x5B016189, 0xA96AE28A, 0x7DA08661, 0x8FCB0562, 0x9C9BF696, 0x6EF07595,
    0x417B1DBC, 0xB3109EBF, 0xA0406D4B, 0x522BEE48, 0x86E18AA3, 0x748A09A0, 0x67DAFA54, 0x95B17957,
    0xCBA24573, 0x39C9C670, 0x2A993584, 0xD8F2B687, 0x0C38D26C, 0xFE53516F, 0xED03A29B, 0x1F682198,
    0x5125DAD3, 0xA34E59D0, 0xB01EAA24, 0x42752927, 0x96BF4DCC, 0x64D4CECF, 0x77843D3B, 0x85EFBE38,
    0xDBFC821C, 0x2997011F, 0x3AC7F2EB, 0xC8AC71E8, 0x1C661503, 0xEE0D9600, 0xFD5D65F4, 0x0F36E6F7,
    0x61C69362, 0x93AD1061, 0x80FDE395, 0x72966096, 0xA65C047D, 0x5437877E, 0x4767748A, 0xB50CF789,
    0xEB1FCBAD, 0x197448AE, 0x0A24BB5A, 0xF84F3859, 0x2C855CB2, 0xDEEEDFB1, 0xCDBE2C45, 0x3FD5AF46,
    0x7198540D, 0x83F3D70E, 0x90A324FA, 0x62C8A7F9, 0xB602C312, 0x44694011, 0x5739B3E5, 0xA55230E6,
    0xFB410CC2, 0x092A8FC1, 0x1A7A7C35, 0xE811FF36, 0x3CDB9BDD, 0xCEB018DE, 0xDDE0EB2A, 0x2F8B6829,
    0x82F63B78, 0x709DB87B, 0x63CD4B8F, 0x91A6C88C, 0x456CAC67, 0xB7072F64, 0xA457DC90, 0x563C5F93,
    0x082F63B7, 0xFA44E0B4, 0xE9141340, 0x1B7F9043, 0xCFB5F4A8, 0x3DDE77AB, 0x2E8E845F, 0xDCE5075C,
    0x92A8FC17, 0x60C37F14, 0x73938CE0, 0x81F80FE3, 0x55326B08, 0xA759E80B, 0xB4091BFF, 0x466298FC,
    0x1871A4D8, 0xEA1A27DB, 0xF94AD42F, 0x0B21572C, 0xDFEB33C7, 0x2D80B0C4, 0x3ED04330, 0xCCBBC033,
    0xA24BB5A6, 0x502036A5, 0x4370C551, 0xB11B4652, 0x65D122B9, 0x97BAA1BA, 0x84EA524E, 0x7681D14D,
    0x2892ED69, 0xDAF96E6A, 0xC9A99D9E, 0x3BC21E9D, 0xEF087A76, 0x1D63F975, 0x0E330A81, 0xFC588982,
    0xB21572C9, 0x407EF1CA, 0x532E023E, 0xA145813D, 0x758FE5D6, 0x87E466D5, 0x94B49521, 0x66DF1622,
    0x38CC2A06, 0xCAA7A905, 0xD9F75AF1, 0x2B9CD9F2, 0xFF56BD19, 0x0D3D3E1A, 0x1E6DCDEE, 0xEC064EED,
    0xC38D26C4, 0x31E6A5C7, 0x22B65633, 0xD0DDD530, 0x0417B1DB, 0xF67C32D8, 0xE52CC12C, 0x1747422F,
    0x49547E0B, 0xBB3FFD08, 0xA86F0EFC, 0x5A048DFF, 0x8ECEE914, 0x7CA56A17, 0x6FF599E3, 0x9D9E1AE0,
    0xD3D3E1AB, 0x21B862A8, 0x32E8915C, 0xC083125F, 0x144976B4, 0xE622F5B7, 0xF5720643, 0x07198540,
    0x590AB964, 0xAB613A67, 0xB831C993, 0x4A5A4A90, 0x9E902E7B, 0x6CFBAD78, 0x7FAB5E8C, 0x8DC0DD8F,
    0xE330A81A, 0x115B2B19, 0x020BD8ED, 0xF0605BEE, 0x24AA3F05, 0xD6C1BC06, 0xC5914FF2, 0x37FACCF1,
    0x69E9F0D5, 0x9B8273D6, 0x88D28022, 0x7AB90321, 0xAE7367CA, 0x5C18E4C9, 0x4F48173D, 0xBD23943E,
    0xF36E6F75, 0x0105EC76, 0x12551F82, 0xE03E9C81, 0x34F4F86A, 0xC69F7B69, 0xD5CF889D, 0x27A40B9E,
    0x79B737BA, 0x8BDCB4B9, 0x988C474D, 0x6AE7C44E, 0xBE2DA0A5, 0x4C4623A6, 0x5F16D052, 0xAD7D5351,
];

const ETHERNET_HEADER_LEN: usize = 14;
const IPV4_HEADER_LEN: usize = 20;
const SCTP_COMMON_HEADER_LEN: usize = 12;
const SCTP_INIT_CHUNK_LEN: usize = 20;
const SCTP_PACKET_LEN: usize = SCTP_COMMON_HEADER_LEN + SCTP_INIT_CHUNK_LEN;
const SCTP_RECEIVER_WINDOW: u32 = 0x0000_8000;
const SCTP_OUTBOUND_STREAMS: u16 = 10;
const SCTP_INBOUND_STREAMS: u16 = 2048;

pub(crate) const SCTP_NEXT_HEADER: IpNextHeaderProtocol = IpNextHeaderProtocol(132);
pub(crate) const SCTP_CHUNK_TYPE_INIT: u8 = 1;
pub(crate) const SCTP_CHUNK_TYPE_INIT_ACK: u8 = 2;
pub(crate) const SCTP_CHUNK_TYPE_ABORT: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SctpResponse {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: u16,
    pub destination_port: u16,
    pub verification_tag: u32,
    pub ttl: u8,
    pub first_chunk_type: u8,
    pub first_chunk_flags: u8,
    pub first_chunk_length: u16,
    pub first_chunk_initiate_tag: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SctpIcmpUnreachable {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub embedded_source_ip: Ipv4Addr,
    pub embedded_destination_ip: Ipv4Addr,
    pub embedded_source_port: u16,
    pub embedded_destination_port: u16,
    pub code: u8,
    pub ttl: u8,
}

pub(crate) fn build_sctp_init_frame(
    route: &EthTable,
    target_ip: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    initiate_tag: u32,
    ttl: u8,
) -> Vec<u8> {
    let total_len = ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + SCTP_PACKET_LEN;
    let mut frame = vec![0u8; total_len];

    let mut ethernet_packet =
        MutableEthernetPacket::new(&mut frame).expect("ethernet frame buffer should fit");
    ethernet_packet.set_source(route.src_mac);
    ethernet_packet.set_destination(route.dst_mac);
    ethernet_packet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut())
        .expect("ipv4 payload should fit inside ethernet frame");
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length((IPV4_HEADER_LEN / 4) as u8);
    ipv4_packet.set_total_length((IPV4_HEADER_LEN + SCTP_PACKET_LEN) as u16);
    ipv4_packet.set_identification(source_port ^ destination_port);
    ipv4_packet.set_ttl(ttl.max(1));
    ipv4_packet.set_flags(Ipv4Flags::DontFragment);
    ipv4_packet.set_next_level_protocol(SCTP_NEXT_HEADER);
    ipv4_packet.set_source(route.src_ip);
    ipv4_packet.set_destination(target_ip);
    let ip_checksum = pnet::packet::ipv4::checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ip_checksum);

    let payload = ipv4_packet.payload_mut();
    payload[0..2].copy_from_slice(&source_port.to_be_bytes());
    payload[2..4].copy_from_slice(&destination_port.to_be_bytes());
    payload[4..8].fill(0);
    payload[8..12].fill(0);
    payload[12] = SCTP_CHUNK_TYPE_INIT;
    payload[13] = 0;
    payload[14..16].copy_from_slice(&(SCTP_INIT_CHUNK_LEN as u16).to_be_bytes());
    payload[16..20].copy_from_slice(&initiate_tag.to_be_bytes());
    payload[20..24].copy_from_slice(&SCTP_RECEIVER_WINDOW.to_be_bytes());
    payload[24..26].copy_from_slice(&SCTP_OUTBOUND_STREAMS.to_be_bytes());
    payload[26..28].copy_from_slice(&SCTP_INBOUND_STREAMS.to_be_bytes());
    payload[28..32].copy_from_slice(&initial_tsn(initiate_tag).to_be_bytes());

    let checksum = sctp_checksum(payload);
    payload[8..12].copy_from_slice(&checksum.to_be_bytes());

    frame
}

pub(crate) fn parse_sctp_response(frame: &[u8]) -> Option<SctpResponse> {
    let ethernet = EthernetPacket::new(frame)?;
    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(ethernet.payload())?;
    if ipv4.get_next_level_protocol() != SCTP_NEXT_HEADER {
        return None;
    }

    let payload = ipv4.payload();
    if payload.len() < 16 {
        return None;
    }

    let first_chunk_length = u16::from_be_bytes([payload[14], payload[15]]);
    if first_chunk_length < 4
        || SCTP_COMMON_HEADER_LEN + first_chunk_length as usize > payload.len()
    {
        return None;
    }

    let first_chunk_initiate_tag = (payload[12] == SCTP_CHUNK_TYPE_INIT_ACK
        && first_chunk_length as usize >= SCTP_INIT_CHUNK_LEN)
        .then(|| u32::from_be_bytes([payload[16], payload[17], payload[18], payload[19]]));

    Some(SctpResponse {
        source_ip: ipv4.get_source(),
        destination_ip: ipv4.get_destination(),
        source_port: u16::from_be_bytes([payload[0], payload[1]]),
        destination_port: u16::from_be_bytes([payload[2], payload[3]]),
        verification_tag: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
        ttl: ipv4.get_ttl(),
        first_chunk_type: payload[12],
        first_chunk_flags: payload[13],
        first_chunk_length,
        first_chunk_initiate_tag,
    })
}

pub(crate) fn parse_icmp_sctp_unreachable(frame: &[u8]) -> Option<SctpIcmpUnreachable> {
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
    if code != 2 && code != 3 {
        return None;
    }

    let icmp_payload = icmp.payload();
    if icmp_payload.len() < 4 {
        return None;
    }

    let embedded_ip = Ipv4Packet::new(&icmp_payload[4..])?;
    if embedded_ip.get_next_level_protocol() != SCTP_NEXT_HEADER {
        return None;
    }

    let embedded_payload = embedded_ip.payload();
    if embedded_payload.len() < 4 {
        return None;
    }

    Some(SctpIcmpUnreachable {
        source_ip: ipv4.get_source(),
        destination_ip: ipv4.get_destination(),
        embedded_source_ip: embedded_ip.get_source(),
        embedded_destination_ip: embedded_ip.get_destination(),
        embedded_source_port: u16::from_be_bytes([embedded_payload[0], embedded_payload[1]]),
        embedded_destination_port: u16::from_be_bytes([embedded_payload[2], embedded_payload[3]]),
        code,
        ttl: ipv4.get_ttl(),
    })
}

pub(crate) fn sctp_checksum(packet: &[u8]) -> u32 {
    if packet.len() < SCTP_COMMON_HEADER_LEN {
        return 0;
    }

    let mut crc32 = !0u32;
    for byte in &packet[..8] {
        crc32 = (crc32 >> 8) ^ CRC32C_TABLE[((crc32 ^ u32::from(*byte)) & 0xff) as usize];
    }
    for _ in 0..4 {
        crc32 = (crc32 >> 8) ^ CRC32C_TABLE[(crc32 & 0xff) as usize];
    }
    for byte in &packet[12..] {
        crc32 = (crc32 >> 8) ^ CRC32C_TABLE[((crc32 ^ u32::from(*byte)) & 0xff) as usize];
    }

    let result = !crc32;
    let byte0 = result as u8;
    let byte1 = (result >> 8) as u8;
    let byte2 = (result >> 16) as u8;
    let byte3 = (result >> 24) as u8;
    u32::from_be_bytes([byte0, byte1, byte2, byte3])
}

fn initial_tsn(initiate_tag: u32) -> u32 {
    initiate_tag.rotate_left(7) ^ 0x461A_DF3D
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_masscan_selftest() {
        let testcase: [u8; 32] = [
            0xd1, 0x60, 0x00, 0x50, 0x00, 0x00, 0x00, 0x00, 0x58, 0xe4, 0x5d, 0x36, 0x01, 0x00,
            0x00, 0x14, 0x9e, 0x8d, 0x52, 0x25, 0x00, 0x00, 0x80, 0x00, 0x00, 0x0a, 0x08, 0x00,
            0x46, 0x1a, 0xdf, 0x3d,
        ];

        assert_eq!(sctp_checksum(&testcase), 0x58e45d36);
    }
}
