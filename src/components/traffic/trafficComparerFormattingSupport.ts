import { parseHttpMessageDocument } from '@/components/http-editor/httpDocument'
import { parseRawHttpRequest } from '@/components/traffic/intruder/http'
import type {
  TrafficCompareMeta,
  TrafficComparePayload,
  TrafficCompareSideMeta,
  TrafficTransferRequest,
} from './transfers'
import type { TrafficMessageViewTab } from './trafficDisplaySettings'

const normalizeText = (text: string): string => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n')

const tryFormatJson = (body: string): string => {
  const trimmed = body.trim()
  if (!trimmed) return body

  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2)
  } catch {
    return body
  }
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

export function buildRepeaterRequestFromComparerText(text: string): TrafficTransferRequest | null {
  const normalized = normalizeText(text)
  const parsed = parseRawHttpRequest(normalized)
  if (!parsed) return null

  const requestTarget = parsed.path.trim()
  if (requestTarget.startsWith('http://') || requestTarget.startsWith('https://')) {
    return {
      method: parsed.method,
      url: requestTarget,
      headers: parsed.headers,
      body: parsed.body || undefined,
    }
  }

  const hostHeaderEntry = Object.entries(parsed.headers).find(([key]) => key.toLowerCase() === 'host')
  const hostHeader = hostHeaderEntry?.[1]?.trim()
  if (!hostHeader) return null

  const [host, portText] = hostHeader.split(':')
  const port = portText ? Number.parseInt(portText, 10) : null
  const protocol = port === 80 ? 'http' : 'https'
  const portSuffix = port && !Number.isNaN(port) && ![80, 443].includes(port) ? `:${port}` : ''
  const path = requestTarget.startsWith('/') ? requestTarget : `/${requestTarget}`

  return {
    method: parsed.method,
    url: `${protocol}://${host}${portSuffix}${path}`,
    headers: parsed.headers,
    body: parsed.body || undefined,
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
