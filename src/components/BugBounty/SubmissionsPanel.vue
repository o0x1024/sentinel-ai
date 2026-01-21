<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div class="flex items-center gap-2">
            <h2 class="card-title">{{ t('bugBounty.submissions.title') }}</h2>
            <span v-if="selectedIds.length > 0" class="badge badge-primary">
              {{ selectedIds.length }} {{ t('bugBounty.batch.selected') }}
            </span>
          </div>
          <div class="flex gap-2 flex-wrap">
            <!-- Batch Actions -->
            <div v-if="selectedIds.length > 0" class="flex gap-2">
              <div class="dropdown dropdown-end">
                <label tabindex="0" class="btn btn-sm btn-outline">
                  <i class="fas fa-edit mr-2"></i>
                  {{ t('bugBounty.batch.updateStatus') }}
                </label>
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
              <button class="btn btn-sm btn-error btn-outline" @click="batchDelete">
                <i class="fas fa-trash mr-2"></i>
                {{ t('bugBounty.batch.delete') }}
              </button>
              <button class="btn btn-sm btn-ghost" @click="clearSelection">
                <i class="fas fa-times"></i>
              </button>
            </div>

            <!-- Filters -->
            <select v-model="filter.status" class="select select-sm select-bordered" @change="$emit('filter-change', filter)">
              <option value="">{{ t('bugBounty.filter.allStatuses') }}</option>
              <option value="draft">{{ t('bugBounty.submissionStatus.draft') }}</option>
              <option value="submitted">{{ t('bugBounty.submissionStatus.submitted') }}</option>
              <option value="triaged">{{ t('bugBounty.submissionStatus.triaged') }}</option>
              <option value="accepted">{{ t('bugBounty.submissionStatus.accepted') }}</option>
              <option value="rejected">{{ t('bugBounty.submissionStatus.rejected') }}</option>
              <option value="duplicate">{{ t('bugBounty.submissionStatus.duplicate') }}</option>
              <option value="resolved">{{ t('bugBounty.submissionStatus.resolved') }}</option>
            </select>
            <button class="btn btn-sm btn-primary" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.createSubmission') }}
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

        <!-- Batch Summary -->
        <div v-if="selectedIds.length > 0" class="mt-4 p-3 bg-primary/10 rounded-lg flex items-center justify-between">
          <div class="text-sm">
            <span class="font-medium">{{ selectedIds.length }}</span> {{ t('bugBounty.batch.itemsSelected') }}
            <span v-if="selectedTotalReward > 0" class="ml-4">
              {{ t('bugBounty.batch.totalReward') }}: <span class="text-success font-medium">${{ selectedTotalReward.toFixed(2) }}</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  submissions: any[]
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'view', submission: any): void
  (e: 'edit', submission: any): void
  (e: 'delete', submission: any): void
  (e: 'filter-change', filter: any): void
  (e: 'batch-update-status', ids: string[], status: string): void
  (e: 'batch-delete', ids: string[]): void
}>()

const filter = reactive({
  status: '',
})

const selectedIds = ref<string[]>([])

// Computed
const filteredSubmissions = computed(() => {
  if (!filter.status) return props.submissions
  return props.submissions.filter(s => s.status === filter.status)
})

const isAllSelected = computed(() => {
  return filteredSubmissions.value.length > 0 && selectedIds.value.length === filteredSubmissions.value.length
})

const isPartialSelected = computed(() => {
  return selectedIds.value.length > 0 && selectedIds.value.length < filteredSubmissions.value.length
})

const selectedTotalReward = computed(() => {
  return props.submissions
    .filter(s => selectedIds.value.includes(s.id))
    .reduce((sum, s) => sum + (s.reward_amount || 0) + (s.bonus_amount || 0), 0)
})

// Methods
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
    selectedIds.value = []
  } else {
    selectedIds.value = filteredSubmissions.value.map(s => s.id)
  }
}

const clearSelection = () => {
  selectedIds.value = []
}

const batchUpdateStatus = (status: string) => {
  if (selectedIds.value.length === 0) return
  emit('batch-update-status', [...selectedIds.value], status)
  clearSelection()
}

const batchDelete = () => {
  if (selectedIds.value.length === 0) return
  emit('batch-delete', [...selectedIds.value])
  clearSelection()
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
