export interface NetworkInterface {
  name: string
  description?: string
  mac?: string
  ipv4?: string
}

export interface ProtocolField {
  name: string
  value: string
  children?: ProtocolField[]
}

export interface ProtocolLayer {
  name: string
  display: string
  fields: ProtocolField[]
}

export interface Packet {
  id: number
  timestamp: number
  src: string
  dst: string
  protocol: string
  length: number
  info: string
  layers: ProtocolLayer[]
  raw: number[]
}

export interface PacketSummary {
  id: number
  timestamp: number
  src: string
  dst: string
  protocol: string
  length: number
  info: string
}

export interface AdvancedFilter {
  protocols: string[]
  srcIp: string
  dstIp: string
  srcPort: string
  dstPort: string
  containsString: string
  containsHex: string
  minLength: number | null
  maxLength: number | null
  tcpFlags: string[]
}

export interface ExtractedFileInfo {
  id: string
  filename: string
  content_type: string
  size: number
  src: string
  dst: string
  packet_ids: number[]
  stream_key: string
  source_type: string
}

export interface VirtualItem<T = PacketSummary> {
  data: T
  offset: number
}

export interface StreamSegment {
  isClient: boolean
  content: string
  packetId: number
}
