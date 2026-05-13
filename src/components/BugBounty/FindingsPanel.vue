<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex flex-col gap-3 mb-4 xl:flex-row xl:items-center xl:justify-between">
          <h2 class="card-title">{{ t('bugBounty.findings.title') }}</h2>
          <div class="flex gap-2 flex-wrap">
            <select
              v-model="filter.severity"
              class="select select-sm select-bordered"
              :disabled="loading || selectionLoading || batchActionLoading"
              @change="onFilterChange"
            >
              <option value="">{{ t('bugBounty.filter.allSeverities') }}</option>
              <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
              <option value="high">{{ t('bugBounty.severity.high') }}</option>
              <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
              <option value="low">{{ t('bugBounty.severity.low') }}</option>
              <option value="info">{{ t('bugBounty.severity.info') }}</option>
            </select>
            <select
              v-model="filter.status"
              class="select select-sm select-bordered"
              :disabled="loading || selectionLoading || batchActionLoading"
              @change="onFilterChange"
            >
              <option value="">{{ t('bugBounty.filter.allStatuses') }}</option>
              <option value="new">{{ t('bugBounty.status.new') }}</option>
              <option value="verified">{{ t('bugBounty.status.verified') }}</option>
              <option value="reported">{{ t('bugBounty.status.reported') }}</option>
              <option value="duplicate">{{ t('bugBounty.status.duplicate') }}</option>
              <option value="fixed">{{ t('bugBounty.status.fixed') }}</option>
            </select>
            <input
              v-model="filter.search"
              type="text"
              class="input input-sm input-bordered w-48"
              :placeholder="t('bugBounty.search')"
              :disabled="loading || selectionLoading || batchActionLoading"
              @input="onFilterChange"
            />
            <button
              class="btn btn-sm btn-outline"
              :disabled="loading || selectionLoading || batchActionLoading"
              @click="$emit('refresh')"
            >
              <i class="fas fa-rotate-right mr-2"></i>
              {{ t('common.refresh', '刷新') }}
            </button>
            <button class="btn btn-sm btn-primary" :disabled="batchActionLoading" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.createFinding.title') }}
            </button>
          </div>
        </div>

        <div
          v-if="total > 0 && hasBatchSelection"
          class="mb-4 flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
        >
          <div class="text-sm text-base-content/70">
            {{ t('bugBounty.batch.selectionSummary', { selected: selectedIds.length, page: findings.length, total }) }}
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <button
              class="btn btn-sm btn-error btn-outline"
              :disabled="selectedIds.length === 0 || loading || selectionLoading || batchActionLoading"
              @click="batchDelete"
            >
              <i class="fas fa-trash mr-2"></i>
              {{ t('bugBounty.batch.delete') }}
            </button>
            <button
              class="btn btn-sm btn-error btn-outline"
              :disabled="selectedIds.length === 0 || loading || batchActionLoading || globalTotal <= 0"
              @click="$emit('delete-all')"
            >
              <i class="fas fa-trash-can mr-2"></i>
              {{ t('bugBounty.batch.deleteAll') }}
            </button>
            <div class="dropdown dropdown-end">
              <button
                tabindex="0"
                class="btn btn-sm btn-outline"
                :disabled="selectedIds.length === 0 || loading || selectionLoading || batchActionLoading"
              >
                <i class="fas fa-edit mr-2"></i>
                {{ t('bugBounty.batch.updateStatus') }}
              </button>
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-lg bg-base-100 rounded-box w-40">
                <li><a @click="batchUpdateStatus('new')">{{ t('bugBounty.status.new') }}</a></li>
                <li><a @click="batchUpdateStatus('verified')">{{ t('bugBounty.status.verified') }}</a></li>
                <li><a @click="batchUpdateStatus('reported')">{{ t('bugBounty.status.reported') }}</a></li>
                <li><a @click="batchUpdateStatus('duplicate')">{{ t('bugBounty.status.duplicate') }}</a></li>
                <li><a @click="batchUpdateStatus('fixed')">{{ t('bugBounty.status.fixed') }}</a></li>
              </ul>
            </div>
          </div>
        </div>

        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="findings.length === 0" class="text-center py-8">
          <i class="fas fa-bug text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.findings.empty') }}</p>
          <button class="btn btn-primary btn-sm mt-4" @click="$emit('create')">
            {{ t('bugBounty.createFirstFinding') }}
          </button>
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
                    :disabled="loading || selectionLoading || batchActionLoading"
                    @change="toggleSelectAll"
                  />
                </th>
                <th>{{ t('bugBounty.table.title') }}</th>
                <th>{{ t('bugBounty.table.type') }}</th>
                <th>{{ t('bugBounty.table.severity') }}</th>
                <th>{{ t('bugBounty.table.status') }}</th>
                <th>{{ t('bugBounty.table.program') }}</th>
                <th>{{ t('bugBounty.table.date') }}</th>
                <th>{{ t('bugBounty.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="finding in findings" :key="finding.id" class="hover" :class="{ 'bg-primary/10': isSelected(finding.id) }">
                <td>
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="isSelected(finding.id)"
                    :disabled="loading || selectionLoading || batchActionLoading"
                    @change="toggleSelect(finding.id)"
                  />
                </td>
                <td>
                  <div class="font-medium">{{ finding.title }}</div>
                  <div v-if="finding.affected_url" class="text-xs text-base-content/60 truncate max-w-xs">
                    <a
                      :href="finding.affected_url"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="cursor-pointer hover:underline"
                    >
                      {{ finding.affected_url }}
                    </a>
                  </div>
                </td>
                <td>
                  <span class="badge badge-ghost badge-sm">{{ finding.finding_type }}</span>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getSeverityClass(finding.severity)">
                    {{ finding.severity }}
                  </span>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getFindingStatusClass(finding.status)">
                    {{ finding.status }}
                  </span>
                </td>
                <td>
                  <span class="text-sm">{{ getProgramName(finding.program_id) }}</span>
                </td>
                <td>
                  <span class="text-sm">{{ formatDate(finding.created_at) }}</span>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button class="btn btn-ghost btn-xs" @click="$emit('view', finding)" :title="t('common.view')">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs" @click="$emit('create-submission', finding)" :title="t('bugBounty.createSubmission')">
                      <i class="fas fa-paper-plane"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs text-error" @click="$emit('delete', finding)" :title="t('common.delete')">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="total > 0" class="flex flex-col gap-3 pt-2 xl:flex-row xl:items-center xl:justify-between">
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{ t('bugBounty.surface.inventory.pageSizeLabel') }}</span>
            <select v-model.number="localPageSize" class="select select-bordered select-sm" :disabled="loading || selectionLoading" @change="onPageSizeChange">
              <option v-for="size in pageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button class="join-item btn btn-sm" :disabled="page <= 1 || loading" @click="goToFirstPage">
                {{ t('bugBounty.surface.inventory.firstPage') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="page <= 1 || loading" @click="goToPrevPage">
                {{ t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{ t('bugBounty.surface.inventory.pageInfo', { page, total: pageCount }) }}
              </button>
              <button class="join-item btn btn-sm" :disabled="page >= pageCount || loading" @click="goToNextPage">
                {{ t('common.next') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="page >= pageCount || loading" @click="goToLastPage">
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
              <button class="btn btn-sm btn-outline" :disabled="loading" @click="applyPageJump">
                {{ t('bugBounty.surface.inventory.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { ref, reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  findings: any[]
  programs: any[]
  loading: boolean
  batchActionLoading: boolean
  batchActionVersion: number
  page: number
  pageSize: number
  pageCount: number
  total: number
  globalTotal: number
  hasNext: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'refresh'): void
  (e: 'view', finding: any): void
  (e: 'delete', finding: any): void
  (e: 'create-submission', finding: any): void
  (e: 'filter-change', filter: any): void
  (e: 'batch-update-status', ids: string[], status: string): void
  (e: 'batch-delete', ids: string[]): void
  (e: 'delete-all'): void
  (e: 'page-change', page: number): void
  (e: 'page-size-change', size: number): void
}>()

const filter = reactive({
  severity: '',
  status: '',
  search: '',
})

const selectedIds = ref<string[]>([])
const pageInput = ref(String(props.page))
const localPageSize = ref(props.pageSize)
const selectionLoading = ref(false)
const pageSizeOptions = [10, 20, 50, 100]

const buildFindingFilter = (includePagination = true) => {
  const nextFilter: Record<string, unknown> = {
    sort_by: 'created_at',
    sort_dir: 'desc',
  }

  if (filter.severity) {
    nextFilter.severities = [filter.severity]
  }
  if (filter.status) {
    nextFilter.statuses = [filter.status]
  }
  if (filter.search) {
    nextFilter.search = filter.search
  }
  if (includePagination) {
    nextFilter.limit = props.pageSize
    nextFilter.offset = (props.page - 1) * props.pageSize
  }

  return nextFilter
}

const isAllSelected = computed(() => {
  return props.findings.length > 0 && props.findings.every(finding => selectedIds.value.includes(finding.id))
})

const isPartialSelected = computed(() => {
  const selectedOnPage = props.findings.filter(finding => selectedIds.value.includes(finding.id)).length
  return selectedOnPage > 0 && selectedOnPage < props.findings.length
})
const hasBatchSelection = computed(() => selectedIds.value.length > 0)
const isSelected = (id: string) => selectedIds.value.includes(id)

const onFilterChange = () => {
  clearSelection()
  emit('filter-change', { ...filter })
}

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
    const currentPageIds = new Set(props.findings.map(finding => finding.id))
    selectedIds.value = selectedIds.value.filter(id => !currentPageIds.has(id))
  } else {
    const next = new Set(selectedIds.value)
    for (const finding of props.findings) {
      next.add(finding.id)
    }
    selectedIds.value = [...next]
  }
}

const clearSelection = () => {
  selectedIds.value = []
}

const batchUpdateStatus = (status: string) => {
  if (selectedIds.value.length === 0 || props.batchActionLoading) return
  emit('batch-update-status', [...selectedIds.value], status)
}

const batchDelete = () => {
  if (selectedIds.value.length === 0 || props.batchActionLoading) return
  emit('batch-delete', [...selectedIds.value])
}

const goToPrevPage = () => {
  if (props.page <= 1) return
  emit('page-change', props.page - 1)
}

const goToNextPage = () => {
  if (props.page >= props.pageCount) return
  emit('page-change', props.page + 1)
}

const goToFirstPage = () => {
  if (props.page <= 1) return
  emit('page-change', 1)
}

const goToLastPage = () => {
  if (props.page >= props.pageCount) return
  emit('page-change', props.pageCount)
}

const applyPageJump = () => {
  const nextPage = Number.parseInt(pageInput.value, 10)
  if (Number.isNaN(nextPage)) {
    pageInput.value = String(props.page)
    return
  }
  const clamped = Math.min(Math.max(1, nextPage), props.pageCount)
  emit('page-change', clamped)
}

const onPageSizeChange = () => {
  if (localPageSize.value === props.pageSize) return
  clearSelection()
  emit('page-size-change', localPageSize.value)
}

watch(
  () => props.page,
  value => {
    pageInput.value = String(value)
  },
)

watch(
  () => props.pageSize,
  value => {
    localPageSize.value = value
  },
)

watch(
  () => props.batchActionVersion,
  (_, previousValue) => {
    if (previousValue !== undefined) {
      clearSelection()
    }
  },
)

const getProgramName = (programId: string) => {
  const program = props.programs.find(p => p.id === programId)
  return program?.name || programId.substring(0, 8)
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return '-'
  const date = new Date(dateStr)
  return date.toLocaleDateString()
}

const getSeverityClass = (severity: string) => {
  const classes: Record<string, string> = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-success',
    info: 'badge-ghost',
  }
  return classes[severity.toLowerCase()] || 'badge-ghost'
}

const getFindingStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    new: 'badge-info',
    verified: 'badge-success',
    reported: 'badge-primary',
    duplicate: 'badge-warning',
    fixed: 'badge-neutral',
    wontfix: 'badge-ghost',
  }
  return classes[status.toLowerCase()] || 'badge-ghost'
}
</script>
