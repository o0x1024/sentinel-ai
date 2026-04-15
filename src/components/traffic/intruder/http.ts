import type {
  IntruderAttackOptions,
  IntruderRequestInput,
  IntruderTarget,
  ParsedHttpRequest,
  ParsedHttpResponse,
} from './types'
import type { HttpEndpoint, HttpHeaderEntry } from '@/components/traffic/http/model'
import {
  buildHttpExchangeRequestFromRawRequest,
  createRawRequestFromHttpExchangeRequest,
  ensureRawRequestTerminator,
  normalizeRequestLineEndings,
  parseRawHttpRequest as parseStructuredRawHttpRequest,
} from '@/components/traffic/http/parser'
import { findHeaderValue, headerEntriesToRecord } from '@/components/traffic/http/headers'
import { buildAbsoluteUrl, buildHostHeaderValue } from '@/components/traffic/http/url'

export function createIntruderId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export { normalizeRequestLineEndings, ensureRawRequestTerminator }

export function parseRawHttpRequest(rawRequest: string): ParsedHttpRequest | null {
  const parsed = parseStructuredRawHttpRequest(rawRequest)
  if (!parsed) return null

  return {
    method: parsed.method,
    path: parsed.target,
    version: parsed.version,
    headers: parsed.headers,
    bodyText: parsed.bodyText,
  }
}

export function parseRawHttpResponse(rawResponse: string, responseTimeMs: number): ParsedHttpResponse | null {
  if (!rawResponse) return null

  let headerEnd = rawResponse.indexOf('\r\n\r\n')
  let separatorLength = 4

  if (headerEnd === -1) {
    headerEnd = rawResponse.indexOf('\n\n')
    separatorLength = 2
  }

  if (headerEnd === -1) {
    return null
  }

  const headerPart = rawResponse.slice(0, headerEnd)
  const body = rawResponse.slice(headerEnd + separatorLength)
  const lines = headerPart.split(/\r\n|\r|\n/)
  const statusCodeMatch = lines[0]?.match(/HTTP\/[\d.]+\s+(\d+)/)
  const headers: HttpHeaderEntry[] = []

  for (const line of lines.slice(1)) {
    const colonIndex = line.indexOf(':')
    if (colonIndex <= 0) continue
    headers.push({
      name: line.slice(0, colonIndex).trim(),
      value: line.slice(colonIndex + 1).trim(),
    })
  }

  return {
    statusCode: statusCodeMatch ? Number.parseInt(statusCodeMatch[1], 10) : 0,
    headers,
    body,
    responseTimeMs,
  }
}

function buildEndpointFromTarget(target: IntruderTarget): HttpEndpoint {
  return {
    scheme: target.useTls ? 'https' : 'http',
    host: target.host,
    port: target.port,
  }
}

export function extractTargetFromRequest(rawRequest: string, fallbackUrl?: string): IntruderTarget {
  const parsed = parseStructuredRawHttpRequest(rawRequest)
  const hostHeader = findHeaderValue(parsed?.headers ?? [], 'host')

  if (hostHeader) {
    const [host, portText] = hostHeader.split(':')
    const port = portText ? Number.parseInt(portText, 10) : 443

    return {
      host,
      port: Number.isFinite(port) ? port : 443,
      useTls: !portText || port === 443,
    }
  }

  if (fallbackUrl) {
    try {
      const url = new URL(fallbackUrl)
      return {
        host: url.hostname,
        port: url.port ? Number.parseInt(url.port, 10) : url.protocol === 'https:' ? 443 : 80,
        useTls: url.protocol === 'https:',
      }
    } catch {
      return { host: '', port: 443, useTls: true }
    }
  }

  return { host: '', port: 443, useTls: true }
}

export function createRawRequestFromSource(request?: IntruderRequestInput): string {
  return createRawRequestFromHttpExchangeRequest(request)
}

export function buildFullUrl(rawRequest: string, target: IntruderTarget): string {
  const parsed = parseStructuredRawHttpRequest(rawRequest)
  if (!parsed) return ''
  return buildAbsoluteUrl(buildEndpointFromTarget(target), parsed.target)
}

export function buildSourceRequestFromRawRequest(
  rawRequest: string,
  target: IntruderTarget,
): IntruderRequestInput | null {
  return buildHttpExchangeRequestFromRawRequest(rawRequest, buildEndpointFromTarget(target))
}

export function applyIntruderRequestSettings(
  rawRequest: string,
  target: IntruderTarget,
  options: Pick<
    IntruderAttackOptions,
    'updateHostHeader' | 'updateContentLength' | 'setConnectionClose'
  >,
): string {
  const normalized = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const separatorIndex = normalized.indexOf('\n\n')
  const headerPart = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const body = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const lines = headerPart.split('\n')

  if (!lines.length) {
    return ensureRawRequestTerminator(rawRequest)
  }

  const requestLine = lines[0]
  const parsed = parseStructuredRawHttpRequest(rawRequest)
  const existingHeaders = parsed?.headers ?? []
  let updatedHeaders = [...existingHeaders]

  if (options.updateHostHeader) {
    const hostHeaderValue = buildHostHeaderValue(buildEndpointFromTarget(target))
    updatedHeaders = upsertHeader(updatedHeaders, 'Host', hostHeaderValue)
  }

  if (options.updateContentLength) {
    if (body.length > 0) {
      updatedHeaders = upsertHeader(
        updatedHeaders,
        'Content-Length',
        new TextEncoder().encode(body).length.toString(),
      )
    } else {
      updatedHeaders = updatedHeaders.filter((header) => header.name.toLowerCase() !== 'content-length')
    }
  }

  if (options.setConnectionClose) {
    updatedHeaders = upsertHeader(updatedHeaders, 'Connection', 'close')
  }

  const headerLines = updatedHeaders.map((header) => `${header.name}: ${header.value}`)
  return ensureRawRequestTerminator([requestLine, ...headerLines, '', body].join('\n'))
}

function upsertHeader(headers: HttpHeaderEntry[], name: string, value: string): HttpHeaderEntry[] {
  const lowerName = name.toLowerCase()
  const nextHeaders = headers.map((header) => ({ ...header }))
  const existingIndex = nextHeaders.findIndex((header) => header.name.toLowerCase() === lowerName)

  if (existingIndex >= 0) {
    nextHeaders[existingIndex] = {
      name: nextHeaders[existingIndex].name,
      value,
    }
    return nextHeaders
  }

  nextHeaders.push({ name, value })
  return nextHeaders
}

export function headersToRecord(headers: HttpHeaderEntry[]): Record<string, string> {
  return headerEntriesToRecord(headers)
}

export function countWords(text: string): number {
  const trimmed = text.trim()
  return trimmed ? trimmed.split(/\s+/).length : 0
}

export function countLines(text: string): number {
  if (!text) return 0
  return text.split(/\r\n|\r|\n/).length
}
