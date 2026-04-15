<template>
  <div class="flex flex-col h-full" @contextmenu.prevent>
    <!-- Intercept Controls Header -->
    <div class="bg-base-200 border-b border-base-300 p-3 flex-shrink-0">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="font-semibold text-base">
            <i class="fas fa-shield-alt mr-2"></i>
            {{ $t('trafficAnalysis.intercept.title') }}
          </h2>
          <!-- HTTP Intercept Status -->
          <div class="tooltip" :data-tip="$t('trafficAnalysis.intercept.tooltip.http')">
            <div class="badge badge-sm cursor-help" :class="interceptEnabled ? 'badge-success' : 'badge-neutral'">
              <i :class="['fas fa-circle mr-2', interceptEnabled ? 'text-success-content' : 'text-neutral-content']"></i>
              HTTP
            </div>
          </div>
          <div class="tooltip" :data-tip="$t('trafficAnalysis.intercept.tooltip.response')">
            <button
              type="button"
              class="badge badge-sm cursor-pointer border-0"
              :class="responseInterceptActive ? 'badge-success' : responseInterceptEnabled ? 'badge-warning' : 'badge-neutral'"
              @click="openResponseInterceptionSettings"
            >
              <i :class="['fas fa-circle mr-2', responseInterceptActive ? 'text-success-content' : responseInterceptEnabled ? 'text-warning-content' : 'text-neutral-content']"></i>
              RESP
            </button>
          </div>
          <!-- WebSocket Intercept Status -->
          <div class="tooltip" :data-tip="$t('trafficAnalysis.intercept.tooltip.websocket')">
            <div class="badge badge-sm cursor-help" :class="websocketInterceptEnabled ? 'badge-success' : 'badge-neutral'">
              <i :class="['fas fa-circle mr-2', websocketInterceptEnabled ? 'text-success-content' : 'text-neutral-content']"></i>
              WS
            </div>
          </div>
        </div>
        
        <div class="flex items-center gap-2">
          <!-- HTTP Toggle -->
          <button 
            @click="toggleIntercept"
            :class="['btn btn-sm', interceptEnabled ? 'btn-error' : 'btn-success']"
            :title="$t('trafficAnalysis.intercept.buttons.toggleHttp')"
          >
            <i :class="['fas', interceptEnabled ? 'fa-stop' : 'fa-play', 'mr-1']"></i>
            HTTP
          </button>

          <!-- WebSocket Toggle -->
          <button 
            @click="toggleWebSocketIntercept"
            :class="['btn btn-sm', websocketInterceptEnabled ? 'btn-error' : 'btn-success']"
            :title="$t('trafficAnalysis.intercept.buttons.toggleWs')"
          >
            <i :class="['fas', websocketInterceptEnabled ? 'fa-stop' : 'fa-plug', 'mr-1']"></i>
            WS
          </button>
          
          <div class="divider divider-horizontal mx-0"></div>
          
          <div class="stats stats-horizontal shadow-sm">
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('trafficAnalysis.intercept.stats.proxyStatus') }}</div>
              <div class="stat-value text-sm" :class="proxyStatus.running ? 'text-success' : 'text-error'">
                {{ proxyStatus.running ? $t('trafficAnalysis.intercept.stats.running') : $t('trafficAnalysis.intercept.stats.stopped') }}
              </div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('trafficAnalysis.intercept.stats.port') }}</div>
              <div class="stat-value text-sm">{{ proxyStatus.port || 8080 }}</div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('trafficAnalysis.intercept.stats.interceptQueue') }}</div>
              <div class="stat-value text-sm">{{ interceptedItems.length }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 flex flex-col overflow-hidden min-h-0">
      <!-- No Request Intercepted State -->
      <div v-if="interceptedItems.length === 0" class="flex-1 flex items-center justify-center bg-base-100">
        <div class="text-center">
          <i class="fas fa-hourglass-half text-6xl text-base-content/30 mb-4"></i>
          <p class="text-lg font-semibold text-base-content/70">{{ $t('trafficAnalysis.intercept.waiting') }}</p>
          <p class="text-sm text-base-content/50 mt-2">
            {{ interceptEnabled || websocketInterceptEnabled ? $t('trafficAnalysis.intercept.enabled') : $t('trafficAnalysis.intercept.disabled') }}
          </p>
          <p class="text-sm text-base-content/50">
            {{ $t('trafficAnalysis.intercept.proxyConfig') }} 127.0.0.1:{{ proxyStatus.port || 8080 }}
          </p>
        </div>
      </div>

      <!-- Request/Response Display Area (with queue on top) -->
      <div v-else class="flex-1 flex flex-col overflow-hidden min-h-0">
        <!-- 上半部分：拦截队列 -->
        <div 
          class="bg-base-100 border-b border-base-300 overflow-hidden flex flex-col flex-shrink-0"
          :style="{ height: queuePanelHeight + 'px' }"
        >
          <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
            <h3 class="font-semibold text-sm">
              <i class="fas fa-list mr-2"></i>
              {{ $t('trafficAnalysis.intercept.queue') }} ({{ interceptedItems.length }})
            </h3>
            <div class="flex items-center gap-2">
              <button 
                @click="forwardAll"
                class="btn btn-xs btn-outline"
                :disabled="isProcessing || interceptedItems.length === 0"
              >
                <i class="fas fa-forward mr-1"></i>
                {{ $t('trafficAnalysis.intercept.buttons.forwardAll') }}
              </button>
              <button 
                @click="dropAll"
                class="btn btn-xs btn-outline btn-error"
                :disabled="isProcessing || interceptedItems.length === 0"
              >
                <i class="fas fa-trash mr-1"></i>
                {{ $t('trafficAnalysis.intercept.buttons.dropAll') }}
              </button>
            </div>
          </div>
          
          <div class="flex-1 overflow-auto p-2">
            <div class="space-y-1">
              <div 
                v-for="(item, index) in interceptedItems" 
                :key="item.data.id"
                class="flex items-center gap-3 text-sm p-2 rounded cursor-pointer hover:bg-base-200 border border-transparent"
                :class="{ 'bg-primary/10 border-primary/30': currentItemIndex === index }"
                @click="selectItem(index)"
                @contextmenu.prevent="showContextMenu($event, item, index)"
              >
                <!-- 请求显示 -->
                <template v-if="item.type === 'request'">
                  <span class="badge badge-sm badge-outline">REQ</span>
                  <span class="badge badge-sm" :class="getMethodClass((item.data as any).method)">{{ (item.data as any).method }}</span>
                  <span class="truncate flex-1 font-mono text-xs">{{ (item.data as any).url }}</span>
                </template>
                <!-- 响应显示 -->
                <template v-else-if="item.type === 'response'">
                  <span class="badge badge-sm badge-secondary">RES</span>
                  <span class="badge badge-sm" :class="getStatusClass((item.data as any).status)">{{ (item.data as any).status }}</span>
                  <span class="truncate flex-1 font-mono text-xs text-base-content/70">{{ $t('trafficAnalysis.intercept.response') }} #{{ (item.data as any).request_id.slice(0, 8) }}</span>
                </template>
                <!-- WebSocket 显示 -->
                <template v-else-if="item.type === 'websocket'">
                  <span class="badge badge-sm badge-accent">WS</span>
                  <i 
                    class="fas text-xs w-4 text-center"
                    :class="(item.data as any).direction === 'client_to_server' ? 'fa-arrow-up text-success' : 'fa-arrow-down text-info'"
                  ></i>
                  <span class="badge badge-xs badge-ghost font-mono">{{ (item.data as any).message_type }}</span>
                  <span class="truncate flex-1 font-mono text-xs text-base-content/70">
                    {{ (item.data as any).content ? truncate((item.data as any).content, 50) : '[No Content]' }}
                  </span>
                </template>
                <span class="text-xs text-base-content/50">{{ formatTimestamp(item.data.timestamp) }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Context Menu -->
        <div 
          v-if="contextMenu.visible"
          class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
          :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
          @click.stop
        >
          <!-- Forward/Drop -->
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuForward"
          >
            <i class="fas fa-arrow-right w-4 text-success"></i>
            {{ $t('trafficAnalysis.intercept.buttons.forward') }}
          </button>
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuDrop"
          >
            <i class="fas fa-times w-4 text-error"></i>
            {{ $t('trafficAnalysis.intercept.buttons.drop') }}
          </button>
          
          <div class="divider my-1 h-px"></div>
          
          <TrafficContextMenuSections
            :sections="interceptSendContextMenuSections"
            label-prefix="trafficAnalysis.intercept.contextMenu"
          />
          
          <div class="divider my-1 h-px"></div>
          
          <TrafficContextSubmenu
            v-if="interceptFilterSubmenu"
            :submenu="interceptFilterSubmenu"
            label-prefix="trafficAnalysis.intercept.contextMenu"
          />
          
          <div class="divider my-1 h-px"></div>
          
          <TrafficContextMenuSections
            :sections="interceptRequestContextMenuSections"
            label-prefix="trafficAnalysis.intercept.contextMenu"
          />
        </div>

        <!-- 分割条 -->
        <div 
          class="h-1 bg-base-300 cursor-row-resize hover:bg-primary/50 transition-colors flex-shrink-0"
          @mousedown="startResize"
        ></div>

        <!-- 下半部分：请求/响应详情 -->
        <div class="flex-1 flex flex-col overflow-hidden min-h-0">
          <!-- Action Buttons -->
          <div class="bg-base-200 border-b border-base-300 p-2 flex items-center gap-2 flex-shrink-0">
            <button 
              @click="forwardCurrentItem"
              class="btn btn-success btn-sm"
              :disabled="isProcessing || !currentItem"
            >
              <i class="fas fa-arrow-right mr-1"></i>
              {{ $t('trafficAnalysis.intercept.buttons.forward') }}
            </button>
            
            <button 
              @click="forwardAll"
              class="btn btn-success btn-sm btn-outline"
              :disabled="isProcessing || interceptedItems.length === 0"
            >
              <i class="fas fa-forward mr-1"></i>
              {{ $t('trafficAnalysis.intercept.buttons.forwardAll') }}
            </button>
            
            <button 
              @click="dropCurrentItem"
              class="btn btn-error btn-sm"
              :disabled="isProcessing || !currentItem"
            >
              <i class="fas fa-times mr-1"></i>
              {{ $t('trafficAnalysis.intercept.buttons.drop') }}
            </button>
            
            <div class="divider divider-horizontal mx-1"></div>
            
            <button
              v-for="item in interceptToolbarSendMenuItems"
              :key="`intercept-toolbar-send-${item.key}`"
              @click="item.onClick"
              class="btn btn-outline btn-sm"
              :disabled="!currentItem"
            >
              <i :class="`${item.iconClass.replace(' text-primary', '').replace(' text-accent', '').replace(' text-secondary', '')} mr-1`"></i>
              {{ $t(`trafficAnalysis.intercept.buttons.${item.labelKey}`) }}
            </button>
            
            <div class="flex-1"></div>
            
            <div v-if="currentItem" class="text-xs text-base-content/70 truncate max-w-md">
              <i class="fas fa-info-circle mr-1"></i>
              <template v-if="currentItem.type === 'request'">
                {{ (currentItem.data as any).method }} {{ (currentItem.data as any).url }}
              </template>
              <template v-else-if="currentItem.type === 'response'">
                {{ $t('trafficAnalysis.intercept.response') }} {{ (currentItem.data as any).status }}
              </template>
              <template v-else-if="currentItem.type === 'websocket'">
                WS {{ (currentItem.data as any).direction === 'client_to_server' ? '↑' : '↓' }} {{ (currentItem.data as any).message_type }}
              </template>
            </div>
          </div>

          <!-- Content Tabs -->
          <div class="tabs tabs-boxed bg-base-200 border-b border-base-300 px-3 py-1 flex-shrink-0">
            <a 
              :class="['tab tab-sm', activeTab === 'pretty' ? 'tab-active' : '']"
              @click="activeTab = 'pretty'"
            >
              {{ $t('trafficAnalysis.intercept.tabs.pretty') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'raw' ? 'tab-active' : '']"
              @click="activeTab = 'raw'"
            >
              {{ $t('trafficAnalysis.intercept.tabs.raw') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'hex' ? 'tab-active' : '']"
              @click="activeTab = 'hex'"
            >
              {{ $t('trafficAnalysis.intercept.tabs.hex') }}
            </a>
          </div>

          <!-- Content View -->
          <div class="flex-1 overflow-hidden bg-base-100 min-h-0 flex flex-col">
            <template v-if="currentItem">
              <!-- Raw View -->
              <div
                v-if="activeTab === 'raw'"
                class="flex-1 min-h-0 flex flex-col"
                @contextmenu.capture.prevent="showCurrentItemContextMenu($event)"
              >
                <HttpMessageSurface
                  v-model="requestContent"
                  :readonly="!isEditable"
                  custom-context-menu
                  :message-type="currentItemType === 'response' ? 'response' : currentItemType === 'request' ? 'request' : 'generic'"
                  height="100%"
                  display-mode="raw"
                  :state-key="currentItem ? `intercept:${currentItem.type}:${currentItemIndex}:raw` : ''"
                  @contextmenu="showCurrentItemContextMenu($event)"
                />
              </div>

              <!-- Pretty View -->
              <div
                v-else-if="activeTab === 'pretty'"
                class="flex-1 min-h-0 flex flex-col"
                @contextmenu.capture.prevent="showCurrentItemContextMenu($event)"
              >
                <HttpMessageSurface
                  v-model="prettyContent"
                  :readonly="!isEditable"
                  custom-context-menu
                  :message-type="currentItemType === 'response' ? 'response' : currentItemType === 'request' ? 'request' : 'generic'"
                  height="100%"
                  display-mode="pretty"
                  :state-key="currentItem ? `intercept:${currentItem.type}:${currentItemIndex}:pretty` : ''"
                  @contextmenu="showCurrentItemContextMenu($event)"
                />
              </div>

              <!-- Hex View -->
              <div v-else-if="activeTab === 'hex'" class="flex-1 overflow-auto p-4 font-mono text-xs min-h-0">
                <pre>{{ hexView }}</pre>
              </div>
            </template>
            
            <!-- No selection -->
            <div v-else class="flex-1 flex items-center justify-center text-base-content/50">
              <div class="text-center">
                <i class="fas fa-mouse-pointer text-4xl mb-2"></i>
                <p>{{ $t('trafficAnalysis.intercept.clickToView') }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Filter Rule Dialog -->
    <AppDialog ref="filterDialogRef" class="modal">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-4">
          <i class="fas fa-filter mr-2"></i>
          {{ $t('trafficAnalysis.intercept.filterDialog.title') }}
        </h3>
        
        <div class="space-y-4">
          <!-- Filter Type -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.intercept.filterDialog.filterType') }}</span>
            </label>
            <select v-model="filterRule.type" class="select select-bordered w-full">
              <option value="request">{{ $t('trafficAnalysis.intercept.filterDialog.typeRequest') }}</option>
              <option value="response">{{ $t('trafficAnalysis.intercept.filterDialog.typeResponse') }}</option>
            </select>
          </div>
          
          <!-- Match Type -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.intercept.filterDialog.matchType') }}</span>
            </label>
            <select v-model="filterRule.matchType" class="select select-bordered w-full">
              <template v-if="filterRule.type === 'request'">
                <option value="domain">{{ $t('trafficAnalysis.intercept.filterDialog.matchDomain') }}</option>
                <option value="url">{{ $t('trafficAnalysis.intercept.filterDialog.matchUrl') }}</option>
                <option value="method">{{ $t('trafficAnalysis.intercept.filterDialog.matchMethod') }}</option>
                <option value="fileExt">{{ $t('trafficAnalysis.intercept.filterDialog.matchFileExt') }}</option>
                <option value="header">{{ $t('trafficAnalysis.intercept.filterDialog.matchHeader') }}</option>
              </template>
              <template v-else>
                <option value="status">{{ $t('trafficAnalysis.intercept.filterDialog.matchStatus') }}</option>
                <option value="contentType">{{ $t('trafficAnalysis.intercept.filterDialog.matchContentType') }}</option>
                <option value="header">{{ $t('trafficAnalysis.intercept.filterDialog.matchHeader') }}</option>
              </template>
            </select>
          </div>
          
          <!-- Relationship -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.intercept.filterDialog.relationship') }}</span>
            </label>
            <select v-model="filterRule.relationship" class="select select-bordered w-full">
              <option value="matches">{{ $t('trafficAnalysis.intercept.filterDialog.matches') }}</option>
              <option value="notMatches">{{ $t('trafficAnalysis.intercept.filterDialog.notMatches') }}</option>
              <option value="contains">{{ $t('trafficAnalysis.intercept.filterDialog.contains') }}</option>
              <option value="notContains">{{ $t('trafficAnalysis.intercept.filterDialog.notContains') }}</option>
            </select>
          </div>
          
          <!-- Condition -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.intercept.filterDialog.condition') }}</span>
            </label>
            <input 
              type="text" 
              v-model="filterRule.condition"
              class="input input-bordered w-full"
              :placeholder="$t('trafficAnalysis.intercept.filterDialog.conditionPlaceholder')"
            />
          </div>
          
          <!-- Action -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.intercept.filterDialog.action') }}</span>
            </label>
            <select v-model="filterRule.action" class="select select-bordered w-full">
              <option value="exclude">{{ $t('trafficAnalysis.intercept.filterDialog.actionExclude') }}</option>
              <option value="include">{{ $t('trafficAnalysis.intercept.filterDialog.actionInclude') }}</option>
            </select>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeFilterDialog">
            {{ $t('trafficAnalysis.intercept.filterDialog.cancel') }}
          </button>
          <button class="btn btn-primary" @click="saveFilterRule" :disabled="!filterRule.condition">
            {{ $t('trafficAnalysis.intercept.filterDialog.save') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </AppDialog>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit as tauriEmit } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue';
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import TrafficContextSubmenu from './TrafficContextSubmenu.vue'
import type { TrafficComparerDraftRequestInput } from './transfers'
import { getDefaultTrafficMessageViewTab } from './trafficDisplaySettings'
import { buildTrafficRequestActionMenuItems } from './trafficRequestActionMenuSupport'
import { buildTrafficContextMenuSections } from './trafficContextMenuSectionSupport'
import { buildTrafficContextSubmenu } from './trafficContextSubmenuSupport'
import { useTrafficSendTargets } from './trafficSendTargets'
import { buildTrafficRequestSendMenuItems } from './trafficSendMenuSupport'
import {
  convertInterceptedItemToProxyRequest as convertToProxyRequest,
  formatInterceptBody as formatBody,
  formatInterceptTimestamp as formatTimestamp,
  getInterceptItemDirection,
  getInterceptItemDomain,
  getInterceptItemMethod,
  getInterceptItemStatus,
  getInterceptMethodClass as getMethodClass,
  getInterceptStatusClass as getStatusClass,
  truncateInterceptText as truncate,
  buildExchangeRequestFromInterceptedRequest,
  type InterceptedItem,
  type InterceptedRequest,
  type InterceptedResponse,
  type InterceptedWebSocketMessage,
  type ProxyRequestForAI,
  type ProxyStats,
  type ProxyStatus,
} from './proxyInterceptSupport';
import type { HttpExchangeRequest } from './http/model'
import { normalizeHttpVersionToken } from './http/version'

const { t } = useI18n();
const router = useRouter();
const { enabledTargets } = useTrafficSendTargets()

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 发送到 Repeater 的事件
const emit = defineEmits<{
  (e: 'openResponseInterceptionSettings'): void
  (e: 'interceptQueueChanged', count: number): void
  (e: 'sendToRepeater', request: HttpExchangeRequest): void
  (e: 'sendDraftRequestToComparer', payload: TrafficComparerDraftRequestInput): void
  (e: 'sendToIntruder', request: HttpExchangeRequest): void
  (e: 'sendToAssistant', requests: any[]): void
}>();

// 响应式状态
const proxyStatus = ref<ProxyStatus>({
  running: false,
  port: 0,
  mitm: false,
  stats: {
    http_requests: 0,
    https_requests: 0,
    errors: 0,
    qps: 0.0,
  },
});

// Intercept 状态
const interceptEnabled = ref(false);
const responseInterceptEnabled = ref(false);
const websocketInterceptEnabled = ref(false);
const responseInterceptActive = computed(() => interceptEnabled.value && responseInterceptEnabled.value);

const interceptedRequests = ref<InterceptedRequest[]>([]);
const interceptedResponses = ref<InterceptedResponse[]>([]);
const interceptedWebsockets = ref<InterceptedWebSocketMessage[]>([]);

const currentItemIndex = ref(0);
const currentItemType = ref<'request' | 'response' | 'websocket'>('request');
const activeTab = ref<'raw' | 'pretty' | 'hex'>(getDefaultTrafficMessageViewTab());
const isEditable = ref(true); // 默认可编辑
const isProcessing = ref(false);
const requestContent = ref('');

// Context menu state
interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  item: InterceptedItem | null;
  index: number;
}

const contextMenu = ref<ContextMenuState>({
  visible: false,
  x: 0,
  y: 0,
  item: null,
  index: -1
});

// Filter dialog state
interface FilterRule {
  type: 'request' | 'response';
  matchType: string;
  relationship: string;
  condition: string;
  action: 'exclude' | 'include';
}

const filterDialogRef = ref<HTMLDialogElement | null>(null);
const filterRule = ref<FilterRule>({
  type: 'request',
  matchType: 'domain',
  relationship: 'matches',
  condition: '',
  action: 'exclude'
});

// 合并的拦截队列（请求、响应、WebSocket）
const interceptedItems = computed<InterceptedItem[]>(() => {
  const items: InterceptedItem[] = [];
  interceptedRequests.value.forEach(req => {
    items.push({ type: 'request', data: req });
  });
  interceptedResponses.value.forEach(resp => {
    items.push({ type: 'response', data: resp });
  });
  interceptedWebsockets.value.forEach(ws => {
    items.push({ type: 'websocket', data: ws });
  });
  // 按时间戳排序
  items.sort((a, b) => a.data.timestamp - b.data.timestamp);
  return items;
});

// 当前选中的项
const currentItem = computed(() => {
  if (interceptedItems.value.length === 0) return null;
  return interceptedItems.value[currentItemIndex.value] || null;
});

// 兼容性：当前请求（用于现有代码）
const currentRequest = computed(() => {
  const item = currentItem.value;
  if (!item || item.type !== 'request') return null;
  return item.data as InterceptedRequest;
});
const currentSendTargets = computed(() => {
  if (currentItem.value?.type === 'request') {
    return ['repeater', 'comparer', 'intruder'] as const
  }
  if (currentItem.value?.type === 'response') {
    return ['comparer'] as const
  }
  return [] as const
})
const interceptContextSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: contextMenu.value.item?.type === 'request'
      ? ['repeater', 'comparer', 'intruder']
      : contextMenu.value.item?.type === 'response'
        ? ['comparer']
        : [],
    actions: {
      repeater: contextMenu.value.item?.type === 'request' ? contextMenuSendToRepeater : undefined,
      comparer: contextMenuSendToComparer,
      intruder: contextMenu.value.item?.type === 'request' ? contextMenuSendToIntruder : undefined,
    },
  }),
)
const interceptToolbarSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: [...currentSendTargets.value],
    actions: {
      repeater: currentItem.value?.type === 'request' ? sendToRepeater : undefined,
      comparer: sendToComparer,
      intruder: currentItem.value?.type === 'request' ? sendToIntruder : undefined,
    },
  }),
)
const interceptContextRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl: contextMenuCopyUrl,
      copyRequest: contextMenuCopyRequest,
      copyAsCurl: contextMenuCopyAsCurl,
      openInBrowser: contextMenuOpenInBrowser,
    },
  }),
)
const interceptSendContextMenuSections = computed(() =>
  buildTrafficContextMenuSections([
    {
      key: 'send',
      items: [
        ...interceptContextSendMenuItems.value,
        contextMenu.value.item?.type === 'request'
          ? {
              key: 'sendToAI',
              iconClass: 'fas fa-robot text-secondary',
              labelKey: 'sendToAI',
              onClick: contextMenuSendToAI,
            }
          : null,
      ],
    },
  ]),
)
const interceptRequestContextMenuSections = computed(() =>
  buildTrafficContextMenuSections([
    {
      key: 'request',
      items: contextMenu.value.item?.type === 'request'
        ? interceptContextRequestActionMenuItems.value.map((item) => ({
            ...item,
            iconClass: `${item.iconClass} w-4`,
          }))
        : [],
    },
    {
      key: 'raw',
      items: [
        {
          key: 'copyRaw',
          iconClass: 'fas fa-file-alt w-4',
          labelKey: 'copyRaw',
          onClick: contextMenuCopyRaw,
        },
      ],
    },
  ]),
)
const interceptFilterSubmenu = computed(() =>
  buildTrafficContextSubmenu({
    key: 'intercept-filter',
    triggerLabelKey: 'addFilter',
    triggerIconClass: 'fas fa-filter w-4 text-warning',
    submenuClass: 'absolute left-full top-0 ml-1 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-56 z-50 hidden group-hover:block',
    items: [
      contextMenu.value.item?.type === 'request'
        ? {
            key: 'filterByDomain',
            iconClass: '',
            labelKey: 'filterByDomain',
            suffixText: getItemDomain(),
            onClick: addFilterByDomain,
          }
        : null,
      contextMenu.value.item?.type === 'request'
        ? {
            key: 'filterByUrl',
            iconClass: '',
            labelKey: 'filterByUrl',
            onClick: addFilterByUrl,
          }
        : null,
      contextMenu.value.item?.type === 'request'
        ? {
            key: 'filterByMethod',
            iconClass: '',
            labelKey: 'filterByMethod',
            suffixText: getItemMethod(),
            onClick: addFilterByMethod,
          }
        : null,
      contextMenu.value.item?.type === 'request'
        ? {
            key: 'filterByFileExt',
            iconClass: '',
            labelKey: 'filterByFileExt',
            onClick: addFilterByFileExt,
          }
        : null,
      contextMenu.value.item?.type === 'response'
        ? {
            key: 'filterByStatus',
            iconClass: '',
            labelKey: 'filterByStatus',
            suffixText: String(getItemStatus()),
            onClick: addFilterByStatus,
          }
        : null,
      contextMenu.value.item?.type === 'response'
        ? {
            key: 'filterByContentType',
            iconClass: '',
            labelKey: 'filterByContentType',
            onClick: addFilterByContentType,
          }
        : null,
      contextMenu.value.item?.type === 'websocket'
        ? {
            key: 'filterByDirection',
            iconClass: '',
            labelKey: 'filterByDirection',
            suffixText: getItemDirection(),
            onClick: addFilterByWsDirection,
          }
        : null,
    ],
    footerItems: [
      {
        key: 'customFilter',
        iconClass: 'fas fa-cog',
        labelKey: 'customFilter',
        onClick: openFilterDialog,
      },
    ],
  }),
)

// 拖拽调整高度
const queuePanelHeight = ref(180);
let isResizing = false;
let startY = 0;
let startHeight = 0;

// Pretty content (formatted) - writable computed
const prettyContent = computed({
  get: () => {
    if (!requestContent.value) return '';
    
    const lines = requestContent.value.split('\n');
    const result: string[] = [];
    let inBody = false;
    const bodyLines: string[] = [];
    
    for (const line of lines) {
      if (!inBody) {
        result.push(line);
        // Empty line marks body start
        if (line.trim() === '') {
          inBody = true;
        }
      } else {
        bodyLines.push(line);
      }
    }
    
    // Format body if present
    if (bodyLines.length > 0) {
      const body = bodyLines.join('\n');
      const formattedBody = formatBody(body);
      result.push(formattedBody);
    }
    
    return result.join('\n');
  },
  set: (value: string) => {
    requestContent.value = value;
  }
});

const hexView = computed(() => {
  if (!requestContent.value) return '';
  const bytes = new TextEncoder().encode(requestContent.value);
  let hex = '';
  for (let i = 0; i < bytes.length; i += 16) {
    const offset = i.toString(16).padStart(8, '0');
    const chunk = bytes.slice(i, i + 16);
    const hexPart = Array.from(chunk)
      .map(b => b.toString(16).padStart(2, '0'))
      .join(' ');
    const asciiPart = Array.from(chunk)
      .map(b => (b >= 32 && b < 127) ? String.fromCharCode(b) : '.')
      .join('');
    hex += `${offset}  ${hexPart.padEnd(48, ' ')}  ${asciiPart}\n`;
  }
  return hex;
});

// 事件监听器
let unlistenProxyStatus: (() => void) | null = null;
let unlistenInterceptRequest: (() => void) | null = null;
let unlistenInterceptResponse: (() => void) | null = null;
let unlistenInterceptWebSocket: (() => void) | null = null;

// 拖拽调整高度
function startResize(event: MouseEvent) {
  isResizing = true;
  startY = event.clientY;
  startHeight = queuePanelHeight.value;
  
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';
}

function handleResize(event: MouseEvent) {
  if (!isResizing) return;
  
  const diff = event.clientY - startY;
  queuePanelHeight.value = Math.max(100, Math.min(startHeight + diff, 400));
}

function stopResize() {
  isResizing = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

// Context Menu Methods
function showContextMenu(event: MouseEvent, item: InterceptedItem, index: number) {
  event.preventDefault();
  event.stopPropagation();
  selectItem(index);
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    item,
    index
  };
  // Close menu on click outside
  document.addEventListener('click', closeContextMenu);
}

function showCurrentItemContextMenu(event: MouseEvent) {
  if (!currentItem.value) return;
  showContextMenu(event, currentItem.value, currentItemIndex.value);
}

function closeContextMenu() {
  contextMenu.value.visible = false;
  document.removeEventListener('click', closeContextMenu);
}

function getItemDomain(): string {
  return getInterceptItemDomain(contextMenu.value.item);
}

function getItemMethod(): string {
  return getInterceptItemMethod(contextMenu.value.item);
}

function getItemStatus(): number {
  return getInterceptItemStatus(contextMenu.value.item);
}

function getItemDirection(): string {
  return getInterceptItemDirection(contextMenu.value.item);
}

async function contextMenuForward() {
  closeContextMenu();
  await forwardCurrentItem();
}

async function contextMenuDrop() {
  closeContextMenu();
  await dropCurrentItem();
}

function contextMenuSendToRepeater() {
  if (contextMenu.value.item?.type === 'request') {
    emit('sendToRepeater', buildExchangeRequestFromInterceptedRequest(contextMenu.value.item.data as InterceptedRequest));
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToRepeater'));
  }
  closeContextMenu();
}

function contextMenuSendToIntruder() {
  if (contextMenu.value.item?.type === 'request') {
    emit('sendToIntruder', buildExchangeRequestFromInterceptedRequest(contextMenu.value.item.data as InterceptedRequest));
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToIntruder'));
  }
  closeContextMenu();
}

function contextMenuSendToComparer() {
  if (contextMenu.value.item?.type === 'request') {
    emit('sendDraftRequestToComparer', {
      request: buildExchangeRequestFromInterceptedRequest(contextMenu.value.item.data as InterceptedRequest),
      label: t('trafficAnalysis.comparer.draft.leftLabel'),
    });
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToComparer'));
  } else if (contextMenu.value.item?.type === 'response') {
    emit('sendDraftRequestToComparer', {
      text: buildInterceptResponseText(contextMenu.value.item.data as InterceptedResponse),
      messageType: 'response',
      label: t('trafficAnalysis.intercept.response'),
    })
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToComparer'));
  }
  closeContextMenu();
}

async function contextMenuSendToAI() {
  const item = contextMenu.value.item;
  if (!item || item.type !== 'request') {
    closeContextMenu();
    return;
  }
  
  // Convert intercepted item to ProxyRequest format for AI assistant
  const proxyRequest = convertToProxyRequest(item);
  if (!proxyRequest) {
    closeContextMenu();
    return;
  }
  
  closeContextMenu();
  
  // Navigate to AI assistant page first
  await router.push('/ai-assistant');
  
  // Wait for the page to mount and set up event listener
  await new Promise(resolve => setTimeout(resolve, 100));
  
  // Send event to AI assistant
  await tauriEmit('traffic:send-to-assistant', { 
    requests: [proxyRequest], 
    type: 'request',
  });
  emit('sendToAssistant', [proxyRequest]);
  
  dialog.toast.success(t('trafficAnalysis.intercept.sentToAssistant', {
    type: t('trafficAnalysis.intercept.request'),
  }));
}

// Filter methods
function openFilterDialog() {
  const item = contextMenu.value.item;
  closeContextMenu();
  
  if (item) {
    filterRule.value.type = item.type === 'response' ? 'response' : 'request';
    filterRule.value.matchType = item.type === 'response' ? 'status' : 'domain';
    filterRule.value.condition = '';
  }
  
  filterDialogRef.value?.showModal();
}

function closeFilterDialog() {
  filterDialogRef.value?.close();
}

async function addFilterByDomain() {
  const domain = getItemDomain();
  if (domain) {
    // Emit event to ProxyConfiguration to add the rule
    await tauriEmit('intercept:add-filter-rule', {
      ruleType: 'request',
      rule: {
        enabled: true,
        operator: 'And',
        matchType: 'domain_name',
        relationship: 'does_not_match',
        condition: domain
      }
    });
    dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
  }
  closeContextMenu();
}

async function addFilterByUrl() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    const url = (item.data as InterceptedRequest).url;
    await tauriEmit('intercept:add-filter-rule', {
      ruleType: 'request',
      rule: {
        enabled: true,
        operator: 'And',
        matchType: 'url',
        relationship: 'does_not_match',
        condition: url
      }
    });
    dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
  }
  closeContextMenu();
}

async function addFilterByMethod() {
  const method = getItemMethod();
  if (method) {
    await tauriEmit('intercept:add-filter-rule', {
      ruleType: 'request',
      rule: {
        enabled: true,
        operator: 'And',
        matchType: 'http_method',
        relationship: 'does_not_match',
        condition: method.toLowerCase()
      }
    });
    dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
  }
  closeContextMenu();
}

async function addFilterByFileExt() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    const path = (item.data as InterceptedRequest).path;
    const match = path.match(/\.([a-zA-Z0-9]+)(?:\?|$)/);
    const ext = match ? match[1] : '';
    if (ext) {
      await tauriEmit('intercept:add-filter-rule', {
        ruleType: 'request',
        rule: {
          enabled: true,
          operator: 'And',
          matchType: 'file_extension',
          relationship: 'does_not_match',
          condition: `^${ext}$`
        }
      });
      dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
    }
  }
  closeContextMenu();
}

async function addFilterByStatus() {
  const status = getItemStatus();
  if (status) {
    await tauriEmit('intercept:add-filter-rule', {
      ruleType: 'response',
      rule: {
        enabled: true,
        operator: 'And',
        matchType: 'status_code',
        relationship: 'does_not_match',
        condition: `^${status}$`
      }
    });
    dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
  }
  closeContextMenu();
}

async function addFilterByContentType() {
  const item = contextMenu.value.item;
  if (item?.type === 'response') {
    const headers = (item.data as InterceptedResponse).headers;
    const contentType = headers['content-type'] || headers['Content-Type'] || '';
    const mainType = contentType.split(';')[0].trim();
    if (mainType) {
      await tauriEmit('intercept:add-filter-rule', {
        ruleType: 'response',
        rule: {
          enabled: true,
          operator: 'And',
          matchType: 'content_type_header',
          relationship: 'does_not_match',
          condition: mainType
        }
      });
      dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
    }
  }
  closeContextMenu();
}

async function addFilterByWsDirection() {
  const item = contextMenu.value.item;
  if (item?.type === 'websocket') {
    const direction = (item.data as InterceptedWebSocketMessage).direction;
    // For now, just show a toast - WebSocket filtering would need backend support
    dialog.toast.info(`Filter by direction: ${direction}`);
  }
  closeContextMenu();
}

async function saveFilterRule() {
  try {
    // Map matchType from dialog to ProxyConfiguration format
    const matchTypeMap: Record<string, string> = {
      'domain': 'domain_name',
      'url': 'url',
      'method': 'http_method',
      'fileExt': 'file_extension',
      'header': 'any_header',
      'status': 'status_code',
      'contentType': 'content_type_header'
    };
    
    // Map relationship from dialog to ProxyConfiguration format
    const relationshipMap: Record<string, string> = {
      'matches': 'matches',
      'notMatches': 'does_not_match',
      'contains': 'matches',
      'notContains': 'does_not_match'
    };
    
    const mappedMatchType = matchTypeMap[filterRule.value.matchType] || filterRule.value.matchType;
    const mappedRelationship = filterRule.value.action === 'exclude' 
      ? 'does_not_match' 
      : relationshipMap[filterRule.value.relationship] || filterRule.value.relationship;
    
    await tauriEmit('intercept:add-filter-rule', {
      ruleType: filterRule.value.type,
      rule: {
        enabled: true,
        operator: 'And',
        matchType: mappedMatchType,
        relationship: mappedRelationship,
        condition: filterRule.value.condition
      }
    });
    
    dialog.toast.success(t('trafficAnalysis.intercept.filterDialog.ruleAdded'));
    closeFilterDialog();
  } catch (error: any) {
    console.error('Failed to save filter rule:', error);
    dialog.toast.error(`${error}`);
  }
}

// Copy methods
async function contextMenuCopyUrl() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    await navigator.clipboard.writeText((item.data as InterceptedRequest).url);
    dialog.toast.success('URL copied');
  }
  closeContextMenu();
}

async function contextMenuCopyAsCurl() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    const req = item.data as InterceptedRequest;
    let curl = `curl -X ${req.method} '${req.url}'`;
    for (const [key, value] of Object.entries(req.headers)) {
      curl += ` -H '${key}: ${value}'`;
    }
    if (req.body) {
      curl += ` -d '${req.body}'`;
    }
    await navigator.clipboard.writeText(curl);
    dialog.toast.success('cURL command copied');
  }
  closeContextMenu();
}

async function contextMenuCopyRequest() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    await navigator.clipboard.writeText(requestContent.value);
    dialog.toast.success('Request copied');
  }
  closeContextMenu();
}

function contextMenuOpenInBrowser() {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    window.open((item.data as InterceptedRequest).url, '_blank');
  }
  closeContextMenu();
}

async function contextMenuCopyRaw() {
  await navigator.clipboard.writeText(requestContent.value);
  dialog.toast.success('Raw content copied');
  closeContextMenu();
}

// 方法
async function toggleIntercept() {
  const newState = !interceptEnabled.value;
  
  try {
    // 关闭拦截时，自动转发所有待处理的请求和响应
    if (!newState && interceptedItems.value.length > 0) {
      await forwardAllSilent();
    }
    
    const response = await invoke<any>('set_intercept_enabled', { enabled: newState });
    if (response.success) {
      interceptEnabled.value = newState;
    } else {
      dialog.toast.error(response.error || '操作失败');
    }
  } catch (error: any) {
    console.error('[ProxyIntercept] Failed to toggle intercept:', error);
    dialog.toast.error(`切换拦截状态失败: ${error}`);
  }
}

function openResponseInterceptionSettings() {
  emit('openResponseInterceptionSettings');
}

async function toggleWebSocketIntercept() {
  const newState = !websocketInterceptEnabled.value;
  
  try {
    const response = await invoke<any>('set_websocket_intercept_enabled', { enabled: newState });
    if (response.success) {
      websocketInterceptEnabled.value = newState;
    } else {
      dialog.toast.error(response.error || '操作失败');
    }
  } catch (error: any) {
    console.error('[ProxyIntercept] Failed to toggle WS intercept:', error);
    dialog.toast.error(`切换 WS 拦截状态失败: ${error}`);
  }
}

async function forwardAll() {
  if (isProcessing.value || interceptedItems.value.length === 0) return;
  
  isProcessing.value = true;
  let successCount = 0;
  
  try {
    // Requests
    for (const req of [...interceptedRequests.value]) {
      const response = await invoke<any>('forward_intercepted_request', { 
        requestId: req.id, modifiedContent: undefined
      });
      if (response.success) successCount++;
    }
    interceptedRequests.value = [];

    // Responses
    for (const resp of [...interceptedResponses.value]) {
      const response = await invoke<any>('forward_intercepted_response', { 
        responseId: resp.id, modifiedContent: undefined
      });
      if (response.success) successCount++;
    }
    interceptedResponses.value = [];

    // WebSockets
    for (const ws of [...interceptedWebsockets.value]) {
      const response = await invoke<any>('forward_intercepted_websocket', { 
        id: ws.id, content: undefined
      });
      if (response.success) successCount++;
    }
    interceptedWebsockets.value = [];
    
    currentItemIndex.value = 0;
    dialog.toast.success(`已批量转发 ${successCount} 个项目`);
  } catch (error: any) {
    console.error('Failed to forward all:', error);
    dialog.toast.error(`批量转发失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

// 静默转发所有（关闭拦截时调用，不显示 toast）
async function forwardAllSilent() {
  if (interceptedItems.value.length === 0) return;
  
  try {
    for (const req of [...interceptedRequests.value]) {
      await invoke<any>('forward_intercepted_request', { requestId: req.id, modifiedContent: undefined });
    }
    for (const resp of [...interceptedResponses.value]) {
      await invoke<any>('forward_intercepted_response', { responseId: resp.id, modifiedContent: undefined });
    }
    // Note: We might want to forward WebSockets too if WS intercept is also turned off, 
    // but this function is called when HTTP intercept is toggled. 
    // If we toggle WS separately, we should handle WS forwarding there.
    
    interceptedRequests.value = [];
    interceptedResponses.value = [];
    currentItemIndex.value = 0;
  } catch (error: any) {
    console.error('Failed to forward all silently:', error);
  }
}

async function dropAll() {
  if (isProcessing.value || interceptedItems.value.length === 0) return;
  
  const confirmed = await dialog.confirm(`确定要丢弃所有 ${interceptedItems.value.length} 个拦截项吗？`);
  if (!confirmed) return;
  
  isProcessing.value = true;
  let droppedCount = 0;
  
  try {
    for (const req of [...interceptedRequests.value]) {
      await invoke<any>('drop_intercepted_request', { requestId: req.id });
      droppedCount++;
    }
    interceptedRequests.value = [];

    for (const resp of [...interceptedResponses.value]) {
      await invoke<any>('drop_intercepted_response', { responseId: resp.id });
      droppedCount++;
    }
    interceptedResponses.value = [];

    for (const ws of [...interceptedWebsockets.value]) {
      await invoke<any>('drop_intercepted_websocket', { id: ws.id });
      droppedCount++;
    }
    interceptedWebsockets.value = [];

    currentItemIndex.value = 0;
    dialog.toast.info(`已丢弃 ${droppedCount} 个项目`);
  } catch (error: any) {
    console.error('Failed to drop all:', error);
    dialog.toast.error(`批量丢弃失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

function sendToRepeater() {
  if (!currentRequest.value) return;
  emit('sendToRepeater', buildExchangeRequestFromInterceptedRequest(currentRequest.value));
}

function sendToIntruder() {
  if (!currentRequest.value) return;
  emit('sendToIntruder', buildExchangeRequestFromInterceptedRequest(currentRequest.value));
}

function sendToComparer() {
  if (currentItem.value?.type === 'request' && currentRequest.value) {
    emit('sendDraftRequestToComparer', {
      request: buildExchangeRequestFromInterceptedRequest(currentRequest.value),
      label: t('trafficAnalysis.comparer.draft.leftLabel'),
    });
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToComparer'));
    return
  }

  if (currentItem.value?.type === 'response') {
    emit('sendDraftRequestToComparer', {
      text: buildInterceptResponseText(currentItem.value.data as InterceptedResponse),
      messageType: 'response',
      label: t('trafficAnalysis.intercept.response'),
    })
    dialog.toast.success(t('trafficAnalysis.intercept.messages.sentToComparer'));
  }
}

function loadRequestContent(request: InterceptedRequest) {
  let content = `${request.method} ${request.path} ${request.protocol}\n`;
  for (const [key, value] of Object.entries(request.headers)) {
    content += `${key}: ${value}\n`;
  }
  if (request.body) {
    content += `\n${request.body}`;
  }
  requestContent.value = content;
}

function getInterceptResponseProtocol(response: InterceptedResponse) {
  const matchingRequest = interceptedRequests.value.find((request) => request.id === response.request_id)
  return normalizeHttpVersionToken(matchingRequest?.protocol)
}

function buildInterceptResponseText(response: InterceptedResponse) {
  let content = `${getInterceptResponseProtocol(response)} ${response.status}\n`;
  for (const [key, value] of Object.entries(response.headers)) {
    content += `${key}: ${value}\n`;
  }
  if (response.body) {
    content += `\n${response.body}`;
  }
  return content
}

function loadResponseContent(response: InterceptedResponse) {
  requestContent.value = buildInterceptResponseText(response);
}

function loadWebSocketContent(msg: InterceptedWebSocketMessage) {
  requestContent.value = msg.content || '';
}

// 加载当前项的内容
function loadCurrentItemContent() {
  const item = currentItem.value;
  if (!item) return;
  
  if (item.type === 'request') {
    loadRequestContent(item.data as InterceptedRequest);
  } else if (item.type === 'response') {
    loadResponseContent(item.data as InterceptedResponse);
  } else if (item.type === 'websocket') {
    loadWebSocketContent(item.data as InterceptedWebSocketMessage);
  }
}

// 选择队列中的项
function selectItem(index: number) {
  currentItemIndex.value = index;
  const item = interceptedItems.value[index];
  if (item) {
    currentItemType.value = item.type;
    loadCurrentItemContent();
  }
}

// 转发当前项
async function forwardCurrentItem() {
  const item = currentItem.value;
  if (!item || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    // 始终传递当前编辑的内容（默认可编辑）
    const modifiedContent = requestContent.value;
    
    if (item.type === 'request') {
      const response = await invoke<any>('forward_intercepted_request', { 
        requestId: item.data.id, modifiedContent
      });
      if (response.success) {
        interceptedRequests.value = interceptedRequests.value.filter(r => r.id !== item.data.id);
      } else {
        dialog.toast.error(response.error || '转发失败');
      }
    } else if (item.type === 'response') {
      const response = await invoke<any>('forward_intercepted_response', { 
        responseId: item.data.id, modifiedContent
      });
      if (response.success) {
        interceptedResponses.value = interceptedResponses.value.filter(r => r.id !== item.data.id);
      } else {
        dialog.toast.error(response.error || '转发失败');
      }
    } else if (item.type === 'websocket') {
      const response = await invoke<any>('forward_intercepted_websocket', { 
        id: item.data.id, content: modifiedContent
      });
      if (response.success) {
        interceptedWebsockets.value = interceptedWebsockets.value.filter(w => w.id !== item.data.id);
      } else {
        dialog.toast.error(response.error || '转发失败');
      }
    }
    
    // 更新索引
    if (currentItemIndex.value >= interceptedItems.value.length) {
      currentItemIndex.value = Math.max(0, interceptedItems.value.length - 1);
    }
    loadCurrentItemContent();
  } catch (error: any) {
    console.error('Failed to forward item:', error);
    dialog.toast.error(`转发失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

// 丢弃当前项
async function dropCurrentItem() {
  const item = currentItem.value;
  if (!item || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    if (item.type === 'request') {
      const response = await invoke<any>('drop_intercepted_request', { requestId: item.data.id });
      if (response.success) {
        interceptedRequests.value = interceptedRequests.value.filter(r => r.id !== item.data.id);
        dialog.toast.info('请求已丢弃');
      }
    } else if (item.type === 'response') {
      const response = await invoke<any>('drop_intercepted_response', { responseId: item.data.id });
      if (response.success) {
        interceptedResponses.value = interceptedResponses.value.filter(r => r.id !== item.data.id);
        dialog.toast.info('响应已丢弃');
      }
    } else if (item.type === 'websocket') {
      const response = await invoke<any>('drop_intercepted_websocket', { id: item.data.id });
      if (response.success) {
        interceptedWebsockets.value = interceptedWebsockets.value.filter(w => w.id !== item.data.id);
        dialog.toast.info('消息已丢弃');
      }
    }
    
    // 更新索引
    if (currentItemIndex.value >= interceptedItems.value.length) {
      currentItemIndex.value = Math.max(0, interceptedItems.value.length - 1);
    }
    loadCurrentItemContent();
  } catch (error: any) {
    console.error('Failed to drop item:', error);
    dialog.toast.error(`丢弃失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

async function refreshStatus() {
  try {
    const response = await invoke<any>('get_proxy_status');
    if (response.success && response.data) {
      proxyStatus.value = response.data;
    }
    
    // 请求拦截状态
    const interceptResponse = await invoke<any>('get_intercept_enabled');
    if (interceptResponse.success) {
      interceptEnabled.value = interceptResponse.data;
    }
    
    // 响应拦截状态
    const responseInterceptResponse = await invoke<any>('get_response_intercept_enabled');
    if (responseInterceptResponse.success) {
      responseInterceptEnabled.value = responseInterceptResponse.data;
    }

    // WebSocket 拦截状态
    const wsInterceptResponse = await invoke<any>('get_websocket_intercept_enabled');
    if (wsInterceptResponse.success) {
      websocketInterceptEnabled.value = wsInterceptResponse.data;
    }
  } catch (error: any) {
    console.error('Failed to refresh proxy status:', error);
  }
}

async function setupEventListeners() {
  // 监听代理状态事件
  unlistenProxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    proxyStatus.value = event.payload;
  });
  
  // 监听拦截请求事件
  unlistenInterceptRequest = await listen<InterceptedRequest>('intercept:request', (event) => {
    const request = event.payload;
    interceptedRequests.value.push(request);
    
    // 如果是第一个项目，自动加载内容
    if (interceptedItems.value.length === 1) {
      currentItemIndex.value = 0;
      currentItemType.value = 'request';
      loadRequestContent(request);
    }
  });
  
  // 监听拦截响应事件
  unlistenInterceptResponse = await listen<InterceptedResponse>('intercept:response', (event) => {
    const response = event.payload;
    interceptedResponses.value.push(response);
    
    if (interceptedItems.value.length === 1) {
      currentItemIndex.value = 0;
      currentItemType.value = 'response';
      loadResponseContent(response);
    }
  });

  // 监听拦截 WebSocket 事件
  unlistenInterceptWebSocket = await listen<InterceptedWebSocketMessage>('proxy:intercept_websocket', (event) => {
    const msg = event.payload;
    console.log('[ProxyIntercept] Received intercept websocket:', msg);
    interceptedWebsockets.value.push(msg);

    if (interceptedItems.value.length === 1) {
      currentItemIndex.value = 0;
      currentItemType.value = 'websocket';
      loadWebSocketContent(msg);
    }
  });
}

// 监听当前项变化，自动加载内容
watch(currentItem, (newItem) => {
  if (newItem) {
    loadCurrentItemContent();
  }
});

watch(
  () => interceptedItems.value.length,
  (count) => {
    emit('interceptQueueChanged', count);
  },
  { immediate: true },
);

// 生命周期
onMounted(async () => {
  await setupEventListeners();
  await refreshStatus();
});

onUnmounted(() => {
  if (unlistenProxyStatus) unlistenProxyStatus();
  if (unlistenInterceptRequest) unlistenInterceptRequest();
  if (unlistenInterceptResponse) unlistenInterceptResponse();
  if (unlistenInterceptWebSocket) unlistenInterceptWebSocket();
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
});

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  await refreshStatus();
});
</script>

<style scoped>
textarea {
  resize: none;
}
</style>
