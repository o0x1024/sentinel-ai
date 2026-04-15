<template>
  <div v-if="selectedRequest" class="flex-1 flex min-h-0 overflow-hidden relative">
    <div
      v-if="isLoadingSelectedRequest"
      class="absolute right-3 top-3 z-10 flex items-center gap-2 rounded-md border border-base-300 bg-base-100/95 px-2 py-1 text-xs text-base-content/70 shadow-sm pointer-events-none"
    >
      <span class="loading loading-spinner loading-xs text-primary"></span>
      <span>{{ $t('trafficAnalysis.history.detailsPanel.loading') }}</span>
    </div>
    <div class="flex flex-col overflow-hidden border-r border-base-300" :style="{ width: leftPanelWidth + 'px' }">
      <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
        <div class="flex items-center gap-2">
          <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.request') }}</h4>
          <div v-if="selectedRequest.was_edited" class="dropdown dropdown-bottom">
            <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
              <span :class="requestViewMode === 'edited' ? 'text-warning' : ''">{{ requestViewMode === 'original' ? $t('trafficAnalysis.history.detailsPanel.originalRequest') : $t('trafficAnalysis.history.detailsPanel.editedRequest') }}</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
              <li><a :class="{ active: requestViewMode === 'original' }" @click="$emit('update:requestViewMode', 'original')">{{ $t('trafficAnalysis.history.detailsPanel.originalRequest') }}</a></li>
              <li><a :class="{ active: requestViewMode === 'edited' }" @click="$emit('update:requestViewMode', 'edited')"><span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedRequest') }}</span></a></li>
            </ul>
          </div>
          <span class="badge badge-xs badge-ghost" :title="$t('trafficAnalysis.history.detailsPanel.scheme')">{{ requestSchemeLabel }}</span>
          <span class="badge badge-xs badge-outline" :title="$t('trafficAnalysis.history.detailsPanel.httpVersion')">{{ requestHttpVersion }}</span>
        </div>
        <div class="btn-group btn-group-xs">
          <button :class="['btn btn-xs', requestTab === 'pretty' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'pretty')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}</button>
          <button :class="['btn btn-xs', requestTab === 'raw' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'raw')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}</button>
          <button :class="['btn btn-xs', requestTab === 'hex' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'hex')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}</button>
        </div>
      </div>
      <div
        v-if="contextEvidenceHighlights.length || contextEvidenceSearchTerms.length"
        class="border-b border-warning/30 bg-warning/10 px-4 py-2 text-xs text-base-content"
      >
        <div class="mb-2 flex items-center gap-2">
          <span class="badge badge-warning badge-xs">证据命中</span>
          <span class="text-base-content/70">
            {{ contextEvidencePane === 'response' ? '已定位到响应证据' : '已根据候选证据高亮当前请求中的匹配位置' }}
          </span>
        </div>
        <div class="flex flex-wrap gap-2">
          <div
            v-for="item in contextEvidenceHighlights"
            :key="item.location"
            class="rounded border border-warning/30 bg-base-100/80 px-2 py-1 cursor-pointer transition-colors hover:border-warning hover:bg-base-100"
            role="button"
            tabindex="0"
            @click="focusEvidenceHighlight(item)"
            @keydown.enter.prevent="focusEvidenceHighlight(item)"
            @keydown.space.prevent="focusEvidenceHighlight(item)"
          >
            <div class="flex items-center gap-1 font-mono text-[11px]">
              <span class="badge badge-ghost badge-xs">{{ getEvidenceSourceLabel(item.source) }}</span>
              <span>{{ item.location }}</span>
            </div>
            <div v-if="item.values.length" class="mt-1 break-all text-[11px] text-warning-content/80">
              {{ item.values.join('，') }}
            </div>
            <div v-else class="mt-1 text-[11px] text-base-content/50">
              当前请求中未解析出具体值
            </div>
          </div>
          <div
            v-for="term in contextEvidenceSearchTerms"
            :key="`term:${term}`"
            class="rounded border border-info/30 bg-base-100/80 px-2 py-1 cursor-pointer transition-colors hover:border-info hover:bg-base-100"
            role="button"
            tabindex="0"
            @click="focusEvidenceSearchTerm(term)"
            @keydown.enter.prevent="focusEvidenceSearchTerm(term)"
            @keydown.space.prevent="focusEvidenceSearchTerm(term)"
          >
            <div class="flex items-center gap-1 font-mono text-[11px]">
              <span class="badge badge-info badge-xs">Search</span>
              <span>{{ term }}</span>
            </div>
          </div>
        </div>
      </div>
      <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent="showDetailContextMenu($event, 'request')">
        <HttpMessageSurface
          v-if="requestTab !== 'hex'"
          ref="requestSurface"
          :model-value="requestContent"
          readonly
          custom-context-menu
          show-search-bar
          message-type="request"
          :display-mode="requestTab"
          :state-key="selectedRequest ? `history:request:${selectedRequest.id}:${requestTab}:${requestViewMode}` : ''"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          @contextmenu="showDetailContextMenu($event, 'request')"
        />
        <HttpMessageSurface
          v-else
          ref="requestSurface"
          :model-value="stringToHex(formatRequestRaw(selectedRequest, requestViewMode))"
          readonly
          custom-context-menu
          show-search-bar
          message-type="generic"
          display-mode="raw"
          :state-key="selectedRequest ? `history:request:${selectedRequest.id}:hex:${requestViewMode}` : ''"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          @contextmenu="showDetailContextMenu($event, 'request')"
        />
      </div>
    </div>
    <div class="w-1 bg-base-300 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0" @mousedown="startVerticalResize"></div>
    <div class="flex-1 flex flex-col overflow-hidden min-w-0">
      <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
        <div class="flex items-center gap-2">
          <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.response') }}</h4>
          <span v-if="isResponseCompressed(selectedRequest)" class="badge badge-xs badge-info" title="响应已自动解压"><i class="fas fa-file-archive mr-1"></i>{{ $t('trafficAnalysis.history.detailsPanel.decompressed') }}</span>
          <div v-if="selectedRequest.was_edited && hasEditedResponse(selectedRequest)" class="dropdown dropdown-bottom">
            <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
              <span :class="responseViewMode === 'edited' ? 'text-warning' : ''">{{ responseViewMode === 'original' ? $t('trafficAnalysis.history.detailsPanel.originalResponse') : $t('trafficAnalysis.history.detailsPanel.editedResponse') }}</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
              <li><a :class="{ active: responseViewMode === 'original' }" @click="$emit('update:responseViewMode', 'original')">{{ $t('trafficAnalysis.history.detailsPanel.originalResponse') }}</a></li>
              <li><a :class="{ active: responseViewMode === 'edited' }" @click="$emit('update:responseViewMode', 'edited')"><span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedResponse') }}</span></a></li>
            </ul>
          </div>
          <span class="badge badge-xs badge-outline" :title="$t('trafficAnalysis.history.detailsPanel.httpVersion')">{{ responseHttpVersion }}</span>
        </div>
        <div class="btn-group btn-group-xs">
          <button :class="['btn btn-xs', responseTab === 'pretty' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'pretty')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}</button>
          <button :class="['btn btn-xs', responseTab === 'raw' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'raw')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}</button>
          <button :class="['btn btn-xs', responseTab === 'hex' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'hex')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}</button>
          <button :class="['btn btn-xs', responseTab === 'render' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'render')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.render') }}</button>
        </div>
      </div>
      <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent="showDetailContextMenu($event, 'response')">
        <iframe v-if="responseTab === 'render'" :srcdoc="getResponseBody(selectedRequest, responseViewMode)" class="w-full h-full border-0 bg-white" sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"></iframe>
        <HttpMessageSurface
          v-else-if="responseTab === 'hex'"
          ref="responseSurface"
          :model-value="stringToHex(formatResponseRaw(selectedRequest, responseViewMode))"
          readonly
          custom-context-menu
          show-search-bar
          message-type="generic"
          display-mode="raw"
          :state-key="selectedRequest ? `history:response:${selectedRequest.id}:hex:${responseViewMode}` : ''"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          @contextmenu="showDetailContextMenu($event, 'response')"
        />
        <HttpMessageSurface
          v-else
          ref="responseSurface"
          :model-value="responseContent"
          readonly
          custom-context-menu
          show-search-bar
          message-type="response"
          :display-mode="responseTab === 'pretty' ? 'pretty' : 'raw'"
          :state-key="selectedRequest ? `history:response:${selectedRequest.id}:${responseTab}:${responseViewMode}` : ''"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          @contextmenu="showDetailContextMenu($event, 'response')"
        />
      </div>
    </div>
  </div>
  <div v-else class="flex h-full items-center justify-center bg-base-100 text-base-content/50">
    <div class="text-center">
      <i class="fas fa-file-alt mb-2 text-3xl"></i>
      <p class="text-sm">{{ $t('trafficAnalysis.history.detailsPanel.empty') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import {
  formatProxyHistorySchemeLabel,
  normalizeProxyHistoryHttpVersion,
} from './proxyHistoryHttpSupport'
import { formatRequest, formatRequestRaw, formatResponse, formatResponseRaw, getResponseBody, hasEditedResponse, isResponseCompressed, stringToHex } from './proxyHistoryFormattingSupport'
import {
  findTrafficContextEvidenceSelectionRange,
  findTrafficContextEvidenceSelectionRangeBySearchTerms,
  resolveTrafficContextEvidenceHighlights,
  type TrafficContextEvidenceHighlight,
  type TrafficContextEvidenceSource,
} from './trafficContextEvidenceHighlightSupport'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import type { ProxyHistoryRequestTab, ProxyHistoryResponseTab, ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'
const props = defineProps<{ selectedRequest: ProxyRequest | null; isLoadingSelectedRequest: boolean; leftPanelWidth: number; requestTab: ProxyHistoryRequestTab; responseTab: ProxyHistoryResponseTab; requestViewMode: ProxyHistoryViewMode; responseViewMode: ProxyHistoryViewMode; contextEvidencePane?: 'request' | 'response'; contextEvidenceMatchedLocations?: string[]; contextEvidenceSearchTerms?: string[]; showDetailContextMenu: (event: MouseEvent, pane: 'request' | 'response') => void; startVerticalResize: (event: MouseEvent) => void }>()
const emit = defineEmits<{ 'update:requestTab': [value: ProxyHistoryRequestTab]; 'update:responseTab': [value: ProxyHistoryResponseTab]; 'update:requestViewMode': [value: ProxyHistoryViewMode]; 'update:responseViewMode': [value: ProxyHistoryViewMode] }>()
const requestSurface = ref<{
  focus?: () => void
  setSelection?: (from: number, to: number) => void
} | null>(null)
const responseSurface = ref<{
  focus?: () => void
  setSelection?: (from: number, to: number) => void
} | null>(null)
const pendingEvidenceLocation = ref<string | null>(null)
const pendingEvidenceSearchTerm = ref<string | null>(null)
const lastAutoFocusedEvidenceKey = ref('')

const requestContent = computed(() =>
  props.selectedRequest ? formatRequest(props.selectedRequest, props.requestTab, props.requestViewMode) : '',
)

const responseContent = computed(() =>
  props.selectedRequest ? formatResponse(props.selectedRequest, props.responseTab, props.responseViewMode) : '',
)

const requestSchemeLabel = computed(() =>
  formatProxyHistorySchemeLabel(props.selectedRequest?.scheme),
)

const requestHttpVersion = computed(() =>
  normalizeProxyHistoryHttpVersion(props.selectedRequest?.http_version_observed),
)

const responseHttpVersion = computed(() =>
  normalizeProxyHistoryHttpVersion(props.selectedRequest?.http_version_observed),
)

const contextEvidenceHighlights = computed<TrafficContextEvidenceHighlight[]>(() =>
  resolveTrafficContextEvidenceHighlights(
    props.selectedRequest,
    props.contextEvidenceMatchedLocations || [],
    props.contextEvidencePane === 'response' ? props.responseViewMode : props.requestViewMode,
  ),
)

const contextEvidencePane = computed(() => props.contextEvidencePane || 'request')
const contextEvidenceSearchTerms = computed(() => props.contextEvidenceSearchTerms || [])
const activeSurfaceContent = computed(() =>
  contextEvidencePane.value === 'response' ? responseContent.value : requestContent.value,
)

function getEvidenceSourceLabel(source: TrafficContextEvidenceSource) {
  switch (source) {
    case 'query':
      return 'Query'
    case 'body':
      return 'Body'
    case 'header':
      return 'Header'
    case 'cookie':
      return 'Cookie'
    case 'path':
      return 'Path'
    default:
      return 'Other'
  }
}

async function focusEvidenceHighlight(item: TrafficContextEvidenceHighlight) {
  pendingEvidenceLocation.value = item.location
  pendingEvidenceSearchTerm.value = null
  if (contextEvidencePane.value === 'request' && props.requestTab === 'hex') {
    emit('update:requestTab', 'raw')
  } else if (contextEvidencePane.value === 'response' && (props.responseTab === 'hex' || props.responseTab === 'render')) {
    emit('update:responseTab', 'raw')
  }
  await nextTick()
  applyPendingEvidenceSelection()
}

async function focusEvidenceSearchTerm(term: string) {
  pendingEvidenceSearchTerm.value = term
  pendingEvidenceLocation.value = null
  if (contextEvidencePane.value === 'request' && props.requestTab === 'hex') {
    emit('update:requestTab', 'raw')
  } else if (contextEvidencePane.value === 'response' && (props.responseTab === 'hex' || props.responseTab === 'render')) {
    emit('update:responseTab', 'raw')
  }
  await nextTick()
  applyPendingEvidenceSelection()
}

function applyPendingEvidenceSelection() {
  if (
    (contextEvidencePane.value === 'request' && props.requestTab === 'hex')
    || (contextEvidencePane.value === 'response' && (props.responseTab === 'hex' || props.responseTab === 'render'))
  ) {
    return
  }

  const activeSurface = contextEvidencePane.value === 'response' ? responseSurface.value : requestSurface.value
  const location = pendingEvidenceLocation.value
  if (location) {
    const target = contextEvidenceHighlights.value.find(item => item.location === location)
    if (!target) {
      pendingEvidenceLocation.value = null
      return
    }

    const range = findTrafficContextEvidenceSelectionRange(activeSurfaceContent.value, target)
    if (range) {
      activeSurface?.setSelection?.(range.from, range.to)
    } else {
      activeSurface?.focus?.()
    }
    pendingEvidenceLocation.value = null
    return
  }

  const searchTerm = pendingEvidenceSearchTerm.value
  if (!searchTerm) {
    return
  }
  const range = findTrafficContextEvidenceSelectionRangeBySearchTerms(activeSurfaceContent.value, [searchTerm])
  if (range) {
    activeSurface?.setSelection?.(range.from, range.to)
  } else {
    activeSurface?.focus?.()
  }
  pendingEvidenceSearchTerm.value = null
}

watch(
  () => [
    props.selectedRequest?.id || 0,
    contextEvidencePane.value,
    (props.contextEvidenceMatchedLocations || []).join('|'),
    (props.contextEvidenceSearchTerms || []).join('|'),
  ] as const,
  ([requestId, pane, joinedLocations, joinedTerms]) => {
    const nextKey = `${requestId}:${pane}:${joinedLocations}:${joinedTerms}`
    if (nextKey === lastAutoFocusedEvidenceKey.value) {
      return
    }
    lastAutoFocusedEvidenceKey.value = nextKey
    pendingEvidenceLocation.value = contextEvidenceHighlights.value[0]?.location || null
    pendingEvidenceSearchTerm.value = pendingEvidenceLocation.value
      ? null
      : contextEvidenceSearchTerms.value[0] || null
  },
  { immediate: true },
)

watch(
  () => [
    props.requestTab,
    props.responseTab,
    activeSurfaceContent.value,
    pendingEvidenceLocation.value,
    pendingEvidenceSearchTerm.value,
    contextEvidencePane.value,
  ] as const,
  async () => {
    if (!pendingEvidenceLocation.value && !pendingEvidenceSearchTerm.value) {
      return
    }
    if (contextEvidencePane.value === 'request' && props.requestTab === 'hex') {
      emit('update:requestTab', 'raw')
      return
    }
    if (contextEvidencePane.value === 'response' && (props.responseTab === 'hex' || props.responseTab === 'render')) {
      emit('update:responseTab', 'raw')
      return
    }
    await nextTick()
    applyPendingEvidenceSelection()
  },
)
</script>
