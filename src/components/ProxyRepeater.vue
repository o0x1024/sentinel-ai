<template>
  <div class="flex flex-col h-full bg-base-100">
    <!-- Tabs Header -->
    <div class="bg-base-200 border-b border-base-300 px-2 py-1 flex items-center gap-2">
      <div class="flex items-center gap-1 overflow-x-auto flex-1">
        <div 
          v-for="(tab, index) in tabs" 
          :key="tab.id"
          class="flex items-center gap-1 px-3 py-1 rounded cursor-pointer border border-base-300 text-sm"
          :class="activeTabIndex === index ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
          @click="selectTab(index)"
        >
          <span class="truncate max-w-24" :title="tab.name">{{ index + 1 }}</span>
          <button 
            @click.stop="closeTab(index)"
            class="btn btn-xs btn-ghost btn-circle opacity-60 hover:opacity-100"
            title="关闭"
          >
            <i class="fas fa-times text-xs"></i>
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
        
        <div class="flex-1"></div>
        
        <!-- Target 显示 -->
        <div class="flex items-center gap-2 text-sm">
          <span class="text-base-content/70">Target:</span>
          <span class="font-mono font-semibold">
            {{ currentTab.useTls ? 'https' : 'http' }}://{{ currentTab.targetHost || 'example.com' }}{{ showPort ? ':' + currentTab.targetPort : '' }}
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
      <dialog 
        :class="['modal', showTargetDialog ? 'modal-open' : '']"
      >
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
          class="request-content flex flex-col overflow-hidden border-base-300"
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
          <div class="flex-1 overflow-hidden flex relative">
            <!-- Pretty/Raw View (可编辑带高亮) -->
            <template v-if="currentTab.requestTab !== 'hex'">
              <div class="line-numbers select-none" ref="requestLineNumbers">
                <div v-for="n in getLineCount(currentTab.rawRequest)" :key="n" class="line-number">{{ n }}</div>
              </div>
              <div class="flex-1 relative overflow-hidden">
                <!-- 高亮层 (底层) -->
                <div 
                  class="absolute inset-0 overflow-auto p-2 http-content pointer-events-none"
                  ref="requestHighlight"
                  v-html="highlightHttpRequest(currentTab.rawRequest)"
                ></div>
                <!-- 编辑层 (透明覆盖) -->
                <textarea 
                  ref="requestTextarea"
                  v-model="currentTab.rawRequest"
                  class="code-editor absolute inset-0 w-full h-full p-2 resize-none border-0 focus:outline-none"
                  :placeholder="'GET /path HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\n\r\n'"
                  spellcheck="false"
                  @scroll="syncRequestScroll"
                ></textarea>
              </div>
            </template>
            
            <!-- Hex View -->
            <template v-else>
              <div class="line-numbers select-none">
                <div v-for="n in getLineCount(toHex(currentTab.rawRequest))" :key="n" class="line-number">{{ n }}</div>
              </div>
              <pre class="flex-1 p-2 font-mono text-xs overflow-auto">{{ toHex(currentTab.rawRequest) }}</pre>
            </template>
          </div>
          
          <!-- Request Footer -->
          <div class="bg-base-200 px-3 py-1 flex items-center gap-2 border-t border-base-300">
            <div class="join flex-1">
                <input 
                type="text" 
                v-model="requestSearch"
                placeholder="Search"
                class="input input-bordered input-xs join-item flex-1"
                @keydown.enter="navigateSearch('request', 'next')"
                @input="requestSearchIndex = 0"
                />
                <button 
                class="btn btn-xs btn-ghost join-item"
                :disabled="requestMatchCount === 0"
                @click="navigateSearch('request', 'prev')"
                title="上一个"
              >
                <i class="fas fa-chevron-left"></i>
                </button>
              <button 
                class="btn btn-xs btn-ghost join-item"
                :disabled="requestMatchCount === 0"
                @click="navigateSearch('request', 'next')"
                title="下一个"
              >
                <i class="fas fa-chevron-right"></i>
              </button>
            </div>
            <span class="text-xs text-base-content/50 min-w-16 text-right">{{ requestSearchResults }}</span>
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
        <div class="response-content flex-1 flex flex-col overflow-hidden min-h-0 min-w-0">
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
          <div class="flex-1 overflow-hidden flex font-mono text-sm">
          <template v-if="currentTab.response || currentTab.rawResponse">
            <!-- Pretty View -->
              <template v-if="currentTab.responseTab === 'pretty'">
                <div class="line-numbers select-none">
                  <div v-for="n in getLineCount(formatPrettyResponse())" :key="n" class="line-number">{{ n }}</div>
                </div>
                <div class="flex-1 overflow-auto p-2 http-content" v-html="highlightHttpResponse(formatPrettyResponse())"></div>
              </template>
            
            <!-- Raw View -->
              <template v-else-if="currentTab.responseTab === 'raw'">
                <div class="line-numbers select-none">
                  <div v-for="n in getLineCount(currentTab.rawResponse)" :key="n" class="line-number">{{ n }}</div>
              </div>
                <div class="flex-1 overflow-auto p-2 http-content" v-html="highlightHttpResponse(currentTab.rawResponse)"></div>
              </template>
              
              <!-- Hex View -->
              <pre v-else-if="currentTab.responseTab === 'hex'" class="w-full h-full p-3 text-xs overflow-auto">{{ toHex(currentTab.rawResponse) }}</pre>
              
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
          <div class="bg-base-200 px-3 py-1 flex items-center justify-between border-t border-base-300">
            <div class="flex items-center gap-2">
              <div class="join">
                <input 
                  type="text" 
                  v-model="responseSearch"
                  placeholder="Search"
                  class="input input-bordered input-xs join-item w-40"
                  @keydown.enter="navigateSearch('response', 'next')"
                  @input="responseSearchIndex = 0"
                />
                <button 
                  class="btn btn-xs btn-ghost join-item"
                  :disabled="responseMatchCount === 0"
                  @click="navigateSearch('response', 'prev')"
                  title="上一个"
                >
                  <i class="fas fa-chevron-left"></i>
                </button>
                <button 
                  class="btn btn-xs btn-ghost join-item"
                  :disabled="responseMatchCount === 0"
                  @click="navigateSearch('response', 'next')"
                  title="下一个"
                >
                  <i class="fas fa-chevron-right"></i>
                </button>
              </div>
              <span class="text-xs text-base-content/50 min-w-16">{{ responseSearchResults }}</span>
            </div>
            <div class="text-xs text-base-content/70" v-if="currentTab.response">
              {{ formatBytes(currentTab.rawResponse?.length || 0) }} | {{ currentTab.response.responseTimeMs }} ms
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <i class="fas fa-redo text-6xl text-base-content/30 mb-4"></i>
        <p class="text-lg font-semibold text-base-content/70">Repeater</p>
        <p class="text-sm text-base-content/50 mt-2">点击 + 新建标签开始</p>
        <button @click="addTab" class="btn btn-primary btn-sm mt-4">
          <i class="fas fa-plus mr-2"></i>
          新建请求
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { dialog } from '@/composables/useDialog';

// Props
const props = defineProps<{
  initialRequest?: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  };
}>();

// 类型定义
interface HeaderItem {
  key: string;
  value: string;
}

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

// 响应式状态
const tabs = ref<RepeaterTab[]>([]);
const activeTabIndex = ref(0);
const isSending = ref(false);
const showTargetDialog = ref(false);
const requestTextarea = ref<HTMLTextAreaElement | null>(null);
const requestHighlight = ref<HTMLDivElement | null>(null);
const requestLineNumbers = ref<HTMLDivElement | null>(null);

// 搜索
const requestSearch = ref('');
const responseSearch = ref('');
const requestSearchIndex = ref(0);
const responseSearchIndex = ref(0);

// 布局模式
const STORAGE_KEY_LAYOUT = 'proxyRepeater.layoutMode';
const STORAGE_KEY_LEFT_WIDTH = 'proxyRepeater.leftPanelWidth';
const STORAGE_KEY_TOP_HEIGHT = 'proxyRepeater.topPanelHeight';

const layoutMode = ref<'horizontal' | 'vertical'>(
  (localStorage.getItem(STORAGE_KEY_LAYOUT) as 'horizontal' | 'vertical') || 'horizontal'
);
const leftPanelWidth = ref(parseInt(localStorage.getItem(STORAGE_KEY_LEFT_WIDTH) || '600'));
const topPanelHeight = ref(parseInt(localStorage.getItem(STORAGE_KEY_TOP_HEIGHT) || '350'));

// 拖拽相关
let isResizing = false;
let startX = 0;
let startY = 0;
let startWidth = 0;
let startHeight = 0;

// 计算属性
const currentTab = computed(() => {
  if (tabs.value.length === 0) return null;
  return tabs.value[activeTabIndex.value] || null;
});

const showPort = computed(() => {
  if (!currentTab.value) return false;
  const port = currentTab.value.targetPort;
  const useTls = currentTab.value.useTls;
  // 隐藏默认端口
  if (useTls && port === 443) return false;
  if (!useTls && port === 80) return false;
  return true;
});

const requestMatchCount = computed(() => {
  if (!requestSearch.value || !currentTab.value?.rawRequest) return 0;
  const regex = new RegExp(escapeRegex(requestSearch.value), 'gi');
  return (currentTab.value.rawRequest.match(regex) || []).length;
});

const responseMatchCount = computed(() => {
  if (!responseSearch.value || !currentTab.value?.rawResponse) return 0;
  const regex = new RegExp(escapeRegex(responseSearch.value), 'gi');
  return (currentTab.value.rawResponse.match(regex) || []).length;
});

const requestSearchResults = computed(() => {
  const count = requestMatchCount.value;
  if (count === 0) return '0 matches';
  return `${requestSearchIndex.value + 1}/${count}`;
});

const responseSearchResults = computed(() => {
  const count = responseMatchCount.value;
  if (count === 0) return '0 matches';
  return `${responseSearchIndex.value + 1}/${count}`;
});

// 方法
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
      
      // 构建 raw request
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
      // 忽略解析错误
    }
  } else {
    // 默认请求模板
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

// 同步请求编辑器滚动
function syncRequestScroll() {
  if (requestTextarea.value && requestHighlight.value && requestLineNumbers.value) {
    requestHighlight.value.scrollTop = requestTextarea.value.scrollTop;
    requestHighlight.value.scrollLeft = requestTextarea.value.scrollLeft;
    requestLineNumbers.value.scrollTop = requestTextarea.value.scrollTop;
  }
}

async function sendRequest() {
  if (!currentTab.value || isSending.value) return;
  
  if (!currentTab.value.targetHost || !currentTab.value.rawRequest.trim()) {
    dialog.toast.warning('请填写目标 Host 和请求内容');
    return;
  }
  
  isSending.value = true;
  currentTab.value.response = null;
  currentTab.value.rawResponse = '';
  
  try {
    // 规范化换行符
      let rawRequest = currentTab.value.rawRequest;
      rawRequest = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n');
      
    // 确保请求以 \r\n\r\n 结尾
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
      
      if (response.success && response.data) {
        currentTab.value.rawResponse = response.data.raw_response;
        
      // 解析响应
        const rawResp = response.data.raw_response;
        const headerEnd = rawResp.indexOf('\r\n\r\n');
        if (headerEnd > 0) {
          const headerPart = rawResp.substring(0, headerEnd);
          const bodyPart = rawResp.substring(headerEnd + 4);
          
          // 解析状态码
          const statusLine = headerPart.split('\r\n')[0];
        const statusMatch = statusLine.match(/HTTP\/[\d.]+\s+(\d+)/);
          const statusCode = statusMatch ? parseInt(statusMatch[1]) : 0;
          
          // 解析响应头
          const headers: Record<string, string> = {};
          const headerLines = headerPart.split('\r\n').slice(1);
          for (const line of headerLines) {
            const colonIndex = line.indexOf(':');
            if (colonIndex > 0) {
              headers[line.substring(0, colonIndex).trim()] = line.substring(colonIndex + 1).trim();
            }
          }
          
          currentTab.value.response = {
            statusCode,
            headers,
            body: bodyPart,
            responseTimeMs: response.data.response_time_ms,
          };
        }
        
        // 更新标签名
      currentTab.value.name = currentTab.value.targetHost;
      } else {
        dialog.toast.error(response.error || '请求失败');
      }
  } catch (error: any) {
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
  
  // 尝试格式化 JSON
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

function getLineCount(text: string): number {
  if (!text) return 1;
  return text.split(/\r\n|\r|\n/).length;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function highlightSearchTerm(html: string, searchTerm: string, currentIndex: number): string {
  if (!searchTerm) return html;
  
  // 在已转义的 HTML 中搜索
  const escapedSearch = escapeHtml(searchTerm);
  const regex = new RegExp(`(${escapeRegex(escapedSearch)})`, 'gi');
  
  let matchIndex = 0;
  return html.replace(regex, (match) => {
    const isCurrent = matchIndex === currentIndex;
    matchIndex++;
    return `<mark class="search-highlight${isCurrent ? ' current' : ''}">${match}</mark>`;
  });
}

// 搜索导航
function navigateSearch(type: 'request' | 'response', direction: 'prev' | 'next') {
  if (type === 'request') {
    const count = requestMatchCount.value;
    if (count === 0) return;
    
    if (direction === 'next') {
      requestSearchIndex.value = (requestSearchIndex.value + 1) % count;
    } else {
      requestSearchIndex.value = (requestSearchIndex.value - 1 + count) % count;
    }
    scrollToSearchMatch('request');
  } else {
    const count = responseMatchCount.value;
    if (count === 0) return;
    
    if (direction === 'next') {
      responseSearchIndex.value = (responseSearchIndex.value + 1) % count;
    } else {
      responseSearchIndex.value = (responseSearchIndex.value - 1 + count) % count;
    }
    scrollToSearchMatch('response');
  }
}

function scrollToSearchMatch(type: 'request' | 'response') {
  nextTick(() => {
    const container = document.querySelector(
      type === 'request' 
        ? '.request-content .http-content' 
        : '.response-content .http-content'
    );
    const currentMark = container?.querySelector('mark.search-highlight.current');
    if (currentMark) {
      currentMark.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  });
}

function highlightHttpRequest(raw: string, applySearch: boolean = true): string {
  if (!raw) return '';
  
  const lines = raw.split(/\r\n|\r|\n/);
  const result: string[] = [];
  let inBody = false;
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    if (i === 0) {
      // 请求行: GET /path HTTP/1.1
      const match = line.match(/^(\w+)\s+(.+?)\s+(HTTP\/[\d.]+)$/);
      if (match) {
        result.push(`<span class="http-method">${escapeHtml(match[1])}</span> <span class="http-path">${escapeHtml(match[2])}</span> <span class="http-version">${escapeHtml(match[3])}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else if (!inBody && line === '') {
      inBody = true;
      result.push('');
    } else if (!inBody) {
      // 请求头: Header-Name: value
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex);
        const value = line.substring(colonIndex + 1);
        result.push(`<span class="http-header-key">${escapeHtml(key)}</span>:<span class="http-header-value">${escapeHtml(value)}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else {
      // Body
      result.push(escapeHtml(line));
    }
  }
  
  let html = result.join('\n');
  
  // 应用搜索高亮
  if (applySearch && requestSearch.value) {
    html = highlightSearchTerm(html, requestSearch.value, requestSearchIndex.value);
  }
  
  return html;
}

function highlightHttpResponse(raw: string, applySearch: boolean = true): string {
  if (!raw) return '';
  
  const lines = raw.split(/\r\n|\r|\n/);
  const result: string[] = [];
  let inBody = false;
  let contentType = '';
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    if (i === 0) {
      // 状态行: HTTP/1.1 200 OK
      const match = line.match(/^(HTTP\/[\d.]+)\s+(\d+)\s*(.*)$/);
      if (match) {
        const statusClass = getStatusColorClass(parseInt(match[2]));
        result.push(`<span class="http-version">${escapeHtml(match[1])}</span> <span class="${statusClass}">${escapeHtml(match[2])}</span> <span class="http-status-text">${escapeHtml(match[3])}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else if (!inBody && line === '') {
      inBody = true;
      result.push('');
    } else if (!inBody) {
      // 响应头
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex);
        const value = line.substring(colonIndex + 1);
        if (key.toLowerCase() === 'content-type') {
          contentType = value.trim();
        }
        result.push(`<span class="http-header-key">${escapeHtml(key)}</span>:<span class="http-header-value">${escapeHtml(value)}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else {
      // Body - 根据 Content-Type 高亮
      if (contentType.includes('html')) {
        result.push(highlightHtml(line));
      } else if (contentType.includes('json')) {
        result.push(highlightJson(line));
      } else {
        result.push(escapeHtml(line));
      }
    }
  }
  
  let html = result.join('\n');
  
  // 应用搜索高亮
  if (applySearch && responseSearch.value) {
    html = highlightSearchTerm(html, responseSearch.value, responseSearchIndex.value);
  }
  
  return html;
}

function getStatusColorClass(status: number): string {
  if (status >= 200 && status < 300) return 'http-status-2xx';
  if (status >= 300 && status < 400) return 'http-status-3xx';
  if (status >= 400 && status < 500) return 'http-status-4xx';
  if (status >= 500) return 'http-status-5xx';
  return '';
}

function highlightHtml(line: string): string {
  // 简单的 HTML 高亮
  return line
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/&lt;(\/?)([\w-]+)/g, '&lt;$1<span class="html-tag">$2</span>')
    .replace(/\s([\w-]+)=/g, ' <span class="html-attr">$1</span>=')
    .replace(/="([^"]*)"/g, '="<span class="html-value">$1</span>"');
}

function highlightJson(line: string): string {
  return escapeHtml(line)
    .replace(/"([^"]+)":/g, '<span class="json-key">"$1"</span>:')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/:\s*(\d+\.?\d*)/g, ': <span class="json-number">$1</span>')
    .replace(/:\s*(true|false|null)/g, ': <span class="json-keyword">$1</span>');
}

// 拖拽调整面板大小
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

// 公开方法
function addRequestFromHistory(request: { method: string; url: string; headers: Record<string, string>; body?: string }) {
  const tab = createTab(request);
  tabs.value.push(tab);
  activeTabIndex.value = tabs.value.length - 1;
}

defineExpose({
  addRequestFromHistory,
});

// 监听
watch(() => props.initialRequest, (newRequest) => {
  if (newRequest) {
    addRequestFromHistory(newRequest);
  }
}, { immediate: true });

watch(layoutMode, (newMode) => {
  localStorage.setItem(STORAGE_KEY_LAYOUT, newMode);
});

// 搜索词变化时重置索引
watch(requestSearch, () => {
  requestSearchIndex.value = 0;
});

watch(responseSearch, () => {
  responseSearchIndex.value = 0;
});

// 生命周期
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
/* 代码字体统一使用系统配置 */
textarea,
pre {
  resize: none;
  font-size: var(--font-size-base, 14px) !important;
  line-height: 1.5 !important;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace !important;
}
textarea:focus {
  outline: none;
  box-shadow: none;
}

/* 透明代码编辑器 */
.code-editor {
  background: transparent;
  color: transparent;
  caret-color: oklch(var(--bc));
  white-space: pre-wrap;
  word-break: break-all;
  overflow: auto;
  z-index: 1;
}
.code-editor::selection {
  background: oklch(var(--p) / 0.3);
}
.code-editor::placeholder {
  color: oklch(var(--bc) / 0.3);
}

/* 行号 */
.line-numbers {
  background: oklch(var(--b2));
  border-right: 1px solid oklch(var(--b3));
  padding: 0.5rem 0;
  min-width: 3rem;
  text-align: right;
  overflow: hidden;
}
.line-number {
  padding: 0 0.5rem;
  font-size: var(--font-size-base, 14px);
  line-height: 1.5;
  color: oklch(var(--bc) / 0.4);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* HTTP 内容 */
.http-content {
  white-space: pre-wrap;
  word-break: break-all;
  font-size: var(--font-size-base, 14px);
  line-height: 1.5;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* HTTP 语法高亮 */
.http-content :deep(.http-method) {
  color: #c41a16;
  font-weight: 600;
}
.http-content :deep(.http-path) {
  color: #1c00cf;
}
.http-content :deep(.http-version) {
  color: #5c5c5c;
}
.http-content :deep(.http-header-key) {
  color: #c41a16;
}
.http-content :deep(.http-header-value) {
  color: #000;
}
.http-content :deep(.http-status-2xx) {
  color: #18794e;
  font-weight: 600;
}
.http-content :deep(.http-status-3xx) {
  color: #0550ae;
  font-weight: 600;
}
.http-content :deep(.http-status-4xx) {
  color: #cf222e;
  font-weight: 600;
}
.http-content :deep(.http-status-5xx) {
  color: #8250df;
  font-weight: 600;
}
.http-content :deep(.http-status-text) {
  color: #5c5c5c;
}

/* HTML 语法高亮 */
.http-content :deep(.html-tag) {
  color: #0550ae;
}
.http-content :deep(.html-attr) {
  color: #c41a16;
}
.http-content :deep(.html-value) {
  color: #0a3069;
}

/* JSON 语法高亮 */
.http-content :deep(.json-key) {
  color: #0550ae;
}
.http-content :deep(.json-string) {
  color: #0a3069;
}
.http-content :deep(.json-number) {
  color: #0550ae;
}
.http-content :deep(.json-keyword) {
  color: #cf222e;
}

/* 搜索高亮 */
.http-content :deep(.search-highlight) {
  background-color: #fef08a;
  color: #000;
  border-radius: 2px;
  padding: 0 1px;
}
.http-content :deep(.search-highlight.current) {
  background-color: #f97316;
  color: #fff;
}

/* 暗色主题适配 */
[data-theme="dark"] .http-content :deep(.http-method),
[data-theme="dark"] .http-content :deep(.http-header-key),
[data-theme="dark"] .http-content :deep(.html-attr) {
  color: #ff7b72;
}
[data-theme="dark"] .http-content :deep(.http-path),
[data-theme="dark"] .http-content :deep(.html-tag),
[data-theme="dark"] .http-content :deep(.json-key),
[data-theme="dark"] .http-content :deep(.json-number) {
  color: #79c0ff;
}
[data-theme="dark"] .http-content :deep(.http-version),
[data-theme="dark"] .http-content :deep(.http-status-text) {
  color: #8b949e;
}
[data-theme="dark"] .http-content :deep(.http-header-value) {
  color: #c9d1d9;
}
[data-theme="dark"] .http-content :deep(.http-status-2xx) {
  color: #3fb950;
}
[data-theme="dark"] .http-content :deep(.http-status-3xx) {
  color: #58a6ff;
}
[data-theme="dark"] .http-content :deep(.http-status-4xx) {
  color: #f85149;
}
[data-theme="dark"] .http-content :deep(.http-status-5xx) {
  color: #a371f7;
}
[data-theme="dark"] .http-content :deep(.html-value),
[data-theme="dark"] .http-content :deep(.json-string) {
  color: #a5d6ff;
}
[data-theme="dark"] .http-content :deep(.json-keyword) {
  color: #ff7b72;
}
[data-theme="dark"] .http-content :deep(.search-highlight) {
  background-color: #854d0e;
  color: #fef9c3;
}
[data-theme="dark"] .http-content :deep(.search-highlight.current) {
  background-color: #ea580c;
  color: #fff;
}
</style>
