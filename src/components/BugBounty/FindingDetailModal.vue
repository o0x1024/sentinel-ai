<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal modal-open">
        <div class="modal-box max-w-4xl h-[85vh] flex flex-col">
      <!-- Header -->
      <div class="flex justify-between items-start mb-4">
        <div>
          <div class="flex items-center gap-2 mb-1">
            <span class="badge" :class="getSeverityClass(finding?.severity)">{{ finding?.severity }}</span>
            <span class="badge" :class="getStatusClass(finding?.status)">{{ finding?.status }}</span>
          </div>
          <h3 class="font-bold text-xl">{{ finding?.title }}</h3>
          <p class="text-sm text-base-content/70 mt-1">
            <span class="badge badge-ghost badge-sm mr-2">{{ finding?.finding_type }}</span>
            <span v-if="finding?.cwe_id">CWE-{{ finding.cwe_id }}</span>
          </p>
        </div>
        <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <!-- Quick Stats -->
      <div class="grid grid-cols-4 gap-3 mb-4">
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.findingDetail.cvss') }}</div>
          <div class="font-bold">{{ finding?.cvss_score?.toFixed(1) || '-' }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.findingDetail.evidence') }}</div>
          <div class="font-bold">{{ evidenceList.length }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.findingDetail.program') }}</div>
          <div class="font-bold text-sm truncate">{{ programName }}</div>
        </div>
        <div class="bg-base-200 rounded-lg px-3 py-2 text-center">
          <div class="text-xs text-base-content/60">{{ t('bugBounty.table.date') }}</div>
          <div class="font-bold text-sm">{{ formatDate(finding?.created_at) }}</div>
        </div>
      </div>

      <!-- Tabs -->
      <div class="tabs tabs-boxed mb-4">
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'details' }" @click="activeTab = 'details'">
          <i class="fas fa-info-circle mr-2"></i>{{ t('bugBounty.findingDetail.detailsTab') }}
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'evidence' }" @click="activeTab = 'evidence'">
          <i class="fas fa-paperclip mr-2"></i>{{ t('bugBounty.findingDetail.evidenceTab') }} ({{ evidenceList.length }})
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'technical' }" @click="activeTab = 'technical'">
          <i class="fas fa-code mr-2"></i>{{ t('bugBounty.findingDetail.technicalTab') }}
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
              {{ finding?.description || t('bugBounty.findingDetail.noDescription') }}
            </div>
          </div>

          <!-- Impact -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.impact') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4 whitespace-pre-wrap min-h-[60px]">
              {{ finding?.impact || '-' }}
            </div>
          </div>

          <!-- Remediation -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.remediation') }}</span></label>
            <div class="bg-base-200 rounded-lg p-4 whitespace-pre-wrap min-h-[60px]">
              {{ finding?.remediation || '-' }}
            </div>
          </div>

          <!-- Affected URL & Endpoint -->
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.affectedUrl') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2 break-all">
                <code v-if="finding?.affected_url" class="text-sm">{{ finding.affected_url }}</code>
                <span v-else class="text-base-content/50">-</span>
              </div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.affectedEndpoint') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2 break-all">
                <code v-if="finding?.affected_endpoint" class="text-sm">{{ finding.affected_endpoint }}</code>
                <span v-else class="text-base-content/50">-</span>
              </div>
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

          <!-- Tags -->
          <div v-if="tags.length > 0" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.tags') }}</span></label>
            <div class="flex flex-wrap gap-2">
              <span v-for="tag in tags" :key="tag" class="badge badge-outline">{{ tag }}</span>
            </div>
          </div>
        </div>

        <!-- Evidence Tab -->
        <div v-if="activeTab === 'evidence'" class="space-y-4">
          <div class="flex justify-between items-center">
            <h4 class="font-semibold">{{ t('bugBounty.findingDetail.evidenceList') }}</h4>
            <button class="btn btn-sm btn-primary" @click="showAddEvidenceModal = true">
              <i class="fas fa-plus mr-2"></i>{{ t('bugBounty.findingDetail.addEvidence') }}
            </button>
          </div>

          <div v-if="loadingEvidence" class="flex justify-center py-8">
            <span class="loading loading-spinner"></span>
          </div>

          <div v-else-if="evidenceList.length === 0" class="text-center py-8 text-base-content/70">
            <i class="fas fa-paperclip text-4xl mb-4 opacity-30"></i>
            <p>{{ t('bugBounty.findingDetail.noEvidence') }}</p>
          </div>

          <div v-else class="space-y-3">
            <div v-for="evidence in evidenceList" :key="evidence.id" class="card bg-base-200">
              <div class="card-body p-4">
                <div class="flex justify-between items-start">
                  <div class="flex-1">
                    <div class="flex items-center gap-2 mb-1">
                      <span class="badge badge-sm" :class="getEvidenceTypeClass(evidence.evidence_type)">
                        <i :class="getEvidenceTypeIcon(evidence.evidence_type)" class="mr-1"></i>
                        {{ evidence.evidence_type }}
                      </span>
                      <span class="font-medium">{{ evidence.title }}</span>
                    </div>
                    <p v-if="evidence.description" class="text-sm text-base-content/70 mb-2">{{ evidence.description }}</p>
                    
                    <!-- Content preview based on type -->
                    <div v-if="evidence.content" class="bg-base-300 rounded p-3 mt-2">
                      <pre class="text-xs overflow-x-auto whitespace-pre-wrap">{{ evidence.content }}</pre>
                    </div>
                    
                    <div v-if="evidence.http_request_json" class="mt-2">
                      <div class="text-xs font-semibold text-base-content/60 mb-1">HTTP Request</div>
                      <pre class="bg-base-300 rounded p-2 text-xs overflow-x-auto">{{ formatJson(evidence.http_request_json) }}</pre>
                    </div>
                    
                    <div v-if="evidence.http_response_json" class="mt-2">
                      <div class="text-xs font-semibold text-base-content/60 mb-1">HTTP Response</div>
                      <pre class="bg-base-300 rounded p-2 text-xs overflow-x-auto max-h-32">{{ formatJson(evidence.http_response_json) }}</pre>
                    </div>

                    <div v-if="evidence.file_path || evidence.file_url" class="mt-2">
                      <a 
                        v-if="evidence.file_url" 
                        :href="evidence.file_url" 
                        target="_blank" 
                        class="link link-primary text-sm"
                      >
                        <i class="fas fa-external-link-alt mr-1"></i>{{ evidence.file_url }}
                      </a>
                      <span v-else class="text-sm text-base-content/70">
                        <i class="fas fa-file mr-1"></i>{{ evidence.file_path }}
                      </span>
                    </div>
                  </div>
                  <button class="btn btn-ghost btn-xs text-error" @click="deleteEvidence(evidence)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Technical Tab -->
        <div v-if="activeTab === 'technical'" class="space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.findingType') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ finding?.finding_type || '-' }}</div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.cweId') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">
                <a v-if="finding?.cwe_id" :href="`https://cwe.mitre.org/data/definitions/${finding.cwe_id}.html`" target="_blank" class="link link-primary">
                  CWE-{{ finding.cwe_id }}
                </a>
                <span v-else>-</span>
              </div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.cvssScore') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ finding?.cvss_score?.toFixed(1) || '-' }}</div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.affectedParameter') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">
                <code v-if="finding?.affected_parameter">{{ finding.affected_parameter }}</code>
                <span v-else>-</span>
              </div>
            </div>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.fingerprint') }}</span></label>
            <div class="bg-base-200 rounded-lg px-4 py-2 font-mono text-xs break-all">{{ finding?.fingerprint || '-' }}</div>
          </div>

          <div class="grid grid-cols-2 gap-4 text-sm text-base-content/70">
            <div>{{ t('bugBounty.programDetail.createdAt') }}: {{ formatDateTime(finding?.created_at) }}</div>
            <div>{{ t('bugBounty.programDetail.updatedAt') }}: {{ formatDateTime(finding?.updated_at) }}</div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="modal-action">
        <button class="btn btn-primary" @click="$emit('create-submission', finding)">
          <i class="fas fa-paper-plane mr-2"></i>{{ t('bugBounty.createSubmission') }}
        </button>
        <button class="btn" @click="$emit('close')">{{ t('common.close') }}</button>
      </div>
    </div>

    <!-- Add Evidence Modal -->
    <div v-if="showAddEvidenceModal" class="modal modal-open z-50">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.findingDetail.addEvidence') }}</h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.evidence.type') }} *</span></label>
            <select v-model="newEvidence.evidence_type" class="select select-bordered">
              <option value="screenshot">{{ t('bugBounty.evidence.types.screenshot') }}</option>
              <option value="http_request">{{ t('bugBounty.evidence.types.httpRequest') }}</option>
              <option value="http_response">{{ t('bugBounty.evidence.types.httpResponse') }}</option>
              <option value="code_snippet">{{ t('bugBounty.evidence.types.codeSnippet') }}</option>
              <option value="poc">{{ t('bugBounty.evidence.types.poc') }}</option>
              <option value="video">{{ t('bugBounty.evidence.types.video') }}</option>
              <option value="log">{{ t('bugBounty.evidence.types.log') }}</option>
              <option value="other">{{ t('bugBounty.evidence.types.other') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.evidence.title') }} *</span></label>
            <input v-model="newEvidence.title" type="text" class="input input-bordered" :placeholder="t('bugBounty.evidence.titlePlaceholder')" />
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.evidence.description') }}</span></label>
            <textarea v-model="newEvidence.description" class="textarea textarea-bordered" rows="2"></textarea>
          </div>

          <!-- Conditional fields based on type -->
          <div v-if="['screenshot', 'video'].includes(newEvidence.evidence_type)" class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.evidence.fileUrl') }}</span></label>
            <input v-model="newEvidence.file_url" type="text" class="input input-bordered" placeholder="https://..." />
          </div>

          <div v-if="['code_snippet', 'poc', 'log', 'other'].includes(newEvidence.evidence_type)" class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.evidence.content') }}</span></label>
            <textarea v-model="newEvidence.content" class="textarea textarea-bordered font-mono text-sm" rows="6"></textarea>
          </div>

          <div v-if="newEvidence.evidence_type === 'http_request'" class="form-control">
            <label class="label"><span class="label-text">HTTP Request (Raw)</span></label>
            <textarea v-model="httpRequestRaw" class="textarea textarea-bordered font-mono text-sm" rows="8" placeholder="GET /api/users HTTP/1.1&#10;Host: example.com&#10;..."></textarea>
          </div>

          <div v-if="newEvidence.evidence_type === 'http_response'" class="form-control">
            <label class="label"><span class="label-text">HTTP Response (Raw)</span></label>
            <textarea v-model="httpResponseRaw" class="textarea textarea-bordered font-mono text-sm" rows="8" placeholder="HTTP/1.1 200 OK&#10;Content-Type: application/json&#10;..."></textarea>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showAddEvidenceModal = false" class="btn">{{ t('common.cancel') }}</button>
          <button 
            @click="createEvidence" 
            class="btn btn-primary"
            :disabled="!newEvidence.title || creatingEvidence"
          >
            <span v-if="creatingEvidence" class="loading loading-spinner loading-sm mr-2"></span>
            {{ t('common.create') }}
          </button>
        </div>
      </div>
    </div>
  </div>
    </Transition>
  </Teleport>
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
  finding: any
  programs: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
  (e: 'create-submission', finding: any): void
}>()

// State
const activeTab = ref('details')
const loadingEvidence = ref(false)
const creatingEvidence = ref(false)
const showAddEvidenceModal = ref(false)
const evidenceList = ref<any[]>([])
const httpRequestRaw = ref('')
const httpResponseRaw = ref('')

const newEvidence = reactive({
  evidence_type: 'screenshot',
  title: '',
  description: '',
  file_url: '',
  content: '',
})

// Computed
const programName = computed(() => {
  const program = props.programs.find(p => p.id === props.finding?.program_id)
  return program?.name || '-'
})

const reproductionSteps = computed(() => {
  if (!props.finding?.reproduction_steps_json) return []
  try {
    return JSON.parse(props.finding.reproduction_steps_json)
  } catch {
    return []
  }
})

const tags = computed(() => {
  if (!props.finding?.tags_json) return []
  try {
    return JSON.parse(props.finding.tags_json)
  } catch {
    return []
  }
})

// Methods
const loadEvidence = async () => {
  if (!props.finding?.id) return
  try {
    loadingEvidence.value = true
    evidenceList.value = await invoke('bounty_list_evidence', { findingId: props.finding.id })
  } catch (error) {
    console.error('Failed to load evidence:', error)
  } finally {
    loadingEvidence.value = false
  }
}

const createEvidence = async () => {
  try {
    creatingEvidence.value = true
    const request: any = {
      finding_id: props.finding.id,
      evidence_type: newEvidence.evidence_type,
      title: newEvidence.title,
      description: newEvidence.description || null,
      file_url: newEvidence.file_url || null,
      content: newEvidence.content || null,
      file_path: null,
      mime_type: null,
      http_request: httpRequestRaw.value ? { raw: httpRequestRaw.value } : null,
      http_response: httpResponseRaw.value ? { raw: httpResponseRaw.value } : null,
      diff: null,
      tags: null,
      display_order: evidenceList.value.length,
    }
    await invoke('bounty_create_evidence', { request })
    toast.success(t('bugBounty.success.evidenceCreated'))
    showAddEvidenceModal.value = false
    resetEvidenceForm()
    await loadEvidence()
    emit('updated')
  } catch (error) {
    console.error('Failed to create evidence:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    creatingEvidence.value = false
  }
}

const deleteEvidence = async (evidence: any) => {
  if (!confirm(t('bugBounty.confirm.deleteEvidence'))) return
  try {
    await invoke('bounty_delete_evidence', { id: evidence.id })
    toast.success(t('bugBounty.success.evidenceDeleted'))
    await loadEvidence()
    emit('updated')
  } catch (error) {
    console.error('Failed to delete evidence:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const resetEvidenceForm = () => {
  newEvidence.evidence_type = 'screenshot'
  newEvidence.title = ''
  newEvidence.description = ''
  newEvidence.file_url = ''
  newEvidence.content = ''
  httpRequestRaw.value = ''
  httpResponseRaw.value = ''
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleDateString()
}

const formatDateTime = (dateStr: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString()
}

const formatJson = (jsonStr: string) => {
  try {
    return JSON.stringify(JSON.parse(jsonStr), null, 2)
  } catch {
    return jsonStr
  }
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
    new: 'badge-info',
    verified: 'badge-success',
    reported: 'badge-primary',
    duplicate: 'badge-warning',
    fixed: 'badge-neutral',
  }
  return classes[status?.toLowerCase()] || 'badge-ghost'
}

const getEvidenceTypeClass = (type: string) => {
  const classes: Record<string, string> = {
    screenshot: 'badge-info',
    http_request: 'badge-warning',
    http_response: 'badge-warning',
    code_snippet: 'badge-secondary',
    poc: 'badge-error',
    video: 'badge-accent',
    log: 'badge-ghost',
  }
  return classes[type?.toLowerCase()] || 'badge-ghost'
}

const getEvidenceTypeIcon = (type: string) => {
  const icons: Record<string, string> = {
    screenshot: 'fas fa-image',
    http_request: 'fas fa-arrow-up',
    http_response: 'fas fa-arrow-down',
    code_snippet: 'fas fa-code',
    poc: 'fas fa-flask',
    video: 'fas fa-video',
    log: 'fas fa-file-alt',
    other: 'fas fa-paperclip',
  }
  return icons[type?.toLowerCase()] || 'fas fa-paperclip'
}

// Watchers
watch(() => props.visible, async (val) => {
  if (val && props.finding) {
    activeTab.value = 'details'
    await loadEvidence()
  }
})
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .modal-box,
.modal-leave-active .modal-box {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.modal-enter-from .modal-box,
.modal-leave-to .modal-box {
  transform: scale(0.95);
  opacity: 0;
}
</style>
