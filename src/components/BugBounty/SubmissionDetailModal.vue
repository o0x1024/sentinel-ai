<template>
  <div v-if="visible" class="modal modal-open">
    <div class="modal-box max-w-4xl h-[85vh] flex flex-col">
      <!-- Header -->
      <div class="flex justify-between items-start mb-4">
        <div>
          <div class="flex items-center gap-2 mb-1">
            <span class="badge" :class="getSeverityClass(submission?.severity)">{{ submission?.severity }}</span>
            <span class="badge" :class="getStatusClass(submission?.status)">{{ getStatusLabel(submission?.status) }}</span>
            <span v-if="submission?.platform_submission_id" class="badge badge-outline">
              #{{ submission.platform_submission_id }}
            </span>
          </div>
          <h3 class="font-bold text-xl">{{ submission?.title }}</h3>
          <p class="text-sm text-base-content/70 mt-1">
            <span class="badge badge-ghost badge-sm mr-2">{{ submission?.vulnerability_type }}</span>
            <span v-if="submission?.cwe_id">CWE-{{ submission.cwe_id }}</span>
          </p>
        </div>
        <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <!-- Quick Stats -->
      <div class="grid grid-cols-5 gap-3 mb-4">
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">CVSS</div>
          <div class="font-bold">{{ submission?.cvss_score?.toFixed(1) || '-' }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.submissionDetail.reward') }}</div>
          <div class="font-bold text-success">${{ (submission?.reward_amount || 0).toFixed(0) }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.submissionDetail.bonus') }}</div>
          <div class="font-bold text-warning">${{ (submission?.bonus_amount || 0).toFixed(0) }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.submissionDetail.responseTime') }}</div>
          <div class="font-bold text-sm">{{ submission?.response_time_hours ? `${submission.response_time_hours}h` : '-' }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.submissionDetail.submitted') }}</div>
          <div class="font-bold text-sm">{{ formatDate(submission?.submitted_at) }}</div>
        </div>
      </div>

      <!-- Tabs -->
      <div class="tabs tabs-boxed mb-4">
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'details' }" @click="activeTab = 'details'">
          <i class="fas fa-info-circle mr-2"></i>{{ t('bugBounty.submissionDetail.detailsTab') }}
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'edit' }" @click="activeTab = 'edit'">
          <i class="fas fa-edit mr-2"></i>{{ t('bugBounty.submissionDetail.editTab') }}
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'timeline' }" @click="activeTab = 'timeline'">
          <i class="fas fa-history mr-2"></i>{{ t('bugBounty.submissionDetail.timelineTab') }}
        </a>
      </div>

      <!-- Tab Content -->
      <div class="flex-1 overflow-auto">
        <!-- Details Tab -->
        <div v-if="activeTab === 'details'" class="space-y-4">
          <!-- Description -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.description') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4 whitespace-pre-wrap min-h-[80px]">
              {{ submission?.description || '-' }}
            </div>
          </div>

          <!-- Impact -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.impact') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4 whitespace-pre-wrap min-h-[60px]">
              {{ submission?.impact || '-' }}
            </div>
          </div>

          <!-- Remediation -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.remediation') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4 whitespace-pre-wrap min-h-[60px]">
              {{ submission?.remediation || '-' }}
            </div>
          </div>

          <!-- Reproduction Steps -->
          <div v-if="reproductionSteps.length > 0" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.reproductionSteps') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4">
              <ol class="list-decimal list-inside space-y-2">
                <li v-for="(step, index) in reproductionSteps" :key="index" class="text-sm">{{ step }}</li>
              </ol>
            </div>
          </div>

          <!-- Platform Info -->
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.platformId') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">
                {{ submission?.platform_submission_id || '-' }}
              </div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.platformUrl') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">
                <a v-if="submission?.platform_url" :href="submission.platform_url" target="_blank" class="link link-primary">
                  {{ submission.platform_url }}
                </a>
                <span v-else>-</span>
              </div>
            </div>
          </div>

          <!-- Times -->
          <div class="grid grid-cols-3 gap-4 text-sm text-base-content/70">
            <div>{{ t('bugBounty.programDetail.createdAt') }}: {{ formatDateTime(submission?.created_at) }}</div>
            <div>{{ t('bugBounty.submissionDetail.submittedAt') }}: {{ formatDateTime(submission?.submitted_at) }}</div>
            <div>{{ t('bugBounty.submissionDetail.closedAt') }}: {{ formatDateTime(submission?.closed_at) }}</div>
          </div>
        </div>

        <!-- Edit Tab -->
        <div v-if="activeTab === 'edit'" class="space-y-4">
          <div class="alert alert-info text-sm">
            <i class="fas fa-info-circle"></i>
            <span>{{ t('bugBounty.submissionDetail.editHint') }}</span>
          </div>

          <!-- Status -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('common.status') }} *</span></label>
            <select v-model="editForm.status" class="select select-bordered">
              <option value="draft">{{ t('bugBounty.submissionStatus.draft') }}</option>
              <option value="submitted">{{ t('bugBounty.submissionStatus.submitted') }}</option>
              <option value="triaged">{{ t('bugBounty.submissionStatus.triaged') }}</option>
              <option value="accepted">{{ t('bugBounty.submissionStatus.accepted') }}</option>
              <option value="rejected">{{ t('bugBounty.submissionStatus.rejected') }}</option>
              <option value="duplicate">{{ t('bugBounty.submissionStatus.duplicate') }}</option>
              <option value="informative">{{ t('bugBounty.submissionStatus.informative') }}</option>
              <option value="resolved">{{ t('bugBounty.submissionStatus.resolved') }}</option>
            </select>
          </div>

          <!-- Platform Info -->
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.platformId') }}</span></label>
              <input v-model="editForm.platform_submission_id" type="text" class="input input-bordered" placeholder="HackerOne: #123456" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.platformUrl') }}</span></label>
              <input v-model="editForm.platform_url" type="text" class="input input-bordered" placeholder="https://hackerone.com/reports/123456" />
            </div>
          </div>

          <!-- Reward -->
          <div class="grid grid-cols-3 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.rewardAmount') }}</span></label>
              <input v-model.number="editForm.reward_amount" type="number" class="input input-bordered" placeholder="0.00" min="0" step="0.01" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.currency') }}</span></label>
              <select v-model="editForm.reward_currency" class="select select-bordered">
                <option value="USD">USD</option>
                <option value="EUR">EUR</option>
                <option value="CNY">CNY</option>
                <option value="GBP">GBP</option>
              </select>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.bonusAmount') }}</span></label>
              <input v-model.number="editForm.bonus_amount" type="number" class="input input-bordered" placeholder="0.00" min="0" step="0.01" />
            </div>
          </div>

          <!-- Response Time -->
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.responseTimeHours') }}</span></label>
              <input v-model.number="editForm.response_time_hours" type="number" class="input input-bordered" placeholder="24" min="0" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.submissionDetail.resolutionTimeHours') }}</span></label>
              <input v-model.number="editForm.resolution_time_hours" type="number" class="input input-bordered" placeholder="168" min="0" />
            </div>
          </div>

          <!-- Save Button -->
          <div class="flex justify-end">
            <button class="btn btn-primary" @click="saveChanges" :disabled="saving">
              <span v-if="saving" class="loading loading-spinner loading-sm mr-2"></span>
              <i v-else class="fas fa-save mr-2"></i>
              {{ t('common.save') }}
            </button>
          </div>
        </div>

        <!-- Timeline Tab -->
        <div v-if="activeTab === 'timeline'" class="space-y-4">
          <div class="text-center py-8 text-base-content/70">
            <i class="fas fa-history text-4xl mb-4 opacity-30"></i>
            <p>{{ t('bugBounty.submissionDetail.timelineEmpty') }}</p>
          </div>
          
          <!-- Status Timeline Preview -->
          <div class="steps steps-vertical">
            <div class="step" :class="{ 'step-primary': isStatusReached('draft') }">
              <div class="text-sm">{{ t('bugBounty.submissionStatus.draft') }}</div>
              <div v-if="submission?.created_at" class="text-xs text-base-content/60">{{ formatDateTime(submission.created_at) }}</div>
            </div>
            <div class="step" :class="{ 'step-primary': isStatusReached('submitted') }">
              <div class="text-sm">{{ t('bugBounty.submissionStatus.submitted') }}</div>
              <div v-if="submission?.submitted_at" class="text-xs text-base-content/60">{{ formatDateTime(submission.submitted_at) }}</div>
            </div>
            <div class="step" :class="{ 'step-primary': isStatusReached('triaged') }">
              <div class="text-sm">{{ t('bugBounty.submissionStatus.triaged') }}</div>
            </div>
            <div class="step" :class="{ 'step-primary': isStatusReached('accepted'), 'step-success': submission?.status === 'accepted' }">
              <div class="text-sm">{{ t('bugBounty.submissionStatus.accepted') }} / {{ t('bugBounty.submissionStatus.resolved') }}</div>
              <div v-if="submission?.closed_at" class="text-xs text-base-content/60">{{ formatDateTime(submission.closed_at) }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="modal-action">
        <a v-if="submission?.platform_url" :href="submission.platform_url" target="_blank" class="btn btn-outline">
          <i class="fas fa-external-link-alt mr-2"></i>{{ t('bugBounty.submissionDetail.openPlatform') }}
        </a>
        <button class="btn" @click="$emit('close')">{{ t('common.close') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  visible: boolean
  submission: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

// State
const activeTab = ref('details')
const saving = ref(false)

const editForm = reactive({
  status: '',
  platform_submission_id: '',
  platform_url: '',
  reward_amount: null as number | null,
  reward_currency: 'USD',
  bonus_amount: null as number | null,
  response_time_hours: null as number | null,
  resolution_time_hours: null as number | null,
})

// Computed
const reproductionSteps = computed(() => {
  if (!props.submission?.reproduction_steps_json) return []
  try {
    return JSON.parse(props.submission.reproduction_steps_json)
  } catch {
    return []
  }
})

// Methods
const initEditForm = () => {
  if (!props.submission) return
  editForm.status = props.submission.status || 'draft'
  editForm.platform_submission_id = props.submission.platform_submission_id || ''
  editForm.platform_url = props.submission.platform_url || ''
  editForm.reward_amount = props.submission.reward_amount || null
  editForm.reward_currency = props.submission.reward_currency || 'USD'
  editForm.bonus_amount = props.submission.bonus_amount || null
  editForm.response_time_hours = props.submission.response_time_hours || null
  editForm.resolution_time_hours = props.submission.resolution_time_hours || null
}

const saveChanges = async () => {
  try {
    saving.value = true
    const request: any = {
      status: editForm.status,
      platform_submission_id: editForm.platform_submission_id || null,
      platform_url: editForm.platform_url || null,
      reward_amount: editForm.reward_amount,
      reward_currency: editForm.reward_currency,
      bonus_amount: editForm.bonus_amount,
    }
    await invoke('bounty_update_submission', { id: props.submission.id, request })
    toast.success(t('bugBounty.success.submissionUpdated'))
    emit('updated')
  } catch (error) {
    console.error('Failed to update submission:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  } finally {
    saving.value = false
  }
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleDateString()
}

const formatDateTime = (dateStr: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString()
}

const getSeverityClass = (severity: string) => {
  const classes: Record<string, string> = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-success',
    info: 'badge-ghost',
  }
  return classes[severity?.toLowerCase()] || 'badge-ghost'
}

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    draft: 'badge-ghost',
    submitted: 'badge-info',
    triaged: 'badge-primary',
    accepted: 'badge-success',
    rejected: 'badge-error',
    duplicate: 'badge-warning',
    informative: 'badge-neutral',
    resolved: 'badge-success',
  }
  return classes[status?.toLowerCase()] || 'badge-ghost'
}

const getStatusLabel = (status: string) => {
  const key = `bugBounty.submissionStatus.${status}`
  const label = t(key)
  return label !== key ? label : status
}

const statusOrder = ['draft', 'submitted', 'triaged', 'accepted', 'resolved']
const isStatusReached = (checkStatus: string) => {
  const currentIndex = statusOrder.indexOf(props.submission?.status?.toLowerCase())
  const checkIndex = statusOrder.indexOf(checkStatus)
  if (currentIndex === -1) return checkStatus === 'draft'
  return checkIndex <= currentIndex
}

// Watchers
watch(() => props.visible, (val) => {
  if (val && props.submission) {
    activeTab.value = 'details'
    initEditForm()
  }
})
</script>
