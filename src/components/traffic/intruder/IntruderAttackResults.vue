<template>
  <AppDialog ref="dialogRef" class="modal intruder-results-modal" @close="$emit('close')">
    <div class="modal-box flex h-[85vh] max-w-[92vw] flex-col overflow-hidden p-0">
      <IntruderResultsContent
        :workspace-name="workspaceName"
        :request-text="requestText"
        :positions="positions"
        :results="results"
        :selected-result-id="selectedResultId"
        :selected-result="selectedResult"
        :progress="progress"
        :is-running="isRunning"
        :capture-filter="captureFilter"
        :view-filter="viewFilter"
        :sort="sort"
        :grep-match-rules="grepMatchRules"
        :grep-extract-rules="grepExtractRules"
        :grep-payload-settings="grepPayloadSettings"
        :visible-columns="visibleColumns"
        @select-result="$emit('selectResult', $event)"
        @update:capture-filter="$emit('update:captureFilter', $event)"
        @update:view-filter="$emit('update:viewFilter', $event)"
        @update:sort="$emit('update:sort', $event)"
        @update:visible-columns="$emit('update:visibleColumns', $event)"
        @send-to-repeater="$emit('sendToRepeater', $event)"
        @send-to-comparer="$emit('sendToComparer', $event)"
      >
        <template #actions>
          <button
            v-if="enabledTargets.compare"
            class="btn btn-sm btn-ghost"
            type="button"
            :disabled="!selectedResult || selectedResult.isBaseline"
            @click="selectedResult?.id && $emit('sendToComparer', selectedResult.id)"
          >
            <i class="fas fa-not-equal"></i>
            {{ $t('trafficAnalysis.tabs.comparer') }}
          </button>
          <button
            v-if="enabledTargets.draft"
            class="btn btn-sm btn-ghost"
            type="button"
            :disabled="!selectedResult?.rawRequest"
            @click="selectedResult?.id && $emit('sendToRepeater', selectedResult.id)"
          >
            <i class="fas fa-redo"></i>
            {{ $t('trafficAnalysis.history.contextMenu.sendToRepeater') }}
          </button>
          <button class="btn btn-sm btn-ghost btn-circle" type="button" @click="closeDialog">
            <i class="fas fa-times"></i>
          </button>
        </template>
      </IntruderResultsContent>
    </div>

    <form method="dialog" class="modal-backdrop">
      <button>{{ $t('common.close') }}</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useTrafficSendTargets } from '@/components/traffic/trafficSendTargets'
import IntruderResultsContent from './IntruderResultsContent.vue'
import type {
  IntruderAttackProgress,
  IntruderAttackResult,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPosition,
  IntruderResultFilter,
  IntruderResultSort,
} from './types'

const props = defineProps<{
  open: boolean
  workspaceName: string
  requestText: string
  positions: IntruderPosition[]
  results: IntruderAttackResult[]
  selectedResultId: string | null
  selectedResult: IntruderAttackResult | null
  progress: IntruderAttackProgress
  isRunning: boolean
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  grepPayloadSettings: IntruderGrepPayloadSettings
  visibleColumns: string[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'selectResult', id: string): void
  (e: 'update:captureFilter', value: IntruderResultFilter): void
  (e: 'update:viewFilter', value: IntruderResultFilter): void
  (e: 'update:sort', value: IntruderResultSort): void
  (e: 'update:visibleColumns', value: string[]): void
  (e: 'sendToRepeater', resultId: string): void
  (e: 'sendToComparer', resultId: string): void
}>()

const { enabledTargets } = useTrafficSendTargets()
const dialogRef = ref<HTMLDialogElement | null>(null)

watch(
  () => props.open,
  (open) => {
    if (!dialogRef.value) return

    if (open && !dialogRef.value.open) {
      dialogRef.value.showModal()
      return
    }

    if (!open && dialogRef.value.open) {
      dialogRef.value.close()
    }
  },
)

function closeDialog() {
  dialogRef.value?.close()
  emit('close')
}
</script>

<style scoped>
.intruder-results-modal::backdrop {
  background: rgba(15, 23, 42, 0.45);
  backdrop-filter: blur(4px);
}
</style>
