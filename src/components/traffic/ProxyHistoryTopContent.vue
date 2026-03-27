<template>
  <div v-if="isLoading || isLoadingWs" class="flex items-center justify-center h-full">
    <i class="fas fa-spinner fa-spin text-2xl"></i>
  </div>
  <div v-else-if="protocolFilter === 'websocket'" class="p-2">
    <div v-if="wsConnections.length === 0" class="flex flex-col items-center justify-center h-32 text-base-content/50">
      <i class="fas fa-plug text-3xl mb-2"></i>
      <span class="text-sm">No WebSocket connections yet</span>
    </div>
    <div v-else class="space-y-2">
      <div v-for="conn in wsConnections" :key="conn.id" class="bg-base-100 border border-base-300 rounded-lg overflow-hidden">
        <div class="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-base-200" @click="toggleWsConnection(conn.id)">
          <i class="fas fa-chevron-right transition-transform duration-200" :class="{ 'rotate-90': expandedWsConnections.has(conn.id) }"></i>
          <span class="badge badge-xs" :class="{ 'badge-success': conn.status === 'open', 'badge-ghost': conn.status === 'closed', 'badge-error': conn.status === 'error' }">{{ conn.status }}</span>
          <span class="text-xs font-mono text-primary truncate flex-1">{{ conn.url }}</span>
          <span class="text-xs text-base-content/50">{{ conn.host }}</span>
          <span class="text-xs text-base-content/40">{{ formatWsTime(conn.opened_at) }}</span>
        </div>
        <div v-if="expandedWsConnections.has(conn.id)" class="border-t border-base-300 bg-base-200/30">
          <div class="tabs tabs-boxed tabs-xs bg-transparent p-2 justify-start rounded-none border-b border-base-300/50">
            <a class="tab tab-xs" :class="{ 'tab-active': getWsActiveTab(conn.id) === 'messages' }" @click.stop="setWsActiveTab(conn.id, 'messages')">Messages ({{ conn.message_ids?.length || 0 }})</a>
            <a class="tab tab-xs" :class="{ 'tab-active': getWsActiveTab(conn.id) === 'handshake' }" @click.stop="setWsActiveTab(conn.id, 'handshake')">Handshake</a>
          </div>
          <div v-if="getWsActiveTab(conn.id) === 'messages'" class="max-h-64 overflow-y-auto">
            <div v-for="msg in getWsMessagesForConnection(conn.id)" :key="msg.id" class="flex items-start gap-2 px-4 py-1.5 text-xs border-b border-base-300/50 last:border-0 hover:bg-base-200/50 transition-colors" :class="{ 'bg-success/5': msg.direction === 'send', 'bg-info/5': msg.direction === 'receive' }">
              <i class="text-base" :class="{ 'fas fa-arrow-up text-success': msg.direction === 'send', 'fas fa-arrow-down text-info': msg.direction === 'receive' }" :title="msg.direction === 'send' ? $t('trafficAnalysis.history.websocket.toServer') : $t('trafficAnalysis.history.websocket.fromServer')"></i>
              <span class="badge badge-xs font-mono" :class="{ 'badge-primary': msg.message_type === 'text', 'badge-secondary': msg.message_type === 'binary', 'badge-ghost': ['ping', 'pong'].includes(msg.message_type), 'badge-warning': msg.message_type === 'close' }">{{ msg.message_type }}</span>
              <div class="flex-1 min-w-0 font-mono break-all select-text">{{ truncateWsContent(msg.content) }}</div>
              <span class="text-base-content/40 whitespace-nowrap">{{ msg.content_length }}B</span>
              <span class="text-base-content/40 whitespace-nowrap">{{ formatWsTime(msg.timestamp) }}</span>
            </div>
            <div v-if="getWsMessagesForConnection(conn.id).length === 0" class="text-center py-4 text-base-content/50 text-xs">No messages recorded yet</div>
          </div>
          <div v-else class="p-4 grid grid-cols-2 gap-4 text-xs font-mono">
            <div class="bg-base-100 p-3 rounded border border-base-300">
              <div class="font-bold mb-2 text-base-content/70">Request Headers</div>
              <pre class="whitespace-pre-wrap break-all select-text overflow-x-auto max-h-48">{{ conn.request_headers || 'No headers captured' }}</pre>
            </div>
            <div class="bg-base-100 p-3 rounded border border-base-300">
              <div class="font-bold mb-2 text-base-content/70">Response Headers</div>
              <pre class="whitespace-pre-wrap break-all select-text overflow-x-auto max-h-48">{{ conn.response_headers || 'No headers captured' }}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
  <div v-else-if="filteredRequests.length > 0" :style="{ height: totalHeight + 'px', position: 'relative' }">
    <div class="sticky top-0 z-10 flex bg-base-200 border-b-2 border-base-300 font-semibold table-row-text" :style="{ height: headerHeight + 'px', minWidth: 'max-content' }">
      <div v-if="isMultiSelectMode" class="flex items-center justify-center px-2 border-r border-base-300" style="width: 40px; min-width: 40px;">
        <input type="checkbox" class="checkbox checkbox-xs checkbox-primary" :checked="selectedRequests.size > 0 && selectedRequests.size === filteredRequests.length" :indeterminate="selectedRequests.size > 0 && selectedRequests.size < filteredRequests.length" @change="selectedRequests.size === filteredRequests.length ? clearSelection() : selectAllVisible()" />
      </div>
      <div v-for="col in visibleColumns" :key="col.id" class="flex items-center px-2 border-r border-base-300 relative" :style="{ width: col.width + 'px', minWidth: col.minWidth + 'px' }">
        <span class="truncate">{{ col.label }}</span>
        <div class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30" @mousedown="startResize(col.id, $event)"></div>
      </div>
    </div>
    <div v-for="item in visibleItems" :key="item.data.id" class="absolute left-0 right-0 flex hover:bg-base-200 cursor-pointer border-b border-base-300 table-row-text" :class="{ 'bg-primary/10': selectedRequest?.id === item.data.id, 'bg-accent/10': isMultiSelectMode && isRequestSelected(item.data), 'bg-error/10 hover:bg-error/20': item.data.status_code === 0 }" :style="{ top: (item.offset + headerHeight) + 'px', height: itemHeight + 'px', minWidth: 'max-content' }" @click="isMultiSelectMode ? toggleSelectRequest(item.data) : (item.data.status_code === 0 ? showCertificateError(item.data) : selectRequest(item.data))" @contextmenu.prevent="showContextMenu($event, item.data)">
      <div v-if="isMultiSelectMode" class="flex items-center justify-center px-2 border-r border-base-300" style="width: 40px; min-width: 40px;" @click.stop="toggleSelectRequest(item.data)">
        <input type="checkbox" class="checkbox checkbox-xs checkbox-accent" :checked="isRequestSelected(item.data)" @click.stop @change="toggleSelectRequest(item.data)" />
      </div>
      <div v-for="col in visibleColumns" :key="col.id" class="flex items-center px-2 border-r border-base-300 overflow-hidden" :style="{ width: col.width + 'px', minWidth: col.minWidth + 'px' }">
        <template v-if="col.id === 'method'">
          <span :class="['badge badge-xs', getMethodClass(item.data.method)]">{{ item.data.method }}</span>
        </template>
        <template v-else-if="col.id === 'status'">
          <div class="flex items-center gap-1">
            <span :class="['badge badge-xs', getStatusClass(item.data.status_code)]" :title="getStatusTitle(item.data.status_code)">{{ getStatusText(item.data.status_code) }}</span>
            <i v-if="item.data.status_code === 0" class="fas fa-exclamation-circle text-error text-xs cursor-help" :title="$t('trafficAnalysis.history.certificateError.title')" @click.stop="showCertificateError(item.data)"></i>
          </div>
        </template>
        <template v-else-if="col.id === 'params'">
          <span v-if="hasParams(item.data.url)" class="text-success">✓</span>
        </template>
        <template v-else-if="col.id === 'tls'">
          <span v-if="item.data.protocol === 'https'" class="text-success">✓</span>
        </template>
        <template v-else>
          <span class="truncate" :title="getColumnValue(item.data, col.id)">{{ getColumnValue(item.data, col.id) }}</span>
        </template>
      </div>
    </div>
  </div>
  <div v-else class="flex items-center justify-center h-full text-base-content/50">
    <div class="text-center">
      <i class="fas fa-inbox text-4xl mb-2"></i>
      <p>{{ $t('trafficAnalysis.history.emptyState.noRequests') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Column, ProxyHistoryProtocolFilter, ProxyHistoryWsTab, ProxyRequest, VirtualItem, WebSocketConnection, WebSocketMessage } from './proxyHistoryTypes'
defineProps<{
  isLoading: boolean
  isLoadingWs: boolean
  protocolFilter: ProxyHistoryProtocolFilter
  wsConnections: WebSocketConnection[]
  expandedWsConnections: Set<string>
  toggleWsConnection: (connectionId: string) => void
  formatWsTime: (timestamp: string) => string
  getWsActiveTab: (connectionId: string) => ProxyHistoryWsTab
  setWsActiveTab: (connectionId: string, tab: ProxyHistoryWsTab) => void
  getWsMessagesForConnection: (connectionId: string) => WebSocketMessage[]
  truncateWsContent: (content?: string, maxLength?: number) => string
  filteredRequests: ProxyRequest[]
  totalHeight: number
  headerHeight: number
  itemHeight: number
  isMultiSelectMode: boolean
  selectedRequests: Set<number>
  clearSelection: () => void
  selectAllVisible: () => void
  visibleColumns: Column[]
  startResize: (columnId: string, event: MouseEvent) => void
  visibleItems: VirtualItem[]
  selectedRequest: ProxyRequest | null
  isRequestSelected: (request: ProxyRequest) => boolean
  toggleSelectRequest: (request: ProxyRequest) => void
  showCertificateError: (request: ProxyRequest) => void
  selectRequest: (request: ProxyRequest) => void
  showContextMenu: (event: MouseEvent, request: ProxyRequest) => void
  getMethodClass: (method: string) => string
  getStatusClass: (statusCode: number) => string
  getStatusTitle: (statusCode: number) => string
  getStatusText: (statusCode: number) => string
  hasParams: (url: string) => boolean
  getColumnValue: (request: ProxyRequest, columnId: string) => string
}>()
</script>
