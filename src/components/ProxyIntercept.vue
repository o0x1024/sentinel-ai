<template>
  <div class="flex flex-col h-full">
    <!-- Intercept Controls Header -->
    <div class="bg-base-200 border-b border-base-300 p-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="font-semibold text-base">
            <i class="fas fa-shield-alt mr-2"></i>
            Intercept
          </h2>
          <div class="badge badge-sm" :class="interceptEnabled ? 'badge-success' : 'badge-error'">
            <i :class="['fas fa-circle mr-2', interceptEnabled ? 'text-success-content' : 'text-error-content']"></i>
            {{ interceptEnabled ? 'Intercept is on' : 'Intercept is off' }}
          </div>
        </div>
        
        <div class="flex items-center gap-2">
          <button 
            @click="toggleIntercept"
            :class="['btn btn-sm', interceptEnabled ? 'btn-error' : 'btn-success']"
          >
            <i :class="['fas', interceptEnabled ? 'fa-stop' : 'fa-play', 'mr-1']"></i>
            {{ interceptEnabled ? 'Turn off' : 'Turn on' }}
          </button>
          
          <div class="divider divider-horizontal mx-0"></div>
          
          <div class="stats stats-horizontal shadow-sm">
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">代理状态</div>
              <div class="stat-value text-sm" :class="proxyStatus.running ? 'text-success' : 'text-error'">
                {{ proxyStatus.running ? '运行中' : '已停止' }}
              </div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">端口</div>
              <div class="stat-value text-sm">{{ proxyStatus.port || 8080 }}</div>
            </div>
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs">拦截队列</div>
              <div class="stat-value text-sm">{{ interceptedRequests.length }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- No Request Intercepted State -->
      <div v-if="!currentRequest" class="flex-1 flex items-center justify-center bg-base-100">
        <div class="text-center">
          <i class="fas fa-hourglass-half text-6xl text-base-content/30 mb-4"></i>
          <p class="text-lg font-semibold text-base-content/70">等待拦截请求...</p>
          <p class="text-sm text-base-content/50 mt-2">
            {{ interceptEnabled ? '当前拦截已启用' : '请启用拦截功能' }}
          </p>
          <p class="text-sm text-base-content/50">
            配置浏览器代理为 127.0.0.1:{{ proxyStatus.port || 8080 }}
          </p>
        </div>
      </div>

      <!-- Request Display Area -->
      <div v-else class="flex-1 flex flex-col overflow-hidden">
        <!-- Action Buttons -->
        <div class="bg-base-200 border-b border-base-300 p-3 flex items-center gap-3">
          <button 
            @click="forwardRequest"
            class="btn btn-success btn-sm"
          >
            <i class="fas fa-arrow-right mr-2"></i>
            Forward
          </button>
          
          <button 
            @click="dropRequest"
            class="btn btn-error btn-sm"
          >
            <i class="fas fa-times mr-2"></i>
            Drop
          </button>
          
          <div class="divider divider-horizontal mx-0"></div>
          
          <button 
            @click="editRequest"
            class="btn btn-outline btn-sm"
            :disabled="!isEditable"
          >
            <i class="fas fa-edit mr-2"></i>
            {{ isEditable ? 'Save' : 'Edit' }}
          </button>
          
          <button 
            @click="openInBrowser"
            class="btn btn-outline btn-sm"
          >
            <i class="fas fa-external-link-alt mr-2"></i>
            Open browser
          </button>
          
          <div class="flex-1"></div>
          
          <div class="text-sm text-base-content/70">
            <i class="fas fa-info-circle mr-1"></i>
            {{ currentRequest.method }} {{ currentRequest.url }}
          </div>
        </div>

        <!-- Request Content Tabs -->
        <div class="tabs tabs-boxed bg-base-200 border-b border-base-300 px-3">
          <a 
            :class="['tab', activeTab === 'raw' ? 'tab-active' : '']"
            @click="activeTab = 'raw'"
          >
            Raw
          </a>
          <a 
            :class="['tab', activeTab === 'pretty' ? 'tab-active' : '']"
            @click="activeTab = 'pretty'"
          >
            Pretty
          </a>
          <a 
            :class="['tab', activeTab === 'hex' ? 'tab-active' : '']"
            @click="activeTab = 'hex'"
          >
            Hex
          </a>
        </div>

        <!-- Request Content -->
        <div class="flex-1 overflow-auto bg-base-100 p-4">
          <!-- Raw View -->
          <div v-if="activeTab === 'raw'" class="h-full">
            <textarea 
              v-model="requestContent"
              :readonly="!isEditable"
              class="textarea textarea-bordered w-full h-full font-mono text-sm"
              spellcheck="false"
            ></textarea>
          </div>

          <!-- Pretty View -->
          <div v-else-if="activeTab === 'pretty'" class="space-y-4">
            <div class="card bg-base-200">
              <div class="card-body p-4">
                <h3 class="font-semibold mb-2">Request Line</h3>
                <div class="font-mono text-sm">
                  {{ currentRequest.method }} {{ currentRequest.path }} {{ currentRequest.protocol }}
                </div>
              </div>
            </div>

            <div class="card bg-base-200">
              <div class="card-body p-4">
                <h3 class="font-semibold mb-2">Headers</h3>
                <div class="space-y-1">
                  <div 
                    v-for="(value, key) in currentRequest.headers" 
                    :key="key"
                    class="flex text-sm font-mono"
                  >
                    <span class="font-semibold text-primary w-1/4">{{ key }}:</span>
                    <span class="flex-1 break-all">{{ value }}</span>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="currentRequest.body" class="card bg-base-200">
              <div class="card-body p-4">
                <h3 class="font-semibold mb-2">Body</h3>
                <pre class="font-mono text-sm whitespace-pre-wrap">{{ currentRequest.body }}</pre>
              </div>
            </div>
          </div>

          <!-- Hex View -->
          <div v-else-if="activeTab === 'hex'" class="font-mono text-xs">
            <pre>{{ hexView }}</pre>
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
const interceptedRequests = ref<InterceptedRequest[]>([]);
const currentRequest = ref<InterceptedRequest | null>(null);
const activeTab = ref<'raw' | 'pretty' | 'hex'>('raw');
const isEditable = ref(false);
const requestContent = ref('');

// 计算属性
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

// 方法
async function toggleIntercept() {
  const newState = !interceptEnabled.value;
  
  // TODO: 后端需要实现 set_intercept_enabled 命令来真正启用/禁用拦截
  // 目前只在前端切换状态，实际的拦截功能需要后端支持
  
  try {
    // 暂时注释掉后端调用，避免命令不存在的错误
    // await invoke('set_intercept_enabled', { enabled: newState });
    
    // 直接更新前端状态
    interceptEnabled.value = newState;
    dialog.toast.info(
      interceptEnabled.value 
        ? '拦截已启用（前端状态，后端功能待实现）' 
        : '拦截已禁用（前端状态）'
    );
    
    console.log('[ProxyIntercept] Intercept toggled:', interceptEnabled.value);
  } catch (error: any) {
    console.error('[ProxyIntercept] Failed to toggle intercept:', error);
    dialog.toast.error(`切换拦截状态失败: ${error}`);
  }
}

async function forwardRequest() {
  if (!currentRequest.value) return;
  
  try {
    await invoke('forward_intercepted_request', { 
      requestId: currentRequest.value.id,
      modifiedContent: isEditable.value ? requestContent.value : undefined
    });
    
    // 移除当前请求，显示下一个
    interceptedRequests.value = interceptedRequests.value.filter(
      r => r.id !== currentRequest.value!.id
    );
    currentRequest.value = interceptedRequests.value[0] || null;
    if (currentRequest.value) {
      loadRequestContent(currentRequest.value);
    }
    isEditable.value = false;
  } catch (error: any) {
    console.error('Failed to forward request:', error);
    dialog.toast.error(`转发请求失败: ${error}`);
  }
}

async function dropRequest() {
  if (!currentRequest.value) return;
  
  try {
    await invoke('drop_intercepted_request', { 
      requestId: currentRequest.value.id 
    });
    
    // 移除当前请求，显示下一个
    interceptedRequests.value = interceptedRequests.value.filter(
      r => r.id !== currentRequest.value!.id
    );
    currentRequest.value = interceptedRequests.value[0] || null;
    if (currentRequest.value) {
      loadRequestContent(currentRequest.value);
    }
    isEditable.value = false;
  } catch (error: any) {
    console.error('Failed to drop request:', error);
    dialog.toast.error(`丢弃请求失败: ${error}`);
  }
}

function editRequest() {
  isEditable.value = !isEditable.value;
  if (!isEditable.value) {
    // 保存编辑
    dialog.toast.info('编辑已保存，点击 Forward 发送修改后的请求');
  }
}

function openInBrowser() {
  if (!currentRequest.value) return;
  window.open(currentRequest.value.url, '_blank');
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

async function refreshStatus() {
  try {
    const response = await invoke<any>('get_proxy_status');
    if (response.success && response.data) {
      proxyStatus.value = response.data;
    }
  } catch (error: any) {
    console.error('Failed to refresh proxy status:', error);
  }
}

// 设置事件监听
async function setupEventListeners() {
  // 监听代理状态事件
  unlistenProxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    proxyStatus.value = event.payload;
  });
  
  // 监听拦截请求事件
  unlistenInterceptRequest = await listen<InterceptedRequest>('intercept:request', (event) => {
    const request = event.payload;
    interceptedRequests.value.push(request);
    
    // 如果没有当前请求，设置为当前请求
    if (!currentRequest.value) {
      currentRequest.value = request;
      loadRequestContent(request);
    }
  });
}

// 生命周期
onMounted(async () => {
  await setupEventListeners();
  await refreshStatus();
});

onUnmounted(() => {
  if (unlistenProxyStatus) unlistenProxyStatus();
  if (unlistenInterceptRequest) unlistenInterceptRequest();
});

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyIntercept] Refresh triggered by parent');
  await refreshStatus();
  // TODO: 刷新拦截状态（需要后端实现 get_intercept_status 命令）
  // 目前拦截状态由前端维护，不需要从后端获取
});
</script>

<style scoped>
textarea {
  resize: none;
}
</style>
