<template>
  <div v-if="selectedRequest" class="flex-1 flex min-h-0 overflow-hidden relative">
    <div v-if="isLoadingSelectedRequest" class="absolute inset-0 z-10 flex items-center justify-center bg-base-100/40 backdrop-blur-[1px] pointer-events-none">
      <span class="loading loading-spinner loading-sm text-primary"></span>
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
        </div>
        <div class="btn-group btn-group-xs">
          <button :class="['btn btn-xs', requestTab === 'pretty' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'pretty')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}</button>
          <button :class="['btn btn-xs', requestTab === 'raw' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'raw')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}</button>
          <button :class="['btn btn-xs', requestTab === 'hex' ? 'btn-active' : '']" @click="$emit('update:requestTab', 'hex')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}</button>
        </div>
      </div>
      <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent="showDetailContextMenu($event)">
        <HttpMessageSurface
          v-if="requestTab !== 'hex'"
          :model-value="requestContent"
          readonly
          message-type="request"
          :display-mode="requestTab"
          :state-key="selectedRequest ? `history:request:${selectedRequest.id}:${requestTab}:${requestViewMode}` : ''"
        />
        <div v-else class="h-full overflow-auto p-2 font-mono text-xs bg-base-100"><pre>{{ stringToHex(formatRequestRaw(selectedRequest, requestViewMode)) }}</pre></div>
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
        </div>
        <div class="btn-group btn-group-xs">
          <button :class="['btn btn-xs', responseTab === 'pretty' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'pretty')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}</button>
          <button :class="['btn btn-xs', responseTab === 'raw' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'raw')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}</button>
          <button :class="['btn btn-xs', responseTab === 'hex' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'hex')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}</button>
          <button :class="['btn btn-xs', responseTab === 'render' ? 'btn-active' : '']" @click="$emit('update:responseTab', 'render')">{{ $t('trafficAnalysis.history.detailsPanel.tabs.render') }}</button>
        </div>
      </div>
      <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent>
        <iframe v-if="responseTab === 'render'" :srcdoc="getResponseBody(selectedRequest, responseViewMode)" class="w-full h-full border-0 bg-white" sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"></iframe>
        <div v-else-if="responseTab === 'hex'" class="h-full overflow-auto p-2 font-mono text-xs bg-base-100"><pre>{{ stringToHex(formatResponseRaw(selectedRequest, responseViewMode)) }}</pre></div>
        <HttpMessageSurface
          v-else
          :model-value="responseContent"
          readonly
          message-type="response"
          :display-mode="responseTab === 'pretty' ? 'pretty' : 'raw'"
          :state-key="selectedRequest ? `history:response:${selectedRequest.id}:${responseTab}:${responseViewMode}` : ''"
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
import { computed } from 'vue'
import { formatRequest, formatRequestRaw, formatResponse, formatResponseRaw, getResponseBody, hasEditedResponse, isResponseCompressed, stringToHex } from './proxyHistoryFormattingSupport'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import type { ProxyHistoryRequestTab, ProxyHistoryResponseTab, ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'
const props = defineProps<{ selectedRequest: ProxyRequest | null; isLoadingSelectedRequest: boolean; leftPanelWidth: number; requestTab: ProxyHistoryRequestTab; responseTab: ProxyHistoryResponseTab; requestViewMode: ProxyHistoryViewMode; responseViewMode: ProxyHistoryViewMode; showDetailContextMenu: (event: MouseEvent) => void; startVerticalResize: (event: MouseEvent) => void }>()
defineEmits<{ 'update:requestTab': [value: ProxyHistoryRequestTab]; 'update:responseTab': [value: ProxyHistoryResponseTab]; 'update:requestViewMode': [value: ProxyHistoryViewMode]; 'update:responseViewMode': [value: ProxyHistoryViewMode] }>()

const requestContent = computed(() =>
  props.selectedRequest ? formatRequest(props.selectedRequest, props.requestTab, props.requestViewMode) : '',
)

const responseContent = computed(() =>
  props.selectedRequest ? formatResponse(props.selectedRequest, props.responseTab, props.responseViewMode) : '',
)
</script>
