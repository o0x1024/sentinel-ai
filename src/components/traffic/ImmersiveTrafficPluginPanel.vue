<template>
  <div class="flex h-full min-h-0 flex-col overflow-hidden">
    <div class="rounded-[24px] border border-base-300/80 bg-base-100/92 p-4 shadow-sm">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
            Traffic Plugins
          </p>
          <h3 class="mt-1 text-xl font-semibold text-base-content">
            {{ t('trafficAnalysis.immersivePlugins.title', '流量分析插件') }}
          </h3>
          <p class="mt-1 text-sm text-base-content/65">
            {{ t('trafficAnalysis.immersivePlugins.description', '在沉浸式挖洞模式下直接启停流量分析插件，不再切出到插件管理。') }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="btn btn-sm"
            :class="trafficAnalysisPluginEnabled ? 'btn-success' : 'btn-outline'"
            :disabled="globalToggleSaving"
            @click="toggleGlobalScanning"
          >
            <i :class="globalToggleSaving ? 'fas fa-spinner fa-spin' : 'fas fa-wave-square'"></i>
            {{ trafficAnalysisPluginEnabled
              ? t('trafficAnalysis.immersivePlugins.globalEnabled', '扫描已开启')
              : t('trafficAnalysis.immersivePlugins.globalDisabled', '扫描已关闭') }}
          </button>
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            :disabled="loading"
            @click="refreshPanel"
          >
            <i :class="loading ? 'fas fa-spinner fa-spin' : 'fas fa-rotate-right'"></i>
            {{ t('trafficAnalysis.immersivePlugins.refresh', '刷新') }}
          </button>
        </div>
      </div>

      <div class="mt-4 grid gap-3 md:grid-cols-3">
        <div class="rounded-2xl border border-base-300/80 bg-base-200/55 px-4 py-3">
          <div class="text-xs uppercase tracking-[0.18em] text-base-content/45">
            {{ t('trafficAnalysis.immersivePlugins.enabledCount', '已启用') }}
          </div>
          <div class="mt-2 text-2xl font-semibold text-base-content">
            {{ enabledPluginCount }}
          </div>
        </div>
        <div class="rounded-2xl border border-base-300/80 bg-base-200/55 px-4 py-3">
          <div class="text-xs uppercase tracking-[0.18em] text-base-content/45">
            {{ t('trafficAnalysis.immersivePlugins.totalCount', '插件总数') }}
          </div>
          <div class="mt-2 text-2xl font-semibold text-base-content">
            {{ trafficPlugins.length }}
          </div>
        </div>
        <div class="rounded-2xl border border-base-300/80 bg-base-200/55 px-4 py-3">
          <div class="text-xs uppercase tracking-[0.18em] text-base-content/45">
            {{ t('trafficAnalysis.immersivePlugins.quickHintTitle', '当前模式') }}
          </div>
          <div class="mt-2 text-sm font-medium text-base-content/80">
            {{ trafficAnalysisPluginEnabled
              ? t('trafficAnalysis.immersivePlugins.quickHintEnabled', '新捕获流量会继续进入插件扫描链路')
              : t('trafficAnalysis.immersivePlugins.quickHintDisabled', '新捕获流量不会触发插件扫描，历史记录仍继续保留') }}
          </div>
        </div>
      </div>
    </div>

    <div v-if="errorMessage" class="mt-4 rounded-2xl border border-error/30 bg-error/10 px-4 py-3 text-sm text-error">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <span>{{ errorMessage }}</span>
        <button type="button" class="btn btn-xs btn-outline" @click="refreshPanel">
          {{ t('trafficAnalysis.immersivePlugins.retry', '重试') }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="flex min-h-0 flex-1 items-center justify-center">
      <div class="flex items-center gap-3 rounded-2xl border border-base-300/80 bg-base-100/92 px-5 py-4 text-sm text-base-content/70 shadow-sm">
        <i class="fas fa-spinner fa-spin text-primary"></i>
        <span>{{ t('trafficAnalysis.immersivePlugins.loading', '正在加载流量插件状态') }}</span>
      </div>
    </div>

    <div
      v-else-if="trafficPlugins.length === 0 && !errorMessage"
      class="flex min-h-0 flex-1 items-center justify-center"
    >
      <div class="max-w-md rounded-[28px] border border-dashed border-base-300 bg-base-100/86 px-8 py-10 text-center shadow-sm">
        <div class="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-base-200 text-base-content/60">
          <i class="fas fa-puzzle-piece text-xl"></i>
        </div>
        <h4 class="mt-4 text-lg font-semibold text-base-content">
          {{ t('trafficAnalysis.immersivePlugins.emptyTitle', '暂无流量分析插件') }}
        </h4>
        <p class="mt-2 text-sm leading-6 text-base-content/60">
          {{ t('trafficAnalysis.immersivePlugins.emptyDescription', '先安装或创建流量分析插件，再回到这里快速启停。') }}
        </p>
      </div>
    </div>

    <div v-else class="mt-4 min-h-0 flex-1 overflow-auto pr-1">
      <div class="overflow-hidden rounded-[26px] border border-base-300/80 bg-base-100/92 shadow-sm">
        <div class="hidden grid-cols-[minmax(0,2.1fr)_minmax(0,1.2fr)_8.5rem] gap-4 border-b border-base-300/80 bg-base-200/55 px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45 lg:grid">
          <span>{{ t('trafficAnalysis.immersivePlugins.listPlugin', '插件') }}</span>
          <span>{{ t('trafficAnalysis.immersivePlugins.listMeta', '分类与标签') }}</span>
          <span class="text-right">{{ t('trafficAnalysis.immersivePlugins.listAction', '操作') }}</span>
        </div>

        <article
          v-for="plugin in trafficPlugins"
          :key="plugin.metadata.id"
          class="grid gap-4 border-b border-base-300/70 px-5 py-4 last:border-b-0 lg:grid-cols-[minmax(0,2.1fr)_minmax(0,1.2fr)_8.5rem] lg:items-center"
          :class="plugin.status === 'Enabled' ? 'bg-success/5' : 'bg-base-100/92'"
        >
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h4 class="truncate text-base font-semibold text-base-content">
                {{ plugin.metadata.name }}
              </h4>
              <span
                class="badge badge-sm"
                :class="plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost'"
              >
                {{ plugin.status === 'Enabled'
                  ? t('trafficAnalysis.immersivePlugins.pluginEnabled', '已启用')
                  : t('trafficAnalysis.immersivePlugins.pluginDisabled', '已禁用') }}
              </span>
            </div>
            <p class="mt-1 break-all text-xs text-base-content/45">
              {{ plugin.metadata.id }}
            </p>
            <p class="mt-2 text-sm leading-6 text-base-content/68">
              {{ plugin.metadata.description?.trim()
                || t('trafficAnalysis.immersivePlugins.noDescription', '这个插件没有描述信息。') }}
            </p>
          </div>

          <div class="flex flex-wrap items-center gap-2 text-xs text-base-content/55">
            <span class="rounded-full bg-base-200 px-2.5 py-1">
              {{ plugin.metadata.category }}
            </span>
            <span v-if="plugin.metadata.default_severity" class="rounded-full bg-base-200 px-2.5 py-1">
              {{ plugin.metadata.default_severity }}
            </span>
            <span
              v-for="tag in plugin.metadata.tags.slice(0, 3)"
              :key="`${plugin.metadata.id}-${tag}`"
              class="rounded-full bg-base-200 px-2.5 py-1"
            >
              {{ tag }}
            </span>
          </div>

          <div class="flex justify-start lg:justify-end">
            <button
              type="button"
              class="btn btn-sm min-w-[7rem]"
              :class="plugin.status === 'Enabled' ? 'btn-outline btn-error' : 'btn-primary'"
              :disabled="isPluginBusy(plugin.metadata.id)"
              @click="togglePlugin(plugin)"
            >
              <i :class="isPluginBusy(plugin.metadata.id) ? 'fas fa-spinner fa-spin' : (plugin.status === 'Enabled' ? 'fas fa-stop' : 'fas fa-play')"></i>
              {{ plugin.status === 'Enabled'
                ? t('trafficAnalysis.immersivePlugins.disablePlugin', '停用')
                : t('trafficAnalysis.immersivePlugins.enablePlugin', '启用') }}
            </button>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type {
  CommandResponse,
  PluginRecord,
} from '@/components/PluginManagement/types'
import { useToast } from '@/composables/useToast'
import {
  isTrafficAnalysisPluginRecord,
  sortTrafficAnalysisPlugins,
} from './immersiveTrafficPluginPanelSupport'

const { t } = useI18n()
const toast = useToast()

const loading = ref(true)
const errorMessage = ref('')
const globalToggleSaving = ref(false)
const togglingPluginIds = ref<string[]>([])
const trafficAnalysisPluginEnabled = ref(true)
const plugins = ref<PluginRecord[]>([])

const trafficPlugins = computed(() =>
  sortTrafficAnalysisPlugins(plugins.value.filter(isTrafficAnalysisPluginRecord)),
)
const enabledPluginCount = computed(
  () => trafficPlugins.value.filter(plugin => plugin.status === 'Enabled').length,
)

function isPluginBusy(pluginId: string) {
  return togglingPluginIds.value.includes(pluginId)
}

async function loadPlugins() {
  const response = await invoke<CommandResponse<PluginRecord[]>>('list_plugins')
  if (!response.success || !response.data) {
    throw new Error(response.error || t('common.unknownError', '未知错误'))
  }

  plugins.value = response.data
}

async function loadGlobalScanningState() {
  const response = await invoke<CommandResponse<boolean>>('get_traffic_analysis_plugin_enabled')
  if (!response.success) {
    throw new Error(response.error || t('common.unknownError', '未知错误'))
  }

  trafficAnalysisPluginEnabled.value = response.data !== false
}

async function loadPanelData() {
  loading.value = true
  errorMessage.value = ''
  try {
    await Promise.all([
      loadPlugins(),
      loadGlobalScanningState(),
    ])
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

async function refreshPanel() {
  await loadPanelData()
}

async function toggleGlobalScanning() {
  const nextValue = !trafficAnalysisPluginEnabled.value
  globalToggleSaving.value = true
  try {
    const response = await invoke<CommandResponse<void>>(
      'set_traffic_analysis_plugin_enabled',
      { enabled: nextValue },
    )
    if (!response.success) {
      throw new Error(response.error || t('common.unknownError', '未知错误'))
    }

    trafficAnalysisPluginEnabled.value = nextValue
    toast.success(
      nextValue
        ? t('trafficAnalysis.immersivePlugins.globalEnableSuccess', '已开启流量插件扫描')
        : t('trafficAnalysis.immersivePlugins.globalDisableSuccess', '已关闭流量插件扫描'),
    )
  } catch (error) {
    toast.error(
      error instanceof Error
        ? error.message
        : t('trafficAnalysis.immersivePlugins.globalToggleError', '切换流量插件扫描状态失败'),
    )
  } finally {
    globalToggleSaving.value = false
  }
}

async function togglePlugin(plugin: PluginRecord) {
  const pluginId = plugin.metadata.id
  const nextEnabled = plugin.status !== 'Enabled'
  togglingPluginIds.value = [...togglingPluginIds.value, pluginId]

  try {
    const response = await invoke<CommandResponse<void>>(
      nextEnabled ? 'enable_plugin' : 'disable_plugin',
      { pluginId },
    )
    if (!response.success) {
      throw new Error(response.error || t('common.unknownError', '未知错误'))
    }

    plugins.value = plugins.value.map(item =>
      item.metadata.id === pluginId
        ? {
            ...item,
            status: nextEnabled ? 'Enabled' : 'Disabled',
          }
        : item,
    )
    toast.success(
      nextEnabled
        ? t('trafficAnalysis.immersivePlugins.enableSuccess', { name: plugin.metadata.name })
        : t('trafficAnalysis.immersivePlugins.disableSuccess', { name: plugin.metadata.name }),
    )
  } catch (error) {
    toast.error(
      error instanceof Error
        ? error.message
        : t('trafficAnalysis.immersivePlugins.pluginToggleError', '切换插件状态失败'),
    )
  } finally {
    togglingPluginIds.value = togglingPluginIds.value.filter(id => id !== pluginId)
  }
}

onMounted(() => {
  void loadPanelData()
})
</script>
