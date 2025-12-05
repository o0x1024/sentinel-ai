<template>
  <div class="flex flex-col h-full bg-base-100">
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
            class="w-4 h-4 flex items-center justify-center rounded opacity-40 hover:opacity-100 hover:bg-base-300 transition-opacity"
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
          <div class="flex-1 overflow-hidden">
            <template v-if="currentTab.requestTab !== 'hex'">
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
          <div class="flex-1 overflow-hidden">
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
import { dialog } from '@/composables/useDialog';
import HttpCodeEditor from '@/components/HttpCodeEditor.vue';

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
    requestTab: 'raw',
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
