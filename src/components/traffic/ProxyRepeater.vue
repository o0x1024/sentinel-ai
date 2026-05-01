<template>
  <div ref="repeaterRoot" class="flex flex-col h-full bg-base-100" @contextmenu.prevent>
    <!-- 右键菜单 -->
    <div
      v-if="contextMenu.visible"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="repeaterContextMenuSections"
        label-prefix="trafficAnalysis.repeater.contextMenu"
      />
    </div>
    <div
      v-if="tabContextMenu.visible"
      class="fixed z-50 min-w-40 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${tabContextMenu.x}px`, top: `${tabContextMenu.y}px` }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="repeaterTabContextMenuSections"
        label-prefix="trafficAnalysis.repeater.tabContextMenu"
      />
    </div>
    <!-- Tabs Header -->
    <div
      class="border-b border-base-300 flex items-center gap-1.5"
      :class="immersiveDrillModeEnabled ? IMMERSIVE_TRAFFIC_TOP_BAR_CLASS : 'bg-base-200 px-1.5 py-0.5'"
    >
      <div class="flex items-center gap-1 overflow-x-auto flex-1">
        <div
          v-for="(tab, index) in tabs" 
          :key="tab.id"
          :data-testid="`repeater-tab-${index}`"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded cursor-pointer border border-base-300 text-sm min-w-14"
          :class="activeTabIndex === index ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
          @click="selectTab(index)"
          @contextmenu.prevent.stop="showTabContextMenu($event, index)"
        >
          <span class="truncate" :title="tab.name">{{ index + 1 }}</span>
          <button 
            @click.stop="closeTab(index)"
            class="w-4 h-4 ml-1.5 flex items-center justify-center rounded opacity-40 hover:opacity-100 hover:bg-base-300 transition-opacity"
            :title="$t('trafficAnalysis.repeater.contextMenu.close')"
          >
            <i class="fas fa-times text-[10px]"></i>
          </button>
        </div>
        <button 
          @click="addTab"
          class="btn btn-xs btn-ghost"
          :title="$t('trafficAnalysis.repeater.contextMenu.newTab')"
        >
          <i class="fas fa-plus"></i>
        </button>
      </div>
      <!-- 布局切换按钮 -->
      <div class="btn-group btn-group-xs">
        <button 
          :class="['btn btn-xs', layoutMode === 'horizontal' ? 'btn-primary' : 'btn-ghost']"
          @click="layoutMode = 'horizontal'"
          :title="$t('trafficAnalysis.repeater.contextMenu.horizontalLayout')"
        >
          <i class="fas fa-columns"></i>
        </button>
        <button 
          :class="['btn btn-xs', layoutMode === 'vertical' ? 'btn-primary' : 'btn-ghost']"
          @click="layoutMode = 'vertical'"
          :title="$t('trafficAnalysis.repeater.contextMenu.verticalLayout')"
        >
          <i class="fas fa-bars"></i>
        </button>
      </div>
      <button
        data-testid="repeater-clear-all-tabs"
        class="btn btn-xs btn-ghost text-error"
        type="button"
        :disabled="tabs.length === 0"
        :title="$t('trafficAnalysis.repeater.contextMenu.clearAll')"
        @click="clearAllTabs"
      >
        <i class="fas fa-trash-alt"></i>
        <span>{{ $t('trafficAnalysis.repeater.contextMenu.clearAll') }}</span>
      </button>
    </div>

    <!-- Tab Content -->
    <div v-if="currentTab" class="flex-1 flex flex-col overflow-hidden">
      <!-- Toolbar -->
      <div
        class="border-b border-base-300 flex items-center gap-2"
        :class="immersiveDrillModeEnabled ? 'bg-base-200/75 px-2 py-1 backdrop-blur-sm' : 'bg-base-200 px-2.5 py-1.5'"
      >
        <button
          @click="sendRequest"
          class="btn btn-primary btn-sm min-h-8 px-2.5"
          :disabled="isSending || !currentTab.targetHost"
        >
          <i :class="['fas', isSending ? 'fa-spinner fa-spin' : 'fa-paper-plane']"></i>
          {{ $t('trafficAnalysis.repeater.contextMenu.sendRequest') }}
        </button>
        <button
          @click="cancelRequest"
          class="btn btn-ghost btn-sm min-h-8 px-2.5"
          :disabled="!isSending"
        >
          <i class="fas fa-stop"></i>
          <span v-if="!immersiveDrillModeEnabled">{{ $t('trafficAnalysis.repeater.contextMenu.cancel') }}</span>
        </button>
        <button
          @click="insertOastPayloadIntoCurrentRequest"
          class="btn btn-outline btn-sm min-h-8 px-2.5"
          :disabled="creatingOastPayload"
        >
          <i :class="creatingOastPayload ? 'fas fa-spinner fa-spin' : 'fas fa-satellite-dish'"></i>
          <span>{{ $t('trafficAnalysis.oast.insertPayload') }}</span>
        </button>
        
        <div class="flex-1"></div>
        
        <!-- Target 显示 -->
        <div v-if="!immersiveDrillModeEnabled" class="flex items-center gap-1.5 text-sm">
          <span class="text-base-content/70">{{ $t('trafficAnalysis.repeater.contextMenu.target') }}:</span>
          <span class="font-mono font-semibold">
            {{ currentTab.useTls ? 'https' : 'http' }}://{{ currentTab.targetHost }}{{ showPort ? ':' + currentTab.targetPort : '' }}
          </span>
          <button 
            @click="showTargetDialog = true"
            class="btn btn-ghost btn-xs btn-circle"
            title="Configure target details"
          >
            <i class="fas fa-pencil-alt text-xs"></i>
          </button>
        </div>
        <div v-else class="flex items-center gap-1.5">
          <span :class="[IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS, 'font-mono']">
            {{ currentTab.useTls ? 'https' : 'http' }}://{{ currentTab.targetHost || 'target' }}
          </span>
          <button
            @click="showTargetDialog = true"
            class="btn btn-ghost btn-xs btn-circle"
            :title="$t('trafficAnalysis.repeater.contextMenu.configureTargetDetails')"
          >
            <i class="fas fa-pencil-alt text-xs"></i>
          </button>
        </div>

        <span v-if="!immersiveDrillModeEnabled" class="badge badge-sm badge-outline">{{ currentRequestProtocol }}</span>
      </div>
      
      <!-- Target 配置对话框 -->
      <AppDialog :class="['modal', showTargetDialog ? 'modal-open' : '']">
        <div class="modal-box max-w-sm">
          <h3 class="font-bold text-lg mb-4">{{ $t('trafficAnalysis.repeater.contextMenu.configureTargetDetails') }}</h3>
          <p class="text-sm text-base-content/70 mb-4">
            {{ $t('trafficAnalysis.repeater.contextMenu.configureTargetDetails') }}
          </p>
          
          <div class="space-y-4">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ $t('trafficAnalysis.repeater.contextMenu.host') }}:</span>
              </label>
              <input 
                v-model="currentTab.targetHost"
                type="text" 
                class="input input-bordered w-full"
                placeholder="example.com"
              />
            </div>
            
            <label class="flex items-center gap-2 cursor-pointer">
              <input 
                type="checkbox"
                v-model="currentTab.overrideSni"
                class="checkbox checkbox-sm"
              />
              <span class="label-text">{{ $t('trafficAnalysis.repeater.contextMenu.overrideSni') }}</span>
            </label>
            
            <div v-if="currentTab.overrideSni" class="form-control pl-6">
              <input 
                v-model="currentTab.sniHost"
                type="text" 
                class="input input-bordered input-sm w-full"
                :placeholder="$t('trafficAnalysis.repeater.contextMenu.sniHostname')"
              />
            </div>
            
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ $t('trafficAnalysis.repeater.contextMenu.port') }}:</span>
              </label>
              <input 
                v-model.number="currentTab.targetPort" 
                type="number" 
                class="input input-bordered w-full"
                min="1"
                max="65535"
              />
            </div>
            
            <label class="flex items-center gap-2 cursor-pointer">
              <input 
                type="checkbox" 
                v-model="currentTab.useTls"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text">{{ $t('trafficAnalysis.repeater.contextMenu.useHttps') }}</span>
            </label>
          </div>
          
          <div class="modal-action">
            <button class="btn btn-sm" @click="showTargetDialog = false">{{ $t('trafficAnalysis.repeater.contextMenu.cancel') }}</button>
            <button class="btn btn-primary btn-sm" @click="showTargetDialog = false">{{ $t('trafficAnalysis.repeater.contextMenu.ok') }}</button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="showTargetDialog = false">
          <button>{{ $t('trafficAnalysis.repeater.contextMenu.close') }}</button>
        </form>
      </AppDialog>

      <!-- Request / Response Panels -->
      <div class="flex-1 flex overflow-hidden" :class="layoutMode === 'horizontal' ? 'flex-row' : 'flex-col'">
        <!-- Request Panel -->
        <div
          ref="requestPanelRef"
          class="request-panel flex flex-col overflow-hidden border-base-300"
          :class="layoutMode === 'horizontal' ? 'border-r' : 'border-b'"
          :style="layoutMode === 'horizontal' 
            ? { width: leftPanelWidth + 'px' } 
            : { height: topPanelHeight + 'px' }"
        >
          <!-- Request Header -->
          <div
            class="flex flex-wrap items-center gap-1.5 border-b border-base-300"
            :class="immersiveDrillModeEnabled ? 'border-b border-base-300 bg-base-200/75 px-2 py-1 backdrop-blur-sm' : 'bg-base-200 px-2 py-0.5'"
          >
            <div class="flex min-w-0 items-center gap-1.5">
              <TrafficVariantSwitch
                v-if="showPreviewRequestVariantSwitch"
                :model-value="activePreviewVariant"
                :prefix-label="$t('trafficAnalysis.repeater.contextMenu.request')"
                :active-label="previewRequestVariantLabel"
                :original-label="$t('trafficAnalysis.history.detailsPanel.originalRequest')"
                :edited-label="$t('trafficAnalysis.history.detailsPanel.editedRequest')"
                button-class="px-1.5 font-semibold normal-case"
                :menu-width-px="144"
                test-id="preview-request-variant-switch"
                @update:model-value="switchPreviewVariant"
              />
              <span v-else class="font-semibold text-sm">{{ $t('trafficAnalysis.repeater.contextMenu.request') }}</span>
              <button
                v-if="!immersiveDrillModeEnabled"
                class="btn btn-ghost btn-xs"
                type="button"
                :disabled="!canCompareCurrentRequestVersions"
                @click="compareCurrentRequestVersions"
                :title="$t('trafficAnalysis.repeater.actions.compareRequestVersions')"
              >
                <i class="fas fa-not-equal"></i>
                <span v-if="!isRequestPaneCompact">{{ $t('trafficAnalysis.repeater.actions.compareRequestVersions') }}</span>
              </button>
            </div>
            <div class="repeater-pane-header-controls ml-auto flex min-w-0 items-center gap-1.5">
              <TrafficMessageViewTabs
                :model-value="currentTab.requestTab"
                :tabs="requestViewTabs"
                :compact="isRequestPaneCompact"
                @update:model-value="currentTab.requestTab = $event as RepeaterTab['requestTab']"
              />
              <TrafficMessageDisplayControls :compact="isRequestPaneCompact" />
            </div>
          </div>

          <!-- Request Content -->
          <div class="flex-1 overflow-hidden" @contextmenu.prevent="showContextMenu($event)">
            <template v-if="currentTab.requestTab === 'pretty'">
              <HttpMessageSurface
                ref="requestEditor"
                :modelValue="formatPrettyRequest()"
                @update:modelValue="onPrettyRequestUpdate"
                :readonly="false"
                custom-context-menu
                show-search-bar
                @contextmenu="showContextMenu($event, 'request')"
                message-type="request"
                height="100%"
                :display-mode="resolveTrafficTextDisplayMode(currentTab.requestTab)"
                :state-key="buildRepeaterRequestStateKey(currentTab.id, currentTab.requestTab)"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
              />
            </template>
            <template v-else-if="currentTab.requestTab === 'raw'">
              <HttpMessageSurface
                ref="requestEditor"
                v-model="currentTab.rawRequest"
                :readonly="false"
                custom-context-menu
                show-search-bar
                @contextmenu="showContextMenu($event, 'request')"
                message-type="request"
                height="100%"
                :display-mode="resolveTrafficTextDisplayMode(currentTab.requestTab)"
                :state-key="buildRepeaterRequestStateKey(currentTab.id, currentTab.requestTab)"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
              />
            </template>
            <template v-else>
              <HttpMessageSurface
                ref="requestEditor"
                :model-value="repeaterTextToHex(currentTab.rawRequest)"
                :readonly="true"
                custom-context-menu
                show-search-bar
                @contextmenu="showContextMenu($event, 'request')"
                message-type="generic"
                height="100%"
                display-mode="raw"
                :state-key="buildRepeaterRequestStateKey(currentTab.id, 'hex')"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
              />
            </template>
          </div>
        </div>

        <!-- Resizer -->
        <div 
          :class="layoutMode === 'horizontal' 
            ? 'w-1 bg-base-300 cursor-col-resize hover:bg-primary/50 flex-shrink-0' 
            : 'h-1 bg-base-300 cursor-row-resize hover:bg-primary/50 flex-shrink-0'"
          @mousedown="startResize"
        ></div>

        <!-- Response Panel -->
        <div ref="responsePanelRef" class="response-panel flex-1 flex flex-col overflow-hidden min-h-0 min-w-0">
          <!-- Response Header -->
          <div
            class="flex flex-wrap items-center gap-1.5 border-b border-base-300"
            :class="immersiveDrillModeEnabled ? 'border-b border-base-300 bg-base-200/75 px-2 py-1 backdrop-blur-sm' : 'bg-base-200 px-2.5 py-0.5'"
          >
            <div class="flex min-w-0 items-center gap-1.5">
              <TrafficVariantSwitch
                v-if="showPreviewResponseVariantSwitch"
                :model-value="activePreviewVariant"
                :prefix-label="$t('trafficAnalysis.repeater.contextMenu.response')"
                :active-label="previewResponseVariantLabel"
                :original-label="$t('trafficAnalysis.history.detailsPanel.originalResponse')"
                :edited-label="$t('trafficAnalysis.history.detailsPanel.editedResponse')"
                button-class="px-1.5 font-semibold normal-case"
                :menu-width-px="144"
                test-id="preview-response-variant-switch"
                @update:model-value="switchPreviewVariant"
              />
              <span v-else class="font-semibold text-sm">{{ $t('trafficAnalysis.repeater.contextMenu.response') }}</span>
              <button
                v-if="!immersiveDrillModeEnabled"
                class="btn btn-ghost btn-xs"
                type="button"
                :disabled="!canCompareCurrentResponseVersions"
                @click="compareCurrentResponseVersions"
                :title="$t('trafficAnalysis.repeater.actions.compareResponseVersions')"
              >
                <i class="fas fa-not-equal"></i>
                <span v-if="!isResponsePaneCompact">{{ $t('trafficAnalysis.repeater.actions.compareResponseVersions') }}</span>
              </button>
              <template v-if="currentTab.response">
                <span 
                  class="badge badge-sm"
                  :class="getStatusClass(currentTab.response.statusCode)"
                >
                  {{ currentTab.response.statusCode }}
                </span>
                <span
                  v-if="currentResponseMeta"
                  class="text-xs text-base-content/70"
                  :title="currentResponseMeta"
                >
                  {{ currentResponseMeta }}
                </span>
              </template>
            </div>
            <div class="repeater-pane-header-controls ml-auto flex min-w-0 items-center gap-1.5">
              <TrafficMessageViewTabs
                :model-value="currentTab.responseTab"
                :tabs="responseViewTabs"
                :compact="isResponsePaneCompact"
                @update:model-value="currentTab.responseTab = $event as RepeaterTab['responseTab']"
              />
              <TrafficMessageDisplayControls
                v-if="currentTab.responseTab !== 'render'"
                :mode-label="''"
                :compact="isResponsePaneCompact"
                :show-line-endings="false"
              />
            </div>
          </div>

          <!-- Response Content -->
          <div class="flex-1 overflow-hidden" @contextmenu.prevent="showContextMenu($event, 'response')">
            <div
              v-if="currentTab.isSending || currentTab.response || currentTab.rawResponse"
              key="response-viewer"
              class="h-full min-h-0"
            >
              <!-- Pretty/Raw View -->
              <HttpMessageSurface
                v-if="currentTab.responseTab === 'pretty' || currentTab.responseTab === 'raw'"
                key="response-text-viewer"
                ref="responseEditor"
                :modelValue="currentTab.responseTab === 'pretty' ? formatRepeaterPrettyResponse(currentTab.response, settings) : currentDisplayedRawResponse"
                readonly
                message-type="response"
                custom-context-menu
                show-search-bar
                @contextmenu="showContextMenu($event, 'response')"
                height="100%"
                :display-mode="resolveTrafficTextDisplayMode(currentTab.responseTab)"
                :state-key="buildRepeaterResponseStateKey(currentTab.id, currentTab.responseTab)"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
              />
              
              <!-- Hex View -->
              <TrafficMessageReader
                v-else-if="currentTab.responseTab === 'hex'"
                key="response-hex-viewer"
                ref="responseEditor"
                :model-value="repeaterTextToHex(currentDisplayedRawResponse)"
                custom-context-menu
                show-search-bar
                @contextmenu="showContextMenu($event, 'response')"
                height="100%"
                :state-key="buildRepeaterResponseStateKey(currentTab.id, 'hex')"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
              />
              
              <!-- Render View -->
              <TrafficResponseRenderPane
                v-else-if="currentTab.responseTab === 'render'"
                key="response-render-viewer"
                :body="currentDisplayedResponseBody"
                :content-type="getCurrentResponseContentType()"
              />
            </div>
            <div v-else key="response-empty" class="flex items-center justify-center w-full h-full text-base-content/50">
              <div class="text-center">
                <i class="fas fa-inbox text-4xl mb-2"></i>
                <p>{{ $t('trafficAnalysis.repeater.contextMenu.clickSendToSendRequest') }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onActivated, onDeactivated, onMounted, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit as tauriEmit } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { openTrafficAssistantPanel } from '@/services/trafficAssistantWorkspace'
import { dialog } from '@/composables/useDialog';
import { createTrafficOastToken } from '@/api/trafficOast'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import TrafficMessageReader from '@/components/traffic/TrafficMessageReader.vue'
import TrafficMessageDisplayControls from '@/components/traffic/TrafficMessageDisplayControls.vue'
import TrafficMessageViewTabs from '@/components/traffic/TrafficMessageViewTabs.vue'
import TrafficResponseRenderPane from './TrafficResponseRenderPane.vue'
import TrafficVariantSwitch from './TrafficVariantSwitch.vue'
import { IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS, IMMERSIVE_TRAFFIC_TOP_BAR_CLASS } from './immersiveTrafficUi'
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import { buildSourceRequestFromRawRequest } from '@/components/traffic/intruder/http'
import type { HttpExchangeRequest, HttpReplayResponse } from './http/model'
import { findHeaderValue } from './http/headers'
import { buildHttpExchangeRequestFromRawRequest } from './http/parser'
import { buildHttpReplayResponseFromCommandResult, type RawReplayCommandResult } from './http/response'
import type { TrafficComparePayload, TrafficComparerDraftRequestInput } from './transfers'
import { getDefaultTrafficMessageViewTab, useTrafficDisplaySettings } from './trafficDisplaySettings'
import { useTrafficSendTargets } from './trafficSendTargets'
import { buildTrafficContextMenuSections } from './trafficContextMenuSectionSupport'
import { buildTrafficRequestContextMenuSections } from './trafficRequestContextMenuSupport'
import { formatRepeaterBytes as formatBytes, formatRepeaterResponseMeta, generateRepeaterId, getRepeaterStatusClass as getStatusClass } from './proxyRepeaterUiSupport';
import { buildRepeaterAssistantTrafficData } from './proxyRepeaterAssistantTransferSupport'
import { buildTrafficRequestActionMenuItems } from './trafficRequestActionMenuSupport'
import { buildTrafficRequestSendMenuItems } from './trafficSendMenuSupport'
import { buildRepeaterRequestVersionComparePayload, buildRepeaterResponseVersionComparePayload, canCompareRepeaterRequestVersions, canCompareRepeaterResponseVersions } from './trafficRepeaterComparerSupport'
import { convertRepeaterPrettyRequestToRaw, formatRepeaterPrettyRequest } from './trafficRepeaterPrettyRequestSupport'
import { buildTrafficDisplayedRawResponse, resolveTrafficResponseBodyText } from './trafficResponseDecodingSupport'
import { formatRepeaterPrettyResponse } from './proxyRepeaterResponsePresentationSupport'
import { useTrafficPaneCompactMode } from './useTrafficPaneCompactMode'
import { buildRepeaterCurlCommand, buildRepeaterFullUrl, parseRepeaterErrorMessage, repeaterTextToHex, syncRepeaterTabTargetFromRequest } from './proxyRepeaterRequestSupport'
import { clearRepeaterTabsStorage, REPEATER_STORAGE_KEY_LAYOUT, REPEATER_STORAGE_KEY_LEFT_WIDTH, REPEATER_STORAGE_KEY_TOP_HEIGHT } from './proxyRepeaterStorageSupport'
import { restoreRepeaterTabResponseFromReplayRuns } from './proxyRepeaterReplayResponseRestoreSupport'
import { createRepeaterTab } from './proxyRepeaterTabSupport'
import type { RepeaterActiveTabState, RepeaterRequestTab, RepeaterResponseTab, RepeaterTab, RepeaterTabStats, ReplayCommandResponse } from './proxyRepeaterTypes'
import { buildRepeaterCompareLabels, buildRepeaterRequestViewTabs, buildRepeaterResponseViewTabs } from './proxyRepeaterViewSupport'
import { buildRepeaterRequestStateKey, buildRepeaterResponseStateKey, resolveTrafficTextDisplayMode } from './trafficMessagePresentationSupport'
import { isEditableKeyboardTarget } from '@/utils/editableKeyboardTarget'
import { useTrafficWorkbenchStore } from './workbench/stores/useTrafficWorkbenchStore'
import type { RequestDraft } from './workbench/model/requestDraft'
import type { TrafficWorkbenchRequestContext, TrafficWorkbenchRequestVariant } from './trafficWorkbenchTypes'

const { t, locale } = useI18n();
const { enabledTargets } = useTrafficSendTargets()
const { settings } = useTrafficDisplaySettings()
const REQUEST_PANE_COMPACT_THRESHOLD = 640
const RESPONSE_PANE_COMPACT_THRESHOLD = 760
const { panelRef: requestPanelRef, isCompact: isRequestPaneCompact } = useTrafficPaneCompactMode(REQUEST_PANE_COMPACT_THRESHOLD)
const { panelRef: responsePanelRef, isCompact: isResponsePaneCompact } = useTrafficPaneCompactMode(RESPONSE_PANE_COMPACT_THRESHOLD)
const requestViewTabs = computed(() => [...buildRepeaterRequestViewTabs(t, locale.value)])
const responseViewTabs = computed(() => [...buildRepeaterResponseViewTabs(t, locale.value)])
const emit = defineEmits<{
  (e: 'openCompare', payload: TrafficComparePayload): void
  (e: 'openDraftCompare', payload: TrafficComparerDraftRequestInput): void
  (e: 'createAttackWorkspace', request: HttpExchangeRequest): void
  (e: 'activeTabModeChanged', state: RepeaterActiveTabState): void
  (e: 'tabStatsChanged', stats: RepeaterTabStats): void
  (e: 'switchPreviewVariant', variant: TrafficWorkbenchRequestVariant): void
}>()

// Props
const props = defineProps<{
  initialRequest?: HttpExchangeRequest
  initialDraftId?: string
  activeRequestContext?: TrafficWorkbenchRequestContext | null
}>()

// Refs
const tabs = ref<RepeaterTab[]>([]);
const activeTabIndex = ref(0);
const showTargetDialog = ref(false);
const repeaterRoot = ref<HTMLElement | null>(null);
const requestEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null);
const responseEditor = ref<{
  focus?: () => void
  focusSearch?: () => void
  getContent?: () => string
  getSelectionRange?: () => { from: number; to: number }
  selectAll?: () => void
  setSelection?: (from: number, to: number) => void
} | null>(null);

// 请求取消控制器映射（每个 tab 一个）
const abortControllers = new Map<string, { cancelled: boolean }>();

// 历史记录限制
const MAX_TABS = 50; // 最多保留50个标签页

// 右键菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  width: 0,
  height: 0,
  pane: 'request' as 'request' | 'response',
});
const tabContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  tabIndex: -1,
})
const creatingOastPayload = ref(false)

// Layout
const layoutMode = ref<'horizontal' | 'vertical'>(
  (localStorage.getItem(REPEATER_STORAGE_KEY_LAYOUT) as 'horizontal' | 'vertical') || 'horizontal'
);
const leftPanelWidth = ref(parseInt(localStorage.getItem(REPEATER_STORAGE_KEY_LEFT_WIDTH) || '600'));
const topPanelHeight = ref(parseInt(localStorage.getItem(REPEATER_STORAGE_KEY_TOP_HEIGHT) || '350'));

let isResizing = false;
let startX = 0;
let startY = 0;
let startWidth = 0;
let startHeight = 0;
let hostDetectionTimer: number | null = null;
let repeaterScrollState = { top: 0, left: 0 };
let applyingWorkbenchDraft = false;
let syncingCurrentTabToDraft = false;
let suppressedPreviewPromotionCount = 0;

const workbenchState = useTrafficWorkbenchStore()

function suppressPreviewPromotionDuring(task: () => void) {
  suppressedPreviewPromotionCount += 1
  try {
    task()
  } finally {
    void nextTick(() => {
      suppressedPreviewPromotionCount = Math.max(0, suppressedPreviewPromotionCount - 1)
    })
  }
}

// Computed
const currentTab = computed(() => {
  if (tabs.value.length === 0) return null;
  return tabs.value[activeTabIndex.value] || null;
});

const activePreviewContext = computed(() => {
  if (!currentTab.value || currentTab.value.mode !== 'preview') {
    return null
  }
  return props.activeRequestContext ?? null
})

const activePreviewVariant = computed(() => activePreviewContext.value?.variant ?? 'original')
const isPreviewingEditedVariant = computed(() => activePreviewVariant.value === 'edited')
const previewRequestVariantLabel = computed(() => (
  isPreviewingEditedVariant.value
    ? (locale.value.startsWith('zh') ? '修改后' : 'Edited')
    : (locale.value.startsWith('zh') ? '原始' : 'Original')
))
const previewResponseVariantLabel = computed(() => (
  isPreviewingEditedVariant.value
    ? (locale.value.startsWith('zh') ? '修改后' : 'Edited')
    : (locale.value.startsWith('zh') ? '原始' : 'Original')
))
const showPreviewRequestVariantSwitch = computed(() => (
  Boolean(activePreviewContext.value?.hasEditedVariant)
))
const showPreviewResponseVariantSwitch = computed(() => (
  Boolean(activePreviewContext.value?.hasEditedResponseVariant)
))

function emitActiveTabModeChanged() {
  emit('activeTabModeChanged', {
    mode: currentTab.value?.mode ?? null,
    draftId: currentTab.value?.draftId ?? null,
    sourceRequestId: currentTab.value?.sourceRequestId ?? null,
  })
}

function emitTabStatsChanged() {
  emit('tabStatsChanged', {
    editedTabCount: tabs.value.filter(tab => tab.userEdited).length,
  })
}

const showPort = computed(() => {
  if (!currentTab.value) return false;
  const port = currentTab.value.targetPort;
  const useTls = currentTab.value.useTls;
  if (useTls && port === 443) return false;
  if (!useTls && port === 80) return false;
  return true;
});

const currentRequestProtocol = computed(() => {
  const requestLine = currentTab.value?.rawRequest.split(/\r\n|\r|\n/)[0]?.trim() || ''
  const protocol = requestLine.split(/\s+/)[2] || ''
  return protocol || (currentTab.value?.useTls ? 'HTTP/1.1' : 'HTTP/1.1')
})
const currentResponseMeta = computed(() => {
  if (!currentTab.value?.response) return ''
  return formatRepeaterResponseMeta(
    currentTab.value.response.rawText,
    currentTab.value.response.responseTimeMs,
  )
})
const currentDisplayedResponseBody = computed(() => {
  if (!currentTab.value?.response) return ''

  return resolveTrafficResponseBodyText(
    currentTab.value.response.bodyText || '',
    getCurrentResponseContentType(),
    settings.value,
    currentTab.value.response.bodyBytesBase64,
  )
})
const currentDisplayedRawResponse = computed(() => {
  if (!currentTab.value?.response) {
    return currentTab.value?.rawResponse || ''
  }

  return buildTrafficDisplayedRawResponse(currentTab.value.response, settings.value)
})

// 向后兼容的 isSending（用于模板）
const isSending = computed(() => currentTab.value?.isSending || false);
const repeaterSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['compare', 'attackWorkspace'],
    actions: {
      compare: contextMenuSendToComparer,
      attackWorkspace: contextMenu.value.pane === 'request' ? contextMenuSendToIntruder : undefined,
    },
  }),
)
const repeaterRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl'],
    actions: {
      copyUrl: contextMenuCopyUrl,
      copyRequest: contextMenuCopyRequest,
      copyAsCurl: contextMenuCopyCurl,
    },
  }),
)
const repeaterCompareMenuItems = computed(() =>
  [
    contextMenu.value.pane === 'request' ? {
      key: 'compareRequestVersions',
      iconClass: 'fas fa-not-equal text-accent',
      labelKey: 'openCompare',
      onClick: () => {
        hideContextMenu()
        compareCurrentRequestVersions()
      },
      disabled: !canCompareCurrentRequestVersions.value,
    } : null,
    contextMenu.value.pane === 'response' ? {
      key: 'compareResponseVersions',
      iconClass: 'fas fa-not-equal text-accent',
      labelKey: 'openCompare',
      onClick: () => {
        hideContextMenu()
        compareCurrentResponseVersions()
      },
      disabled: !canCompareCurrentResponseVersions.value,
    } : null,
  ],
)
const repeaterContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: contextMenu.value.pane === 'request'
      ? [
          {
            key: 'sendRequest',
            iconClass: 'fas fa-paper-plane text-primary',
            labelKey: 'sendRequest',
            onClick: contextMenuSend,
          },
          {
            key: 'sendToNewTab',
            iconClass: 'fas fa-plus text-success',
            labelKey: 'sendToNewTab',
            onClick: contextMenuSendToNewTab,
          },
          ...repeaterSendMenuItems.value,
        ]
      : repeaterSendMenuItems.value.filter((item) => item.key === 'compare'),
    compareItems: repeaterCompareMenuItems.value,
    requestItems: contextMenu.value.pane === 'request' ? repeaterRequestActionMenuItems.value : [],
    assistantItems: contextMenu.value.pane === 'request'
      ? [
          {
            key: 'sendToAssistant',
            iconClass: 'fas fa-upload text-accent',
            labelKey: 'sendToAssistant',
            onClick: contextMenuSendRequestToAssistant,
          },
        ]
      : [],
    editorItems: contextMenu.value.pane === 'request'
      ? [
          {
            key: 'insertOastPayload',
            iconClass: 'fas fa-satellite-dish text-info',
            labelKey: 'insertOastPayload',
            onClick: insertOastPayloadIntoCurrentRequest,
          },
        ]
      : [],
  }),
)
const repeaterTabContextMenuSections = computed(() =>
  buildTrafficContextMenuSections([
    {
      key: 'tab-actions',
      items: [
        {
          key: 'deleteCurrent',
          iconClass: 'fas fa-trash-alt text-error',
          labelKey: 'deleteCurrent',
          onClick: () => {
            const index = tabContextMenu.value.tabIndex
            hideTabContextMenu()
            if (index >= 0) void closeTab(index)
          },
        },
        {
          key: 'deleteOthers',
          iconClass: 'fas fa-layer-group text-warning',
          labelKey: 'deleteOthers',
          onClick: () => {
            const index = tabContextMenu.value.tabIndex
            hideTabContextMenu()
            if (index >= 0) void deleteOtherTabs(index)
          },
        },
        {
          key: 'deleteAll',
          iconClass: 'fas fa-trash text-error',
          labelKey: 'deleteAll',
          onClick: () => {
            hideTabContextMenu()
            void clearAllTabs()
          },
          disabled: tabs.value.length === 0,
        },
      ],
    },
  ]),
)
const canCompareCurrentRequestVersions = computed(() => canCompareRepeaterRequestVersions(currentTab.value ?? null));
const canCompareCurrentResponseVersions = computed(() => canCompareRepeaterResponseVersions(currentTab.value ?? null));

// Methods
function createTab(request?: HttpExchangeRequest): RepeaterTab {
  return createRepeaterTab({
    request,
    fallbackNameIndex: tabs.value.length + 1,
    generateId: generateRepeaterId,
    defaultRequestTab: getDefaultTrafficMessageViewTab() as RepeaterRequestTab,
    defaultResponseTab: getDefaultTrafficMessageViewTab() as RepeaterResponseTab,
  })
}

function buildDraftEndpointFromTab(tab: RepeaterTab) {
  return {
    scheme: tab.useTls ? 'https' : 'http',
    host: tab.targetHost,
    port: tab.targetPort || (tab.useTls ? 443 : 80),
    sniHost: tab.overrideSni && tab.sniHost.trim() ? tab.sniHost.trim() : undefined,
  } as const
}

function buildExchangeRequestFromTab(tab: RepeaterTab): HttpExchangeRequest | null {
  return buildHttpExchangeRequestFromRawRequest(tab.rawRequest, buildDraftEndpointFromTab(tab))
}

function syncTabFromDraft(tab: RepeaterTab, draft: RequestDraft) {
  tab.draftId = draft.id
  tab.mode = 'draft'
  tab.name = draft.title
  tab.sourceRequestId = draft.source?.requestId ?? tab.sourceRequestId
  tab.targetHost = draft.endpoint.host
  tab.targetPort = draft.endpoint.port
  tab.useTls = draft.endpoint.scheme === 'https'
  tab.overrideSni = Boolean(draft.endpoint.sniHost)
  tab.sniHost = draft.endpoint.sniHost || ''
  tab.rawRequest = draft.rawRequest
  tab.prettyRequest = formatRepeaterPrettyRequest(draft.rawRequest)
  tab.requestTab = draft.preferredView === 'raw' ? 'raw' : 'pretty'
  restoreRepeaterTabResponseFromReplayRuns(tab, workbenchState.replay.replayRuns.value)
}

function restoreMissingDraftTabResponsesFromReplayRuns() {
  for (const tab of tabs.value) {
    if (!tab.draftId || tab.isSending || tab.response || tab.rawResponse) {
      continue
    }
    restoreRepeaterTabResponseFromReplayRuns(tab, workbenchState.replay.replayRuns.value)
  }
}

function createTabFromDraft(draft: RequestDraft): RepeaterTab {
  const exchangeRequest = buildHttpExchangeRequestFromRawRequest(draft.rawRequest, draft.endpoint)
  const tab = createTab(exchangeRequest || undefined)
  tab.draftId = draft.id
  tab.mode = 'draft'
  syncTabFromDraft(tab, draft)
  tab.initialRawRequest = draft.rawRequest
  tab.modified = false
  tab.userEdited = true
  return tab
}

function syncTabFromExchangeRequest(tab: RepeaterTab, request: HttpExchangeRequest) {
  const nextTab = createTab(request)
  tab.name = nextTab.name
  tab.sourceRequestId = nextTab.sourceRequestId
  tab.targetHost = nextTab.targetHost
  tab.targetPort = nextTab.targetPort
  tab.useTls = nextTab.useTls
  tab.overrideSni = false
  tab.sniHost = ''
  tab.initialRawRequest = nextTab.rawRequest
  tab.rawRequest = nextTab.rawRequest
  tab.prettyRequest = nextTab.prettyRequest
  tab.requestTab = nextTab.requestTab
  tab.responseTab = nextTab.responseTab
  tab.response = nextTab.response
  tab.rawResponse = nextTab.rawResponse
  tab.lastCompletedRawResponse = nextTab.lastCompletedRawResponse
  tab.previousRawResponse = ''
  tab.isSending = false
  tab.modified = false
  tab.userEdited = false
}

function openPreviewRequest(request: HttpExchangeRequest) {
  suppressPreviewPromotionDuring(() => {
    const existingPreviewIndex = tabs.value.findIndex(tab => tab.mode === 'preview')
    applyingWorkbenchDraft = true
    try {
      if (existingPreviewIndex >= 0) {
        const tab = tabs.value[existingPreviewIndex]
        tab.draftId = null
        tab.mode = 'preview'
        syncTabFromExchangeRequest(tab, request)
        activeTabIndex.value = existingPreviewIndex
        return
      }

      if (tabs.value.length >= MAX_TABS) {
        dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }))
        return
      }

      const tab = createTab(request)
      tab.mode = 'preview'
      tab.draftId = null
      tabs.value.push(tab)
      activeTabIndex.value = tabs.value.length - 1
    } finally {
      applyingWorkbenchDraft = false
    }
  })
}

function switchPreviewVariant(variant: TrafficWorkbenchRequestVariant) {
  if (!activePreviewContext.value || activePreviewContext.value.variant === variant) {
    return
  }
  emit('switchPreviewVariant', variant)
}

function ensureDraftForTab(tab: RepeaterTab) {
  if (tab.draftId) {
    tab.userEdited = true
    return findDraftById(tab.draftId)
  }

  const exchangeRequest = buildExchangeRequestFromTab(tab)
  if (!exchangeRequest) {
    return null
  }

  const draft = workbenchState.drafts.createDraftFromExchangeRequest({
    request: exchangeRequest,
    source: tab.sourceRequestId
      ? { kind: 'history', label: `历史记录 #${tab.sourceRequestId}`, requestId: tab.sourceRequestId }
      : { kind: 'repeater', label: t('trafficAnalysis.tabs.repeater', '重放器') },
    title: tab.name,
  })
  tab.draftId = draft.id
  tab.mode = 'draft'
  tab.userEdited = true
  workbenchState.drafts.selectDraft(draft.id)
  workbenchState.selection.selectDraft(draft)
  return draft
}

function findDraftById(draftId: string | null | undefined) {
  if (!draftId) {
    return null
  }
  return workbenchState.drafts.drafts.value.find(draft => draft.id === draftId) ?? null
}

function openDraftInRepeater(draftId: string | null | undefined) {
  const draft = findDraftById(draftId)
  if (!draft) {
    return
  }

  const existingIndex = tabs.value.findIndex(tab => tab.draftId === draft.id)
  applyingWorkbenchDraft = true
  try {
    if (existingIndex >= 0) {
      syncTabFromDraft(tabs.value[existingIndex], draft)
      activeTabIndex.value = existingIndex
      return
    }

    if (tabs.value.length >= MAX_TABS) {
      dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }))
      return
    }

    const tab = createTabFromDraft(draft)
    tabs.value.push(tab)
    activeTabIndex.value = tabs.value.length - 1
  } finally {
    applyingWorkbenchDraft = false
  }
}

function openAllDraftsInRepeater() {
  const activeDraftId = workbenchState.drafts.activeDraftId.value
  const draftIds = workbenchState.drafts.drafts.value.map(draft => draft.id)
  if (!draftIds.length) {
    if (!tabs.value.some(tab => tab.mode === 'draft')) return
    abortControllers.forEach(controller => { controller.cancelled = true })
    abortControllers.clear()
    tabs.value = []
    activeTabIndex.value = 0
    return
  }
  for (const draftId of draftIds) {
    if (draftId !== activeDraftId) {
      openDraftInRepeater(draftId)
    }
  }
  if (activeDraftId) {
    openDraftInRepeater(activeDraftId)
  }
}

function syncCurrentTabBackToDraft() {
  if (applyingWorkbenchDraft || syncingCurrentTabToDraft || !currentTab.value) {
    return
  }

  const draft = findDraftById(currentTab.value.draftId)
  if (!draft) {
    return
  }

  syncingCurrentTabToDraft = true
  try {
    const nextRawRequest = currentTab.value.rawRequest
    if (draft.rawRequest !== nextRawRequest) {
      workbenchState.drafts.updateDraftRequest(draft.id, nextRawRequest)
    }

    const nextEndpoint = buildDraftEndpointFromTab(currentTab.value)
    const endpointChanged = draft.endpoint.scheme !== nextEndpoint.scheme
      || draft.endpoint.host !== nextEndpoint.host
      || draft.endpoint.port !== nextEndpoint.port
      || (draft.endpoint.sniHost || '') !== (nextEndpoint.sniHost || '')
    if (endpointChanged) {
      workbenchState.drafts.updateDraftEndpoint(draft.id, nextEndpoint, 'manual')
    }

    if (workbenchState.drafts.activeDraftId.value !== draft.id) {
      workbenchState.drafts.selectDraft(draft.id)
    }
  } finally {
    syncingCurrentTabToDraft = false
  }
}

function createDraftBackedTab(request?: HttpExchangeRequest) {
  const baseTab = createTab(request)
  if (!baseTab.targetHost) {
    syncRepeaterTabTargetFromRequest(baseTab, baseTab.rawRequest)
  }
  const exchangeRequest = buildExchangeRequestFromTab(baseTab)
  if (!exchangeRequest) {
    tabs.value.push(baseTab)
    activeTabIndex.value = tabs.value.length - 1
    return
  }

  const draft = workbenchState.drafts.createDraftFromExchangeRequest({
    request: exchangeRequest,
    source: { kind: 'repeater', label: t('trafficAnalysis.tabs.repeater', '重放器') },
    title: baseTab.name,
  })
  workbenchState.drafts.selectDraft(draft.id)
  openDraftInRepeater(draft.id)
}

function addTab() {
  if (tabs.value.length >= MAX_TABS) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }));
    return;
  }

  createDraftBackedTab();
}

async function confirmDiscardTabs(candidateTabs: RepeaterTab[]) {
  if (!candidateTabs.some(tab => tab.modified && tab.rawRequest.trim())) return true

  return dialog.confirm({
    title: t('trafficAnalysis.repeater.messages.confirmCloseAllTabs'),
    message: t('trafficAnalysis.repeater.messages.confirmCloseAllTabsMessage'),
    variant: 'warning',
  })
}

async function clearAllTabs() {
  if (tabs.value.length === 0) return

  if (!await confirmDiscardTabs(tabs.value)) return

  abortControllers.forEach(controller => { controller.cancelled = true })
  abortControllers.clear()
  tabs.value = []
  activeTabIndex.value = 0
  showTargetDialog.value = false
  clearRepeaterTabsStorage()
  workbenchState.drafts.resetDraftStore()
}

async function deleteOtherTabs(index: number) {
  const targetTab = tabs.value[index]
  if (!targetTab) return

  const tabsToRemove = tabs.value.filter((_, currentIndex) => currentIndex !== index)
  if (!await confirmDiscardTabs(tabsToRemove)) return

  tabsToRemove.forEach(tab => {
    abortControllers.delete(tab.id)
    if (tab.draftId) workbenchState.drafts.removeDraft(tab.draftId)
  })
  tabs.value = [targetTab]
  activeTabIndex.value = 0
  if (targetTab.draftId) {
    workbenchState.drafts.selectDraft(targetTab.draftId)
  }
}

async function closeTab(index: number) {
  const tab = tabs.value[index];
  
  // 检查是否有未保存的修改
  if (tab && tab.modified && tab.rawRequest.trim()) {
    const confirmed = await dialog.confirm({
      title: t('trafficAnalysis.repeater.messages.confirmCloseTab'),
      message: t('trafficAnalysis.repeater.messages.confirmCloseTabMessage'),
    });
    
    if (!confirmed) {
      return;
    }
  }
  
  // 清理该 tab 的取消控制器
  if (tab) {
    abortControllers.delete(tab.id);
  }
  
  if (tabs.value.length === 1) {
    tabs.value = [];
    if (tab.draftId) {
      workbenchState.drafts.removeDraft(tab.draftId)
    }
    createDraftBackedTab()
    return;
  }
  
  tabs.value.splice(index, 1);
  if (tab.draftId) {
    workbenchState.drafts.removeDraft(tab.draftId)
  }
  if (activeTabIndex.value >= tabs.value.length) {
    activeTabIndex.value = tabs.value.length - 1;
  }
  
}

function selectTab(index: number) {
  activeTabIndex.value = index;
  syncCurrentTabBackToDraft()
}

function cancelRequest() {
  if (!currentTab.value) return;
  
  const controller = abortControllers.get(currentTab.value.id);
  if (controller) {
    controller.cancelled = true;
  }
  
  currentTab.value.isSending = false;
  dialog.toast.info(t('trafficAnalysis.repeater.messages.requestCancelled'));
}

function refocusRequestEditor() {
  void nextTick(() => {
    requestEditor.value?.focus?.()
  })
}

function syncCurrentRequestEditorContentToTab() {
  const tab = currentTab.value
  if (!tab || tab.requestTab === 'hex') return

  const editorContent = requestEditor.value?.getContent?.()
  if (typeof editorContent !== 'string') return

  const nextRawRequest = tab.requestTab === 'pretty'
    ? convertRepeaterPrettyRequestToRaw(editorContent)
    : editorContent
  const nextPrettyRequest = tab.requestTab === 'pretty'
    ? editorContent
    : formatRepeaterPrettyRequest(editorContent)

  if (tab.rawRequest !== nextRawRequest || tab.prettyRequest !== nextPrettyRequest) {
    tab.rawRequest = nextRawRequest
    tab.prettyRequest = nextPrettyRequest
    tab.modified = true
    tab.userEdited = true
  }

  syncRepeaterTabTargetFromRequest(tab, tab.rawRequest)
}

async function sendRequest() {
  if (!currentTab.value || currentTab.value.isSending) return;

  syncCurrentRequestEditorContentToTab()
  
  if (!currentTab.value.targetHost || !currentTab.value.rawRequest.trim()) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.fillTargetAndRequest'));
    return;
  }
  
  const tab = currentTab.value;
  const draft = ensureDraftForTab(tab)
  syncCurrentTabBackToDraft()
  const revision = draft ? workbenchState.drafts.appendRevision(draft.id, 'send') : null
  const replayRun = draft && revision
    ? workbenchState.replay.startReplayRun(draft.id, revision.id)
    : null
  
  // 验证端口号
  if (!tab.targetPort || tab.targetPort < 1 || tab.targetPort > 65535) {
    dialog.toast.error(t('trafficAnalysis.repeater.messages.invalidPort'));
    return;
  }
  
  // 验证主机名
  if (!tab.targetHost || tab.targetHost.trim().length === 0) {
    dialog.toast.error(t('trafficAnalysis.repeater.messages.invalidHost'));
    return;
  }
  
  tab.isSending = true;
  tab.response = null;
  tab.rawResponse = '';
  refocusRequestEditor()
  
  // 创建取消控制器
  const controller = { cancelled: false };
  abortControllers.set(tab.id, controller);
  
  const tabId = tab.id;
  
  try {
    const exchangeRequest = buildSourceRequestFromRawRequest(tab.rawRequest, {
      host: tab.targetHost,
      port: tab.targetPort || 443,
      useTls: tab.useTls,
    }, tab.sourceRequestId)
    if (!exchangeRequest) {
      throw new Error('Invalid request')
    }
    if (tab.overrideSni && tab.sniHost.trim()) {
      exchangeRequest.endpoint.sniHost = tab.sniHost.trim()
    }
    tab.prettyRequest = formatRepeaterPrettyRequest(tab.rawRequest)
    
    const response = await invoke<ReplayCommandResponse<RawReplayCommandResult>>('replay_raw_request', {
      endpoint: exchangeRequest.endpoint,
      request: exchangeRequest.request,
      timeoutSecs: 30,
      originKind: 'draft',
      originRefId: tab.id,
      parentRequestId: tab.sourceRequestId,
      sourceDraftRevisionId: revision?.id ?? null,
    });
    
    // 检查是否已取消
    if (controller.cancelled) return;
    
    // 查找目标 tab（可能已被删除或切换）
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (!targetTab) return;
    
    if (response.success && response.data) {
      targetTab.previousRawResponse = targetTab.lastCompletedRawResponse;
      const replayResponse = buildHttpReplayResponseFromCommandResult(response.data)
      targetTab.lastCompletedRawResponse = replayResponse.rawText;
      targetTab.rawResponse = replayResponse.rawText;
      
      // 检查响应体大小
      const responseSize = replayResponse.rawText.length;
      const MAX_RESPONSE_SIZE = 10 * 1024 * 1024; // 10MB
      if (responseSize > MAX_RESPONSE_SIZE) {
        dialog.toast.warning(t('trafficAnalysis.repeater.messages.largeResponse', { 
          size: formatBytes(responseSize) 
        }));
      }
      
      // 解析响应
      targetTab.response = replayResponse;
      
      targetTab.name = targetTab.targetHost;
      targetTab.modified = false;
      if (replayRun) {
        workbenchState.replay.completeReplayRun(replayRun.id, replayResponse)
      }
    } else {
      const errorMsg = parseRepeaterErrorMessage(response.error, t);
      dialog.toast.error(errorMsg);
      if (replayRun) {
        workbenchState.replay.failReplayRun(replayRun.id, errorMsg)
      }
    }
  } catch (error: any) {
    if (controller.cancelled) return;
    
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (!targetTab) return;
    
    console.error('Failed to send request:', error);
    const errorMsg = parseRepeaterErrorMessage(error, t);
    dialog.toast.error(errorMsg);
    if (replayRun) {
      workbenchState.replay.failReplayRun(replayRun.id, errorMsg)
    }
  } finally {
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (targetTab) {
      targetTab.isSending = false;
    }
    abortControllers.delete(tabId);
  }
}

function formatPrettyRequest(): string {
  return currentTab.value?.prettyRequest || ''
}

function onPrettyRequestUpdate(value: string) {
  if (!currentTab.value) return;
  if (suppressedPreviewPromotionCount > 0) {
    currentTab.value.prettyRequest = value
    currentTab.value.rawRequest = convertRepeaterPrettyRequestToRaw(value)
    return
  }
  if (currentTab.value.mode === 'preview') {
    ensureDraftForTab(currentTab.value)
  }

  currentTab.value.prettyRequest = value
  currentTab.value.rawRequest = convertRepeaterPrettyRequestToRaw(value)
  syncRepeaterTabTargetFromRequest(currentTab.value, value);
  currentTab.value.modified = true;
  currentTab.value.userEdited = true;
}

function getCurrentResponseContentType(): string {
  if (!currentTab.value?.response) return '';
  return findHeaderValue(currentTab.value.response.headers, 'content-type') || '';
}

// 右键菜单
function showContextMenu(event: MouseEvent, pane: 'request' | 'response' = 'request') {
  hideTabContextMenu()
  // 估算菜单尺寸（基于菜单项数量）
  const MENU_WIDTH = 220;
  const MENU_HEIGHT = 400;
  
  // 计算菜单位置，确保不超出视口
  let x = event.clientX;
  let y = event.clientY;
  
  // 考虑滚动位置
  const scrollX = window.scrollX || window.pageXOffset;
  const scrollY = window.scrollY || window.pageYOffset;
  
  // 调整位置避免超出视口
  if (x + MENU_WIDTH > window.innerWidth) {
    x = window.innerWidth - MENU_WIDTH - 10;
  }
  if (y + MENU_HEIGHT > window.innerHeight) {
    y = window.innerHeight - MENU_HEIGHT - 10;
  }
  
  contextMenu.value = {
    visible: true,
    x: Math.max(0, x),
    y: Math.max(0, y),
    width: MENU_WIDTH,
    height: MENU_HEIGHT,
    pane,
  };
  
  setTimeout(() => {
    document.addEventListener('click', hideContextMenu);
  }, 0);
}

function showTabContextMenu(event: MouseEvent, index: number) {
  hideContextMenu()
  const MENU_WIDTH = 180
  const MENU_HEIGHT = 136
  let x = event.clientX
  let y = event.clientY

  if (x + MENU_WIDTH > window.innerWidth) {
    x = window.innerWidth - MENU_WIDTH - 10
  }
  if (y + MENU_HEIGHT > window.innerHeight) {
    y = window.innerHeight - MENU_HEIGHT - 10
  }

  tabContextMenu.value = {
    visible: true,
    x: Math.max(0, x),
    y: Math.max(0, y),
    tabIndex: index,
  }

  window.setTimeout(() => {
    document.addEventListener('click', hideTabContextMenu)
  }, 0)
}

function hideContextMenu() {
  contextMenu.value.visible = false;
  document.removeEventListener('click', hideContextMenu);
}

function hideTabContextMenu() {
  tabContextMenu.value.visible = false
  document.removeEventListener('click', hideTabContextMenu)
}

function contextMenuSend() {
  hideContextMenu();
  sendRequest();
}

function contextMenuSendToNewTab() {
  hideContextMenu();
  if (!currentTab.value) return;
  
  if (tabs.value.length >= MAX_TABS) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }));
    return;
  }

  const request = buildCurrentRequestTransfer()
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForIntruder'))
    return
  }
  createDraftBackedTab(request)
}

function contextMenuCopyUrl() {
  hideContextMenu();
  const url = buildRepeaterFullUrl(currentTab.value);
  if (url) {
    navigator.clipboard.writeText(url)
      .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.urlCopied')))
      .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
  }
}

function buildCurrentRequestTransfer(): HttpExchangeRequest | null {
  if (!currentTab.value) return null

  return buildSourceRequestFromRawRequest(currentTab.value.rawRequest, {
    host: currentTab.value.targetHost,
    port: currentTab.value.targetPort || (currentTab.value.useTls ? 443 : 80),
    useTls: currentTab.value.useTls,
  }, currentTab.value.sourceRequestId)
}

function insertTextAtSelection(
  content: string,
  selection: { from: number; to: number } | undefined,
  insert: string,
) {
  const from = Math.max(0, Math.min(selection?.from ?? content.length, content.length))
  const to = Math.max(from, Math.min(selection?.to ?? from, content.length))
  return {
    content: `${content.slice(0, from)}${insert}${content.slice(to)}`,
    selectionStart: from,
    selectionEnd: from + insert.length,
  }
}

async function insertOastPayloadIntoCurrentRequest() {
  hideContextMenu()

  if (!currentTab.value) {
    return
  }

  if (currentTab.value.requestTab === 'hex') {
    dialog.toast.warning(t('trafficAnalysis.oast.readOnlyMode'))
    return
  }

  creatingOastPayload.value = true
  try {
    const record = await createTrafficOastToken({
      label: currentTab.value.targetHost || undefined,
      sourceTool: 'repeater',
      sourceRequestId: currentTab.value.sourceRequestId,
    })
    const payloadText = record.httpsUrl || record.httpUrl || record.fqdn
    const currentText = currentTab.value.requestTab === 'pretty'
      ? currentTab.value.prettyRequest
      : currentTab.value.rawRequest
    const next = insertTextAtSelection(
      currentText,
      requestEditor.value?.getSelectionRange?.(),
      payloadText,
    )

    if (currentTab.value.requestTab === 'pretty') {
      currentTab.value.prettyRequest = next.content
      currentTab.value.rawRequest = convertRepeaterPrettyRequestToRaw(next.content)
    } else {
      currentTab.value.rawRequest = next.content
      currentTab.value.prettyRequest = formatRepeaterPrettyRequest(next.content)
    }

    currentTab.value.modified = true
    currentTab.value.userEdited = true
    await nextTick()
    requestEditor.value?.setSelection?.(next.selectionStart, next.selectionEnd)
    requestEditor.value?.focus?.()
    dialog.toast.success(t('trafficAnalysis.oast.inserted'))
  } catch (error) {
    console.error('[ProxyRepeater] Failed to insert OAST payload:', error)
    dialog.toast.error(String(error))
  } finally {
    creatingOastPayload.value = false
  }
}

function contextMenuSendToComparer() {
  hideContextMenu();

  if (!currentTab.value) {
    return
  }

  if (contextMenu.value.pane === 'response') {
    if (!currentTab.value.rawResponse.trim()) {
      dialog.toast.warning(t('trafficAnalysis.repeater.messages.noResponseData'))
      return
    }

    emit('openDraftCompare', {
      text: currentTab.value.rawResponse,
      messageType: 'response',
      name: currentTab.value.name || undefined,
      label: t('trafficAnalysis.repeater.contextMenu.response'),
    })
    dialog.toast.success(t('trafficAnalysis.repeater.messages.compareOpened'))
    return
  }

  const request = buildCurrentRequestTransfer()
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForComparer'))
    return
  }

  emit('openDraftCompare', {
    request,
    name: currentTab.value?.name || undefined,
    label: t('trafficAnalysis.repeater.contextMenu.request'),
  })
  dialog.toast.success(t('trafficAnalysis.repeater.messages.compareOpened'))
}

function contextMenuSendToIntruder() {
  hideContextMenu();

  const request = buildCurrentRequestTransfer()
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForIntruder'))
    return
  }

  emit('createAttackWorkspace', request)
  dialog.toast.success(t('trafficAnalysis.repeater.messages.attackWorkspaceCreated'))
}

function contextMenuCopyRequest() {
  hideContextMenu();
  if (!currentTab.value) return;

  const content = contextMenu.value.pane === 'response'
    ? currentTab.value.rawResponse
    : currentTab.value.rawRequest
  if (!content) return;
  
  navigator.clipboard.writeText(content)
    .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.requestCopied')))
    .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
}

function contextMenuCopyCurl() {
  hideContextMenu();
  const curl = buildRepeaterCurlCommand(currentTab.value);
  if (curl) {
    navigator.clipboard.writeText(curl)
      .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.curlCopied')))
      .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
  }
}


function compareCurrentRequestVersions() {
  if (!currentTab.value) return;

  const payload = buildRepeaterRequestVersionComparePayload(currentTab.value, buildRepeaterCompareLabels(t));
  if (!payload) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.noRequestVersionsToCompare'));
    return;
  }

  emit('openCompare', payload);
  dialog.toast.success(t('trafficAnalysis.repeater.messages.compareOpened'));
}

function compareCurrentResponseVersions() {
  if (!currentTab.value) return;

  const payload = buildRepeaterResponseVersionComparePayload(currentTab.value, buildRepeaterCompareLabels(t));
  if (!payload) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.noResponseVersionsToCompare'));
    return;
  }

  emit('openCompare', payload);
  dialog.toast.success(t('trafficAnalysis.repeater.messages.compareOpened'));
}

async function sendRequestToAssistant() {
  if (!currentTab.value) return;
  const trafficData = buildRepeaterAssistantTrafficData(currentTab.value)
  
  // 发送全局事件
  openTrafficAssistantPanel()
  await tauriEmit('traffic:send-to-assistant', { requests: [trafficData], type: 'request' });
  dialog.toast.success(t('trafficAnalysis.repeater.messages.sentToAssistant', {
    type: t('trafficAnalysis.repeater.types.request'),
  }));
}

function contextMenuSendRequestToAssistant() {
  hideContextMenu();
  sendRequestToAssistant();
}

// Resize
function startResize(event: MouseEvent) {
  isResizing = true;
  startX = event.clientX;
  startY = event.clientY;
  startWidth = leftPanelWidth.value;
  startHeight = topPanelHeight.value;
  
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
  document.body.style.cursor = layoutMode.value === 'horizontal' ? 'col-resize' : 'row-resize';
  document.body.style.userSelect = 'none';
}

function handleResize(event: MouseEvent) {
  if (!isResizing) return;
  
  if (layoutMode.value === 'horizontal') {
    const diff = event.clientX - startX;
    leftPanelWidth.value = Math.max(300, Math.min(startWidth + diff, window.innerWidth - 400));
  } else {
    const diff = event.clientY - startY;
    topPanelHeight.value = Math.max(200, Math.min(startHeight + diff, window.innerHeight - 300));
  }
}

function stopResize() {
  isResizing = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  localStorage.setItem(REPEATER_STORAGE_KEY_LEFT_WIDTH, String(leftPanelWidth.value));
  localStorage.setItem(REPEATER_STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value));
}

// Expose
function addRequestFromHistory(request: HttpExchangeRequest) {
  openPreviewRequest(request)
}

defineExpose({
  openPreviewRequest,
  openDraftFromWorkbench: openDraftInRepeater,
});

// Watchers
watch(() => props.initialRequest, (newRequest) => {
  if (newRequest) {
    addRequestFromHistory(newRequest);
  }
}, { immediate: true });

watch(() => props.initialDraftId, (draftId) => {
  if (draftId) {
    openDraftInRepeater(draftId)
  }
}, { immediate: true })

watch(
  () => workbenchState.drafts.drafts.value.map(draft => draft.id).join('|'),
  openAllDraftsInRepeater,
  { immediate: true },
)

watch(
  () => workbenchState.replay.replayRuns.value
    .map(run => [
      run.id,
      run.draftId,
      run.state,
      run.updatedAt,
      run.response?.bodyText?.length ?? 0,
      run.response?.rawText?.length ?? 0,
    ].join(':'))
    .join('|'),
  restoreMissingDraftTabResponsesFromReplayRuns,
  { immediate: true },
)

watch(() => workbenchState.drafts.activeDraftId.value, (draftId) => {
  if (draftId) {
    if (currentTab.value?.draftId === draftId) {
      return
    }
    openDraftInRepeater(draftId)
  }
}, { immediate: true })

watch(layoutMode, (newMode) => {
  localStorage.setItem(REPEATER_STORAGE_KEY_LAYOUT, newMode);
});

watch(
  () => currentTab.value?.id,
  () => {
    syncCurrentTabBackToDraft()
  },
)

watch(
  () => [
    currentTab.value?.id,
    currentTab.value?.mode,
    currentTab.value?.draftId,
    currentTab.value?.sourceRequestId,
  ] as const,
  () => {
    emitActiveTabModeChanged()
  },
  { immediate: true },
)

watch(
  () => tabs.value.map(tab => `${tab.id}:${tab.userEdited ? '1' : '0'}`).join('|'),
  () => {
    emitTabStatsChanged()
  },
  { immediate: true },
)

watch(
  () => [
    currentTab.value?.id,
    currentTab.value?.rawRequest,
    currentTab.value?.targetHost,
    currentTab.value?.targetPort,
    currentTab.value?.useTls,
    currentTab.value?.overrideSni,
    currentTab.value?.sniHost,
    currentTab.value?.requestTab,
  ] as const,
  () => {
    syncCurrentTabBackToDraft()
  },
)

// 监听 rawRequest 变化，自动检测 Host（使用防抖避免频繁触发）
watch(() => currentTab.value?.rawRequest, (newRequest, oldRequest) => {
  if (newRequest && currentTab.value && newRequest !== oldRequest) {
    if (suppressedPreviewPromotionCount > 0) {
      return
    }
    if (currentTab.value.mode === 'preview') {
      ensureDraftForTab(currentTab.value)
    }
    if (currentTab.value.requestTab !== 'pretty') {
      currentTab.value.prettyRequest = formatRepeaterPrettyRequest(newRequest)
    }
    currentTab.value.userEdited = true

    // 清除之前的定时器
    if (hostDetectionTimer !== null) {
      clearTimeout(hostDetectionTimer);
    }
    
    // 使用防抖，500ms 后执行检测
    hostDetectionTimer = window.setTimeout(() => {
      if (currentTab.value) {
        syncRepeaterTabTargetFromRequest(currentTab.value, newRequest);
      }
      hostDetectionTimer = null;
    }, 500);
  }
}, { deep: false });

// 键盘快捷键处理
function shouldHandleRepeaterShortcut(event: KeyboardEvent) {
  if (!repeaterRoot.value || repeaterRoot.value.offsetParent === null) return false
  if (!currentTab.value || showTargetDialog.value) return false
  if (!(event.metaKey || event.ctrlKey) || event.altKey) return false

  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement) {
    if (activeElement !== document.body && !repeaterRoot.value.contains(activeElement)) {
      return false
    }

    if (isEditableKeyboardTarget(activeElement) && !repeaterRoot.value.contains(activeElement)) {
      return false
    }
  }

  const targetNode = event.target instanceof Node ? event.target : null
  if (targetNode && !repeaterRoot.value.contains(targetNode)) {
    return false
  }

  return true
}

function handleKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.repeat) return;
  if (!shouldHandleRepeaterShortcut(event)) return;

  // Cmd/Ctrl + R 发送当前请求到新标签（Send to Repeater）
  const normalizedKey = event.key.toLowerCase()

  if (normalizedKey === 'r' && currentTab.value.rawRequest.trim()) {
    event.preventDefault();
    event.stopPropagation();
    contextMenuSendToNewTab();
    return
  }

  if (normalizedKey === 'i' && currentTab.value.rawRequest.trim()) {
    event.preventDefault()
    event.stopPropagation()
    contextMenuSendToIntruder()
  }
}

function saveRepeaterScrollState() {
  if (!repeaterRoot.value) return;
  repeaterScrollState = {
    top: repeaterRoot.value.scrollTop,
    left: repeaterRoot.value.scrollLeft,
  };
}

function restoreRepeaterScrollState() {
  if (!repeaterRoot.value) return;
  repeaterRoot.value.scrollTop = repeaterScrollState.top;
  repeaterRoot.value.scrollLeft = repeaterScrollState.left;
}

// Lifecycle
onMounted(() => {
  clearRepeaterTabsStorage()

  if (
    tabs.value.length === 0
    && !props.initialRequest
    && !props.initialDraftId
    && !workbenchState.drafts.activeDraftId.value
  ) {
    addTab();
  }
  
  // 添加键盘快捷键监听
  document.addEventListener('keydown', handleKeydown);
  repeaterRoot.value?.addEventListener('scroll', saveRepeaterScrollState, { passive: true });
});

onDeactivated(() => {
  saveRepeaterScrollState();
});

onActivated(() => {
  void nextTick(() => {
    requestAnimationFrame(() => {
      restoreRepeaterScrollState();
    });
  });
});

onUnmounted(() => {
  saveRepeaterScrollState();
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.removeEventListener('keydown', handleKeydown);
  document.removeEventListener('click', hideTabContextMenu)
  repeaterRoot.value?.removeEventListener('scroll', saveRepeaterScrollState);
  
  // 清理所有取消控制器
  abortControllers.clear();
  
  // 清理 host 检测定时器
  if (hostDetectionTimer !== null) {
    clearTimeout(hostDetectionTimer);
    hostDetectionTimer = null;
  }
  
  clearRepeaterTabsStorage()
});
</script>

<style scoped>
.request-panel,
.response-panel {
  min-width: 300px;
  min-height: 200px;
}

.repeater-pane-header-controls {
  max-width: 100%;
  overflow-x: auto;
  scrollbar-width: thin;
}

pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
