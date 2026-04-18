<template>
  <div class="flex flex-col h-full bg-base-200 overflow-hidden" @contextmenu.prevent>
    <!-- 证书错误提示弹窗 -->
    <AppDialog ref="certErrorDialog" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4 flex items-center gap-2">
          <i class="fas fa-exclamation-triangle text-warning"></i>
          {{ $t('trafficAnalysis.history.certificateError.title') }}
        </h3>
        
        <div class="space-y-4">
          <div class="alert alert-warning">
            <i class="fas fa-info-circle"></i>
            <span>{{ $t('trafficAnalysis.history.certificateError.message') }}</span>
          </div>
          
          <div v-if="certErrorInfo" class="bg-base-200 p-4 rounded-lg">
            <h4 class="font-semibold mb-2 text-sm">{{ $t('trafficAnalysis.history.certificateError.details') }}</h4>
            <div class="text-xs space-y-1 font-mono">
              <div><span class="text-base-content/70">Host:</span> {{ certErrorInfo.host }}</div>
              <div><span class="text-base-content/70">URL:</span> {{ certErrorInfo.url }}</div>
              <div v-if="certErrorInfo.error"><span class="text-base-content/70">Error:</span> {{ certErrorInfo.error }}</div>
            </div>
          </div>
          
          <div class="bg-base-300/50 p-4 rounded-lg">
            <h4 class="font-semibold mb-2 text-sm flex items-center gap-2">
              <i class="fas fa-lightbulb text-info"></i>
              {{ $t('trafficAnalysis.history.certificateError.commonIssues.invalidCN') }}
            </h4>
            <ul class="text-xs space-y-1 list-disc list-inside text-base-content/80">
              <li>{{ $t('trafficAnalysis.history.certificateError.tips.installCA') }}</li>
              <li>{{ $t('trafficAnalysis.history.certificateError.tips.serverCertIssue') }}</li>
            </ul>
          </div>
        </div>
        
        <div class="modal-action justify-between">
          <button class="btn btn-sm btn-ghost" @click="closeCertErrorDialog">
            {{ $t('trafficAnalysis.history.detailsPanel.close') }}
          </button>
          <div class="flex gap-2">
            <button class="btn btn-sm btn-info" @click="checkCAInstallation">
              <i class="fas fa-certificate mr-2"></i>
              {{ $t('trafficAnalysis.history.certificateError.tips.checkCAInstallation') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="closeCertErrorDialog">
              {{ $t('trafficAnalysis.history.certificateError.actions.ignore') }}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.history.detailsPanel.close') }}</button>
      </form>
    </AppDialog>
    
    <!-- 历史列表右键菜单 -->
    <div 
      v-if="contextMenu.visible"
      ref="contextMenuRef"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="historyContextMenuSections"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div class="divider my-1 h-0"></div>
      <TrafficContextSubmenu
        v-if="historyFilterSubmenu"
        :submenu="historyFilterSubmenu"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div class="divider my-1 h-0"></div>
      <button
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="addContextRequestToBasket"
      >
        <i class="fas fa-basket-shopping text-primary"></i>
        加入请求篮子
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2 text-error"
        @click="clearHistoryFromMenu"
      >
        <i class="fas fa-trash"></i>
        {{ $t('trafficAnalysis.history.contextMenu.clearHistory') }}
      </button>
    </div>

    <!-- 请求详情区域右键菜单 -->
    <div 
      v-if="detailContextMenu.visible"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: detailContextMenu.x + 'px', top: detailContextMenu.y + 'px' }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="historyDetailContextMenuSections"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div v-if="detailContextMenu.pane === 'request'" class="divider my-1 h-0"></div>
      <button
        v-if="detailContextMenu.pane === 'request'"
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="addDetailRequestToBasket"
      >
        <i class="fas fa-basket-shopping text-primary"></i>
        加入请求篮子
      </button>
    </div>

    <!-- 筛选器配置弹窗 -->
    <AppDialog ref="filterDialog" class="modal">
      <div class="modal-box max-w-6xl max-h-[90vh]">
        <h3 class="font-bold text-lg mb-4">Configure HTTP Proxy filter</h3>
        
        <!-- Mode Tabs -->
        <div class="tabs tabs-boxed mb-4 bg-base-200">
          <a 
            class="tab"
            :class="{ 'tab-active': filterMode === 'settings' }"
            @click="filterMode = 'settings'"
          >
            Settings mode
          </a>
          <a 
            class="tab"
            :class="{ 'tab-active': filterMode === 'bambda' }"
            @click="filterMode = 'bambda'"
          >
            Bambda mode
          </a>
        </div>

        <!-- Settings Mode Content -->
        <div v-if="filterMode === 'settings'" class="space-y-4 max-h-[60vh] overflow-y-auto pr-2">
          <div class="grid grid-cols-3 gap-4">
            <!-- Filter by request type -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by request type</h4>
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.showOnlyInScope" class="checkbox checkbox-sm" />
                  <span class="text-sm">Show only in-scope items</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.hideWithoutResponse" class="checkbox checkbox-sm" />
                  <span class="text-sm">Hide items without responses</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.showOnlyWithParams" class="checkbox checkbox-sm" />
                  <span class="text-sm">Show only parameterized requests</span>
                </label>
              </div>
            </div>

            <!-- Filter by MIME type -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by MIME type</h4>
              <div class="grid grid-cols-2 gap-x-3 gap-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.html" class="checkbox checkbox-sm" />
                  <span class="text-sm">HTML</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.otherText" class="checkbox checkbox-sm" />
                  <span class="text-sm">Other text</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.script" class="checkbox checkbox-sm" />
                  <span class="text-sm">Script</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.images" class="checkbox checkbox-sm" />
                  <span class="text-sm">Images</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.xml" class="checkbox checkbox-sm" />
                  <span class="text-sm">XML</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.flash" class="checkbox checkbox-sm" />
                  <span class="text-sm">Flash</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.css" class="checkbox checkbox-sm" />
                  <span class="text-sm">CSS</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.otherBinary" class="checkbox checkbox-sm" />
                  <span class="text-sm">Other binary</span>
                </label>
              </div>
            </div>

            <!-- Filter by status code -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by status code</h4>
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s2xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">2xx [success]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s3xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">3xx [redirection]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s4xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">4xx [request error]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s5xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">5xx [server error]</span>
                </label>
              </div>
            </div>
          </div>

          <!-- Second Row -->
          <div class="grid grid-cols-3 gap-4">
            <!-- Filter by search term -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by search term</h4>
              <input 
                type="text" 
                v-model="filterConfig.search.term" 
                placeholder="Search..." 
                class="input input-bordered input-sm w-full mb-3" 
              />
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.regex" class="checkbox checkbox-sm" />
                  <span class="text-sm">Regex</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.caseSensitive" class="checkbox checkbox-sm" />
                  <span class="text-sm">Case sensitive</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.negative" class="checkbox checkbox-sm" />
                  <span class="text-sm">Negative search</span>
                </label>
              </div>
            </div>

            <!-- Filter by file extension -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by file extension</h4>
              <div class="space-y-3">
                <div class="form-control">
                  <label class="flex items-center gap-2 cursor-pointer mb-1">
                    <input type="checkbox" v-model="filterConfig.extension.showOnlyEnabled" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only:</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.extension.showOnly" 
                    placeholder="asp,aspx,jsp,php" 
                    class="input input-bordered input-sm w-full"
                    :disabled="!filterConfig.extension.showOnlyEnabled"
                  />
                </div>
                <div class="form-control">
                  <label class="flex items-center gap-2 cursor-pointer mb-1">
                    <input type="checkbox" v-model="filterConfig.extension.hideEnabled" class="checkbox checkbox-sm" />
                    <span class="text-sm">Hide:</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.extension.hide" 
                    placeholder="js,gif,jpg,png,css,ico,woff,woff2" 
                    class="input input-bordered input-sm w-full"
                    :disabled="!filterConfig.extension.hideEnabled"
                  />
                </div>
              </div>
            </div>

            <!-- Filter by annotation and listener -->
            <div class="space-y-4">
              <!-- Filter by annotation -->
              <div class="border border-base-300 rounded-lg p-3">
                <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by annotation</h4>
                <div class="space-y-2">
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" v-model="filterConfig.annotation.showOnlyWithNotes" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only items with notes</span>
                  </label>
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" v-model="filterConfig.annotation.showOnlyHighlighted" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only highlighted items</span>
                  </label>
                </div>
              </div>

              <!-- Filter by listener -->
              <div class="border border-base-300 rounded-lg p-3">
                <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by listener</h4>
                <div class="form-control">
                  <label class="label py-0 pb-1">
                    <span class="label-text text-sm">Port</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.listener.port" 
                    placeholder="e.g. 8080" 
                    class="input input-bordered input-sm w-full" 
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Bambda Mode Content -->
        <div v-else class="space-y-4">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span class="text-sm">Bambda mode allows you to write custom filter logic using JavaScript-like expressions.</span>
          </div>
          <textarea 
            class="textarea textarea-bordered w-full h-64 font-mono text-sm"
            placeholder="// Write your Bambda filter expression here&#10;// Example: request.url().contains('api') && response.statusCode() == 200"
            v-model="filterConfig.bambdaExpression"
          ></textarea>
          <div class="text-xs text-base-content/70">
            <p class="mb-1">Available variables:</p>
            <ul class="list-disc list-inside ml-2 space-y-0.5">
              <li><code class="bg-base-200 px-1 rounded">request</code> - Access request properties (url, method, headers, body)</li>
              <li><code class="bg-base-200 px-1 rounded">response</code> - Access response properties (statusCode, headers, body)</li>
              <li><code class="bg-base-200 px-1 rounded">annotations</code> - Access item annotations (notes, highlights)</li>
            </ul>
          </div>
        </div>

        <!-- 底部按钮 -->
        <div class="modal-action justify-between mt-6 pt-4 border-t border-base-300">
          <div class="flex gap-2">
            <button class="btn btn-sm btn-ghost" @click="showAllFilters">
              <i class="fas fa-eye mr-1"></i>
              Show all
            </button>
            <button class="btn btn-sm btn-ghost" @click="hideAllFilters">
              <i class="fas fa-eye-slash mr-1"></i>
              Hide all
            </button>
            <button class="btn btn-sm btn-ghost" @click="revertFilterChanges">
              <i class="fas fa-undo mr-1"></i>
              Revert changes
            </button>
          </div>
          <div class="flex gap-2 items-center">
            <button 
              v-if="filterMode === 'bambda'"
              class="btn btn-sm btn-ghost"
              @click="convertToBambda"
              title="Convert current settings to Bambda expression"
            >
              <i class="fas fa-exchange-alt mr-1"></i>
              Convert to Bambda
            </button>
            <button class="btn btn-sm" @click="closeFilterDialog">Cancel</button>
            <button class="btn btn-sm btn-primary" @click="applyFilterConfig">
              <i class="fas fa-check mr-1"></i>
              Apply
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </AppDialog>

    <TrafficContextCandidateDialog
      :open="showContextCandidateDialog"
      :loading="contextCandidateLoading"
      :applying="contextCandidateApplying"
      :previewing="contextCandidatePreviewLoading"
      :result="contextCandidateResult"
      :preview-result="contextCandidatePreviewResult"
      :selected-candidate-ids="selectedCandidateIds"
      :preferred-preview-focus="preferredPreviewFocus"
      @close="showContextCandidateDialog = false"
      @apply="applySelectedContextCandidates"
      @open-evidence-request="openContextCandidateEvidenceRequest"
      @preview="previewSelectedContextCandidates"
      @update:preferred-preview-focus="preferredPreviewFocus = $event"
      @update:selected-candidate-ids="updateSelectedCandidateIds"
    />

    <!-- 可调整大小的上下分割布局 -->
    <div ref="mainContainer" class="flex-1 flex flex-col min-h-0 overflow-hidden">
      <!-- 上半部分：请求历史列表 -->
      <div 
        ref="topPanel"
        class="bg-base-100 border-b border-base-300 overflow-hidden flex flex-col flex-shrink-0"
        :style="{ height: topPanelHeight + 'px' }"
      >
        <ProxyHistoryToolbar
          :protocol-filter="protocolFilter"
          :has-active-filters="hasActiveFilters"
          :filters-enabled="filtersEnabled"
          :filter-summary="filterSummary"
          :is-multi-select-mode="isMultiSelectMode"
          :selected-count="selectedRequests.size"
          :filtered-count="filteredRequests.length"
          :open-filter-dialog="openFilterDialog"
          :toggle-filters-enabled="toggleFiltersEnabled"
          :toggle-multi-select-mode="toggleMultiSelectMode"
          :select-all-visible="selectAllVisible"
          :clear-selection="clearSelection"
          :generate-candidates-from-filtered="generateContextCandidatesFromFiltered"
          :generate-candidates-from-selection="generateContextCandidatesFromSelection"
          :send-selected-to-assistant="sendSelectedToAssistant"
          :send-selected-request-versions-to-comparer="sendSelectedRequestVersionsToComparer"
          :send-selected-response-versions-to-comparer="sendSelectedResponseVersionsToComparer"
          :export-selected-to-file="exportSelectedToFile"
          :exportAsHAR="exportAsHAR"
          :refresh-requests="refreshRequests"
          :clear-history="clearHistory"
          @update:protocol-filter="protocolFilter = $event"
        />

        <!-- 虚拟滚动列表容器 -->
        <div ref="scrollContainer" class="flex-1 overflow-auto min-h-0" @scroll="handleScroll">
          <ProxyHistoryTopContent
            :is-loading="isLoading"
            :is-loading-ws="isLoadingWs"
            :protocol-filter="protocolFilter"
            :ws-connections="wsConnections"
            :expanded-ws-connections="expandedWsConnections"
            :toggle-ws-connection="toggleWsConnection"
            :format-ws-time="formatWsTime"
            :get-ws-active-tab="getWsActiveTab"
            :set-ws-active-tab="setWsActiveTab"
            :get-ws-messages-for-connection="getWsMessagesForConnection"
            :truncate-ws-content="truncateWsContent"
            :filtered-requests="filteredRequests"
            :total-height="totalHeight"
            :header-height="headerHeight"
            :item-height="itemHeight"
            :is-multi-select-mode="isMultiSelectMode"
            :selected-requests="selectedRequests"
            :clear-selection="clearSelection"
            :select-all-visible="selectAllVisible"
            :visible-columns="visibleColumns"
            :sort-state="sortState"
            :toggle-sort="toggleSort"
            :start-resize="startResize"
            :visible-rows="visibleRows"
            :selected-request="selectedRequest"
            :is-request-selected="isRequestSelected"
            :toggle-select-request="toggleSelectRequest"
            :show-certificate-error="showCertificateError"
            :select-request="selectRequest"
            :show-context-menu="showContextMenu"
            :get-method-class="getMethodClass"
            :get-status-class="getStatusClass"
            :get-status-title="getStatusTitle"
            :get-status-text="getStatusText"
            :has-params="hasParams"
          />
        </div>
      </div>

      <!-- 水平分割条 -->
      <div 
        ref="horizontalResizer"
        class="h-1 bg-base-300 cursor-row-resize hover:bg-primary/50 transition-colors flex-shrink-0"
        @mousedown="startHorizontalResize"
      ></div>

      <!-- 下半部分：请求/响应详情 -->
      <div 
        ref="bottomPanel"
        class="bg-base-100 overflow-hidden flex flex-col flex-1 min-h-0"
      >
        <ProxyHistoryDetailsPanel
          :selected-request="selectedRequest"
          :is-loading-selected-request="isSelectedRequestLoading"
          :left-panel-width="leftPanelWidth"
          :request-tab="requestTab"
          :response-tab="responseTab"
          :request-view-mode="requestViewMode"
          :response-view-mode="responseViewMode"
          :context-evidence-pane="selectedRequestEvidence?.pane || 'request'"
          :context-evidence-matched-locations="selectedRequestEvidence?.matchedLocations || []"
          :context-evidence-search-terms="selectedRequestEvidence?.searchTerms || []"
          :show-detail-context-menu="showDetailContextMenu"
          :start-vertical-resize="startVerticalResize"
          @update:request-tab="requestTab = $event"
          @update:response-tab="responseTab = $event"
          @update:request-view-mode="requestViewMode = $event"
          @update:response-view-mode="responseViewMode = $event"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated, nextTick, watch, inject } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit as tauriEmit } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import {
  getTrafficContextExtractionSettings,
  mergeCandidatesIntoTrafficContextExtractionSettings,
  previewTrafficContextExtractionChanges,
  recommendTrafficContextDictionaryCandidates,
  setTrafficContextExtractionSettings,
} from '@/services/trafficContextCandidates'
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import TrafficContextCandidateDialog from './TrafficContextCandidateDialog.vue'
import TrafficContextSubmenu from './TrafficContextSubmenu.vue'
import { buildTrafficContextCandidateId } from './trafficContextCandidateSupport'
import { useTrafficContextCandidatePreferences } from './useTrafficContextCandidatePreferences'
import { getDefaultTrafficMessageViewTab } from './trafficDisplaySettings'
import { buildTrafficRequestActionMenuItems } from './trafficRequestActionMenuSupport'
import { buildTrafficContextSubmenu } from './trafficContextSubmenuSupport'
import { useTrafficSendTargets } from './trafficSendTargets'
import { buildTrafficRequestSendMenuItems } from './trafficSendMenuSupport'
import ProxyHistoryDetailsPanel from './ProxyHistoryDetailsPanel.vue';
import ProxyHistoryTopContent from './ProxyHistoryTopContent.vue';
import ProxyHistoryToolbar from './ProxyHistoryToolbar.vue';
import { useProxyHistoryDerivedList } from './useProxyHistoryDerivedList';
import {
  buildDefaultProxyHistoryFilterConfig,
  buildProxyHistoryFilterCache,
  buildProxyHistoryFilterSummary,
  convertProxyHistoryFilterConfigToBambda,
  getExtension,
  getMimeTypeCategory,
  hasActiveProxyHistoryFilters,
  hasParams,
  hideAllProxyHistoryFilters,
  mergeStoredProxyHistoryFilterConfig,
  showAllProxyHistoryFilters,
} from './proxyHistoryFilterSupport';
import {
  formatBytes,
  formatRequest,
  formatRequestRaw,
  formatResponse,
  formatResponseRaw,
  getColumnValue,
  getMethodClass,
  getResponseBody,
  getStatusClass,
  getStatusText,
  getStatusTitle,
  hasEditedResponse,
  isResponseCompressed,
  stringToHex,
  truncateText,
} from './proxyHistoryFormattingSupport';
import {
  buildProxyHistoryVisibleItems,
  defaultProxyHistoryColumns,
  getProxyHistoryDefaultSortDirection,
  isProxyHistoryDefaultSort,
  loadProxyHistoryColumnsFromStorage,
  loadProxyHistorySortFromStorage,
  PROXY_HISTORY_BUFFER_SIZE,
  PROXY_HISTORY_COLUMNS_STORAGE_KEY,
  PROXY_HISTORY_HEADER_HEIGHT,
  PROXY_HISTORY_ITEM_HEIGHT,
  PROXY_HISTORY_SORT_STORAGE_KEY,
  translateProxyHistoryColumns,
} from './proxyHistoryTableSupport';
import { classifyProxyHistoryRequestIdStep } from './proxyHistoryListStepSupport';
import { useProxyHistoryActions } from './useProxyHistoryActions';
import { useProxyHistoryData } from './useProxyHistoryData';
import {
  canCompareRequestVersions,
  canCompareResponseVersions,
} from './trafficHistoryComparerSupport';
import { buildTrafficRequestContextMenuSections } from './trafficRequestContextMenuSupport';
import type { TrafficComparePayload, TrafficComparerDraftRequestInput } from './transfers';
import type { HttpExchangeRequest } from './http/model'
import { buildHttpExchangeRequestFromHistory } from './proxyHistoryHttpSupport'
import type {
  RecommendTrafficContextDictionaryCandidatesResponse,
  TrafficContextCandidateEvidenceSelection,
  TrafficContextExtractionPreviewResponse,
} from './trafficContextCandidateTypes'
import type {
  Column,
  ProxyHistoryFilterCache,
  ProxyHistoryProtocolFilter,
  ProxyHistoryRequestTab,
  ProxyHistoryResponseTab,
  ProxyHistorySortState,
  ProxyHistoryViewMode,
  ProxyHistoryWsTab,
  ProxyRequest,
  VirtualItem,
  WebSocketConnection,
  WebSocketMessage,
} from './proxyHistoryTypes';

const { t } = useI18n();
const { preferredPreviewFocus } = useTrafficContextCandidatePreferences()

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 定义组件名称，用于 keep-alive
defineOptions({
  name: 'ProxyHistory'
});

// Emit 声明
const emit = defineEmits<{
  (e: 'sendToRepeater', request: HttpExchangeRequest): void
  (e: 'sendToIntruder', request: HttpExchangeRequest): void
  (e: 'sendDraftRequestToComparer', payload: TrafficComparerDraftRequestInput): void
  (e: 'sendToComparer', payload: TrafficComparePayload): void
  (e: 'sendToAssistant', requests: ProxyRequest[]): void
  (e: 'addFilterRule', rule: { matchType: string; condition: string; relationship?: string }): void
  (e: 'addToBasket', payload: { request: HttpExchangeRequest; requestId?: number; title: string; host: string }): void
}>();

// 多选状态
const selectedRequests = ref<Set<number>>(new Set());
const isMultiSelectMode = ref(false);

// 历史列表右键菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  request: null as ProxyRequest | null,
});
const contextMenuRef = ref<HTMLElement | null>(null);
const { enabledTargets } = useTrafficSendTargets()

// 请求详情区域右键菜单状态
const detailContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  pane: 'request' as 'request' | 'response',
});

// 响应式状态
const requests = ref<ProxyRequest[]>([]);
const selectedRequest = ref<ProxyRequest | null>(null);
const selectedRequestEvidence = ref<TrafficContextCandidateEvidenceSelection | null>(null)
const isSelectedRequestLoading = ref(false);
// 协议类型过滤: 'all' | 'http' | 'websocket'
const protocolFilter = ref<ProxyHistoryProtocolFilter>('all');

// WebSocket 状态
const wsConnections = ref<WebSocketConnection[]>([]);
const wsMessages = ref<WebSocketMessage[]>([]);
const selectedWsConnection = ref<WebSocketConnection | null>(null);
const expandedWsConnections = ref<Set<string>>(new Set());
// WebSocket 连接内部 Tab 状态: connectionId -> 'messages' | 'handshake'
const activeWsTabs = ref<Map<string, ProxyHistoryWsTab>>(new Map());
const isLoadingWs = ref(false);
// WebSocket 消息缓存：connectionId -> messages
const wsMessagesCache = ref<Map<string, WebSocketMessage[]>>(new Map());

const showDetailsModal = ref(false);
const showColumnSettings = ref(false);
const isLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);
const showContextCandidateDialog = ref(false)
const contextCandidateLoading = ref(false)
const contextCandidateApplying = ref(false)
const contextCandidateResult = ref<RecommendTrafficContextDictionaryCandidatesResponse | null>(null)
const selectedCandidateIds = ref<string[]>([])

// 面板引用
const mainContainer = ref<HTMLElement | null>(null);
const topPanel = ref<HTMLElement | null>(null);
const bottomPanel = ref<HTMLElement | null>(null);
const horizontalResizer = ref<HTMLElement | null>(null);
const verticalResizer = ref<HTMLElement | null>(null);
let mainContainerResizeObserver: ResizeObserver | null = null;

// 面板尺寸（从 localStorage 恢复或使用默认值）
const STORAGE_KEY_TOP_HEIGHT = 'proxyHistory.topPanelHeight';
const STORAGE_KEY_LEFT_WIDTH = 'proxyHistory.leftPanelWidth';

const topPanelHeight = ref(300); // 上面板高度（会在 onMounted 中初始化）
const leftPanelWidth = ref(parseInt(localStorage.getItem(STORAGE_KEY_LEFT_WIDTH) || '600')); // 默认左面板宽度

// 拖拽状态
const isResizingHorizontal = ref(false);
const isResizingVertical = ref(false);
const resizeStartY = ref(0);
const resizeStartX = ref(0);
const resizeStartTopHeight = ref(0);
const resizeStartLeftWidth = ref(0);

// 分页和性能优化
const maxRequestsInMemory = 1000; // 增加到 1000 条
const initialLoadLimit = 100; // 初次加载 100 条
const batchUpdateThreshold = 10; // 批量更新阈值（增加到 10）
const loadMoreSize = 50; // 每次加载更多的数量
const hasMore = ref(true); // 是否还有更多数据
const isLoadingMore = ref(false); // 是否正在加载更多
// 请求 ID 集合（用于快速去重）
const requestIdSet = ref<Set<number>>(new Set());

// 详情面板的标签页
const requestTab = ref<ProxyHistoryRequestTab>(getDefaultTrafficMessageViewTab());
const responseTab = ref<ProxyHistoryResponseTab>(getDefaultTrafficMessageViewTab());

// Original/Edited 切换（类似 Burp Suite）
const requestViewMode = ref<ProxyHistoryViewMode>('edited');
const responseViewMode = ref<ProxyHistoryViewMode>('edited');

const stats = ref({
  total: 0,
  http: 0,
  https: 0,
  avgResponseTime: 0,
});

// 筛选器弹窗引用
const filterDialog = ref<HTMLDialogElement | null>(null);

// 证书错误弹窗引用
const certErrorDialog = ref<HTMLDialogElement | null>(null);
const certErrorInfo = ref<{host: string; url: string; error?: string} | null>(null);

// Filter mode
const filterMode = ref<'settings' | 'bambda'>('settings');

// 默认筛选器配置
const defaultFilterConfig = buildDefaultProxyHistoryFilterConfig;

// 筛选器配置（可编辑）
const filterConfig = ref(defaultFilterConfig());
// 已应用的筛选器配置
const appliedFilterConfig = ref(defaultFilterConfig());
// 备份配置（用于 revert）
const backupFilterConfig = ref(defaultFilterConfig());
const filtersEnabled = ref(localStorage.getItem('proxyHistory.filtersEnabled') !== 'false');

// 旧的 filters 保留用于兼容
const filters = ref({
  protocol: '',
  method: '',
  statusCode: '',
  search: '',
});

// 列配置（从 localStorage 恢复或使用默认值）
const columns = ref<Column[]>(loadProxyHistoryColumnsFromStorage());
const sortState = ref<ProxyHistorySortState>(loadProxyHistorySortFromStorage());

// 列调整相关
const resizingColumn = ref<string | null>(null);
const columnResizeStartX = ref(0);
const resizeStartWidth = ref(0);

// 虚拟滚动相关
const itemHeight = PROXY_HISTORY_ITEM_HEIGHT;
const headerHeight = PROXY_HISTORY_HEADER_HEIGHT;
const scrollTop = ref(0);
const containerHeight = ref(600);
const bufferSize = PROXY_HISTORY_BUFFER_SIZE;
const PREFETCH_VIEWPORT_MULTIPLIER = 2;
let scrollFrameId: number | null = null;
let pendingScrollTop = 0;
let prefetchTimer: number | null = null;
let isPrefetching = false;
const AUTO_FOLLOW_TOP_THRESHOLD = itemHeight;
const savedScrollTop = ref(0)

const effectiveFilterConfig = computed(() =>
  filtersEnabled.value ? appliedFilterConfig.value : showAllProxyHistoryFilters(appliedFilterConfig.value),
);
const filterCache = computed<ProxyHistoryFilterCache>(() => buildProxyHistoryFilterCache(effectiveFilterConfig.value));
const shouldBypassFrontendFilters = computed(() =>
  !filtersEnabled.value || !hasActiveProxyHistoryFilters(appliedFilterConfig.value),
)
const hasActiveFilters = computed(() => hasActiveProxyHistoryFilters(appliedFilterConfig.value));
const filterSummary = computed(() =>
  filtersEnabled.value && hasActiveFilters.value
    ? buildProxyHistoryFilterSummary(appliedFilterConfig.value)
    : t('trafficAnalysis.history.filterBar.showAllContent'),
);
const translatedColumns = computed(() => translateProxyHistoryColumns(columns.value, t));
const visibleColumns = computed(() => translatedColumns.value.filter((col) => col.visible));
const {
  filteredRequests,
  sortedRequests,
} = useProxyHistoryDerivedList({
  requests,
  shouldBypassFrontendFilters,
  effectiveFilterConfig,
  filterCache,
  sortState,
})
const totalHeight = computed(() => sortedRequests.value.length * itemHeight + headerHeight);
const visibleItems = computed((): VirtualItem[] =>
  buildProxyHistoryVisibleItems(sortedRequests.value, scrollTop.value, containerHeight.value),
);
const sortedRequestIds = computed(() => sortedRequests.value.map((request) => request.id))
const visibleRows = computed(() =>
  visibleItems.value.map((item) => {
    const cellValues: Record<string, string> = {}
    visibleColumns.value.forEach((column) => {
      if (['method', 'status', 'params', 'tls'].includes(column.id)) {
        return
      }
      cellValues[column.id] = getColumnValue(item.data, column.id)
    })
    return {
      ...item,
      cellValues,
    }
  }),
)
const {
  cleanupDataRuntime,
  fetchRequestDetails,
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
} = useProxyHistoryData({
  requests,
  isLoading,
  hasMore,
  isLoadingMore,
  requestIdSet,
  stats,
  wsConnections,
  wsMessages,
  expandedWsConnections,
  activeWsTabs,
  isLoadingWs,
  wsMessagesCache,
  initialLoadLimit,
  loadMoreSize,
  maxRequestsInMemory,
  batchUpdateThreshold,
  t,
});
const {
  addFilterToDomain,
  addFilterToExtension,
  addFilterToMethod,
  addFilterToUrl,
  cleanupActionRuntime,
  clearHistory,
  clearHistoryFromMenu,
  clearSelection,
  compareRequestVersions,
  compareResponseVersions,
  copyAsCurl,
  copyRequest,
  copyUrl,
  detailCompareRequestVersions,
  detailCompareResponseVersions,
  detailCopyAsCurl,
  detailCopyRequest,
  detailCopyUrl,
  detailOpenInBrowser,
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
  sendRequestToAssistantFromMenu,
  sendSelectedRequestVersionsToComparer,
  sendSelectedResponseVersionsToComparer,
  sendSelectedToAssistant,
  sendToComparer,
  sendToIntruder,
  sendToRepeater,
  showContextMenu,
  showDetailContextMenu,
  toggleMultiSelectMode,
  toggleSelectRequest,
} = useProxyHistoryActions({
  contextMenu,
  detailContextMenu,
  selectedRequest,
  selectedRequests,
  isMultiSelectMode,
  filteredRequests,
  requests,
  stats,
  requestTab,
  responseTab,
  requestViewMode,
  responseViewMode,
  topPanelHeight,
  mainContainer,
  emitSendToRepeater: (request) => emit('sendToRepeater', request),
  emitSendToIntruder: (request) => emit('sendToIntruder', request),
  emitSendDraftRequestToComparer: (payload) => emit('sendDraftRequestToComparer', payload),
  emitSendToComparer: (payload) => emit('sendToComparer', payload),
  emitSendToAssistant: (requests) => emit('sendToAssistant', requests),
  emitAddFilterRule: (rule) => emit('addFilterRule', rule),
  fetchRequestDetails,
  updateStats,
  t,
});

const canCompareRequestFromContext = computed(() => canCompareRequestVersions(contextMenu.value.request))
const canCompareResponseFromContext = computed(() => canCompareResponseVersions(contextMenu.value.request))
const canCompareRequestFromDetail = computed(() => canCompareRequestVersions(selectedRequest.value))
const canCompareResponseFromDetail = computed(() => canCompareResponseVersions(selectedRequest.value))
const contextRequestSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['repeater', 'comparer', 'intruder'],
    actions: {
      repeater: sendToRepeater,
      comparer: sendToComparer,
      intruder: sendToIntruder,
    },
  }),
)
const detailRequestSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['repeater', 'comparer', 'intruder'],
    actions: {
      repeater: detailSendToRepeater,
      comparer: detailSendToComparer,
      intruder: detailSendToIntruder,
    },
  }),
)
const contextRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl,
      copyRequest,
      copyAsCurl,
      openInBrowser,
    },
  }),
)
const detailRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl: detailCopyUrl,
      copyRequest: detailCopyRequest,
      copyAsCurl: detailCopyAsCurl,
      openInBrowser: detailOpenInBrowser,
    },
  }),
)
const historyContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: contextRequestSendMenuItems.value,
    compareItems: [
      canCompareRequestFromContext.value
        ? {
            key: 'compareRequestVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'sendToComparer',
            onClick: compareRequestVersions,
          }
        : null,
      canCompareResponseFromContext.value
        ? {
            key: 'compareResponseVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'sendToComparer',
            onClick: compareResponseVersions,
          }
        : null,
    ],
    requestItems: contextRequestActionMenuItems.value,
    assistantItems: [
      {
        key: 'sendToAssistant',
        iconClass: 'fas fa-upload text-accent',
        labelKey: 'sendToAssistant',
        onClick: sendRequestToAssistantFromMenu,
      },
    ],
  }),
)
const historyDetailContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: detailContextMenu.value.pane === 'request'
      ? detailRequestSendMenuItems.value
      : detailRequestSendMenuItems.value.filter((item) => item.key === 'comparer'),
    compareItems: [
      detailContextMenu.value.pane === 'request' && canCompareRequestFromDetail.value
        ? {
            key: 'detailCompareRequestVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'sendToComparer',
            onClick: detailCompareRequestVersions,
          }
        : null,
      detailContextMenu.value.pane === 'response' && canCompareResponseFromDetail.value
        ? {
            key: 'detailCompareResponseVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'sendToComparer',
            onClick: detailCompareResponseVersions,
          }
        : null,
    ],
    requestItems: detailContextMenu.value.pane === 'request' ? detailRequestActionMenuItems.value : [],
    assistantItems: detailContextMenu.value.pane === 'request'
      ? [
          {
            key: 'detailSendToAssistant',
            iconClass: 'fas fa-upload text-accent',
            labelKey: 'sendToAssistant',
            onClick: detailSendRequestToAssistant,
          },
        ]
      : [],
  }),
)
const historyFilterSubmenu = computed(() =>
  buildTrafficContextSubmenu({
    key: 'history-filter',
    triggerLabelKey: 'addToFilter',
    triggerIconClass: 'fas fa-filter text-primary',
    items: [
      {
        key: 'filterByDomain',
        iconClass: 'fas fa-globe text-xs',
        labelKey: 'filterByDomain',
        onClick: addFilterToDomain,
      },
      {
        key: 'filterByUrl',
        iconClass: 'fas fa-link text-xs',
        labelKey: 'filterByUrl',
        onClick: addFilterToUrl,
      },
      {
        key: 'filterByMethod',
        iconClass: 'fas fa-code text-xs',
        labelKey: 'filterByMethod',
        onClick: addFilterToMethod,
      },
      {
        key: 'filterByExtension',
        iconClass: 'fas fa-file text-xs',
        labelKey: 'filterByExtension',
        onClick: addFilterToExtension,
      },
    ],
  }),
)
const selectedContextCandidates = computed(() => {
  const selectedIds = new Set(selectedCandidateIds.value)
  return (contextCandidateResult.value?.candidates || []).filter(candidate =>
    selectedIds.has(buildTrafficContextCandidateId(candidate)),
  )
})
const contextCandidateRequestIds = ref<number[]>([])
const contextCandidatePreviewLoading = ref(false)
const contextCandidatePreviewResult = ref<TrafficContextExtractionPreviewResponse | null>(null)

// 方法
async function generateContextCandidatesFromFiltered() {
  await generateContextCandidatesFromRequestIds(filteredRequests.value.map(request => request.id))
}

async function generateContextCandidatesFromSelection() {
  await generateContextCandidatesFromRequestIds([...selectedRequests.value])
}

async function generateContextCandidatesFromRequestIds(requestIds: number[]) {
  const dedupedRequestIds = [...new Set(requestIds)].slice(0, 300)
  if (dedupedRequestIds.length === 0) {
    dialog.toast.warning('当前没有可用于生成候选的历史记录')
    return
  }
  if (dedupedRequestIds.length < requestIds.length) {
    dialog.toast.info('首版候选生成最多分析前 300 条请求，请先缩小范围再重试')
  }

  showContextCandidateDialog.value = true
  contextCandidateLoading.value = true
  contextCandidateResult.value = null
  contextCandidatePreviewResult.value = null
  contextCandidateRequestIds.value = dedupedRequestIds
  selectedCandidateIds.value = []

  try {
    const result = await recommendTrafficContextDictionaryCandidates({
      requestIds: dedupedRequestIds,
      maxCandidatesPerCategory: 12,
    })
    contextCandidateResult.value = result
    selectedCandidateIds.value = result.candidates
      .filter(candidate => candidate.confidence === 'high' && !candidate.alreadyCoveredBy)
      .map(buildTrafficContextCandidateId)
  } catch (error) {
    console.error('Failed to recommend traffic context dictionary candidates:', error)
    dialog.toast.error(`生成词典候选失败: ${String(error)}`)
    showContextCandidateDialog.value = false
  } finally {
    contextCandidateLoading.value = false
  }
}

async function applySelectedContextCandidates() {
  if (selectedContextCandidates.value.length === 0) {
    return
  }

  contextCandidateApplying.value = true
  try {
    const latestSettings = await getTrafficContextExtractionSettings()
    const nextSettings = mergeCandidatesIntoTrafficContextExtractionSettings(
      latestSettings,
      selectedContextCandidates.value,
    )
    await setTrafficContextExtractionSettings(nextSettings)
    dialog.toast.success(`已把 ${selectedContextCandidates.value.length} 项候选合并到上下文词典`)
    showContextCandidateDialog.value = false
    selectedCandidateIds.value = []
  } catch (error) {
    console.error('Failed to apply traffic context candidates:', error)
    dialog.toast.error(`应用候选失败: ${String(error)}`)
  } finally {
    contextCandidateApplying.value = false
  }
}

async function previewSelectedContextCandidates() {
  if (selectedContextCandidates.value.length === 0 || contextCandidateRequestIds.value.length === 0) {
    return
  }

  contextCandidatePreviewLoading.value = true
  try {
    const currentSettings = await getTrafficContextExtractionSettings()
    const previewSettings = mergeCandidatesIntoTrafficContextExtractionSettings(
      currentSettings,
      selectedContextCandidates.value,
    )
    contextCandidatePreviewResult.value = await previewTrafficContextExtractionChanges({
      requestIds: contextCandidateRequestIds.value,
      currentSettings,
      previewSettings,
      sampleLimit: 6,
    })
  } catch (error) {
    console.error('Failed to preview traffic context candidate changes:', error)
    dialog.toast.error(`预览命中变化失败: ${String(error)}`)
  } finally {
    contextCandidatePreviewLoading.value = false
  }
}

async function openRequestById(
  requestId: number,
  matchedLocations: string[] = [],
  pane: 'request' | 'response' = 'request',
  searchTerms: string[] = [],
) {
  if (!Number.isFinite(requestId)) {
    return
  }

  protocolFilter.value = 'http'
  selectedRequestEvidence.value = {
    requestId,
    pane,
    matchedLocations: [...matchedLocations],
    searchTerms: [...searchTerms],
  }
  let nextRequest = requests.value.find(request => request.id === requestId) || null

  if (!nextRequest || nextRequest.has_full_details === false) {
    nextRequest = await fetchRequestDetails(requestId) || nextRequest
  }

  if (!nextRequest) {
    dialog.toast.warning(`未找到历史请求 #${requestId}`)
    return
  }

  selectRequest(nextRequest)
  await nextTick()
  scrollRequestRowIntoView(requestId)
}

async function openContextCandidateEvidenceRequest(payload: TrafficContextCandidateEvidenceSelection) {
  await openRequestById(
    payload.requestId,
    payload.matchedLocations,
    payload.pane || 'request',
    payload.searchTerms || [],
  )
}

async function addContextRequestToBasket() {
  hideContextMenu()
  const request = contextMenu.value.request
  if (!request) {
    return
  }

  const detailed = await fetchRequestDetails(request.id) || request
  emit('addToBasket', {
    request: buildHttpExchangeRequestFromHistory(detailed),
    requestId: detailed.id,
    title: detailed.url,
    host: detailed.host || '',
  })
}

async function addDetailRequestToBasket() {
  hideDetailContextMenu()
  if (!selectedRequest.value) {
    return
  }

  const detailed = await fetchRequestDetails(selectedRequest.value.id) || selectedRequest.value
  emit('addToBasket', {
    request: buildHttpExchangeRequestFromHistory(detailed),
    requestId: detailed.id,
    title: detailed.url,
    host: detailed.host || '',
  })
}

function updateSelectedCandidateIds(nextValue: string[]) {
  selectedCandidateIds.value = nextValue
  contextCandidatePreviewResult.value = null
}

function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  pendingScrollTop = target.scrollTop;

  if (scrollFrameId !== null) {
    return;
  }

  scrollFrameId = window.requestAnimationFrame(() => {
    scrollTop.value = pendingScrollTop;
    scrollFrameId = null;
    schedulePrefetchCheck();
  });
}

function isEditableKeyboardTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }

  if (target.closest('input, textarea, select, [contenteditable="true"]')) {
    return true
  }

  return Boolean(target.closest('.editor-search-bar, .cm-textfield'))
}

function shouldHandleHistoryArrowNavigation(event: KeyboardEvent) {
  if (!mainContainer.value || mainContainer.value.offsetParent === null) {
    return false
  }

  if (protocolFilter.value === 'websocket' || sortedRequests.value.length === 0) {
    return false
  }

  if (contextMenu.value.visible || detailContextMenu.value.visible) {
    return false
  }

  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
    return false
  }

  if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') {
    return false
  }

  if (isEditableKeyboardTarget(event.target)) {
    return false
  }

  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement) {
    if (isEditableKeyboardTarget(activeElement)) {
      return false
    }

    if (activeElement !== document.body && !mainContainer.value.contains(activeElement)) {
      return false
    }
  }

  return true
}

function scrollRequestRowIntoView(requestId: number) {
  if (!scrollContainer.value) {
    return
  }

  const rowIndex = sortedRequests.value.findIndex((request) => request.id === requestId)
  if (rowIndex < 0) {
    return
  }

  const rowTop = rowIndex * itemHeight + headerHeight
  const rowBottom = rowTop + itemHeight
  const currentScrollTop = scrollContainer.value.scrollTop
  const visibleTop = currentScrollTop + headerHeight
  const visibleBottom = currentScrollTop + scrollContainer.value.clientHeight
  let nextScrollTop: number | null = null

  if (rowTop < visibleTop) {
    nextScrollTop = Math.max(0, rowTop - headerHeight)
  } else if (rowBottom > visibleBottom) {
    nextScrollTop = Math.max(0, rowBottom - scrollContainer.value.clientHeight)
  }

  if (nextScrollTop === null) {
    return
  }

  scrollContainer.value.scrollTop = nextScrollTop
  scrollTop.value = nextScrollTop
  schedulePrefetchCheck()
}

function navigateHistorySelection(direction: -1 | 1) {
  const navigableRequests = sortedRequests.value.filter((request) => request.status_code !== 0)
  if (navigableRequests.length === 0) {
    return false
  }

  const currentIndex = selectedRequest.value
    ? navigableRequests.findIndex((request) => request.id === selectedRequest.value?.id)
    : -1

  const nextIndex = currentIndex < 0
    ? (direction > 0 ? 0 : navigableRequests.length - 1)
    : Math.min(navigableRequests.length - 1, Math.max(0, currentIndex + direction))

  const nextRequest = navigableRequests[nextIndex]
  if (!nextRequest) {
    return false
  }

  if (selectedRequest.value?.id !== nextRequest.id) {
    selectRequest(nextRequest)
  }

  scrollRequestRowIntoView(nextRequest.id)
  return true
}

function schedulePrefetchCheck() {
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer);
  }

  prefetchTimer = window.setTimeout(() => {
    prefetchTimer = null;
    void ensurePrefetchBuffer();
  }, 60);
}

function preserveScrollAnchorOnPrependedRows(previousIds: number[], nextIds: number[]) {
  if (!scrollContainer.value || protocolFilter.value === 'websocket') {
    return
  }

  if (!isProxyHistoryDefaultSort(sortState.value)) {
    return
  }

  const currentScrollTop = scrollContainer.value.scrollTop
  if (currentScrollTop <= AUTO_FOLLOW_TOP_THRESHOLD) {
    return
  }

  const step = classifyProxyHistoryRequestIdStep(previousIds, nextIds)
  if (step.type !== 'prepend' || step.addedFrontCount <= 0) {
    return
  }

  const nextScrollTop = currentScrollTop + step.addedFrontCount * itemHeight
  scrollContainer.value.scrollTop = nextScrollTop
  scrollTop.value = nextScrollTop
}

async function ensurePrefetchBuffer() {
  if (
    isPrefetching ||
    protocolFilter.value === 'websocket' ||
    isLoading.value ||
    isLoadingMore.value ||
    !hasMore.value
  ) {
    return;
  }

  const visibleCount = Math.max(1, Math.ceil(containerHeight.value / itemHeight));
  const desiredRemainingRows = Math.max(loadMoreSize, visibleCount * PREFETCH_VIEWPORT_MULTIPLIER);
  const currentLastVisibleIndex = Math.ceil((scrollTop.value + containerHeight.value) / itemHeight);
  const remainingRows = sortedRequests.value.length - currentLastVisibleIndex;

  if (remainingRows > desiredRemainingRows) {
    return;
  }

  isPrefetching = true;
  try {
    for (let attempts = 0; attempts < 3; attempts += 1) {
      if (!hasMore.value || isLoadingMore.value) {
        break;
      }

      const loadedCount = await loadMoreRequests();
      if (loadedCount <= 0) {
        break;
      }

      await nextTick();

      const nextLastVisibleIndex = Math.ceil((scrollTop.value + containerHeight.value) / itemHeight);
      const nextRemainingRows = sortedRequests.value.length - nextLastVisibleIndex;
      if (nextRemainingRows > desiredRemainingRows) {
        break;
      }
    }
  } finally {
    isPrefetching = false;
  }
}

// 更新容器高度
function updateContainerHeight() {
  if (scrollContainer.value) {
    containerHeight.value = scrollContainer.value.clientHeight;
    schedulePrefetchCheck();
  }
}

function syncVirtualViewport() {
  if (!scrollContainer.value) {
    return
  }

  containerHeight.value = scrollContainer.value.clientHeight
  scrollTop.value = scrollContainer.value.scrollTop
  schedulePrefetchCheck()
}

async function refreshVirtualLayout() {
  await nextTick()

  window.requestAnimationFrame(() => {
    syncVirtualViewport()
  })
}

function captureScrollPosition() {
  if (!scrollContainer.value) {
    return
  }

  savedScrollTop.value = scrollContainer.value.scrollTop
}

async function ensureRowsForScrollPosition(targetScrollTop: number) {
  if (protocolFilter.value === 'websocket' || targetScrollTop <= 0) {
    return
  }

  const viewportHeight = Math.max(containerHeight.value, scrollContainer.value?.clientHeight || 0, 1)
  const requiredRowCount = Math.ceil((targetScrollTop + viewportHeight) / itemHeight) + bufferSize
  let attempts = 0

  while (sortedRequests.value.length < requiredRowCount && hasMore.value && attempts < 20) {
    const loadedCount = await loadMoreRequests()
    if (loadedCount <= 0) {
      break
    }
    attempts += 1
    await nextTick()
  }
}

async function restoreSavedScrollPosition() {
  if (!scrollContainer.value) {
    return
  }

  await nextTick()
  syncVirtualViewport()
  await ensureRowsForScrollPosition(savedScrollTop.value)

  if (!scrollContainer.value) {
    return
  }

  const maxScrollTop = Math.max(0, scrollContainer.value.scrollHeight - scrollContainer.value.clientHeight)
  const nextScrollTop = Math.min(savedScrollTop.value, maxScrollTop)
  scrollContainer.value.scrollTop = nextScrollTop
  syncVirtualViewport()
}

// 使用 ResizeObserver 监听容器大小变化
let resizeObserver: ResizeObserver | null = null;

function setupResizeObserver() {
  if (scrollContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      updateContainerHeight();
    });
    resizeObserver.observe(scrollContainer.value);
  }
}

// 列调整相关方法
function startResize(columnId: string, event: MouseEvent) {
  event.preventDefault();
  resizingColumn.value = columnId;
  columnResizeStartX.value = event.clientX;
  const column = columns.value.find(col => col.id === columnId);
  if (column) {
    resizeStartWidth.value = column.width;
  }
  
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
}

function handleResize(event: MouseEvent) {
  if (!resizingColumn.value) return;
  
  const column = columns.value.find(col => col.id === resizingColumn.value);
  if (!column) return;
  
  const diff = event.clientX - columnResizeStartX.value;
  const newWidth = Math.max(column.minWidth, resizeStartWidth.value + diff);
  column.width = newWidth;
}

function toggleSort(columnId: string) {
  if (sortState.value.columnId === columnId) {
    sortState.value = {
      columnId,
      direction: sortState.value.direction === 'asc' ? 'desc' : 'asc',
    };
  } else {
    sortState.value = {
      columnId,
      direction: getProxyHistoryDefaultSortDirection(columnId),
    };
  }

  localStorage.setItem(PROXY_HISTORY_SORT_STORAGE_KEY, JSON.stringify(sortState.value));
  scrollTop.value = 0;
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0;
  }
}

function stopResize() {
  resizingColumn.value = null;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  
  // 保存列配置到 localStorage
  localStorage.setItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY, JSON.stringify(columns.value));
}

// 水平分割条调整（上下面板）
function startHorizontalResize(event: MouseEvent) {
  event.preventDefault();
  isResizingHorizontal.value = true;
  resizeStartY.value = event.clientY;
  resizeStartTopHeight.value = topPanelHeight.value;
  
  document.addEventListener('mousemove', handleHorizontalResize);
  document.addEventListener('mouseup', stopHorizontalResize);
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';
}

function handleHorizontalResize(event: MouseEvent) {
  if (!isResizingHorizontal.value) return;
  
  const containerHeight = mainContainer.value?.clientHeight || 600;
  const diffY = event.clientY - resizeStartY.value;
  const newTopHeight = Math.max(150, Math.min(resizeStartTopHeight.value + diffY, containerHeight - 200));
  topPanelHeight.value = newTopHeight;
}

function stopHorizontalResize() {
  isResizingHorizontal.value = false;
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  // 保存尺寸到 localStorage
  localStorage.setItem(STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value));
}

// 垂直分割条调整（左右面板）
function startVerticalResize(event: MouseEvent) {
  event.preventDefault();
  isResizingVertical.value = true;
  resizeStartX.value = event.clientX;
  resizeStartLeftWidth.value = leftPanelWidth.value;
  
  document.addEventListener('mousemove', handleVerticalResize);
  document.addEventListener('mouseup', stopVerticalResize);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function handleVerticalResize(event: MouseEvent) {
  if (!isResizingVertical.value) return;
  
  const diffX = event.clientX - resizeStartX.value;
  const containerWidth = bottomPanel.value?.clientWidth || 1200;
  const newLeftWidth = Math.max(300, Math.min(resizeStartLeftWidth.value + diffX, containerWidth - 300));
  leftPanelWidth.value = newLeftWidth;
}

function stopVerticalResize() {
  isResizingVertical.value = false;
  document.removeEventListener('mousemove', handleVerticalResize);
  document.removeEventListener('mouseup', stopVerticalResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  // 保存尺寸到 localStorage
  localStorage.setItem(STORAGE_KEY_LEFT_WIDTH, String(leftPanelWidth.value));
}

function toggleColumn(columnId: string) {
  const column = columns.value.find(col => col.id === columnId);
  if (column) {
    column.visible = !column.visible;
    // 保存列配置到 localStorage
    localStorage.setItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY, JSON.stringify(columns.value));
  }
}

function resetColumns() {
  columns.value = [...defaultProxyHistoryColumns];
  // 清除保存的配置
  localStorage.removeItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY);
}

// 显示证书错误弹窗
function showCertificateError(request: ProxyRequest) {
  certErrorInfo.value = {
    host: request.host,
    url: request.url,
    error: request.status_code === 0 ? 'TLS Handshake Failed' : 'Certificate validation error'
  };
  certErrorDialog.value?.showModal();
}

// 关闭证书错误弹窗
function closeCertErrorDialog() {
  certErrorDialog.value?.close();
  certErrorInfo.value = null;
}

// 检查CA证书安装
async function checkCAInstallation() {
  try {
    const response = await invoke<any>('export_root_ca');
    if (response.success && response.data) {
      dialog.toast.info(t('trafficAnalysis.history.certificateError.tips.installCA'));
      // 打开证书文件位置
      await invoke('show_in_folder', { path: response.data });
    }
  } catch (error: any) {
    console.error('Failed to check CA installation:', error);
    dialog.toast.error(`Failed to check certificate: ${error}`);
  }
}

function applyFilters() {
  scrollTop.value = 0;
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0;
  }
}

function toggleFiltersEnabled() {
  filtersEnabled.value = !filtersEnabled.value;
  localStorage.setItem('proxyHistory.filtersEnabled', String(filtersEnabled.value));
  applyFilters();
}


// 筛选器弹窗相关方法
function openFilterDialog() {
  // 备份当前配置
  backupFilterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
  // 复制应用的配置到编辑配置
  filterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
  filterDialog.value?.showModal();
}

function closeFilterDialog() {
  filterDialog.value?.close();
}

function applyFilterConfig() {
  // 应用配置
  appliedFilterConfig.value = JSON.parse(JSON.stringify(filterConfig.value));
  // 保存到 localStorage
  localStorage.setItem('proxyHistory.filterConfig', JSON.stringify(appliedFilterConfig.value));
  closeFilterDialog();
  applyFilters();
}

function revertFilterChanges() {
  filterConfig.value = JSON.parse(JSON.stringify(backupFilterConfig.value));
}

function showAllFilters() {
  filterConfig.value = showAllProxyHistoryFilters(filterConfig.value);
}

function hideAllFilters() {
  filterConfig.value = hideAllProxyHistoryFilters(filterConfig.value);
}

// Convert current settings to Bambda expression
function convertToBambda() {
  filterConfig.value.bambdaExpression = convertProxyHistoryFilterConfigToBambda(filterConfig.value);
  dialog.toast.success('Converted to Bambda expression');
}

// 加载保存的筛选器配置
function loadFilterConfig() {
  try {
    const saved = localStorage.getItem('proxyHistory.filterConfig');
    if (saved) {
      const parsed = JSON.parse(saved);
      appliedFilterConfig.value = mergeStoredProxyHistoryFilterConfig(parsed);
      filterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
    }
  } catch (e) {
    console.error('Failed to load filter config:', e);
  }
}

// 初始化面板高度（基于容器实际高度）
function initPanelHeights() {
  const containerHeight = mainContainer.value?.clientHeight || 600;
  const savedTopHeight = localStorage.getItem(STORAGE_KEY_TOP_HEIGHT);
  const immersiveTargetHeight = Math.floor(containerHeight * 0.32)
  
  if (savedTopHeight) {
    // 确保保存的值不超过容器高度
    topPanelHeight.value = Math.min(parseInt(savedTopHeight), containerHeight - 200);
  } else {
    topPanelHeight.value = immersiveDrillModeEnabled.value
      ? immersiveTargetHeight
      : Math.floor(containerHeight * 0.4);
  }

  if (immersiveDrillModeEnabled.value) {
    topPanelHeight.value = Math.min(topPanelHeight.value, immersiveTargetHeight)
  }
  
  // 初始化左面板宽度
  const containerWidth = mainContainer.value?.clientWidth || 1200;
  if (!localStorage.getItem(STORAGE_KEY_LEFT_WIDTH)) {
    leftPanelWidth.value = Math.floor(containerWidth * 0.5);
  }
}

// 设置主容器的 ResizeObserver
function setupMainContainerResizeObserver() {
  if (mainContainer.value) {
    mainContainerResizeObserver = new ResizeObserver(() => {
      // 当容器大小改变时，重新初始化面板高度
      if (selectedRequest.value) {
        const containerHeight = mainContainer.value?.clientHeight || 600;
        // 确保 topPanelHeight 不超过容器高度
        if (topPanelHeight.value > containerHeight - 200) {
          topPanelHeight.value = Math.floor(containerHeight * 0.4);
        }
      }
      updateContainerHeight();
    });
    mainContainerResizeObserver.observe(mainContainer.value);
  }
}

// 键盘快捷键处理
async function handleKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.repeat) return;
  if (!mainContainer.value || mainContainer.value.offsetParent === null) return;

  if (shouldHandleHistoryArrowNavigation(event)) {
    event.preventDefault();
    event.stopPropagation();
    navigateHistorySelection(event.key === 'ArrowDown' ? 1 : -1);
    return;
  }

  // Cmd/Ctrl + R 发送到 Repeater
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
    // 如果有选中的请求，发送到 Repeater
    if (selectedRequest.value) {
      event.preventDefault();
      event.stopPropagation();
      
      const req = selectedRequest.value;
      const detailedRequest = req.has_full_details === false
        ? await fetchRequestDetails(req.id) || req
        : req
      
      emit('sendToRepeater', buildHttpExchangeRequestFromHistory(detailedRequest));
    }
  }
}

// 生命周期
onMounted(async () => {
  // 加载筛选器配置
  loadFilterConfig();
  
  await setupEventListeners();
  await refreshRequests();
  
  // 等待 DOM 渲染完成后再设置 ResizeObserver
  await nextTick();
  updateContainerHeight();
  setupResizeObserver();
  setupMainContainerResizeObserver();
  initPanelHeights();
  schedulePrefetchCheck();
  
  // 添加键盘快捷键监听
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  cleanupDataRuntime();
  if (scrollFrameId !== null) {
    cancelAnimationFrame(scrollFrameId);
    scrollFrameId = null;
  }
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer);
    prefetchTimer = null;
  }
  
  // 清理 ResizeObserver
  if (resizeObserver && scrollContainer.value) {
    resizeObserver.unobserve(scrollContainer.value);
    resizeObserver.disconnect();
  }
  if (mainContainerResizeObserver) {
    mainContainerResizeObserver.disconnect();
  }
  // 清理拖拽事件监听
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.removeEventListener('mousemove', handleVerticalResize);
  document.removeEventListener('mouseup', stopVerticalResize);
  // 清理键盘快捷键监听
  document.removeEventListener('keydown', handleKeydown);
  
  // 清理内存 - 防止泄漏
  selectedRequests.value.clear();
});

// 监听详情面板的打开/关闭，更新容器高度
watch(selectedRequest, async () => {
  if (selectedRequestEvidence.value && selectedRequest.value?.id !== selectedRequestEvidence.value.requestId) {
    selectedRequestEvidence.value = null
  }
  await nextTick();
  updateContainerHeight();
});

watch(
  () => [sortedRequests.value.length, containerHeight.value, protocolFilter.value] as const,
  () => {
    schedulePrefetchCheck();
  },
);

watch(
  sortedRequestIds,
  (nextIds, previousIds = []) => {
    preserveScrollAnchorOnPrependedRows(previousIds, nextIds)
  },
  { flush: 'post' },
)

watch(
  () => [selectedRequest.value?.id, selectedRequest.value?.has_full_details] as const,
  async ([requestId, hasFullDetails]) => {
    if (!requestId || hasFullDetails !== false) {
      isSelectedRequestLoading.value = false
      return
    }

    isSelectedRequestLoading.value = true
    try {
      const detailedRequest = await fetchRequestDetails(requestId)
      if (detailedRequest && selectedRequest.value?.id === requestId) {
        selectedRequest.value = detailedRequest
      }
    } finally {
      if (selectedRequest.value?.id === requestId) {
        isSelectedRequestLoading.value = false
      }
    }
  },
)

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyHistory] Refresh triggered by parent');
  await refreshRequests();
  await restoreSavedScrollPosition();
});

// 监听协议类型切换
// 监听协议类型切换
watch(protocolFilter, async (newFilter) => {
  console.log('[ProxyHistory] Protocol filter changed to:', newFilter);
  
  // 清除详情面板选择，避免混淆
  selectedRequest.value = null;
  selectedRequestEvidence.value = null
  
  if (newFilter === 'websocket') {
    // 切换到 WebSocket 时加载连接列表
    await loadWsConnections();
  } else {
    // 切换到 HTTP 时刷新请求
    await refreshRequests();
    await nextTick();
    schedulePrefetchCheck();
  }
});

watch(
  () => immersiveDrillModeEnabled.value,
  enabled => {
    if (!enabled || !mainContainer.value) {
      return
    }

    const containerHeight = mainContainer.value.clientHeight || 600
    topPanelHeight.value = Math.min(topPanelHeight.value, Math.floor(containerHeight * 0.32))
  },
)

// 设置 WebSocket 事件监听
let unlistenWsConnection: (() => void) | null = null;
let unlistenWsMessage: (() => void) | null = null;

async function setupWsEventListeners() {
  try {
    unlistenWsConnection = await listen<any>('proxy:websocket_connection', (event) => {
      console.log('[ProxyHistory] WebSocket connection event:', event.payload);
      // 添加到连接列表
      const conn = event.payload as WebSocketConnection;
      // 确保 message_ids 字段存在
      if (!conn.message_ids) {
        conn.message_ids = [];
      }
      const existingIndex = wsConnections.value.findIndex(c => c.id === conn.id);
      if (existingIndex >= 0) {
        wsConnections.value[existingIndex] = conn;
      } else {
        wsConnections.value.unshift(conn);
      }
    });

    unlistenWsMessage = await listen<any>('proxy:websocket_message', (event) => {
      console.log('[ProxyHistory] WebSocket message event:', event.payload);
      // 添加到消息列表
      const msg = event.payload as WebSocketMessage;
      wsMessages.value.push(msg);
    });
  } catch (error) {
    console.error('Failed to setup WebSocket event listeners:', error);
  }
}

// 在 onMounted 时也设置 WebSocket 事件
onMounted(async () => {
  await setupWsEventListeners();
});

onActivated(() => {
  void refreshVirtualLayout()
})

onDeactivated(() => {
  captureScrollPosition()
})

// 在 onUnmounted 时清理 WebSocket 事件
onUnmounted(() => {
  if (unlistenWsConnection) unlistenWsConnection();
  if (unlistenWsMessage) unlistenWsMessage();
});

// 根据过滤规则移除匹配的记录
function removeMatchingRecords(rule: { matchType: string; condition: string; relationship: string }) {
  const beforeCount = requests.value.length;
  
  requests.value = requests.value.filter(req => {
    let shouldKeep = true;
    
    try {
      switch (rule.matchType) {
        case 'domain_name': {
          // Extract domain from URL
          const url = new URL(req.url);
          const domain = url.hostname;
          
          if (rule.relationship === 'matches') {
            shouldKeep = domain !== rule.condition;
          } else if (rule.relationship === 'does_not_match') {
            shouldKeep = domain === rule.condition;
          }
          break;
        }
        
        case 'url': {
          if (rule.relationship === 'matches') {
            shouldKeep = req.url !== rule.condition;
          } else if (rule.relationship === 'does_not_match') {
            shouldKeep = req.url === rule.condition;
          }
          break;
        }
        
        case 'http_method': {
          const method = req.method.toLowerCase();
          const conditionMethod = rule.condition.toLowerCase();
          
          if (rule.relationship === 'matches') {
            shouldKeep = method !== conditionMethod;
          } else if (rule.relationship === 'does_not_match') {
            shouldKeep = method === conditionMethod;
          }
          break;
        }
        
        case 'file_extension': {
          // Extract file extension from URL
          const url = new URL(req.url);
          const pathname = url.pathname;
          const lastDot = pathname.lastIndexOf('.');
          
          if (lastDot > 0) {
            const extension = pathname.substring(lastDot + 1).split('?')[0];
            // rule.condition is a regex pattern like ^js$
            const regex = new RegExp(rule.condition);
            
            if (rule.relationship === 'matches') {
              shouldKeep = !regex.test(extension);
            } else if (rule.relationship === 'does_not_match') {
              shouldKeep = regex.test(extension);
            }
          }
          break;
        }
      }
    } catch (e) {
      console.error('Error matching filter rule:', e);
      // Keep the record if there's an error
      shouldKeep = true;
    }
    
    return shouldKeep;
  });
  
  const removedCount = beforeCount - requests.value.length;
  
  if (removedCount > 0) {
    console.log(`[ProxyHistory] Removed ${removedCount} matching records`);
    dialog.toast.info(`Removed ${removedCount} matching record(s) from history`);
    
    // Clear selection if selected request was removed
    if (selectedRequest.value && !requests.value.find(r => r.id === selectedRequest.value?.id)) {
      selectedRequest.value = null;
    }
  }
}

// Expose methods for parent component
defineExpose({
  captureScrollPosition,
  removeMatchingRecords,
  openRequestById,
  refreshVirtualLayout,
  restoreSavedScrollPosition,
});
</script>

<style scoped>
pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
