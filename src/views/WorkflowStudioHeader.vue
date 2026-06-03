<template>
  <div class="flex items-center justify-between flex-shrink-0">
    <div class="flex items-center gap-3">
      <h1 class="text-2xl font-bold">{{ t('trafficAnalysis.workflowStudio.title') }}</h1>
      <input
        :value="workflowName"
        class="input input-bordered input-sm w-48"
        :placeholder="t('trafficAnalysis.workflowStudio.header.namePlaceholder')"
        @input="$emit('update:workflowName', ($event.target as HTMLInputElement).value)"
      />
      <button class="btn btn-xs btn-ghost" :title="t('trafficAnalysis.workflowStudio.header.editMetadataTooltip')" @click="onOpenMetaDialog">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
      </button>

      <div v-if="workflowName.trim()" class="text-xs text-base-content/60 ml-2">
        <span v-if="isAutoSaving" class="flex items-center gap-1">
          <span class="loading loading-spinner loading-xs"></span>
          {{ t('trafficAnalysis.workflowStudio.status.saving') }}
        </span>
        <span v-else-if="hasUnsavedChanges" class="text-warning">{{ t('trafficAnalysis.workflowStudio.status.unsaved') }}</span>
        <span v-else class="text-success">{{ t('trafficAnalysis.workflowStudio.status.saved') }}</span>
      </div>
    </div>

    <div class="flex gap-2">
      <button class="btn btn-sm" :class="showWorkflowListPanel ? 'btn-primary' : 'btn-outline'" :title="t('trafficAnalysis.workflowStudio.toolbar.workflowListTooltip')" @click="onToggleWorkflowListPanel">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
        </svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.workflowList') }}
      </button>
      <button class="btn btn-sm btn-primary" :disabled="!workflowName.trim()" :title="t('trafficAnalysis.workflowStudio.toolbar.saveTooltip')" @click="onSaveWorkflow">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
        </svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.save') }}
      </button>
      <div class="dropdown dropdown-end">
        <button tabindex="0" class="btn btn-sm btn-outline" :title="t('trafficAnalysis.workflowStudio.toolbar.exportImportTooltip')">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
          </svg>
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </button>
        <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52 z-50">
          <li><a @click="onExportWorkflowJson"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>{{ t('trafficAnalysis.workflowStudio.export.exportJson') }}</a></li>
          <li><a @click="onTriggerImportFile"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>{{ t('trafficAnalysis.workflowStudio.export.importJson') }}</a></li>
          <li><a @click="onExportWorkflowImage"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" /></svg>{{ t('trafficAnalysis.workflowStudio.export.exportImage') }}</a></li>
        </ul>
      </div>
      <button class="btn btn-sm btn-outline" :title="t('trafficAnalysis.workflowStudio.toolbar.refreshCatalogTooltip')" @click="onRefreshCatalog">{{ t('trafficAnalysis.workflowStudio.toolbar.refreshCatalog') }}</button>
      <button class="btn btn-sm btn-outline" :title="t('trafficAnalysis.workflowStudio.toolbar.resetCanvasTooltip')" @click="onResetCanvas">{{ t('trafficAnalysis.workflowStudio.toolbar.resetCanvas') }}</button>
      <button v-if="!workflowRunning" class="btn btn-sm btn-success" :title="t('trafficAnalysis.workflowStudio.toolbar.runTooltip')" @click="onStartRun">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.run') }}
      </button>
      <button v-else class="btn btn-sm btn-error" :title="t('trafficAnalysis.workflowStudio.toolbar.stopTooltip')" @click="onStopRun">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.stop') }}
      </button>
      <button v-if="!scheduleRunning" class="btn btn-sm btn-warning" :disabled="!workflowName.trim() || !hasScheduleTrigger" :title="hasScheduleTrigger ? t('trafficAnalysis.workflowStudio.toolbar.startScheduleTooltip') : t('trafficAnalysis.workflowStudio.toolbar.startScheduleDisabledTooltip')" @click="onStartSchedule">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.schedule') }}
      </button>
      <button v-else class="btn btn-sm btn-error" :title="t('trafficAnalysis.workflowStudio.toolbar.stopScheduleTooltip')" @click="onStopSchedule">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.stop') }}
      </button>
      <button class="btn btn-sm" :class="showLogs ? 'btn-primary' : 'btn-ghost'" :title="t('trafficAnalysis.workflowStudio.toolbar.toggleLogsTooltip')" @click="onToggleLogs">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.logs') }}
      </button>
      <button class="btn btn-sm" :class="showExecutionHistory ? 'btn-secondary' : 'btn-ghost'" :title="t('trafficAnalysis.workflowStudio.toolbar.executionHistoryTooltip')" @click="onToggleExecutionHistory">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
        {{ t('trafficAnalysis.workflowStudio.toolbar.history') }}
        <span v-if="executionHistoryCount" class="badge badge-xs badge-primary ml-1">{{ executionHistoryCount }}</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  workflowName: string
  isAutoSaving: boolean
  hasUnsavedChanges: boolean
  showWorkflowListPanel: boolean
  workflowRunning: boolean
  scheduleRunning: boolean
  hasScheduleTrigger: boolean
  showLogs: boolean
  showExecutionHistory: boolean
  executionHistoryCount: number
  onOpenMetaDialog: () => void
  onToggleWorkflowListPanel: () => void
  onSaveWorkflow: () => void
  onExportWorkflowJson: () => void
  onTriggerImportFile: () => void
  onExportWorkflowImage: () => void
  onRefreshCatalog: () => void
  onResetCanvas: () => void
  onStartRun: () => void
  onStopRun: () => void
  onStartSchedule: () => void
  onStopSchedule: () => void
  onToggleLogs: () => void
  onToggleExecutionHistory: () => void
}>()

defineEmits<{
  'update:workflowName': [value: string]
}>()
</script>
