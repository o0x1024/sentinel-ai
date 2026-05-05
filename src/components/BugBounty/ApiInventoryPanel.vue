<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="card-title">{{ t('bugBounty.apiInventory.title') }}</h2>
            <p class="text-sm text-base-content/60">
              {{ currentProgramLabel }}
            </p>
          </div>
          <div class="flex items-center gap-2">
            <button class="btn btn-sm btn-outline" :disabled="loading" @click="loadTargets(true)">
              <i class="fas fa-sync-alt mr-2"></i>
              {{ t('common.refresh') }}
            </button>
            <button class="btn btn-sm btn-ghost" :disabled="loading" @click="clearFilters">
              <i class="fas fa-filter-circle-xmark mr-2"></i>
              {{ t('bugBounty.apiInventory.clearFilters') }}
            </button>
          </div>
        </div>

        <div class="mt-4 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-5">
          <label class="input input-bordered input-sm flex items-center gap-2">
            <i class="fas fa-search text-base-content/50"></i>
            <input
              v-model.trim="search"
              type="text"
              class="grow"
              :placeholder="t('bugBounty.apiInventory.searchCompactPlaceholder')"
            />
          </label>

          <select
            v-model="programFilter"
            class="select select-bordered select-sm"
            @change="handleProgramFilterChange"
          >
            <option value="">{{ t('bugBounty.apiInventory.allPrograms') }}</option>
            <option v-for="program in programOptions" :key="program.id" :value="String(program.id)">
              {{ program.name || program.id }}
            </option>
          </select>

          <select v-model="statusFilter" class="select select-bordered select-sm">
            <option value="all">{{ t('bugBounty.apiInventory.allStatuses') }}</option>
            <option value="success">{{ t('common.success') }}</option>
            <option value="failed">{{ t('common.failed') }}</option>
          </select>
          
          <select v-model="sortBy" class="select select-bordered select-sm">
            <option value="observed_desc">{{ t('bugBounty.apiInventory.sortNewest') }}</option>
            <option value="endpoint_desc">{{ t('bugBounty.apiInventory.sortEndpointCount') }}</option>
            <option value="changes_desc">{{ t('bugBounty.apiInventory.sortChanges') }}</option>
            <option value="base_url_asc">{{ t('bugBounty.apiInventory.sortBaseUrl') }}</option>
          </select>

          <button
            type="button"
            class="btn btn-sm btn-outline"
            :class="{ 'btn-primary': advancedFiltersExpanded || hasAdvancedFilters }"
            @click="advancedFiltersExpanded = !advancedFiltersExpanded"
          >
            <i class="fas" :class="advancedFiltersExpanded ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
            {{ advancedFiltersExpanded ? t('bugBounty.apiInventory.hideMoreFilters') : t('bugBounty.apiInventory.moreFilters') }}
            <span v-if="activeAdvancedFilterCount > 0" class="badge badge-primary badge-sm">
              {{ activeAdvancedFilterCount }}
            </span>
          </button>
        </div>

        <div
          v-if="advancedFiltersExpanded"
          class="mt-3 grid grid-cols-1 gap-3 rounded-lg border border-base-200 bg-base-200/40 p-3 md:grid-cols-2"
        >
          <select v-model="executionModeFilter" class="select select-bordered select-sm">
            <option value="all">{{ t('bugBounty.apiInventory.allExecutionModes') }}</option>
            <option value="scheduler">{{ t('bugBounty.monitor.scheduledRun') }}</option>
            <option value="manual">{{ t('bugBounty.monitor.manualRun') }}</option>
          </select>

          <select v-model="capabilityFilter" class="select select-bordered select-sm">
            <option value="all">{{ t('bugBounty.apiInventory.allCapabilities') }}</option>
            <option value="changed">{{ t('bugBounty.apiInventory.changedOnly') }}</option>
            <option value="errors">{{ t('bugBounty.apiInventory.withErrors') }}</option>
          </select>
        </div>

        <div class="mt-4 grid grid-cols-2 gap-4 xl:grid-cols-5">
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.targets') }}</div>
            <div class="stat-value text-lg">{{ filteredTargetCount }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.endpoints') }}</div>
            <div class="stat-value text-lg">{{ totalEndpoints }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('common.success') }}</div>
            <div class="stat-value text-lg text-success">{{ successfulTargets }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('common.failed') }}</div>
            <div class="stat-value text-lg text-error">{{ failedTargets }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.changes') }}</div>
            <div class="stat-value text-lg text-warning">{{ changedTargets }}</div>
          </div>
        </div>
                  <span class="text-xs font-normal text-base-content/60">
                    {{ t('bugBounty.apiInventory.filterSummary', {
                      total: totalTargetCount,
                      filtered: filteredTargetCount,
                    }) }}
          </span>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(360px,440px)_minmax(0,1fr)]">
      <div class="card bg-base-100 shadow-md">
        <div class="card-body p-0">
          <div class="border-b border-base-200 px-4 py-3">
            <div class="flex flex-col gap-3">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>{{ t('bugBounty.apiInventory.targets') }}</span>

                  <span v-if="selectedTargetKeys.length > 0" class="badge badge-primary badge-sm">
                    {{ selectedTargetKeys.length }} {{ t('bugBounty.batch.selected') }}
                  </span>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <button
                    class="btn btn-xs btn-ghost"
                    :disabled="loading || deleting || filteredTargetCount === 0 || allFilteredSelected"
                    @click="selectAllFilteredTargets"
                  >
                    <i class="fas fa-layer-group mr-2"></i>
                    {{ t('bugBounty.batch.selectAllFiltered') }}
                  </button>
                  <button
                    v-if="selectedTargetKeys.length > 0"
                    class="btn btn-xs btn-error btn-outline"
                    :disabled="loading || deleting"
                    @click="batchDeleteTargets"
                  >
                    <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
                    <i v-else class="fas fa-trash mr-2"></i>
                    {{ t('bugBounty.batch.delete') }}
                  </button>
                  <button
                    v-if="selectedTargetKeys.length > 0"
                    class="btn btn-xs btn-ghost"
                    :disabled="loading || deleting"
                    @click="clearSelection"
                  >
                    <i class="fas fa-times"></i>
                  </button>
                </div>
              </div>

              <div v-if="selectedTargetKeys.length > 0" class="text-xs text-base-content/60">
                {{ t('bugBounty.batch.selectionSummary', {
                  selected: selectedTargetKeys.length,
                  page: targets.length,
                  total: filteredTargetCount,
                }) }}
              </div>
            </div>
          </div>

          <div v-if="loading" class="flex justify-center py-10">
            <span class="loading loading-spinner loading-lg"></span>
          </div>

          <div v-else-if="targets.length === 0" class="px-4 py-10 text-center text-base-content/60">
            <i class="fas fa-plug text-4xl opacity-30"></i>
            <p class="mt-3">{{ t('bugBounty.apiInventory.empty') }}</p>
          </div>

          <div v-else class="max-h-[72vh] overflow-auto" @scroll="handleTargetListScroll">
            <div
              v-for="target in targets"
              :key="targetKeyOf(target)"
              class="w-full border-b border-base-200 px-4 py-3 text-left transition-colors hover:bg-base-200/70"
              :class="selectedTargetKey === targetKeyOf(target) ? 'bg-primary/10' : ''"
              @click="selectTarget(target)"
            >
              <div class="flex items-start gap-3">
                <input
                  type="checkbox"
                  class="checkbox checkbox-sm mt-1"
                  :checked="isTargetSelected(target)"
                  :disabled="loading || deleting"
                  @click.stop
                  @change="toggleTargetSelection(target)"
                />

                <div class="min-w-0 flex-1">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate font-medium">{{ target.base_url }}</div>
                      <div class="mt-2 flex flex-wrap gap-2 text-xs">
                        <span class="badge badge-outline badge-sm">
                          {{ programNameForId(target.program_id) }}
                        </span>
                        <span v-if="target.execution_mode" class="badge badge-ghost badge-sm">
                          {{ target.execution_mode }}
                        </span>
                        <span v-if="target.task_name" class="badge badge-ghost badge-sm">
                          {{ target.task_name }}
                        </span>
                      </div>
                    </div>

                    <div class="flex shrink-0 items-start gap-2">
                      <span class="badge badge-sm" :class="target.success ? 'badge-success' : 'badge-error'">
                        {{ target.success ? t('common.success') : t('common.failed') }}
                      </span>
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs text-error"
                        :disabled="loading || deleting"
                        :title="t('common.delete')"
                        @click.stop="deleteSingleTarget(target)"
                      >
                        <i class="fas fa-trash"></i>
                      </button>
                    </div>
                  </div>

                  <div class="mt-2 flex flex-wrap items-center gap-3 text-xs text-base-content/60">
                    <span>{{ t('bugBounty.apiInventory.endpoints') }}: {{ target.endpoint_count }}</span>
                    <span v-if="target.added_endpoints_count > 0" class="text-success">
                      +{{ target.added_endpoints_count }}
                    </span>
                    <span v-if="target.removed_endpoints_count > 0" class="text-error">
                      -{{ target.removed_endpoints_count }}
                    </span>
                    <span v-if="target.error_message" class="text-warning">
                      {{ t('bugBounty.apiInventory.withErrors') }}
                    </span>
                  </div>

                  <div class="mt-2 text-xs text-base-content/50">
                    {{ formatDateTime(target.observed_at) }}
                  </div>
                </div>
              </div>
            </div>

            <div v-if="loadingMore" class="flex justify-center py-4">
              <span class="loading loading-spinner loading-md"></span>
            </div>

            <div v-else-if="hasMoreTargets" class="px-4 py-3 text-center text-xs text-base-content/50">
              {{ t('common.loadMore') }}
            </div>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <div v-if="loadingDetail" class="flex justify-center py-16">
            <span class="loading loading-spinner loading-lg"></span>
          </div>

          <div v-else-if="!selectedDetail" class="flex flex-col items-center justify-center py-16 text-base-content/60">
            <i class="fas fa-diagram-project text-5xl opacity-30"></i>
            <p class="mt-3">{{ t('bugBounty.apiInventory.selectTarget') }}</p>
          </div>

          <template v-else>
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <h3 class="truncate text-lg font-semibold">{{ selectedDetail.base_url }}</h3>
                <div class="mt-2 flex flex-wrap gap-2 text-xs text-base-content/60">
                  <span class="badge badge-outline badge-sm">
                    {{ programNameForId(selectedDetail.program_id) }}
                  </span>
                  <span v-if="selectedDetail.task_name" class="badge badge-ghost badge-sm">
                    {{ selectedDetail.task_name }}
                  </span>
                  <span v-if="selectedDetail.execution_mode" class="badge badge-outline badge-sm">
                    {{ selectedDetail.execution_mode }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ t('bugBounty.apiInventory.endpoints') }}: {{ selectedDetail.endpoint_count }}
                  </span>
                </div>
              </div>
              <div class="text-right text-xs text-base-content/60">
                <div>{{ formatDateTime(selectedDetail.observed_at) }}</div>
                <div v-if="selectedDetail.last_checked">{{ selectedDetail.last_checked }}</div>
              </div>
            </div>

            <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
              <div class="rounded-lg bg-base-200 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.targetStatus') }}</div>
                <div class="mt-1">
                  <span class="badge badge-sm" :class="selectedDetail.success ? 'badge-success' : 'badge-error'">
                    {{ selectedDetail.success ? t('common.success') : t('common.failed') }}
                  </span>
                </div>
              </div>
              <div class="rounded-lg bg-base-200 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.changes') }}</div>
                <div class="mt-1 flex items-center gap-3 text-sm font-medium">
                  <span class="text-success">+{{ selectedDetail.added_endpoints.length }}</span>
                  <span class="text-error">-{{ selectedDetail.removed_endpoints.length }}</span>
                </div>
              </div>
            </div>

            <div v-if="selectedDetail.error_message" class="alert alert-warning mt-4">
              <i class="fas fa-triangle-exclamation"></i>
              <span>{{ selectedDetail.error_message }}</span>
            </div>

            <div class="tabs tabs-boxed mt-4">
              <a class="tab" :class="{ 'tab-active': endpointTab === 'all' }" @click="endpointTab = 'all'">
                {{ t('bugBounty.apiInventory.allEndpoints') }}
              </a>
              <a class="tab" :class="{ 'tab-active': endpointTab === 'added' }" @click="endpointTab = 'added'">
                {{ t('bugBounty.apiInventory.addedEndpoints') }}
              </a>
              <a class="tab" :class="{ 'tab-active': endpointTab === 'removed' }" @click="endpointTab = 'removed'">
                {{ t('bugBounty.apiInventory.removedEndpoints') }}
              </a>
            </div>

            <div class="mt-4 overflow-hidden rounded-lg border border-base-200">
              <div class="max-h-[42vh] overflow-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>{{ t('bugBounty.apiInventory.path') }}</th>
                      <th>{{ t('bugBounty.apiInventory.source') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="endpoint in visibleEndpoints"
                      :key="endpointKeyOf(endpoint)"
                      class="cursor-pointer"
                      :class="selectedEndpointKey === endpointKeyOf(endpoint) ? 'bg-primary/10' : ''"
                      @click="selectEndpoint(endpoint)"
                    >
                      <td class="font-mono text-xs">{{ endpoint.path }}</td>
                      <td class="text-xs text-base-content/60">{{ endpoint.source || '-' }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-if="visibleEndpoints.length === 0" class="px-4 py-10 text-center text-sm text-base-content/60">
                {{ t('bugBounty.apiInventory.noEndpointsInTab') }}
              </div>
            </div>

            <div v-if="selectedEndpoint" class="mt-4 rounded-lg border border-base-200 bg-base-200/30 p-4">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="font-medium">{{ selectedEndpoint.path }}</div>
                </div>
              </div>

              <div class="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
                <div class="rounded-lg bg-base-200 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.requestUrl') }}</div>
                  <div class="mt-1 break-all font-mono text-xs">
                    {{ resolveEndpointUrl(selectedDetail.base_url, selectedEndpoint.path) }}
                  </div>
                </div>
                <div class="rounded-lg bg-base-200 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.source') }}</div>
                  <div class="mt-1 text-sm font-medium">
                    {{ selectedEndpoint.source || '-' }}
                  </div>
                </div>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'
import { dialog } from '../../composables/useDialog'

interface ApiInventoryEndpoint {
  path: string
  source?: string | null
}

type RawApiInventoryEndpoint = ApiInventoryEndpoint

interface ApiInventoryTargetSummary {
  program_id: string
  base_url: string
  success: boolean
  endpoint_count: number
  last_checked?: string | null
  observed_at: string
  run_id: string
  task_name?: string | null
  execution_mode?: string | null
  added_endpoints_count: number
  removed_endpoints_count: number
  sample_endpoints: string[]
  error_message?: string | null
}

interface ApiInventoryTargetDetail {
  program_id: string
  base_url: string
  success: boolean
  endpoint_count: number
  last_checked?: string | null
  observed_at: string
  run_id: string
  task_name?: string | null
  execution_mode?: string | null
  endpoints: ApiInventoryEndpoint[]
  added_endpoints: ApiInventoryEndpoint[]
  removed_endpoints: ApiInventoryEndpoint[]
  error_message?: string | null
}

interface RawApiInventoryTargetDetail extends Omit<ApiInventoryTargetDetail, 'endpoints' | 'added_endpoints' | 'removed_endpoints'> {
  endpoints: RawApiInventoryEndpoint[]
  added_endpoints: RawApiInventoryEndpoint[]
  removed_endpoints: RawApiInventoryEndpoint[]
}

interface ApiInventoryDeleteTarget {
  program_id: string
  base_url: string
}

interface RawApiInventoryDeleteTarget {
  program_id?: string
  base_url?: string
  programId?: string
  baseUrl?: string
}

interface ApiInventoryListStats {
  total_endpoints: number
  successful_targets: number
  failed_targets: number
  changed_targets: number
}

interface ApiInventoryListResponse {
  items: ApiInventoryTargetSummary[]
  total: number
  filtered_total: number
  has_more: boolean
  stats: ApiInventoryListStats
}

type StatusFilter = 'all' | 'success' | 'failed'
type ExecutionModeFilter = 'all' | 'scheduler' | 'manual'
type CapabilityFilter = 'all' | 'changed' | 'errors'
type EndpointTab = 'all' | 'added' | 'removed'
type SortBy = 'observed_desc' | 'endpoint_desc' | 'changes_desc' | 'base_url_asc'

const TARGET_PAGE_SIZE = 50

const props = defineProps<{
  selectedProgram?: any
  programs?: any[]
}>()

const { t } = useI18n()
const toast = useToast()

const loading = ref(false)
const loadingMore = ref(false)
const loadingDetail = ref(false)
const search = ref('')
const statusFilter = ref<StatusFilter>('all')
const executionModeFilter = ref<ExecutionModeFilter>('all')
const capabilityFilter = ref<CapabilityFilter>('all')
const sortBy = ref<SortBy>('observed_desc')
const programFilter = ref('')
const programFilterTouched = ref(false)
const advancedFiltersExpanded = ref(false)
const deleting = ref(false)
const targets = ref<ApiInventoryTargetSummary[]>([])
const totalTargetCount = ref(0)
const filteredTargetCount = ref(0)
const hasMoreTargets = ref(false)
const selectedTargetEntries = ref<ApiInventoryDeleteTarget[]>([])
const targetStats = ref<ApiInventoryListStats>({
  total_endpoints: 0,
  successful_targets: 0,
  failed_targets: 0,
  changed_targets: 0,
})
const selectedTargetKey = ref('')
const selectedDetail = ref<ApiInventoryTargetDetail | null>(null)
const endpointTab = ref<EndpointTab>('all')
const selectedEndpointKey = ref('')

const propSelectedProgramId = computed(() => String(props.selectedProgram?.id || ''))
const programOptions = computed(() => Array.isArray(props.programs) ? props.programs : [])
const programNameById = computed(() => new Map(
  programOptions.value.map(program => [String(program.id), String(program.name || program.id)]),
))

const targetKeyOf = (target: Pick<ApiInventoryTargetSummary, 'program_id' | 'base_url'>) =>
  `${target.program_id}::${target.base_url}`

const normalizeDeleteTarget = (target: RawApiInventoryDeleteTarget): ApiInventoryDeleteTarget | null => {
  const program_id = String(target.program_id || target.programId || '').trim()
  const base_url = String(target.base_url || target.baseUrl || '').trim()
  if (!program_id || !base_url) return null
  return { program_id, base_url }
}

const endpointKeyOf = (endpoint: ApiInventoryEndpoint) => `${endpoint.path}::${endpoint.source || ''}`

const normalizeEndpoint = (endpoint: RawApiInventoryEndpoint): ApiInventoryEndpoint => ({
  path: endpoint.path,
  source: endpoint.source ?? null,
})

const normalizeTargetDetail = (detail: RawApiInventoryTargetDetail | null): ApiInventoryTargetDetail | null => {
  if (!detail) return null
  return {
    ...detail,
    endpoints: Array.isArray(detail.endpoints) ? detail.endpoints.map(normalizeEndpoint) : [],
    added_endpoints: Array.isArray(detail.added_endpoints) ? detail.added_endpoints.map(normalizeEndpoint) : [],
    removed_endpoints: Array.isArray(detail.removed_endpoints) ? detail.removed_endpoints.map(normalizeEndpoint) : [],
  }
}

const programNameForId = (programId: string) => (
  programNameById.value.get(String(programId)) || String(programId || '-')
)

const currentProgramLabel = computed(() => (
  programFilter.value
    ? programNameForId(programFilter.value)
    : t('bugBounty.apiInventory.allPrograms')
))

const hasAdvancedFilters = computed(() => (
  executionModeFilter.value !== 'all'
  || capabilityFilter.value !== 'all'
))

const activeAdvancedFilterCount = computed(() => (
  Number(executionModeFilter.value !== 'all')
  + Number(capabilityFilter.value !== 'all')
))

const selectedTargetKeys = computed(() => (
  selectedTargetEntries.value.map(target => targetKeyOf(target))
))

const selectedTargetSet = computed(() => new Set(selectedTargetKeys.value))

const allFilteredSelected = computed(() => (
  filteredTargetCount.value > 0
  && selectedTargetEntries.value.length >= filteredTargetCount.value
))

const totalEndpoints = computed(() =>
  targetStats.value.total_endpoints,
)

const successfulTargets = computed(() =>
  targetStats.value.successful_targets,
)

const failedTargets = computed(() =>
  targetStats.value.failed_targets,
)

const changedTargets = computed(() =>
  targetStats.value.changed_targets,
)

const visibleEndpoints = computed(() => {
  if (!selectedDetail.value) return []
  switch (endpointTab.value) {
    case 'added':
      return selectedDetail.value.added_endpoints
    case 'removed':
      return selectedDetail.value.removed_endpoints
    default:
      return selectedDetail.value.endpoints
  }
})

const selectedEndpoint = computed(() => {
  if (visibleEndpoints.value.length === 0) return null
  return (
    visibleEndpoints.value.find(endpoint => endpointKeyOf(endpoint) === selectedEndpointKey.value)
    || visibleEndpoints.value[0]
  )
})

const buildFilterPayload = (offset = 0, limit = TARGET_PAGE_SIZE) => ({
  programId: programFilter.value || null,
  search: search.value.trim() || null,
  status: statusFilter.value,
  executionMode: executionModeFilter.value,
  capability: capabilityFilter.value,
  sortBy: sortBy.value,
  offset,
  limit,
})

const syncSelectedTargetWithFilters = async () => {
  const selectedVisible = targets.value.find(target => targetKeyOf(target) === selectedTargetKey.value) || null
  if (selectedVisible) return

  const nextTarget = targets.value[0] || null
  selectedTargetKey.value = nextTarget ? targetKeyOf(nextTarget) : ''
  if (nextTarget) {
    await loadTargetDetail(nextTarget)
  } else {
    selectedDetail.value = null
    selectedEndpointKey.value = ''
  }
}

const loadTargets = async (reset = true) => {
  const offset = reset ? 0 : targets.value.length
  try {
    if (reset) {
      loading.value = true
    } else {
      loadingMore.value = true
    }

    const response = await invoke<ApiInventoryListResponse>('bounty_list_api_inventory_targets', {
      filter: buildFilterPayload(offset),
    })
    targets.value = reset
      ? (Array.isArray(response?.items) ? response.items : [])
      : [...targets.value, ...(Array.isArray(response?.items) ? response.items : [])]
    totalTargetCount.value = Number(response?.total || 0)
    filteredTargetCount.value = Number(response?.filtered_total || 0)
    hasMoreTargets.value = Boolean(response?.has_more)
    targetStats.value = response?.stats || {
      total_endpoints: 0,
      successful_targets: 0,
      failed_targets: 0,
      changed_targets: 0,
    }
    await syncSelectedTargetWithFilters()
  } catch (error) {
    console.error('Failed to load API inventory targets:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    if (reset) {
      targets.value = []
      totalTargetCount.value = 0
      filteredTargetCount.value = 0
      hasMoreTargets.value = false
      targetStats.value = {
        total_endpoints: 0,
        successful_targets: 0,
        failed_targets: 0,
        changed_targets: 0,
      }
      selectedDetail.value = null
      selectedEndpointKey.value = ''
    }
  } finally {
    if (reset) {
      loading.value = false
    } else {
      loadingMore.value = false
    }
  }
}

const loadMoreTargets = async () => {
  if (loading.value || loadingMore.value || !hasMoreTargets.value) return
  await loadTargets(false)
}

const loadTargetDetail = async (target: ApiInventoryTargetSummary | null) => {
  if (!target?.base_url) {
    selectedDetail.value = null
    selectedEndpointKey.value = ''
    return
  }

  try {
    loadingDetail.value = true
    const detail = await invoke<RawApiInventoryTargetDetail | null>('bounty_get_api_inventory_target', {
      programId: target.program_id,
      baseUrl: target.base_url,
    })
    selectedDetail.value = normalizeTargetDetail(detail)
    endpointTab.value = 'all'
  } catch (error) {
    console.error('Failed to load API inventory detail:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    selectedDetail.value = null
    selectedEndpointKey.value = ''
  } finally {
    loadingDetail.value = false
  }
}

const selectTarget = async (target: ApiInventoryTargetSummary) => {
  selectedTargetKey.value = targetKeyOf(target)
  await loadTargetDetail(target)
}

const selectEndpoint = (endpoint: ApiInventoryEndpoint) => {
  selectedEndpointKey.value = endpointKeyOf(endpoint)
}

const isTargetSelected = (target: ApiInventoryTargetSummary) => (
  selectedTargetSet.value.has(targetKeyOf(target))
)

const toggleTargetSelection = (target: ApiInventoryTargetSummary) => {
  const key = targetKeyOf(target)
  const next = new Map(selectedTargetEntries.value.map(item => [targetKeyOf(item), item]))
  if (next.has(key)) {
    next.delete(key)
  } else {
    next.set(key, {
      program_id: target.program_id,
      base_url: target.base_url,
    })
  }
  selectedTargetEntries.value = [...next.values()]
}

const selectAllFilteredTargets = async () => {
  try {
    const rows = await invoke<RawApiInventoryDeleteTarget[]>('bounty_list_api_inventory_target_keys', {
      filter: buildFilterPayload(0, TARGET_PAGE_SIZE),
    })
    selectedTargetEntries.value = Array.isArray(rows)
      ? rows
        .map(normalizeDeleteTarget)
        .filter((target): target is ApiInventoryDeleteTarget => Boolean(target))
      : []
  } catch (error) {
    console.error('Failed to select filtered API inventory targets:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  }
}

const clearSelection = () => {
  selectedTargetEntries.value = []
}

const deleteTargets = async (targetRows: ApiInventoryDeleteTarget[]) => {
  if (!targetRows.length) return 0

  try {
    deleting.value = true
    const deleted = await invoke<number>('bounty_batch_delete_api_inventory_targets', {
      targets: targetRows.map(target => ({
        programId: target.program_id,
        baseUrl: target.base_url,
      })),
    })
    toast.success(t('bugBounty.batch.deleteSuccess', { count: deleted }))
    const deletedKeys = new Set(targetRows.map(target => targetKeyOf(target)))
    selectedTargetEntries.value = selectedTargetEntries.value.filter(target => !deletedKeys.has(targetKeyOf(target)))
    await loadTargets(true)
    return deleted
  } catch (error) {
    console.error('Failed to delete API inventory targets:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
    return 0
  } finally {
    deleting.value = false
  }
}

const deleteSingleTarget = async (target: ApiInventoryTargetSummary) => {
  if (!(await dialog.confirm(t('bugBounty.apiInventory.confirmDeleteTarget', { target: target.base_url })))) {
    return
  }
  await deleteTargets([{
    program_id: target.program_id,
    base_url: target.base_url,
  }])
}

const batchDeleteTargets = async () => {
  if (!selectedTargetEntries.value.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDelete', { count: selectedTargetEntries.value.length })))) {
    return
  }
  await deleteTargets(selectedTargetEntries.value)
}

const handleTargetListScroll = async (event: Event) => {
  const element = event.target as HTMLElement | null
  if (!element) return
  const threshold = 160
  const distanceToBottom = element.scrollHeight - element.scrollTop - element.clientHeight
  if (distanceToBottom <= threshold) {
    await loadMoreTargets()
  }
}

const handleProgramFilterChange = () => {
  programFilterTouched.value = true
}

const clearFilters = () => {
  search.value = ''
  statusFilter.value = 'all'
  executionModeFilter.value = 'all'
  capabilityFilter.value = 'all'
  sortBy.value = 'observed_desc'
  programFilterTouched.value = false
  programFilter.value = propSelectedProgramId.value
  advancedFiltersExpanded.value = false
}

const formatDateTime = (value?: string | null) => {
  if (!value) return '-'
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleString()
}

const resolveEndpointUrl = (baseUrl: string, path: string) => {
  try {
    return new URL(path, baseUrl).toString()
  } catch {
    return path
  }
}

watch(propSelectedProgramId, next => {
  programFilterTouched.value = false
  programFilter.value = next
}, { immediate: true })

watch(
  () => targets.value.map(target => targetKeyOf(target)).join('|'),
  async () => {
    await syncSelectedTargetWithFilters()
  },
)

watch(
  () => visibleEndpoints.value.map(endpoint => endpointKeyOf(endpoint)).join('|'),
  () => {
    const activeKey = selectedEndpoint.value ? endpointKeyOf(selectedEndpoint.value) : ''
    selectedEndpointKey.value = activeKey
  },
  { immediate: true },
)

watch(endpointTab, () => {
  selectedEndpointKey.value = ''
})

watch(
  () => [
    programFilter.value,
    search.value,
    statusFilter.value,
    executionModeFilter.value,
    capabilityFilter.value,
    sortBy.value,
  ].join('|'),
  async () => {
    clearSelection()
    await loadTargets(true)
  },
)

watch(hasAdvancedFilters, next => {
  if (next) {
    advancedFiltersExpanded.value = true
  }
})

watch(
  () => selectedDetail.value?.run_id || '',
  () => {
    selectedEndpointKey.value = ''
  },
)

onMounted(async () => {
  await loadTargets(true)
})
</script>
