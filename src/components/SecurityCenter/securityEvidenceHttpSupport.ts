import { parseStoredHeaderEntries } from '@/components/traffic/http/headers'
import { buildHostHeaderValue, endpointFromUrl } from '@/components/traffic/http/url'
import { DEFAULT_HTTP_VERSION } from '@/components/traffic/http/version'
import { getHarStatusText } from '@/components/traffic/proxyHistoryFormattingSupport'
import type { WorkbenchEvidenceExchange } from './securityWorkbenchSystemAgentContent'

const stringifyStructuredValue = (value: unknown) => {
  if (value == null) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value)
  }

  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

const normalizeLineEndings = (value?: unknown) =>
  stringifyStructuredValue(value).replace(/\r\n/g, '\n').replace(/\r/g, '\n')

const readTrimmedText = (value?: unknown) => normalizeLineEndings(value).trim()

const isAbsoluteHttpUrl = (value?: unknown) => Boolean(readTrimmedText(value) && /^https?:\/\//i.test(readTrimmedText(value)))

const resolveAbsoluteUrl = (requestUrl?: unknown, fallbackUrl?: unknown) => {
  const normalizedRequestUrl = readTrimmedText(requestUrl)
  const normalizedFallbackUrl = readTrimmedText(fallbackUrl)

  if (isAbsoluteHttpUrl(normalizedRequestUrl)) {
    return normalizedRequestUrl
  }

  if (!normalizedRequestUrl || !isAbsoluteHttpUrl(normalizedFallbackUrl)) {
    return ''
  }

  try {
    return new URL(normalizedRequestUrl, normalizedFallbackUrl).toString()
  } catch {
    return ''
  }
}

const toOriginFormTarget = (requestUrl?: unknown, fallbackUrl?: unknown) => {
  const absoluteUrl = resolveAbsoluteUrl(requestUrl, fallbackUrl)
  if (absoluteUrl) {
    const parsed = new URL(absoluteUrl)
    return parsed.pathname + parsed.search || '/'
  }

  const normalizedRequestUrl = readTrimmedText(requestUrl)
  if (!normalizedRequestUrl) return '/'
  if (normalizedRequestUrl.startsWith('/')) return normalizedRequestUrl
  if (normalizedRequestUrl.startsWith('?')) return `/${normalizedRequestUrl}`
  return `/${normalizedRequestUrl}`
}

const resolveRequestHostHeader = (requestUrl?: unknown, fallbackUrl?: unknown) => {
  const absoluteUrl = resolveAbsoluteUrl(requestUrl, fallbackUrl)
  if (!absoluteUrl) return ''

  try {
    return buildHostHeaderValue(endpointFromUrl(absoluteUrl))
  } catch {
    return ''
  }
}

const buildHeaderLines = (
  rawHeaders?: unknown,
  options: { injectHostHeader?: string } = {},
) => {
  const headerLines: string[] = []
  const normalizedRawHeaders = normalizeLineEndings(rawHeaders)
  const parsedHeaders = typeof rawHeaders === 'string'
    ? parseStoredHeaderEntries(rawHeaders || undefined)
    : []

  if (parsedHeaders.length > 0) {
    headerLines.push(...parsedHeaders.map((header) => `${header.name}: ${header.value}`))
  } else if (rawHeaders && typeof rawHeaders === 'object' && !Array.isArray(rawHeaders)) {
    headerLines.push(
      ...Object.entries(rawHeaders as Record<string, unknown>).map(
        ([name, value]) => `${name}: ${stringifyStructuredValue(value)}`,
      ),
    )
  } else if (
    Array.isArray(rawHeaders)
    && rawHeaders.every((entry) => entry && typeof entry === 'object' && 'name' in entry)
  ) {
    headerLines.push(
      ...rawHeaders.map((entry) => {
        const item = entry as { name?: unknown; value?: unknown }
        return `${stringifyStructuredValue(item.name)}: ${stringifyStructuredValue(item.value)}`
      }),
    )
  } else if (normalizedRawHeaders.trim()) {
    headerLines.push(
      ...normalizedRawHeaders
        .split('\n')
        .map((line) => line.trimEnd())
        .filter(Boolean),
    )
  }

  const hasHostHeader = headerLines.some((line) => line.split(':', 1)[0]?.trim().toLowerCase() === 'host')
  if (options.injectHostHeader && !hasHostHeader) {
    headerLines.unshift(`Host: ${options.injectHostHeader}`)
  }

  return headerLines
}

const buildHttpMessage = (startLine: string, headerLines: string[], body?: unknown) => {
  const normalizedBody = normalizeLineEndings(body)
  const head = [startLine, ...headerLines].join('\r\n')
  return normalizedBody ? `${head}\r\n\r\n${normalizedBody.replace(/\n/g, '\r\n')}` : `${head}\r\n\r\n`
}

const getResponseStatusLine = (responseStatus?: number | null) => {
  if (typeof responseStatus !== 'number') {
    return `${DEFAULT_HTTP_VERSION} 000 Unknown`
  }
  return `${DEFAULT_HTTP_VERSION} ${responseStatus} ${getHarStatusText(responseStatus)}`
}

export function buildSecurityEvidenceRawRequest(
  exchange?: WorkbenchEvidenceExchange | null,
  fallbackUrl?: unknown,
): string {
  if (!exchange) return ''

  const method = readTrimmedText(exchange.requestMethod) || 'GET'
  const target = toOriginFormTarget(exchange.requestUrl, fallbackUrl)
  const hostHeader = resolveRequestHostHeader(exchange.requestUrl, fallbackUrl)
  const headerLines = buildHeaderLines(exchange.requestHeaders, { injectHostHeader: hostHeader })
  return buildHttpMessage(`${method} ${target} ${DEFAULT_HTTP_VERSION}`, headerLines, exchange.requestBody)
}

export function buildSecurityEvidenceRawResponse(
  exchange?: WorkbenchEvidenceExchange | null,
): string {
  if (!exchange) return ''

  const hasResponseData =
    typeof exchange.responseStatus === 'number'
    || Boolean(exchange.responseHeaders || exchange.responseBody)

  if (!hasResponseData) return ''

  const headerLines = buildHeaderLines(exchange.responseHeaders)
  return buildHttpMessage(
    getResponseStatusLine(exchange.responseStatus),
    headerLines,
    exchange.responseBody,
  )
}
