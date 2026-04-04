<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">行为来源</div>
        <div class="text-xs text-base-content/60 mt-1">
          当前 `Traffic Logic Triage` 使用的行为上下文来源。
        </div>
      </div>
      <span class="badge badge-sm" :class="effectiveModeBadgeClass">
        {{ effectiveModeLabel }}
      </span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
      <div class="rounded-lg bg-base-200/60 border border-base-300 p-3">
        <div class="text-xs text-base-content/60">已选择模式</div>
        <div class="mt-1 font-medium">{{ selectedModeLabel }}</div>
      </div>
      <div class="rounded-lg bg-base-200/60 border border-base-300 p-3">
        <div class="text-xs text-base-content/60">扩展连接状态</div>
        <div class="mt-1 flex items-center gap-2">
          <span class="badge badge-sm" :class="connected ? 'badge-success' : 'badge-ghost'">
            {{ connected ? '已连接' : '未连接' }}
          </span>
          <span v-if="connected && lastSeenAt" class="text-xs text-base-content/60">
            {{ formatDate(lastSeenAt) }}
          </span>
        </div>
      </div>
    </div>

    <div class="text-xs text-base-content/60 space-y-1">
      <div>
        本地 bridge：
        <code class="font-mono">{{ bridgeUrl }}</code>
      </div>
      <div v-if="selectedMode === 'browser_extension' && !connected">
        浏览器扩展未连接时，系统会自动回退到代理侧弱行为推断。
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TrafficBehaviorSignalSettings } from '@/components/traffic/proxyConfigurationTypes'
import { computed } from 'vue'

const props = defineProps<{
  settings: TrafficBehaviorSignalSettings
  bridgeUrl?: string
}>()

const selectedMode = computed(() => props.settings.mode)
const connected = computed(() => props.settings.browserExtensionConnected)
const lastSeenAt = computed(() => props.settings.browserExtensionLastSeenAt)
const bridgeUrl = computed(() => props.bridgeUrl || 'http://127.0.0.1:18931')

const selectedModeLabel = computed(() => {
  if (selectedMode.value === 'browser_extension') return '浏览器扩展行为采集'
  return '代理侧弱行为推断'
})

const effectiveModeLabel = computed(() => {
  if (selectedMode.value === 'browser_extension' && connected.value) {
    return '实际生效：浏览器扩展'
  }
  return '实际生效：代理推断'
})

const effectiveModeBadgeClass = computed(() => {
  if (selectedMode.value === 'browser_extension' && connected.value) {
    return 'badge-success'
  }
  return 'badge-info'
})

function formatDate(value?: string | null) {
  if (!value) return '-'
  try {
    return new Date(value).toLocaleString()
  } catch {
    return value
  }
}
</script>
