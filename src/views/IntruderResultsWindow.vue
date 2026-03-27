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
      :visible-columns="windowState.visibleColumns"
      @select-result="updateSelectedResult"
      @update:capture-filter="updateCaptureFilter"
      @update:view-filter="updateViewFilter"
      @update:sort="updateSort"
      @update:visible-columns="updateVisibleColumns"
    >
      <template #actions>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!selectedResult || selectedResult.isBaseline" @click="sendToComparer">
          <i class="fas fa-not-equal"></i>
          {{ $t('trafficAnalysis.tabs.comparer') }}
        </button>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!selectedResult?.rawRequest" @click="sendToRepeater">
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
import IntruderResultsContent from '@/components/traffic/intruder/IntruderResultsContent.vue'
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
    visibleColumns: normalizeVisibleColumns(
      windowState.value.visibleColumns,
      windowState.value.grepMatchRules || [],
      windowState.value.grepExtractRules || [],
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
    ),
  }
  persistState()
}

function handleStorage(event: StorageEvent) {
  if (!event.key || event.key !== getIntruderResultsStorageKey(workspaceId.value)) return
  hydrateWindowState()
}

function sendToRepeater() {
  if (!windowState.value || !selectedResult.value?.rawRequest) return

  const request = buildSourceRequestFromRawRequest(selectedResult.value.rawRequest, windowState.value.target)
  if (!request) return
  queueRepeaterTransfer(request)
}

function sendToComparer() {
  if (!windowState.value || !selectedResult.value || selectedResult.value.isBaseline) return

  const baseline = windowState.value.results.find((result) => result.isBaseline)
  if (!baseline) return

  queueComparerTransfer({
    name: `${windowState.value.workspaceName} #${selectedResult.value.index}`,
    leftLabel: 'Baseline',
    rightLabel: `#${selectedResult.value.index}`,
    leftText: baseline.rawResponse || baseline.error || '',
    rightText: selectedResult.value.rawResponse || selectedResult.value.error || '',
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
