import type { Column, ProxyRequest, VirtualItem } from './proxyHistoryTypes'

export const PROXY_HISTORY_COLUMNS_STORAGE_KEY = 'proxyHistory.columns'
export const PROXY_HISTORY_ITEM_HEIGHT = 32
export const PROXY_HISTORY_HEADER_HEIGHT = 34
export const PROXY_HISTORY_BUFFER_SIZE = 5

export const defaultProxyHistoryColumns: Column[] = [
  { id: 'id', label: 'ID', visible: true, width: 60, minWidth: 50 },
  { id: 'host', label: 'Host', visible: true, width: 180, minWidth: 100 },
  { id: 'method', label: 'Method', visible: true, width: 80, minWidth: 60 },
  { id: 'url', label: 'URL', visible: true, width: 300, minWidth: 150 },
  { id: 'params', label: 'Params', visible: true, width: 70, minWidth: 60 },
  { id: 'status', label: 'Status', visible: true, width: 90, minWidth: 80 },
  { id: 'length', label: 'Length', visible: true, width: 80, minWidth: 60 },
  { id: 'mime', label: 'MIME Type', visible: true, width: 100, minWidth: 80 },
  { id: 'extension', label: 'Extension', visible: true, width: 90, minWidth: 70 },
  { id: 'title', label: 'Title', visible: true, width: 150, minWidth: 100 },
  { id: 'tls', label: 'TLS', visible: true, width: 60, minWidth: 50 },
  { id: 'ip', label: 'IP', visible: true, width: 120, minWidth: 100 },
  { id: 'time', label: 'Time', visible: true, width: 160, minWidth: 120 },
  { id: 'listener', label: 'Listener', visible: true, width: 100, minWidth: 80 },
  { id: 'responseTimer', label: 'Response Timer', visible: true, width: 120, minWidth: 100 },
]

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
        : column.id === 'status' ? t('trafficAnalysis.history.table.status')
        : column.id === 'length' ? t('trafficAnalysis.history.table.length')
        : column.id === 'mime' ? t('trafficAnalysis.history.table.mimeType')
        : column.id === 'time' ? t('trafficAnalysis.history.table.time')
        : t('trafficAnalysis.history.table.actions'),
  }))

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
