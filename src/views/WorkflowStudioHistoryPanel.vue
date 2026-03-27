<template>
  <div v-if="showExecutionHistory" ref="historyPanelRef" class="fixed inset-y-0 right-0 w-[700px] bg-base-100 shadow-xl border-l border-base-300 z-50 flex flex-col" @click.stop>
    <div class="p-3 flex items-center justify-between border-b border-base-300">
      <h2 class="text-base font-semibold">{{ t('trafficAnalysis.workflowStudio.executionHistory.title') }}</h2>
      <button class="btn btn-xs btn-ghost" @click="onClose">✕</button>
    </div>

    <div class="p-3 border-b border-base-300">
      <div class="flex gap-2">
        <input
          :value="historySearchQuery"
          class="input input-bordered input-sm flex-1"
          :placeholder="t('trafficAnalysis.workflowStudio.executionHistory.searchPlaceholder')"
          @input="$emit('update:historySearchQuery', ($event.target as HTMLInputElement).value)"
          @keyup.enter="onLoadHistory"
        />
        <button class="btn btn-sm btn-primary" @click="onLoadHistory">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-auto p-3">
      <table class="table table-sm table-zebra w-full">
        <thead class="sticky top-0 bg-base-100 z-10">
          <tr>
            <th class="w-48">{{ t('trafficAnalysis.workflowStudio.executionHistory.table.name') }}</th>
            <th class="w-40">{{ t('trafficAnalysis.workflowStudio.executionHistory.table.startTime') }}</th>
            <th class="w-24 text-right">{{ t('trafficAnalysis.workflowStudio.executionHistory.table.duration') }}</th>
            <th class="w-24 text-center">{{ t('trafficAnalysis.workflowStudio.executionHistory.table.status') }}</th>
            <th class="w-28 text-center">{{ t('trafficAnalysis.workflowStudio.executionHistory.table.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="historyLoading">
            <td colspan="5" class="text-center py-8"><span class="loading loading-spinner loading-md"></span></td>
          </tr>
          <tr v-else-if="historyData.length === 0">
            <td colspan="5" class="text-center py-8 text-base-content/50">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
              <p class="text-sm">{{ t('trafficAnalysis.workflowStudio.executionHistory.emptyTitle') }}</p>
            </td>
          </tr>
          <tr v-for="(execution, idx) in historyData" :key="execution.execution_id" class="hover">
            <td class="truncate max-w-[180px] cursor-pointer hover:text-primary font-medium" :title="execution.workflow_name" @click.stop="onViewExecutionDetail(execution.execution_id)">
              {{ execution.workflow_name }} #{{ historyTotal - (historyPage - 1) * historyPageSize - idx }}
            </td>
            <td class="text-xs text-base-content/70">{{ formatDatetime(execution.started_at) }}</td>
            <td class="text-right text-xs">{{ formatDuration(execution.duration_ms) }}</td>
            <td class="text-center"><span :class="getStatusBadgeClass(execution.status)" class="badge badge-sm">{{ getStatusText(execution.status) }}</span></td>
            <td class="text-center">
              <div class="flex justify-center gap-1">
                <button class="btn btn-xs btn-ghost" :title="t('trafficAnalysis.workflowStudio.executionHistory.table.viewDetail')" @click.stop="onViewExecutionDetail(execution.execution_id)">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" /></svg>
                </button>
                <button class="btn btn-xs btn-ghost text-error" :title="t('trafficAnalysis.workflowStudio.executionHistory.table.delete')" @click.stop="onDeleteHistoryRecord(execution.execution_id)">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="p-3 border-t border-base-300 flex items-center justify-between">
      <span class="text-sm text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.pagination.total', { total: historyTotal }) }}</span>
      <div class="join">
        <button class="join-item btn btn-sm" :disabled="historyPage <= 1" @click="$emit('update:historyPage', historyPage - 1); onLoadHistory()">«</button>
        <button class="join-item btn btn-sm">{{ historyPage }} / {{ Math.max(1, Math.ceil(historyTotal / historyPageSize)) }}</button>
        <button class="join-item btn btn-sm" :disabled="historyPage >= Math.ceil(historyTotal / historyPageSize)" @click="$emit('update:historyPage', historyPage + 1); onLoadHistory()">»</button>
      </div>
      <select class="select select-bordered select-sm w-24" :value="historyPageSize" @change="$emit('update:historyPageSize', Number(($event.target as HTMLSelectElement).value)); $emit('update:historyPage', 1); onLoadHistory()">
        <option :value="10">10</option>
        <option :value="20">20</option>
        <option :value="50">50</option>
      </select>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { HistoryItem } from './workflowStudioExecutionSupport'

const { t } = useI18n()
const historyPanelRef = ref<HTMLElement | null>(null)

defineProps<{
  showExecutionHistory: boolean
  historySearchQuery: string
  historyLoading: boolean
  historyData: HistoryItem[]
  historyTotal: number
  historyPage: number
  historyPageSize: number
  formatDatetime: (dateStr?: string) => string
  formatDuration: (ms?: number) => string
  getStatusBadgeClass: (status: string) => string
  getStatusText: (status: string) => string
  onLoadHistory: () => void
  onViewExecutionDetail: (runId: string) => void
  onDeleteHistoryRecord: (runId: string) => void
  onClose: () => void
}>()

defineEmits<{
  'update:historySearchQuery': [value: string]
  'update:historyPage': [value: number]
  'update:historyPageSize': [value: number]
}>()

defineExpose({
  historyPanelRef,
})
</script>
