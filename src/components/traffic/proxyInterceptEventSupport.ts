import type { listen as TauriListen } from '@tauri-apps/api/event'
import type { InterceptedRequest, InterceptedResponse, InterceptedWebSocketMessage, ProxyStatus } from './proxyInterceptSupport'
import type { ProxyInterceptListenerCleanup, ProxyInterceptSetupListenersOptions, ProxyInterceptStatusSnapshot } from './proxyInterceptTypes'

export async function refreshProxyInterceptStatus(
  invoke: (command: string, args?: Record<string, unknown>) => Promise<any>,
): Promise<ProxyInterceptStatusSnapshot> {
  const snapshot: ProxyInterceptStatusSnapshot = {}
  const proxyStatusResponse = await invoke('get_proxy_status')
  if (proxyStatusResponse.success && proxyStatusResponse.data) {
    snapshot.proxyStatus = proxyStatusResponse.data
  }

  const interceptResponse = await invoke('get_intercept_enabled')
  if (interceptResponse.success) {
    snapshot.interceptEnabled = interceptResponse.data
  }

  const responseInterceptResponse = await invoke('get_response_intercept_enabled')
  if (responseInterceptResponse.success) {
    snapshot.responseInterceptEnabled = responseInterceptResponse.data
  }

  const websocketInterceptResponse = await invoke('get_websocket_intercept_enabled')
  if (websocketInterceptResponse.success) {
    snapshot.websocketInterceptEnabled = websocketInterceptResponse.data
  }

  return snapshot
}

export async function setupProxyInterceptEventListeners(
  options: ProxyInterceptSetupListenersOptions,
): Promise<ProxyInterceptListenerCleanup> {
  const {
    listen,
    onProxyStatus,
    onInterceptRequest,
    onInterceptResponse,
    onInterceptWebSocket,
  } = options

  const proxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    onProxyStatus(event.payload)
  })

  const interceptRequest = await listen<InterceptedRequest>('intercept:request', (event) => {
    onInterceptRequest(event.payload)
  })

  const interceptResponse = await listen<InterceptedResponse>('intercept:response', (event) => {
    onInterceptResponse(event.payload)
  })

  const interceptWebSocket = await listen<InterceptedWebSocketMessage>('proxy:intercept_websocket', (event) => {
    onInterceptWebSocket(event.payload)
  })

  return {
    proxyStatus,
    interceptRequest,
    interceptResponse,
    interceptWebSocket,
  }
}
