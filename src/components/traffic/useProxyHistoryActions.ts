import { computed, type ComputedRef, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as tauriEmit } from '@tauri-apps/api/event'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useRouter } from 'vue-router'
import { dialog } from '@/composables/useDialog'
import {
  formatRequest,
  formatRequestRaw,
  formatResponseRaw,
  getHarStatusText,
} from './proxyHistoryFormattingSupport'
import type {
  ProxyHistoryRequestTab,
  ProxyHistoryViewMode,
  ProxyRequest,
} from './proxyHistoryTypes'

type ProxyHistoryStats = {
  total: number
  http: number
  https: number
  avgResponseTime: number
}

type ContextMenuState = {
  visible: boolean
  x: number
  y: number
  request: ProxyRequest | null
}

type DetailContextMenuState = {
  visible: boolean
  x: number
  y: number
}

type SendType = 'request' | 'response' | 'both'

type Params = {
  contextMenu: Ref<ContextMenuState>
  detailContextMenu: Ref<DetailContextMenuState>
  showFilterSubmenu: Ref<boolean>
  selectedRequest: Ref<ProxyRequest | null>
  selectedRequests: Ref<Set<number>>
  isMultiSelectMode: Ref<boolean>
  filteredRequests: ComputedRef<ProxyRequest[]>
  requests: Ref<ProxyRequest[]>
  stats: Ref<ProxyHistoryStats>
  requestTab: Ref<ProxyHistoryRequestTab>
  responseTab: Ref<'pretty' | 'raw' | 'hex' | 'render'>
  requestViewMode: Ref<ProxyHistoryViewMode>
  topPanelHeight: Ref<number>
  mainContainer: Ref<HTMLElement | null>
  hideContextMenu?: () => void
  hideDetailContextMenu?: () => void
  emitSendToRepeater: (request: { method: string; url: string; headers: Record<string, string>; body?: string }) => void
  emitSendToIntruder: (request: { method: string; url: string; headers: Record<string, string>; body?: string }) => void
  emitSendToAssistant: (requests: ProxyRequest[]) => void
  emitAddFilterRule: (rule: { matchType: string; condition: string; relationship?: string }) => void
  updateStats: () => void
  t: (key: string, params?: Record<string, unknown>) => string
}

const buildHeaders = (request: ProxyRequest) => {
  if (!request.request_headers) {
    return {}
  }
  try {
    return JSON.parse(request.request_headers) as Record<string, string>
  } catch {
    return {}
  }
}

const buildCurlCommand = (request: ProxyRequest) => {
  let curl = `curl -X ${request.method} '${request.url}'`
  const headers = buildHeaders(request)
  for (const [key, value] of Object.entries(headers)) {
    curl += ` \\\n  -H '${key}: ${value}'`
  }
  if (request.request_body) {
    curl += ` \\\n  -d '${request.request_body.replace(/'/g, "'\\''")}'`
  }
  return curl
}

export const useProxyHistoryActions = (params: Params) => {
  const router = useRouter()

  const clearSelection = () => {
    params.selectedRequests.value.clear()
  }

  const closeDetails = () => {
    params.selectedRequest.value = null
    params.requestTab.value = 'pretty'
    params.responseTab.value = 'pretty'
  }

  const hideContextMenu = () => {
    params.contextMenu.value.visible = false
    params.contextMenu.value.request = null
    params.showFilterSubmenu.value = false
    document.removeEventListener('click', hideContextMenu)
    document.removeEventListener('contextmenu', hideContextMenu)
  }

  const hideDetailContextMenu = () => {
    params.detailContextMenu.value.visible = false
    document.removeEventListener('click', hideDetailContextMenu)
    document.removeEventListener('contextmenu', hideDetailContextMenu)
  }

  const clearHistory = async () => {
    try {
      const response = await invoke<any>('clear_proxy_requests')
      if (response.success) {
        params.requests.value = []
        params.updateStats()
        closeDetails()
        dialog.toast.success('请求历史已清空')
      }
    } catch (error: any) {
      console.error('Failed to clear requests:', error)
      dialog.toast.error(`清空失败: ${error}`)
    }
  }

  const openDetails = (request: ProxyRequest) => {
    params.selectedRequest.value = request
  }

  const selectRequest = (request: ProxyRequest) => {
    if (params.selectedRequest.value?.id === request.id) {
      closeDetails()
      return
    }

    params.selectedRequest.value = request
    params.requestTab.value = 'pretty'
    params.responseTab.value = 'pretty'
    const containerHeight = params.mainContainer.value?.clientHeight || 600
    params.topPanelHeight.value = Math.floor(containerHeight * 0.4)
  }

  const showContextMenu = (event: MouseEvent, request: ProxyRequest) => {
    params.contextMenu.value = {
      visible: true,
      x: event.clientX,
      y: event.clientY,
      request,
    }
    setTimeout(() => {
      document.addEventListener('click', hideContextMenu)
      document.addEventListener('contextmenu', hideContextMenu)
    }, 0)
  }

  const showDetailContextMenu = (event: MouseEvent) => {
    if (!params.selectedRequest.value) return
    params.detailContextMenu.value = {
      visible: true,
      x: Math.min(event.clientX, window.innerWidth - 200),
      y: Math.min(event.clientY, window.innerHeight - 200),
    }
    setTimeout(() => {
      document.addEventListener('click', hideDetailContextMenu)
      document.addEventListener('contextmenu', hideDetailContextMenu)
    }, 0)
  }

  const detailSendToRepeater = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    const request = params.selectedRequest.value
    params.emitSendToRepeater({
      method: request.method,
      url: request.url,
      headers: buildHeaders(request),
      body: request.request_body || undefined,
    })
  }

  const detailSendToIntruder = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    const request = params.selectedRequest.value
    params.emitSendToIntruder({
      method: request.method,
      url: request.url,
      headers: buildHeaders(request),
      body: request.request_body || undefined,
    })
  }

  const detailCopyUrl = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    navigator.clipboard.writeText(params.selectedRequest.value.url)
      .then(() => dialog.toast.success('URL 已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const detailCopyRequest = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    const requestText = formatRequest(
      params.selectedRequest.value,
      params.requestTab.value,
      params.requestViewMode.value,
    )
    navigator.clipboard.writeText(requestText)
      .then(() => dialog.toast.success('请求已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const detailCopyAsCurl = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    navigator.clipboard.writeText(buildCurlCommand(params.selectedRequest.value))
      .then(() => dialog.toast.success('cURL 命令已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const sendToRepeater = () => {
    if (!params.contextMenu.value.request) return
    const request = params.contextMenu.value.request
    params.emitSendToRepeater({
      method: request.method,
      url: request.url,
      headers: buildHeaders(request),
      body: request.request_body || undefined,
    })
    hideContextMenu()
  }

  const sendToIntruder = () => {
    if (!params.contextMenu.value.request) return
    const request = params.contextMenu.value.request
    params.emitSendToIntruder({
      method: request.method,
      url: request.url,
      headers: buildHeaders(request),
      body: request.request_body || undefined,
    })
    hideContextMenu()
  }

  const copyUrl = () => {
    if (!params.contextMenu.value.request) return
    navigator.clipboard.writeText(params.contextMenu.value.request.url)
      .then(() => dialog.toast.success('URL 已复制'))
      .catch(() => dialog.toast.error('复制失败'))
    hideContextMenu()
  }

  const copyAsCurl = () => {
    if (!params.contextMenu.value.request) return
    navigator.clipboard.writeText(buildCurlCommand(params.contextMenu.value.request))
      .then(() => dialog.toast.success('cURL 命令已复制'))
      .catch(() => dialog.toast.error('复制失败'))
    hideContextMenu()
  }

  const openInBrowser = () => {
    if (!params.contextMenu.value.request) return
    window.open(params.contextMenu.value.request.url, '_blank')
    hideContextMenu()
  }

  const clearHistoryFromMenu = () => {
    hideContextMenu()
    clearHistory()
  }

  const addFilterToDomain = () => {
    if (!params.contextMenu.value.request) return
    try {
      const domain = new URL(params.contextMenu.value.request.url).hostname
      params.emitAddFilterRule({
        matchType: 'domain_name',
        condition: domain,
        relationship: 'matches',
      })
      dialog.toast.success(`Added domain filter: ${domain}`)
    } catch {
      dialog.toast.error('Failed to parse URL')
    }
    hideContextMenu()
  }

  const addFilterToUrl = () => {
    if (!params.contextMenu.value.request) return
    const urlPattern = params.contextMenu.value.request.url
    params.emitAddFilterRule({
      matchType: 'url',
      condition: urlPattern,
      relationship: 'matches',
    })
    dialog.toast.success(`Added URL filter: ${urlPattern}`)
    hideContextMenu()
  }

  const addFilterToMethod = () => {
    if (!params.contextMenu.value.request) return
    const method = params.contextMenu.value.request.method.toLowerCase()
    params.emitAddFilterRule({
      matchType: 'http_method',
      condition: method,
      relationship: 'matches',
    })
    dialog.toast.success(`Added method filter: ${method}`)
    hideContextMenu()
  }

  const addFilterToExtension = () => {
    if (!params.contextMenu.value.request) return
    try {
      const pathname = new URL(params.contextMenu.value.request.url).pathname
      const lastDot = pathname.lastIndexOf('.')
      if (lastDot > 0) {
        const extension = pathname.substring(lastDot + 1).split('?')[0]
        params.emitAddFilterRule({
          matchType: 'file_extension',
          condition: `^${extension}$`,
          relationship: 'matches',
        })
        dialog.toast.success(`Added extension filter: ${extension}`)
      } else {
        dialog.toast.warning('No file extension found in URL')
      }
    } catch {
      dialog.toast.error('Failed to parse URL')
    }
    hideContextMenu()
  }

  const toggleMultiSelectMode = () => {
    params.isMultiSelectMode.value = !params.isMultiSelectMode.value
    if (!params.isMultiSelectMode.value) {
      clearSelection()
    }
  }

  const toggleSelectRequest = (request: ProxyRequest) => {
    if (params.selectedRequests.value.has(request.id)) {
      params.selectedRequests.value.delete(request.id)
    } else {
      params.selectedRequests.value.add(request.id)
    }
  }

  const selectAllVisible = () => {
    params.filteredRequests.value.forEach((request) => {
      params.selectedRequests.value.add(request.id)
    })
  }

  const isRequestSelected = (request: ProxyRequest) => params.selectedRequests.value.has(request.id)

  const sendSelectedToAssistant = async (type: SendType = 'both') => {
    const selected = params.filteredRequests.value.filter((request) => params.selectedRequests.value.has(request.id))
    if (selected.length === 0) {
      dialog.toast.warning('请先选择要发送的请求')
      return
    }
    await tauriEmit('traffic:send-to-assistant', { requests: selected, type })
    params.emitSendToAssistant(selected)
    const typeText = type === 'request' ? '请求' : type === 'response' ? '响应' : '流量'
    dialog.toast.success(`已发送 ${selected.length} 条${typeText}到 AI 助手`)
    clearSelection()
    params.isMultiSelectMode.value = false
    router.push('/ai-assistant')
  }

  const sendSingleToAssistant = async (request: ProxyRequest, type: SendType = 'both') => {
    await tauriEmit('traffic:send-to-assistant', { requests: [request], type })
    params.emitSendToAssistant([request])
    const typeText = type === 'request' ? '请求' : type === 'response' ? '响应' : '流量'
    dialog.toast.success(`已发送${typeText}到 AI 助手`)
    router.push('/ai-assistant')
  }

  const sendRequestToAssistantFromMenu = () => {
    if (!params.contextMenu.value.request) return
    sendSingleToAssistant(params.contextMenu.value.request, 'request')
    hideContextMenu()
  }

  const sendResponseToAssistantFromMenu = () => {
    if (!params.contextMenu.value.request) return
    sendSingleToAssistant(params.contextMenu.value.request, 'response')
    hideContextMenu()
  }

  const detailSendRequestToAssistant = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    sendSingleToAssistant(params.selectedRequest.value, 'request')
  }

  const detailSendResponseToAssistant = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    sendSingleToAssistant(params.selectedRequest.value, 'response')
  }

  const exportSelectedToFile = async (type: 'request' | 'response') => {
    const selected = params.filteredRequests.value.filter((request) => params.selectedRequests.value.has(request.id))
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.export.noSelection'))
      return
    }

    try {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5)
      const filePath = await save({
        defaultPath: `${type}-export-${timestamp}.txt`,
        filters: [
          { name: 'Text Files', extensions: ['txt'] },
          { name: 'HTTP Files', extensions: ['http'] },
          { name: 'JSON Files', extensions: ['json'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      })
      if (!filePath) return

      let content = ''
      if (filePath.endsWith('.json')) {
        content = JSON.stringify(selected.map((request) => ({
          id: request.id,
          url: request.url,
          method: request.method,
          host: request.host,
          protocol: request.protocol,
          status_code: request.status_code,
          request_headers: request.request_headers,
          request_body: request.request_body,
          response_headers: request.response_headers,
          response_body: request.response_body,
          response_size: request.response_size,
          response_time: request.response_time,
          timestamp: request.timestamp,
          was_edited: request.was_edited,
        })), null, 2)
      } else {
        selected.forEach((request, index) => {
          if (index > 0) {
            content += `\n\n${'='.repeat(80)}\n\n`
          }
          content += `# ${type.toUpperCase()} ${index + 1}/${selected.length}\n`
          content += `# URL: ${request.url}\n`
          content += `# Method: ${request.method}\n`
          content += `# Status: ${request.status_code || 'N/A'}\n`
          content += `# Timestamp: ${request.timestamp}\n`
          if (request.was_edited) {
            content += '# Modified: Yes\n'
          }
          content += '\n'
          content += type === 'request'
            ? formatRequestRaw(request, 'edited')
            : formatResponseRaw(request, 'edited')
        })
      }

      await writeTextFile(filePath, content)
      const typeText = type === 'request'
        ? params.t('trafficAnalysis.history.export.request')
        : params.t('trafficAnalysis.history.export.response')
      dialog.toast.success(params.t('trafficAnalysis.history.export.success', {
        count: selected.length,
        type: typeText,
      }))
      clearSelection()
      params.isMultiSelectMode.value = false
    } catch (error) {
      console.error('Export failed:', error)
      dialog.toast.error(params.t('trafficAnalysis.history.export.failed', { error: String(error) }))
    }
  }

  const exportAsHAR = async () => {
    const selected = Array.from(params.selectedRequests.value)
      .map((id) => params.requests.value.find((request) => request.id === id))
      .filter((request): request is ProxyRequest => Boolean(request))
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.export.noSelection'))
      return
    }

    try {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5)
      const filePath = await save({
        defaultPath: `traffic-export-${timestamp}.har`,
        filters: [
          { name: 'HAR Files', extensions: ['har'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      })
      if (!filePath) return

      const har = {
        log: {
          version: '1.2',
          creator: { name: 'Sentinel AI', version: '1.0.0' },
          entries: selected.map((request) => {
            const requestHeaders = buildHeaders(request)
            let responseHeaders: Record<string, string> = {}
            if (request.response_headers) {
              try {
                responseHeaders = JSON.parse(request.response_headers) as Record<string, string>
              } catch {
                responseHeaders = {}
              }
            }
            const urlObj = new URL(request.url)
            return {
              startedDateTime: request.timestamp,
              time: request.response_time || 0,
              request: {
                method: request.method,
                url: request.url,
                httpVersion: 'HTTP/1.1',
                headers: Object.entries(requestHeaders).map(([name, value]) => ({ name, value })),
                queryString: Array.from(urlObj.searchParams.entries()).map(([name, value]) => ({ name, value })),
                cookies: [],
                headersSize: -1,
                bodySize: request.request_body ? request.request_body.length : 0,
                postData: request.request_body ? {
                  mimeType: requestHeaders['content-type'] || 'text/plain',
                  text: request.request_body,
                } : undefined,
              },
              response: {
                status: request.status_code || 0,
                statusText: getHarStatusText(request.status_code),
                httpVersion: 'HTTP/1.1',
                headers: Object.entries(responseHeaders).map(([name, value]) => ({ name, value })),
                cookies: [],
                content: {
                  size: request.response_size || 0,
                  mimeType: responseHeaders['content-type'] || 'text/plain',
                  text: request.response_body || '',
                },
                redirectURL: '',
                headersSize: -1,
                bodySize: request.response_size || 0,
              },
              cache: {},
              timings: {
                send: 0,
                wait: request.response_time || 0,
                receive: 0,
              },
            }
          }),
        },
      }

      await writeTextFile(filePath, JSON.stringify(har, null, 2))
      dialog.toast.success(params.t('trafficAnalysis.history.messages.exportSuccess'))
      clearSelection()
      params.isMultiSelectMode.value = false
    } catch (error) {
      console.error('HAR export failed:', error)
      dialog.toast.error(params.t('trafficAnalysis.history.export.failed', { error: String(error) }))
    }
  }

  const cleanupActionRuntime = () => {
    document.removeEventListener('click', hideContextMenu)
    document.removeEventListener('contextmenu', hideContextMenu)
    document.removeEventListener('click', hideDetailContextMenu)
    document.removeEventListener('contextmenu', hideDetailContextMenu)
  }

  const selectedCount = computed(() => params.selectedRequests.value.size)

  return {
    addFilterToDomain,
    addFilterToExtension,
    addFilterToMethod,
    addFilterToUrl,
    cleanupActionRuntime,
    clearHistory,
    clearHistoryFromMenu,
    clearSelection,
    closeDetails,
    copyAsCurl,
    copyUrl,
    detailCopyAsCurl,
    detailCopyRequest,
    detailCopyUrl,
    detailSendRequestToAssistant,
    detailSendResponseToAssistant,
    detailSendToIntruder,
    detailSendToRepeater,
    exportAsHAR,
    exportSelectedToFile,
    hideContextMenu,
    hideDetailContextMenu,
    isRequestSelected,
    openDetails,
    openInBrowser,
    selectAllVisible,
    selectRequest,
    selectedCount,
    sendRequestToAssistantFromMenu,
    sendResponseToAssistantFromMenu,
    sendSelectedToAssistant,
    sendToIntruder,
    sendToRepeater,
    showContextMenu,
    showDetailContextMenu,
    toggleMultiSelectMode,
    toggleSelectRequest,
  }
}
