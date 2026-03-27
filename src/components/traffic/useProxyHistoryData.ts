import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Ref } from 'vue'
import { dialog } from '@/composables/useDialog'
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

  const updateStats = () => {
    const total = params.requests.value.length
    const https = params.requests.value.filter((request) => request.protocol === 'https').length
    const http = params.requests.value.filter((request) => request.protocol === 'http').length
    const totalResponseTime = params.requests.value.reduce((sum, request) => sum + request.response_time, 0)

    params.stats.value = {
      total,
      http,
      https,
      avgResponseTime: total > 0 ? Math.round(totalResponseTime / total) : 0,
    }
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

  const refreshRequests = async () => {
    params.isLoading.value = true
    try {
      const response = await invoke<any>('list_proxy_requests', {
        limit: params.initialLoadLimit,
        offset: 0,
      })

      if (response.success && response.data) {
        params.requests.value = response.data
        params.hasMore.value = response.data.length === params.initialLoadLimit
        params.requestIdSet.value.clear()
        response.data.forEach((request: ProxyRequest) => params.requestIdSet.value.add(request.id))
        updateStats()
      }
    } catch (error: any) {
      console.error('Failed to refresh requests:', error)
      dialog.toast.error(`${params.t('trafficAnalysis.history.errors.loadFailed')}: ${error}`)
    } finally {
      params.isLoading.value = false
    }
  }

  const loadMoreRequests = async () => {
    if (params.isLoadingMore.value || !params.hasMore.value) return

    params.isLoadingMore.value = true
    try {
      const response = await invoke<any>('list_proxy_requests', {
        limit: params.loadMoreSize,
        offset: params.requests.value.length,
      })

      if (response.success && response.data) {
        if (response.data.length > 0) {
          params.requests.value = [...params.requests.value, ...response.data]
          params.hasMore.value = response.data.length === params.loadMoreSize
          if (params.requests.value.length > params.maxRequestsInMemory) {
            params.requests.value = params.requests.value.slice(0, params.maxRequestsInMemory)
            params.hasMore.value = false
          }
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

    const newRequests = pendingUpdates.filter((request) => !params.requestIdSet.value.has(request.id))
    if (newRequests.length > 0) {
      params.requests.value = [...newRequests, ...params.requests.value]
      newRequests.forEach((request) => params.requestIdSet.value.add(request.id))
      if (params.requests.value.length > params.maxRequestsInMemory) {
        const removed = params.requests.value.slice(params.maxRequestsInMemory)
        params.requests.value = params.requests.value.slice(0, params.maxRequestsInMemory)
        removed.forEach((request) => params.requestIdSet.value.delete(request.id))
      }
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
  }

  return {
    cleanupDataRuntime,
    formatWsTime,
    getWsActiveTab,
    getWsMessagesForConnection,
    loadMoreRequests,
    loadWsConnections,
    refreshRequests,
    setWsActiveTab,
    setupEventListeners,
    toggleWsConnection,
    truncateWsContent,
    updateStats,
  }
}
