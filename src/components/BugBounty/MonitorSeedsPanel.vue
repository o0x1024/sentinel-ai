<template>
  <Teleport to="body">
    <div v-if="open" class="modal modal-open">
      <div class="modal-box max-w-6xl max-h-[90vh] overflow-y-auto overflow-x-hidden">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h3 class="font-bold text-lg">{{ t('bugBounty.monitor.seeds.title') }}</h3>
            <p class="mt-1 text-sm text-base-content/60">
              {{ t('bugBounty.monitor.seeds.description') }}
            </p>
          </div>
          <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="emit('close')">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="mt-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <select
            v-if="!props.selectedProgram?.id"
            v-model="localProgramId"
            class="select select-bordered select-sm w-56"
          >
            <option value="">{{ t('bugBounty.program.selectProgramPlaceholder') }}</option>
            <option v-for="program in props.programs || []" :key="program.id" :value="program.id">
              {{ program.name }}
            </option>
          </select>
          <button
            type="button"
            class="btn btn-sm btn-outline"
            :disabled="!resolvedProgramId || syncing"
            @click="syncSeedsFromAssets"
          >
            <span v-if="syncing" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-rotate"></i>
            {{ t('bugBounty.monitor.seeds.syncFromAssets') }}
          </button>
          <button
            type="button"
            class="btn btn-sm btn-primary"
            :disabled="!resolvedProgramId"
            @click="openCreateModal"
          >
            <i class="fas fa-plus mr-1"></i>
            {{ t('bugBounty.monitor.seeds.addSeed') }}
          </button>
        </div>
      </div>

      <div v-if="resolvedProgramLabel" class="mt-3 text-xs text-base-content/60">
        <i class="fas fa-folder-open mr-1"></i>
        {{ resolvedProgramLabel }}
      </div>

      <div v-if="!resolvedProgramId" class="alert mt-4">
        <i class="fas fa-circle-info"></i>
        <span>{{ t('bugBounty.monitor.seeds.selectProgramFirst') }}</span>
      </div>

      <div v-else-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
      </div>

      <div v-else class="mt-4 space-y-3">
        <div class="rounded-lg border border-base-300 bg-base-100 p-3">
          <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_180px_180px_180px_auto]">
            <label class="input input-bordered input-sm flex items-center gap-2">
              <i class="fas fa-search text-base-content/50"></i>
              <input
                v-model.trim="searchQuery"
                type="text"
                class="grow"
                :placeholder="t('bugBounty.monitor.seeds.searchPlaceholder')"
              />
            </label>
            <select v-model="typeFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allTypes') }}</option>
              <option v-for="option in seedTypeOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
            <select v-model="statusFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allStatuses') }}</option>
              <option value="active">{{ t('bugBounty.monitor.seeds.active') }}</option>
              <option value="disabled">{{ t('bugBounty.monitor.seeds.disabled') }}</option>
            </select>
            <select v-model="sourceFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allSources') }}</option>
              <option v-for="item in sourceDistribution" :key="item.key" :value="item.key">
                {{ item.label }}
              </option>
            </select>
            <button type="button" class="btn btn-sm btn-ghost" @click="resetFilters">
              {{ t('bugBounty.monitor.seeds.clearFilters') }}
            </button>
          </div>

          <div v-if="hasDistribution" class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
            <div class="flex flex-wrap items-center gap-2 text-xs">
              <span class="text-base-content/50">{{ t('bugBounty.monitor.seeds.typeDistribution') }}</span>
              <button
                v-for="item in typeDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="typeFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="typeFilter = typeFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
              <span class="ml-2 text-base-content/50">{{ t('bugBounty.monitor.seeds.statusDistribution') }}</span>
              <button
                v-for="item in statusDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="statusFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="statusFilter = statusFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
              <span class="ml-2 text-base-content/50">{{ t('bugBounty.monitor.seeds.sourceDistribution') }}</span>
              <button
                v-for="item in sourceDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="sourceFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="sourceFilter = sourceFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
            </div>
          </div>
        </div>

        <div v-if="totalSeeds === 0 && !hasActiveFilters" class="text-center py-8 text-base-content/60">
          <i class="fas fa-seedling text-3xl mb-3"></i>
          <p>{{ t('bugBounty.monitor.seeds.empty') }}</p>
        </div>

        <div v-else-if="totalSeeds === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-filter text-3xl mb-3"></i>
          <p>{{ t('bugBounty.monitor.seeds.emptyFiltered') }}</p>
        </div>

        <div v-else class="overflow-x-auto rounded-lg border border-base-300">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>{{ t('bugBounty.monitor.seeds.type') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.value') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.status') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.source') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.confidence') }}</th>
                <th class="w-28 text-right">{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="seed in seeds" :key="seed.id">
                <td>
                  <span class="font-medium">{{ seedTypeLabel(seed.seed_type) }}</span>
                </td>
                <td class="max-w-[360px]">
                  <div class="truncate font-mono text-xs" :title="seed.seed_value">{{ seed.seed_value }}</div>
                </td>
                <td>
                  <span class="badge badge-sm" :class="seed.status === 'active' ? 'badge-success badge-outline' : 'badge-ghost'">
                    {{ seed.status === 'active' ? t('bugBounty.monitor.seeds.active') : t('bugBounty.monitor.seeds.disabled') }}
                  </span>
                </td>
                <td>{{ sourceLabel(seed.source) }}</td>
                <td>{{ formatConfidence(seed.confidence_score) }}</td>
                <td>
                  <div class="flex justify-end gap-1">
                    <button type="button" class="btn btn-ghost btn-xs" @click="openEditModal(seed)">
                      <i class="fas fa-pen"></i>
                    </button>
                    <button type="button" class="btn btn-ghost btn-xs text-error" @click="openDeleteModal(seed)">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div
          v-if="totalSeeds > 0"
          class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-3 xl:flex-row xl:items-center xl:justify-between"
        >
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{ t('bugBounty.surface.inventory.pageSizeLabel') }}</span>
            <select v-model.number="pageSize" class="select select-bordered select-sm">
              <option v-for="size in pageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button class="join-item btn btn-sm" :disabled="currentPage <= 1" @click="goToFirstPage">
                {{ t('bugBounty.surface.inventory.firstPage') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage <= 1" @click="goToPreviousPage">
                {{ t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{ t('bugBounty.surface.inventory.pageInfo', { page: currentPage, total: pageCount }) }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount" @click="goToNextPage">
                {{ t('common.next') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount" @click="goToLastPage">
                {{ t('bugBounty.surface.inventory.lastPage') }}
              </button>
            </div>

            <div class="flex items-center gap-2">
              <input
                v-model="pageInput"
                type="number"
                min="1"
                :max="pageCount"
                class="input input-bordered input-sm w-24"
                :placeholder="t('bugBounty.surface.inventory.jumpPlaceholder')"
                @keyup.enter="applyPageJump"
              />
              <button class="btn btn-sm btn-outline" @click="applyPageJump">
                {{ t('bugBounty.surface.inventory.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>
        </div>
      </div>
      <div class="modal-backdrop" @click="emit('close')"></div>
    </div>

    <div v-if="open && showEditorModal" class="modal modal-open">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-4">
          {{ editingSeed ? t('bugBounty.monitor.seeds.editSeed') : t('bugBounty.monitor.seeds.addSeed') }}
        </h3>
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.type') }}</span>
            </label>
            <select v-model="seedForm.seed_type" class="select select-bordered">
              <option v-for="option in seedTypeOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.value') }}</span>
            </label>
            <input
              v-model="seedForm.seed_value"
              type="text"
              class="input input-bordered"
              :placeholder="t('bugBounty.monitor.seeds.valuePlaceholder')"
            />
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.status') }}</span>
            </label>
            <select v-model="seedForm.status" class="select select-bordered">
              <option value="active">{{ t('bugBounty.monitor.seeds.active') }}</option>
              <option value="disabled">{{ t('bugBounty.monitor.seeds.disabled') }}</option>
            </select>
          </div>
        </div>
        <div class="modal-action">
          <button type="button" class="btn" @click="closeEditorModal">
            {{ t('common.cancel') }}
          </button>
          <button type="button" class="btn btn-primary" :disabled="saving" @click="saveSeed">
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            {{ t('common.save') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeEditorModal"></div>
    </div>

    <div v-if="open && seedPendingDelete" class="modal modal-open">
      <div class="modal-box max-w-md">
        <h3 class="font-bold text-lg">{{ t('bugBounty.monitor.seeds.deleteTitle') }}</h3>
        <p class="py-4 text-sm text-base-content/70">
          {{ t('bugBounty.monitor.seeds.deleteConfirm') }}
        </p>
        <div class="rounded-md bg-base-200 px-3 py-2 font-mono text-xs">
          {{ seedPendingDelete.seed_value }}
        </div>
        <div class="modal-action">
          <button type="button" class="btn" @click="seedPendingDelete = null">
            {{ t('common.cancel') }}
          </button>
          <button type="button" class="btn btn-error" :disabled="deleting" @click="deleteSeed">
            <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
            {{ t('common.delete') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="seedPendingDelete = null"></div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'

defineOptions({ name: 'MonitorSeedsPanel' })

const props = defineProps<{
  open: boolean
  selectedProgram?: any
  programs?: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

interface SurfaceSeedRow {
  id: string
  program_id: string
  seed_type: string
  seed_value: string
  status: string
  source?: string | null
  confidence_score?: number | null
  last_run_at?: string | null
  metadata_json?: string | null
  created_at: string
  updated_at: string
}

interface SeedSyncResult {
  requested: number
  created: number
  reactivated: number
  disabled: number
}

interface DistributionBucket {
  key: string
  count: number
}

interface SurfaceSeedDistribution {
  types: DistributionBucket[]
  statuses: DistributionBucket[]
  sources: DistributionBucket[]
}

interface SurfaceSeedQueryResult {
  items: SurfaceSeedRow[]
  total: number
}

const { t } = useI18n()
const toast = useToast()

const loading = ref(false)
const saving = ref(false)
const deleting = ref(false)
const syncing = ref(false)
const seeds = ref<SurfaceSeedRow[]>([])
const totalSeeds = ref(0)
const distribution = ref<SurfaceSeedDistribution>({
  types: [],
  statuses: [],
  sources: [],
})
const localProgramId = ref('')
const showEditorModal = ref(false)
const editingSeed = ref<SurfaceSeedRow | null>(null)
const seedPendingDelete = ref<SurfaceSeedRow | null>(null)
const searchQuery = ref('')
const typeFilter = ref('all')
const statusFilter = ref('all')
const sourceFilter = ref('all')
const currentPage = ref(1)
const pageSize = ref(20)
const pageInput = ref('')
const pageSizeOptions = [10, 20, 50, 100]
const seedForm = reactive({
  id: '',
  seed_type: 'root_domain',
  seed_value: '',
  status: 'active',
})

const resolvedProgramId = computed(() => String(props.selectedProgram?.id || localProgramId.value || ''))
const resolvedProgramLabel = computed(() => {
  const programId = resolvedProgramId.value
  if (!programId) return ''
  const selected = (props.programs || []).find(program => program.id === programId)
  return selected?.name || props.selectedProgram?.name || ''
})

const seedTypeOptions = computed(() => [
  { value: 'root_domain', label: t('bugBounty.monitor.seeds.types.root_domain') },
  { value: 'favicon_hash', label: t('bugBounty.monitor.seeds.types.favicon_hash') },
  { value: 'brand_keyword', label: t('bugBounty.monitor.seeds.types.brand_keyword') },
  { value: 'domain', label: t('bugBounty.monitor.seeds.types.domain') },
  { value: 'org_name', label: t('bugBounty.monitor.seeds.types.org_name') },
  { value: 'asn', label: t('bugBounty.monitor.seeds.types.asn') },
  { value: 'cname_keyword', label: t('bugBounty.monitor.seeds.types.cname_keyword') },
  { value: 'title_keyword', label: t('bugBounty.monitor.seeds.types.title_keyword') },
  { value: 'body_keyword', label: t('bugBounty.monitor.seeds.types.body_keyword') },
  { value: 'header_keyword', label: t('bugBounty.monitor.seeds.types.header_keyword') },
])
const typeDistribution = computed(() => distribution.value.types.map(item => ({
  key: item.key,
  label: seedTypeLabel(item.key),
  count: item.count,
})))
const statusDistribution = computed(() => distribution.value.statuses.map(item => ({
  key: item.key,
  label: item.key === 'active' ? t('bugBounty.monitor.seeds.active') : t('bugBounty.monitor.seeds.disabled'),
  count: item.count,
})))
const sourceDistribution = computed(() => distribution.value.sources.map(item => ({
  key: item.key,
  label: sourceLabel(item.key),
  count: item.count,
})))
const hasDistribution = computed(() => (
  typeDistribution.value.length > 0
  || statusDistribution.value.length > 0
  || sourceDistribution.value.length > 0
))
const hasActiveFilters = computed(() => (
  searchQuery.value.trim().length > 0
  || typeFilter.value !== 'all'
  || statusFilter.value !== 'all'
  || sourceFilter.value !== 'all'
))
const pageCount = computed(() => Math.max(1, Math.ceil(totalSeeds.value / pageSize.value)))

const loadSeeds = async () => {
  if (!resolvedProgramId.value) {
    seeds.value = []
    totalSeeds.value = 0
    return
  }
  try {
    loading.value = true
    const result = await invoke<SurfaceSeedQueryResult>('surface_query_seeds', {
      request: {
        program_id: resolvedProgramId.value,
        search: searchQuery.value || null,
        seed_type: typeFilter.value === 'all' ? null : typeFilter.value,
        status: statusFilter.value === 'all' ? null : statusFilter.value,
        source: sourceFilter.value === 'all' ? null : sourceFilter.value,
        limit: pageSize.value,
        offset: (currentPage.value - 1) * pageSize.value,
      },
    })
    seeds.value = result.items
    totalSeeds.value = result.total
  } catch (error) {
    console.error('Failed to load surface seeds:', error)
    seeds.value = []
    totalSeeds.value = 0
    toast.error(t('bugBounty.monitor.seeds.loadFailed'))
  } finally {
    loading.value = false
  }
}

const loadDistribution = async () => {
  if (!resolvedProgramId.value) {
    distribution.value = { types: [], statuses: [], sources: [] }
    return
  }
  try {
    distribution.value = await invoke<SurfaceSeedDistribution>('surface_get_seed_distribution', {
      programId: resolvedProgramId.value,
      search: searchQuery.value || null,
      seedType: typeFilter.value === 'all' ? null : typeFilter.value,
      status: statusFilter.value === 'all' ? null : statusFilter.value,
      source: sourceFilter.value === 'all' ? null : sourceFilter.value,
    })
  } catch (error) {
    console.error('Failed to load surface seed distribution:', error)
    distribution.value = { types: [], statuses: [], sources: [] }
  }
}

const resetSeedForm = () => {
  seedForm.id = ''
  seedForm.seed_type = 'root_domain'
  seedForm.seed_value = ''
  seedForm.status = 'active'
}

const resetFilters = () => {
  searchQuery.value = ''
  typeFilter.value = 'all'
  statusFilter.value = 'all'
  sourceFilter.value = 'all'
  currentPage.value = 1
  pageInput.value = ''
}

const openCreateModal = () => {
  editingSeed.value = null
  resetSeedForm()
  showEditorModal.value = true
}

const openEditModal = (seed: SurfaceSeedRow) => {
  editingSeed.value = seed
  seedForm.id = seed.id
  seedForm.seed_type = seed.seed_type
  seedForm.seed_value = seed.seed_value
  seedForm.status = seed.status || 'active'
  showEditorModal.value = true
}

const closeEditorModal = () => {
  showEditorModal.value = false
  editingSeed.value = null
  resetSeedForm()
}

const saveSeed = async () => {
  if (!resolvedProgramId.value) return
  try {
    saving.value = true
    await invoke<SurfaceSeedRow>('surface_upsert_seed', {
      request: {
        id: seedForm.id || null,
        program_id: resolvedProgramId.value,
        seed_type: seedForm.seed_type,
        seed_value: seedForm.seed_value,
        status: seedForm.status,
        source: 'manual',
        confidence_score: 1.0,
        metadata: null,
      },
    })
    toast.success(editingSeed.value ? t('bugBounty.monitor.seeds.updated') : t('bugBounty.monitor.seeds.created'))
    closeEditorModal()
    await loadSeeds()
    await loadDistribution()
  } catch (error) {
    console.error('Failed to save surface seed:', error)
    toast.error(t('bugBounty.monitor.seeds.saveFailed'))
  } finally {
    saving.value = false
  }
}

const openDeleteModal = (seed: SurfaceSeedRow) => {
  seedPendingDelete.value = seed
}

const deleteSeed = async () => {
  if (!seedPendingDelete.value) return
  try {
    deleting.value = true
    await invoke('surface_delete_seed', { seedId: seedPendingDelete.value.id })
    seedPendingDelete.value = null
    toast.success(t('bugBounty.monitor.seeds.deleted'))
    await loadSeeds()
    await loadDistribution()
  } catch (error) {
    console.error('Failed to delete surface seed:', error)
    toast.error(t('bugBounty.monitor.seeds.deleteFailed'))
  } finally {
    deleting.value = false
  }
}

const syncSeedsFromAssets = async () => {
  if (!resolvedProgramId.value) return
  try {
    syncing.value = true
    const result = await invoke<SeedSyncResult>('surface_sync_program_seeds', {
      programId: resolvedProgramId.value,
    })
    toast.success(
      t('bugBounty.monitor.seeds.syncedSummary', {
        created: result.created,
        reactivated: result.reactivated,
        disabled: result.disabled,
      })
    )
    await loadSeeds()
    await loadDistribution()
  } catch (error) {
    console.error('Failed to sync surface seeds:', error)
    toast.error(t('bugBounty.monitor.seeds.syncFailed'))
  } finally {
    syncing.value = false
  }
}

const seedTypeLabel = (value: string) => {
  const match = seedTypeOptions.value.find(option => option.value === value)
  return match?.label || value
}

const sourceLabel = (value?: string | null) => {
  if (!value) return '-'
  if (value === 'surface_asset_sync') {
    return t('bugBounty.monitor.seeds.sourceSynced')
  }
  if (value === 'manual') {
    return t('bugBounty.monitor.seeds.sourceManual')
  }
  return value
}

const goToFirstPage = () => {
  currentPage.value = 1
}

const goToPreviousPage = () => {
  currentPage.value = Math.max(1, currentPage.value - 1)
}

const goToNextPage = () => {
  currentPage.value = Math.min(pageCount.value, currentPage.value + 1)
}

const goToLastPage = () => {
  currentPage.value = pageCount.value
}

const applyPageJump = () => {
  const parsed = Number.parseInt(pageInput.value, 10)
  if (!Number.isFinite(parsed)) return
  currentPage.value = Math.min(pageCount.value, Math.max(1, parsed))
  pageInput.value = ''
}

const formatConfidence = (value?: number | null) => {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-'
  return value.toFixed(2)
}

watch(
  () => props.selectedProgram?.id,
  value => {
    localProgramId.value = String(value || '')
  },
  { immediate: true },
)

watch(resolvedProgramId, () => {
  loadSeeds()
  loadDistribution()
}, { immediate: true })

watch([searchQuery, typeFilter, statusFilter, sourceFilter], () => {
  currentPage.value = 1
  pageInput.value = ''
  loadSeeds()
  loadDistribution()
})

watch(pageSize, () => {
  currentPage.value = 1
  pageInput.value = ''
  loadSeeds()
})

watch(pageCount, value => {
  if (currentPage.value > value) {
    currentPage.value = value
  }
})

watch(currentPage, () => {
  loadSeeds()
})

watch(
  () => props.open,
  open => {
    if (!open) {
      showEditorModal.value = false
      editingSeed.value = null
      seedPendingDelete.value = null
      resetFilters()
      resetSeedForm()
    }
  }
)
</script>
