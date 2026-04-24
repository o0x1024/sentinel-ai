<template>
  <section class="workbench-main-stage flex h-full min-h-0 flex-col overflow-hidden rounded-[26px] border border-base-300/70">
    <div class="workbench-header-surface border-b border-base-300/70 px-4 py-2.5">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 flex-wrap items-center gap-2">
            <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-primary/75">
              {{ t('trafficAnalysis.workbench.mainStage.title', '主工作区') }}
            </p>
            <span class="rounded-full bg-base-200 px-2 py-0.5 text-[11px] font-medium text-base-content/65">
              {{ activeWorkbenchMeta.shortTitle }}
            </span>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            v-for="chip in toolChips"
            :key="`main-${chip.tool}`"
            type="button"
            class="btn btn-xs btn-outline rounded-2xl"
            :class="{
              'border-primary bg-primary/10 text-primary': workbenchOpen && activeWorkbenchTool === chip.tool,
            }"
            @click="$emit('openTool', chip.tool)"
          >
            <span>{{ chip.shortLabel }}</span>
            <span v-if="chip.count > 0" class="badge badge-xs badge-primary">{{ chip.count }}</span>
          </button>
        </div>
      </div>

      <div
        v-if="activeRequestContext"
        class="mt-2 flex min-w-0 flex-wrap items-center gap-2 rounded-2xl border border-base-300/70 bg-base-100/75 px-3 py-2 text-xs"
      >
        <span class="rounded-full bg-primary/10 px-2 py-0.5 font-semibold text-primary">
          {{ activeRequestContext.modeLabel }}
        </span>
        <span class="rounded-full bg-base-200 px-2 py-0.5 text-base-content/60">
          {{ activeRequestContext.sourceLabel }}
        </span>
        <span class="badge badge-sm font-mono">{{ activeRequestContext.method }}</span>
        <span
          v-if="activeRequestContext.statusCode !== null"
          class="badge badge-sm"
          :class="statusBadgeClass(activeRequestContext.statusCode)"
        >
          {{ activeRequestContext.statusCode }}
        </span>
        <span class="min-w-0 flex-1 truncate font-mono text-base-content/75">
          {{ activeRequestContext.host }}{{ activeRequestContext.path }}
        </span>
        <span
          v-if="activeRequestContext.mode === 'preview'"
          class="text-[11px] text-base-content/45"
        >
          编辑或发送后自动保存为草稿
        </span>
        <div
          v-if="activeRequestContext.hasEditedVariant"
          class="join"
          aria-label="请求版本"
        >
          <button
            type="button"
            class="btn join-item btn-xs"
            :class="activeRequestContext.variant === 'original' ? 'btn-primary' : 'btn-ghost'"
            :disabled="activeRequestContext.variant === 'original'"
            @click="$emit('switchRequestVariant', 'original')"
          >
            原始
          </button>
          <button
            type="button"
            class="btn join-item btn-xs"
            :class="activeRequestContext.variant === 'edited' ? 'btn-warning' : 'btn-ghost'"
            :disabled="activeRequestContext.variant === 'edited'"
            @click="$emit('switchRequestVariant', 'edited')"
          >
            编辑后
          </button>
        </div>
      </div>
    </div>

    <div class="workbench-content-surface relative min-h-0 flex-1 overflow-hidden">
      <div
        v-if="!workbenchOpen"
        class="absolute inset-0 flex flex-col items-center justify-center gap-3 px-8 text-center"
      >
        <div class="rounded-full bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-[0.18em] text-base-content/50">
          {{ t('trafficAnalysis.workbench.mainStage.idleBadge', '工作区空闲') }}
        </div>
        <h4 class="text-lg font-semibold text-base-content">
          {{ t('trafficAnalysis.workbench.mainStage.idleTitle', '从历史记录选择请求进行预览') }}
        </h4>
        <p class="max-w-md text-sm text-base-content/55">
          {{ t('trafficAnalysis.workbench.mainStage.idleDescription', '编辑请求内容或发送请求后会自动保存为重放器草稿；发送到爆破器请在历史记录或预览中右键操作。') }}
        </p>
      </div>

      <template v-else-if="workbenchToolsMounted">
        <ProxyRepeater
          v-show="activeWorkbenchTool === 'repeater'"
          ref="repeaterRef"
          :initial-request="pendingRepeaterRequest"
          :initial-draft-id="pendingRepeaterDraftId"
          class="absolute inset-0 h-full overflow-auto"
          @openCompare="$emit('openCompareFromRepeater', $event)"
          @openDraftCompare="$emit('openDraftCompareFromRepeater', $event)"
          @createAttackWorkspace="$emit('createAttackWorkspaceFromRepeater', $event)"
          @active-tab-mode-changed="$emit('repeaterTabModeChanged', $event)"
          @tab-stats-changed="$emit('repeaterTabStatsChanged', $event)"
        />
        <ProxyIntruder
          v-show="activeWorkbenchTool === 'intruder'"
          ref="intruderRef"
          :initial-request="pendingIntruderRequest"
          :initial-workspace-id="pendingIntruderWorkspaceId"
          class="absolute inset-0 h-full overflow-auto"
          @createDraft="$emit('createDraftFromIntruder', $event)"
          @openCompare="$emit('openCompareFromIntruder', $event)"
          @openDraftCompare="$emit('openDraftCompareFromIntruder', $event)"
        />
        <ProxyComparer
          v-show="activeWorkbenchTool === 'comparer'"
          ref="comparerRef"
          class="absolute inset-0 h-full overflow-auto"
          @createDraft="$emit('createDraftFromComparer', $event)"
        />
        <TrafficOastPanel
          v-show="activeWorkbenchTool === 'oast'"
          class="absolute inset-0 h-full overflow-auto"
          @openConfig="$emit('openProxySettings')"
          @openSourceRequest="$emit('openHistoryRequestFromOast', $event)"
        />
        <PacketCapture
          v-show="activeWorkbenchTool === 'capture'"
          class="absolute inset-0 h-full overflow-auto"
        />
      </template>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import PacketCapture from '../../PacketCapture.vue'
import ProxyComparer from '../../ProxyComparer.vue'
import ProxyIntruder from '../../ProxyIntruder.vue'
import ProxyRepeater from '../../ProxyRepeater.vue'
import TrafficOastPanel from '../../TrafficOastPanel.vue'
import type { HttpExchangeRequest } from '../../http/model'
import type { RepeaterActiveTabState, RepeaterTabStats } from '../../proxyRepeaterTypes'
import type { TrafficComparerDraftRequestInput, TrafficComparePayload } from '../../transfers'
import type {
  TrafficWorkbenchRequestContext,
  TrafficWorkbenchRequestVariant,
  TrafficWorkbenchToolSession,
} from '../../trafficWorkbenchTypes'

type WorkbenchTool = TrafficWorkbenchToolSession['tool']

const { t } = useI18n()

defineProps<{
  workbenchOpen: boolean
  activeWorkbenchTool: WorkbenchTool
  workbenchToolsMounted: boolean
  activeWorkbenchMeta: {
    title: string
    shortTitle: string
  }
  toolChips: Array<{
    tool: WorkbenchTool
    shortLabel: string
    count: number
  }>
  pendingRepeaterRequest?: HttpExchangeRequest
  pendingRepeaterDraftId?: string
  pendingIntruderRequest?: HttpExchangeRequest
  pendingIntruderWorkspaceId?: string
  activeRequestContext?: TrafficWorkbenchRequestContext | null
}>()

defineEmits<{
  (e: 'openTool', tool: WorkbenchTool): void
  (e: 'openCompareFromRepeater', payload: TrafficComparePayload): void
  (e: 'openDraftCompareFromRepeater', payload: TrafficComparerDraftRequestInput): void
  (e: 'createAttackWorkspaceFromRepeater', request: HttpExchangeRequest): void
  (e: 'repeaterTabModeChanged', state: RepeaterActiveTabState): void
  (e: 'repeaterTabStatsChanged', stats: RepeaterTabStats): void
  (e: 'switchRequestVariant', variant: TrafficWorkbenchRequestVariant): void
  (e: 'createDraftFromIntruder', request: HttpExchangeRequest): void
  (e: 'openCompareFromIntruder', payload: TrafficComparePayload): void
  (e: 'openDraftCompareFromIntruder', payload: TrafficComparerDraftRequestInput): void
  (e: 'createDraftFromComparer', request: HttpExchangeRequest): void
  (e: 'openProxySettings'): void
  (e: 'openHistoryRequestFromOast', requestId: number): void
}>()

const repeaterRef = ref<InstanceType<typeof ProxyRepeater> | null>(null)
const intruderRef = ref<InstanceType<typeof ProxyIntruder> | null>(null)
const comparerRef = ref<InstanceType<typeof ProxyComparer> | null>(null)

function statusBadgeClass(statusCode: number) {
  if (statusCode >= 200 && statusCode < 300) return 'badge-success'
  if (statusCode >= 300 && statusCode < 400) return 'badge-info'
  if (statusCode >= 400 && statusCode < 500) return 'badge-warning'
  if (statusCode >= 500) return 'badge-error'
  return 'badge-ghost'
}

function hasRepeater() {
  return Boolean(repeaterRef.value)
}

function hasIntruder() {
  return Boolean(intruderRef.value)
}

function hasComparer() {
  return Boolean(comparerRef.value)
}

function openPreviewRequest(request: HttpExchangeRequest) {
  repeaterRef.value?.openPreviewRequest?.(request)
}

function addComparison(payload: TrafficComparePayload) {
  comparerRef.value?.addComparison(payload)
}

function addDraftRequest(payload: TrafficComparerDraftRequestInput) {
  comparerRef.value?.addDraftRequest(payload)
}

defineExpose({
  hasRepeater,
  hasIntruder,
  hasComparer,
  openPreviewRequest,
  addComparison,
  addDraftRequest,
})
</script>
