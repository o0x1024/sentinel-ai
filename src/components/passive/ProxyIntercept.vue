<template>
  <div class="flex flex-col h-full">
    <!-- Intercept Controls Header -->
    <div class="bg-base-200 border-b border-base-300 p-3 flex-shrink-0">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="font-semibold text-base">
            <i class="fas fa-shield-alt mr-2"></i>
            {{ $t('passiveScan.intercept.title') }}
          </h2>
          <div class="badge badge-sm" :class="interceptEnabled ? 'badge-success' : 'badge-error'">
            <i :class="['fas fa-circle mr-2', interceptEnabled ? 'text-success-content' : 'text-error-content']"></i>
            {{ interceptEnabled ? $t('passiveScan.intercept.status.on') : $t('passiveScan.intercept.status.off') }}
          </div>
        </div>
        
        <div class="flex items-center gap-2">
          <button 
            @click="toggleIntercept"
            :class="['btn btn-sm', interceptEnabled ? 'btn-error' : 'btn-success']"
          >
            <i :class="['fas', interceptEnabled ? 'fa-stop' : 'fa-play', 'mr-1']"></i>
            {{ interceptEnabled ? $t('passiveScan.intercept.buttons.turnOff') : $t('passiveScan.intercept.buttons.turnOn') }}
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
            {{ interceptEnabled || responseInterceptEnabled ? $t('passiveScan.intercept.enabled') : $t('passiveScan.intercept.disabled') }}
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
              >
                <!-- 请求显示 -->
                <template v-if="item.type === 'request'">
                  <span class="badge badge-sm badge-outline">REQ</span>
                  <span class="badge badge-sm" :class="getMethodClass((item.data as any).method)">{{ (item.data as any).method }}</span>
                  <span class="truncate flex-1 font-mono text-xs">{{ (item.data as any).url }}</span>
                </template>
                <!-- 响应显示 -->
                <template v-else>
                  <span class="badge badge-sm badge-secondary">RES</span>
                  <span class="badge badge-sm" :class="getStatusClass((item.data as any).status)">{{ (item.data as any).status }}</span>
                  <span class="truncate flex-1 font-mono text-xs text-base-content/70">{{ $t('passiveScan.intercept.response') }} #{{ (item.data as any).request_id.slice(0, 8) }}</span>
                </template>
                <span class="text-xs text-base-content/50">{{ formatTimestamp(item.data.timestamp) }}</span>
              </div>
            </div>
          </div>
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
              @click="dropCurrentItem"
              class="btn btn-error btn-sm"
              :disabled="isProcessing || !currentItem"
            >
              <i class="fas fa-times mr-1"></i>
              {{ $t('passiveScan.intercept.buttons.drop') }}
            </button>
            
            <div class="divider divider-horizontal mx-1"></div>
            
            <button 
              @click="toggleEdit"
              class="btn btn-outline btn-sm"
              :class="{ 'btn-primary': isEditable }"
              :disabled="!currentItem"
            >
              <i :class="['fas', isEditable ? 'fa-check' : 'fa-edit', 'mr-1']"></i>
              {{ isEditable ? $t('passiveScan.intercept.buttons.save') : $t('passiveScan.intercept.buttons.edit') }}
            </button>
            
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
              <template v-else>
                {{ $t('passiveScan.intercept.response') }} {{ (currentItem.data as any).status }} - #{{ (currentItem.data as any).request_id.slice(0, 8) }}
              </template>
            </div>
          </div>

          <!-- Request Content Tabs -->
          <div class="tabs tabs-boxed bg-base-200 border-b border-base-300 px-3 py-1 flex-shrink-0">
            <a 
              :class="['tab tab-sm', activeTab === 'raw' ? 'tab-active' : '']"
              @click="activeTab = 'raw'"
            >
              {{ $t('passiveScan.intercept.tabs.raw') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'pretty' ? 'tab-active' : '']"
              @click="activeTab = 'pretty'"
            >
              {{ $t('passiveScan.intercept.tabs.pretty') }}
            </a>
            <a 
              :class="['tab tab-sm', activeTab === 'hex' ? 'tab-active' : '']"
              @click="activeTab = 'hex'"
            >
              {{ $t('passiveScan.intercept.tabs.hex') }}
            </a>
          </div>

          <!-- Request/Response Content -->
          <div class="flex-1 overflow-hidden bg-base-100 min-h-0 flex flex-col">
            <template v-if="currentItem">
              <!-- Raw View -->
              <div v-if="activeTab === 'raw'" class="flex-1 p-2 min-h-0 flex flex-col">
                <textarea 
                  v-model="requestContent"
                  :readonly="!isEditable"
                  class="w-full h-full font-mono text-sm leading-relaxed border border-base-300 rounded-lg p-3 focus:outline-none focus:border-primary resize-none"
                  :class="isEditable ? 'bg-base-100' : 'bg-base-200'"
                  spellcheck="false"
                  style="flex: 1; min-height: 0;"
                ></textarea>
              </div>

              <!-- Pretty View -->
              <div v-else-if="activeTab === 'pretty'" class="flex-1 overflow-auto p-4 space-y-4">
                <!-- 请求的 Pretty View -->
                <template v-if="currentItem.type === 'request'">
                  <div class="card bg-base-200">
                    <div class="card-body p-4">
                      <h3 class="font-semibold mb-2 text-sm">{{ $t('passiveScan.intercept.requestLine') }}</h3>
                      <div class="font-mono text-sm">
                        {{ (currentItem.data as any).method }} {{ (currentItem.data as any).path }} {{ (currentItem.data as any).protocol }}
                      </div>
                    </div>
                  </div>
                </template>
                
                <!-- 响应的 Pretty View -->
                <template v-else>
                  <div class="card bg-base-200">
                    <div class="card-body p-4">
                      <h3 class="font-semibold mb-2 text-sm">{{ $t('passiveScan.intercept.statusLine') }}</h3>
                      <div class="font-mono text-sm">
                        HTTP/1.1 <span :class="getStatusClass((currentItem.data as any).status)">{{ (currentItem.data as any).status }}</span>
                      </div>
                    </div>
                  </div>
                </template>

                <div class="card bg-base-200">
                  <div class="card-body p-4">
                    <h3 class="font-semibold mb-2 text-sm">{{ $t('passiveScan.intercept.headers') }}</h3>
                    <div class="space-y-1">
                      <div 
                        v-for="(value, key) in currentItem.data.headers" 
                        :key="key"
                        class="flex text-sm font-mono"
                      >
                        <span class="font-semibold text-primary w-48 flex-shrink-0">{{ key }}:</span>
                        <span class="flex-1 break-all">{{ value }}</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div v-if="currentItem.data.body" class="card bg-base-200">
                  <div class="card-body p-4">
                    <h3 class="font-semibold mb-2 text-sm">{{ $t('passiveScan.intercept.body') }}</h3>
                    <pre class="font-mono text-sm whitespace-pre-wrap break-all">{{ currentItem.data.body }}</pre>
                  </div>
                </div>
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 发送到 Repeater 的事件
const emit = defineEmits<{
  (e: 'sendToRepeater', request: InterceptedRequest): void
}>();

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

// 拦截项类型
type InterceptedItem = 
  | { type: 'request'; data: InterceptedRequest }
  | { type: 'response'; data: InterceptedResponse };

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
const interceptedRequests = ref<InterceptedRequest[]>([]);
const interceptedResponses = ref<InterceptedResponse[]>([]);
const currentItemIndex = ref(0);
const currentItemType = ref<'request' | 'response'>('request');
const activeTab = ref<'raw' | 'pretty' | 'hex'>('raw');
const isEditable = ref(false);
const isProcessing = ref(false);
const requestContent = ref('');

// 合并的拦截队列（请求和响应）
const interceptedItems = computed<InterceptedItem[]>(() => {
  const items: InterceptedItem[] = [];
  interceptedRequests.value.forEach(req => {
    items.push({ type: 'request', data: req });
  });
  interceptedResponses.value.forEach(resp => {
    items.push({ type: 'response', data: resp });
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
const currentRequestIndex = ref(0);
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

// 方法
async function toggleIntercept() {
  const newState = !interceptEnabled.value;
  
  try {
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

async function forwardRequest() {
  if (!currentRequest.value || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    const modifiedContent = isEditable.value ? requestContent.value : undefined;
    const response = await invoke<any>('forward_intercepted_request', { 
      requestId: currentRequest.value.id,
      modifiedContent
    });
    
    if (response.success) {
      removeCurrentRequest();
      isEditable.value = false;
    } else {
      dialog.toast.error(response.error || '转发失败');
    }
  } catch (error: any) {
    console.error('Failed to forward request:', error);
    dialog.toast.error(`转发请求失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

async function dropRequest() {
  if (!currentRequest.value || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    const response = await invoke<any>('drop_intercepted_request', { 
      requestId: currentRequest.value.id 
    });
    
    if (response.success) {
      removeCurrentRequest();
      isEditable.value = false;
      dialog.toast.info('请求已丢弃');
    } else {
      dialog.toast.error(response.error || '丢弃失败');
    }
  } catch (error: any) {
    console.error('Failed to drop request:', error);
    dialog.toast.error(`丢弃请求失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

async function forwardAll() {
  if (isProcessing.value || interceptedRequests.value.length === 0) return;
  
  isProcessing.value = true;
  const total = interceptedRequests.value.length;
  let forwarded = 0;
  
  try {
    for (const req of [...interceptedRequests.value]) {
      const response = await invoke<any>('forward_intercepted_request', { 
        requestId: req.id,
        modifiedContent: undefined
      });
      if (response.success) {
        forwarded++;
      }
    }
    interceptedRequests.value = [];
    currentRequestIndex.value = 0;
    dialog.toast.success(`已转发 ${forwarded}/${total} 个请求`);
  } catch (error: any) {
    console.error('Failed to forward all requests:', error);
    dialog.toast.error(`批量转发失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

async function dropAll() {
  if (isProcessing.value || interceptedRequests.value.length === 0) return;
  
  const confirmed = await dialog.confirm(`确定要丢弃所有 ${interceptedRequests.value.length} 个请求吗？`);
  if (!confirmed) return;
  
  isProcessing.value = true;
  const total = interceptedRequests.value.length;
  let dropped = 0;
  
  try {
    for (const req of [...interceptedRequests.value]) {
      const response = await invoke<any>('drop_intercepted_request', { 
        requestId: req.id 
      });
      if (response.success) {
        dropped++;
      }
    }
    interceptedRequests.value = [];
    currentRequestIndex.value = 0;
    dialog.toast.info(`已丢弃 ${dropped}/${total} 个请求`);
  } catch (error: any) {
    console.error('Failed to drop all requests:', error);
    dialog.toast.error(`批量丢弃失败: ${error}`);
  } finally {
    isProcessing.value = false;
  }
}

function toggleEdit() {
  isEditable.value = !isEditable.value;
  if (!isEditable.value) {
    dialog.toast.info('编辑已保存，点击 Forward 发送修改后的请求');
  }
}

function sendToRepeater() {
  if (!currentRequest.value) return;
  emit('sendToRepeater', currentRequest.value);
}

function selectRequest(index: number) {
  currentRequestIndex.value = index;
  if (currentRequest.value) {
    loadRequestContent(currentRequest.value);
  }
}

function removeCurrentRequest() {
  if (interceptedRequests.value.length === 0) return;
  
  interceptedRequests.value = interceptedRequests.value.filter(
    (_, i) => i !== currentRequestIndex.value
  );
  
  if (currentRequestIndex.value >= interceptedRequests.value.length) {
    currentRequestIndex.value = Math.max(0, interceptedRequests.value.length - 1);
  }
  
  if (currentRequest.value) {
    loadRequestContent(currentRequest.value);
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

// 加载当前项的内容
function loadCurrentItemContent() {
  const item = currentItem.value;
  if (!item) return;
  
  if (item.type === 'request') {
    loadRequestContent(item.data as InterceptedRequest);
  } else {
    loadResponseContent(item.data as InterceptedResponse);
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

// 转发当前项（请求或响应）
async function forwardCurrentItem() {
  const item = currentItem.value;
  if (!item || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    const modifiedContent = isEditable.value ? requestContent.value : undefined;
    
    if (item.type === 'request') {
      const response = await invoke<any>('forward_intercepted_request', { 
        requestId: item.data.id,
        modifiedContent
      });
      
      if (response.success) {
        interceptedRequests.value = interceptedRequests.value.filter(r => r.id !== item.data.id);
        isEditable.value = false;
      } else {
        dialog.toast.error(response.error || '转发失败');
      }
    } else {
      const response = await invoke<any>('forward_intercepted_response', { 
        responseId: item.data.id,
        modifiedContent
      });
      
      if (response.success) {
        interceptedResponses.value = interceptedResponses.value.filter(r => r.id !== item.data.id);
        isEditable.value = false;
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

// 丢弃当前项（请求或响应）
async function dropCurrentItem() {
  const item = currentItem.value;
  if (!item || isProcessing.value) return;
  
  isProcessing.value = true;
  try {
    if (item.type === 'request') {
      const response = await invoke<any>('drop_intercepted_request', { 
        requestId: item.data.id 
      });
      
      if (response.success) {
        interceptedRequests.value = interceptedRequests.value.filter(r => r.id !== item.data.id);
        isEditable.value = false;
        dialog.toast.info('请求已丢弃');
      } else {
        dialog.toast.error(response.error || '丢弃失败');
      }
    } else {
      const response = await invoke<any>('drop_intercepted_response', { 
        responseId: item.data.id 
      });
      
      if (response.success) {
        interceptedResponses.value = interceptedResponses.value.filter(r => r.id !== item.data.id);
        isEditable.value = false;
        dialog.toast.info('响应已丢弃');
      } else {
        dialog.toast.error(response.error || '丢弃失败');
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
    
    // 同步获取请求拦截状态
    const interceptResponse = await invoke<any>('get_intercept_enabled');
    if (interceptResponse.success) {
      interceptEnabled.value = interceptResponse.data;
    }
    
    // 同步获取响应拦截状态
    const responseInterceptResponse = await invoke<any>('get_response_intercept_enabled');
    if (responseInterceptResponse.success) {
      responseInterceptEnabled.value = responseInterceptResponse.data;
    }
  } catch (error: any) {
    console.error('Failed to refresh proxy status:', error);
  }
}

// 设置事件监听
let unlistenInterceptResponse: (() => void) | null = null;

async function setupEventListeners() {
  // 监听代理状态事件
  unlistenProxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    proxyStatus.value = event.payload;
  });
  
  // 监听拦截请求事件
  unlistenInterceptRequest = await listen<InterceptedRequest>('intercept:request', (event) => {
    const request = event.payload;
    console.log('[ProxyIntercept] Received intercept request:', request);
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
    console.log('[ProxyIntercept] Received intercept response:', response);
    interceptedResponses.value.push(response);
    
    // 如果是第一个项目，自动加载内容
    if (interceptedItems.value.length === 1) {
      currentItemIndex.value = 0;
      currentItemType.value = 'response';
      loadResponseContent(response);
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
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
});

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyIntercept] Refresh triggered by parent');
  await refreshStatus();
});
</script>

<style scoped>
textarea {
  resize: none;
}
</style>
