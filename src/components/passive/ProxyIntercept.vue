<template>
  <div class="flex flex-col h-full" @contextmenu.prevent>
    <!-- Intercept Controls Header -->
    <div class="bg-base-200 border-b border-base-300 p-3 flex-shrink-0">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="font-semibold text-base">
            <i class="fas fa-shield-alt mr-2"></i>
            {{ $t('passiveScan.intercept.title') }}
          </h2>
          <!-- HTTP Intercept Status -->
          <div class="tooltip" :data-tip="$t('passiveScan.intercept.tooltip.http')">
            <div class="badge badge-sm cursor-help" :class="interceptEnabled ? 'badge-success' : 'badge-neutral'">
              <i :class="['fas fa-circle mr-2', interceptEnabled ? 'text-success-content' : 'text-neutral-content']"></i>
              HTTP
            </div>
          </div>
          <!-- WebSocket Intercept Status -->
          <div class="tooltip" :data-tip="$t('passiveScan.intercept.tooltip.websocket')">
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
            :title="$t('passiveScan.intercept.buttons.toggleHttp')"
          >
            <i :class="['fas', interceptEnabled ? 'fa-stop' : 'fa-play', 'mr-1']"></i>
            HTTP
          </button>

          <!-- WebSocket Toggle -->
          <button 
            @click="toggleWebSocketIntercept"
            :class="['btn btn-sm', websocketInterceptEnabled ? 'btn-error' : 'btn-success']"
            :title="$t('passiveScan.intercept.buttons.toggleWs')"
          >
            <i :class="['fas', websocketInterceptEnabled ? 'fa-stop' : 'fa-plug', 'mr-1']"></i>
            WS
          </button>
          
          <div class="divider divider-horizontal mx-0"></div>
          
          <div class="stats stats-horizontal shadow-sm">
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('passiveScan.intercept.stats.proxyStatus') }}</div>
              <div class="stat-value text-sm" :class="proxyStatus.running ? 'text-success' : 'text-error'">
                {{ proxyStatus.running ? $t('passiveScan.intercept.stats.running') : $t('passiveScan.intercept.stats.stopped') }}
              </div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('passiveScan.intercept.stats.port') }}</div>
              <div class="stat-value text-sm">{{ proxyStatus.port || 8080 }}</div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">{{ $t('passiveScan.intercept.stats.interceptQueue') }}</div>
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
          <p class="text-lg font-semibold text-base-content/70">{{ $t('passiveScan.intercept.waiting') }}</p>
          <p class="text-sm text-base-content/50 mt-2">
            {{ interceptEnabled || websocketInterceptEnabled ? $t('passiveScan.intercept.enabled') : $t('passiveScan.intercept.disabled') }}
          </p>
          <p class="text-sm text-base-content/50">
            {{ $t('passiveScan.intercept.proxyConfig') }} 127.0.0.1:{{ proxyStatus.port || 8080 }}
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
              {{ $t('passiveScan.intercept.queue') }} ({{ interceptedItems.length }})
            </h3>
            <div class="flex items-center gap-2">
              <button 
                @click="forwardAll"
                class="btn btn-xs btn-outline"
                :disabled="isProcessing || interceptedItems.length === 0"
              >
                <i class="fas fa-forward mr-1"></i>
                {{ $t('passiveScan.intercept.buttons.forwardAll') }}
              </button>
              <button 
                @click="dropAll"
                class="btn btn-xs btn-outline btn-error"
                :disabled="isProcessing || interceptedItems.length === 0"
              >
                <i class="fas fa-trash mr-1"></i>
                {{ $t('passiveScan.intercept.buttons.dropAll') }}
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
                  <span class="truncate flex-1 font-mono text-xs text-base-content/70">{{ $t('passiveScan.intercept.response') }} #{{ (item.data as any).request_id.slice(0, 8) }}</span>
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
            {{ $t('passiveScan.intercept.buttons.forward') }}
          </button>
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuDrop"
          >
            <i class="fas fa-times w-4 text-error"></i>
            {{ $t('passiveScan.intercept.buttons.drop') }}
          </button>
          
          <div class="divider my-1 h-px"></div>
          
          <!-- Send to -->
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuSendToRepeater"
            :disabled="contextMenu.item?.type !== 'request'"
            :class="{ 'opacity-50 cursor-not-allowed': contextMenu.item?.type !== 'request' }"
          >
            <i class="fas fa-redo w-4 text-primary"></i>
            {{ $t('passiveScan.intercept.contextMenu.sendToRepeater') }}
          </button>
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuSendToAI"
          >
            <i class="fas fa-robot w-4 text-secondary"></i>
            {{ $t('passiveScan.intercept.contextMenu.sendToAI') }}
          </button>
          
          <div class="divider my-1 h-px"></div>
          
          <!-- Filter submenu -->
          <div class="relative group">
            <button 
              class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2 justify-between"
            >
              <span class="flex items-center gap-2">
                <i class="fas fa-filter w-4 text-warning"></i>
                {{ $t('passiveScan.intercept.contextMenu.addFilter') }}
              </span>
              <i class="fas fa-chevron-right text-xs"></i>
            </button>
            <!-- Filter submenu -->
            <div class="absolute left-full top-0 ml-1 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-56 hidden group-hover:block">
              <template v-if="contextMenu.item?.type === 'request'">
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByDomain"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByDomain') }}: {{ getItemDomain() }}
                </button>
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByUrl"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByUrl') }}
                </button>
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByMethod"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByMethod') }}: {{ getItemMethod() }}
                </button>
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByFileExt"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByFileExt') }}
                </button>
              </template>
              <template v-else-if="contextMenu.item?.type === 'response'">
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByStatus"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByStatus') }}: {{ getItemStatus() }}
                </button>
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByContentType"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByContentType') }}
                </button>
              </template>
              <template v-else-if="contextMenu.item?.type === 'websocket'">
                <button 
                  class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                  @click="addFilterByWsDirection"
                >
                  {{ $t('passiveScan.intercept.contextMenu.filterByDirection') }}: {{ getItemDirection() }}
                </button>
              </template>
              <div class="divider my-1 h-px"></div>
              <button 
                class="w-full px-4 py-2 text-left text-sm hover:bg-base-200"
                @click="openFilterDialog"
              >
                <i class="fas fa-cog mr-2"></i>
                {{ $t('passiveScan.intercept.contextMenu.customFilter') }}
              </button>
            </div>
          </div>
          
          <div class="divider my-1 h-px"></div>
          
          <!-- Copy -->
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuCopyUrl"
            v-if="contextMenu.item?.type === 'request'"
          >
            <i class="fas fa-copy w-4"></i>
            {{ $t('passiveScan.intercept.contextMenu.copyUrl') }}
          </button>
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuCopyAsCurl"
            v-if="contextMenu.item?.type === 'request'"
          >
            <i class="fas fa-terminal w-4"></i>
            {{ $t('passiveScan.intercept.contextMenu.copyAsCurl') }}
          </button>
          <button 
            class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
            @click="contextMenuCopyRaw"
          >
            <i class="fas fa-file-alt w-4"></i>
            {{ $t('passiveScan.intercept.contextMenu.copyRaw') }}
          </button>
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
              {{ $t('passiveScan.intercept.buttons.forward') }}
            </button>
            
            <button 
              @click="forwardAll"
              class="btn btn-success btn-sm btn-outline"
              :disabled="isProcessing || interceptedItems.length === 0"
            >
              <i class="fas fa-forward mr-1"></i>
              {{ $t('passiveScan.intercept.buttons.forwardAll') }}
            </button>
            
            <button 
              @click="dropCurrentItem"
              class="btn btn-error btn-sm"
              :disabled="isProcessing || !currentItem"
            >
              <i class="fas fa-times mr-1"></i>
              {{ $t('passiveScan.intercept.buttons.drop') }}
            </button>
            
            <div class="divider divider-horizontal mx-1"></div>
            
            <button 
              @click="sendToRepeater"
              class="btn btn-outline btn-sm"
              :disabled="!currentItem || currentItem.type !== 'request'"
            >
              <i class="fas fa-redo mr-1"></i>
              {{ $t('passiveScan.intercept.buttons.sendToRepeater') }}
            </button>
            
            <div class="flex-1"></div>
            
            <div v-if="currentItem" class="text-xs text-base-content/70 truncate max-w-md">
              <i class="fas fa-info-circle mr-1"></i>
              <template v-if="currentItem.type === 'request'">
                {{ (currentItem.data as any).method }} {{ (currentItem.data as any).url }}
              </template>
              <template v-else-if="currentItem.type === 'response'">
                {{ $t('passiveScan.intercept.response') }} {{ (currentItem.data as any).status }}
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
              {{ $t('passiveScan.intercept.tabs.pretty') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'raw' ? 'tab-active' : '']"
              @click="activeTab = 'raw'"
            >
              {{ $t('passiveScan.intercept.tabs.raw') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'hex' ? 'tab-active' : '']"
              @click="activeTab = 'hex'"
            >
              {{ $t('passiveScan.intercept.tabs.hex') }}
            </a>
          </div>

          <!-- Content View -->
          <div class="flex-1 overflow-hidden bg-base-100 min-h-0 flex flex-col">
            <template v-if="currentItem">
              <!-- Raw View -->
              <div v-if="activeTab === 'raw'" class="flex-1 min-h-0 flex flex-col">
                <HttpCodeEditor
                  v-model="requestContent"
                  :readonly="!isEditable"
                  height="100%"
                />
              </div>

              <!-- Pretty View -->
              <div v-else-if="activeTab === 'pretty'" class="flex-1 min-h-0 flex flex-col">
                <HttpCodeEditor
                  v-model="prettyContent"
                  :readonly="!isEditable"
                  height="100%"
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
                <p>{{ $t('passiveScan.intercept.clickToView') }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Filter Rule Dialog -->
    <dialog ref="filterDialogRef" class="modal">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-4">
          <i class="fas fa-filter mr-2"></i>
          {{ $t('passiveScan.intercept.filterDialog.title') }}
        </h3>
        
        <div class="space-y-4">
          <!-- Filter Type -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('passiveScan.intercept.filterDialog.filterType') }}</span>
            </label>
            <select v-model="filterRule.type" class="select select-bordered w-full">
              <option value="request">{{ $t('passiveScan.intercept.filterDialog.typeRequest') }}</option>
              <option value="response">{{ $t('passiveScan.intercept.filterDialog.typeResponse') }}</option>
            </select>
          </div>
          
          <!-- Match Type -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('passiveScan.intercept.filterDialog.matchType') }}</span>
            </label>
            <select v-model="filterRule.matchType" class="select select-bordered w-full">
              <template v-if="filterRule.type === 'request'">
                <option value="domain">{{ $t('passiveScan.intercept.filterDialog.matchDomain') }}</option>
                <option value="url">{{ $t('passiveScan.intercept.filterDialog.matchUrl') }}</option>
                <option value="method">{{ $t('passiveScan.intercept.filterDialog.matchMethod') }}</option>
                <option value="fileExt">{{ $t('passiveScan.intercept.filterDialog.matchFileExt') }}</option>
                <option value="header">{{ $t('passiveScan.intercept.filterDialog.matchHeader') }}</option>
              </template>
              <template v-else>
                <option value="status">{{ $t('passiveScan.intercept.filterDialog.matchStatus') }}</option>
                <option value="contentType">{{ $t('passiveScan.intercept.filterDialog.matchContentType') }}</option>
                <option value="header">{{ $t('passiveScan.intercept.filterDialog.matchHeader') }}</option>
              </template>
            </select>
          </div>
          
          <!-- Relationship -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('passiveScan.intercept.filterDialog.relationship') }}</span>
            </label>
            <select v-model="filterRule.relationship" class="select select-bordered w-full">
              <option value="matches">{{ $t('passiveScan.intercept.filterDialog.matches') }}</option>
              <option value="notMatches">{{ $t('passiveScan.intercept.filterDialog.notMatches') }}</option>
              <option value="contains">{{ $t('passiveScan.intercept.filterDialog.contains') }}</option>
              <option value="notContains">{{ $t('passiveScan.intercept.filterDialog.notContains') }}</option>
            </select>
          </div>
          
          <!-- Condition -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('passiveScan.intercept.filterDialog.condition') }}</span>
            </label>
            <input 
              type="text" 
              v-model="filterRule.condition"
              class="input input-bordered w-full"
              :placeholder="$t('passiveScan.intercept.filterDialog.conditionPlaceholder')"
            />
          </div>
          
          <!-- Action -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('passiveScan.intercept.filterDialog.action') }}</span>
            </label>
            <select v-model="filterRule.action" class="select select-bordered w-full">
              <option value="exclude">{{ $t('passiveScan.intercept.filterDialog.actionExclude') }}</option>
              <option value="include">{{ $t('passiveScan.intercept.filterDialog.actionInclude') }}</option>
            </select>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeFilterDialog">
            {{ $t('passiveScan.intercept.filterDialog.cancel') }}
          </button>
          <button class="btn btn-primary" @click="saveFilterRule" :disabled="!filterRule.condition">
            {{ $t('passiveScan.intercept.filterDialog.save') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit as tauriEmit } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import HttpCodeEditor from '@/components/HttpCodeEditor.vue';

const { t } = useI18n();
const router = useRouter();

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 发送到 Repeater 的事件
const emit = defineEmits<{
  (e: 'sendToRepeater', request: InterceptedRequest): void
  (e: 'sendToAssistant', requests: any[]): void
}>();

// 用于发送到 AI 助手的请求格式（兼容 ProxyHistory）
interface ProxyRequestForAI {
  id: number;
  url: string;
  host: string;
  protocol: string;
  method: string;
  status_code: number;
  request_headers?: string;
  request_body?: string;
  response_headers?: string;
  response_body?: string;
  response_size: number;
  response_time: number;
  timestamp: string;
}

// 类型定义
interface ProxyStats {
  http_requests: number;
  https_requests: number;
  errors: number;
  qps: number;
}

interface ProxyStatus {
  running: boolean;
  port: number;
  mitm: boolean;
  stats: ProxyStats;
}

interface InterceptedRequest {
  id: string;
  method: string;
  url: string;
  path: string;
  protocol: string;
  headers: Record<string, string>;
  body?: string;
  timestamp: number;
}

interface InterceptedResponse {
  id: string;
  request_id: string;
  status: number;
  headers: Record<string, string>;
  body?: string;
  timestamp: number;
}

interface InterceptedWebSocketMessage {
  id: string;
  connection_id: string;
  direction: 'client_to_server' | 'server_to_client';
  message_type: 'text' | 'binary' | 'ping' | 'pong' | 'close';
  content?: string;
  timestamp: number;
}

// 拦截项类型
type InterceptedItem = 
  | { type: 'request'; data: InterceptedRequest }
  | { type: 'response'; data: InterceptedResponse }
  | { type: 'websocket'; data: InterceptedWebSocketMessage };

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

const interceptedRequests = ref<InterceptedRequest[]>([]);
const interceptedResponses = ref<InterceptedResponse[]>([]);
const interceptedWebsockets = ref<InterceptedWebSocketMessage[]>([]);

const currentItemIndex = ref(0);
const currentItemType = ref<'request' | 'response' | 'websocket'>('request');
const activeTab = ref<'raw' | 'pretty' | 'hex'>('pretty');
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

// 拖拽调整高度
const queuePanelHeight = ref(180);
let isResizing = false;
let startY = 0;
let startHeight = 0;

// Format content for Pretty view (JSON, XML, etc.)
function formatBody(body: string): string {
  if (!body) return body;
  
  const trimmed = body.trim();
  
  // Try JSON
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try {
      const parsed = JSON.parse(trimmed);
      return JSON.stringify(parsed, null, 2);
    } catch {
      // Not valid JSON
    }
  }
  
  // Try XML/HTML
  if (trimmed.startsWith('<')) {
    try {
      return formatXml(trimmed);
    } catch {
      // Not valid XML
    }
  }
  
  return body;
}

// Simple XML formatter
function formatXml(xml: string): string {
  let formatted = '';
  let indent = 0;
  const lines = xml.replace(/></g, '>\n<').split('\n');
  
  for (const line of lines) {
    const trimmedLine = line.trim();
    if (!trimmedLine) continue;
    
    // Closing tag
    if (trimmedLine.startsWith('</')) {
      indent = Math.max(0, indent - 1);
    }
    
    formatted += '  '.repeat(indent) + trimmedLine + '\n';
    
    // Opening tag (not self-closing, not closing)
    if (trimmedLine.startsWith('<') && !trimmedLine.startsWith('</') && 
        !trimmedLine.startsWith('<?') && !trimmedLine.startsWith('<!') &&
        !trimmedLine.endsWith('/>') && !trimmedLine.includes('</')) {
      indent++;
    }
  }
  
  return formatted.trim();
}

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

// 格式化时间戳
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('zh-CN');
}

function truncate(str: string, len: number) {
  if (!str) return '';
  return str.length > len ? str.substring(0, len) + '...' : str;
}

// Context Menu Methods
function showContextMenu(event: MouseEvent, item: InterceptedItem, index: number) {
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

function closeContextMenu() {
  contextMenu.value.visible = false;
  document.removeEventListener('click', closeContextMenu);
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
    emit('sendToRepeater', contextMenu.value.item.data as InterceptedRequest);
    dialog.toast.success('Sent to Repeater');
  }
  closeContextMenu();
}

async function contextMenuSendToAI() {
  const item = contextMenu.value.item;
  if (!item) {
    closeContextMenu();
    return;
  }
  
  // Convert intercepted item to ProxyRequest format for AI assistant
  const proxyRequest = convertToProxyRequest(item);
  if (!proxyRequest) {
    closeContextMenu();
    return;
  }
  
  // Determine send type based on item type
  const sendType = item.type === 'response' ? 'response' : 'request';
  
  closeContextMenu();
  
  // Navigate to AI assistant page first
  await router.push('/ai-assistant');
  
  // Wait for the page to mount and set up event listener
  await new Promise(resolve => setTimeout(resolve, 100));
  
  // Send event to AI assistant
  await tauriEmit('traffic:send-to-assistant', { 
    requests: [proxyRequest], 
    type: sendType 
  });
  emit('sendToAssistant', [proxyRequest]);
  
  const typeText = sendType === 'request' ? t('passiveScan.intercept.request') : t('passiveScan.intercept.response');
  dialog.toast.success(t('passiveScan.intercept.sentToAssistant', { type: typeText }));
}

// Convert intercepted item to ProxyRequest format for AI assistant
function convertToProxyRequest(item: InterceptedItem): ProxyRequestForAI | null {
  if (item.type === 'request') {
    const req = item.data as InterceptedRequest;
    let host = '';
    try {
      const url = new URL(req.url);
      host = url.hostname;
    } catch {
      host = '';
    }
    
    return {
      id: Date.now(),
      url: req.url,
      host,
      protocol: req.protocol || 'HTTP/1.1',
      method: req.method,
      status_code: 0,
      request_headers: JSON.stringify(req.headers),
      request_body: req.body,
      response_headers: undefined,
      response_body: undefined,
      response_size: 0,
      response_time: 0,
      timestamp: new Date(req.timestamp).toISOString()
    };
  } else if (item.type === 'response') {
    const res = item.data as InterceptedResponse;
    return {
      id: Date.now(),
      url: '', // Response doesn't have URL directly
      host: '',
      protocol: 'HTTP/1.1',
      method: '',
      status_code: res.status,
      request_headers: undefined,
      request_body: undefined,
      response_headers: JSON.stringify(res.headers),
      response_body: res.body,
      response_size: res.body?.length || 0,
      response_time: 0,
      timestamp: new Date(res.timestamp).toISOString()
    };
  } else if (item.type === 'websocket') {
    const ws = item.data as InterceptedWebSocketMessage;
    return {
      id: Date.now(),
      url: `ws://${ws.connection_id}`,
      host: '',
      protocol: 'WebSocket',
      method: ws.direction === 'client_to_server' ? 'WS_SEND' : 'WS_RECV',
      status_code: 0,
      request_headers: undefined,
      request_body: ws.content,
      response_headers: undefined,
      response_body: undefined,
      response_size: ws.content?.length || 0,
      response_time: 0,
      timestamp: new Date(ws.timestamp).toISOString()
    };
  }
  return null;
}

// Get item properties for context menu
function getItemDomain(): string {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    try {
      const url = new URL((item.data as InterceptedRequest).url);
      return url.hostname;
    } catch {
      return '';
    }
  }
  return '';
}

function getItemMethod(): string {
  const item = contextMenu.value.item;
  if (item?.type === 'request') {
    return (item.data as InterceptedRequest).method;
  }
  return '';
}

function getItemStatus(): number {
  const item = contextMenu.value.item;
  if (item?.type === 'response') {
    return (item.data as InterceptedResponse).status;
  }
  return 0;
}

function getItemDirection(): string {
  const item = contextMenu.value.item;
  if (item?.type === 'websocket') {
    return (item.data as InterceptedWebSocketMessage).direction === 'client_to_server' 
      ? 'Client → Server' 
      : 'Server → Client';
  }
  return '';
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
    dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
    dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
    dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
      dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
    dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
      dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
    
    dialog.toast.success(t('passiveScan.intercept.filterDialog.ruleAdded'));
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
  emit('sendToRepeater', currentRequest.value);
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

function loadResponseContent(response: InterceptedResponse) {
  let content = `HTTP/1.1 ${response.status}\n`;
  for (const [key, value] of Object.entries(response.headers)) {
    content += `${key}: ${value}\n`;
  }
  if (response.body) {
    content += `\n${response.body}`;
  }
  requestContent.value = content;
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

function getMethodClass(method: string) {
  switch (method.toUpperCase()) {
    case 'GET': return 'badge-info';
    case 'POST': return 'badge-success';
    case 'PUT': return 'badge-warning';
    case 'DELETE': return 'badge-error';
    case 'PATCH': return 'badge-accent';
    default: return 'badge-ghost';
  }
}

function getStatusClass(status: number) {
  if (status >= 200 && status < 300) return 'badge-success';
  if (status >= 300 && status < 400) return 'badge-info';
  if (status >= 400 && status < 500) return 'badge-warning';
  if (status >= 500) return 'badge-error';
  return 'badge-ghost';
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
