<template>
  <div class="card bg-base-100 shadow-xl">
    <div class="card-body">
      <h2 class="card-title text-base">
        <i class="fas fa-shield-alt mr-2"></i>
        被动扫描代理
      </h2>
      
      <!-- 代理状态 -->
      <div class="stats shadow mb-4">
        <div class="stat">
          <div class="stat-figure text-primary">
            <i :class="['fas fa-circle', proxyStatus.running ? 'text-success' : 'text-error']" style="font-size: 1.875rem;"></i>
          </div>
          <div class="stat-title text-xs">代理状态</div>
          <div class="stat-value text-base" :class="proxyStatus.running ? 'text-success' : 'text-error'">
            {{ proxyStatus.running ? '运行中' : '已停止' }}
          </div>
          <div class="stat-desc">{{ proxyStatus.running ? `端口: ${proxyStatus.port}` : '未启动' }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-secondary">
            <i class="fas fa-lock" style="font-size: 1.875rem;"></i>
          </div>
          <div class="stat-title text-xs">MITM 状态</div>
          <div class="stat-value text-base">{{ proxyStatus.mitm ? '已启用' : '未启用' }}</div>
          <div class="stat-desc text-xs">中间人拦截</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-accent">
            <i class="fas fa-tachometer-alt" style="font-size: 1.875rem;"></i>
          </div>
          <div class="stat-title text-xs">QPS</div>
          <div class="stat-value text-base">{{ proxyStatus.stats.qps.toFixed(2) }}</div>
          <div class="stat-desc text-xs">每秒请求数</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-info">
            <i class="fas fa-exchange-alt" style="font-size: 1.875rem;"></i>
          </div>
          <div class="stat-title text-xs">请求统计</div>
          <div class="stat-value text-base">{{ totalRequests }}</div>
          <div class="stat-desc">
            HTTP: {{ proxyStatus.stats.http_requests }} | 
            HTTPS: {{ proxyStatus.stats.https_requests }}
          </div>
        </div>
      </div>
      
      <!-- 控制按钮 -->
      <div class="flex gap-3 mb-4">
        <button 
          @click="toggleProxy"
          :class="['btn', proxyStatus.running ? 'btn-error' : 'btn-success']"
          :disabled="isToggling"
        >
          <i v-if="isToggling" class="fas fa-spinner fa-spin mr-2"></i>
          <i v-else :class="['fas', proxyStatus.running ? 'fa-stop' : 'fa-play', 'mr-2']"></i>
          {{ isToggling ? '处理中...' : (proxyStatus.running ? '停止代理' : '启动代理') }}
        </button>
        
        <button 
          @click="refreshStatus"
          class="btn btn-outline btn-primary"
          :disabled="isRefreshing"
        >
          <i :class="['fas fa-sync-alt mr-2', { 'fa-spin': isRefreshing }]"></i>
          刷新状态
        </button>
      </div>
      
      <!-- 证书助手 -->
      <div class="divider">证书管理</div>
      
      <div class="alert alert-warning mb-4">
        <i class="fas fa-exclamation-triangle"></i>
        <div class="text-sm">
          <p class="font-semibold mb-2">⚠️ HTTPS 拦截需要信任 CA 证书</p>
          <p class="mb-1">如果访问 HTTPS 网站时出现 <code class="text-error">ERR_CERT_AUTHORITY_INVALID</code> 错误：</p>
          <ol class="list-decimal list-inside ml-2 space-y-1">
            <li>先启动代理服务</li>
            <li>点击"<strong>信任证书 (macOS)</strong>"按钮，输入管理员密码</li>
            <li>完全重启浏览器（关闭所有窗口）</li>
            <li>配置浏览器代理为 127.0.0.1:{{ proxyStatus.port || 8080 }}</li>
          </ol>
          <p class="mt-2 text-xs opacity-75">
            <i class="fas fa-book mr-1"></i>
            详细说明请查看 <a href="#" @click.prevent="openCertGuide" class="link link-primary">证书安装指南</a>
          </p>
        </div>
      </div>
      
      <div class="flex gap-3 flex-wrap">
        <button 
          @click="trustCACert"
          class="btn btn-success"
          :disabled="isTrustingCert || !proxyStatus.running"
        >
          <i :class="['fas fa-shield-alt mr-2', { 'fa-spin': isTrustingCert }]"></i>
          {{ isTrustingCert ? '正在安装...' : '一键信任证书 (macOS)' }}
        </button>
        
        <button 
          @click="downloadCACert"
          class="btn btn-outline btn-primary"
          :disabled="isDownloadingCert"
        >
          <i :class="['fas fa-download mr-2', { 'fa-spin': isDownloadingCert }]"></i>
          下载 CA 证书（手动安装）
        </button>
        
        <button 
          @click="regenerateCACert"
          class="btn btn-outline btn-warning"
          :disabled="isRegeneratingCert"
        >
          <i :class="['fas fa-sync-alt mr-2', { 'fa-spin': isRegeneratingCert }]"></i>
          重新生成证书
        </button>
      </div>
      
      <!-- 插件管理 -->
      <div class="divider">插件管理</div>
      
      <div class="alert alert-warning mb-4">
        <i class="fas fa-exclamation-triangle"></i>
        <div class="text-sm">
          <p class="font-semibold">插件权限提示</p>
          <p>插件运行在 Deno 环境中，具有完整系统权限。仅启用可信插件。</p>
        </div>
      </div>
      
      <div v-if="isLoadingPlugins" class="flex justify-center py-4">
        <i class="fas fa-spinner fa-spin text-primary" style="font-size: 1.875rem;"></i>
      </div>
      
      <div v-else-if="plugins.length === 0" class="alert alert-info">
        <i class="fas fa-info-circle"></i>
        <span>未找到可用插件。请将插件文件放置在插件目录中。</span>
      </div>
      
      <div v-else class="space-y-3">
        <div 
          v-for="plugin in plugins" 
          :key="plugin.metadata.id"
          class="card bg-base-200 shadow-sm"
        >
          <div class="card-body p-4">
            <div class="flex items-center justify-between">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="font-semibold text-base">{{ plugin.metadata.name }}</h3>
                  <span 
                    :class="[
                      'badge', 
                      'badge-sm',
                      plugin.status === 'Enabled' ? 'badge-success' : 
                      plugin.status === 'Disabled' ? 'badge-warning' : 
                      plugin.status === 'Error' ? 'badge-error' : 'badge-ghost'
                    ]"
                  >
                    {{ getStatusText(plugin.status) }}
                  </span>
                  <span class="badge badge-sm badge-outline">v{{ plugin.metadata.version }}</span>
                  <span 
                    v-if="plugin.metadata.severity"
                    :class="[
                      'badge', 
                      'badge-sm',
                      getSeverityClass(plugin.metadata.severity)
                    ]"
                  >
                    {{ plugin.metadata.severity }}
                  </span>
                </div>
                <p class="text-sm text-base-content/70 mb-2">{{ plugin.metadata.description || '暂无描述' }}</p>
                <div class="flex gap-2 text-xs text-base-content/60">
                  <span><i class="fas fa-tag mr-1"></i>{{ plugin.metadata.category }}</span>
                  <span><i class="fas fa-fingerprint mr-1"></i>{{ plugin.metadata.id }}</span>
                </div>
                <div v-if="plugin.last_error" class="mt-2">
                  <div class="alert alert-error py-2">
                    <i class="fas fa-exclamation-circle text-sm"></i>
                    <span class="text-xs">{{ plugin.last_error }}</span>
                  </div>
                </div>
              </div>
              
              <div class="flex flex-col gap-2 ml-4">
                <label class="swap swap-flip">
                  <input 
                    type="checkbox" 
                    :checked="plugin.status === 'Enabled'"
                    @change="togglePlugin(plugin.metadata.id, plugin.status)"
                    :disabled="isTogglingPlugin === plugin.metadata.id || plugin.status === 'Error'"
                  />
                  <div class="swap-on">
                    <button class="btn btn-success btn-sm">
                      <i v-if="isTogglingPlugin === plugin.metadata.id" class="fas fa-spinner fa-spin"></i>
                      <i v-else class="fas fa-check"></i>
                      已启用
                    </button>
                  </div>
                  <div class="swap-off">
                    <button class="btn btn-ghost btn-sm">
                      <i v-if="isTogglingPlugin === plugin.metadata.id" class="fas fa-spinner fa-spin"></i>
                      <i v-else class="fas fa-times"></i>
                      已禁用
                    </button>
                  </div>
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div class="flex gap-3 mt-4">
        <button 
          @click="refreshPlugins"
          class="btn btn-outline btn-primary"
          :disabled="isLoadingPlugins"
        >
          <i :class="['fas fa-sync-alt mr-2', { 'fa-spin': isLoadingPlugins }]"></i>
          刷新插件列表
        </button>
        
      </div>
      
      <!-- 配置选项 -->
      <div class="divider">高级配置</div>
      
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="form-control">
          <label class="label">
            <span class="label-text">起始端口</span>
          </label>
          <input 
            type="number" 
            v-model.number="proxyConfig.start_port" 
            class="input input-bordered input-sm"
            :disabled="proxyStatus.running"
            placeholder="8080"
            min="1024"
            max="65535"
          />
          <label class="label">
            <span class="label-text-alt text-xs">代理将从此端口开始尝试绑定</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">最大尝试次数</span>
          </label>
          <input 
            type="number" 
            v-model.number="proxyConfig.max_port_attempts" 
            class="input input-bordered input-sm"
            :disabled="proxyStatus.running"
            placeholder="10"
            min="1"
            max="100"
          />
          <label class="label">
            <span class="label-text-alt text-xs">端口被占用时的尝试次数</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text">启用 MITM (HTTPS 拦截)</span>
            <input 
              type="checkbox" 
              v-model="proxyConfig.mitm_enabled" 
              class="toggle toggle-primary"
              :disabled="proxyStatus.running"
            />
          </label>
          <label class="label">
            <span class="label-text-alt text-xs">启用后可拦截 HTTPS 流量</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">请求体大小限制 (MB)</span>
          </label>
          <input 
            type="number" 
            v-model.number="requestBodySizeMB" 
            class="input input-bordered input-sm"
            :disabled="proxyStatus.running"
            placeholder="2"
            min="1"
            max="100"
            @input="updateRequestBodySize"
          />
          <label class="label">
            <span class="label-text-alt text-xs">超过此大小的请求将被跳过</span>
          </label>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';

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

interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  description?: string;
  category: string;
  severity?: string;
}

interface PluginRecord {
  metadata: PluginMetadata;
  path: string;
  status: 'Loaded' | 'Enabled' | 'Disabled' | 'Error';
  last_error?: string;
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

const proxyConfig = ref({
  start_port: 8080,
  max_port_attempts: 10,
  mitm_enabled: true,
  max_request_body_size: 2 * 1024 * 1024,
  max_response_body_size: 2 * 1024 * 1024,
});

const plugins = ref<PluginRecord[]>([]);

const isToggling = ref(false);
const isRefreshing = ref(false);
const isDownloadingCert = ref(false);
const isTrustingCert = ref(false);
const isRegeneratingCert = ref(false);
const isLoadingPlugins = ref(false);
const isScanningPlugins = ref(false);
const isTogglingPlugin = ref<string | null>(null);

// 辅助变量：请求体大小（MB）
const requestBodySizeMB = ref(2);
const responseBodySizeMB = ref(2);

// 计算属性
const totalRequests = computed(() => {
  return proxyStatus.value.stats.http_requests + proxyStatus.value.stats.https_requests;
});

// 更新请求体大小（MB -> 字节）
function updateRequestBodySize() {
  proxyConfig.value.max_request_body_size = requestBodySizeMB.value * 1024 * 1024;
}

// 更新响应体大小（MB -> 字节）
function updateResponseBodySize() {
  proxyConfig.value.max_response_body_size = responseBodySizeMB.value * 1024 * 1024;
}

// 事件监听器
let unlistenProxyStatus: (() => void) | null = null;
let unlistenScanStats: (() => void) | null = null;
let unlistenPluginChanged: (() => void) | null = null;

// 方法
async function toggleProxy() {
  isToggling.value = true;
  try {
    if (proxyStatus.value.running) {
      await invoke('stop_passive_scan');
      dialog.toast.success('代理已停止');
    } else {
      const response = await invoke<any>('start_passive_scan', { 
        config: proxyConfig.value 
      });
      if (response.success && response.data) {
        dialog.toast.success(`代理已启动，监听端口: ${response.data}`);
      } else {
        throw new Error(response.error || '启动失败');
      }
    }
    await refreshStatus();
  } catch (error: any) {
    console.error('Failed to toggle proxy:', error);
    dialog.toast.error(`操作失败: ${error}`);
  } finally {
    isToggling.value = false;
  }
}

async function refreshStatus() {
  isRefreshing.value = true;
  try {
    const response = await invoke<any>('get_proxy_status');
    if (response.success && response.data) {
      proxyStatus.value = response.data;
    }
  } catch (error: any) {
    console.error('Failed to refresh proxy status:', error);
  } finally {
    isRefreshing.value = false;
  }
}

async function downloadCACert() {
  isDownloadingCert.value = true;
  try {
    const response = await invoke<any>('download_ca_cert');
    if (response.success && response.data) {
      dialog.toast.success(`证书已下载到: ${response.data.path}`);
    } else {
      dialog.toast.error(`下载证书失败: ${response.message || '未知错误'}`);
    }
  } catch (error: any) {
    console.error('Failed to download CA cert:', error);
    dialog.toast.error(`下载证书失败: ${error}`);
  } finally {
    isDownloadingCert.value = false;
  }
}

async function trustCACert() {
  isTrustingCert.value = true;
  try {
    await invoke('trust_ca_cert');
    dialog.toast.success('证书已添加到系统信任列表，请重启浏览器');
  } catch (error: any) {
    console.error('Failed to trust CA cert:', error);
    dialog.toast.error(`信任证书失败: ${error}。您可能需要以管理员权限运行，或使用手动安装方式`);
  } finally {
    isTrustingCert.value = false;
  }
}

async function regenerateCACert() {
  if (!confirm('重新生成证书将使旧证书失效，需要重新安装。确定继续吗？')) {
    return;
  }
  
  isRegeneratingCert.value = true;
  try {
    // 先停止代理
    if (proxyStatus.value.running) {
      await invoke('stop_passive_scan');
    }
    
    // 重新生成证书
    await invoke('regenerate_ca_cert');
    dialog.toast.success('证书已重新生成，请重新安装到系统');
    
    // 刷新状态
    await refreshStatus();
  } catch (error: any) {
    console.error('Failed to regenerate CA cert:', error);
    dialog.toast.error(`重新生成证书失败: ${error}`);
  } finally {
    isRegeneratingCert.value = false;
  }
}

function openCertGuide() {
  // 打开证书安装指南
  const guideUrl = 'https://github.com/o0x1024/sentinel-ai/blob/main/docs/CERTIFICATE_INSTALLATION_GUIDE.md';
  window.open(guideUrl, '_blank');
}

// 插件管理方法
async function refreshPlugins() {
  isLoadingPlugins.value = true;
  try {
    const response = await invoke<any>('list_plugins');
    if (response.success && response.data) {
      plugins.value = response.data;
    }
  } catch (error: any) {
    console.error('Failed to load plugins:', error);
    dialog.toast.error(`加载插件列表失败: ${error}`);
  } finally {
    isLoadingPlugins.value = false;
  }
}

async function togglePlugin(pluginId: string, currentStatus: string) {
  isTogglingPlugin.value = pluginId;
  try {
    if (currentStatus === 'Enabled') {
      await invoke('disable_plugin', { pluginId });
      dialog.toast.info(`插件 ${pluginId} 已禁用`);
    } else {
      await invoke('enable_plugin', { pluginId });
      dialog.toast.success(`插件 ${pluginId} 已启用`);
    }
    await refreshPlugins();
  } catch (error: any) {
    console.error('Failed to toggle plugin:', error);
    dialog.toast.error(`切换插件状态失败: ${error}`);
  } finally {
    isTogglingPlugin.value = null;
  }
}

function getStatusText(status: string): string {
  const statusMap: Record<string, string> = {
    Loaded: '已加载',
    Enabled: '已启用',
    Disabled: '已禁用',
    Error: '错误',
  };
  return statusMap[status] || status;
}

function getSeverityClass(severity: string | undefined): string {
  if (!severity) {
    return 'badge-ghost';
  }
  const severityMap: Record<string, string> = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-success',
    info: 'badge-ghost',
  };
  return severityMap[severity.toLowerCase()] || 'badge-ghost';
}

// 设置事件监听
async function setupEventListeners() {
  // 监听代理状态事件
  unlistenProxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    proxyStatus.value = event.payload;
  });
  
  // 监听扫描统计事件
  unlistenScanStats = await listen<any>('scan:stats', (event) => {
    if (proxyStatus.value.running) {
      proxyStatus.value.stats = {
        http_requests: event.payload.requests || 0,
        https_requests: 0, // 后端会合并到 requests
        errors: 0,
        qps: event.payload.qps || 0,
      };
    }
  });
  
  // 监听插件变更事件
  unlistenPluginChanged = await listen<any>('plugin:changed', async (event) => {
    console.log('Plugin changed:', event.payload);
    await refreshPlugins();
  });
}

// 生命周期
onMounted(async () => {
  await setupEventListeners();
  await refreshStatus();
  await refreshPlugins();
});

onUnmounted(() => {
  if (unlistenProxyStatus) unlistenProxyStatus();
  if (unlistenScanStats) unlistenScanStats();
  if (unlistenPluginChanged) unlistenPluginChanged();
});
</script>
