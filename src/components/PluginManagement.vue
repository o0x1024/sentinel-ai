<template>
  <div class="card bg-base-100 shadow-xl">
    <div class="card-body">
      <h2 class="card-title text-base mb-3">
        <i class="fas fa-plug mr-2"></i>
        插件管理
      </h2>
      
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';

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

const plugins = ref<PluginRecord[]>([]);
const isLoadingPlugins = ref(false);
const isTogglingPlugin = ref<string | null>(null);

let unlistenPluginChanged: (() => void) | null = null;

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

onMounted(async () => {
  unlistenPluginChanged = await listen<any>('plugin:changed', async (event) => {
    console.log('Plugin changed:', event.payload);
    await refreshPlugins();
  });
  
  await refreshPlugins();
});

onUnmounted(() => {
  if (unlistenPluginChanged) unlistenPluginChanged();
});
</script>
