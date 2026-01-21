<template>
  <div v-if="visible" class="modal modal-open">
    <div class="modal-box max-w-3xl">
      <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="$emit('close')">✕</button>
      
      <h3 class="font-bold text-lg flex items-center gap-2 mb-4">
        <i :class="getEventTypeIcon(event?.event_type)" class="text-xl"></i>
        {{ event?.title }}
      </h3>

      <div class="tabs tabs-boxed mb-4">
        <a class="tab" :class="{ 'tab-active': activeTab === 'details' }" @click="activeTab = 'details'">
          {{ t('bugBounty.findingDetail.detailsTab') }}
        </a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'diff' }" @click="activeTab = 'diff'">
          {{ t('bugBounty.changeEvents.diffTab') }}
        </a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'workflows' }" @click="activeTab = 'workflows'">
          {{ t('bugBounty.changeEvents.workflowsTab') }}
          <span v-if="triggeredWorkflows.length > 0" class="badge badge-sm ml-1">{{ triggeredWorkflows.length }}</span>
        </a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'findings' }" @click="activeTab = 'findings'">
          {{ t('bugBounty.tabs.findings') }}
          <span v-if="generatedFindings.length > 0" class="badge badge-sm ml-1">{{ generatedFindings.length }}</span>
        </a>
      </div>

      <!-- Details Tab -->
      <div v-if="activeTab === 'details'" class="space-y-4">
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.eventType') }}</span></label>
            <div class="badge badge-outline">
              <i :class="getEventTypeIcon(event?.event_type)" class="mr-1"></i>
              {{ formatEventType(event?.event_type) }}
            </div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.table.status') }}</span></label>
            <div class="flex items-center gap-2">
              <span class="badge" :class="getStatusClass(event?.status)">{{ formatStatus(event?.status) }}</span>
              <select 
                v-model="newStatus" 
                class="select select-xs select-bordered"
                @change="updateStatus"
              >
                <option value="new">{{ t('bugBounty.changeEvents.statuses.new') }}</option>
                <option value="analyzing">{{ t('bugBounty.changeEvents.statuses.analyzing') }}</option>
                <option value="review_required">{{ t('bugBounty.changeEvents.statuses.reviewRequired') }}</option>
                <option value="acknowledged">{{ t('bugBounty.changeEvents.statuses.acknowledged') }}</option>
                <option value="resolved">{{ t('bugBounty.changeEvents.statuses.resolved') }}</option>
                <option value="ignored">{{ t('bugBounty.changeEvents.statuses.ignored') }}</option>
              </select>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.table.severity') }}</span></label>
            <span class="badge" :class="getSeverityClass(event?.severity)">{{ event?.severity }}</span>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.riskScore') }}</span></label>
            <div class="flex items-center gap-2">
              <progress class="progress w-24" :class="getRiskProgressClass(event?.risk_score)" :value="event?.risk_score || 0" max="100"></progress>
              <span class="font-mono">{{ event?.risk_score?.toFixed(1) || '0.0' }}</span>
            </div>
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.asset') }}</span></label>
          <div class="font-mono text-sm bg-base-200 p-2 rounded">{{ event?.asset_id }}</div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.description') }}</span></label>
          <div class="bg-base-200 p-3 rounded min-h-[60px]">
            {{ event?.description || t('bugBounty.findingDetail.noDescription') }}
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.detectionMethod') }}</span></label>
          <div class="badge badge-ghost">{{ event?.detection_method }}</div>
        </div>

        <div class="grid grid-cols-2 gap-4 text-sm text-base-content/70">
          <div>
            <span class="font-medium">{{ t('bugBounty.programDetail.createdAt') }}:</span>
            {{ formatDateTime(event?.created_at) }}
          </div>
          <div v-if="event?.resolved_at">
            <span class="font-medium">{{ t('bugBounty.changeEvents.resolvedAt') }}:</span>
            {{ formatDateTime(event?.resolved_at) }}
          </div>
        </div>
      </div>

      <!-- Diff Tab -->
      <div v-if="activeTab === 'diff'" class="space-y-4">
        <div v-if="event?.old_value || event?.new_value" class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.oldValue') }}</span></label>
            <div class="bg-error/10 p-3 rounded font-mono text-sm max-h-40 overflow-auto">
              {{ event?.old_value || '-' }}
            </div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.newValue') }}</span></label>
            <div class="bg-success/10 p-3 rounded font-mono text-sm max-h-40 overflow-auto">
              {{ event?.new_value || '-' }}
            </div>
          </div>
        </div>

        <div v-if="event?.diff" class="form-control">
          <label class="label"><span class="label-text font-medium">{{ t('bugBounty.changeEvents.diff') }}</span></label>
          <pre class="bg-base-200 p-3 rounded font-mono text-sm max-h-60 overflow-auto whitespace-pre-wrap">{{ event?.diff }}</pre>
        </div>

        <div v-if="!event?.old_value && !event?.new_value && !event?.diff" class="text-center py-8 text-base-content/60">
          <i class="fas fa-file-code text-4xl mb-3 opacity-30"></i>
          <p>{{ t('bugBounty.changeEvents.noDiff') }}</p>
        </div>
      </div>

      <!-- Workflows Tab -->
      <div v-if="activeTab === 'workflows'" class="space-y-4">
        <div class="flex justify-between items-center">
          <span class="text-sm text-base-content/70">{{ t('bugBounty.changeEvents.triggeredWorkflows') }}</span>
          <button 
            v-if="event?.auto_trigger_enabled" 
            class="btn btn-sm btn-primary"
            @click="$emit('trigger-workflow', event)"
          >
            <i class="fas fa-play mr-2"></i>
            {{ t('bugBounty.changeEvents.triggerWorkflow') }}
          </button>
        </div>

        <div v-if="triggeredWorkflows.length === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-project-diagram text-4xl mb-3 opacity-30"></i>
          <p>{{ t('bugBounty.changeEvents.noWorkflows') }}</p>
        </div>

        <div v-else class="space-y-2">
          <div v-for="wfId in triggeredWorkflows" :key="wfId" class="flex items-center justify-between bg-base-200 p-3 rounded">
            <div class="flex items-center gap-2">
              <i class="fas fa-project-diagram text-primary"></i>
              <span class="font-mono text-sm">{{ wfId }}</span>
            </div>
            <button class="btn btn-xs btn-ghost">
              <i class="fas fa-external-link-alt"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- Findings Tab -->
      <div v-if="activeTab === 'findings'" class="space-y-4">
        <div class="flex justify-between items-center">
          <span class="text-sm text-base-content/70">{{ t('bugBounty.changeEvents.generatedFindings') }}</span>
        </div>

        <div v-if="generatedFindings.length === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-bug text-4xl mb-3 opacity-30"></i>
          <p>{{ t('bugBounty.changeEvents.noFindings') }}</p>
        </div>

        <div v-else class="space-y-2">
          <div v-for="fId in generatedFindings" :key="fId" class="flex items-center justify-between bg-base-200 p-3 rounded">
            <div class="flex items-center gap-2">
              <i class="fas fa-bug text-error"></i>
              <span class="font-mono text-sm">{{ fId }}</span>
            </div>
            <button class="btn btn-xs btn-ghost">
              <i class="fas fa-external-link-alt"></i>
            </button>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="$emit('close')">{{ t('common.close') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  visible: boolean
  event: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
  (e: 'trigger-workflow', event: any): void
}>()

const activeTab = ref('details')
const newStatus = ref('')

// Computed
const triggeredWorkflows = computed(() => {
  if (!props.event?.triggered_workflows_json) return []
  try {
    return JSON.parse(props.event.triggered_workflows_json)
  } catch {
    return []
  }
})

const generatedFindings = computed(() => {
  if (!props.event?.generated_findings_json) return []
  try {
    return JSON.parse(props.event.generated_findings_json)
  } catch {
    return []
  }
})

// Watch
watch(() => props.event, (event) => {
  if (event) {
    newStatus.value = event.status
  }
}, { immediate: true })

// Methods
const updateStatus = async () => {
  if (!props.event || newStatus.value === props.event.status) return
  try {
    await invoke('bounty_update_change_event_status', { 
      id: props.event.id, 
      status: newStatus.value 
    })
    toast.success(t('bugBounty.changeEvents.statusUpdated'))
    emit('updated')
  } catch (error) {
    console.error('Failed to update status:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const formatDateTime = (date: string) => {
  if (!date) return '-'
  return new Date(date).toLocaleString()
}

const formatEventType = (type: string) => {
  const map: Record<string, string> = {
    asset_discovered: t('bugBounty.changeEvents.types.assetDiscovered'),
    asset_removed: t('bugBounty.changeEvents.types.assetRemoved'),
    asset_modified: t('bugBounty.changeEvents.types.assetModified'),
    dns_change: t('bugBounty.changeEvents.types.dnsChange'),
    certificate_change: t('bugBounty.changeEvents.types.certificateChange'),
    technology_change: t('bugBounty.changeEvents.types.technologyChange'),
    port_change: t('bugBounty.changeEvents.types.portChange'),
    service_change: t('bugBounty.changeEvents.types.serviceChange'),
    content_change: t('bugBounty.changeEvents.types.contentChange'),
    api_change: t('bugBounty.changeEvents.types.apiChange'),
    configuration_exposed: t('bugBounty.changeEvents.types.configurationExposed'),
  }
  return map[type] || type
}

const formatStatus = (status: string) => {
  const map: Record<string, string> = {
    new: t('bugBounty.changeEvents.statuses.new'),
    analyzing: t('bugBounty.changeEvents.statuses.analyzing'),
    workflow_triggered: t('bugBounty.changeEvents.statuses.workflowTriggered'),
    review_required: t('bugBounty.changeEvents.statuses.reviewRequired'),
    acknowledged: t('bugBounty.changeEvents.statuses.acknowledged'),
    resolved: t('bugBounty.changeEvents.statuses.resolved'),
    ignored: t('bugBounty.changeEvents.statuses.ignored'),
  }
  return map[status] || status
}

const getEventTypeIcon = (type: string) => {
  const icons: Record<string, string> = {
    asset_discovered: 'fas fa-plus-circle text-success',
    asset_removed: 'fas fa-minus-circle text-error',
    asset_modified: 'fas fa-edit text-info',
    dns_change: 'fas fa-globe text-primary',
    certificate_change: 'fas fa-certificate text-warning',
    technology_change: 'fas fa-code text-secondary',
    port_change: 'fas fa-door-open text-info',
    service_change: 'fas fa-server text-primary',
    content_change: 'fas fa-file-alt text-accent',
    api_change: 'fas fa-plug text-warning',
    configuration_exposed: 'fas fa-exclamation-triangle text-error',
  }
  return icons[type] || 'fas fa-info-circle'
}

const getSeverityClass = (severity: string) => {
  const classes: Record<string, string> = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-ghost',
  }
  return classes[severity] || 'badge-ghost'
}

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    new: 'badge-primary',
    analyzing: 'badge-info',
    workflow_triggered: 'badge-secondary',
    review_required: 'badge-warning',
    acknowledged: 'badge-accent',
    resolved: 'badge-success',
    ignored: 'badge-ghost',
  }
  return classes[status] || 'badge-ghost'
}

const getRiskProgressClass = (score: number) => {
  if (score >= 70) return 'progress-error'
  if (score >= 40) return 'progress-warning'
  return 'progress-success'
}
</script>
