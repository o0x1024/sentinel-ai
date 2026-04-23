export interface ProxyRequest {
  id: number
  db_request_id?: number | null
  traffic_request_id?: string | null
  url: string
  host: string
  scheme: string
  http_version_observed?: string
  method: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  response_size: number
  response_time: number
  timestamp: string
  ip?: string
  listener?: string
  extension?: string
  title?: string
  mime_type?: string
  was_edited?: boolean
  edited_request_headers?: string
  edited_request_body?: string
  edited_method?: string
  edited_url?: string
  edited_response_headers?: string
  edited_response_body?: string
  edited_status_code?: number
  has_full_details?: boolean
}

export interface VirtualItem {
  data: ProxyRequest
  offset: number
}

export interface Column {
  id: string
  label: string
  visible: boolean
  width: number
  minWidth: number
}

export type ProxyHistorySortDirection = 'asc' | 'desc'

export interface ProxyHistorySortState {
  columnId: string
  direction: ProxyHistorySortDirection
}

export interface WebSocketConnection {
  id: string
  url: string
  host: string
  protocol: string
  request_headers?: string
  response_headers?: string
  status: 'open' | 'closed' | 'error'
  opened_at: string
  closed_at?: string
  close_code?: number
  close_reason?: string
  message_ids?: number[]
}

export interface WebSocketMessage {
  id: number
  connection_id: string
  direction: 'send' | 'receive'
  message_type: 'text' | 'binary' | 'ping' | 'pong' | 'close'
  content?: string
  content_length: number
  timestamp: string
}

export interface ProxyHistoryFilterConfig {
  requestType: {
    showOnlyInScope: boolean
    showOnlyWithParams: boolean
    hideWithoutResponse: boolean
  }
  mimeType: {
    html: boolean
    script: boolean
    xml: boolean
    css: boolean
    otherText: boolean
    images: boolean
    flash: boolean
    otherBinary: boolean
  }
  statusCode: {
    s2xx: boolean
    s3xx: boolean
    s4xx: boolean
    s5xx: boolean
  }
  search: {
    term: string
    regex: boolean
    caseSensitive: boolean
    negative: boolean
  }
  extension: {
    showOnlyEnabled: boolean
    showOnly: string
    hideEnabled: boolean
    hide: string
  }
  annotation: {
    showOnlyWithNotes: boolean
    showOnlyHighlighted: boolean
  }
  listener: {
    port: string
  }
  bambdaExpression: string
}

export interface ProxyHistoryFilterCache {
  config: ProxyHistoryFilterConfig | null
  searchRegex: RegExp | null
  searchTerm: string
  showExts: Set<string> | null
  hideExts: Set<string> | null
}

export type ProxyHistoryViewMode = 'original' | 'edited'
export type ProxyHistoryRequestTab = 'pretty' | 'raw' | 'hex'
export type ProxyHistoryResponseTab = 'pretty' | 'raw' | 'hex' | 'render'
export type ProxyHistoryProtocolFilter = 'all' | 'http' | 'websocket'
export type ProxyHistoryWsTab = 'messages' | 'handshake'
