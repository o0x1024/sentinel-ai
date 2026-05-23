<template>
  <section class="workbench-main-stage flex h-full min-h-0 flex-col overflow-hidden rounded-[14px] border border-base-300/70">
    <div class="workbench-header-surface border-b border-base-300/70 px-2.5 py-1.5">
      <div class="flex flex-wrap items-center justify-between gap-1.5">
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 flex-wrap items-center gap-1.5">
            <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-primary/75">
              {{ t('trafficAnalysis.workbench.mainStage.title', '主工作区') }}
            </p>
            <span class="rounded-full bg-base-200 px-1.5 py-0.5 text-[10px] font-medium text-base-content/65">
              {{ activeWorkbenchMeta.shortTitle }}
            </span>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-1">
          <button
            v-for="chip in toolChips"
            :key="`main-${chip.tool}`"
            type="button"
            class="btn btn-xs btn-outline min-h-7 rounded-2xl px-2"
            :class="{
              'border-primary bg-primary/10 text-primary': workbenchOpen && activeWorkbenchTool === chip.tool,
            }"
            @click="$emit('openTool', chip.tool)"
          >
            <span>{{ chip.shortLabel }}</span>
            <span v-if="chip.showCount !== false && chip.count > 0" class="badge badge-xs badge-primary">{{ chip.count }}</span>
          </button>
        </div>
      </div>
    </div>

    <div class="workbench-content-surface relative min-h-0 flex-1 overflow-hidden">
      <div
        v-if="!workbenchOpen"
        class="absolute inset-0 flex flex-col items-center justify-center gap-2.5 px-6 text-center"
      >
        <div class="rounded-full bg-base-200 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/50">
          {{ t('trafficAnalysis.workbench.mainStage.idleBadge', '工作区空闲') }}
        </div>
        <h4 class="text-base font-semibold text-base-content">
          {{ t('trafficAnalysis.workbench.mainStage.idleTitle', '从历史记录选择请求进行预览') }}
        </h4>
        <p class="max-w-md text-xs text-base-content/55">
          {{ t('trafficAnalysis.workbench.mainStage.idleDescription', '编辑请求内容或发送请求后会自动保存到重放器历史；发送到爆破器会进入爆破器历史。') }}
        </p>
      </div>

      <template v-else>
        <ProxyRepeater
          v-if="retainedTools.includes('repeater')"
          ref="repeaterRef"
          :initial-request="pendingRepeaterRequest"
          :initial-draft-id="pendingRepeaterDraftId"
          :active-request-context="activeRequestContext"
          class="absolute inset-0 h-full overflow-auto"
          @openCompare="$emit('openCompareFromRepeater', $event)"
          @openDraftCompare="$emit('openDraftCompareFromRepeater', $event)"
          @createAttackWorkspace="$emit('createAttackWorkspaceFromRepeater', $event)"
          @active-tab-mode-changed="$emit('repeaterTabModeChanged', $event)"
          @tab-stats-changed="$emit('repeaterTabStatsChanged', $event)"
          @switch-preview-variant="$emit('switchRequestVariant', $event)"
        />
        <ProxyIntruder
          v-if="retainedTools.includes('intruder')"
          ref="intruderRef"
          :initial-request="pendingIntruderRequest"
          :initial-workspace-id="pendingIntruderWorkspaceId"
          class="absolute inset-0 h-full overflow-auto"
          @createDraft="$emit('createDraftFromIntruder', $event)"
          @openCompare="$emit('openCompareFromIntruder', $event)"
          @openDraftCompare="$emit('openDraftCompareFromIntruder', $event)"
          @workspaceStatsChanged="$emit('intruderWorkspaceStatsChanged', $event)"
        />
        <ProxyComparer
          v-if="retainedTools.includes('comparer')"
          ref="comparerRef"
          class="absolute inset-0 h-full overflow-auto"
          @createDraft="$emit('createDraftFromComparer', $event)"
        />
        <TrafficOastPanel
          v-if="retainedTools.includes('oast')"
          class="absolute inset-0 h-full overflow-auto"
          @openConfig="$emit('openProxySettings')"
          @openSourceRequest="$emit('openHistoryRequestFromOast', $event)"
        />
        <PacketCapture
          v-if="retainedTools.includes('capture')"
          class="absolute inset-0 h-full overflow-auto"
        />
      </template>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
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

const PacketCapture = defineAsyncComponent(() => import('../../PacketCapture.vue'))
const ProxyComparer = defineAsyncComponent(() => import('../../ProxyComparer.vue'))
const ProxyIntruder = defineAsyncComponent(() => import('../../ProxyIntruder.vue'))
const ProxyRepeater = defineAsyncComponent(() => import('../../ProxyRepeater.vue'))
const TrafficOastPanel = defineAsyncComponent(() => import('../../TrafficOastPanel.vue'))

const props = defineProps<{
  workbenchOpen: boolean
  activeWorkbenchTool: WorkbenchTool
  mountedTools: WorkbenchTool[]
  activeWorkbenchMeta: {
    title: string
    shortTitle: string
  }
  toolChips: Array<{
    tool: WorkbenchTool
    shortLabel: string
    count: number
    showCount?: boolean
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
  (e: 'intruderWorkspaceStatsChanged', stats: { openWorkspaceCount: number }): void
  (e: 'createDraftFromComparer', request: HttpExchangeRequest): void
  (e: 'openProxySettings'): void
  (e: 'openHistoryRequestFromOast', requestId: number): void
}>()

const repeaterRef = ref<InstanceType<typeof ProxyRepeater> | null>(null)
const intruderRef = ref<InstanceType<typeof ProxyIntruder> | null>(null)
const comparerRef = ref<InstanceType<typeof ProxyComparer> | null>(null)
const pendingComparisonPayloads = ref<TrafficComparePayload[]>([])
const pendingDraftRequests = ref<TrafficComparerDraftRequestInput[]>([])
const retainedTools = computed(() => {
  if (!props.workbenchOpen) {
    return []
  }
  return props.mountedTools.filter(tool => tool === props.activeWorkbenchTool)
})

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
  if (comparerRef.value) {
    comparerRef.value.addComparison(payload)
    return
  }

  pendingComparisonPayloads.value = [...pendingComparisonPayloads.value, payload]
}

function addDraftRequest(payload: TrafficComparerDraftRequestInput) {
  if (comparerRef.value) {
    comparerRef.value.addDraftRequest(payload)
    return
  }

  pendingDraftRequests.value = [...pendingDraftRequests.value, payload]
}

async function flushPendingComparerPayloads() {
  if (props.activeWorkbenchTool !== 'comparer') {
    return
  }

  await nextTick()
  const comparer = comparerRef.value
  if (!comparer) {
    return
  }

  const comparisonPayloads = pendingComparisonPayloads.value
  const draftRequests = pendingDraftRequests.value
  pendingComparisonPayloads.value = []
  pendingDraftRequests.value = []

  comparisonPayloads.forEach(payload => comparer.addComparison(payload))
  draftRequests.forEach(payload => comparer.addDraftRequest(payload))
}

watch(
  () => [props.activeWorkbenchTool, props.workbenchOpen] as const,
  () => {
    void flushPendingComparerPayloads()
  },
  { immediate: true },
)

defineExpose({
  hasRepeater,
  hasIntruder,
  hasComparer,
  openPreviewRequest,
  addComparison,
  addDraftRequest,
})
</script>
