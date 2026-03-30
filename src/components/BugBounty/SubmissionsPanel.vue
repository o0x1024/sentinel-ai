<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex flex-col gap-3 mb-4 xl:flex-row xl:items-center xl:justify-between">
          <h2 class="card-title">{{ t('bugBounty.submissions.title') }}</h2>
          <div class="flex gap-2 flex-wrap">
            <select
              v-model="filter.status"
              class="select select-sm select-bordered"
              :disabled="loading || selectionLoading || batchActionLoading"
              @change="onFilterChange"
            >
              <option value="">{{ t('bugBounty.filter.allStatuses') }}</option>
              <option value="draft">{{ t('bugBounty.submissionStatus.draft') }}</option>
              <option value="submitted">{{ t('bugBounty.submissionStatus.submitted') }}</option>
              <option value="triaged">{{ t('bugBounty.submissionStatus.triaged') }}</option>
              <option value="accepted">{{ t('bugBounty.submissionStatus.accepted') }}</option>
              <option value="rejected">{{ t('bugBounty.submissionStatus.rejected') }}</option>
              <option value="duplicate">{{ t('bugBounty.submissionStatus.duplicate') }}</option>
              <option value="resolved">{{ t('bugBounty.submissionStatus.resolved') }}</option>
            </select>
            <input
              v-model="filter.search"
              type="text"
              class="input input-sm input-bordered w-48"
              :placeholder="t('bugBounty.search')"
              :disabled="loading || selectionLoading || batchActionLoading"
              @input="onFilterChange"
            />
            <button class="btn btn-sm btn-primary" :disabled="batchActionLoading" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.createSubmission') }}
            </button>
          </div>
        </div>

        <div
          v-if="total > 0"
          class="mb-4 flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
        >
          <div class="space-y-1 text-sm text-base-content/70">
            <div>
              {{ t('bugBounty.batch.selectionSummary', { selected: selectedIds.length, page: filteredSubmissions.length, total }) }}
            </div>
            <div v-if="showSelectedTotalReward && selectedTotalReward > 0">
              {{ t('bugBounty.batch.totalReward') }}:
              <span class="font-medium text-success">${{ selectedTotalReward.toFixed(2) }}</span>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <button
              class="btn btn-sm btn-outline"
              :disabled="loading || selectionLoading || batchActionLoading || total === 0 || allFilteredSelected"
              @click="selectAllFiltered"
            >
              <span v-if="selectionLoading" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-layer-group mr-2"></i>
              {{ t('bugBounty.batch.selectAllFiltered') }}
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
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-lg bg-base-100 rounded-box w-44">
                <li><a @click="batchUpdateStatus('draft')">{{ t('bugBounty.submissionStatus.draft') }}</a></li>
                <li><a @click="batchUpdateStatus('submitted')">{{ t('bugBounty.submissionStatus.submitted') }}</a></li>
                <li><a @click="batchUpdateStatus('triaged')">{{ t('bugBounty.submissionStatus.triaged') }}</a></li>
                <li><a @click="batchUpdateStatus('accepted')">{{ t('bugBounty.submissionStatus.accepted') }}</a></li>
                <li><a @click="batchUpdateStatus('rejected')">{{ t('bugBounty.submissionStatus.rejected') }}</a></li>
                <li><a @click="batchUpdateStatus('duplicate')">{{ t('bugBounty.submissionStatus.duplicate') }}</a></li>
                <li><a @click="batchUpdateStatus('resolved')">{{ t('bugBounty.submissionStatus.resolved') }}</a></li>
              </ul>
            </div>
            <button
              class="btn btn-sm btn-error btn-outline"
              :disabled="selectedIds.length === 0 || loading || selectionLoading || batchActionLoading"
              @click="batchDelete"
            >
              <i class="fas fa-trash mr-2"></i>
              {{ t('bugBounty.batch.delete') }}
            </button>
          </div>
        </div>
        
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="filteredSubmissions.length === 0" class="text-center py-8">
          <i class="fas fa-paper-plane text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.submissions.empty') }}</p>
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
                <th>{{ t('bugBounty.table.reward') }}</th>
                <th>{{ t('bugBounty.table.submittedAt') }}</th>
                <th>{{ t('bugBounty.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="submission in filteredSubmissions" :key="submission.id" class="hover" :class="{ 'bg-primary/10': isSelected(submission.id) }">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    :checked="isSelected(submission.id)"
                    :disabled="loading || selectionLoading || batchActionLoading"
                    @change="toggleSelect(submission.id)"
                  />
                </td>
                <td>
                  <div class="font-medium">{{ submission.title }}</div>
                  <div v-if="submission.platform_submission_id" class="text-xs text-base-content/60">
                    #{{ submission.platform_submission_id }}
                  </div>
                </td>
                <td>
                  <span class="badge badge-ghost badge-sm">{{ submission.vulnerability_type }}</span>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getSeverityClass(submission.severity)">
                    {{ submission.severity }}
                  </span>
                </td>
                <td>
                  <span class="badge badge-sm" :class="getSubmissionStatusClass(submission.status)">
                    {{ submission.status }}
                  </span>
                </td>
                <td>
                  <span v-if="submission.reward_amount" class="text-success font-medium">
                    ${{ submission.reward_amount }}
                  </span>
                  <span v-else class="text-base-content/50">-</span>
                </td>
                <td>
                  <span class="text-sm">{{ submission.submitted_at ? formatDate(submission.submitted_at) : '-' }}</span>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button class="btn btn-ghost btn-xs" @click="$emit('view', submission)" :title="t('common.view')">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs" @click="$emit('edit', submission)" :title="t('common.edit')">
                      <i class="fas fa-edit"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs text-error" @click="$emit('delete', submission)" :title="t('common.delete')">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="filteredSubmissions.length > 0" class="flex justify-center py-4">
          <div class="join">
            <button class="join-item btn btn-sm" :disabled="page <= 1" @click="goToPrevPage">«</button>
            <button class="join-item btn btn-sm">{{ page }}</button>
            <button class="join-item btn btn-sm" :disabled="!hasNext" @click="goToNextPage">»</button>
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
  submissions: any[]
  loading: boolean
  batchActionLoading: boolean
  batchActionVersion: number
  page: number
  pageSize: number
  total: number
  hasNext: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'view', submission: any): void
  (e: 'edit', submission: any): void
  (e: 'delete', submission: any): void
  (e: 'filter-change', filter: any): void
  (e: 'batch-update-status', ids: string[], status: string): void
  (e: 'batch-delete', ids: string[]): void
  (e: 'page-change', page: number): void
}>()

const filter = reactive({
  status: '',
  search: '',
})

const selectedIds = ref<string[]>([])
const selectionLoading = ref(false)

// Computed
const filteredSubmissions = computed(() => props.submissions)

const isAllSelected = computed(() => {
  return filteredSubmissions.value.length > 0 && filteredSubmissions.value.every(submission => selectedIds.value.includes(submission.id))
})

const isPartialSelected = computed(() => {
  const selectedOnPage = filteredSubmissions.value.filter(submission => selectedIds.value.includes(submission.id)).length
  return selectedOnPage > 0 && selectedOnPage < filteredSubmissions.value.length
})
const allFilteredSelected = computed(() => props.total > 0 && selectedIds.value.length >= props.total)
const showSelectedTotalReward = computed(() => {
  if (selectedIds.value.length === 0) return false
  const visibleIds = new Set(props.submissions.map(submission => submission.id))
  return selectedIds.value.every(id => visibleIds.has(id))
})

const selectedTotalReward = computed(() => {
  return props.submissions
    .filter(s => selectedIds.value.includes(s.id))
    .reduce((sum, s) => sum + (s.reward_amount || 0) + (s.bonus_amount || 0), 0)
})

// Methods
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
    const currentPageIds = new Set(filteredSubmissions.value.map(submission => submission.id))
    selectedIds.value = selectedIds.value.filter(id => !currentPageIds.has(id))
  } else {
    const next = new Set(selectedIds.value)
    for (const submission of filteredSubmissions.value) {
      next.add(submission.id)
    }
    selectedIds.value = [...next]
  }
}

const clearSelection = () => {
  selectedIds.value = []
}

const selectAllFiltered = async () => {
  if (props.total === 0 || allFilteredSelected.value || props.batchActionLoading) return

  try {
    selectionLoading.value = true
    const filterPayload: Record<string, unknown> = {
      sort_by: 'created_at',
      sort_dir: 'desc',
    }

    if (filter.status) {
      filterPayload.statuses = [filter.status]
    }
    if (filter.search) {
      filterPayload.search = filter.search
    }

    const rows = await invoke<any[]>('bounty_list_submissions', { filter: filterPayload })
    selectedIds.value = rows.map(row => row.id)
  } catch (error) {
    console.error('Failed to load submissions for batch selection:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    selectionLoading.value = false
  }
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
  if (!props.hasNext) return
  emit('page-change', props.page + 1)
}

watch(
  () => props.batchActionVersion,
  (_, previousValue) => {
    if (previousValue !== undefined) {
      clearSelection()
    }
  },
)

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

const getSubmissionStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    draft: 'badge-ghost',
    submitted: 'badge-info',
    triaged: 'badge-primary',
    accepted: 'badge-success',
    rejected: 'badge-error',
    duplicate: 'badge-warning',
    resolved: 'badge-success',
  }
  return classes[status.toLowerCase()] || 'badge-ghost'
}
</script>
