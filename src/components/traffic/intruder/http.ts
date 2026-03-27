import type {
  IntruderSourceRequest,
  IntruderTarget,
  IntruderAttackOptions,
  ParsedHttpRequest,
  ParsedHttpResponse,
} from './types'

export function createIntruderId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export function normalizeRequestLineEndings(rawRequest: string): string {
  return rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n')
}

export function ensureRawRequestTerminator(rawRequest: string): string {
  let normalized = normalizeRequestLineEndings(rawRequest)

  const hasHeaderBodySplit = normalized.includes('\r\n\r\n')
  if (!hasHeaderBodySplit) {
    normalized += '\r\n\r\n'
  } else if (normalized.endsWith('\r\n')) {
    return normalized
  }

  return normalized
}

export function parseRawHttpRequest(rawRequest: string): ParsedHttpRequest | null {
  const normalized = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const separatorIndex = normalized.indexOf('\n\n')
  const headerPart = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const body = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const lines = headerPart.split('\n')

  if (!lines[0]) {
    return null
  }

  const [method = 'GET', path = '/', protocol = 'HTTP/1.1'] = lines[0].trim().split(/\s+/)
  const headers: Record<string, string> = {}

  for (const line of lines.slice(1)) {
    const colonIndex = line.indexOf(':')
    if (colonIndex <= 0) continue
    const key = line.slice(0, colonIndex).trim()
    const value = line.slice(colonIndex + 1).trim()
    headers[key] = value
  }

  return {
    method,
    path,
    protocol,
    headers,
    body,
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
  const headers: Record<string, string> = {}

  for (const line of lines.slice(1)) {
    const colonIndex = line.indexOf(':')
    if (colonIndex <= 0) continue
    const key = line.slice(0, colonIndex).trim()
    const value = line.slice(colonIndex + 1).trim()
    headers[key] = value
  }

  return {
    statusCode: statusCodeMatch ? Number.parseInt(statusCodeMatch[1], 10) : 0,
    headers,
    body,
    responseTimeMs,
  }
}

export function extractTargetFromRequest(rawRequest: string, fallbackUrl?: string): IntruderTarget {
  const parsed = parseRawHttpRequest(rawRequest)
  const hostHeader = Object.entries(parsed?.headers ?? {}).find(([key]) => key.toLowerCase() === 'host')?.[1]

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

export function createRawRequestFromSource(request?: IntruderSourceRequest): string {
  if (!request?.url) {
    return 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n'
  }

  try {
    const url = new URL(request.url)
    const path = `${url.pathname || '/'}${url.search}`
    const lines: string[] = [
      `${request.method || 'GET'} ${path || '/'} HTTP/1.1`,
      `Host: ${url.host}`,
    ]

    for (const [key, value] of Object.entries(request.headers ?? {})) {
      if (key.toLowerCase() === 'host') continue
      lines.push(`${key}: ${value}`)
    }

    lines.push('', request.body ?? '')
    return lines.join('\r\n')
  } catch {
    return 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n'
  }
}

export function buildFullUrl(rawRequest: string, target: IntruderTarget): string {
  const parsed = parseRawHttpRequest(rawRequest)
  if (!parsed) return ''

  if (parsed.path.startsWith('http://') || parsed.path.startsWith('https://')) {
    return parsed.path
  }

  const protocol = target.useTls ? 'https' : 'http'
  const defaultPort = target.useTls ? 443 : 80
  const portSuffix = target.port !== defaultPort ? `:${target.port}` : ''
  const path = parsed.path.startsWith('/') ? parsed.path : `/${parsed.path}`

  return `${protocol}://${target.host}${portSuffix}${path}`
}

export function buildSourceRequestFromRawRequest(
  rawRequest: string,
  target: IntruderTarget,
): IntruderSourceRequest | null {
  const parsed = parseRawHttpRequest(rawRequest)
  if (!parsed) return null

  return {
    method: parsed.method,
    url: buildFullUrl(rawRequest, target),
    headers: parsed.headers,
    body: parsed.body || undefined,
  }
}

export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B'

  const units = ['B', 'KB', 'MB', 'GB']
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const size = bytes / 1024 ** exponent
  return `${Math.round(size * 100) / 100} ${units[exponent]}`
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
  const headers = lines.slice(1)
  const updatedHeaders: string[] = []
  let hasHost = false
  let hasContentLength = false
  let hasConnection = false

  for (const header of headers) {
    const colonIndex = header.indexOf(':')
    if (colonIndex <= 0) {
      updatedHeaders.push(header)
      continue
    }

    const key = header.slice(0, colonIndex).trim()
    const lowerKey = key.toLowerCase()

    if (lowerKey === 'host') {
      hasHost = true
      updatedHeaders.push(
        options.updateHostHeader ? `Host: ${buildHostHeaderValue(target)}` : header,
      )
      continue
    }

    if (lowerKey === 'content-length') {
      hasContentLength = true
      updatedHeaders.push(
        options.updateContentLength ? `Content-Length: ${new TextEncoder().encode(body).length}` : header,
      )
      continue
    }

    if (lowerKey === 'connection') {
      hasConnection = true
      updatedHeaders.push(options.setConnectionClose ? 'Connection: close' : header)
      continue
    }

    updatedHeaders.push(header)
  }

  if (options.updateHostHeader && !hasHost) {
    updatedHeaders.unshift(`Host: ${buildHostHeaderValue(target)}`)
  }

  if (options.updateContentLength && body.length > 0 && !hasContentLength) {
    updatedHeaders.push(`Content-Length: ${new TextEncoder().encode(body).length}`)
  }

  if (options.setConnectionClose && !hasConnection) {
    updatedHeaders.push('Connection: close')
  }

  return ensureRawRequestTerminator([requestLine, ...updatedHeaders, '', body].join('\n'))
}

function buildHostHeaderValue(target: IntruderTarget): string {
  const defaultPort = target.useTls ? 443 : 80
  return target.port === defaultPort ? target.host : `${target.host}:${target.port}`
}

export function countWords(text: string): number {
  const trimmed = text.trim()
  return trimmed ? trimmed.split(/\s+/).length : 0
}

export function countLines(text: string): number {
  if (!text) return 0
  return text.split(/\r\n|\r|\n/).length
}
