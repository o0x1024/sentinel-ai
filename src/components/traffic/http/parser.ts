import type { HttpEndpoint, HttpExchangeRequest, HttpHeaderEntry, HttpVersion } from './model'
import { normalizeHeaderEntries } from './headers'
import { buildAbsoluteUrl, buildHostHeaderValue } from './url'
import { normalizeHttpVersionToken } from './version'

export interface ParsedRawHttpRequest {
  method: string
  target: string
  version: HttpVersion
  headers: HttpHeaderEntry[]
  bodyText: string
}

export function normalizeRequestLineEndings(rawRequest: string): string {
  return rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n')
}

export function ensureRawRequestTerminator(rawRequest: string): string {
  let normalized = normalizeRequestLineEndings(rawRequest)

  if (!normalized.includes('\r\n\r\n')) {
    normalized += '\r\n\r\n'
  } else if (normalized.endsWith('\r\n')) {
    return normalized
  }

  return normalized
}

export function parseRawHttpRequest(rawRequest: string): ParsedRawHttpRequest | null {
  const normalized = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const separatorIndex = normalized.indexOf('\n\n')
  const headerPart = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const bodyText = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const lines = headerPart.split('\n')

  if (!lines[0]) {
    return null
  }

  const [method = 'GET', target = '/', versionToken] = lines[0].trim().split(/\s+/)
  const headers = normalizeHeaderEntries(
    lines.slice(1).flatMap((line) => {
      const colonIndex = line.indexOf(':')
      if (colonIndex <= 0) return []
      return [{
        name: line.slice(0, colonIndex).trim(),
        value: line.slice(colonIndex + 1).trim(),
      }]
    }),
  )

  return {
    method,
    target,
    version: normalizeHttpVersionToken(versionToken) as HttpVersion,
    headers,
    bodyText,
  }
}

export function buildHttpExchangeRequestFromRawRequest(
  rawRequest: string,
  endpoint: HttpEndpoint,
): HttpExchangeRequest | null {
  const parsed = parseRawHttpRequest(rawRequest)
  if (!parsed) return null

  return {
    endpoint,
    absoluteUrl: buildAbsoluteUrl(endpoint, parsed.target),
    request: {
      method: parsed.method,
      target: parsed.target,
      versionPreference: parsed.version,
      headers: parsed.headers,
      bodyText: parsed.bodyText,
    },
  }
}

export function createRawRequestFromHttpExchangeRequest(exchange?: HttpExchangeRequest): string {
  if (!exchange) {
    return 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n'
  }

  const target = exchange.request.target.startsWith('/')
    || exchange.request.target.startsWith('http://')
    || exchange.request.target.startsWith('https://')
    ? exchange.request.target
    : `/${exchange.request.target}`

  const headers = normalizeHeaderEntries(exchange.request.headers)
  const hasHost = headers.some((header) => header.name.toLowerCase() === 'host')
  const lines = [
    `${exchange.request.method || 'GET'} ${target || '/'} ${exchange.request.versionPreference === 'AUTO' ? 'HTTP/1.1' : exchange.request.versionPreference}`,
    ...(hasHost ? [] : [`Host: ${buildHostHeaderValue(exchange.endpoint)}`]),
    ...headers.map((header) => `${header.name}: ${header.value}`),
    '',
    exchange.request.bodyText ?? '',
  ]

  return ensureRawRequestTerminator(lines.join('\r\n'))
}
