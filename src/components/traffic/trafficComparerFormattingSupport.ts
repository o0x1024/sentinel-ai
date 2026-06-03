import { parseHttpMessageDocument } from '@/components/http-editor/httpDocument'
import { parseRawHttpRequest } from '@/components/traffic/intruder/http'
import type { HttpExchangeRequest } from './http/model'
import { buildAbsoluteUrl, endpointFromUrl } from './http/url'
import { formatTrafficJsonBody } from './trafficJsonFormattingSupport'
import type {
  TrafficCompareMeta,
  TrafficComparePayload,
  TrafficCompareSideMeta,
} from './transfers'
import type { TrafficMessageViewTab } from './trafficDisplaySettings'

const normalizeText = (text: string): string => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n')

const tryFormatJson = (body: string): string => {
  return formatTrafficJsonBody(body)
}

export function formatComparerText(text: string, viewMode: TrafficMessageViewTab): string {
  const normalized = normalizeText(text)
  if (viewMode === 'raw') return normalized

  const parsed = parseHttpMessageDocument(normalized)
  const looksLikeHttpMessage = parsed.parsedStartLine.kind !== 'plain'

  if (!looksLikeHttpMessage) {
    return normalized
  }

  const lines = [parsed.startLine]
  for (const header of parsed.headers) {
    lines.push(header.raw)
  }

  if (!parsed.hasSeparatorLine) {
    return lines.join('\n')
  }

  const formattedBody = parsed.bodyLanguage === 'json'
    ? tryFormatJson(parsed.body)
    : parsed.body

  return `${lines.join('\n')}\n\n${formattedBody}`
}

export function buildComparerDiffSummary(leftText: string, rightText: string) {
  const leftLines = normalizeText(leftText).split('\n')
  const rightLines = normalizeText(rightText).split('\n')
  const maxLines = Math.max(leftLines.length, rightLines.length)
  let changedLines = 0

  for (let index = 0; index < maxLines; index += 1) {
    if ((leftLines[index] ?? '') !== (rightLines[index] ?? '')) {
      changedLines += 1
    }
  }

  const similarityBase = Math.max(maxLines, 1)
  const similarity = Math.max(0, Math.round(((similarityBase - changedLines) / similarityBase) * 100))

  return {
    changedLines,
    similarity,
  }
}

export function isComparerRequestText(text: string): boolean {
  const parsed = parseHttpMessageDocument(normalizeText(text))
  return parsed.parsedStartLine.kind === 'request'
}

export function buildRepeaterRequestFromComparerText(text: string): HttpExchangeRequest | null {
  const normalized = normalizeText(text)
  const parsed = parseRawHttpRequest(normalized)
  if (!parsed) return null

  const requestTarget = parsed.path.trim()
  if (requestTarget.startsWith('http://') || requestTarget.startsWith('https://')) {
    return {
      endpoint: endpointFromUrl(requestTarget),
      absoluteUrl: requestTarget,
      request: {
        method: parsed.method,
        target: requestTarget,
        versionPreference: parsed.version,
        headers: parsed.headers,
        bodyText: parsed.bodyText,
      },
    }
  }

  const hostHeader = parsed.headers.find((header) => header.name.toLowerCase() === 'host')?.value?.trim()
  if (!hostHeader) return null

  const [host, portText] = hostHeader.split(':')
  const port = portText ? Number.parseInt(portText, 10) : null
  const protocol = port === 80 ? 'http' : 'https'
  const portSuffix = port && !Number.isNaN(port) && ![80, 443].includes(port) ? `:${port}` : ''
  const path = requestTarget.startsWith('/') ? requestTarget : `/${requestTarget}`
  const absoluteUrl = `${protocol}://${host}${portSuffix}${path}`
  const endpoint = endpointFromUrl(absoluteUrl)

  return {
    endpoint,
    absoluteUrl: buildAbsoluteUrl(endpoint, path),
    request: {
      method: parsed.method,
      target: path,
      versionPreference: parsed.version,
      headers: parsed.headers,
      bodyText: parsed.bodyText,
    },
  }
}

export function inferComparerSideMeta(text: string): TrafficCompareSideMeta {
  const normalized = normalizeText(text)
  const parsed = parseHttpMessageDocument(normalized)

  if (parsed.parsedStartLine.kind === 'request') {
    return {
      messageType: 'request',
      repeaterRequest: buildRepeaterRequestFromComparerText(normalized) ?? undefined,
    }
  }

  if (parsed.parsedStartLine.kind === 'response') {
    return {
      messageType: 'response',
    }
  }

  return {
    messageType: 'generic',
  }
}

export function resolveComparerSideMeta(
  payload: TrafficComparePayload | null,
  side: 'left' | 'right',
): TrafficCompareSideMeta {
  if (!payload) {
    return { messageType: 'generic' }
  }

  const meta = side === 'left' ? payload.leftMeta : payload.rightMeta
  if (meta) {
    return meta
  }

  const text = side === 'left' ? payload.leftText : payload.rightText
  return inferComparerSideMeta(text)
}

export function resolveComparerMeta(payload: TrafficComparePayload | null): TrafficCompareMeta {
  return payload?.compareMeta ?? {
    source: 'generic',
    kind: 'generic',
  }
}
