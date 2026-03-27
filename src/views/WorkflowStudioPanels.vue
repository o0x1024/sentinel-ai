<template>
  <div v-if="showLogs" class="card bg-base-100 shadow-xl mt-4 flex-shrink-0">
    <div class="card-body p-3">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-base font-semibold">{{ t('trafficAnalysis.workflowStudio.logs.title') }}</h2>
        <div class="flex gap-2">
          <button class="btn btn-xs btn-outline" @click="onClearLogs">{{ t('trafficAnalysis.workflowStudio.logs.clear') }}</button>
          <button class="btn btn-xs btn-ghost" @click="$emit('update:showLogs', false)">✕</button>
        </div>
      </div>
      <div ref="logsContainerRef" class="overflow-y-auto bg-base-200 rounded p-2 font-mono text-xs" style="max-height: 120px">
        <div v-if="executionLogs.length === 0" class="text-center text-base-content/60 py-4">{{ t('trafficAnalysis.workflowStudio.logs.empty') }}</div>
        <div v-for="(log, idx) in executionLogs" :key="idx" class="mb-1">
          <div :class="getLogClass(log.level)">
            <span class="opacity-60">[{{ formatTime(log.timestamp) }}]</span>
            <span class="font-semibold">[{{ log.level }}]</span>
            <span v-if="log.node_id" class="text-primary">[{{ log.node_id }}]</span>
            <span>{{ log.message }}</span>
            <button v-if="log.details" class="btn btn-xs btn-ghost ml-2" :title="expandedLogs.has(idx) ? t('trafficAnalysis.workflowStudio.logs.collapseDetails') : t('trafficAnalysis.workflowStudio.logs.expandDetails')" @click="onToggleLogDetails(idx)">
              {{ expandedLogs.has(idx) ? '▼' : '▶' }}
            </button>
          </div>
          <pre v-if="log.details && expandedLogs.has(idx)" class="ml-4 mt-1 text-xs opacity-80 bg-base-300 p-2 rounded overflow-x-auto max-h-60">{{ log.details }}</pre>
        </div>
      </div>
    </div>
  </div>

  <dialog :open="showNewWorkflowConfirm" class="modal">
    <div class="modal-box">
      <h3 class="font-bold text-lg mb-4">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.title') }}</h3>
      <p class="text-base-content/80">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.message') }}</p>
      <div class="modal-action">
        <button class="btn btn-primary btn-sm" @click="onConfirmNewWorkflowSave">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.saveAndNew') }}</button>
        <button class="btn btn-warning btn-sm" @click="onConfirmNewWorkflowDiscard">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.discardAndNew') }}</button>
        <button class="btn btn-ghost btn-sm" @click="$emit('update:showNewWorkflowConfirm', false)">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.cancel') }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="$emit('update:showNewWorkflowConfirm', false)">{{ t('trafficAnalysis.workflowStudio.newWorkflowConfirm.close') }}</button>
    </form>
  </dialog>

  <dialog :open="showMetaDialog" class="modal" @click.self="$emit('update:showMetaDialog', false)">
    <div class="modal-box">
      <h3 class="font-bold text-lg mb-4">{{ t('trafficAnalysis.workflowStudio.metaDialog.title') }}</h3>
      <div class="space-y-3">
        <div class="form-control">
          <label class="label"><span class="label-text">{{ t('trafficAnalysis.workflowStudio.metaDialog.name') }} <span class="text-error">*</span></span></label>
          <input :value="workflowName" class="input input-bordered" :placeholder="t('trafficAnalysis.workflowStudio.metaDialog.namePlaceholder')" @input="$emit('update:workflowName', ($event.target as HTMLInputElement).value)" />
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text">{{ t('trafficAnalysis.workflowStudio.metaDialog.description') }}</span></label>
          <textarea :value="workflowDescription" class="textarea textarea-bordered" rows="3" :placeholder="t('trafficAnalysis.workflowStudio.metaDialog.descriptionPlaceholder')" @input="$emit('update:workflowDescription', ($event.target as HTMLTextAreaElement).value)"></textarea>
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text">{{ t('trafficAnalysis.workflowStudio.metaDialog.tags') }}</span></label>
          <input :value="workflowTags" class="input input-bordered" :placeholder="t('trafficAnalysis.workflowStudio.metaDialog.tagsPlaceholder')" @input="$emit('update:workflowTags', ($event.target as HTMLInputElement).value)" />
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text">{{ t('trafficAnalysis.workflowStudio.metaDialog.version') }}</span></label>
          <input :value="workflowVersion" class="input input-bordered" placeholder="v1.0.0" @input="$emit('update:workflowVersion', ($event.target as HTMLInputElement).value)" />
        </div>
        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text">{{ t('trafficAnalysis.workflowStudio.metaDialog.asAiTool') }}</span>
            <input :checked="workflowIsTool" type="checkbox" class="toggle toggle-primary" @change="$emit('update:workflowIsTool', ($event.target as HTMLInputElement).checked)" />
          </label>
          <label class="label py-0"><span class="label-text-alt text-base-content/60">{{ t('trafficAnalysis.workflowStudio.metaDialog.asAiToolHelp') }}</span></label>
        </div>
        <div class="stats shadow w-full">
          <div class="stat py-2">
            <div class="stat-title text-xs">{{ t('trafficAnalysis.workflowStudio.metaDialog.stats.nodes') }}</div>
            <div class="stat-value text-2xl">{{ nodeCount }}</div>
          </div>
          <div class="stat py-2">
            <div class="stat-title text-xs">{{ t('trafficAnalysis.workflowStudio.metaDialog.stats.edges') }}</div>
            <div class="stat-value text-2xl">{{ edgeCount }}</div>
          </div>
        </div>
      </div>
      <div class="modal-action">
        <button class="btn btn-sm btn-primary" :disabled="!workflowName.trim()" @click="$emit('update:showMetaDialog', false)">{{ t('trafficAnalysis.workflowStudio.metaDialog.confirm') }}</button>
        <button class="btn btn-sm" @click="$emit('update:showMetaDialog', false)">{{ t('trafficAnalysis.workflowStudio.metaDialog.cancel') }}</button>
      </div>
    </div>
  </dialog>

  <Transition name="fade">
    <div v-if="drawerOpen" class="fixed inset-0 bg-black/20 z-40" @click="onCloseDrawer"></div>
  </Transition>

  <Transition name="drawer-right">
    <div v-if="drawerOpen" class="fixed inset-y-0 right-0 w-[350px] bg-base-100 shadow-xl border-l border-base-300 z-50">
      <div class="p-3 flex items-center justify-between border-b border-base-300">
        <h2 class="text-base font-semibold">{{ t('trafficAnalysis.workflowStudio.paramsEditor.title') }}</h2>
        <button class="btn btn-xs btn-ghost" @click="onCloseDrawer">✕</button>
      </div>
      <div class="p-3 border-b border-base-300">
        <div class="text-sm font-semibold">{{ selectedNode?.name }}</div>
        <div class="text-xs text-base-content/60 mt-1">{{ selectedNode?.type }}</div>
      </div>
      <div v-if="selectedSchema" class="p-3 space-y-3 overflow-auto h-[calc(100%-140px)]">
        <div v-if="!selectedSchema.properties || Object.keys(selectedSchema.properties).length === 0" class="text-center text-sm text-base-content/60 py-4">{{ t('trafficAnalysis.workflowStudio.paramsEditor.noParams') }}</div>
        <div v-for="(prop, key) in selectedSchema.properties" :key="key" class="form-control">
          <label class="label py-1">
            <span class="label-text text-xs font-semibold">{{ key }}<span v-if="selectedSchema.required?.includes(key)" class="text-error">*</span></span>
            <span v-if="prop.description" class="label-text-alt text-xs opacity-60" :title="prop.description">?</span>
          </label>

          <div v-if="String(key) === 'notification_rule_id' && selectedNode?.type === 'notify'" class="space-y-2">
            <select v-model="paramValues[key]" class="select select-bordered select-sm w-full" :class="{ 'select-error': selectedSchema.required?.includes(key) && !paramValues[key] }">
              <option value="">{{ t('trafficAnalysis.workflowStudio.paramsEditor.selectNotificationRule') }}</option>
              <option v-for="rule in notificationRules" :key="rule.id" :value="rule.id">{{ rule.type_name }} ({{ rule.channel }})</option>
            </select>
            <div v-if="notificationRules.length === 0" class="text-xs text-warning">
              <span>{{ t('trafficAnalysis.workflowStudio.paramsEditor.noNotificationRules') }}</span>
              <router-link to="/notification-management" class="link link-primary">{{ t('trafficAnalysis.workflowStudio.paramsEditor.goToConfigure') }}</router-link>
            </div>
          </div>

          <div v-else-if="prop['x-ui-widget'] === 'ai-provider-select'" class="space-y-2">
            <select v-model="paramValues[key]" class="select select-bordered select-sm w-full">
              <option value="">{{ t('trafficAnalysis.workflowStudio.paramsEditor.useDefaultConfig') }}</option>
              <option v-for="provider in getEnabledProviders()" :key="provider" :value="provider">{{ provider }}</option>
            </select>
            <div v-if="getEnabledProviders().length === 0" class="text-xs text-warning">
              <span>{{ t('trafficAnalysis.workflowStudio.paramsEditor.noAiProviders') }}</span>
              <router-link to="/settings" class="link link-primary">{{ t('trafficAnalysis.workflowStudio.paramsEditor.goToConfigure') }}</router-link>
            </div>
          </div>

          <div v-else-if="prop['x-ui-widget'] === 'ai-model-select'" class="space-y-2">
            <select v-model="paramValues[key]" class="select select-bordered select-sm w-full" :disabled="!paramValues['provider']">
              <option value="">{{ paramValues['provider'] ? t('trafficAnalysis.workflowStudio.paramsEditor.selectModel') : t('trafficAnalysis.workflowStudio.paramsEditor.selectProviderFirst') }}</option>
              <option v-for="model in getProviderModels(paramValues['provider'])" :key="model.id" :value="model.id">{{ model.name }}{{ model.description ? ' - ' + model.description : '' }}</option>
            </select>
          </div>

          <div v-else-if="prop['x-ui-widget'] === 'tools-multiselect'" class="space-y-2">
            <div class="max-h-48 overflow-y-auto border border-base-300 rounded-lg p-2 space-y-1">
              <div v-if="availableTools.length === 0" class="text-xs text-base-content/60 text-center py-2">{{ t('trafficAnalysis.workflowStudio.paramsEditor.noTools') }}</div>
              <label v-for="tool in availableTools" :key="tool.name" class="flex items-center gap-2 p-1 hover:bg-base-200 rounded cursor-pointer">
                <input type="checkbox" class="checkbox checkbox-sm checkbox-primary" :value="tool.name" :checked="(paramValues[key] || []).includes(tool.name)" @change="onToggleToolSelection(String(key), tool.name)" />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium truncate">{{ tool.name }}</div>
                  <div v-if="tool.description" class="text-xs text-base-content/60 truncate">{{ tool.description }}</div>
                </div>
              </label>
            </div>
            <div class="text-xs text-base-content/60">{{ t('trafficAnalysis.workflowStudio.paramsEditor.selectedToolsCount', { count: (paramValues[key] || []).length }) }}</div>
          </div>

          <textarea v-else-if="prop['x-ui-widget'] === 'textarea'" v-model="paramValues[key]" class="textarea textarea-bordered textarea-sm w-full" :placeholder="prop.default || t('trafficAnalysis.workflowStudio.paramsEditor.enterField', { key: String(key) })" :class="{ 'textarea-error': selectedSchema.required?.includes(key) && !paramValues[key] }" rows="3"></textarea>

          <div v-else-if="prop['x-ui-widget'] === 'textarea-lines'" class="space-y-1">
            <textarea v-model="paramValues[key]" class="textarea textarea-bordered textarea-sm font-mono text-xs w-full" :placeholder="prop.description || t('trafficAnalysis.workflowStudio.paramsEditor.onePerLine')" :class="{ 'textarea-error': selectedSchema.required?.includes(key) && !paramValues[key] }" rows="4"></textarea>
            <div class="text-xs text-base-content/50">{{ t('trafficAnalysis.workflowStudio.paramsEditor.onePerLine') }}</div>
          </div>

          <input v-else-if="prop.type === 'string' && !prop.enum" v-model="paramValues[key]" class="input input-bordered input-sm w-full" :placeholder="prop.default || t('trafficAnalysis.workflowStudio.paramsEditor.enterField', { key: String(key) })" :class="{ 'input-error': selectedSchema.required?.includes(key) && !paramValues[key] }" />

          <input v-else-if="prop.type === 'integer' || prop.type === 'float' || prop.type === 'number'" v-model.number="paramValues[key]" type="number" class="input input-bordered input-sm w-full" :placeholder="prop.default?.toString() || '0'" :min="prop.minimum" :max="prop.maximum" :step="prop.type === 'integer' ? 1 : 0.1" />

          <select v-else-if="prop.enum && prop.enum.length" v-model="paramValues[key]" class="select select-bordered select-sm">
            <option value="">{{ t('trafficAnalysis.workflowStudio.paramsEditor.pleaseSelect') }}</option>
            <option v-for="opt in prop.enum" :key="opt" :value="opt">{{ opt }}</option>
          </select>

          <div v-else-if="prop.type === 'boolean'" class="flex items-center gap-2">
            <input v-model="paramValues[key]" type="checkbox" class="toggle toggle-sm toggle-primary" />
            <span class="text-xs">{{ paramValues[key] ? t('trafficAnalysis.workflowStudio.paramsEditor.booleanYes') : t('trafficAnalysis.workflowStudio.paramsEditor.booleanNo') }}</span>
          </div>

          <div v-else-if="prop.type === 'array'" class="space-y-1">
            <textarea v-model="paramValues[key]" class="textarea textarea-bordered textarea-sm font-mono text-xs w-full" :placeholder="t('trafficAnalysis.workflowStudio.paramsEditor.arrayPlaceholder')" rows="4"></textarea>
            <div class="text-xs text-base-content/50">{{ t('trafficAnalysis.workflowStudio.paramsEditor.onePerLine') }}</div>
          </div>

          <div v-else-if="prop.type === 'object'" class="space-y-1">
            <textarea v-model="paramValues[key]" class="textarea textarea-bordered textarea-sm font-mono text-xs" placeholder='{ "key": "value" }' rows="4" @blur="onValidateJson(String(key))"></textarea>
            <div v-if="jsonErrors[key]" class="text-xs text-error">{{ jsonErrors[key] }}</div>
          </div>

          <textarea v-else v-model="paramValues[key]" class="textarea textarea-bordered textarea-sm" rows="2"></textarea>

          <label v-if="prop.description && prop.description.trim() && prop.description.trim() !== '/'" class="label py-0">
            <span class="label-text-alt text-xs opacity-60">{{ prop.description }}</span>
          </label>
          <label v-if="prop.default !== undefined && !paramValues[key]" class="label py-0">
            <span class="label-text-alt text-xs text-info">{{ t('trafficAnalysis.workflowStudio.paramsEditor.defaultValue', { value: String(prop.default) }) }}</span>
          </label>
        </div>
      </div>
      <div class="p-3 flex gap-2 border-t border-base-300">
        <button class="btn btn-primary btn-sm flex-1" :disabled="hasValidationErrors" @click="onSaveParamsAndClose">{{ t('trafficAnalysis.workflowStudio.paramsEditor.save') }}</button>
        <button class="btn btn-outline btn-sm" @click="onCloseDrawer">{{ t('trafficAnalysis.workflowStudio.paramsEditor.cancel') }}</button>
      </div>
    </div>
  </Transition>

  <dialog ref="detailDialogRef" :open="showDetailDialog" class="modal" @click.self="$emit('update:showDetailDialog', false)">
    <div :class="['modal-box', detailDialogFullscreen ? 'max-w-[95vw] w-[95vw] max-h-[95vh] h-[95vh]' : 'max-w-3xl max-h-[80vh]']">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-bold text-lg">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.title') }}</h3>
        <div class="flex gap-2">
          <button class="btn btn-sm btn-ghost" :title="detailDialogFullscreen ? t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.exitFullscreen') : t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.fullscreen')" @click="$emit('update:detailDialogFullscreen', !detailDialogFullscreen)">
            <svg v-if="!detailDialogFullscreen" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" /></svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
          <button class="btn btn-sm btn-ghost" @click="$emit('update:showDetailDialog', false)">✕</button>
        </div>
      </div>
      <div v-if="detailLoading" class="flex justify-center py-8"><span class="loading loading-spinner loading-lg"></span></div>
      <div v-else-if="detailData" class="space-y-4">
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div><span class="text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.workflowName') }}:</span><span class="font-medium ml-2">{{ detailData.workflow_name }}</span></div>
          <div><span class="text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.status') }}:</span><span :class="getStatusBadgeClass(detailData.status)" class="badge badge-sm ml-2">{{ getStatusText(detailData.status) }}</span></div>
          <div><span class="text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.startTime') }}:</span><span class="ml-2">{{ formatDatetime(detailData.started_at) }}</span></div>
          <div><span class="text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.duration') }}:</span><span class="ml-2">{{ formatDuration(detailData.duration_ms) }}</span></div>
          <div v-if="detailData.error_message" class="col-span-2"><span class="text-base-content/60">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.error') }}:</span><span class="text-error ml-2">{{ detailData.error_message }}</span></div>
        </div>
        <div class="divider">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.steps') }}</div>
        <div :class="['space-y-2 overflow-y-auto', detailDialogFullscreen ? 'max-h-[calc(95vh-280px)]' : 'max-h-[40vh]']">
          <div v-if="!detailData.steps || detailData.steps.length === 0" class="text-center text-base-content/50 py-4">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.noSteps') }}</div>
          <div v-for="(step, idx) in detailData.steps" :key="step.step_id" class="collapse collapse-arrow bg-base-200">
            <input type="checkbox" :checked="idx === 0" />
            <div class="collapse-title text-sm font-medium flex items-center gap-2">
              <span class="badge badge-xs" :class="getStatusBadgeClass(step.status)">{{ idx + 1 }}</span>
              <span>{{ step.step_name || step.step_id }}</span>
              <span class="text-xs text-base-content/50 ml-auto mr-4">{{ formatDuration(step.duration_ms) }}</span>
            </div>
            <div class="collapse-content">
              <div v-if="step.error_message" class="text-error text-xs mb-2">{{ step.error_message }}</div>
              <pre v-if="step.result !== undefined && step.result !== null" :class="['text-xs bg-base-300 p-2 rounded overflow-x-auto', detailDialogFullscreen ? 'max-h-[60vh]' : 'max-h-48']">{{ formatResult(step.result) }}</pre>
              <div v-else class="text-xs text-base-content/50">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.noResult') }}</div>
            </div>
          </div>
        </div>
      </div>
      <div class="modal-action">
        <button class="btn btn-sm btn-ghost" :title="t('trafficAnalysis.workflowStudio.executionHistory.copyResultsTooltip')" @click="onCopyDetailResult"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.copy') }}</button>
        <button class="btn btn-sm" @click="$emit('update:showDetailDialog', false)">{{ t('trafficAnalysis.workflowStudio.executionHistory.detailDialog.close') }}</button>
      </div>
    </div>
  </dialog>

  <div v-if="showResultPanel" ref="resultPanelRef" class="fixed inset-y-0 right-0 w-[500px] bg-base-100 shadow-xl border-l border-base-300 z-50">
    <div class="p-3 flex items-center justify-between border-b border-base-300">
      <h2 class="text-base font-semibold">{{ t('trafficAnalysis.workflowStudio.resultPanel.title') }}</h2>
      <div class="flex gap-2">
        <button class="btn btn-xs btn-outline" :title="t('trafficAnalysis.workflowStudio.resultPanel.copyTooltip')" @click="onCopyResultToClipboard"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg></button>
        <button class="btn btn-xs btn-ghost" @click="onCloseResultPanel">✕</button>
      </div>
    </div>
    <div class="p-3 border-b border-base-300">
      <div class="text-sm font-semibold">{{ t('trafficAnalysis.workflowStudio.resultPanel.nodeId') }}</div>
      <div class="text-xs text-base-content/60 mt-1 font-mono">{{ selectedStepResult?.step_id }}</div>
      <div class="text-sm font-semibold mt-2">{{ t('trafficAnalysis.workflowStudio.resultPanel.nodeName') }}</div>
      <div class="text-xs text-base-content/60 mt-1">{{ selectedNodeName || t('trafficAnalysis.workflowStudio.resultPanel.unknown') }}</div>
    </div>
    <div class="p-3 overflow-auto h-[calc(100%-140px)]">
      <div class="text-sm font-semibold mb-2">{{ t('trafficAnalysis.workflowStudio.resultPanel.executionResult') }}</div>
      <pre class="bg-base-200 p-3 rounded text-xs font-mono overflow-x-auto">{{ formatResult(selectedStepResult?.result) }}</pre>
    </div>
    <div class="p-3 flex gap-2 border-t border-base-300">
      <button class="btn btn-primary btn-sm flex-1" @click="onEditNodeParams">{{ t('trafficAnalysis.workflowStudio.resultPanel.editParams') }}</button>
      <button class="btn btn-outline btn-sm" @click="onCloseResultPanel">{{ t('trafficAnalysis.workflowStudio.resultPanel.close') }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DetailData, ExecutionLog } from './workflowStudioExecutionSupport'

const { t } = useI18n()
const logsContainerRef = ref<HTMLElement | null>(null)
const detailDialogRef = ref<HTMLDialogElement | null>(null)
const resultPanelRef = ref<HTMLElement | null>(null)

defineProps<{
  showLogs: boolean
  executionLogs: ExecutionLog[]
  expandedLogs: Set<number>
  getLogClass: (level: string) => string
  formatTime: (date: Date) => string
  showNewWorkflowConfirm: boolean
  showMetaDialog: boolean
  workflowName: string
  workflowDescription: string
  workflowTags: string
  workflowVersion: string
  workflowIsTool: boolean
  nodeCount: number
  edgeCount: number
  drawerOpen: boolean
  selectedNode: any
  selectedSchema: any
  paramValues: Record<string, any>
  notificationRules: any[]
  availableTools: any[]
  jsonErrors: Record<string, string>
  hasValidationErrors: boolean
  showDetailDialog: boolean
  detailDialogFullscreen: boolean
  detailLoading: boolean
  detailData: DetailData | null
  showResultPanel: boolean
  selectedStepResult: { step_id: string; result: any } | null
  selectedNodeName: string
  formatDatetime: (dateStr?: string) => string
  formatDuration: (ms?: number) => string
  getStatusBadgeClass: (status: string) => string
  getStatusText: (status: string) => string
  formatResult: (result: any) => string
  getEnabledProviders: () => string[]
  getProviderModels: (providerKey: string) => any[]
  onClearLogs: () => void
  onToggleLogDetails: (idx: number) => void
  onConfirmNewWorkflowSave: () => void
  onConfirmNewWorkflowDiscard: () => void
  onCloseDrawer: () => void
  onToggleToolSelection: (key: string, toolName: string) => void
  onValidateJson: (key: string) => void
  onSaveParamsAndClose: () => void
  onCopyDetailResult: () => void
  onCopyResultToClipboard: () => void
  onCloseResultPanel: () => void
  onEditNodeParams: () => void
}>()

defineEmits<{
  'update:showLogs': [value: boolean]
  'update:showNewWorkflowConfirm': [value: boolean]
  'update:showMetaDialog': [value: boolean]
  'update:workflowName': [value: string]
  'update:workflowDescription': [value: string]
  'update:workflowTags': [value: string]
  'update:workflowVersion': [value: string]
  'update:workflowIsTool': [value: boolean]
  'update:showDetailDialog': [value: boolean]
  'update:detailDialogFullscreen': [value: boolean]
}>()

defineExpose({
  logsContainerRef,
  detailDialogRef,
  resultPanelRef,
})
</script>
