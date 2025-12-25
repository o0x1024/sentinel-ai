<template>
  <div class="card bg-base-100 shadow-xl">
    <div class="card-body">
      <h2 class="card-title text-base mb-3">
        <i class="fas fa-power-off mr-2"></i>
        {{ $t('trafficAnalysis.control.title') }}
      </h2>
      
      <!-- 代理状态 -->
      <div class="stats shadow mb-4">
        <div class="stat">
          <div class="stat-figure text-primary">
            <i :class="['fas fa-circle', proxyStatus.running ? 'text-success' : 'text-error', 'stat-icon']"></i>
          </div>
          <div class="stat-title text-xs">{{ $t('trafficAnalysis.control.stats.proxyStatus') }}</div>
          <div class="stat-value text-base" :class="proxyStatus.running ? 'text-success' : 'text-error'">
            {{ proxyStatus.running ? $t('trafficAnalysis.control.stats.running') : $t('trafficAnalysis.control.stats.stopped') }}
          </div>
          <div class="stat-desc">{{ proxyStatus.running ? `${$t('trafficAnalysis.control.stats.port')}: ${proxyStatus.port}` : $t('trafficAnalysis.control.stats.notStarted') }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-secondary">
            <i class="fas fa-lock stat-icon"></i>
          </div>
          <div class="stat-title text-xs">{{ $t('trafficAnalysis.control.stats.mitmStatus') }}</div>
          <div class="stat-value text-base">{{ proxyStatus.mitm ? $t('trafficAnalysis.control.stats.enabled') : $t('trafficAnalysis.control.stats.disabled') }}</div>
          <div class="stat-desc text-xs">{{ $t('trafficAnalysis.control.stats.mitmDesc') }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-accent">
            <i class="fas fa-tachometer-alt stat-icon"></i>
          </div>
          <div class="stat-title text-xs">{{ $t('trafficAnalysis.control.stats.qps') }}</div>
          <div class="stat-value text-base">{{ proxyStatus.stats.qps.toFixed(2) }}</div>
          <div class="stat-desc text-xs">{{ $t('trafficAnalysis.control.stats.qpsDesc') }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-figure text-info">
            <i class="fas fa-exchange-alt stat-icon"></i>
          </div>
          <div class="stat-title text-xs">{{ $t('trafficAnalysis.control.stats.requestStats') }}</div>
          <div class="stat-value text-base">{{ totalRequests }}</div>
          <div class="stat-desc">
            {{ $t('trafficAnalysis.control.stats.http') }}: {{ proxyStatus.stats.http_requests }} | 
            {{ $t('trafficAnalysis.control.stats.https') }}: {{ proxyStatus.stats.https_requests }}
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
          {{ isToggling ? $t('trafficAnalysis.control.processing') : (proxyStatus.running ? $t('trafficAnalysis.control.stopProxy') : $t('trafficAnalysis.control.startProxy')) }}
        </button>
        
        <button 
          @click="refreshStatus"
          class="btn btn-outline btn-primary"
          :disabled="isRefreshing"
        >
          <i :class="['fas fa-sync-alt mr-2', { 'fa-spin': isRefreshing }]"></i>
          {{ $t('trafficAnalysis.control.refreshStatus') }}
        </button>
      </div>
      
      <div class="alert alert-info">
        <i class="fas fa-info-circle"></i>
        <div class="text-sm">
          <p>{{ $t('trafficAnalysis.control.proxyConfig') }} <code class="font-mono bg-base-300 px-2 py-1 rounded">127.0.0.1:{{ proxyStatus.port || 8080 }}</code></p>
          <p class="mt-1">{{ $t('trafficAnalysis.control.proxySettings') }}</p>
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

const isToggling = ref(false);
const isRefreshing = ref(false);

const totalRequests = computed(() => {
  return proxyStatus.value.stats.http_requests + proxyStatus.value.stats.https_requests;
});

let unlistenProxyStatus: (() => void) | null = null;
let unlistenScanStats: (() => void) | null = null;

async function toggleProxy() {
  isToggling.value = true;
  try {
    if (proxyStatus.value.running) {
      await invoke('stop_passive_scan');
      dialog.toast.success('代理已停止');
    } else {
      // 从配置文件加载配置
      const configResponse = await invoke<any>('get_proxy_config');
      const config = configResponse.success && configResponse.data 
        ? configResponse.data 
        : {
            start_port: 8080,
            max_port_attempts: 10,
            mitm_enabled: true,
            max_request_body_size: 2 * 1024 * 1024,
            max_response_body_size: 2 * 1024 * 1024,
          };
      
      const response = await invoke<any>('start_passive_scan', { config });
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

async function setupEventListeners() {
  unlistenProxyStatus = await listen<ProxyStatus>('proxy:status', (event) => {
    proxyStatus.value = event.payload;
  });
  
  unlistenScanStats = await listen<any>('scan:stats', (event) => {
    if (proxyStatus.value.running) {
      proxyStatus.value.stats = {
        http_requests: event.payload.requests || 0,
        https_requests: 0,
        errors: 0,
        qps: event.payload.qps || 0,
      };
    }
  });
}

onMounted(async () => {
  await setupEventListeners();
  await refreshStatus();
});

onUnmounted(() => {
  if (unlistenProxyStatus) unlistenProxyStatus();
  if (unlistenScanStats) unlistenScanStats();
});

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyControl] Refresh triggered by parent');
  await refreshStatus();
});
</script>

<style scoped>
.stat-icon {
  font-size: calc(var(--font-size-base, 14px) * 1.875);
}
</style>
