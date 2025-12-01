<template>
  <div class="flex flex-col h-full bg-base-100">
    <!-- Tabs Header -->
    <div class="bg-base-200 border-b border-base-300 p-2 flex items-center gap-2">
      <div class="flex-1 flex items-center gap-1 overflow-x-auto">
        <div 
          v-for="(tab, index) in tabs" 
          :key="tab.id"
          class="flex items-center gap-1 px-3 py-1 rounded-t cursor-pointer border border-base-300 border-b-0 relative"
          :class="activeTabIndex === index ? 'bg-base-100' : 'bg-base-200 hover:bg-base-300'"
          @click="selectTab(index)"
        >
          <span class="text-sm truncate max-w-32" :title="tab.name">{{ tab.name }}</span>
          <button 
            @click.stop="closeTab(index)"
            class="btn btn-xs btn-ghost btn-circle ml-1"
            title="关闭"
          >
            <i class="fas fa-times text-xs"></i>
          </button>
        </div>
        <button 
          @click="addTab"
          class="btn btn-xs btn-ghost btn-circle"
          title="新建标签"
        >
          <i class="fas fa-plus"></i>
        </button>
      </div>
    </div>

    <!-- Tab Content -->
    <div v-if="currentTab" class="flex-1 flex flex-col overflow-hidden">
      <!-- Request Builder -->
      <div class="flex flex-col" :style="{ height: topPanelHeight + 'px' }">
        <!-- URL Bar / Raw Target Bar -->
        <div class="bg-base-200 p-3 border-b border-base-300 flex items-center gap-2">
          <!-- 发送模式切换 -->
          <div class="btn-group">
            <button 
              :class="['btn btn-xs', !currentTab.rawMode ? 'btn-primary' : 'btn-ghost']"
              @click="currentTab.rawMode = false"
              title="结构化请求"
            >
              <i class="fas fa-list"></i>
            </button>
            <button 
              :class="['btn btn-xs', currentTab.rawMode ? 'btn-primary' : 'btn-ghost']"
              @click="switchToRawMode"
              title="Raw 请求"
            >
              <i class="fas fa-code"></i>
            </button>
          </div>
          
          <template v-if="!currentTab.rawMode">
            <!-- 结构化模式 -->
            <select v-model="currentTab.method" class="select select-bordered select-sm w-28">
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="DELETE">DELETE</option>
              <option value="PATCH">PATCH</option>
              <option value="HEAD">HEAD</option>
              <option value="OPTIONS">OPTIONS</option>
            </select>
            <input 
              v-model="currentTab.url" 
              type="text" 
              placeholder="输入请求 URL" 
              class="input input-bordered input-sm flex-1"
              @keydown.enter="sendRequest"
            />
          </template>
          <template v-else>
            <!-- Raw 模式：目标配置 -->
            <input 
              v-model="currentTab.targetHost" 
              type="text" 
              placeholder="Host" 
              class="input input-bordered input-sm w-48"
            />
            <input 
              v-model.number="currentTab.targetPort" 
              type="number" 
              placeholder="Port" 
              class="input input-bordered input-sm w-20"
              min="1"
              max="65535"
            />
            <label class="label cursor-pointer gap-1">
              <input 
                type="checkbox" 
                v-model="currentTab.useTls"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text text-xs">TLS</span>
            </label>
          </template>
          
          <button 
            @click="sendRequest"
            class="btn btn-primary btn-sm"
            :disabled="isSending || (!currentTab.rawMode && !currentTab.url) || (currentTab.rawMode && !currentTab.targetHost)"
          >
            <i :class="['fas', isSending ? 'fa-spinner fa-spin' : 'fa-paper-plane', 'mr-2']"></i>
            Send
          </button>
        </div>

        <!-- Request Content -->
        <div class="flex-1 flex flex-col overflow-hidden">
          <div class="tabs tabs-boxed bg-base-200 px-3 py-1">
            <a 
              :class="['tab tab-sm', currentTab.requestTab === 'raw' ? 'tab-active' : '']"
              @click="currentTab.requestTab = 'raw'"
            >
              Raw
            </a>
            <a 
              :class="['tab tab-sm', currentTab.requestTab === 'headers' ? 'tab-active' : '']"
              @click="currentTab.requestTab = 'headers'"
            >
              Headers
            </a>
            <a 
              :class="['tab tab-sm', currentTab.requestTab === 'body' ? 'tab-active' : '']"
              @click="currentTab.requestTab = 'body'"
            >
              Body
            </a>
          </div>

          <div class="flex-1 overflow-auto p-3">
            <!-- Headers Editor -->
            <div v-if="currentTab.requestTab === 'headers'" class="space-y-2">
              <div 
                v-for="(header, index) in currentTab.headers" 
                :key="index"
                class="flex items-center gap-2"
              >
                <input 
                  v-model="header.key" 
                  placeholder="Header Name" 
                  class="input input-bordered input-sm w-1/3"
                />
                <input 
                  v-model="header.value" 
                  placeholder="Header Value" 
                  class="input input-bordered input-sm flex-1"
                />
                <button 
                  @click="removeHeader(index)"
                  class="btn btn-ghost btn-sm btn-circle"
                >
                  <i class="fas fa-minus"></i>
                </button>
              </div>
              <button @click="addHeader" class="btn btn-ghost btn-sm">
                <i class="fas fa-plus mr-2"></i>
                Add Header
              </button>
            </div>

            <!-- Body Editor -->
            <div v-else-if="currentTab.requestTab === 'body'" class="h-full">
              <textarea 
                v-model="currentTab.body"
                class="textarea textarea-bordered w-full h-full font-mono text-sm"
                placeholder="请求体内容（JSON、Form Data 等）"
              ></textarea>
            </div>

            <!-- Raw Request View -->
            <div v-else-if="currentTab.requestTab === 'raw'" class="h-full">
              <textarea 
                v-model="currentTab.rawRequest"
                class="textarea textarea-bordered w-full h-full font-mono text-sm"
                :placeholder="currentTab.rawMode 
                  ? 'GET /path HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\n\r\n'
                  : '完整原始请求（编辑后会自动解析到其他字段）'"
                @input="!currentTab.rawMode && parseRawRequest()"
              ></textarea>
            </div>
          </div>
        </div>
      </div>

      <!-- Resizer -->
      <div 
        class="h-1 bg-base-300 cursor-row-resize hover:bg-primary/50"
        @mousedown="startResize"
      ></div>

      <!-- Response Viewer -->
      <div class="flex-1 flex flex-col overflow-hidden min-h-0">
        <div class="bg-base-200 px-3 py-2 border-b border-base-300 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="font-semibold text-sm">Response</span>
            <template v-if="currentTab.response || currentTab.rawResponse">
              <span 
                v-if="currentTab.response"
                class="badge badge-sm"
                :class="getStatusClass(currentTab.response.statusCode)"
              >
                {{ currentTab.response.statusCode }}
              </span>
              <span class="text-xs text-base-content/70">
                {{ currentTab.response?.responseTimeMs || 0 }}ms
              </span>
              <span class="text-xs text-base-content/70">
                {{ formatBytes(currentTab.rawMode ? currentTab.rawResponse.length : (currentTab.response?.body?.length || 0)) }}
              </span>
            </template>
          </div>
          <div class="tabs tabs-boxed tabs-xs">
            <a 
              :class="['tab', currentTab.responseTab === 'pretty' ? 'tab-active' : '']"
              @click="currentTab.responseTab = 'pretty'"
            >
              Pretty
            </a>
            <a 
              :class="['tab', currentTab.responseTab === 'raw' ? 'tab-active' : '']"
              @click="currentTab.responseTab = 'raw'"
            >
              Raw
            </a>
            <a 
              :class="['tab', currentTab.responseTab === 'headers' ? 'tab-active' : '']"
              @click="currentTab.responseTab = 'headers'"
            >
              Headers
            </a>
            <a 
              v-if="currentTab.rawMode && currentTab.rawResponse"
              :class="['tab', currentTab.responseTab === 'fullraw' ? 'tab-active' : '']"
              @click="currentTab.responseTab = 'fullraw'"
            >
              Full Raw
            </a>
          </div>
        </div>

        <div class="flex-1 overflow-auto p-3 font-mono text-sm">
          <template v-if="currentTab.response || currentTab.rawResponse">
            <!-- Pretty View -->
            <pre v-if="currentTab.responseTab === 'pretty'" class="whitespace-pre-wrap break-all">{{ formatResponseBody(currentTab.response?.body || '') }}</pre>
            
            <!-- Raw View -->
            <pre v-else-if="currentTab.responseTab === 'raw'" class="whitespace-pre-wrap break-all">{{ currentTab.response?.body || '' }}</pre>
            
            <!-- Headers View -->
            <div v-else-if="currentTab.responseTab === 'headers'" class="space-y-1">
              <div 
                v-for="(value, key) in (currentTab.response?.headers || {})" 
                :key="key"
                class="flex"
              >
                <span class="font-semibold text-primary w-1/4">{{ key }}:</span>
                <span class="flex-1 break-all">{{ value }}</span>
              </div>
            </div>
            
            <!-- Full Raw View (仅 Raw 模式) -->
            <pre v-else-if="currentTab.responseTab === 'fullraw'" class="whitespace-pre-wrap break-all">{{ currentTab.rawResponse }}</pre>
          </template>
          <div v-else class="flex items-center justify-center h-full text-base-content/50">
            <div class="text-center">
              <i class="fas fa-inbox text-4xl mb-2"></i>
              <p>点击 Send 发送请求</p>
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
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
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
  method: string;
  url: string;
  headers: HeaderItem[];
  body: string;
  rawRequest: string;
  requestTab: 'headers' | 'body' | 'raw';
  responseTab: 'pretty' | 'raw' | 'headers' | 'fullraw';
  response: ReplayResponse | null;
  // Raw 模式相关
  rawMode: boolean;
  targetHost: string;
  targetPort: number;
  useTls: boolean;
  rawResponse: string;
}

// 响应式状态
const tabs = ref<RepeaterTab[]>([]);
const activeTabIndex = ref(0);
const isSending = ref(false);
const topPanelHeight = ref(350);

// 拖拽相关
let isResizing = false;
let startY = 0;
let startHeight = 0;

// 计算属性
const currentTab = computed(() => {
  if (tabs.value.length === 0) return null;
  return tabs.value[activeTabIndex.value] || null;
});

// 方法
function generateId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function createTab(name?: string, request?: { method: string; url: string; headers: Record<string, string>; body?: string }): RepeaterTab {
  const headers: HeaderItem[] = request?.headers 
    ? Object.entries(request.headers).map(([key, value]) => ({ key, value }))
    : [{ key: 'User-Agent', value: 'Sentinel-AI/1.0' }];
  
  // 从 URL 解析 host 和 port
  let targetHost = '';
  let targetPort = 443;
  let useTls = true;
  if (request?.url) {
    try {
      const urlObj = new URL(request.url);
      targetHost = urlObj.hostname;
      targetPort = urlObj.port ? parseInt(urlObj.port) : (urlObj.protocol === 'https:' ? 443 : 80);
      useTls = urlObj.protocol === 'https:';
    } catch {
      // 忽略解析错误
    }
  }
  
  // 构建初始 raw 请求
  let initialRawRequest = '';
  if (request?.url) {
    try {
      const urlObj = new URL(request.url);
      const path = urlObj.pathname + urlObj.search;
      initialRawRequest = `${request.method || 'GET'} ${path} HTTP/1.1\r\n`;
      initialRawRequest += `Host: ${urlObj.hostname}\r\n`;
      for (const h of headers) {
        if (h.key.trim() && h.key.toLowerCase() !== 'host') {
          initialRawRequest += `${h.key}: ${h.value}\r\n`;
        }
      }
      initialRawRequest += '\r\n';
      if (request.body) {
        initialRawRequest += request.body;
      }
    } catch {
      // 忽略解析错误
    }
  }
  
  return {
    id: generateId(),
    name: name || `Request ${tabs.value.length + 1}`,
    method: request?.method || 'GET',
    url: request?.url || '',
    headers,
    body: request?.body || '',
    rawRequest: initialRawRequest,
    requestTab: 'raw',
    responseTab: 'pretty',
    response: null,
    rawMode: true,
    targetHost,
    targetPort,
    useTls,
    rawResponse: '',
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

function addHeader() {
  if (!currentTab.value) return;
  currentTab.value.headers.push({ key: '', value: '' });
}

function removeHeader(index: number) {
  if (!currentTab.value) return;
  currentTab.value.headers.splice(index, 1);
}

// 切换到 Raw 模式时，自动将结构化请求转换为 raw 格式
function switchToRawMode() {
  if (!currentTab.value) return;
  
  // 如果已经是 raw 模式，直接返回
  if (currentTab.value.rawMode) return;
  
  // 构建 raw 请求
  buildRawRequest();
  
  // 切换到 raw 模式
  currentTab.value.rawMode = true;
  
  // 自动切换到 Raw 标签页
  currentTab.value.requestTab = 'raw';
}

// 从结构化数据构建 raw 请求
function buildRawRequest() {
  if (!currentTab.value) return;
  
  const tab = currentTab.value;
  
  // 解析 URL 获取路径和 host
  let path = '/';
  let host = '';
  let port = 443;
  let useTls = true;
  
  if (tab.url) {
    try {
      const urlObj = new URL(tab.url);
      path = urlObj.pathname + urlObj.search;
      host = urlObj.hostname;
      port = urlObj.port ? parseInt(urlObj.port) : (urlObj.protocol === 'https:' ? 443 : 80);
      useTls = urlObj.protocol === 'https:';
    } catch {
      // 如果 URL 解析失败，尝试提取路径
      path = tab.url.startsWith('/') ? tab.url : '/' + tab.url;
    }
  }
  
  // 更新目标配置
  if (host) {
    tab.targetHost = host;
    tab.targetPort = port;
    tab.useTls = useTls;
  }
  
  // 构建请求行
  let rawRequest = `${tab.method} ${path} HTTP/1.1\r\n`;
  
  // 添加 Host 头（如果没有的话）
  const hasHost = tab.headers.some(h => h.key.toLowerCase() === 'host');
  if (!hasHost && host) {
    rawRequest += `Host: ${host}\r\n`;
  }
  
  // 添加其他请求头
  for (const header of tab.headers) {
    if (header.key.trim()) {
      rawRequest += `${header.key}: ${header.value}\r\n`;
    }
  }
  
  // 添加空行
  rawRequest += '\r\n';
  
  // 添加请求体
  if (tab.body) {
    rawRequest += tab.body;
  }
  
  tab.rawRequest = rawRequest;
}

function parseRawRequest() {
  if (!currentTab.value) return;
  
  const raw = currentTab.value.rawRequest;
  const lines = raw.split('\n');
  
  if (lines.length === 0) return;
  
  // 解析请求行
  const requestLine = lines[0].trim();
  const [method, path] = requestLine.split(' ');
  if (method && path) {
    currentTab.value.method = method;
    // 如果是相对路径，尝试构建完整 URL
    if (path.startsWith('/')) {
      const hostHeader = currentTab.value.headers.find(h => h.key.toLowerCase() === 'host');
      if (hostHeader) {
        currentTab.value.url = `https://${hostHeader.value}${path}`;
      }
    } else if (path.startsWith('http')) {
      currentTab.value.url = path;
    }
  }
  
  // 解析请求头
  const headers: HeaderItem[] = [];
  let bodyStart = -1;
  
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === '') {
      bodyStart = i + 1;
      break;
    }
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      headers.push({
        key: line.substring(0, colonIndex).trim(),
        value: line.substring(colonIndex + 1).trim(),
      });
    }
  }
  
  currentTab.value.headers = headers;
  
  // 解析请求体
  if (bodyStart > 0 && bodyStart < lines.length) {
    currentTab.value.body = lines.slice(bodyStart).join('\n');
  }
}

async function sendRequest() {
  if (!currentTab.value || isSending.value) return;
  
  isSending.value = true;
  currentTab.value.response = null;
  currentTab.value.rawResponse = '';
  
  try {
    if (currentTab.value.rawMode) {
      // Raw 模式：直接通过 TCP socket 发送原始请求
      if (!currentTab.value.targetHost || !currentTab.value.rawRequest.trim()) {
        dialog.toast.warning('请填写目标 Host 和 Raw 请求内容');
        return;
      }
      
      // 规范化换行符：统一转换为 \r\n
      let rawRequest = currentTab.value.rawRequest;
      // 先将 \r\n 替换为 \n，再将 \n 替换为 \r\n，避免 \r\r\n 的情况
      rawRequest = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\n/g, '\r\n');
      
      // 确保请求以 \r\n\r\n 结尾（请求头和请求体之间的空行）
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
        
        // 尝试解析响应
        const rawResp = response.data.raw_response;
        const headerEnd = rawResp.indexOf('\r\n\r\n');
        if (headerEnd > 0) {
          const headerPart = rawResp.substring(0, headerEnd);
          const bodyPart = rawResp.substring(headerEnd + 4);
          
          // 解析状态码
          const statusLine = headerPart.split('\r\n')[0];
          const statusMatch = statusLine.match(/HTTP\/\d\.\d\s+(\d+)/);
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
        currentTab.value.name = `${currentTab.value.targetHost}:${currentTab.value.targetPort}`;
      } else {
        dialog.toast.error(response.error || '请求失败');
      }
    } else {
      // 结构化模式
      if (!currentTab.value.url) {
        dialog.toast.warning('请输入请求 URL');
        return;
      }
      
      // 构建请求头
      const headers: Record<string, string> = {};
      for (const h of currentTab.value.headers) {
        if (h.key.trim()) {
          headers[h.key] = h.value;
        }
      }
      
      const response = await invoke<any>('replay_request', {
        method: currentTab.value.method,
        url: currentTab.value.url,
        headers: Object.keys(headers).length > 0 ? headers : null,
        body: currentTab.value.body || null,
      });
      
      if (response.success && response.data) {
        currentTab.value.response = {
          statusCode: response.data.status_code,
          headers: response.data.headers,
          body: response.data.body,
          responseTimeMs: response.data.response_time_ms,
        };
        
        // 更新标签名为 URL 的 host
        try {
          const urlObj = new URL(currentTab.value.url);
          currentTab.value.name = urlObj.host;
        } catch {
          // 忽略
        }
      } else {
        dialog.toast.error(response.error || '请求失败');
      }
    }
  } catch (error: any) {
    console.error('Failed to send request:', error);
    dialog.toast.error(`发送请求失败: ${error}`);
  } finally {
    isSending.value = false;
  }
}

function formatResponseBody(body: string): string {
  if (!body) return '';
  
  // 尝试格式化 JSON
  try {
    const json = JSON.parse(body);
    return JSON.stringify(json, null, 2);
  } catch {
    return body;
  }
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

// 拖拽调整高度
function startResize(event: MouseEvent) {
  isResizing = true;
  startY = event.clientY;
  startHeight = topPanelHeight.value;
  
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';
}

function handleResize(event: MouseEvent) {
  if (!isResizing) return;
  
  const diff = event.clientY - startY;
  topPanelHeight.value = Math.max(200, Math.min(startHeight + diff, window.innerHeight - 300));
}

function stopResize() {
  isResizing = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

// 公开方法：从外部添加请求
function addRequestFromHistory(request: { method: string; url: string; headers: Record<string, string>; body?: string }) {
  const tab = createTab(undefined, request);
  try {
    const urlObj = new URL(request.url);
    tab.name = urlObj.host;
  } catch {
    // 忽略
  }
  tabs.value.push(tab);
  activeTabIndex.value = tabs.value.length - 1;
}

// 暴露方法给父组件
defineExpose({
  addRequestFromHistory,
});

// 监听初始请求
watch(() => props.initialRequest, (newRequest) => {
  if (newRequest) {
    addRequestFromHistory(newRequest);
  }
}, { immediate: true });

// 生命周期
onMounted(() => {
  // 如果没有标签，创建一个默认标签
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
textarea {
  resize: none;
}
</style>

