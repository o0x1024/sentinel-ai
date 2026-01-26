<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal modal-open">
        <div class="modal-box max-w-4xl h-[80vh] flex flex-col">
      <!-- Header -->
      <div class="flex justify-between items-start mb-4">
        <div>
          <h3 class="font-bold text-xl">{{ program?.name }}</h3>
          <p class="text-base-content/70">{{ program?.organization }}</p>
          <div class="flex gap-2 mt-2">
            <span class="badge badge-primary">{{ program?.platform }}</span>
            <span class="badge" :class="getStatusClass(program?.status)">{{ program?.status }}</span>
            <a v-if="program?.url" :href="program.url" target="_blank" class="badge badge-ghost gap-1">
              <i class="fas fa-external-link-alt text-xs"></i>
              {{ t('bugBounty.programDetail.openUrl') }}
            </a>
          </div>
        </div>
        <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-4 gap-4 mb-4">
        <div class="stat bg-base-200 rounded-lg py-2">
          <div class="stat-title text-xs">{{ t('bugBounty.programDetail.scopes') }}</div>
          <div class="stat-value text-lg">{{ scopes.length }}</div>
        </div>
        <div class="stat bg-base-200 rounded-lg py-2">
          <div class="stat-title text-xs">{{ t('bugBounty.programDetail.findings') }}</div>
          <div class="stat-value text-lg">{{ programFindings.length }}</div>
        </div>
        <div class="stat bg-base-200 rounded-lg py-2">
          <div class="stat-title text-xs">{{ t('bugBounty.programs.submissions') }}</div>
          <div class="stat-value text-lg">{{ program?.total_submissions || 0 }}</div>
        </div>
        <div class="stat bg-base-200 rounded-lg py-2">
          <div class="stat-title text-xs">{{ t('bugBounty.programs.earnings') }}</div>
          <div class="stat-value text-lg">${{ (program?.total_earnings || 0).toFixed(0) }}</div>
        </div>
      </div>

      <!-- Tabs -->
      <div class="tabs tabs-boxed mb-4">
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'scopes' }" @click="activeTab = 'scopes'">
          <i class="fas fa-bullseye mr-2"></i>
          {{ t('bugBounty.programDetail.scopeTab') }}
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'findings' }" @click="activeTab = 'findings'">
          <i class="fas fa-bug mr-2"></i>
          {{ t('bugBounty.programDetail.findingsTab') }}
        </a>
        <a class="tab tab-sm" :class="{ 'tab-active': activeTab === 'info' }" @click="activeTab = 'info'">
          <i class="fas fa-info-circle mr-2"></i>
          {{ t('bugBounty.programDetail.infoTab') }}
        </a>
      </div>

      <!-- Tab Content -->
      <div class="flex-1 overflow-auto">
        <!-- Scopes Tab -->
        <div v-if="activeTab === 'scopes'" class="space-y-4">
          <div class="flex justify-between items-center">
            <h4 class="font-semibold">{{ t('bugBounty.programDetail.scopeList') }}</h4>
            <button class="btn btn-sm btn-primary" @click="showCreateScopeModal = true">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.programDetail.addScope') }}
            </button>
          </div>

          <div v-if="loadingScopes" class="flex justify-center py-8">
            <span class="loading loading-spinner"></span>
          </div>

          <div v-else-if="scopes.length === 0" class="text-center py-8 text-base-content/70">
            <i class="fas fa-bullseye text-4xl mb-4 opacity-30"></i>
            <p>{{ t('bugBounty.programDetail.noScopes') }}</p>
          </div>

          <div v-else class="space-y-2">
            <!-- In Scope -->
            <div v-if="inScopeItems.length > 0" class="collapse collapse-arrow bg-success/10 border border-success/30">
              <input type="checkbox" checked />
              <div class="collapse-title font-medium text-success">
                <i class="fas fa-check-circle mr-2"></i>
                {{ t('bugBounty.scope.inScope') }} ({{ inScopeItems.length }})
              </div>
              <div class="collapse-content">
                <div class="overflow-x-auto">
                  <table class="table table-sm">
                    <thead>
                      <tr>
                        <th>{{ t('bugBounty.scope.target') }}</th>
                        <th>{{ t('bugBounty.scope.type') }}</th>
                        <th>{{ t('bugBounty.scope.description') }}</th>
                        <th>{{ t('bugBounty.table.actions') }}</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="scope in inScopeItems" :key="scope.id">
                        <td>
                          <code class="text-sm bg-base-200 px-2 py-1 rounded">{{ scope.target }}</code>
                        </td>
                        <td>
                          <span class="badge badge-ghost badge-sm">{{ scope.target_type }}</span>
                        </td>
                        <td class="text-sm text-base-content/70 max-w-xs truncate">
                          {{ scope.description || '-' }}
                        </td>
                        <td>
                          <button class="btn btn-ghost btn-xs text-error" @click="deleteScope(scope)">
                            <i class="fas fa-trash"></i>
                          </button>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>

            <!-- Out of Scope -->
            <div v-if="outOfScopeItems.length > 0" class="collapse collapse-arrow bg-error/10 border border-error/30">
              <input type="checkbox" />
              <div class="collapse-title font-medium text-error">
                <i class="fas fa-times-circle mr-2"></i>
                {{ t('bugBounty.scope.outOfScope') }} ({{ outOfScopeItems.length }})
              </div>
              <div class="collapse-content">
                <div class="overflow-x-auto">
                  <table class="table table-sm">
                    <thead>
                      <tr>
                        <th>{{ t('bugBounty.scope.target') }}</th>
                        <th>{{ t('bugBounty.scope.type') }}</th>
                        <th>{{ t('bugBounty.scope.description') }}</th>
                        <th>{{ t('bugBounty.table.actions') }}</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="scope in outOfScopeItems" :key="scope.id">
                        <td>
                          <code class="text-sm bg-base-200 px-2 py-1 rounded">{{ scope.target }}</code>
                        </td>
                        <td>
                          <span class="badge badge-ghost badge-sm">{{ scope.target_type }}</span>
                        </td>
                        <td class="text-sm text-base-content/70 max-w-xs truncate">
                          {{ scope.description || '-' }}
                        </td>
                        <td>
                          <button class="btn btn-ghost btn-xs text-error" @click="deleteScope(scope)">
                            <i class="fas fa-trash"></i>
                          </button>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Findings Tab -->
        <div v-if="activeTab === 'findings'" class="space-y-4">
          <div v-if="programFindings.length === 0" class="text-center py-8 text-base-content/70">
            <i class="fas fa-bug text-4xl mb-4 opacity-30"></i>
            <p>{{ t('bugBounty.programDetail.noFindings') }}</p>
          </div>

          <div v-else class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>{{ t('bugBounty.table.title') }}</th>
                  <th>{{ t('bugBounty.table.type') }}</th>
                  <th>{{ t('bugBounty.table.severity') }}</th>
                  <th>{{ t('bugBounty.table.status') }}</th>
                  <th>{{ t('bugBounty.table.date') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="finding in programFindings" :key="finding.id" class="hover">
                  <td class="font-medium">{{ finding.title }}</td>
                  <td><span class="badge badge-ghost badge-sm">{{ finding.finding_type }}</span></td>
                  <td><span class="badge badge-sm" :class="getSeverityClass(finding.severity)">{{ finding.severity }}</span></td>
                  <td><span class="badge badge-sm" :class="getStatusClass(finding.status)">{{ finding.status }}</span></td>
                  <td class="text-sm">{{ formatDate(finding.created_at) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Info Tab -->
        <div v-if="activeTab === 'info'" class="space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.programName') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ program?.name }}</div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.organization') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ program?.organization }}</div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.platform') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ program?.platform }}</div>
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text font-medium">{{ t('common.status') }}</span></label>
              <div class="bg-base-200 rounded-lg px-4 py-2">{{ program?.status }}</div>
            </div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.description') }}</span></label>
            <div class="bg-base-200 rounded-lg px-4 py-2 min-h-[100px] whitespace-pre-wrap">
              {{ program?.description || t('bugBounty.programDetail.noDescription') }}
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4 text-sm text-base-content/70">
            <div>{{ t('bugBounty.programDetail.createdAt') }}: {{ formatDate(program?.created_at) }}</div>
            <div>{{ t('bugBounty.programDetail.updatedAt') }}: {{ formatDate(program?.updated_at) }}</div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="modal-action">
        <button class="btn" @click="$emit('close')">{{ t('common.close') }}</button>
      </div>
    </div>

    <!-- Create Scope Modal -->
    <div v-if="showCreateScopeModal" class="modal modal-open z-50">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.programDetail.addScope') }}</h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.scope.scopeType') }} *</span></label>
            <select v-model="newScope.scope_type" class="select select-bordered">
              <option value="in_scope">{{ t('bugBounty.scope.inScope') }}</option>
              <option value="out_of_scope">{{ t('bugBounty.scope.outOfScope') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.scope.targetType') }} *</span></label>
            <select v-model="newScope.target_type" class="select select-bordered">
              <option value="domain">{{ t('bugBounty.scope.targetTypes.domain') }}</option>
              <option value="wildcard_domain">{{ t('bugBounty.scope.targetTypes.wildcardDomain') }}</option>
              <option value="url">{{ t('bugBounty.scope.targetTypes.url') }}</option>
              <option value="ip">{{ t('bugBounty.scope.targetTypes.ip') }}</option>
              <option value="ip_range">{{ t('bugBounty.scope.targetTypes.ipRange') }}</option>
              <option value="api">{{ t('bugBounty.scope.targetTypes.api') }}</option>
              <option value="mobile_app">{{ t('bugBounty.scope.targetTypes.mobileApp') }}</option>
              <option value="other">{{ t('bugBounty.scope.targetTypes.other') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.scope.target') }} *</span></label>
            <input 
              v-model="newScope.target" 
              type="text" 
              class="input input-bordered"
              placeholder="*.example.com"
            />
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.scope.description') }}</span></label>
            <textarea 
              v-model="newScope.description" 
              class="textarea textarea-bordered"
              rows="2"
            ></textarea>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showCreateScopeModal = false" class="btn">{{ t('common.cancel') }}</button>
          <button 
            @click="createScope" 
            class="btn btn-primary"
            :disabled="!newScope.target || creatingSope"
          >
            <span v-if="creatingSope" class="loading loading-spinner loading-sm mr-2"></span>
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
  program: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

// State
const activeTab = ref('scopes')
const loadingScopes = ref(false)
const creatingSope = ref(false)
const showCreateScopeModal = ref(false)

const scopes = ref<any[]>([])
const programFindings = ref<any[]>([])

const newScope = reactive({
  scope_type: 'in_scope',
  target_type: 'domain',
  target: '',
  description: '',
})

// Computed
const inScopeItems = computed(() => scopes.value.filter(s => s.scope_type === 'in_scope'))
const outOfScopeItems = computed(() => scopes.value.filter(s => s.scope_type === 'out_of_scope'))

// Methods
const loadScopes = async () => {
  if (!props.program?.id) return
  try {
    loadingScopes.value = true
    scopes.value = await invoke('bounty_list_scopes', { 
      filter: { program_ids: [props.program.id] } 
    })
  } catch (error) {
    console.error('Failed to load scopes:', error)
  } finally {
    loadingScopes.value = false
  }
}

const loadProgramFindings = async () => {
  if (!props.program?.id) return
  try {
    programFindings.value = await invoke('bounty_list_findings', { 
      filter: { program_id: props.program.id } 
    })
  } catch (error) {
    console.error('Failed to load findings:', error)
  }
}

const createScope = async () => {
  try {
    creatingSope.value = true
    const request = {
      program_id: props.program.id,
      scope_type: newScope.scope_type,
      target_type: newScope.target_type,
      target: newScope.target,
      description: newScope.description || null,
      allowed_tests: null,
      instructions: null,
      requires_auth: null,
      priority: null,
    }
    await invoke('bounty_create_scope', { request })
    toast.success(t('bugBounty.success.scopeCreated'))
    showCreateScopeModal.value = false
    resetScopeForm()
    await loadScopes()
    emit('updated')
  } catch (error) {
    console.error('Failed to create scope:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    creatingSope.value = false
  }
}

const deleteScope = async (scope: any) => {
  try {
    await invoke('bounty_delete_scope', { id: scope.id })
    toast.success(t('bugBounty.success.scopeDeleted'))
    await loadScopes()
    emit('updated')
  } catch (error) {
    console.error('Failed to delete scope:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const resetScopeForm = () => {
  newScope.scope_type = 'in_scope'
  newScope.target_type = 'domain'
  newScope.target = ''
  newScope.description = ''
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleDateString()
}

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    active: 'badge-success',
    paused: 'badge-warning',
    ended: 'badge-neutral',
    new: 'badge-info',
    verified: 'badge-success',
    reported: 'badge-primary',
    duplicate: 'badge-warning',
    fixed: 'badge-neutral',
  }
  return classes[status?.toLowerCase()] || 'badge-ghost'
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

// Watchers
watch(() => props.visible, async (val) => {
  if (val && props.program) {
    activeTab.value = 'scopes'
    await loadScopes()
    await loadProgramFindings()
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
