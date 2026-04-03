import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Ref } from 'vue'
import { dialog } from '@/composables/useDialog'
import { clearProxyHistoryDerivedCache, pruneProxyHistoryDerivedCache } from './proxyHistoryDerivedSupport'
import type { ProxyHistoryWsTab, ProxyRequest, WebSocketConnection, WebSocketMessage } from './proxyHistoryTypes'

type ProxyHistoryStats = {
  total: number
  http: number
  https: number
  avgResponseTime: number
}

type Params = {
  requests: Ref<ProxyRequest[]>
  isLoading: Ref<boolean>
  hasMore: Ref<boolean>
  isLoadingMore: Ref<boolean>
  requestIdSet: Ref<Set<number>>
  stats: Ref<ProxyHistoryStats>
  wsConnections: Ref<WebSocketConnection[]>
  wsMessages: Ref<WebSocketMessage[]>
  expandedWsConnections: Ref<Set<string>>
  activeWsTabs: Ref<Map<string, ProxyHistoryWsTab>>
  isLoadingWs: Ref<boolean>
  wsMessagesCache: Ref<Map<string, WebSocketMessage[]>>
  initialLoadLimit: number
  loadMoreSize: number
  maxRequestsInMemory: number
  batchUpdateThreshold: number
  t: (key: string, params?: Record<string, unknown>) => string
}

export const useProxyHistoryData = (params: Params) => {
  let pendingUpdates: ProxyRequest[] = []
  let updateTimer: number | null = null
  let unlistenRequest: (() => void) | null = null

  const normalizeRequest = (request: ProxyRequest): ProxyRequest => ({
    ...request,
    has_full_details: request.has_full_details ?? Boolean(
      request.request_body
      || request.response_body
      || request.edited_request_body
      || request.edited_response_body,
    ),
  })

  const normalizeRequests = (requests: ProxyRequest[]) => requests.map(normalizeRequest)

  const collectStats = (requests: ProxyRequest[]) => {
    let http = 0
    let https = 0
    let totalResponseTime = 0

    requests.forEach((request) => {
      if (request.protocol === 'https') {
        https += 1
      } else if (request.protocol === 'http') {
        http += 1
      }
      totalResponseTime += request.response_time
    })

    return {
      total: requests.length,
      http,
      https,
      avgResponseTime: requests.length > 0 ? Math.round(totalResponseTime / requests.length) : 0,
    }
  }

  const mergeRequestIntoList = (request: ProxyRequest) => {
    const index = params.requests.value.findIndex((item) => item.id === request.id)
    if (index === -1) {
      return request
    }

    const merged = { ...params.requests.value[index], ...request }
    params.requests.value.splice(index, 1, merged)
    return merged
  }

  const updateStats = () => {
    params.stats.value = collectStats(params.requests.value)
  }

  const updateStatsIncremental = (request: ProxyRequest) => {
    params.stats.value.total += 1
    if (request.protocol === 'https') {
      params.stats.value.https += 1
    } else if (request.protocol === 'http') {
      params.stats.value.http += 1
    }
    if (params.stats.value.total > 0) {
      params.stats.value.avgResponseTime = Math.round(
        (params.stats.value.avgResponseTime * (params.stats.value.total - 1) + request.response_time)
          / params.stats.value.total,
      )
    }
  }

  const updateStatsIncrementalBatch = (requests: ProxyRequest[]) => {
    requests.forEach(updateStatsIncremental)
  }

  const refreshRequests = async () => {
    params.isLoading.value = true
    try {
      const response = await invoke<any>('list_proxy_requests', {
        limit: params.initialLoadLimit,
        offset: 0,
      })

      if (response.success && response.data) {
        const normalizedRequests = normalizeRequests(response.data as ProxyRequest[])
        params.requests.value = normalizedRequests
        params.hasMore.value = normalizedRequests.length === params.initialLoadLimit
        params.requestIdSet.value.clear()
        normalizedRequests.forEach((request) => params.requestIdSet.value.add(request.id))
        pruneProxyHistoryDerivedCache(params.requestIdSet.value)
        updateStats()
      }
    } catch (error: any) {
      console.error('Failed to refresh requests:', error)
      dialog.toast.error(`${params.t('trafficAnalysis.history.errors.loadFailed')}: ${error}`)
    } finally {
      params.isLoading.value = false
    }
  }

  const loadMoreRequests = async (): Promise<number> => {
    if (params.isLoadingMore.value || !params.hasMore.value) return 0

    params.isLoadingMore.value = true
    try {
      const response = await invoke<any>('list_proxy_requests', {
        limit: params.loadMoreSize,
        offset: params.requests.value.length,
      })

      if (response.success && response.data) {
        const normalizedRequests = normalizeRequests(response.data as ProxyRequest[])
        if (normalizedRequests.length > 0) {
          params.requests.value.push(...normalizedRequests)
          normalizedRequests.forEach((request) => params.requestIdSet.value.add(request.id))
          params.hasMore.value = normalizedRequests.length === params.loadMoreSize
          if (params.requests.value.length > params.maxRequestsInMemory) {
            params.requests.value = params.requests.value.slice(0, params.maxRequestsInMemory)
            params.requestIdSet.value = new Set(params.requests.value.map((request) => request.id))
            params.hasMore.value = false
            updateStats()
          } else {
            updateStatsIncrementalBatch(normalizedRequests)
          }
          pruneProxyHistoryDerivedCache(params.requestIdSet.value)
          return normalizedRequests.length
        } else {
          params.hasMore.value = false
        }
      }
    } catch (error: any) {
      console.error('Failed to load more requests:', error)
      dialog.toast.error(`加载更多失败: ${error}`)
    } finally {
      params.isLoadingMore.value = false
    }

    return 0
  }

  const fetchRequestDetails = async (requestId: number): Promise<ProxyRequest | null> => {
    try {
      const response = await invoke<any>('get_proxy_request', { id: requestId })
      if (!response.success || !response.data) {
        return null
      }
      return mergeRequestIntoList({
        ...response.data,
        has_full_details: true,
      } as ProxyRequest)
    } catch (error) {
      console.error(`Failed to load request details for #${requestId}:`, error)
      return null
    }
  }

  const loadWsConnections = async () => {
    params.isLoadingWs.value = true
    try {
      const response = await invoke<any>('list_websocket_connections', {
        limit: 100,
        offset: 0,
      })
      if (response.success && response.data) {
        params.wsConnections.value = response.data
      }
    } catch (error: any) {
      console.error('Failed to load WebSocket connections:', error)
    } finally {
      params.isLoadingWs.value = false
    }
  }

  const loadWsMessages = async (connectionId: string) => {
    if (params.wsMessagesCache.value.has(connectionId)) {
      return
    }

    try {
      const response = await invoke<any>('list_websocket_messages', {
        connectionId,
        limit: 100,
        offset: 0,
      })
      if (response.success && response.data) {
        const messages = response.data as WebSocketMessage[]
        params.wsMessagesCache.value.set(connectionId, messages)
        const existingIds = new Set(params.wsMessages.value.map((message) => message.id))
        const newMessages = messages.filter((message) => !existingIds.has(message.id))
        params.wsMessages.value = [...params.wsMessages.value, ...newMessages]
      }
    } catch (error: any) {
      console.error('Failed to load WebSocket messages:', error)
    }
  }

  const toggleWsConnection = (connectionId: string) => {
    if (params.expandedWsConnections.value.has(connectionId)) {
      params.expandedWsConnections.value.delete(connectionId)
    } else {
      params.expandedWsConnections.value.add(connectionId)
      if (!params.activeWsTabs.value.has(connectionId)) {
        params.activeWsTabs.value.set(connectionId, 'messages')
      }
      loadWsMessages(connectionId)
    }
    params.expandedWsConnections.value = new Set(params.expandedWsConnections.value)
  }

  const getWsActiveTab = (connectionId: string): ProxyHistoryWsTab =>
    params.activeWsTabs.value.get(connectionId) || 'messages'

  const setWsActiveTab = (connectionId: string, tab: ProxyHistoryWsTab) => {
    params.activeWsTabs.value.set(connectionId, tab)
    params.activeWsTabs.value = new Map(params.activeWsTabs.value)
  }

  const formatWsTime = (timestamp: string): string => {
    try {
      return new Date(timestamp).toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      })
    } catch {
      return timestamp
    }
  }

  const getWsMessagesForConnection = (connectionId: string): WebSocketMessage[] =>
    params.wsMessagesCache.value.get(connectionId)
    || params.wsMessages.value.filter((message) => message.connection_id === connectionId)

  const truncateWsContent = (content?: string, maxLength = 100): string => {
    if (!content) return '[empty]'
    if (content.startsWith('[BASE64]')) return '[binary data]'
    if (content.length <= maxLength) return content
    return `${content.substring(0, maxLength)}...`
  }

  const processPendingUpdates = () => {
    if (pendingUpdates.length === 0) return

    const newRequests = pendingUpdates
      .map(normalizeRequest)
      .filter((request) => !params.requestIdSet.value.has(request.id))
    if (newRequests.length > 0) {
      params.requests.value.unshift(...newRequests)
      newRequests.forEach((request) => params.requestIdSet.value.add(request.id))
      if (params.requests.value.length > params.maxRequestsInMemory) {
        const removed = params.requests.value.splice(params.maxRequestsInMemory)
        removed.forEach((request) => params.requestIdSet.value.delete(request.id))
      }
      pruneProxyHistoryDerivedCache(params.requestIdSet.value)
      newRequests.forEach(updateStatsIncremental)
    }

    pendingUpdates = []
    updateTimer = null
  }

  const setupEventListeners = async () => {
    unlistenRequest = await listen<ProxyRequest>('proxy:request', (event) => {
      pendingUpdates.push(event.payload)
      if (pendingUpdates.length >= params.batchUpdateThreshold) {
        if (updateTimer !== null) {
          clearTimeout(updateTimer)
        }
        processPendingUpdates()
      } else if (updateTimer === null) {
        updateTimer = window.setTimeout(() => {
          processPendingUpdates()
        }, 50)
      }
    })
  }

  const cleanupDataRuntime = () => {
    if (unlistenRequest) {
      unlistenRequest()
      unlistenRequest = null
    }
    if (updateTimer !== null) {
      clearTimeout(updateTimer)
      updateTimer = null
    }
    pendingUpdates = []
    params.requestIdSet.value.clear()
    params.wsMessagesCache.value.clear()
    params.expandedWsConnections.value.clear()
    params.activeWsTabs.value.clear()
    clearProxyHistoryDerivedCache()
  }

  return {
    cleanupDataRuntime,
    formatWsTime,
    getWsActiveTab,
    getWsMessagesForConnection,
    loadMoreRequests,
    loadWsConnections,
    fetchRequestDetails,
    refreshRequests,
    setWsActiveTab,
    setupEventListeners,
    toggleWsConnection,
    truncateWsContent,
    updateStats,
  }
}
