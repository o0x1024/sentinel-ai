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
            <span v-if="selectedIds.length > 0" class="badge badge-primary">
              {{ selectedIds.length }} {{ t('bugBounty.batch.selected') }}
            </span>
          </div>
          <div class="flex gap-2 flex-wrap">
            <div v-if="events.length > 0" class="flex gap-2">
              <button class="btn btn-sm btn-ghost" @click="selectCurrentPage">
                <i class="fas fa-list-check mr-2"></i>
                {{ t('bugBounty.batch.selectCurrentPage') }}
              </button>
              <button class="btn btn-sm btn-ghost" @click="selectAllFiltered">
                <i class="fas fa-layer-group mr-2"></i>
                {{ t('bugBounty.batch.selectAllFiltered') }}
              </button>
            </div>

            <div v-if="selectedIds.length > 0" class="flex gap-2">
              <div class="dropdown dropdown-end">
                <label tabindex="0" class="btn btn-sm btn-outline">
                  <i class="fas fa-edit mr-2"></i>
                  {{ t('bugBounty.batch.updateStatus') }}
                </label>
                <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-lg bg-base-100 rounded-box w-48">
                  <li><a @click="batchUpdateStatus('new')">{{ t('bugBounty.changeEvents.statuses.new') }}</a></li>
                  <li><a @click="batchUpdateStatus('analyzing')">{{ t('bugBounty.changeEvents.statuses.analyzing') }}</a></li>
                  <li><a @click="batchUpdateStatus('workflow_triggered')">{{ t('bugBounty.changeEvents.statuses.workflowTriggered') }}</a></li>
                  <li><a @click="batchUpdateStatus('review_required')">{{ t('bugBounty.changeEvents.statuses.reviewRequired') }}</a></li>
                  <li><a @click="batchUpdateStatus('acknowledged')">{{ t('bugBounty.changeEvents.statuses.acknowledged') }}</a></li>
                  <li><a @click="batchUpdateStatus('resolved')">{{ t('bugBounty.changeEvents.statuses.resolved') }}</a></li>
                  <li><a @click="batchUpdateStatus('ignored')">{{ t('bugBounty.changeEvents.statuses.ignored') }}</a></li>
                </ul>
              </div>
              <button class="btn btn-sm btn-error btn-outline" @click="batchDelete">
                <i class="fas fa-trash mr-2"></i>
                {{ t('bugBounty.batch.delete') }}
              </button>
              <button class="btn btn-sm btn-ghost" @click="clearSelection">
                <i class="fas fa-times"></i>
              </button>
            </div>

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
            <button class="btn btn-sm btn-primary" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.changeEvents.createEvent') }}
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
                <th class="w-10">
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    :checked="isAllSelected"
                    :indeterminate="isPartialSelected"
                    @change="toggleSelectAll"
                  />
                </th>
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
              <tr v-for="event in events" :key="event.id" class="hover" :class="{ 'bg-primary/10': isSelected(event.id) }">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    :checked="isSelected(event.id)"
                    @change="toggleSelect(event.id)"
                  />
                </td>
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
                    <button type="button" class="btn btn-ghost btn-xs" @click="viewEvent(event)" :title="t('common.view')">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button 
                      type="button"
                      class="btn btn-ghost btn-xs text-primary" 
                      @click="triggerWorkflow(event)" 
                      :title="t('bugBounty.changeEvents.triggerWorkflow')"
                    >
                      <i class="fas fa-play"></i>
                    </button>
                    <button
                      type="button"
                      class="btn btn-ghost btn-xs"
                      :title="t('bugBounty.table.actions')"
                      @click.stop="toggleActionMenu($event, event)"
                    >
                        <i class="fas fa-ellipsis-v"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="events.length > 0" class="flex justify-center py-4">
          <div class="join">
            <button class="join-item btn btn-sm" :disabled="page <= 1 || loading" @click="goToPrevPage">«</button>
            <button class="join-item btn btn-sm">{{ t('bugBounty.changeEvents.pageInfo', { page }) }}</button>
            <button class="join-item btn btn-sm" :disabled="!hasNext || loading" @click="goToNextPage">»</button>
          </div>
        </div>
      </div>
    </div>
  </div>
  <Teleport to="body">
    <div
      v-if="actionMenu.visible && actionMenu.event"
      class="fixed inset-0 z-[1200]"
      @click="closeActionMenu"
    >
      <ul
        ref="actionMenuRef"
        class="menu fixed w-44 rounded-box border border-base-300 bg-base-100 p-2 shadow-2xl"
        :style="{
          top: `${actionMenu.top}px`,
          left: `${actionMenu.left}px`,
        }"
        @click.stop
      >
        <li><a @click="handleActionMenuStatus('acknowledged')">{{ t('bugBounty.changeEvents.acknowledge') }}</a></li>
        <li><a @click="handleActionMenuStatus('resolved')">{{ t('bugBounty.changeEvents.resolve') }}</a></li>
        <li><a @click="handleActionMenuStatus('ignored')">{{ t('bugBounty.changeEvents.ignore') }}</a></li>
        <li class="divider my-1"></li>
        <li><a class="text-error" @click="handleActionMenuDelete">{{ t('common.delete') }}</a></li>
      </ul>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed, watch, onBeforeUnmount, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'
import { dialog } from '../../composables/useDialog'

const { t } = useI18n()
const toast = useToast()

const emit = defineEmits<{
  (e: 'view', event: any): void
  (e: 'trigger-workflow', event: any): void
  (e: 'create'): void
}>()

// State
const loading = ref(false)
const events = ref<any[]>([])
const page = ref(1)
const pageSize = 10
const hasNext = ref(false)
const selectedIds = ref<string[]>([])
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

const actionMenuRef = ref<HTMLElement | null>(null)
const actionMenu = reactive({
  visible: false,
  top: 0,
  left: 0,
  event: null as any | null,
})

const buildFilterParams = (includePagination = true) => {
  const filterParams: any = {}
  if (filter.event_type) filterParams.event_types = [filter.event_type]
  if (filter.severity) filterParams.severities = [filter.severity]
  if (filter.status) filterParams.statuses = [filter.status]
  if (includePagination) {
    filterParams.limit = pageSize + 1
    filterParams.offset = (page.value - 1) * pageSize
  }
  return filterParams
}

const isAllSelected = computed(() => {
  return events.value.length > 0 && events.value.every(event => selectedIds.value.includes(event.id))
})

const isPartialSelected = computed(() => {
  const selectedOnPage = events.value.filter(event => selectedIds.value.includes(event.id)).length
  return selectedOnPage > 0 && selectedOnPage < events.value.length
})

const isSelected = (id: string) => selectedIds.value.includes(id)

const toggleSelect = (id: string) => {
  const index = selectedIds.value.indexOf(id)
  if (index === -1) {
    selectedIds.value.push(id)
  } else {
    selectedIds.value.splice(index, 1)
  }
}

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    const currentPageIds = new Set(events.value.map(event => event.id))
    selectedIds.value = selectedIds.value.filter(id => !currentPageIds.has(id))
  } else {
    const next = new Set(selectedIds.value)
    for (const event of events.value) {
      next.add(event.id)
    }
    selectedIds.value = [...next]
  }
}

const clearSelection = () => {
  selectedIds.value = []
}

const selectCurrentPage = () => {
  selectedIds.value = events.value.map(event => event.id)
}

const selectAllFiltered = async () => {
  try {
    loading.value = true
    const rows = await invoke<any[]>('bounty_list_change_events', {
      filter: Object.keys(buildFilterParams(false)).length > 0 ? buildFilterParams(false) : null,
    })
    selectedIds.value = rows.map(event => event.id)
  } catch (error) {
    console.error('Failed to load filtered change events for batch selection:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loading.value = false
  }
}

// Methods
const loadEvents = async () => {
  try {
    loading.value = true
    const filterParams = buildFilterParams()
    
    const rows = await invoke<any[]>('bounty_list_change_events', { 
      filter: Object.keys(filterParams).length > 0 ? filterParams : null 
    })
    hasNext.value = rows.length > pageSize
    events.value = hasNext.value ? rows.slice(0, pageSize) : rows
    clearSelection()
  } catch (error) {
    console.error('Failed to load change events:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    events.value = []
    hasNext.value = false
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
  page.value = 1
  clearSelection()
  loadEvents()
}

const goToPrevPage = () => {
  if (page.value <= 1) return
  page.value -= 1
  clearSelection()
  loadEvents()
}

const goToNextPage = () => {
  if (!hasNext.value) return
  page.value += 1
  clearSelection()
  loadEvents()
}

const batchUpdateStatus = async (status: string) => {
  if (selectedIds.value.length === 0) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmUpdateStatus', { count: selectedIds.value.length })))) return

  try {
    loading.value = true
    const ids = [...selectedIds.value]
    await Promise.all(
      ids.map(id => invoke('bounty_update_change_event_status', { id, status }))
    )
    toast.success(t('bugBounty.batch.updateSuccess', { count: ids.length }))
    clearSelection()
    await loadEvents()
    await loadStats()
  } catch (error) {
    console.error('Failed to batch update change event statuses:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  } finally {
    loading.value = false
  }
}

const batchDelete = async () => {
  if (selectedIds.value.length === 0) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDelete', { count: selectedIds.value.length })))) return

  try {
    loading.value = true
    const ids = [...selectedIds.value]
    await Promise.all(ids.map(id => invoke('bounty_delete_change_event', { id })))
    toast.success(t('bugBounty.batch.deleteSuccess', { count: ids.length }))
    clearSelection()
    if (events.value.length === ids.length && page.value > 1) {
      page.value -= 1
    }
    await loadEvents()
    await loadStats()
  } catch (error) {
    console.error('Failed to batch delete change events:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    loading.value = false
  }
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
  if (!(await dialog.confirm(t('bugBounty.changeEvents.confirmDelete')))) return
  try {
    await invoke('bounty_delete_change_event', { id: event.id })
    toast.success(t('bugBounty.changeEvents.deleted'))
    if (events.value.length === 1 && page.value > 1) {
      page.value -= 1
    }
    await loadEvents()
    await loadStats()
  } catch (error) {
    console.error('Failed to delete event:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const closeActionMenu = () => {
  actionMenu.visible = false
  actionMenu.event = null
}

const positionActionMenu = (trigger: HTMLElement) => {
  const menuWidth = actionMenuRef.value?.offsetWidth || 176
  const menuHeight = actionMenuRef.value?.offsetHeight || 180
  const gap = 8
  const margin = 8
  const rect = trigger.getBoundingClientRect()

  const preferredLeft = rect.left - menuWidth - gap
  const fallbackLeft = rect.right + gap
  const nextLeft = preferredLeft >= margin
    ? preferredLeft
    : Math.min(fallbackLeft, window.innerWidth - menuWidth - margin)

  const centeredTop = rect.top + (rect.height / 2) - (menuHeight / 2)
  const nextTop = Math.min(
    Math.max(margin, centeredTop),
    window.innerHeight - menuHeight - margin,
  )

  actionMenu.left = nextLeft
  actionMenu.top = nextTop
}

const toggleActionMenu = async (mouseEvent: MouseEvent, event: any) => {
  const trigger = mouseEvent.currentTarget
  if (!(trigger instanceof HTMLElement)) return

  if (actionMenu.visible && actionMenu.event?.id === event.id) {
    closeActionMenu()
    return
  }

  actionMenu.event = event
  actionMenu.visible = true
  await nextTick()
  positionActionMenu(trigger)
}

const handleActionMenuStatus = async (status: string) => {
  const event = actionMenu.event
  closeActionMenu()
  if (!event) return
  await updateStatus(event, status)
}

const handleActionMenuDelete = async () => {
  const event = actionMenu.event
  closeActionMenu()
  if (!event) return
  await deleteEvent(event)
}

const handleActionMenuEscape = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    closeActionMenu()
  }
}

const bindActionMenuListeners = () => {
  document.addEventListener('scroll', closeActionMenu, true)
  window.addEventListener('resize', closeActionMenu)
  document.addEventListener('keydown', handleActionMenuEscape)
}

const unbindActionMenuListeners = () => {
  document.removeEventListener('scroll', closeActionMenu, true)
  window.removeEventListener('resize', closeActionMenu)
  document.removeEventListener('keydown', handleActionMenuEscape)
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

watch(
  () => actionMenu.visible,
  (visible) => {
    if (visible) {
      bindActionMenuListeners()
    } else {
      unbindActionMenuListeners()
    }
  },
)

// Lifecycle
onMounted(async () => {
  await loadEvents()
  await loadStats()
})

onBeforeUnmount(() => {
  unbindActionMenuListeners()
})
</script>
