import { decodeUnicodeEscapes, encodeUnicodeEscapes } from '@/utils/unicodeEscapes'
import type { TrafficContextMenuActionItem } from './trafficContextMenuSectionSupport'
import { buildTrafficContextSubmenu, type TrafficContextSubmenu } from './trafficContextSubmenuSupport'

export type TrafficTextCodec = 'base64' | 'url' | 'hex' | 'unicode'
export type TrafficTextCodecDirection = 'encode' | 'decode'

export interface TrafficTextSelection {
  from: number
  to: number
}

export interface TrafficTextCodecAction {
  key: string
  codec: TrafficTextCodec
  direction: TrafficTextCodecDirection
  labelKey: string
  iconClass: string
}

const UTF8_DECODER = new TextDecoder('utf-8', { fatal: true })
const UTF8_ENCODER = new TextEncoder()
const BYTE_STRING_CHUNK_SIZE = 0x8000

export const TRAFFIC_TEXT_CODEC_ACTIONS: TrafficTextCodecAction[] = [
  { key: 'base64Encode', codec: 'base64', direction: 'encode', labelKey: 'base64Encode', iconClass: 'fas fa-lock text-info' },
  { key: 'base64Decode', codec: 'base64', direction: 'decode', labelKey: 'base64Decode', iconClass: 'fas fa-lock-open text-info' },
  { key: 'urlEncode', codec: 'url', direction: 'encode', labelKey: 'urlEncode', iconClass: 'fas fa-link text-primary' },
  { key: 'urlDecode', codec: 'url', direction: 'decode', labelKey: 'urlDecode', iconClass: 'fas fa-unlink text-primary' },
  { key: 'hexEncode', codec: 'hex', direction: 'encode', labelKey: 'hexEncode', iconClass: 'fas fa-hashtag text-warning' },
  { key: 'hexDecode', codec: 'hex', direction: 'decode', labelKey: 'hexDecode', iconClass: 'fas fa-hashtag text-warning' },
  { key: 'unicodeEncode', codec: 'unicode', direction: 'encode', labelKey: 'unicodeEncode', iconClass: 'fas fa-language text-accent' },
  { key: 'unicodeDecode', codec: 'unicode', direction: 'decode', labelKey: 'unicodeDecode', iconClass: 'fas fa-language text-accent' },
]

function bytesToBinaryString(bytes: Uint8Array): string {
  const chunks: string[] = []
  for (let index = 0; index < bytes.length; index += BYTE_STRING_CHUNK_SIZE) {
    chunks.push(String.fromCharCode(...bytes.subarray(index, index + BYTE_STRING_CHUNK_SIZE)))
  }
  return chunks.join('')
}

function encodeBase64(input: string): string {
  return btoa(bytesToBinaryString(UTF8_ENCODER.encode(input)))
}

function decodeBase64(input: string): string {
  const normalized = input.replace(/\s+/g, '')
  const binary = atob(normalized)
  const bytes = Uint8Array.from(binary, char => char.charCodeAt(0))
  return UTF8_DECODER.decode(bytes)
}

function encodeHex(input: string): string {
  return Array.from(UTF8_ENCODER.encode(input))
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('')
}

function decodeHex(input: string): string {
  const normalized = input.replace(/\s+/g, '')
  if (normalized.length % 2 !== 0 || /[^0-9a-fA-F]/.test(normalized)) {
    throw new Error('Invalid Hex input')
  }

  const bytes = new Uint8Array(normalized.length / 2)
  for (let index = 0; index < normalized.length; index += 2) {
    bytes[index / 2] = parseInt(normalized.slice(index, index + 2), 16)
  }
  return UTF8_DECODER.decode(bytes)
}

export function transformTrafficTextCodec(input: string, action: TrafficTextCodecAction): string {
  if (action.codec === 'base64') {
    return action.direction === 'encode' ? encodeBase64(input) : decodeBase64(input)
  }
  if (action.codec === 'url') {
    return action.direction === 'encode'
      ? encodeURIComponent(input)
      : decodeURIComponent(input.replace(/\+/g, ' '))
  }
  if (action.codec === 'hex') {
    return action.direction === 'encode' ? encodeHex(input) : decodeHex(input)
  }
  return action.direction === 'encode' ? encodeUnicodeEscapes(input) : decodeUnicodeEscapes(input)
}

export function hasNonEmptyTextSelection(selection: TrafficTextSelection | null | undefined): selection is TrafficTextSelection {
  return Boolean(selection && selection.to > selection.from)
}

export function replaceTrafficTextSelection(
  content: string,
  selection: TrafficTextSelection | null | undefined,
  replacement: string,
) {
  const from = Math.max(0, Math.min(selection?.from ?? content.length, content.length))
  const to = Math.max(from, Math.min(selection?.to ?? from, content.length))

  return {
    content: `${content.slice(0, from)}${replacement}${content.slice(to)}`,
    selectionStart: from,
    selectionEnd: from + replacement.length,
  }
}

export function getTrafficTextCodecErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function buildTrafficTextCodecMenuItems(options: {
  disabled?: boolean
  onClick: (action: TrafficTextCodecAction) => void | Promise<void>
}): TrafficContextMenuActionItem[] {
  return TRAFFIC_TEXT_CODEC_ACTIONS.map((action) => ({
    key: action.key,
    iconClass: action.iconClass,
    labelKey: action.labelKey,
    disabled: options.disabled,
    onClick: () => options.onClick(action),
  }))
}

export function buildTrafficTextCodecSubmenu(options: {
  disabled?: boolean
  onClick: (action: TrafficTextCodecAction) => void | Promise<void>
  submenuClass?: string
}): TrafficContextSubmenu | null {
  return buildTrafficContextSubmenu({
    key: 'text-codec',
    triggerLabelKey: 'codec',
    triggerIconClass: 'fas fa-code text-accent',
    submenuClass: options.submenuClass,
    items: buildTrafficTextCodecMenuItems(options),
  })
}

export function splitTrafficMenuItemsAfterKey<T extends { key: string }>(items: T[], key: string) {
  const index = items.findIndex(item => item.key === key)
  if (index < 0) {
    return { before: items, after: [] }
  }

  return {
    before: items.slice(0, index + 1),
    after: items.slice(index + 1),
  }
}
