import type {
  InterceptedItem,
  InterceptedRequest,
  InterceptedResponse,
  InterceptedWebSocketMessage,
  ProxyStatus,
} from './proxyInterceptSupport'

export interface ContextMenuState {
  visible: boolean
  x: number
  y: number
  item: InterceptedItem | null
  index: number
  editorContext: boolean
  selection: { from: number; to: number } | null
}

export interface FilterRule {
  type: 'request' | 'response'
  matchType: string
  relationship: string
  condition: string
  action: 'exclude' | 'include'
}

export interface ProxyInterceptStatusSnapshot {
  proxyStatus?: ProxyStatus
  interceptEnabled?: boolean
  responseInterceptEnabled?: boolean
  websocketInterceptEnabled?: boolean
}

export interface ProxyInterceptListenerCleanup {
  proxyStatus: (() => void) | null
  interceptRequest: (() => void) | null
  interceptResponse: (() => void) | null
  interceptWebSocket: (() => void) | null
}

export interface ProxyInterceptSetupListenersOptions {
  listen: typeof import('@tauri-apps/api/event').listen
  onProxyStatus: (status: ProxyStatus) => void
  onInterceptRequest: (request: InterceptedRequest) => void
  onInterceptResponse: (response: InterceptedResponse) => void
  onInterceptWebSocket: (message: InterceptedWebSocketMessage) => void
}
