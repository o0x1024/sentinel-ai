import { parseStoredHeaderEntries } from './http/headers'
import type { ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'

export type TrafficContextEvidenceSource = 'query' | 'body' | 'header' | 'cookie' | 'path' | 'unknown'

export interface TrafficContextEvidenceHighlight {
  location: string
  source: TrafficContextEvidenceSource
  key: string
  values: string[]
}

export interface TrafficContextEvidenceSelectionRange {
  from: number
  to: number
}

interface ParsedEvidenceLocation {
  source: TrafficContextEvidenceSource
  key: string
}

export function resolveTrafficContextEvidenceHighlights(
  request: ProxyRequest | null,
  matchedLocations: string[],
  viewMode: ProxyHistoryViewMode = 'edited',
): TrafficContextEvidenceHighlight[] {
  if (!request || matchedLocations.length === 0) {
    return []
  }

  const url = getEffectiveRequestUrl(request, viewMode)
  const headers = parseStoredHeaderEntries(getEffectiveRequestHeaders(request, viewMode))
  const bodyValues = extractBodyLeafValues(getEffectiveRequestBody(request, viewMode))
  const query = safeParseUrl(url)
  const pathSegments = query?.pathname
    .split('/')
    .map(segment => segment.trim())
    .filter(Boolean) || []

  return matchedLocations.map((location) => {
    const parsed = parseEvidenceLocation(location)
    return {
      location,
      source: parsed.source,
      key: parsed.key,
      values: resolveEvidenceValues(parsed, {
        url: query,
        headers,
        bodyValues,
        pathSegments,
      }),
    }
  })
}

export function findTrafficContextEvidenceSelectionRange(
  content: string,
  highlight: TrafficContextEvidenceHighlight,
): TrafficContextEvidenceSelectionRange | null {
  const candidates = buildEvidenceSearchCandidates(highlight)

  for (const candidate of candidates) {
    const matchIndex = findCaseInsensitiveIndex(content, candidate)
    if (matchIndex >= 0) {
      return {
        from: matchIndex,
        to: matchIndex + candidate.length,
      }
    }
  }

  return null
}

export function findTrafficContextEvidenceSelectionRangeBySearchTerms(
  content: string,
  searchTerms: string[],
): TrafficContextEvidenceSelectionRange | null {
  const candidates = [...new Set(searchTerms.filter(Boolean))].sort((left, right) => right.length - left.length)
  for (const candidate of candidates) {
    const matchIndex = findCaseInsensitiveIndex(content, candidate)
    if (matchIndex >= 0) {
      return {
        from: matchIndex,
        to: matchIndex + candidate.length,
      }
    }
  }
  return null
}

function getEffectiveRequestUrl(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  if (viewMode === 'edited' && request.was_edited && request.edited_url) {
    return request.edited_url
  }
  return request.url
}

function getEffectiveRequestHeaders(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  if (viewMode === 'edited' && request.was_edited && request.edited_request_headers) {
    return request.edited_request_headers
  }
  return request.request_headers || ''
}

function getEffectiveRequestBody(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  if (viewMode === 'edited' && request.was_edited && request.edited_request_body) {
    return request.edited_request_body
  }
  return request.request_body || ''
}

function parseEvidenceLocation(location: string): ParsedEvidenceLocation {
  const [sourcePart, ...rest] = location.split('.')
  const key = rest.join('.').trim()
  switch (sourcePart) {
    case 'query':
    case 'body':
    case 'header':
    case 'cookie':
    case 'path':
      return {
        source: sourcePart,
        key,
      }
    default:
      return {
        source: 'unknown',
        key: location,
      }
  }
}

function buildEvidenceSearchCandidates(highlight: TrafficContextEvidenceHighlight): string[] {
  const rawCandidates: string[] = []
  const values = highlight.values.filter(Boolean)

  switch (highlight.source) {
    case 'query':
      values.forEach((value) => rawCandidates.push(`${highlight.key}=${value}`))
      rawCandidates.push(`${highlight.key}=`)
      rawCandidates.push(highlight.key)
      break
    case 'body':
      values.forEach((value) => {
        rawCandidates.push(`"${highlight.key}": "${value}"`)
        rawCandidates.push(`"${highlight.key}": ${value}`)
        rawCandidates.push(`${highlight.key}=${value}`)
      })
      rawCandidates.push(`"${highlight.key}":`)
      rawCandidates.push(`${highlight.key}=`)
      rawCandidates.push(highlight.key)
      break
    case 'header':
      rawCandidates.push(`${highlight.key}:`)
      values.forEach((value) => rawCandidates.push(`${highlight.key}: ${value}`))
      rawCandidates.push(highlight.key)
      break
    case 'cookie':
      values.forEach((value) => rawCandidates.push(`${highlight.key}=${value}`))
      rawCandidates.push(`${highlight.key}=`)
      rawCandidates.push(highlight.key)
      break
    case 'path':
      rawCandidates.push(`/${highlight.key}`)
      rawCandidates.push(highlight.key)
      break
    default:
      rawCandidates.push(highlight.key)
      break
  }

  return [...new Set(rawCandidates.filter(Boolean))].sort((left, right) => right.length - left.length)
}

function resolveEvidenceValues(
  location: ParsedEvidenceLocation,
  context: {
    url: URL | null
    headers: Array<{ name: string; value: string }>
    bodyValues: Map<string, string[]>
    pathSegments: string[]
  },
): string[] {
  if (!location.key) {
    return []
  }

  switch (location.source) {
    case 'query':
      return context.url?.searchParams.getAll(location.key).filter(Boolean) || []
    case 'body':
      return context.bodyValues.get(location.key) || []
    case 'header':
      return context.headers
        .filter((header) => header.name.trim().toLowerCase() === location.key.toLowerCase())
        .map((header) => header.value)
        .filter(Boolean)
    case 'cookie':
      return resolveCookieValues(context.headers, location.key)
    case 'path':
      return context.pathSegments.filter((segment) => segment === location.key)
    default:
      return []
  }
}

function resolveCookieValues(
  headers: Array<{ name: string; value: string }>,
  cookieName: string,
): string[] {
  const cookieHeaderValues = headers
    .filter((header) => header.name.trim().toLowerCase() === 'cookie')
    .map((header) => header.value)

  const matches = new Set<string>()
  for (const rawCookieHeader of cookieHeaderValues) {
    for (const entry of rawCookieHeader.split(';')) {
      const [name, ...valueParts] = entry.split('=')
      if (name.trim() !== cookieName) {
        continue
      }
      const value = valueParts.join('=').trim()
      if (value) {
        matches.add(value)
      }
    }
  }
  return [...matches]
}

function extractBodyLeafValues(body: string): Map<string, string[]> {
  if (!body) {
    return new Map()
  }

  try {
    const parsed = JSON.parse(body)
    const values = new Map<string, string[]>()
    flattenJsonLeafValues(parsed, '', values)
    return values
  } catch {
    const values = new Map<string, string[]>()
    for (const [key, value] of new URLSearchParams(body)) {
      pushValue(values, key, value)
    }
    return values
  }
}

function flattenJsonLeafValues(value: unknown, path: string, output: Map<string, string[]>) {
  if (Array.isArray(value)) {
    value.forEach(item => flattenJsonLeafValues(item, path, output))
    return
  }

  if (value && typeof value === 'object') {
    Object.entries(value as Record<string, unknown>).forEach(([key, nested]) => {
      const nextPath = path ? `${path}.${key}` : key
      flattenJsonLeafValues(nested, nextPath, output)
    })
    return
  }

  if (!path || value === null || value === undefined) {
    return
  }

  pushValue(output, leafFieldName(path), String(value))
}

function leafFieldName(path: string): string {
  return path.split('.').filter(Boolean).pop() || path
}

function pushValue(store: Map<string, string[]>, key: string, value: string) {
  if (!key) {
    return
  }
  const next = store.get(key) || []
  if (!next.includes(value)) {
    next.push(value)
  }
  store.set(key, next)
}

function safeParseUrl(rawUrl: string): URL | null {
  try {
    return new URL(rawUrl)
  } catch {
    return null
  }
}

function findCaseInsensitiveIndex(content: string, candidate: string): number {
  if (!candidate) {
    return -1
  }
  return content.toLowerCase().indexOf(candidate.toLowerCase())
}
