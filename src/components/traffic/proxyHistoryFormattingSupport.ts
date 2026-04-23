import { detectHttpBodyLanguage } from '@/components/http-editor/httpDocument'
import { parseStoredHeaderEntries } from './http/headers'
import { normalizeProxyHistoryHttpVersion } from './proxyHistoryHttpSupport'
import { getProxyHistoryDerived } from './proxyHistoryDerivedSupport'
import {
  getDisplayResponseBody,
  isImageResponseContentType,
} from './trafficResponsePreviewSupport'
import type {
  ProxyHistoryRequestTab,
  ProxyHistoryResponseTab,
  ProxyHistoryViewMode,
  ProxyRequest,
} from './proxyHistoryTypes'

interface ResponseFormattingOptions {
  bodyText?: string
}

export const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const index = Math.floor(Math.log(bytes) / Math.log(k))
  return `${Math.round((bytes / Math.pow(k, index)) * 100) / 100} ${sizes[index]}`
}

export const formatHeaders = (headers: string) => {
  const parsed = parseStoredHeaderEntries(headers)
  if (!parsed.length) return headers
  return parsed.map((header) => `${header.name}: ${header.value}`).join('\n')
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
    case 'httpVersion':
      return normalizeProxyHistoryHttpVersion(request.http_version_observed)
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
      return request.scheme === 'https' ? '✓' : ''
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
    101: 'Switching Protocols',
    200: 'OK',
    201: 'Created',
    202: 'Accepted',
    204: 'No Content',
    206: 'Partial Content',
    301: 'Moved Permanently',
    302: 'Found',
    303: 'See Other',
    307: 'Temporary Redirect',
    308: 'Permanent Redirect',
    304: 'Not Modified',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    405: 'Method Not Allowed',
    409: 'Conflict',
    429: 'Too Many Requests',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
  }
  if (code === -1) return 'TUNNEL'
  if (code === 0) return 'TLS ERR'
  return statusTexts[code] || 'Unknown'
}

const getStartLineHttpVersion = (request: ProxyRequest) =>
  normalizeProxyHistoryHttpVersion(request.http_version_observed)

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

  const parsedHeaders = parseStoredHeaderEntries(headersJsonOrRaw)
  if (parsedHeaders.length > 0) {
    const lines = parsedHeaders
      .filter((header) => !skip.has(header.name.toLowerCase()))
      .map((header) => `${header.name}: ${header.value}`)
    return lines.length ? `${lines.join('\n')}\n` : ''
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
  let result = `${method} ${requestPath} ${getStartLineHttpVersion(request)}\n`
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
  let result = `${method} ${requestPath} ${getStartLineHttpVersion(request)}\n`
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
    const parsed = parseStoredHeaderEntries(headers)
    return parsed.find((header) => header.name.toLowerCase() === 'content-type')?.value || ''
  }
  return ''
}

export const formatResponse = (
  request: ProxyRequest,
  tab: ProxyHistoryResponseTab | string,
  viewMode: ProxyHistoryViewMode = 'edited',
  options: ResponseFormattingOptions = {},
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
  const storedBody = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body

  let result = `${getStartLineHttpVersion(request)} ${statusCode} ${getHarStatusText(statusCode)}\n`
  result += formatHeaderBlock(headers)

  const displayBody = options.bodyText ?? (storedBody ? getDisplayResponseBody(storedBody, getResponseContentType(request, viewMode)) : storedBody)

  if (displayBody) {
    result += '\n'
    const contentType = getResponseContentType(request, viewMode)
    const bodyFormat = detectHttpBodyLanguage(displayBody, contentType)

    if (isImageResponseContentType(contentType)) {
      result += displayBody
    } else if (bodyFormat === 'json') {
      try {
        result += JSON.stringify(JSON.parse(displayBody), null, 2)
      } catch {
        result += displayBody
      }
    } else if (bodyFormat === 'html' || bodyFormat === 'xml' || bodyFormat === 'text') {
      result += displayBody
    } else {
      const bodySize = new Blob([displayBody]).size
      result += `[Binary data - ${formatBytes(bodySize)}]\n`
      result += `Content-Type: ${contentType}\n`
      result += `\nFirst 200 characters:\n${displayBody.substring(0, 200)}...`
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
  options: ResponseFormattingOptions = {},
): string => {
  const useEdited = viewMode === 'edited' && request.was_edited
  const statusCode = useEdited && request.edited_status_code ? request.edited_status_code : request.status_code
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers
  const storedBody = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body
  const contentType = getResponseContentType(request, viewMode)
  const body = options.bodyText ?? (storedBody ? getDisplayResponseBody(storedBody, contentType) : storedBody)

  let result = `${getStartLineHttpVersion(request)} ${statusCode} ${getHarStatusText(statusCode)}\n`
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
