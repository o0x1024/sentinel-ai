import { detectHttpBodyLanguage } from '@/components/http-editor/httpDocument'
import { getProxyHistoryDerived } from './proxyHistoryDerivedSupport'
import type {
  ProxyHistoryRequestTab,
  ProxyHistoryResponseTab,
  ProxyHistoryViewMode,
  ProxyRequest,
} from './proxyHistoryTypes'

export const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const index = Math.floor(Math.log(bytes) / Math.log(k))
  return `${Math.round((bytes / Math.pow(k, index)) * 100) / 100} ${sizes[index]}`
}

export const formatHeaders = (headers: string) => {
  try {
    const parsed = JSON.parse(headers)
    return Object.entries(parsed)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n')
  } catch {
    return headers
  }
}

export const truncateText = (text: string, maxLength: number) => {
  if (!text) return ''
  if (text.length <= maxLength) return text
  return `${text.substring(0, maxLength)}...`
}

export const formatTime = (timestamp: string) => new Date(timestamp).toLocaleString('zh-CN')

export const getMimeType = (request: ProxyRequest): string => {
  return getProxyHistoryDerived(request).mimeType
}

export const getMethodClass = (method: string) => {
  switch (method.toUpperCase()) {
    case 'GET':
      return 'badge-info'
    case 'POST':
      return 'badge-success'
    case 'PUT':
      return 'badge-warning'
    case 'DELETE':
      return 'badge-error'
    case 'PATCH':
      return 'badge-accent'
    default:
      return 'badge-ghost'
  }
}

export const getStatusClass = (statusCode: number) => {
  if (statusCode === -1) return 'badge-warning'
  if (statusCode === 0) return 'badge-error'
  if (statusCode >= 200 && statusCode < 300) return 'badge-success'
  if (statusCode >= 300 && statusCode < 400) return 'badge-info'
  if (statusCode >= 400 && statusCode < 500) return 'badge-warning'
  if (statusCode >= 500) return 'badge-error'
  return 'badge-ghost'
}

export const getStatusText = (statusCode: number) => {
  if (statusCode === -1) return 'TUNNEL'
  if (statusCode === 0) return 'TLS ERR'
  return String(statusCode)
}

export const getStatusTitle = (statusCode: number) => {
  if (statusCode === -1) return 'HTTPS Tunnel (CONNECT) - TLS handshake may have failed'
  if (statusCode === 0) return 'TLS Handshake Failed - Certificate Error'
  return ''
}

export const getColumnValue = (request: ProxyRequest, columnId: string): string => {
  const derived = getProxyHistoryDerived(request)

  switch (columnId) {
    case 'id':
      return String(request.id)
    case 'host':
      return request.host
    case 'method':
      return request.method
    case 'url':
      return request.url
    case 'params':
      return derived.hasParams ? '✓' : ''
    case 'status':
      return String(request.status_code)
    case 'length':
      return formatBytes(request.response_size)
    case 'mime':
      return derived.mimeType
    case 'extension':
      return derived.extension
    case 'title':
      return request.title || ''
    case 'tls':
      return request.protocol === 'https' ? '✓' : ''
    case 'ip':
      return request.ip || ''
    case 'time':
      return derived.formattedTime
    case 'listener':
      return derived.listenerValue
    case 'responseTimer':
      return `${request.response_time}ms`
    default:
      return ''
  }
}

export const getHarStatusText = (code: number): string => {
  const statusTexts: Record<number, string> = {
    200: 'OK',
    201: 'Created',
    204: 'No Content',
    301: 'Moved Permanently',
    302: 'Found',
    304: 'Not Modified',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
  }
  return statusTexts[code] || 'Unknown'
}

const getRequestPath = (url: string): string => {
  try {
    const urlObj = new URL(url)
    return urlObj.pathname + urlObj.search || '/'
  } catch {
    const match = url.match(/^https?:\/\/[^/]+(\/.*)?$/)
    if (match) {
      return match[1] || '/'
    }
    return url
  }
}

const getHostFromUrl = (url: string): string | null => {
  try {
    return new URL(url).host || null
  } catch {
    return null
  }
}

const formatHeaderBlock = (headersJsonOrRaw: string | undefined, opts: { skipHost?: boolean } = {}): string => {
  if (!headersJsonOrRaw) return ''
  const skip = new Set<string>()
  if (opts.skipHost) skip.add('host')

  try {
    const parsed = JSON.parse(headersJsonOrRaw)
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      let output = ''
      for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
        const keyLower = key.toLowerCase()
        if (skip.has(keyLower)) continue
        if (Array.isArray(value)) {
          for (const item of value) {
            output += `${key}: ${String(item)}\n`
          }
        } else {
          output += `${key}: ${String(value)}\n`
        }
      }
      return output
    }
  } catch {
    // fall through to raw mode
  }

  const lines = headersJsonOrRaw
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter((line) => line.trim().length > 0)
    .filter((line) => {
      const index = line.indexOf(':')
      if (index <= 0) return true
      return !skip.has(line.slice(0, index).trim().toLowerCase())
    })

  return lines.length ? `${lines.join('\n')}\n` : ''
}

const formatJsonBody = (body: string): string => {
  if (!body) return ''
  try {
    return JSON.stringify(JSON.parse(body), null, 2)
  } catch {
    return body
  }
}

export const formatRequest = (
  request: ProxyRequest,
  tab: ProxyHistoryRequestTab | string,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  if (tab === 'hex') {
    return stringToHex(formatRequestRaw(request, viewMode))
  }
  if (tab === 'raw') {
    return formatRequestRaw(request, viewMode)
  }

  const useEdited = viewMode === 'edited' && request.was_edited
  const method = useEdited && request.edited_method ? request.edited_method : request.method
  const url = useEdited && request.edited_url ? request.edited_url : request.url
  const headers = useEdited && request.edited_request_headers ? request.edited_request_headers : request.request_headers
  const body = useEdited && request.edited_request_body ? request.edited_request_body : request.request_body

  const requestPath = getRequestPath(url)
  let result = `${method} ${requestPath} HTTP/1.1\n`
  const hostValue = request.host || getHostFromUrl(url)
  if (hostValue) result += `Host: ${hostValue}\n`
  result += formatHeaderBlock(headers, { skipHost: !!hostValue })

  if (body) {
    result += '\n'
    result += formatJsonBody(body)
  }

  return result
}

export const formatRequestRaw = (
  request: ProxyRequest,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  const useEdited = viewMode === 'edited' && request.was_edited
  const method = useEdited && request.edited_method ? request.edited_method : request.method
  const url = useEdited && request.edited_url ? request.edited_url : request.url
  const headers = useEdited && request.edited_request_headers ? request.edited_request_headers : request.request_headers
  const body = useEdited && request.edited_request_body ? request.edited_request_body : request.request_body

  const requestPath = getRequestPath(url)
  let result = `${method} ${requestPath} HTTP/1.1\n`
  const hostValue = request.host || getHostFromUrl(url)
  if (hostValue) result += `Host: ${hostValue}\n`
  result += formatHeaderBlock(headers, { skipHost: !!hostValue })

  if (body) {
    result += `\n${body}`
  }
  return result
}

export const getResponseContentType = (
  request: ProxyRequest,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  const useEdited = viewMode === 'edited' && request.was_edited
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers

  if (headers) {
    try {
      const parsed = JSON.parse(headers)
      return parsed['content-type'] || parsed['Content-Type'] || ''
    } catch {
      return ''
    }
  }
  return ''
}

export const formatResponse = (
  request: ProxyRequest,
  tab: ProxyHistoryResponseTab | string,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  if (tab === 'hex') {
    return stringToHex(formatResponseRaw(request, viewMode))
  }
  if (tab === 'raw') {
    return formatResponseRaw(request, viewMode)
  }

  const useEdited = viewMode === 'edited' && request.was_edited
  const statusCode = useEdited && request.edited_status_code ? request.edited_status_code : request.status_code
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers
  const body = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body

  let result = `HTTP/1.2 ${statusCode} OK\n`
  result += formatHeaderBlock(headers)

  if (body) {
    result += '\n'
    const contentType = getResponseContentType(request, viewMode)
    const bodyFormat = detectHttpBodyLanguage(body, contentType)

    if (bodyFormat === 'json') {
      try {
        result += JSON.stringify(JSON.parse(body), null, 2)
      } catch {
        result += body
      }
    } else if (bodyFormat === 'html' || bodyFormat === 'xml' || bodyFormat === 'text') {
      result += body
    } else {
      const bodySize = new Blob([body]).size
      result += `[Binary data - ${formatBytes(bodySize)}]\n`
      result += `Content-Type: ${contentType}\n`
      result += `\nFirst 200 characters:\n${body.substring(0, 200)}...`
    }
  }

  return result
}

export const isResponseCompressed = (request: ProxyRequest): boolean => {
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers)
      const encoding = headers['content-encoding'] || headers['Content-Encoding']
      return Boolean(encoding && (encoding.includes('gzip') || encoding.includes('br') || encoding.includes('deflate')))
    } catch {
      return false
    }
  }
  return false
}

export const formatResponseRaw = (
  request: ProxyRequest,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  const useEdited = viewMode === 'edited' && request.was_edited
  const statusCode = useEdited && request.edited_status_code ? request.edited_status_code : request.status_code
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers
  const body = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body

  let result = `HTTP/1.2 ${statusCode} OK\n`
  result += formatHeaderBlock(headers)
  if (body) {
    result += `\n${body}`
  }
  return result
}

export const hasEditedResponse = (request: ProxyRequest): boolean =>
  Boolean(request.edited_response_headers || request.edited_response_body || request.edited_status_code)

export const stringToHex = (str: string): string => {
  let hex = ''
  for (let index = 0; index < str.length; index += 1) {
    hex += `${str.charCodeAt(index).toString(16).padStart(2, '0')} `
    if ((index + 1) % 16 === 0) {
      hex += '\n'
    }
  }
  return hex
}

export const getResponseBody = (
  request: ProxyRequest,
  viewMode: ProxyHistoryViewMode = 'edited',
): string => {
  const useEdited = viewMode === 'edited' && request.was_edited
  return (useEdited && request.edited_response_body ? request.edited_response_body : request.response_body) || ''
}
