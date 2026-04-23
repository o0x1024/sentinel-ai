<template>
  <div v-if="selectedRequest" class="flex-1 flex min-h-0 overflow-hidden relative">
    <div
      v-if="isLoadingSelectedRequest"
      class="absolute right-3 top-3 z-10 flex items-center gap-2 rounded-md border border-base-300 bg-base-100/95 px-2 py-1 text-xs text-base-content/70 shadow-sm pointer-events-none"
    >
      <span class="loading loading-spinner loading-xs text-primary"></span>
      <span>{{ $t('trafficAnalysis.history.detailsPanel.loading') }}</span>
    </div>
    <div ref="requestPanelRef" class="flex flex-col overflow-hidden border-r border-base-300" :style="{ width: leftPanelWidth + 'px' }">
      <div
        class="border-b border-base-300 flex flex-wrap items-center gap-2 flex-shrink-0"
        :class="immersiveDrillModeEnabled ? IMMERSIVE_TRAFFIC_PANE_HEADER_CLASS : 'bg-base-200 px-4 py-2'"
      >
        <div class="flex min-w-0 items-center gap-2">
          <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.request') }}</h4>
          <div v-if="selectedRequest.was_edited" class="dropdown dropdown-bottom">
            <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
              <span :class="requestViewMode === 'edited' ? 'text-warning' : ''">{{ requestViewModeLabel }}</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
              <li><a :class="{ active: requestViewMode === 'original' }" @click="$emit('update:requestViewMode', 'original')">{{ $t('trafficAnalysis.history.detailsPanel.originalRequest') }}</a></li>
              <li><a :class="{ active: requestViewMode === 'edited' }" @click="$emit('update:requestViewMode', 'edited')"><span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedRequest') }}</span></a></li>
            </ul>
          </div>
          <span v-if="!immersiveDrillModeEnabled && !isRequestPaneCompact" class="badge badge-xs badge-ghost" :title="$t('trafficAnalysis.history.detailsPanel.scheme')">{{ requestSchemeLabel }}</span>
          <span v-if="!immersiveDrillModeEnabled && !isRequestPaneCompact" class="badge badge-xs badge-outline" :title="$t('trafficAnalysis.history.detailsPanel.httpVersion')">{{ requestHttpVersion }}</span>
        </div>
        <div class="ml-auto flex min-w-0 items-center gap-2 overflow-x-auto">
          <TrafficMessageViewTabs
            :model-value="requestTab"
            :tabs="requestViewTabs"
            :compact="isRequestPaneCompact"
            @update:model-value="$emit('update:requestTab', $event as ProxyHistoryRequestTab)"
          />
          <TrafficMessageDisplayControls
            :mode-label="''"
            :compact="isRequestPaneCompact"
            :show-line-endings="false"
          />
        </div>
      </div>
      <div
        v-if="contextEvidenceHighlights.length || contextEvidenceSearchTerms.length"
        class="border-b border-warning/30 text-xs text-base-content"
        :class="immersiveDrillModeEnabled ? 'bg-warning/8 px-2.5 py-1.5' : 'bg-warning/10 px-4 py-2'"
      >
        <div class="mb-2 flex items-center gap-2">
          <span class="badge badge-warning badge-xs">证据命中</span>
          <span v-if="!immersiveDrillModeEnabled" class="text-base-content/70">
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
          message-type="request"
          custom-context-menu
          show-search-bar
          :display-mode="resolveTrafficTextDisplayMode(requestTab)"
          :state-key="buildHistoryRequestStateKey(selectedRequest?.id, requestTab, requestViewMode)"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          :show-display-toolbar="false"
          @contextmenu="showDetailContextMenu($event, 'request')"
        />
        <TrafficMessageReader
          v-else
          ref="requestSurface"
          :model-value="stringToHex(requestRawContent)"
          custom-context-menu
          show-search-bar
          :state-key="buildHistoryRequestStateKey(selectedRequest?.id, 'hex', requestViewMode)"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          :show-display-toolbar="false"
          @contextmenu="showDetailContextMenu($event, 'request')"
        />
      </div>
    </div>
    <div class="w-1 bg-base-300 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0" @mousedown="startVerticalResize"></div>
    <div ref="responsePanelRef" class="flex-1 flex flex-col overflow-hidden min-w-0">
      <div
        class="border-b border-base-300 flex flex-wrap items-center gap-2 flex-shrink-0"
        :class="immersiveDrillModeEnabled ? IMMERSIVE_TRAFFIC_PANE_HEADER_CLASS : 'bg-base-200 px-4 py-2'"
      >
        <div class="flex min-w-0 items-center gap-2">
          <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.response') }}</h4>
          <span v-if="isResponseCompressed(selectedRequest)" class="badge badge-xs badge-info" title="响应已自动解压"><i class="fas fa-file-archive mr-1"></i>{{ $t('trafficAnalysis.history.detailsPanel.decompressed') }}</span>
          <div v-if="selectedRequest.was_edited && hasEditedResponse(selectedRequest)" class="dropdown dropdown-bottom">
            <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
              <span :class="responseViewMode === 'edited' ? 'text-warning' : ''">{{ responseViewModeLabel }}</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
              <li><a :class="{ active: responseViewMode === 'original' }" @click="$emit('update:responseViewMode', 'original')">{{ $t('trafficAnalysis.history.detailsPanel.originalResponse') }}</a></li>
              <li><a :class="{ active: responseViewMode === 'edited' }" @click="$emit('update:responseViewMode', 'edited')"><span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedResponse') }}</span></a></li>
            </ul>
          </div>
          <span v-if="!immersiveDrillModeEnabled && !isResponsePaneCompact" class="badge badge-xs badge-outline" :title="$t('trafficAnalysis.history.detailsPanel.httpVersion')">{{ responseHttpVersion }}</span>
        </div>
        <div class="ml-auto flex min-w-0 items-center gap-2 overflow-x-auto">
          <TrafficMessageViewTabs
            :model-value="responseTab"
            :tabs="responseViewTabs"
            :compact="isResponsePaneCompact"
            @update:model-value="$emit('update:responseTab', $event as ProxyHistoryResponseTab)"
          />
          <TrafficMessageDisplayControls
            v-if="responseTab !== 'render'"
            :mode-label="''"
            :compact="isResponsePaneCompact"
            :show-line-endings="false"
          />
        </div>
      </div>
      <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent="showDetailContextMenu($event, 'response')">
        <TrafficResponseRenderPane
          v-if="responseTab === 'render'"
          :body="responseBodyText"
          :content-type="responseContentType"
        />
        <TrafficMessageReader
          v-else-if="responseTab === 'hex'"
          ref="responseSurface"
          :model-value="stringToHex(responseRawContent)"
          custom-context-menu
          show-search-bar
          :state-key="buildHistoryResponseStateKey(selectedRequest?.id, 'hex', responseViewMode)"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          :show-display-toolbar="false"
          @contextmenu="showDetailContextMenu($event, 'response')"
        />
        <HttpMessageSurface
          v-else
          ref="responseSurface"
          :model-value="responseContent"
          readonly
          message-type="response"
          custom-context-menu
          show-search-bar
          :display-mode="resolveTrafficTextDisplayMode(responseTab)"
          :state-key="buildHistoryResponseStateKey(selectedRequest?.id, responseTab, responseViewMode)"
          :search-placeholder="$t('trafficAnalysis.history.detailsPanel.search.placeholder')"
          :search-next-title="$t('trafficAnalysis.history.detailsPanel.search.next')"
          :search-previous-title="$t('trafficAnalysis.history.detailsPanel.search.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.history.detailsPanel.search.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.history.detailsPanel.search.regexp')"
          :search-clear-title="$t('trafficAnalysis.history.detailsPanel.search.clear')"
          :search-no-matches-text="$t('trafficAnalysis.history.detailsPanel.search.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.history.detailsPanel.search.invalidRegexp')"
          :show-display-toolbar="false"
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
import { useI18n } from 'vue-i18n'
import {
  formatProxyHistorySchemeLabel,
  normalizeProxyHistoryHttpVersion,
} from './proxyHistoryHttpSupport'
import {
  formatRequest,
  formatRequestRaw,
  formatResponse,
  formatResponseRaw,
  getResponseContentType,
  hasEditedResponse,
  isResponseCompressed,
  stringToHex,
} from './proxyHistoryFormattingSupport'
import {
  findTrafficContextEvidenceSelectionRange,
  findTrafficContextEvidenceSelectionRangeBySearchTerms,
  resolveTrafficContextEvidenceHighlights,
  type TrafficContextEvidenceHighlight,
  type TrafficContextEvidenceSource,
} from './trafficContextEvidenceHighlightSupport'
import TrafficMessageReader from '@/components/traffic/TrafficMessageReader.vue'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import TrafficMessageDisplayControls from '@/components/traffic/TrafficMessageDisplayControls.vue'
import TrafficMessageViewTabs from '@/components/traffic/TrafficMessageViewTabs.vue'
import TrafficResponseRenderPane from './TrafficResponseRenderPane.vue'
import { useTrafficDisplaySettings } from './trafficDisplaySettings'
import { resolveStoredTrafficResponseBodyText } from './trafficResponseDecodingSupport'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { IMMERSIVE_TRAFFIC_PANE_HEADER_CLASS } from './immersiveTrafficUi'
import {
  buildHistoryRequestStateKey,
  buildHistoryResponseStateKey,
  resolveTrafficTextDisplayMode,
} from './trafficMessagePresentationSupport'
import type { ProxyHistoryRequestTab, ProxyHistoryResponseTab, ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'
import { useTrafficPaneCompactMode } from './useTrafficPaneCompactMode'
const { t, locale } = useI18n()
const { settings } = useTrafficDisplaySettings()
const {
  panelRef: requestPanelRef,
  isCompact: isRequestPaneCompact,
} = useTrafficPaneCompactMode(640)
const {
  panelRef: responsePanelRef,
  isCompact: isResponsePaneCompact,
} = useTrafficPaneCompactMode(760)
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
const requestViewTabs = computed(() => [
  { value: 'pretty', label: t('trafficAnalysis.history.detailsPanel.tabs.pretty'), shortLabel: locale.value.startsWith('zh') ? '格式' : 'Fmt' },
  { value: 'raw', label: t('trafficAnalysis.history.detailsPanel.tabs.raw'), shortLabel: locale.value.startsWith('zh') ? '原始' : 'Raw' },
  { value: 'hex', label: t('trafficAnalysis.history.detailsPanel.tabs.hex'), shortLabel: 'Hex' },
])
const responseViewTabs = computed(() => [
  { value: 'pretty', label: t('trafficAnalysis.history.detailsPanel.tabs.pretty'), shortLabel: locale.value.startsWith('zh') ? '格式' : 'Fmt' },
  { value: 'raw', label: t('trafficAnalysis.history.detailsPanel.tabs.raw'), shortLabel: locale.value.startsWith('zh') ? '原始' : 'Raw' },
  { value: 'hex', label: t('trafficAnalysis.history.detailsPanel.tabs.hex'), shortLabel: 'Hex' },
  { value: 'render', label: t('trafficAnalysis.history.detailsPanel.tabs.render'), shortLabel: locale.value.startsWith('zh') ? '渲染' : 'View' },
])
const requestViewModeLabel = computed(() => {
  if (isRequestPaneCompact.value) {
    if (props.requestViewMode === 'original') return locale.value.startsWith('zh') ? '原始' : 'Orig'
    return locale.value.startsWith('zh') ? '编辑' : 'Edit'
  }
  return props.requestViewMode === 'original'
    ? t('trafficAnalysis.history.detailsPanel.originalRequest')
    : t('trafficAnalysis.history.detailsPanel.editedRequest')
})
const responseViewModeLabel = computed(() => {
  if (isResponsePaneCompact.value) {
    if (props.responseViewMode === 'original') return locale.value.startsWith('zh') ? '原始' : 'Orig'
    return locale.value.startsWith('zh') ? '编辑' : 'Edit'
  }
  return props.responseViewMode === 'original'
    ? t('trafficAnalysis.history.detailsPanel.originalResponse')
    : t('trafficAnalysis.history.detailsPanel.editedResponse')
})

const requestRawContent = computed(() =>
  props.selectedRequest ? formatRequestRaw(props.selectedRequest, props.requestViewMode) : '',
)
const requestContent = computed(() =>
  props.selectedRequest ? formatRequest(props.selectedRequest, props.requestTab, props.requestViewMode) : '',
)
const responseBodyText = computed(() => {
  if (!props.selectedRequest) return ''

  const useEdited = props.responseViewMode === 'edited' && props.selectedRequest.was_edited
  const headers = useEdited && props.selectedRequest.edited_response_headers
    ? props.selectedRequest.edited_response_headers
    : props.selectedRequest.response_headers
  const body = useEdited && props.selectedRequest.edited_response_body
    ? props.selectedRequest.edited_response_body
    : props.selectedRequest.response_body

  return resolveStoredTrafficResponseBodyText(body || '', headers, settings.value)
})
const responseRawContent = computed(() =>
  props.selectedRequest
    ? formatResponseRaw(props.selectedRequest, props.responseViewMode, { bodyText: responseBodyText.value })
    : '',
)
const responseContent = computed(() =>
  props.selectedRequest
    ? formatResponse(props.selectedRequest, props.responseTab, props.responseViewMode, { bodyText: responseBodyText.value })
    : '',
)
const responseContentType = computed(() =>
  props.selectedRequest ? getResponseContentType(props.selectedRequest, props.responseViewMode) : '',
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
