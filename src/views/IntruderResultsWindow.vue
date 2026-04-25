<template>
  <div class="flex h-screen min-h-0 flex-col bg-base-100">
    <IntruderResultsContent
      v-if="windowState"
      :workspace-name="windowState.workspaceName"
      :request-text="windowState.requestText"
      :positions="windowState.positions"
      :results="windowState.results"
      :selected-result-id="windowState.selectedResultId"
      :selected-result="selectedResult"
      :progress="windowState.progress"
      :is-running="windowState.isRunning"
      :capture-filter="windowState.captureFilter"
      :view-filter="windowState.viewFilter"
      :sort="windowState.sort"
      :grep-match-rules="windowState.grepMatchRules"
      :grep-extract-rules="windowState.grepExtractRules"
      :grep-payload-settings="windowState.grepPayloadSettings"
      :visible-columns="windowState.visibleColumns"
      @select-result="updateSelectedResult"
      @update:capture-filter="updateCaptureFilter"
      @update:view-filter="updateViewFilter"
      @update:sort="updateSort"
      @update:visible-columns="updateVisibleColumns"
      @send-to-repeater="sendToRepeater($event)"
      @send-to-comparer="sendToComparer($event)"
    >
      <template #actions>
        <button
          v-if="enabledTargets.compare"
          class="btn btn-sm btn-ghost"
          type="button"
          :disabled="!selectedResult || selectedResult.isBaseline"
          @click="selectedResult?.id && sendToComparer(selectedResult.id)"
        >
          <i class="fas fa-not-equal"></i>
          {{ $t('trafficAnalysis.tabs.comparer') }}
        </button>
        <button
          v-if="enabledTargets.draft"
          class="btn btn-sm btn-ghost"
          type="button"
          :disabled="!selectedResult?.rawRequest"
          @click="selectedResult?.id && sendToRepeater(selectedResult.id)"
        >
          <i class="fas fa-redo"></i>
          {{ $t('trafficAnalysis.history.contextMenu.sendToRepeater') }}
        </button>
        <button class="btn btn-sm btn-ghost btn-circle" type="button" @click="closeWindow">
          <i class="fas fa-times"></i>
        </button>
      </template>
    </IntruderResultsContent>

    <div v-else class="flex flex-1 items-center justify-center text-sm text-base-content/60">
      {{ $t('trafficAnalysis.intruder.empty.noResults') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useI18n } from 'vue-i18n'
import IntruderResultsContent from '@/components/traffic/intruder/IntruderResultsContent.vue'
import { useTrafficSendTargets } from '@/components/traffic/trafficSendTargets'
import { normalizeGrepPayloadSettings } from '@/components/traffic/intruder/analysis'
import { buildSourceRequestFromRawRequest } from '@/components/traffic/intruder/http'
import {
  getIntruderResultsStorageKey,
  loadIntruderResultsWindowState,
  saveIntruderResultsWindowState,
} from '@/components/traffic/intruder/results'
import { normalizeVisibleColumns } from '@/components/traffic/intruder/storage'
import { queueComparerTransfer, queueRepeaterTransfer } from '@/components/traffic/transfers'
import type {
  IntruderAttackResult,
  IntruderResultFilter,
  IntruderResultSort,
  IntruderResultsWindowState,
} from '@/components/traffic/intruder/types'

const route = useRoute()
const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()
const workspaceId = computed(() => String(route.params.workspaceId ?? ''))
const windowState = ref<IntruderResultsWindowState | null>(null)

const selectedResult = computed<IntruderAttackResult | null>(() => {
  if (!windowState.value?.selectedResultId) return null
  return windowState.value.results.find((result) => result.id === windowState.value?.selectedResultId) ?? null
})

function hydrateWindowState() {
  if (!workspaceId.value) {
    windowState.value = null
    return
  }

  windowState.value = loadIntruderResultsWindowState(workspaceId.value)
  if (!windowState.value) return

  windowState.value = {
    ...windowState.value,
    grepPayloadSettings: normalizeGrepPayloadSettings(windowState.value.grepPayloadSettings),
    visibleColumns: normalizeVisibleColumns(
      windowState.value.visibleColumns,
      windowState.value.grepMatchRules || [],
      windowState.value.grepExtractRules || [],
      normalizeGrepPayloadSettings(windowState.value.grepPayloadSettings),
    ),
  }
}

function persistState() {
  if (!windowState.value) return
  saveIntruderResultsWindowState(windowState.value)
}

function updateSelectedResult(resultId: string) {
  if (!windowState.value) return
  windowState.value = {
    ...windowState.value,
    selectedResultId: resultId,
  }
  persistState()
}

function updateCaptureFilter(value: IntruderResultFilter) {
  if (!windowState.value) return
  windowState.value = {
    ...windowState.value,
    captureFilter: value,
  }
  persistState()
}

function updateViewFilter(value: IntruderResultFilter) {
  if (!windowState.value) return
  windowState.value = {
    ...windowState.value,
    viewFilter: value,
  }
  persistState()
}

function updateSort(value: IntruderResultSort) {
  if (!windowState.value) return
  windowState.value = {
    ...windowState.value,
    sort: value,
  }
  persistState()
}

function updateVisibleColumns(value: string[]) {
  if (!windowState.value) return
  windowState.value = {
    ...windowState.value,
    visibleColumns: normalizeVisibleColumns(
      value,
      windowState.value.grepMatchRules || [],
      windowState.value.grepExtractRules || [],
      normalizeGrepPayloadSettings(windowState.value.grepPayloadSettings),
    ),
  }
  persistState()
}

function handleStorage(event: StorageEvent) {
  if (!event.key || event.key !== getIntruderResultsStorageKey(workspaceId.value)) return
  hydrateWindowState()
}

function sendToRepeater(resultId?: string) {
  if (!windowState.value) return

  const result = resultId
    ? windowState.value.results.find((item) => item.id === resultId) ?? null
    : selectedResult.value
  if (!result?.rawRequest) return

  const request = buildSourceRequestFromRawRequest(result.rawRequest, windowState.value.target)
  if (!request) return
  queueRepeaterTransfer(request)
}

function sendToComparer(resultId?: string) {
  if (!windowState.value) return

  const result = resultId
    ? windowState.value.results.find((item) => item.id === resultId) ?? null
    : selectedResult.value
  if (!result || result.isBaseline) return

  const baseline = windowState.value.results.find((result) => result.isBaseline)
  if (!baseline) return

  const protocol: 'http' | 'https' = windowState.value.target.useTls ? 'https' : 'http'
  queueComparerTransfer({
    name: `${windowState.value.workspaceName} #${result.index}`,
    leftLabel: t('trafficAnalysis.intruder.labels.baseline'),
    rightLabel: `#${result.index}`,
    leftText: baseline.rawResponse || baseline.error || '',
    rightText: result.rawResponse || result.error || '',
    compareMeta: {
      source: 'intruder',
      kind: 'responseDiff',
    },
    leftMeta: {
      messageType: 'response',
      protocol,
    },
    rightMeta: {
      messageType: 'response',
      protocol,
    },
  })
}

async function closeWindow() {
  await getCurrentWebviewWindow().close()
}

onMounted(() => {
  hydrateWindowState()
  window.addEventListener('storage', handleStorage)
})

onUnmounted(() => {
  window.removeEventListener('storage', handleStorage)
})
</script>
