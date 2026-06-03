import type { Packet } from './packetCaptureTypes'

export const MAX_PACKET_BUFFER_SIZE = 10000
export const TRIMMED_PACKET_BUFFER_SIZE = 8000

const packetAsciiTextCache = new WeakMap<Packet, string>()
const packetHexTextCache = new WeakMap<Packet, string>()

const timeLabelCache = new Map<number, string>()
const packetTimeFormatter = new Intl.DateTimeFormat('zh-CN', {
  hour12: false,
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  fractionalSecondDigits: 3,
} as Intl.DateTimeFormatOptions)

export function appendPacketsWithLimit<T extends { id: number }>(
  target: T[],
  incoming: T[],
  markedPackets: Set<number>,
  ignoredPackets: Set<number>,
  maxPackets = MAX_PACKET_BUFFER_SIZE,
  trimTo = TRIMMED_PACKET_BUFFER_SIZE,
): Set<number> {
  if (incoming.length === 0) {
    return new Set<number>()
  }

  target.push(...incoming)

  if (target.length <= maxPackets) {
    return new Set<number>()
  }

  const removeCount = Math.max(0, target.length - trimTo)
  const removedIds = new Set(target.slice(0, removeCount).map((packet) => packet.id))

  target.splice(0, removeCount)

  removedIds.forEach((id) => {
    markedPackets.delete(id)
    ignoredPackets.delete(id)
  })

  return removedIds
}

export function getPacketAsciiText(packet: Packet): string {
  const cached = packetAsciiTextCache.get(packet)
  if (cached) {
    return cached
  }

  const value = packet.raw
    .map((byte) => (byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : ''))
    .join('')
    .toLowerCase()

  packetAsciiTextCache.set(packet, value)
  return value
}

export function getPacketHexText(packet: Packet): string {
  const cached = packetHexTextCache.get(packet)
  if (cached) {
    return cached
  }

  const value = packet.raw.map((byte) => byte.toString(16).padStart(2, '0')).join('')
  packetHexTextCache.set(packet, value)
  return value
}

export function formatPacketTime(timestamp: number): string {
  const cached = timeLabelCache.get(timestamp)
  if (cached) {
    return cached
  }

  if (timeLabelCache.size > 20000) {
    timeLabelCache.clear()
  }

  const value = packetTimeFormatter.format(new Date(timestamp))
  timeLabelCache.set(timestamp, value)
  return value
}
