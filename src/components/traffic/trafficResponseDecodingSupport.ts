import type { HttpReplayResponse } from './http/model'
import { normalizeHeaderEntries, parseStoredHeaderEntries } from './http/headers'
import type { TrafficDisplaySettings } from './trafficDisplaySettings'
import {
  extractBase64BodyPayload,
  getDisplayResponseBody,
  isImageResponseContentType,
} from './trafficResponsePreviewSupport'

const DEFAULT_CHARSET = 'utf-8'
const CHUNK_SIZE = 0x8000

function parseCharsetFromContentType(contentType: string): string | null {
  if (!contentType) return null

  const parts = contentType.split(';').slice(1)
  for (const part of parts) {
    const [name, value] = part.split('=')
    if (name?.trim().toLowerCase() !== 'charset') continue
    const normalized = value?.trim().replace(/^['"]|['"]$/g, '')
    if (normalized) return normalized
  }

  return null
}

function decodeBase64ToBytes(payload?: string | null): Uint8Array | null {
  if (!payload) return null

  try {
    const binary = atob(payload.replace(/\s+/g, ''))
    const bytes = new Uint8Array(binary.length)
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index)
    }
    return bytes
  } catch {
    return null
  }
}

function encodeBytesAsRawByteString(bytes: Uint8Array): string {
  if (!bytes.length) return ''

  const parts: string[] = []
  for (let index = 0; index < bytes.length; index += CHUNK_SIZE) {
    const chunk = bytes.subarray(index, index + CHUNK_SIZE)
    let chunkText = ''
    for (const value of chunk) {
      chunkText += String.fromCharCode(value)
    }
    parts.push(chunkText)
  }

  return parts.join('')
}

function decodeBytesWithCharset(bytes: Uint8Array, charset: string): string {
  try {
    return new TextDecoder(charset).decode(bytes)
  } catch {
    return new TextDecoder(DEFAULT_CHARSET).decode(bytes)
  }
}

function resolveCharset(
  contentType: string,
  settings: TrafficDisplaySettings,
): string {
  switch (settings.charsetMode) {
    case 'platformDefault':
      return DEFAULT_CHARSET
    case 'specific':
      return settings.specificCharset || DEFAULT_CHARSET
    case 'auto':
    default:
      return parseCharsetFromContentType(contentType) || DEFAULT_CHARSET
  }
}

function resolveResponseBytes(
  bodyText: string,
  bodyBytesBase64?: string | null,
): Uint8Array | null {
  return (
    decodeBase64ToBytes(bodyBytesBase64)
    || decodeBase64ToBytes(extractBase64BodyPayload(bodyText))
  )
}

export function resolveTrafficResponseBodyText(
  bodyText: string,
  contentType: string,
  settings: TrafficDisplaySettings,
  bodyBytesBase64?: string | null,
): string {
  if (!bodyText && !bodyBytesBase64) {
    return ''
  }

  const bytes = resolveResponseBytes(bodyText, bodyBytesBase64)
  if (!bytes) {
    return getDisplayResponseBody(bodyText, contentType)
  }

  if (isImageResponseContentType(contentType) || settings.charsetMode === 'rawBytes') {
    return encodeBytesAsRawByteString(bytes)
  }

  return decodeBytesWithCharset(bytes, resolveCharset(contentType, settings))
}

export function buildTrafficDisplayedRawResponse(
  response: Pick<HttpReplayResponse, 'statusCode' | 'versionObserved' | 'statusText' | 'headers' | 'bodyText' | 'bodyBytesBase64'>,
  settings: TrafficDisplaySettings,
): string {
  const version = response.versionObserved || 'HTTP/1.1'
  const statusText = response.statusText || ''
  const headers = normalizeHeaderEntries(response.headers || [])
  const contentType = headers.find((header) => header.name.toLowerCase() === 'content-type')?.value || ''
  const bodyText = resolveTrafficResponseBodyText(
    response.bodyText || '',
    contentType,
    settings,
    response.bodyBytesBase64,
  )

  const lines = [
    `${version} ${response.statusCode} ${statusText}`.trimEnd(),
    ...headers.map((header) => `${header.name}: ${header.value}`),
    '',
    bodyText,
  ]

  return lines.join('\r\n')
}

export function getStoredResponseContentType(headers?: string): string {
  return parseStoredHeaderEntries(headers)
    .find((header) => header.name.toLowerCase() === 'content-type')
    ?.value || ''
}

export function resolveStoredTrafficResponseBodyText(
  bodyText: string,
  headers: string | undefined,
  settings: TrafficDisplaySettings,
): string {
  return resolveTrafficResponseBodyText(
    bodyText,
    getStoredResponseContentType(headers),
    settings,
  )
}

export function buildTrafficDisplayedRawResponseFromStored(
  input: {
    statusCode: number
    versionObserved?: string
    statusText?: string
    headers?: string
    bodyText?: string
  },
  settings: TrafficDisplaySettings,
): string {
  const normalizedHeaders = parseStoredHeaderEntries(input.headers)
  const contentType = normalizedHeaders.find((header) => header.name.toLowerCase() === 'content-type')?.value || ''
  const bodyText = resolveTrafficResponseBodyText(
    input.bodyText || '',
    contentType,
    settings,
  )

  const lines = [
    `${input.versionObserved || 'HTTP/1.1'} ${input.statusCode} ${input.statusText || ''}`.trimEnd(),
    ...normalizedHeaders.map((header) => `${header.name}: ${header.value}`),
    '',
    bodyText,
  ]

  return lines.join('\r\n')
}
