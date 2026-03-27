export interface ProxyRequestForAI {
  id: number
  url: string
  host: string
  protocol: string
  method: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  response_size: number
  response_time: number
  timestamp: string
}

export interface ProxyStats {
  http_requests: number
  https_requests: number
  errors: number
  qps: number
}

export interface ProxyStatus {
  running: boolean
  port: number
  mitm: boolean
  stats: ProxyStats
}

export interface InterceptedRequest {
  id: string
  timestamp: number
  method: string
  url: string
  path: string
  protocol: string
  headers: Record<string, string>
  body: string
}

export interface InterceptedResponse {
  id: string
  request_id: string
  timestamp: number
  status: number
  headers: Record<string, string>
  body: string
}

export interface InterceptedWebSocketMessage {
  id: string
  connection_id: string
  timestamp: number
  direction: 'client_to_server' | 'server_to_client'
  message_type: string
  content: string
}

export type InterceptedItem =
  | { type: 'request'; data: InterceptedRequest }
  | { type: 'response'; data: InterceptedResponse }
  | { type: 'websocket'; data: InterceptedWebSocketMessage }

export function formatInterceptBody(body: string): string {
  if (!body) return ''
  try {
    const parsed = JSON.parse(body)
    return JSON.stringify(parsed, null, 2)
  } catch {
    if (body.trim().startsWith('<')) {
      return formatInterceptXml(body)
    }
    return body
  }
}

export function formatInterceptXml(xml: string): string {
  const formatted = xml.replace(/(>)(<)(\/*)/g, '$1\n$2$3')
  let indent = 0
  return formatted
    .split('\n')
    .map((line) => {
      let currentIndent = indent
      if (line.match(/^<\/\w/)) {
        indent = Math.max(indent - 1, 0)
        currentIndent = indent
      }
      const result = '  '.repeat(currentIndent) + line
      if (line.match(/^<[^!?/][^>]*[^/]>/)) {
        indent += 1
      }
      return result
    })
    .join('\n')
}

export function formatInterceptTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString()
}

export function truncateInterceptText(value: string, length: number) {
  if (value.length <= length) return value
  return `${value.slice(0, length)}...`
}

export function getInterceptMethodClass(method: string) {
  switch (method.toUpperCase()) {
    case 'GET': return 'badge-info'
    case 'POST': return 'badge-success'
    case 'PUT': return 'badge-warning'
    case 'DELETE': return 'badge-error'
    case 'PATCH': return 'badge-accent'
    default: return 'badge-ghost'
  }
}

export function getInterceptStatusClass(status: number) {
  if (status >= 200 && status < 300) return 'badge-success'
  if (status >= 300 && status < 400) return 'badge-info'
  if (status >= 400 && status < 500) return 'badge-warning'
  if (status >= 500) return 'badge-error'
  return 'badge-ghost'
}

export function convertInterceptedItemToProxyRequest(item: InterceptedItem): ProxyRequestForAI | null {
  if (item.type === 'request') {
    const request = item.data
    let host = ''
    try {
      host = new URL(request.url).hostname
    } catch {
      host = ''
    }
    return {
      id: Date.now(),
      url: request.url,
      host,
      protocol: request.protocol || 'HTTP/1.1',
      method: request.method,
      status_code: 0,
      request_headers: JSON.stringify(request.headers),
      request_body: request.body,
      response_headers: undefined,
      response_body: undefined,
      response_size: 0,
      response_time: 0,
      timestamp: new Date(request.timestamp).toISOString(),
    }
  }

  if (item.type === 'response') {
    const response = item.data
    return {
      id: Date.now(),
      url: '',
      host: '',
      protocol: 'HTTP/1.1',
      method: '',
      status_code: response.status,
      request_headers: undefined,
      request_body: undefined,
      response_headers: JSON.stringify(response.headers),
      response_body: response.body,
      response_size: response.body?.length || 0,
      response_time: 0,
      timestamp: new Date(response.timestamp).toISOString(),
    }
  }

  if (item.type === 'websocket') {
    const ws = item.data
    return {
      id: Date.now(),
      url: `ws://${ws.connection_id}`,
      host: '',
      protocol: 'WebSocket',
      method: ws.direction === 'client_to_server' ? 'WS_SEND' : 'WS_RECV',
      status_code: 0,
      request_headers: undefined,
      request_body: ws.content,
      response_headers: undefined,
      response_body: undefined,
      response_size: ws.content?.length || 0,
      response_time: 0,
      timestamp: new Date(ws.timestamp).toISOString(),
    }
  }

  return null
}

export function getInterceptItemDomain(item: InterceptedItem | null): string {
  if (item?.type !== 'request') return ''
  try {
    return new URL(item.data.url).hostname
  } catch {
    return ''
  }
}

export function getInterceptItemMethod(item: InterceptedItem | null): string {
  return item?.type === 'request' ? item.data.method : ''
}

export function getInterceptItemStatus(item: InterceptedItem | null): number {
  return item?.type === 'response' ? item.data.status : 0
}

export function getInterceptItemDirection(item: InterceptedItem | null): string {
  if (item?.type !== 'websocket') return ''
  return item.data.direction === 'client_to_server' ? 'Client → Server' : 'Server → Client'
}
