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
        Send Request
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendToNewTab"
      >
        <i class="fas fa-plus text-success"></i>
        Send to New Tab
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyUrl"
      >
        <i class="fas fa-link text-info"></i>
        Copy URL
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyRequest"
      >
        <i class="fas fa-copy text-secondary"></i>
        Copy Request
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuCopyCurl"
      >
        <i class="fas fa-terminal text-warning"></i>
        Copy as cURL
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendRequestToAssistant"
      >
        <i class="fas fa-upload text-accent"></i>
        Send Request to Assistant
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuSendResponseToAssistant"
        :disabled="!currentTab?.response"
      >
        <i class="fas fa-download text-accent"></i>
        Send Response to Assistant
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuPaste"
      >
        <i class="fas fa-paste text-accent"></i>
        Paste
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="contextMenuClear"
      >
        <i class="fas fa-eraser text-error"></i>
        Clear
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
            title="关闭"
          >
            <i class="fas fa-times text-[10px]"></i>
          </button>
        </div>
        <button 
          @click="addTab"
          class="btn btn-xs btn-ghost"
          title="新建标签"
        >
          <i class="fas fa-plus"></i>
        </button>
      </div>
      <!-- 布局切换按钮 -->
      <div class="btn-group btn-group-xs">
        <button 
          :class="['btn btn-xs', layoutMode === 'horizontal' ? 'btn-primary' : 'btn-ghost']"
          @click="layoutMode = 'horizontal'"
          title="左右布局"
        >
          <i class="fas fa-columns"></i>
        </button>
        <button 
          :class="['btn btn-xs', layoutMode === 'vertical' ? 'btn-primary' : 'btn-ghost']"
          @click="layoutMode = 'vertical'"
          title="上下布局"
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
          Send
        </button>
        <button 
          @click="cancelRequest"
          class="btn btn-ghost btn-sm"
          :disabled="!isSending"
        >
          <i class="fas fa-stop"></i>
          Cancel
        </button>
        
        <div class="flex-1"></div>
        
        <!-- Target 显示 -->
        <div class="flex items-center gap-2 text-sm">
          <span class="text-base-content/70">Target:</span>
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
          <h3 class="font-bold text-lg mb-4">Configure target details</h3>
          <p class="text-sm text-base-content/70 mb-4">
            Specify the details of the server to which the request will be sent.
          </p>
          
          <div class="space-y-4">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">Host:</span>
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
              <span class="label-text">Override SNI</span>
            </label>
            
            <div v-if="currentTab.overrideSni" class="form-control pl-6">
              <input 
                v-model="currentTab.sniHost"
                type="text" 
                class="input input-bordered input-sm w-full"
                placeholder="SNI hostname"
              />
            </div>
            
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">Port:</span>
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
              <span class="label-text">Use HTTPS</span>
            </label>
          </div>
          
          <div class="modal-action">
            <button class="btn btn-sm" @click="showTargetDialog = false">Cancel</button>
            <button class="btn btn-primary btn-sm" @click="showTargetDialog = false">OK</button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="showTargetDialog = false">
          <button>close</button>
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
            <span class="font-semibold text-sm">Request</span>
            <div class="tabs tabs-boxed tabs-xs bg-base-300">
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'pretty' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'pretty'"
              >Pretty</a>
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'raw' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'raw'"
              >Raw</a>
              <a 
                :class="['tab tab-xs', currentTab.requestTab === 'hex' ? 'tab-active' : '']"
                @click="currentTab.requestTab = 'hex'"
              >Hex</a>
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
              <span class="font-semibold text-sm">Response</span>
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
              >Pretty</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'raw' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'raw'"
              >Raw</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'hex' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'hex'"
              >Hex</a>
              <a 
                :class="['tab tab-xs', currentTab.responseTab === 'render' ? 'tab-active' : '']"
                @click="currentTab.responseTab = 'render'"
              >Render</a>
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
                sandbox="allow-same-origin"
              ></iframe>
            </template>
            <div v-else class="flex items-center justify-center w-full h-full text-base-content/50">
              <div class="text-center">
                <i class="fas fa-inbox text-4xl mb-2"></i>
                <p>点击 Send 发送请求</p>
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
import { dialog } from '@/composables/useDialog';
import HttpCodeEditor from '@/components/HttpCodeEditor.vue';

const router = useRouter();

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
}

// Refs
const tabs = ref<RepeaterTab[]>([]);
const activeTabIndex = ref(0);
const isSending = ref(false);
const showTargetDialog = ref(false);
const requestEditor = ref<InstanceType<typeof HttpCodeEditor> | null>(null);
const responseEditor = ref<InstanceType<typeof HttpCodeEditor> | null>(null);

let requestCancelled = false;

// 右键菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
});

// Layout
const STORAGE_KEY_LAYOUT = 'proxyRepeater.layoutMode';
const STORAGE_KEY_LEFT_WIDTH = 'proxyRepeater.leftPanelWidth';
const STORAGE_KEY_TOP_HEIGHT = 'proxyRepeater.topPanelHeight';

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
      targetPort = urlObj.port ? parseInt(urlObj.port) : (urlObj.protocol === 'https:' ? 443 : 80);
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
    } catch {
      // Ignore
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
  };
}

function addTab() {
  const tab = createTab();
  tabs.value.push(tab);
  activeTabIndex.value = tabs.value.length - 1;
}

function closeTab(index: number) {
  if (tabs.value.length === 1) {
    tabs.value = [];
    activeTabIndex.value = 0;
    return;
  }
  
  tabs.value.splice(index, 1);
  if (activeTabIndex.value >= tabs.value.length) {
    activeTabIndex.value = tabs.value.length - 1;
  }
}

function selectTab(index: number) {
  activeTabIndex.value = index;
}

function cancelRequest() {
  requestCancelled = true;
  isSending.value = false;
  dialog.toast.info('请求已取消');
}

async function sendRequest() {
  if (!currentTab.value || isSending.value) return;
  
  if (!currentTab.value.targetHost || !currentTab.value.rawRequest.trim()) {
    dialog.toast.warning('请填写目标 Host 和请求内容');
    return;
  }
  
  isSending.value = true;
  requestCancelled = false;
  currentTab.value.response = null;
  currentTab.value.rawResponse = '';
  
  const tabId = currentTab.value.id;
  
  try {
    let rawRequest = currentTab.value.rawRequest;
    rawRequest = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n');
    
    if (!rawRequest.endsWith('\r\n\r\n')) {
      if (rawRequest.endsWith('\r\n')) {
        rawRequest += '\r\n';
      } else {
        rawRequest += '\r\n\r\n';
      }
    }
    
    const response = await invoke<any>('replay_raw_request', {
      host: currentTab.value.targetHost,
      port: currentTab.value.targetPort || 443,
      useTls: currentTab.value.useTls,
      rawRequest: rawRequest,
      timeoutSecs: 30,
    });
    
    if (requestCancelled) return;
    const targetTab = tabs.value.find(t => t.id === tabId);
    if (!targetTab) return;
    
    if (response.success && response.data) {
      targetTab.rawResponse = response.data.raw_response;
      
      const rawResp = response.data.raw_response;
      const headerEnd = rawResp.indexOf('\r\n\r\n');
      if (headerEnd > 0) {
        const headerPart = rawResp.substring(0, headerEnd);
        const bodyPart = rawResp.substring(headerEnd + 4);
        
        const statusLine = headerPart.split('\r\n')[0];
        const statusMatch = statusLine.match(/HTTP\/[\d.]+\s+(\d+)/);
        const statusCode = statusMatch ? parseInt(statusMatch[1]) : 0;
        
        const headers: Record<string, string> = {};
        const headerLines = headerPart.split('\r\n').slice(1);
        for (const line of headerLines) {
          const colonIndex = line.indexOf(':');
          if (colonIndex > 0) {
            headers[line.substring(0, colonIndex).trim()] = line.substring(colonIndex + 1).trim();
          }
        }
        
        targetTab.response = {
          statusCode,
          headers,
          body: bodyPart,
          responseTimeMs: response.data.response_time_ms,
        };
      }
      
      targetTab.name = targetTab.targetHost;
    } else {
      dialog.toast.error(response.error || '请求失败');
    }
  } catch (error: any) {
    if (requestCancelled) return;
    console.error('Failed to send request:', error);
    dialog.toast.error(`发送请求失败: ${error}`);
  } finally {
    isSending.value = false;
  }
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
  let hex = '';
  let ascii = '';
  let lineCount = 0;
  
  for (let i = 0; i < str.length; i++) {
    const charCode = str.charCodeAt(i);
    hex += charCode.toString(16).padStart(2, '0') + ' ';
    ascii += charCode >= 32 && charCode < 127 ? str[i] : '.';
    lineCount++;
    
    if (lineCount === 16) {
      hex += ' ' + ascii + '\n';
      ascii = '';
      lineCount = 0;
    }
  }
  
  if (lineCount > 0) {
    hex += '   '.repeat(16 - lineCount) + ' ' + ascii;
  }
  
  return hex;
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
  contextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 200),
    y: Math.min(event.clientY, window.innerHeight - 300),
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
      .then(() => dialog.toast.success('URL 已复制'))
      .catch(() => dialog.toast.error('复制失败'));
  }
}

function contextMenuCopyRequest() {
  hideContextMenu();
  if (!currentTab.value?.rawRequest) return;
  
  navigator.clipboard.writeText(currentTab.value.rawRequest)
    .then(() => dialog.toast.success('请求已复制'))
    .catch(() => dialog.toast.error('复制失败'));
}

function contextMenuCopyCurl() {
  hideContextMenu();
  const curl = buildCurlCommand();
  if (curl) {
    navigator.clipboard.writeText(curl)
      .then(() => dialog.toast.success('cURL 命令已复制'))
      .catch(() => dialog.toast.error('复制失败'));
  }
}

async function contextMenuPaste() {
  hideContextMenu();
  if (!currentTab.value) return;
  
  try {
    const text = await navigator.clipboard.readText();
    currentTab.value.rawRequest += text;
    dialog.toast.success('已粘贴');
  } catch {
    dialog.toast.error('无法读取剪贴板');
  }
}

function contextMenuClear() {
  hideContextMenu();
  if (!currentTab.value) return;
  currentTab.value.rawRequest = '';
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
  
  const typeText = type === 'request' ? '请求' : type === 'response' ? '响应' : '流量';
  dialog.toast.success(`已发送${typeText}到 AI 助手`);
  
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
    dialog.toast.warning('暂无响应数据');
    return;
  }
  sendToAssistant('response');
}

function buildFullUrl(): string {
  if (!currentTab.value) return '';
  
  const protocol = currentTab.value.useTls ? 'https' : 'http';
  const host = currentTab.value.targetHost;
  const port = currentTab.value.targetPort;
  const defaultPort = currentTab.value.useTls ? 443 : 80;
  const portStr = port !== defaultPort ? `:${port}` : '';
  
  // 从 rawRequest 提取路径
  const firstLine = currentTab.value.rawRequest.split(/\r\n|\r|\n/)[0];
  const match = firstLine.match(/^\w+\s+(\S+)/);
  const path = match ? match[1] : '/';
  
  return `${protocol}://${host}${portStr}${path}`;
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

// Lifecycle
onMounted(() => {
  if (tabs.value.length === 0 && !props.initialRequest) {
    addTab();
  }
});

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
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
