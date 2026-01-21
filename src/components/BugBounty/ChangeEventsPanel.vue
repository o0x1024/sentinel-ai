<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div class="flex items-center gap-2">
            <h2 class="card-title">{{ t('bugBounty.changeEvents.title') }}</h2>
            <span v-if="stats.pending_review > 0" class="badge badge-warning">
              {{ stats.pending_review }} {{ t('bugBounty.changeEvents.pendingReview') }}
            </span>
          </div>
          <div class="flex gap-2 flex-wrap">
            <!-- Filters -->
            <select v-model="filter.event_type" class="select select-sm select-bordered" @change="onFilterChange">
              <option value="">{{ t('bugBounty.changeEvents.allTypes') }}</option>
              <option value="asset_discovered">{{ t('bugBounty.changeEvents.types.assetDiscovered') }}</option>
              <option value="dns_change">{{ t('bugBounty.changeEvents.types.dnsChange') }}</option>
              <option value="certificate_change">{{ t('bugBounty.changeEvents.types.certificateChange') }}</option>
              <option value="content_change">{{ t('bugBounty.changeEvents.types.contentChange') }}</option>
              <option value="technology_change">{{ t('bugBounty.changeEvents.types.technologyChange') }}</option>
              <option value="api_change">{{ t('bugBounty.changeEvents.types.apiChange') }}</option>
            </select>
            <select v-model="filter.severity" class="select select-sm select-bordered" @change="onFilterChange">
              <option value="">{{ t('bugBounty.filter.allSeverities') }}</option>
              <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
              <option value="high">{{ t('bugBounty.severity.high') }}</option>
              <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
              <option value="low">{{ t('bugBounty.severity.low') }}</option>
            </select>
            <select v-model="filter.status" class="select select-sm select-bordered" @change="onFilterChange">
              <option value="">{{ t('bugBounty.filter.allStatuses') }}</option>
              <option value="new">{{ t('bugBounty.changeEvents.statuses.new') }}</option>
              <option value="analyzing">{{ t('bugBounty.changeEvents.statuses.analyzing') }}</option>
              <option value="workflow_triggered">{{ t('bugBounty.changeEvents.statuses.workflowTriggered') }}</option>
              <option value="review_required">{{ t('bugBounty.changeEvents.statuses.reviewRequired') }}</option>
              <option value="acknowledged">{{ t('bugBounty.changeEvents.statuses.acknowledged') }}</option>
              <option value="resolved">{{ t('bugBounty.changeEvents.statuses.resolved') }}</option>
              <option value="ignored">{{ t('bugBounty.changeEvents.statuses.ignored') }}</option>
            </select>
            <button class="btn btn-sm btn-outline" @click="loadEvents">
              <i class="fas fa-sync-alt mr-2"></i>
              {{ t('common.refresh') }}
            </button>
          </div>
        </div>
        
        <!-- Stats Cards -->
        <div class="grid grid-cols-4 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.changeEvents.totalEvents') }}</div>
            <div class="stat-value text-lg">{{ stats.total_events }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.changeEvents.pendingReview') }}</div>
            <div class="stat-value text-lg text-warning">{{ stats.pending_review }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.changeEvents.avgRiskScore') }}</div>
            <div class="stat-value text-lg">{{ stats.average_risk_score?.toFixed(1) || '0.0' }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.changeEvents.highSeverity') }}</div>
            <div class="stat-value text-lg text-error">{{ (stats.by_severity?.high || 0) + (stats.by_severity?.critical || 0) }}</div>
          </div>
        </div>
        
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="events.length === 0" class="text-center py-8">
          <i class="fas fa-shield-alt text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.changeEvents.empty') }}</p>
          <p class="text-sm text-base-content/50 mt-2">{{ t('bugBounty.changeEvents.emptyHint') }}</p>
        </div>
        
        <div v-else class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>{{ t('bugBounty.changeEvents.eventType') }}</th>
                <th>{{ t('bugBounty.table.title') }}</th>
                <th>{{ t('bugBounty.changeEvents.asset') }}</th>
                <th>{{ t('bugBounty.table.severity') }}</th>
                <th>{{ t('bugBounty.changeEvents.riskScore') }}</th>
                <th>{{ t('bugBounty.table.status') }}</th>
                <th>{{ t('bugBounty.table.date') }}</th>
                <th>{{ t('bugBounty.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="event in events" :key="event.id" class="hover">
                <td>
                  <span class="badge badge-outline badge-sm">
                    <i :class="getEventTypeIcon(event.event_type)" class="mr-1"></i>
                    {{ formatEventType(event.event_type) }}
                  </span>
                </td>
                <td>
                  <div class="font-medium">{{ event.title }}</div>
                  <div v-if="event.description" class="text-xs text-base-content/60 truncate max-w-xs">
                    {{ event.description }}
                  </div>
                </td>
                <td>
                  <span class="text-sm font-mono">{{ truncateAssetId(event.asset_id) }}</span>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getSeverityClass(event.severity)">
                    {{ event.severity }}
                  </span>
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <div class="radial-progress text-xs" :style="getRiskScoreStyle(event.risk_score)" :class="getRiskScoreClass(event.risk_score)">
                      {{ event.risk_score?.toFixed(0) || 0 }}
                    </div>
                  </div>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getStatusClass(event.status)">
                    {{ formatStatus(event.status) }}
                  </span>
                </td>
                <td>
                  <span class="text-sm">{{ formatDate(event.created_at) }}</span>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button class="btn btn-ghost btn-xs" @click="viewEvent(event)" :title="t('common.view')">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button 
                      v-if="event.auto_trigger_enabled && event.status === 'new'"
                      class="btn btn-ghost btn-xs text-primary" 
                      @click="triggerWorkflow(event)" 
                      :title="t('bugBounty.changeEvents.triggerWorkflow')"
                    >
                      <i class="fas fa-play"></i>
                    </button>
                    <div class="dropdown dropdown-end">
                      <label tabindex="0" class="btn btn-ghost btn-xs">
                        <i class="fas fa-ellipsis-v"></i>
                      </label>
                      <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-lg bg-base-100 rounded-box w-40">
                        <li><a @click="updateStatus(event, 'acknowledged')">{{ t('bugBounty.changeEvents.acknowledge') }}</a></li>
                        <li><a @click="updateStatus(event, 'resolved')">{{ t('bugBounty.changeEvents.resolve') }}</a></li>
                        <li><a @click="updateStatus(event, 'ignored')">{{ t('bugBounty.changeEvents.ignore') }}</a></li>
                        <li class="divider"></li>
                        <li><a class="text-error" @click="deleteEvent(event)">{{ t('common.delete') }}</a></li>
                      </ul>
                    </div>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const emit = defineEmits<{
  (e: 'view', event: any): void
  (e: 'trigger-workflow', event: any): void
}>()

// State
const loading = ref(false)
const events = ref<any[]>([])
const stats = ref({
  total_events: 0,
  by_type: {} as Record<string, number>,
  by_severity: {} as Record<string, number>,
  by_status: {} as Record<string, number>,
  pending_review: 0,
  average_risk_score: 0,
})

const filter = reactive({
  event_type: '',
  severity: '',
  status: '',
})

// Methods
const loadEvents = async () => {
  try {
    loading.value = true
    const filterParams: any = {}
    if (filter.event_type) filterParams.event_types = [filter.event_type]
    if (filter.severity) filterParams.severities = [filter.severity]
    if (filter.status) filterParams.statuses = [filter.status]
    
    events.value = await invoke('bounty_list_change_events', { 
      filter: Object.keys(filterParams).length > 0 ? filterParams : null 
    })
  } catch (error) {
    console.error('Failed to load change events:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loading.value = false
  }
}

const loadStats = async () => {
  try {
    stats.value = await invoke('bounty_get_change_event_stats', { programId: null })
  } catch (error) {
    console.error('Failed to load change event stats:', error)
  }
}

const onFilterChange = () => {
  loadEvents()
}

const viewEvent = (event: any) => {
  emit('view', event)
}

const triggerWorkflow = (event: any) => {
  emit('trigger-workflow', event)
}

const updateStatus = async (event: any, status: string) => {
  try {
    await invoke('bounty_update_change_event_status', { id: event.id, status })
    toast.success(t('bugBounty.changeEvents.statusUpdated'))
    await loadEvents()
    await loadStats()
  } catch (error) {
    console.error('Failed to update status:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const deleteEvent = async (event: any) => {
  if (!confirm(t('bugBounty.changeEvents.confirmDelete'))) return
  try {
    await invoke('bounty_delete_change_event', { id: event.id })
    toast.success(t('bugBounty.changeEvents.deleted'))
    await loadEvents()
    await loadStats()
  } catch (error) {
    console.error('Failed to delete event:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

// Helpers
const formatDate = (date: string) => {
  if (!date) return '-'
  return new Date(date).toLocaleDateString()
}

const truncateAssetId = (id: string) => {
  if (!id) return '-'
  return id.length > 20 ? `${id.substring(0, 20)}...` : id
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

const getRiskScoreStyle = (score: number) => {
  const percentage = Math.min(score || 0, 100)
  return `--value:${percentage}; --size:2rem;`
}

const getRiskScoreClass = (score: number) => {
  if (score >= 70) return 'text-error'
  if (score >= 40) return 'text-warning'
  return 'text-success'
}

// Lifecycle
onMounted(async () => {
  await loadEvents()
  await loadStats()
})
</script>
