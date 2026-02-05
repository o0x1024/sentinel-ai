<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div class="flex items-center gap-2">
            <h2 class="card-title">{{ t('bugBounty.findings.title') }}</h2>
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
                <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-lg bg-base-100 rounded-box w-40">
                  <li><a @click="batchUpdateStatus('new')">{{ t('bugBounty.status.new') }}</a></li>
                  <li><a @click="batchUpdateStatus('verified')">{{ t('bugBounty.status.verified') }}</a></li>
                  <li><a @click="batchUpdateStatus('reported')">{{ t('bugBounty.status.reported') }}</a></li>
                  <li><a @click="batchUpdateStatus('duplicate')">{{ t('bugBounty.status.duplicate') }}</a></li>
                  <li><a @click="batchUpdateStatus('fixed')">{{ t('bugBounty.status.fixed') }}</a></li>
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
            <select v-model="filter.severity" class="select select-sm select-bordered" @change="$emit('filter-change', filter)">
              <option value="">{{ t('bugBounty.filter.allSeverities') }}</option>
              <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
              <option value="high">{{ t('bugBounty.severity.high') }}</option>
              <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
              <option value="low">{{ t('bugBounty.severity.low') }}</option>
              <option value="info">{{ t('bugBounty.severity.info') }}</option>
            </select>
            <select v-model="filter.status" class="select select-sm select-bordered" @change="$emit('filter-change', filter)">
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
              @input="$emit('filter-change', filter)"
            />
            <button class="btn btn-sm btn-primary" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.createFinding') }}
            </button>
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
                    @change="toggleSelect(finding.id)"
                  />
                </td>
                <td>
                  <div class="font-medium">{{ finding.title }}</div>
                  <div v-if="finding.affected_url" class="text-xs text-base-content/60 truncate max-w-xs">
                    {{ finding.affected_url }}
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

        <div v-if="findings.length > 0" class="flex justify-center py-4">
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
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  findings: any[]
  programs: any[]
  loading: boolean
  page: number
  pageSize: number
  hasNext: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'view', finding: any): void
  (e: 'delete', finding: any): void
  (e: 'create-submission', finding: any): void
  (e: 'filter-change', filter: any): void
  (e: 'batch-update-status', ids: string[], status: string): void
  (e: 'batch-delete', ids: string[]): void
  (e: 'page-change', page: number): void
}>()

const filter = reactive({
  severity: '',
  status: '',
  search: '',
})

const selectedIds = ref<string[]>([])

// Computed
const isAllSelected = computed(() => {
  return props.findings.length > 0 && selectedIds.value.length === props.findings.length
})

const isPartialSelected = computed(() => {
  return selectedIds.value.length > 0 && selectedIds.value.length < props.findings.length
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
    selectedIds.value = props.findings.map(f => f.id)
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

const goToPrevPage = () => {
  if (props.page <= 1) return
  emit('page-change', props.page - 1)
}

const goToNextPage = () => {
  if (!props.hasNext) return
  emit('page-change', props.page + 1)
}

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
