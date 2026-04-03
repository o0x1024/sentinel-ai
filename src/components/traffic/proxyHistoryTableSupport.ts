import type { Column, ProxyHistorySortDirection, ProxyHistorySortState, ProxyRequest, VirtualItem } from './proxyHistoryTypes'
import { getProxyHistoryDerived } from './proxyHistoryDerivedSupport'

export const PROXY_HISTORY_COLUMNS_STORAGE_KEY = 'proxyHistory.columns'
export const PROXY_HISTORY_SORT_STORAGE_KEY = 'proxyHistory.sort'
export const PROXY_HISTORY_ITEM_HEIGHT = 18
export const PROXY_HISTORY_HEADER_HEIGHT = 24
export const PROXY_HISTORY_BUFFER_SIZE = 5

const proxyHistorySortCollator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base',
})

export const defaultProxyHistoryColumns: Column[] = [
  { id: 'id', label: 'ID', visible: true, width: 52, minWidth: 46 },
  { id: 'host', label: 'Host', visible: true, width: 220, minWidth: 120 },
  { id: 'method', label: 'Method', visible: true, width: 78, minWidth: 64 },
  { id: 'url', label: 'URL', visible: true, width: 380, minWidth: 180 },
  { id: 'params', label: 'Params', visible: true, width: 62, minWidth: 56 },
  { id: 'status', label: 'Status', visible: true, width: 78, minWidth: 68 },
  { id: 'length', label: 'Length', visible: true, width: 92, minWidth: 72 },
  { id: 'mime', label: 'MIME Type', visible: true, width: 120, minWidth: 90 },
  { id: 'extension', label: 'Extension', visible: true, width: 92, minWidth: 74 },
  { id: 'title', label: 'Title', visible: true, width: 170, minWidth: 120 },
  { id: 'tls', label: 'TLS', visible: true, width: 56, minWidth: 48 },
  { id: 'ip', label: 'IP', visible: true, width: 132, minWidth: 100 },
  { id: 'time', label: 'Time', visible: true, width: 168, minWidth: 128 },
  { id: 'listener', label: 'Listener', visible: true, width: 100, minWidth: 80 },
  { id: 'responseTimer', label: 'Response Timer', visible: true, width: 108, minWidth: 90 },
]

const DEFAULT_PROXY_HISTORY_SORT_STATE: ProxyHistorySortState = {
  columnId: 'id',
  direction: 'desc',
}

export const isProxyHistoryDefaultSort = (sortState: ProxyHistorySortState) =>
  sortState.columnId === DEFAULT_PROXY_HISTORY_SORT_STATE.columnId
  && sortState.direction === DEFAULT_PROXY_HISTORY_SORT_STATE.direction

export const loadProxyHistoryColumnsFromStorage = () => {
  try {
    const saved = localStorage.getItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY)
    if (saved) {
      const savedColumns = JSON.parse(saved) as Column[]
      return defaultProxyHistoryColumns.map((column) => {
        const savedColumn = savedColumns.find((item) => item.id === column.id)
        return savedColumn ? { ...column, ...savedColumn } : column
      })
    }
  } catch (error) {
    console.error('Failed to load columns from storage:', error)
  }
  return defaultProxyHistoryColumns
}

export const translateProxyHistoryColumns = (
  columns: Column[],
  t: (key: string) => string,
) =>
  columns.map((column) => ({
    ...column,
    label:
      column.id === 'id' ? t('trafficAnalysis.history.table.id')
        : column.id === 'host' ? t('trafficAnalysis.history.table.host')
        : column.id === 'method' ? t('trafficAnalysis.history.table.method')
        : column.id === 'url' ? t('trafficAnalysis.history.table.url')
        : column.id === 'params' ? t('trafficAnalysis.history.table.params')
        : column.id === 'status' ? t('trafficAnalysis.history.table.status')
        : column.id === 'length' ? t('trafficAnalysis.history.table.length')
        : column.id === 'mime' ? t('trafficAnalysis.history.table.mimeType')
        : column.id === 'extension' ? t('trafficAnalysis.history.table.extension')
        : column.id === 'title' ? t('trafficAnalysis.history.table.title')
        : column.id === 'tls' ? t('trafficAnalysis.history.table.tls')
        : column.id === 'ip' ? t('trafficAnalysis.history.table.ip')
        : column.id === 'time' ? t('trafficAnalysis.history.table.time')
        : column.id === 'listener' ? t('trafficAnalysis.history.table.listener')
        : column.id === 'responseTimer' ? t('trafficAnalysis.history.table.responseTimer')
        : t('trafficAnalysis.history.table.actions'),
  }))

export const loadProxyHistorySortFromStorage = (): ProxyHistorySortState => {
  try {
    const saved = localStorage.getItem(PROXY_HISTORY_SORT_STORAGE_KEY)
    if (!saved) return { ...DEFAULT_PROXY_HISTORY_SORT_STATE }
    const parsed = JSON.parse(saved) as Partial<ProxyHistorySortState>
    if (!parsed.columnId || (parsed.direction !== 'asc' && parsed.direction !== 'desc')) {
      return { ...DEFAULT_PROXY_HISTORY_SORT_STATE }
    }
    return {
      columnId: parsed.columnId,
      direction: parsed.direction,
    }
  } catch {
    return { ...DEFAULT_PROXY_HISTORY_SORT_STATE }
  }
}

export const getProxyHistoryDefaultSortDirection = (columnId: string): ProxyHistorySortDirection =>
  ['id', 'time', 'status', 'length', 'responseTimer'].includes(columnId) ? 'desc' : 'asc'

function getProxyHistorySortValue(request: ProxyRequest, columnId: string): number | string {
  const derived = getProxyHistoryDerived(request)

  switch (columnId) {
    case 'id':
      return request.id
    case 'host':
      return request.host || ''
    case 'method':
      return request.method || ''
    case 'url':
      return request.url || ''
    case 'params':
      return derived.hasParams ? 1 : 0
    case 'status':
      return request.status_code
    case 'length':
      return request.response_size
    case 'mime':
      return derived.mimeType
    case 'extension':
      return derived.extension
    case 'title':
      return request.title || ''
    case 'tls':
      return request.protocol === 'https' ? 1 : 0
    case 'ip':
      return request.ip || ''
    case 'time':
      return derived.timestampMs
    case 'listener':
      return derived.listenerValue
    case 'responseTimer':
      return request.response_time
    default:
      return ''
  }
}

export const compareProxyHistoryRequests = (
  left: ProxyRequest,
  right: ProxyRequest,
  sortState: ProxyHistorySortState,
) => {
  const multiplier = sortState.direction === 'asc' ? 1 : -1
  const leftValue = getProxyHistorySortValue(left, sortState.columnId)
  const rightValue = getProxyHistorySortValue(right, sortState.columnId)

  if (typeof leftValue === 'number' && typeof rightValue === 'number') {
    if (leftValue === rightValue) return right.id - left.id
    return (leftValue - rightValue) * multiplier
  }

  const compared = proxyHistorySortCollator.compare(String(leftValue), String(rightValue))
  if (compared === 0) return right.id - left.id
  return compared * multiplier
}

export const sortProxyHistoryRequests = (
  requests: ProxyRequest[],
  sortState: ProxyHistorySortState,
): ProxyRequest[] => {
  if (requests.length < 2) {
    return requests
  }

  if (isProxyHistoryDefaultSort(sortState)) {
    return requests
  }

  return [...requests].sort((left, right) => compareProxyHistoryRequests(left, right, sortState))
}

export const buildProxyHistoryVisibleItems = (
  requests: ProxyRequest[],
  scrollTop: number,
  containerHeight: number,
): VirtualItem[] => {
  const startIndex = Math.max(
    0,
    Math.floor(scrollTop / PROXY_HISTORY_ITEM_HEIGHT) - PROXY_HISTORY_BUFFER_SIZE,
  )
  const visibleCount = Math.ceil(containerHeight / PROXY_HISTORY_ITEM_HEIGHT)
  const neededCount = visibleCount + PROXY_HISTORY_BUFFER_SIZE * 2
  const endIndex = Math.min(requests.length, startIndex + neededCount)
  const items: VirtualItem[] = []

  for (let index = startIndex; index < endIndex; index += 1) {
    items.push({
      data: requests[index],
      offset: index * PROXY_HISTORY_ITEM_HEIGHT,
    })
  }

  return items
}
