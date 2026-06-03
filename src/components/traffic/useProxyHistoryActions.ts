import { computed, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as tauriEmit } from '@tauri-apps/api/event'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { dialog } from '@/composables/useDialog'
import { openTrafficAssistantPanel } from '@/services/trafficAssistantWorkspace'
import type { ReferencedTraffic, TrafficSendType } from '@/types/agentReferences'
import type { HttpExchangeRequest } from './http/model'
import { parseStoredHeaderEntries } from './http/headers'
import { clearProxyHistoryDerivedCache } from './proxyHistoryDerivedSupport'
import {
  buildHttpExchangeRequestFromHistory,
  normalizeProxyHistoryHttpVersion,
} from './proxyHistoryHttpSupport'
import {
  formatRequestRaw,
  formatResponseRaw,
  getResponseContentType,
  getHarStatusText,
  hasEditedRequest,
  hasEditedResponse,
} from './proxyHistoryFormattingSupport'
import {
  buildRequestVersionComparePayload,
  buildResponseVersionComparePayload,
} from './trafficHistoryComparerSupport'
import { buildProxyHistoryRangeSelectionIds } from './proxyHistorySelectionChangeSupport'
import type {
  ProxyHistoryRequestTab,
  ProxyHistoryViewMode,
  ProxyRequest,
} from './proxyHistoryTypes'
import type { TrafficComparePayload, TrafficComparerDraftRequestInput } from './transfers'

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
  pane: 'request' | 'response'
}

type SendType = TrafficSendType

const REQUEST_RESPONSE_PREVIEW_LIMIT = 4096
const RESPONSE_PREVIEW_LIMIT = 8192

type Params = {
  contextMenu: Ref<ContextMenuState>
  detailContextMenu: Ref<DetailContextMenuState>
  selectedRequest: Ref<ProxyRequest | null>
  selectedRequests: Ref<Set<number>>
  isMultiSelectMode: Ref<boolean>
  filteredRequests: Ref<ProxyRequest[]>
  orderedRequests: Ref<ProxyRequest[]>
  requests: Ref<ProxyRequest[]>
  stats: Ref<ProxyHistoryStats>
  requestTab: Ref<ProxyHistoryRequestTab>
  responseTab: Ref<'pretty' | 'raw' | 'hex' | 'render'>
  requestViewMode: Ref<ProxyHistoryViewMode>
  responseViewMode: Ref<ProxyHistoryViewMode>
  topPanelHeight: Ref<number>
  mainContainer: Ref<HTMLElement | null>
  keepDetailsOpenOnRepeatSelect?: boolean
  hideContextMenu?: () => void
  hideDetailContextMenu?: () => void
  emitCreateDraft: (request: HttpExchangeRequest) => void
  emitCreateAttackWorkspace: (request: HttpExchangeRequest) => void
  emitOpenDraftCompare: (payload: TrafficComparerDraftRequestInput) => void
  emitOpenCompare: (payload: TrafficComparePayload) => void
  emitSendToAssistant: (requests: ReferencedTraffic[]) => void
  emitAddFilterRule: (rule: { matchType: string; condition: string; relationship?: string }) => void
  fetchRequestDetails: (requestId: number) => Promise<ProxyRequest | null>
  updateStats: () => void
  t: (key: string, params?: Record<string, unknown>) => string
}

const buildHeaders = (request: ProxyRequest) => parseStoredHeaderEntries(request.request_headers)

const binaryResponseContentTypes = [
  'application/octet-stream',
  'application/pdf',
  'application/zip',
  'application/x-gzip',
  'application/x-tar',
  'audio/',
  'font/',
  'image/',
  'video/',
]

const isBinaryResponse = (contentType: string, body?: string) => {
  const normalized = contentType.toLowerCase()
  return Boolean(
    body?.startsWith('[BASE64]')
    || binaryResponseContentTypes.some((item) => normalized.includes(item)),
  )
}

const buildResponseBodyPreview = (
  request: ProxyRequest,
  body: string | undefined,
  contentType: string,
  limit: number,
) => {
  if (!body) {
    return {
      preview: undefined,
      truncated: false,
      available: request.response_size <= 0,
    }
  }

  if (isBinaryResponse(contentType, body)) {
    return {
      preview: `[Binary response omitted: ${contentType || 'unknown content type'}, ${request.response_size || body.length} bytes]`,
      truncated: true,
      available: true,
    }
  }

  return {
    preview: body.length > limit ? body.slice(0, limit) : body,
    truncated: body.length > limit,
    available: true,
  }
}

const buildReferencedTrafficPayload = (request: ProxyRequest, type: SendType): ReferencedTraffic => {
  const useEditedRequest = hasEditedRequest(request)
  const useEditedResponse = hasEditedResponse(request)
  const responseBody = useEditedResponse && request.edited_response_body
    ? request.edited_response_body
    : request.response_body
  const responseContentType = getResponseContentType(request, useEditedResponse ? 'edited' : 'original')
  const responsePreview = buildResponseBodyPreview(
    request,
    responseBody,
    responseContentType,
    type === 'request' ? REQUEST_RESPONSE_PREVIEW_LIMIT : RESPONSE_PREVIEW_LIMIT,
  )

  return {
    id: request.id,
    db_request_id: request.db_request_id ?? null,
    url: useEditedRequest && request.edited_url ? request.edited_url : request.url,
    method: useEditedRequest && request.edited_method ? request.edited_method : request.method,
    host: request.host,
    status_code: useEditedResponse && request.edited_status_code ? request.edited_status_code : request.status_code,
    request_headers: useEditedRequest && request.edited_request_headers ? request.edited_request_headers : request.request_headers,
    request_body: useEditedRequest && request.edited_request_body ? request.edited_request_body : request.request_body,
    response_headers: useEditedResponse && request.edited_response_headers ? request.edited_response_headers : request.response_headers,
    response_body_preview: responsePreview.preview,
    response_body_truncated: responsePreview.truncated,
    response_body_available: responsePreview.available,
    response_size: request.response_size,
    response_time: request.response_time,
    response_content_type: responseContentType,
    sendType: type,
  }
}

const buildCurlCommand = (request: ProxyRequest) => {
  let curl = `curl -X ${request.method} '${request.url}'`
  const headers = buildHeaders(request)
  for (const header of headers) {
    curl += ` \\\n  -H '${header.name}: ${header.value}'`
  }
  if (request.request_body) {
    curl += ` \\\n  -d '${request.request_body.replace(/'/g, "'\\''")}'`
  }
  return curl
}

export const useProxyHistoryActions = (params: Params) => {
  const compareVersionLabels = {
    requestVersions: params.t('trafficAnalysis.history.batchCompare.requestVersions'),
    responseVersions: params.t('trafficAnalysis.history.batchCompare.responseVersions'),
    originalRequest: params.t('trafficAnalysis.history.detailsPanel.originalRequest'),
    editedRequest: params.t('trafficAnalysis.history.detailsPanel.editedRequest'),
    originalResponse: params.t('trafficAnalysis.history.detailsPanel.originalResponse'),
    editedResponse: params.t('trafficAnalysis.history.detailsPanel.editedResponse'),
  }

  const needsFullRequestDetails = (request: ProxyRequest) => request.has_full_details === false
  let selectionAnchorRequestId: number | null = null

  const resolveRequestDetails = async (request: ProxyRequest | null): Promise<ProxyRequest | null> => {
    if (!request) return null
    if (!needsFullRequestDetails(request)) return request
    return (await params.fetchRequestDetails(request.id)) || request
  }

  const resolveRequestDetailsBatch = async (requests: ProxyRequest[]) =>
    Promise.all(requests.map(async (request) => (await resolveRequestDetails(request)) || request))

  const clearSelection = () => {
    params.selectedRequests.value.clear()
    selectionAnchorRequestId = null
  }

  const getSelectedRequests = () =>
    params.orderedRequests.value.filter((request) => params.selectedRequests.value.has(request.id))

  const getContextActionRequests = () => {
    const request = params.contextMenu.value.request
    if (!request) {
      return []
    }

    if (params.selectedRequests.value.size > 1 && params.selectedRequests.value.has(request.id)) {
      return getSelectedRequests()
    }

    return [request]
  }

  const resetMultiSelection = () => {
    clearSelection()
    params.isMultiSelectMode.value = false
  }

  const closeDetails = () => {
    params.selectedRequest.value = null
    params.requestTab.value = 'pretty'
    params.responseTab.value = 'pretty'
  }

  const hideContextMenu = () => {
    params.contextMenu.value.visible = false
    params.contextMenu.value.request = null
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
        clearProxyHistoryDerivedCache()
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
    selectionAnchorRequestId = request.id
    if (params.selectedRequest.value?.id === request.id) {
      if (params.keepDetailsOpenOnRepeatSelect === false) {
        closeDetails()
      }
      return
    }

    const detailsWereClosed = !params.selectedRequest.value
    params.selectedRequest.value = request
    params.requestTab.value = 'pretty'
    params.responseTab.value = 'pretty'
    if (detailsWereClosed) {
      const containerHeight = params.mainContainer.value?.clientHeight || 600
      const maxTopPanelHeight = Math.max(160, containerHeight - 220)
      if (params.topPanelHeight.value < 160 || params.topPanelHeight.value > maxTopPanelHeight) {
        params.topPanelHeight.value = Math.floor(containerHeight * 0.4)
      }
    }
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

  const showDetailContextMenu = (event: MouseEvent, pane: 'request' | 'response' = 'request') => {
    if (!params.selectedRequest.value) return
    params.detailContextMenu.value = {
      visible: true,
      x: Math.min(event.clientX, window.innerWidth - 200),
      y: Math.min(event.clientY, window.innerHeight - 200),
      pane,
    }
    setTimeout(() => {
      document.addEventListener('click', hideDetailContextMenu)
      document.addEventListener('contextmenu', hideDetailContextMenu)
    }, 0)
  }

  const detailSendToRepeater = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    params.emitCreateDraft(buildHttpExchangeRequestFromHistory(request))
  }

  const detailSendToIntruder = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    params.emitCreateAttackWorkspace(buildHttpExchangeRequestFromHistory(request))
  }

  const detailSendToComparer = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return

    if (params.detailContextMenu.value.pane === 'response') {
      params.emitOpenDraftCompare({
        text: formatResponseRaw(request, params.responseViewMode.value),
        name: request.host || request.url,
        label: params.t('trafficAnalysis.history.detailsPanel.response'),
      })
      return
    }

    params.emitOpenDraftCompare({
      request: buildHttpExchangeRequestFromHistory(request),
      name: request.host || request.url,
      label: params.t('trafficAnalysis.history.detailsPanel.request'),
    })
  }

  const detailCompareRequestVersions = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    const payload = buildRequestVersionComparePayload(request, compareVersionLabels)
    if (!payload) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedRequestVersion'))
      return
    }
    params.emitOpenCompare(payload)
    dialog.toast.success(params.t('trafficAnalysis.history.messages.compareOpened'))
  }

  const detailCompareResponseVersions = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    const payload = buildResponseVersionComparePayload(request, compareVersionLabels)
    if (!payload) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedResponseVersion'))
      return
    }
    params.emitOpenCompare(payload)
    dialog.toast.success(params.t('trafficAnalysis.history.messages.compareOpened'))
  }

  const detailCopyUrl = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    navigator.clipboard.writeText(params.selectedRequest.value.url)
      .then(() => dialog.toast.success('URL 已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const detailCopyRequest = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    const requestText = formatRequestRaw(request, params.requestViewMode.value)
    navigator.clipboard.writeText(requestText)
      .then(() => dialog.toast.success('请求已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const detailCopyAsCurl = async () => {
    hideDetailContextMenu()
    const request = await resolveRequestDetails(params.selectedRequest.value)
    if (!request) return
    navigator.clipboard.writeText(buildCurlCommand(request))
      .then(() => dialog.toast.success('cURL 命令已复制'))
      .catch(() => dialog.toast.error('复制失败'))
  }

  const createDraft = async () => {
    const requests = getContextActionRequests()
    if (requests.length === 0) return
    const detailedRequests = await resolveRequestDetailsBatch(requests)
    detailedRequests.forEach((request) => {
      params.emitCreateDraft(buildHttpExchangeRequestFromHistory(request))
    })
    if (detailedRequests.length > 1) {
      dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToRepeater', {
        count: detailedRequests.length,
      }))
      resetMultiSelection()
    }
    hideContextMenu()
  }

  const createAttackWorkspace = async () => {
    const requests = getContextActionRequests()
    if (requests.length === 0) return
    const detailedRequests = await resolveRequestDetailsBatch(requests)
    detailedRequests.forEach((request) => {
      params.emitCreateAttackWorkspace(buildHttpExchangeRequestFromHistory(request))
    })
    if (detailedRequests.length > 1) {
      dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToIntruder', {
        count: detailedRequests.length,
      }))
      resetMultiSelection()
    }
    hideContextMenu()
  }

  const openDraftCompare = async () => {
    const requests = getContextActionRequests()
    if (requests.length === 0) return
    const detailedRequests = await resolveRequestDetailsBatch(requests)
    detailedRequests.forEach((request) => {
      params.emitOpenDraftCompare({
        request: buildHttpExchangeRequestFromHistory(request),
        name: request.host || request.url,
        label: params.t('trafficAnalysis.history.detailsPanel.request'),
      })
    })
    if (detailedRequests.length > 1) {
      dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchRequestsToComparer', {
        count: detailedRequests.length,
      }))
      resetMultiSelection()
    }
    hideContextMenu()
  }

  const compareRequestVersions = async () => {
    const request = await resolveRequestDetails(params.contextMenu.value.request)
    if (!request) return
    const payload = buildRequestVersionComparePayload(request, compareVersionLabels)
    if (!payload) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedRequestVersion'))
      hideContextMenu()
      return
    }
    params.emitOpenCompare(payload)
    dialog.toast.success(params.t('trafficAnalysis.history.messages.compareOpened'))
    hideContextMenu()
  }

  const compareResponseVersions = async () => {
    const request = await resolveRequestDetails(params.contextMenu.value.request)
    if (!request) return
    const payload = buildResponseVersionComparePayload(request, compareVersionLabels)
    if (!payload) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedResponseVersion'))
      hideContextMenu()
      return
    }
    params.emitOpenCompare(payload)
    dialog.toast.success(params.t('trafficAnalysis.history.messages.compareOpened'))
    hideContextMenu()
  }

  const copyUrl = () => {
    if (!params.contextMenu.value.request) return
    navigator.clipboard.writeText(params.contextMenu.value.request.url)
      .then(() => dialog.toast.success('URL 已复制'))
      .catch(() => dialog.toast.error('复制失败'))
    hideContextMenu()
  }

  const copyAsCurl = async () => {
    const request = await resolveRequestDetails(params.contextMenu.value.request)
    if (!request) return
    navigator.clipboard.writeText(buildCurlCommand(request))
      .then(() => dialog.toast.success('cURL 命令已复制'))
      .catch(() => dialog.toast.error('复制失败'))
    hideContextMenu()
  }

  const copyRequest = async () => {
    const request = await resolveRequestDetails(params.contextMenu.value.request)
    if (!request) return
    navigator.clipboard.writeText(formatRequestRaw(request))
      .then(() => dialog.toast.success('请求已复制'))
      .catch(() => dialog.toast.error('复制失败'))
    hideContextMenu()
  }

  const openInBrowser = () => {
    if (!params.contextMenu.value.request) return
    window.open(params.contextMenu.value.request.url, '_blank')
    hideContextMenu()
  }

  const detailOpenInBrowser = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    window.open(params.selectedRequest.value.url, '_blank')
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

  const toggleSelectRequest = (request: ProxyRequest, event?: MouseEvent | Event) => {
    if (event && 'shiftKey' in event && event.shiftKey) {
      params.isMultiSelectMode.value = true
      const anchorRequestId = selectionAnchorRequestId ?? params.selectedRequest.value?.id ?? request.id
      buildProxyHistoryRangeSelectionIds(params.orderedRequests.value, anchorRequestId, request.id).forEach((id) => {
        params.selectedRequests.value.add(id)
      })
      selectionAnchorRequestId = request.id
      return
    }

    if (params.selectedRequests.value.has(request.id)) {
      params.selectedRequests.value.delete(request.id)
    } else {
      params.selectedRequests.value.add(request.id)
    }
    selectionAnchorRequestId = request.id
  }

  const selectAllVisible = () => {
    params.filteredRequests.value.forEach((request) => {
      params.selectedRequests.value.add(request.id)
    })
  }

  const isRequestSelected = (request: ProxyRequest) => params.selectedRequests.value.has(request.id)

  const sendSelectedToAssistant = async (type: SendType = 'request') => {
    const selected = getSelectedRequests()
    if (selected.length === 0) {
      dialog.toast.warning('请先选择要发送的请求')
      return
    }
    const detailedSelected = await resolveRequestDetailsBatch(selected)
    openTrafficAssistantPanel()
    const referencedTraffic = detailedSelected.map((request) => buildReferencedTrafficPayload(request, type))
    await tauriEmit('traffic:send-to-assistant', { requests: referencedTraffic, type })
    params.emitSendToAssistant(referencedTraffic)
    dialog.toast.success(`已发送 ${detailedSelected.length} 条请求到 AI 助手`)
    resetMultiSelection()
  }

  const sendSelectedToDraft = async () => {
    const selected = getSelectedRequests()
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noSelectionForSend'))
      return
    }

    const detailedSelected = await resolveRequestDetailsBatch(selected)
    detailedSelected.forEach((request) => {
      params.emitCreateDraft(buildHttpExchangeRequestFromHistory(request))
    })
    dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToRepeater', {
      count: detailedSelected.length,
    }))
    resetMultiSelection()
  }

  const sendSelectedToComparer = async () => {
    const selected = getSelectedRequests()
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noSelectionForComparer'))
      return
    }

    const detailedSelected = await resolveRequestDetailsBatch(selected)
    detailedSelected.forEach((request) => {
      params.emitOpenDraftCompare({
        request: buildHttpExchangeRequestFromHistory(request),
        name: request.host || request.url,
        label: params.t('trafficAnalysis.history.detailsPanel.request'),
      })
    })
    dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchRequestsToComparer', {
      count: detailedSelected.length,
    }))
    resetMultiSelection()
  }

  const sendSelectedToIntruder = async () => {
    const selected = getSelectedRequests()
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noSelectionForSend'))
      return
    }

    const detailedSelected = await resolveRequestDetailsBatch(selected)
    detailedSelected.forEach((request) => {
      params.emitCreateAttackWorkspace(buildHttpExchangeRequestFromHistory(request))
    })
    dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToIntruder', {
      count: detailedSelected.length,
    }))
    resetMultiSelection()
  }

  const sendSelectedRequestVersionsToComparer = async () => {
    const selected = params.filteredRequests.value.filter((request) => params.selectedRequests.value.has(request.id))
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noSelectionForComparer'))
      return
    }

    const detailedSelected = await resolveRequestDetailsBatch(selected)
    const payloads = detailedSelected
      .map((request) => buildRequestVersionComparePayload(request, compareVersionLabels))
      .filter((payload): payload is TrafficComparePayload => Boolean(payload))

    if (payloads.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedRequestVersion'))
      return
    }

    payloads.forEach((payload) => params.emitOpenCompare(payload))
    dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToComparer', { count: payloads.length }))
    resetMultiSelection()
  }

  const sendSelectedResponseVersionsToComparer = async () => {
    const selected = params.filteredRequests.value.filter((request) => params.selectedRequests.value.has(request.id))
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noSelectionForComparer'))
      return
    }

    const detailedSelected = await resolveRequestDetailsBatch(selected)
    const payloads = detailedSelected
      .map((request) => buildResponseVersionComparePayload(request, compareVersionLabels))
      .filter((payload): payload is TrafficComparePayload => Boolean(payload))

    if (payloads.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.messages.noEditedResponseVersion'))
      return
    }

    payloads.forEach((payload) => params.emitOpenCompare(payload))
    dialog.toast.success(params.t('trafficAnalysis.history.messages.sentBatchToComparer', { count: payloads.length }))
    resetMultiSelection()
  }

  const sendSingleToAssistant = async (request: ProxyRequest, type: SendType = 'request') => {
    const detailedRequest = (await resolveRequestDetails(request)) || request
    const referencedTraffic = buildReferencedTrafficPayload(detailedRequest, type)
    openTrafficAssistantPanel()
    await tauriEmit('traffic:send-to-assistant', { requests: [referencedTraffic], type })
    params.emitSendToAssistant([referencedTraffic])
    dialog.toast.success('已发送请求到 AI 助手')
  }

  const sendRequestToAssistantFromMenu = () => {
    if (!params.contextMenu.value.request) return
    sendSingleToAssistant(params.contextMenu.value.request, 'request')
    hideContextMenu()
  }

  const detailSendRequestToAssistant = () => {
    hideDetailContextMenu()
    if (!params.selectedRequest.value) return
    sendSingleToAssistant(params.selectedRequest.value, 'request')
  }

  const exportSelectedToFile = async (type: 'request' | 'response') => {
    const selected = params.filteredRequests.value.filter((request) => params.selectedRequests.value.has(request.id))
    if (selected.length === 0) {
      dialog.toast.warning(params.t('trafficAnalysis.history.export.noSelection'))
      return
    }

    try {
      const detailedSelected = await resolveRequestDetailsBatch(selected)
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
        content = JSON.stringify(detailedSelected.map((request) => {
          const isEdited = hasEditedRequest(request) || hasEditedResponse(request)
          return {
          id: request.id,
          url: request.url,
          method: request.method,
          host: request.host,
          scheme: request.scheme,
          http_version_observed: request.http_version_observed,
          status_code: request.status_code,
          request_headers: request.request_headers,
          request_body: request.request_body,
          response_headers: request.response_headers,
          response_body: request.response_body,
          response_size: request.response_size,
          response_time: request.response_time,
          timestamp: request.timestamp,
          was_edited: isEdited,
          }
        }), null, 2)
      } else {
        detailedSelected.forEach((request, index) => {
          const isEdited = hasEditedRequest(request) || hasEditedResponse(request)
          if (index > 0) {
            content += `\n\n${'='.repeat(80)}\n\n`
          }
          content += `# ${type.toUpperCase()} ${index + 1}/${detailedSelected.length}\n`
          content += `# URL: ${request.url}\n`
          content += `# Method: ${request.method}\n`
          content += `# Status: ${request.status_code || 'N/A'}\n`
          content += `# Timestamp: ${request.timestamp}\n`
          if (isEdited) {
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
        count: detailedSelected.length,
        type: typeText,
      }))
      resetMultiSelection()
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
      const detailedSelected = await resolveRequestDetailsBatch(selected)
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
          entries: detailedSelected.map((request) => {
            const requestHeaders = buildHeaders(request)
            const responseHeaders = parseStoredHeaderEntries(request.response_headers)
            const requestContentType = requestHeaders.find((header) => header.name.toLowerCase() === 'content-type')?.value
            const responseContentType = responseHeaders.find((header) => header.name.toLowerCase() === 'content-type')?.value
            const httpVersion = normalizeProxyHistoryHttpVersion(request.http_version_observed)
            const urlObj = new URL(request.url)
            return {
              startedDateTime: request.timestamp,
              time: request.response_time || 0,
              request: {
                method: request.method,
                url: request.url,
                httpVersion,
                headers: requestHeaders.map((header) => ({ name: header.name, value: header.value })),
                queryString: Array.from(urlObj.searchParams.entries()).map(([name, value]) => ({ name, value })),
                cookies: [],
                headersSize: -1,
                bodySize: request.request_body ? request.request_body.length : 0,
                postData: request.request_body ? {
                  mimeType: requestContentType || 'text/plain',
                  text: request.request_body,
                } : undefined,
              },
              response: {
                status: request.status_code || 0,
                statusText: getHarStatusText(request.status_code),
                httpVersion,
                headers: responseHeaders.map((header) => ({ name: header.name, value: header.value })),
                cookies: [],
                content: {
                  size: request.response_size || 0,
                  mimeType: responseContentType || 'text/plain',
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
      resetMultiSelection()
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
    copyRequest,
    compareRequestVersions,
    compareResponseVersions,
    copyUrl,
    detailCopyAsCurl,
    detailCopyRequest,
    detailCopyUrl,
    detailOpenInBrowser,
    detailCompareRequestVersions,
    detailCompareResponseVersions,
    detailSendRequestToAssistant,
    detailSendToComparer,
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
    sendSelectedRequestVersionsToComparer,
    sendSelectedResponseVersionsToComparer,
    sendSelectedToComparer,
    sendSelectedToDraft,
    sendSelectedToIntruder,
    sendSelectedToAssistant,
    openDraftCompare,
    createAttackWorkspace,
    createDraft,
    showContextMenu,
    showDetailContextMenu,
    toggleMultiSelectMode,
    toggleSelectRequest,
  }
}
