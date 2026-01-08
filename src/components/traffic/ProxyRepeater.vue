<template>
  <div class="flex flex-col h-full bg-base-100" @contextmenu.prevent>
    <!-- 右键菜单 -->
    <div 
      v-if="contextMenu.visible"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click.stop
    >
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSend"
      >
        <i class="fas fa-paper-plane text-primary"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.sendRequest') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendToNewTab"
      >
        <i class="fas fa-plus text-success"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.sendToNewTab') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyUrl"
      >
        <i class="fas fa-link text-info"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.copyUrl') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyRequest"
      >
        <i class="fas fa-copy text-secondary"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.copyRequest') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyCurl"
      >
        <i class="fas fa-terminal text-warning"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.copyAsCurl') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendRequestToAssistant"
      >
        <i class="fas fa-upload text-accent"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.sendRequestToAssistant') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendResponseToAssistant"
        :disabled="!currentTab?.response"
      >
        <i class="fas fa-download text-accent"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.sendResponseToAssistant') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuPaste"
      >
        <i class="fas fa-paste text-accent"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.paste') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuClear"
      >
        <i class="fas fa-eraser text-error"></i>
        {{ $t('trafficAnalysis.repeater.contextMenu.clear') }}
      </button>
    </div>
    <!-- Tabs Header -->
    <div class="bg-base-200 border-b border-base-300 px-2 py-1 flex items-center gap-2">
      <div class="flex items-center gap-1 overflow-x-auto flex-1">
        <div 
          v-for="(tab, index) in tabs" 
          :key="tab.id"
          class="flex items-center gap-2 px-3 py-1.5 rounded cursor-pointer border border-base-300 text-sm min-w-16"
          :class="activeTabIndex === index ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
          @click="selectTab(index)"
        >
          <span class="truncate" :title="tab.name">{{ index + 1 }}</span>
          <button 
            @click.stop="closeTab(index)"
            class="w-4 h-4 ml-2 flex items-center justify-center rounded opacity-40 hover:opacity-100 hover:bg-base-300 transition-opacity"
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
    </div>

    <!-- Tab Content -->
    <div v-if="currentTab" class="flex-1 flex flex-col overflow-hidden">
      <!-- Toolbar -->
      <div class="bg-base-200 px-3 py-2 border-b border-base-300 flex items-center gap-3">
        <button 
          @click="sendRequest"
          class="btn btn-primary btn-sm"
          :disabled="isSending || !currentTab.targetHost"
        >
          <i :class="['fas', isSending ? 'fa-spinner fa-spin' : 'fa-paper-plane']"></i>
          {{ $t('trafficAnalysis.repeater.contextMenu.sendRequest') }}
        </button>
        <button 
          @click="cancelRequest"
          class="btn btn-ghost btn-sm"
          :disabled="!isSending"
        >
          <i class="fas fa-stop"></i>
          {{ $t('trafficAnalysis.repeater.contextMenu.cancel') }}
        </button>
        
        <div class="flex-1"></div>
        
        <!-- Target 显示 -->
        <div class="flex items-center gap-2 text-sm">
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
          
        <span v-if="currentTab.useTls" class="badge badge-sm badge-outline">HTTP/2</span>
      </div>
      
      <!-- Target 配置对话框 -->
      <dialog :class="['modal', showTargetDialog ? 'modal-open' : '']">
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
      </dialog>

      <!-- Request / Response Panels -->
      <div class="flex-1 flex overflow-hidden" :class="layoutMode === 'horizontal' ? 'flex-row' : 'flex-col'">
        <!-- Request Panel -->
        <div 
          class="request-panel flex flex-col overflow-hidden border-base-300"
          :class="layoutMode === 'horizontal' ? 'border-r' : 'border-b'"
          :style="layoutMode === 'horizontal' 
            ? { width: leftPanelWidth + 'px' } 
            : { height: topPanelHeight + 'px' }"
        >
          <!-- Request Header -->
          <div class="bg-base-200 px-3 py-1 flex items-center justify-between border-b border-base-300">
            <span class="font-semibold text-sm">{{ $t('trafficAnalysis.repeater.contextMenu.request') }}</span>
            <div class="tabs tabs-boxed tabs-xs bg-base-300">
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'pretty' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'pretty'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.pretty') }}</a>
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'raw' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'raw'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.raw') }}</a>
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'hex' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'hex'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.hex') }}</a>
            </div>
          </div>

          <!-- Request Content -->
          <div class="flex-1 overflow-hidden" @contextmenu.prevent="showContextMenu($event)">
            <template v-if="currentTab.requestTab === 'pretty'">
              <HttpCodeEditor
                ref="requestEditor"
                :modelValue="formatPrettyRequest()"
                @update:modelValue="onPrettyRequestUpdate"
                :readonly="false"
                height="100%"
              />
            </template>
            <template v-else-if="currentTab.requestTab === 'raw'">
              <HttpCodeEditor
                ref="requestEditor"
                v-model="currentTab.rawRequest"
                :readonly="false"
                height="100%"
              />
            </template>
            <template v-else>
              <div class="h-full overflow-auto p-2 font-mono text-xs bg-base-100">
                <pre>{{ toHex(currentTab.rawRequest) }}</pre>
              </div>
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
        <div class="response-panel flex-1 flex flex-col overflow-hidden min-h-0 min-w-0">
          <!-- Response Header -->
          <div class="bg-base-200 px-3 py-1 flex items-center justify-between border-b border-base-300">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-sm">{{ $t('trafficAnalysis.repeater.contextMenu.response') }}</span>
              <template v-if="currentTab.response">
                <span 
                  class="badge badge-sm"
                  :class="getStatusClass(currentTab.response.statusCode)"
                >
                  {{ currentTab.response.statusCode }}
                </span>
              </template>
            </div>
            <div class="tabs tabs-boxed tabs-xs bg-base-300">
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'pretty' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'pretty'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.pretty') }}</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'raw' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'raw'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.raw') }}</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'hex' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'hex'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.hex') }}</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'render' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'render'"
              >{{ $t('trafficAnalysis.repeater.contextMenu.render') }}</a>
            </div>
          </div>

          <!-- Response Content -->
          <div class="flex-1 overflow-hidden" @contextmenu.prevent>
            <template v-if="currentTab.response || currentTab.rawResponse">
              <!-- Pretty/Raw View -->
              <template v-if="currentTab.responseTab === 'pretty' || currentTab.responseTab === 'raw'">
                <HttpCodeEditor
                  ref="responseEditor"
                  :modelValue="currentTab.responseTab === 'pretty' ? formatPrettyResponse() : currentTab.rawResponse"
                  :readonly="true"
                  height="100%"
                />
              </template>
              
              <!-- Hex View -->
              <template v-else-if="currentTab.responseTab === 'hex'">
                <div class="h-full overflow-auto p-2 font-mono text-xs bg-base-100">
                  <pre>{{ toHex(currentTab.rawResponse) }}</pre>
                </div>
              </template>
              
              <!-- Render View -->
              <iframe 
                v-else-if="currentTab.responseTab === 'render'"
                :srcdoc="currentTab.response?.body || ''"
                class="w-full h-full border-0 bg-white"
                sandbox="allow-scripts allow-forms allow-popups allow-modals"
              ></iframe>
            </template>
            <div v-else class="flex items-center justify-center w-full h-full text-base-content/50">
              <div class="text-center">
                <i class="fas fa-inbox text-4xl mb-2"></i>
                <p>{{ $t('trafficAnalysis.repeater.contextMenu.clickSendToSendRequest') }}</p>
              </div>
            </div>
          </div>
          
          <!-- Response Footer -->
          <div class="bg-base-200 px-3 py-1 flex items-center justify-end border-t border-base-300">
            <div class="text-xs text-base-content/70" v-if="currentTab.response">
              {{ formatBytes(currentTab.rawResponse?.length || 0) }} | {{ currentTab.response.responseTimeMs }} ms
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit as tauriEmit } from '@tauri-apps/api/event';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { dialog } from '@/composables/useDialog';
import HttpCodeEditor from '@/components/HttpCodeEditor.vue';

const router = useRouter();
const { t } = useI18n();

// Props
const props = defineProps<{
  initialRequest?: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  };
}>();

// Types
interface ReplayResponse {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
  responseTimeMs: number;
}

interface RepeaterTab {
  id: string;
  name: string;
  targetHost: string;
  targetPort: number;
  useTls: boolean;
  overrideSni: boolean;
  sniHost: string;
  rawRequest: string;
  rawResponse: string;
  requestTab: 'pretty' | 'raw' | 'hex';
  responseTab: 'pretty' | 'raw' | 'hex' | 'render';
  response: ReplayResponse | null;
  isSending: boolean; // 每个 tab 独立的发送状态
  modified: boolean; // 标记是否有未保存的修改
}

// Refs
const tabs = ref<RepeaterTab[]>([]);
const activeTabIndex = ref(0);
const showTargetDialog = ref(false);
const requestEditor = ref<InstanceType<typeof HttpCodeEditor> | null>(null);
const responseEditor = ref<InstanceType<typeof HttpCodeEditor> | null>(null);

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
});

// Layout
const STORAGE_KEY_LAYOUT = 'proxyRepeater.layoutMode';
const STORAGE_KEY_LEFT_WIDTH = 'proxyRepeater.leftPanelWidth';
const STORAGE_KEY_TOP_HEIGHT = 'proxyRepeater.topPanelHeight';
const STORAGE_KEY_TABS = 'proxyRepeater.tabs';

const layoutMode = ref<'horizontal' | 'vertical'>(
  (localStorage.getItem(STORAGE_KEY_LAYOUT) as 'horizontal' | 'vertical') || 'horizontal'
);
const leftPanelWidth = ref(parseInt(localStorage.getItem(STORAGE_KEY_LEFT_WIDTH) || '600'));
const topPanelHeight = ref(parseInt(localStorage.getItem(STORAGE_KEY_TOP_HEIGHT) || '350'));

let isResizing = false;
let startX = 0;
let startY = 0;
let startWidth = 0;
let startHeight = 0;
let saveTimer: number | null = null;
let hostDetectionTimer: number | null = null;

// Computed
const currentTab = computed(() => {
  if (tabs.value.length === 0) return null;
  return tabs.value[activeTabIndex.value] || null;
});

const showPort = computed(() => {
  if (!currentTab.value) return false;
  const port = currentTab.value.targetPort;
  const useTls = currentTab.value.useTls;
  if (useTls && port === 443) return false;
  if (!useTls && port === 80) return false;
  return true;
});

// 向后兼容的 isSending（用于模板）
const isSending = computed(() => currentTab.value?.isSending || false);

// Methods
function generateId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function createTab(request?: { method: string; url: string; headers: Record<string, string>; body?: string }): RepeaterTab {
  let targetHost = '';
  let targetPort = 443;
  let useTls = true;
  let rawRequest = '';
  
  if (request?.url) {
    try {
      const urlObj = new URL(request.url);
      targetHost = urlObj.hostname;
      
      // 安全的端口解析
      const parsedPort = urlObj.port ? parseInt(urlObj.port, 10) : null;
      if (parsedPort !== null && !isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535) {
        targetPort = parsedPort;
      } else {
        targetPort = urlObj.protocol === 'https:' ? 443 : 80;
      }
      
      useTls = urlObj.protocol === 'https:';
      
      const path = urlObj.pathname + urlObj.search;
      rawRequest = `${request.method || 'GET'} ${path} HTTP/1.1\r\n`;
      rawRequest += `Host: ${urlObj.hostname}\r\n`;
      
      for (const [key, value] of Object.entries(request.headers || {})) {
        if (key.toLowerCase() !== 'host') {
          rawRequest += `${key}: ${value}\r\n`;
        }
      }
      rawRequest += '\r\n';
      if (request.body) {
        rawRequest += request.body;
      }
    } catch (error) {
      console.error('Failed to parse URL:', error);
      // 使用默认值
      rawRequest = 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n';
    }
  } else {
    rawRequest = 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n';
  }
  
  return {
    id: generateId(),
    name: targetHost || `Request ${tabs.value.length + 1}`,
    targetHost,
    targetPort,
    useTls,
    overrideSni: false,
    sniHost: '',
    rawRequest,
    rawResponse: '',
    requestTab: 'pretty',
    responseTab: 'pretty',
    response: null,
    isSending: false,
    modified: false,
  };
}

function addTab() {
  // 限制标签页数量
  if (tabs.value.length >= MAX_TABS) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }));
    return;
  }
  
  const tab = createTab();
  tabs.value.push(tab);
  activeTabIndex.value = tabs.value.length - 1;
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
    // 关闭最后一个 tab 时，创建一个新的空 tab
    tabs.value = [createTab()];
    activeTabIndex.value = 0;
    return;
  }
  
  tabs.value.splice(index, 1);
  if (activeTabIndex.value >= tabs.value.length) {
    activeTabIndex.value = tabs.value.length - 1;
  }
  
  // 保存到 localStorage
  saveTabs();
}

function selectTab(index: number) {
  activeTabIndex.value = index;
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

async function sendRequest() {
  if (!currentTab.value || currentTab.value.isSending) return;
  
  if (!currentTab.value.targetHost || !currentTab.value.rawRequest.trim()) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.fillTargetAndRequest'));
    return;
  }
  
  const tab = currentTab.value;
  
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
  
  // 创建取消控制器
  const controller = { cancelled: false };
  abortControllers.set(tab.id, controller);
  
  const tabId = tab.id;
  
  try {
    let rawRequest = tab.rawRequest;
    rawRequest = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n');
    
    if (!rawRequest.endsWith('\r\n\r\n')) {
      if (rawRequest.endsWith('\r\n')) {
        rawRequest += '\r\n';
      } else {
        rawRequest += '\r\n\r\n';
      }
    }
    
    const response = await invoke<any>('replay_raw_request', {
      host: tab.targetHost,
      port: tab.targetPort || 443,
      useTls: tab.useTls,
      rawRequest: rawRequest,
      timeoutSecs: 30,
    });
    
    // 检查是否已取消
    if (controller.cancelled) return;
    
    // 查找目标 tab（可能已被删除或切换）
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (!targetTab) return;
    
    if (response.success && response.data) {
      targetTab.rawResponse = response.data.raw_response;
      
      // 检查响应体大小
      const responseSize = response.data.raw_response?.length || 0;
      const MAX_RESPONSE_SIZE = 10 * 1024 * 1024; // 10MB
      if (responseSize > MAX_RESPONSE_SIZE) {
        dialog.toast.warning(t('trafficAnalysis.repeater.messages.largeResponse', { 
          size: formatBytes(responseSize) 
        }));
      }
      
      // 解析响应
      const parsedResponse = parseRawResponse(response.data.raw_response, response.data.response_time_ms);
      if (parsedResponse) {
        targetTab.response = parsedResponse;
      }
      
      targetTab.name = targetTab.targetHost;
      targetTab.modified = false;
    } else {
      const errorMsg = parseErrorMessage(response.error);
      dialog.toast.error(errorMsg);
    }
  } catch (error: any) {
    if (controller.cancelled) return;
    
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (!targetTab) return;
    
    console.error('Failed to send request:', error);
    const errorMsg = parseErrorMessage(error);
    dialog.toast.error(errorMsg);
  } finally {
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (targetTab) {
      targetTab.isSending = false;
    }
    abortControllers.delete(tabId);
  }
}

// 解析原始响应
function parseRawResponse(rawResp: string, responseTimeMs: number): ReplayResponse | null {
  if (!rawResp) return null;
  
  // 支持多种分隔符
  let headerEnd = rawResp.indexOf('\r\n\r\n');
  let separatorLen = 4;
  
  if (headerEnd === -1) {
    headerEnd = rawResp.indexOf('\n\n');
    separatorLen = 2;
  }
  
  if (headerEnd === -1) {
    headerEnd = rawResp.indexOf('\r\r');
    separatorLen = 2;
  }
  
  if (headerEnd === -1) {
    // 没有找到分隔符，可能是纯头部
    return null;
  }
  
  const headerPart = rawResp.substring(0, headerEnd);
  const bodyPart = rawResp.substring(headerEnd + separatorLen);
  
  // 解析状态行
  const lines = headerPart.split(/\r\n|\r|\n/);
  const statusLine = lines[0];
  const statusMatch = statusLine.match(/HTTP\/[\d.]+\s+(\d+)/);
  const statusCode = statusMatch ? parseInt(statusMatch[1]) : 0;
  
  // 解析响应头
  const headers: Record<string, string> = {};
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      const key = line.substring(0, colonIndex).trim();
      const value = line.substring(colonIndex + 1).trim();
      headers[key] = value;
    }
  }
  
  return {
    statusCode,
    headers,
    body: bodyPart,
    responseTimeMs,
  };
}

// 解析错误信息
function parseErrorMessage(error: any): string {
  if (!error) return t('trafficAnalysis.repeater.messages.unknownError');
  
  const errorStr = String(error).toLowerCase();
  
  if (errorStr.includes('timeout')) {
    return t('trafficAnalysis.repeater.messages.timeout');
  }
  if (errorStr.includes('connection refused') || errorStr.includes('econnrefused')) {
    return t('trafficAnalysis.repeater.messages.connectionRefused');
  }
  if (errorStr.includes('network')) {
    return t('trafficAnalysis.repeater.messages.networkError');
  }
  
  // 返回原始错误信息（但限制长度）
  const msg = String(error);
  return msg.length > 100 ? msg.substring(0, 100) + '...' : msg;
}

function formatPrettyRequest(): string {
  if (!currentTab.value?.rawRequest) return '';
  
  const raw = currentTab.value.rawRequest;
  
  // 查找 header 和 body 的分隔位置（支持 \r\n\r\n 和 \n\n）
  let headerBodySplit = raw.indexOf('\r\n\r\n');
  let separatorLen = 4;
  
  if (headerBodySplit === -1) {
    headerBodySplit = raw.indexOf('\n\n');
    separatorLen = 2;
  }
  
  if (headerBodySplit === -1) {
    // 没有 body，直接返回
    return raw.replace(/\r\n/g, '\n');
  }
  
  const headerPart = raw.substring(0, headerBodySplit);
  const bodyPart = raw.substring(headerBodySplit + separatorLen);
  
  let result = headerPart.replace(/\r\n/g, '\n') + '\n\n';
  
  if (bodyPart && bodyPart.trim()) {
    // 尝试格式化 JSON body
    try {
      const json = JSON.parse(bodyPart.trim());
      result += JSON.stringify(json, null, 2);
    } catch {
      result += bodyPart;
    }
  }
  
  return result;
}

function onPrettyRequestUpdate(value: string) {
  if (!currentTab.value) return;
  
  // 将格式化的内容转换回原始格式
  const lines = value.split('\n');
  let headerEnd = -1;
  
  // 查找空行分隔 headers 和 body
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === '') {
      headerEnd = i;
      break;
    }
  }
  
  if (headerEnd === -1) {
    // 没有 body
    currentTab.value.rawRequest = value.replace(/\n/g, '\r\n');
    autoDetectHostFromRequest(value);
    currentTab.value.modified = true;
    return;
  }
  
  const headerPart = lines.slice(0, headerEnd).join('\r\n');
  const bodyPart = lines.slice(headerEnd + 1).join('\n').trim();
  
  // 尝试压缩 JSON body（如果是 JSON 的话）
  let finalBody = bodyPart;
  try {
    const json = JSON.parse(bodyPart);
    finalBody = JSON.stringify(json);
  } catch {
    finalBody = bodyPart;
  }
  
  currentTab.value.rawRequest = headerPart + '\r\n\r\n' + finalBody;
  autoDetectHostFromRequest(value);
  currentTab.value.modified = true;
}

function formatPrettyResponse(): string {
  if (!currentTab.value?.response) return '';
  
  const resp = currentTab.value.response;
  let result = `HTTP/1.1 ${resp.statusCode} OK\r\n`;
  
  for (const [key, value] of Object.entries(resp.headers)) {
    result += `${key}: ${value}\r\n`;
  }
  result += '\r\n';
  
  const contentType = resp.headers['content-type'] || resp.headers['Content-Type'] || '';
  if (contentType.includes('json')) {
    try {
      const json = JSON.parse(resp.body);
      result += JSON.stringify(json, null, 2);
    } catch {
      result += resp.body;
    }
  } else {
    result += resp.body;
  }
  
  return result;
}

function toHex(str: string): string {
  if (!str) return '';
  
  // 限制最大显示长度（1MB）
  const MAX_HEX_LENGTH = 1024 * 1024;
  const limited = str.length > MAX_HEX_LENGTH;
  const displayStr = limited ? str.substring(0, MAX_HEX_LENGTH) : str;
  
  const lines: string[] = [];
  let hex = '';
  let ascii = '';
  let lineCount = 0;
  
  // 限制最大行数（避免渲染过多行导致卡顿）
  const MAX_LINES = 10000;
  let totalLines = 0;
  
  for (let i = 0; i < displayStr.length && totalLines < MAX_LINES; i++) {
    const charCode = displayStr.charCodeAt(i);
    hex += charCode.toString(16).padStart(2, '0') + ' ';
    ascii += charCode >= 32 && charCode < 127 ? displayStr[i] : '.';
    lineCount++;
    
    if (lineCount === 16) {
      lines.push(hex + ' ' + ascii);
      hex = '';
      ascii = '';
      lineCount = 0;
      totalLines++;
    }
  }
  
  if (lineCount > 0) {
    lines.push(hex + '   '.repeat(16 - lineCount) + ' ' + ascii);
  }
  
  if (limited || totalLines >= MAX_LINES) {
    lines.push('');
    lines.push(t('trafficAnalysis.repeater.messages.hexDisplayLimited', { size: formatBytes(MAX_HEX_LENGTH) }));
  }
  
  return lines.join('\n');
}

function getStatusClass(status: number): string {
  if (status >= 200 && status < 300) return 'badge-success';
  if (status >= 300 && status < 400) return 'badge-info';
  if (status >= 400 && status < 500) return 'badge-warning';
  if (status >= 500) return 'badge-error';
  return 'badge-ghost';
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

// 右键菜单
function showContextMenu(event: MouseEvent) {
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
  };
  
  setTimeout(() => {
    document.addEventListener('click', hideContextMenu);
  }, 0);
}

function hideContextMenu() {
  contextMenu.value.visible = false;
  document.removeEventListener('click', hideContextMenu);
}

function contextMenuSend() {
  hideContextMenu();
  sendRequest();
}

function contextMenuSendToNewTab() {
  hideContextMenu();
  if (!currentTab.value) return;
  
  // 限制标签页数量
  if (tabs.value.length >= MAX_TABS) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }));
    return;
  }
  
  const newTab = createTab();
  newTab.targetHost = currentTab.value.targetHost;
  newTab.targetPort = currentTab.value.targetPort;
  newTab.useTls = currentTab.value.useTls;
  newTab.rawRequest = currentTab.value.rawRequest;
  tabs.value.push(newTab);
  activeTabIndex.value = tabs.value.length - 1;
}

function contextMenuCopyUrl() {
  hideContextMenu();
  const url = buildFullUrl();
  if (url) {
    navigator.clipboard.writeText(url)
      .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.urlCopied')))
      .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
  }
}

function contextMenuCopyRequest() {
  hideContextMenu();
  if (!currentTab.value?.rawRequest) return;
  
  navigator.clipboard.writeText(currentTab.value.rawRequest)
    .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.requestCopied')))
    .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
}

function contextMenuCopyCurl() {
  hideContextMenu();
  const curl = buildCurlCommand();
  if (curl) {
    navigator.clipboard.writeText(curl)
      .then(() => dialog.toast.success(t('trafficAnalysis.repeater.messages.curlCopied')))
      .catch(() => dialog.toast.error(t('trafficAnalysis.repeater.messages.copyFailed')));
  }
}

async function contextMenuPaste() {
  hideContextMenu();
  if (!currentTab.value) return;
  
  try {
    const text = await navigator.clipboard.readText();
    currentTab.value.rawRequest += text;
    currentTab.value.modified = true;
    
    // 立即检测 Host（粘贴操作时不使用防抖）
    autoDetectHostFromRequest(currentTab.value.rawRequest);
    
    dialog.toast.success(t('trafficAnalysis.repeater.messages.pasted'));
  } catch {
    dialog.toast.error(t('trafficAnalysis.repeater.messages.cannotReadClipboard'));
  }
}

function contextMenuClear() {
  hideContextMenu();
  if (!currentTab.value) return;
  currentTab.value.rawRequest = '';
  currentTab.value.modified = true;
}

// 发送到 AI 助手
type SendType = 'request' | 'response' | 'both';

async function sendToAssistant(type: SendType) {
  if (!currentTab.value) return;
  
  // 从 rawRequest 解析请求信息
  const lines = currentTab.value.rawRequest.split(/\r\n|\r|\n/);
  const firstLine = lines[0];
  const methodMatch = firstLine.match(/^(\w+)\s+(\S+)/);
  const method = methodMatch ? methodMatch[1] : 'GET';
  const path = methodMatch ? methodMatch[2] : '/';
  
  // 解析请求头
  const requestHeaders: Record<string, string> = {};
  let inBody = false;
  const bodyLines: string[] = [];
  
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    if (!inBody && line === '') {
      inBody = true;
      continue;
    }
    if (!inBody) {
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex).trim();
        const value = line.substring(colonIndex + 1).trim();
        requestHeaders[key] = value;
      }
    } else {
      bodyLines.push(line);
    }
  }
  
  const requestBody = bodyLines.join('\n').trim();
  const url = buildFullUrl();
  
  // 构建流量数据
  const trafficData = {
    id: Date.now(),
    url,
    method,
    host: currentTab.value.targetHost,
    status_code: currentTab.value.response?.statusCode || 0,
    request_headers: JSON.stringify(requestHeaders),
    request_body: requestBody || undefined,
    response_headers: currentTab.value.response ? JSON.stringify(currentTab.value.response.headers) : undefined,
    response_body: currentTab.value.response?.body || undefined,
  };
  
  // 发送全局事件
  await tauriEmit('traffic:send-to-assistant', { requests: [trafficData], type });
  
  const typeKey = type === 'request' ? 'request' : type === 'response' ? 'response' : 'both';
  const typeText = t(`trafficAnalysis.repeater.types.${typeKey}`);
  dialog.toast.success(t('trafficAnalysis.repeater.messages.sentToAssistant', { type: typeText }));
  
  // 跳转到 AI 助手页面
  router.push('/ai-assistant');
}

function contextMenuSendRequestToAssistant() {
  hideContextMenu();
  sendToAssistant('request');
}

function contextMenuSendResponseToAssistant() {
  hideContextMenu();
  if (!currentTab.value?.response) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.noResponseData'));
    return;
  }
  sendToAssistant('response');
}

function buildFullUrl(): string {
  if (!currentTab.value) return '';
  
  try {
    const protocol = currentTab.value.useTls ? 'https' : 'http';
    const host = currentTab.value.targetHost;
    const port = currentTab.value.targetPort;
    const defaultPort = currentTab.value.useTls ? 443 : 80;
    const portStr = port !== defaultPort ? `:${port}` : '';
    
    // 从 rawRequest 提取路径
    const firstLine = currentTab.value.rawRequest.split(/\r\n|\r|\n/)[0];
    const match = firstLine.match(/^\w+\s+(\S+)/);
    let path = match ? match[1] : '/';
    
    // 确保路径以 / 开头
    if (!path.startsWith('/') && !path.startsWith('http')) {
      path = '/' + path;
    }
    
    // 如果路径已经是完整 URL，直接返回
    if (path.startsWith('http://') || path.startsWith('https://')) {
      return path;
    }
    
    const url = `${protocol}://${host}${portStr}${path}`;
    
    // 验证 URL 格式
    try {
      new URL(url);
      return url;
    } catch {
      console.warn('Invalid URL constructed:', url);
      return url; // 仍然返回，但已记录警告
    }
  } catch (error) {
    console.error('Error building URL:', error);
    return '';
  }
}

function buildCurlCommand(): string {
  if (!currentTab.value) return '';
  
  const lines = currentTab.value.rawRequest.split(/\r\n|\r|\n/);
  const firstLine = lines[0];
  const methodMatch = firstLine.match(/^(\w+)\s+(\S+)/);
  const method = methodMatch ? methodMatch[1] : 'GET';
  
  const url = buildFullUrl();
  let curl = `curl -X ${method} '${url}'`;
  
  // 解析 headers
  let inBody = false;
  const bodyLines: string[] = [];
  
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    if (!inBody && line === '') {
      inBody = true;
      continue;
    }
    if (!inBody) {
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex).trim();
        const value = line.substring(colonIndex + 1).trim();
        if (key.toLowerCase() !== 'host' && key.toLowerCase() !== 'content-length') {
          curl += ` \\\n  -H '${key}: ${value}'`;
        }
      }
    } else {
      bodyLines.push(line);
    }
  }
  
  const body = bodyLines.join('\n').trim();
  if (body) {
    curl += ` \\\n  -d '${body.replace(/'/g, "'\\''")}'`;
  }
  
  return curl;
}

// 数据持久化
function saveTabs() {
  try {
    const tabsToSave = tabs.value.map(tab => ({
      id: tab.id,
      name: tab.name,
      targetHost: tab.targetHost,
      targetPort: tab.targetPort,
      useTls: tab.useTls,
      overrideSni: tab.overrideSni,
      sniHost: tab.sniHost,
      rawRequest: tab.rawRequest,
      requestTab: tab.requestTab,
      responseTab: tab.responseTab,
      // 不保存响应数据和发送状态
    }));
    
    localStorage.setItem(STORAGE_KEY_TABS, JSON.stringify(tabsToSave));
  } catch (error) {
    console.error('Failed to save tabs:', error);
  }
}

function loadTabs() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY_TABS);
    if (!saved) return;
    
    const tabsData = JSON.parse(saved);
    if (!Array.isArray(tabsData) || tabsData.length === 0) return;
    
    tabs.value = tabsData.map(data => ({
      ...data,
      rawResponse: '',
      response: null,
      isSending: false,
      modified: false,
    }));
    
    dialog.toast.success(t('trafficAnalysis.repeater.messages.tabRestored', { count: tabs.value.length }));
  } catch (error) {
    console.error('Failed to load tabs:', error);
  }
}

// 自动检测并填充 Host 配置
function autoDetectHostFromRequest(requestText: string) {
  if (!currentTab.value || !requestText) return;
  
  // 解析请求文本，查找 Host 头
  const lines = requestText.split(/\r\n|\r|\n/);
  let hostValue = '';
  
  for (const line of lines) {
    const trimmedLine = line.trim();
    if (trimmedLine === '') break; // 到达 body 部分
    
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      const key = line.substring(0, colonIndex).trim();
      const value = line.substring(colonIndex + 1).trim();
      
      if (key.toLowerCase() === 'host') {
        hostValue = value;
        break;
      }
    }
  }
  
  if (!hostValue) return;
  
  // 解析 Host 值，可能包含端口号
  const hostPortMatch = hostValue.match(/^([^:]+)(?::(\d+))?$/);
  if (!hostPortMatch) return;
  
  const hostname = hostPortMatch[1];
  const port = hostPortMatch[2] ? parseInt(hostPortMatch[2]) : null;
  
  // 检查是否需要更新（避免不必要的更新）
  const needsUpdate = !currentTab.value.targetHost || 
                      currentTab.value.targetHost === 'example.com' ||
                      currentTab.value.targetHost !== hostname;
  
  if (needsUpdate) {
    currentTab.value.targetHost = hostname;
    
    // 根据端口号判断协议
    if (port !== null) {
      currentTab.value.targetPort = port;
      currentTab.value.useTls = port === 443;
    } else {
      // 没有指定端口，默认使用 HTTPS/443
      currentTab.value.targetPort = 443;
      currentTab.value.useTls = true;
    }
    
    // 更新 tab 名称
    currentTab.value.name = hostname;
    
    console.log(`Auto-detected host: ${hostname}, port: ${currentTab.value.targetPort}, TLS: ${currentTab.value.useTls}`);
  }
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
  
  localStorage.setItem(STORAGE_KEY_LEFT_WIDTH, String(leftPanelWidth.value));
  localStorage.setItem(STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value));
}

// Expose
function addRequestFromHistory(request: { method: string; url: string; headers: Record<string, string>; body?: string }) {
  // 限制标签页数量
  if (tabs.value.length >= MAX_TABS) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.tooManyTabs', { max: MAX_TABS }));
    // 删除最旧的标签页
    if (tabs.value.length > 0) {
      const oldestTab = tabs.value[0];
      abortControllers.delete(oldestTab.id);
      tabs.value.shift();
      if (activeTabIndex.value > 0) {
        activeTabIndex.value--;
      }
    }
  }
  
  const tab = createTab(request);
  tabs.value.push(tab);
  activeTabIndex.value = tabs.value.length - 1;
}

defineExpose({
  addRequestFromHistory,
});

// Watchers
watch(() => props.initialRequest, (newRequest) => {
  if (newRequest) {
    addRequestFromHistory(newRequest);
  }
}, { immediate: true });

watch(layoutMode, (newMode) => {
  localStorage.setItem(STORAGE_KEY_LAYOUT, newMode);
});

// 监听 tabs 变化，自动保存（防抖）
watch(tabs, () => {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
  }
  
  saveTimer = window.setTimeout(() => {
    saveTabs();
    saveTimer = null;
  }, 1000); // 1秒后保存
}, { deep: true });

// 监听 rawRequest 变化，自动检测 Host（使用防抖避免频繁触发）
watch(() => currentTab.value?.rawRequest, (newRequest, oldRequest) => {
  if (newRequest && currentTab.value && newRequest !== oldRequest) {
    // 清除之前的定时器
    if (hostDetectionTimer !== null) {
      clearTimeout(hostDetectionTimer);
    }
    
    // 使用防抖，500ms 后执行检测
    hostDetectionTimer = window.setTimeout(() => {
      if (currentTab.value) {
        autoDetectHostFromRequest(newRequest);
      }
      hostDetectionTimer = null;
    }, 500);
  }
}, { deep: false });

// 键盘快捷键处理
function handleKeydown(event: KeyboardEvent) {
  // Cmd/Ctrl + R 发送当前请求到新标签（Send to Repeater）
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
    if (currentTab.value && currentTab.value.rawRequest.trim()) {
      event.preventDefault();
      event.stopPropagation();
      contextMenuSendToNewTab();
    }
  }
}

// Lifecycle
onMounted(() => {
  // 尝试从 localStorage 恢复 tabs
  // loadTabs();
  
  // 如果没有恢复到任何 tab，创建一个新的
  if (tabs.value.length === 0 && !props.initialRequest) {
    addTab();
  }
  
  // 添加键盘快捷键监听
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.removeEventListener('keydown', handleKeydown);
  
  // 清理所有取消控制器
  abortControllers.clear();
  
  // 清理 host 检测定时器
  if (hostDetectionTimer !== null) {
    clearTimeout(hostDetectionTimer);
    hostDetectionTimer = null;
  }
  
  // 清理保存定时器
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  
  // 保存 tabs 到 localStorage
  saveTabs();
});
</script>

<style scoped>
.request-panel,
.response-panel {
  min-width: 300px;
  min-height: 200px;
}

pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
