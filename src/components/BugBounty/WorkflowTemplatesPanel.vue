<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div class="flex items-center gap-2">
            <h2 class="card-title">{{ t('bugBounty.workflowTemplates.title') }}</h2>
            <span class="badge badge-primary">{{ templates.length }}</span>
          </div>
          <div class="flex gap-2">
            <select v-model="filter.category" class="select select-sm select-bordered" @change="loadTemplates">
              <option value="">{{ t('bugBounty.workflowTemplates.allCategories') }}</option>
              <option v-for="category in workflowTemplateCategories" :key="category" :value="category">
                {{ t(`bugBounty.workflowTemplates.categories.${category}`) }}
              </option>
            </select>
            <button class="btn btn-sm btn-outline" @click="initBuiltinTemplates">
              <i class="fas fa-magic mr-2"></i>
              {{ t('bugBounty.workflowTemplates.initBuiltin') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="showCreateModal = true">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.workflowTemplates.create') }}
            </button>
          </div>
        </div>

        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="templates.length === 0" class="text-center py-8">
          <i class="fas fa-project-diagram text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.workflowTemplates.empty') }}</p>
          <button class="btn btn-primary btn-sm mt-4" @click="initBuiltinTemplates">
            {{ t('bugBounty.workflowTemplates.initBuiltin') }}
          </button>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div 
            v-for="template in templates" 
            :key="template.id" 
            class="card bg-base-200 hover:shadow-lg transition-shadow cursor-pointer"
            @click="viewTemplate(template)"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div class="flex items-center gap-2">
                  <i :class="getCategoryIcon(template.category)" class="text-xl"></i>
                  <div>
                    <h3 class="font-semibold">{{ template.name }}</h3>
                    <span class="badge badge-ghost badge-xs">{{ getCategoryLabel(template.category) }}</span>
                  </div>
                </div>
                <span v-if="template.is_built_in" class="badge badge-info badge-sm">
                  {{ t('bugBounty.workflowTemplates.builtIn') }}
                </span>
              </div>
              
              <p class="text-sm text-base-content/70 mt-2 line-clamp-2">
                {{ template.description || t('bugBounty.findingDetail.noDescription') }}
              </p>
              
              <div class="flex items-center justify-between mt-3 text-xs text-base-content/60">
                <div class="flex items-center gap-2">
                  <span><i class="fas fa-list mr-1"></i>{{ getStepsCount(template) }} {{ t('bugBounty.workflowTemplates.steps') }}</span>
                  <span v-if="template.estimated_duration_mins">
                    <i class="fas fa-clock mr-1"></i>~{{ template.estimated_duration_mins }}m
                  </span>
                </div>
                <div class="flex gap-1">
                  <button 
                    class="btn btn-ghost btn-xs" 
                    @click.stop="bindToProgram(template)"
                    :title="t('bugBounty.workflowTemplates.bindToProgram')"
                  >
                    <i class="fas fa-link"></i>
                  </button>
                  <button 
                    v-if="!template.is_built_in"
                    class="btn btn-ghost btn-xs text-error" 
                    @click.stop="deleteTemplate(template)"
                  >
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>

              <div v-if="getTags(template).length > 0" class="flex flex-wrap gap-1 mt-2">
                <span 
                  v-for="tag in getTags(template).slice(0, 3)" 
                  :key="tag" 
                  class="badge badge-outline badge-xs"
                >
                  {{ tag }}
                </span>
              </div>

              <!-- Bound Programs -->
              <div v-if="getBoundPrograms(template.id).length > 0" class="mt-2 pt-2 border-t border-base-300">
                <div class="flex items-center gap-1 text-xs text-base-content/60">
                  <i class="fas fa-link"></i>
                  <span>{{ t('bugBounty.workflowTemplates.boundTo') }}:</span>
                </div>
                <div class="flex flex-wrap gap-1 mt-1">
                  <span 
                    v-for="programName in getBoundPrograms(template.id).slice(0, 2)" 
                    :key="programName" 
                    class="badge badge-primary badge-xs"
                  >
                    {{ programName }}
                  </span>
                  <span 
                    v-if="getBoundPrograms(template.id).length > 2"
                    class="badge badge-ghost badge-xs"
                  >
                    +{{ getBoundPrograms(template.id).length - 2 }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Bindings Section -->
    <div v-if="selectedProgram" class="card bg-base-100 shadow-md">
      <div class="card-body">
        <h3 class="card-title text-lg">
          {{ t('bugBounty.workflowTemplates.programBindings') }}
          <span class="badge badge-sm">{{ selectedProgram.name }}</span>
        </h3>

        <div v-if="bindings.length === 0" class="text-center py-4 text-base-content/60">
          {{ t('bugBounty.workflowTemplates.noBindings') }}
        </div>

        <div v-else class="space-y-2">
          <div 
            v-for="binding in bindings" 
            :key="binding.id"
            class="flex items-center justify-between bg-base-200 p-3 rounded-lg"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="form-control">
                <input 
                  type="checkbox" 
                  class="toggle toggle-sm toggle-primary" 
                  :checked="binding.is_enabled"
                  @change="toggleBinding(binding)"
                />
              </div>
              <div class="min-w-0">
                <div class="font-medium">{{ getTemplateName(binding.workflow_template_id) }}</div>
                <div class="mt-1 flex flex-wrap gap-1">
                  <span v-if="binding.scope_id" class="badge badge-outline badge-xs">
                    {{ getScopeName(binding.program_id, binding.scope_id) }}
                  </span>
                  <span v-if="binding.auto_run_on_change" class="badge badge-success badge-xs mr-1">
                    {{ t('bugBounty.workflowTemplates.autoRun') }}
                  </span>
                  <span
                    v-for="summary in getBindingConditionSummaries(binding)"
                    :key="summary"
                    class="badge badge-ghost badge-xs"
                  >
                    {{ summary }}
                  </span>
                </div>
                <div class="text-xs text-base-content/60 mt-1">
                  <span v-if="binding.last_run_at">
                    {{ t('bugBounty.workflowTemplates.lastRun') }}: {{ formatDate(binding.last_run_at) }}
                  </span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="badge badge-ghost badge-sm">
                {{ binding.run_count }} {{ t('bugBounty.workflowTemplates.runs') }}
              </span>
              <button class="btn btn-ghost btn-xs" :title="t('common.edit')" @click="openEditBinding(binding)">
                <i class="fas fa-pen"></i>
              </button>
              <button class="btn btn-ghost btn-xs text-error" @click="deleteBinding(binding)">
                <i class="fas fa-unlink"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Create Template Modal -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showCreateModal" class="modal modal-open">
          <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.workflowTemplates.createTitle') }}</h3>
        
        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.templateName') }}</span></label>
          <input 
            v-model="createForm.name" 
            type="text" 
            class="input input-bordered" 
            :placeholder="t('bugBounty.workflowTemplates.templateNamePlaceholder')"
          />
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.category') }}</span></label>
          <select v-model="createForm.category" class="select select-bordered">
            <option v-for="category in workflowTemplateCategories" :key="category" :value="category">
              {{ t(`bugBounty.workflowTemplates.categories.${category}`) }}
            </option>
          </select>
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.description') }}</span></label>
          <textarea 
            v-model="createForm.description" 
            class="textarea textarea-bordered h-20" 
            :placeholder="t('bugBounty.workflowTemplates.descriptionPlaceholder')"
          ></textarea>
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.tags') }}</span></label>
          <input 
            v-model="createForm.tags" 
            type="text" 
            class="input input-bordered" 
            :placeholder="t('bugBounty.workflowTemplates.tagsPlaceholder')"
          />
          <label class="label">
            <span class="label-text-alt text-base-content/60">{{ t('bugBounty.workflowTemplates.tagsHint') }}</span>
          </label>
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.estimatedDuration') }}</span></label>
          <input 
            v-model.number="createForm.estimated_duration_mins" 
            type="number" 
            class="input input-bordered w-32" 
            min="1"
            :placeholder="t('bugBounty.workflowTemplates.minutes')"
          />
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeCreateModal">{{ t('common.cancel') }}</button>
          <button 
            class="btn btn-primary" 
            @click="createTemplate" 
            :disabled="!createForm.name || createLoading"
          >
            <span v-if="createLoading" class="loading loading-spinner loading-sm mr-2"></span>
            {{ t('common.create') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeCreateModal"></div>
    </div>
      </Transition>
    </Teleport>

    <!-- Bind to Program Modal -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showBindModal" class="modal modal-open">
          <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.workflowTemplates.bindToProgram') }}</h3>
        
        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.form.program') }}</span></label>
          <div class="rounded-lg border border-base-300 bg-base-200/40">
            <div class="flex items-center justify-between px-3 py-2 border-b border-base-300 text-xs text-base-content/70">
              <span>{{ t('bugBounty.workflowTemplates.programsSelected', { count: bindForm.program_ids.length }) }}</span>
              <div class="flex gap-2">
                <button type="button" class="btn btn-ghost btn-xs" @click="selectAllPrograms">
                  {{ t('bugBounty.workflowTemplates.selectAllPrograms') }}
                </button>
                <button type="button" class="btn btn-ghost btn-xs" @click="clearSelectedPrograms">
                  {{ t('common.clear') }}
                </button>
              </div>
            </div>
            <div class="max-h-64 overflow-y-auto divide-y divide-base-300">
              <label
                v-for="p in programs"
                :key="p.id"
                class="flex items-center gap-3 px-3 py-2 cursor-pointer hover:bg-base-200"
              >
                <input
                  :checked="bindForm.program_ids.includes(p.id)"
                  type="checkbox"
                  class="checkbox checkbox-primary checkbox-sm"
                  @change="toggleBindProgram(p.id, ($event.target as HTMLInputElement).checked)"
                />
                <span class="text-sm">{{ p.name }}</span>
              </label>
            </div>
          </div>
        </div>

        <div class="form-control mb-4">
          <label class="label cursor-pointer">
            <span class="label-text">{{ t('bugBounty.workflowTemplates.autoRunOnChange') }}</span>
            <input type="checkbox" v-model="bindForm.auto_run_on_change" class="checkbox checkbox-primary" />
          </label>
        </div>

        <div v-if="singleBindProgramId" class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.scope') }}</span></label>
          <select v-model="bindForm.scope_id" class="select select-bordered">
            <option value="">{{ t('bugBounty.workflowTemplates.allScopes') }}</option>
            <option v-for="scope in singleBindProgramScopes" :key="scope.id" :value="scope.id">
              {{ formatScopeOption(scope) }}
            </option>
          </select>
          <label v-if="singleBindProgramScopes.length === 0" class="label">
            <span class="label-text-alt text-base-content/60">{{ t('bugBounty.workflowTemplates.noScopes') }}</span>
          </label>
        </div>

        <div v-else-if="bindForm.program_ids.length > 1" class="alert alert-info mb-4 text-sm">
          <i class="fas fa-info-circle"></i>
          <span>{{ t('bugBounty.workflowTemplates.scopeSingleProgramHint') }}</span>
        </div>

        <div class="divider">{{ t('bugBounty.workflowTemplates.triggerConditions') }}</div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.eventTypes') }}</span></label>
          <div class="grid grid-cols-2 gap-2 rounded-lg border border-base-300 bg-base-200/40 p-3">
            <label
              v-for="option in triggerEventTypeOptions"
              :key="option.value"
              class="flex items-center gap-2 cursor-pointer text-sm"
            >
              <input
                type="checkbox"
                class="checkbox checkbox-primary checkbox-sm"
                :checked="bindForm.trigger_event_types.includes(option.value)"
                @change="toggleTriggerEventType(bindForm.trigger_event_types, option.value, ($event.target as HTMLInputElement).checked)"
              />
              <span>{{ option.label }}</span>
            </label>
          </div>
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.minSeverity') }}</span></label>
          <select v-model="bindForm.min_severity" class="select select-bordered">
            <option value="">{{ t('common.all') }}</option>
            <option v-for="option in severityOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.assetTags') }}</span></label>
          <input
            v-model="bindForm.asset_tags"
            type="text"
            class="input input-bordered"
            :placeholder="t('bugBounty.workflowTemplates.assetTagsPlaceholder')"
          />
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeBindModal">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" @click="createBinding" :disabled="bindForm.program_ids.length === 0">
            {{ t('bugBounty.workflowTemplates.bind') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeBindModal"></div>
    </div>
      </Transition>
    </Teleport>

    <!-- Edit Binding Modal -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showEditBindingModal" class="modal modal-open">
          <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('common.edit') }} {{ getTemplateName(editingBinding?.workflow_template_id || '') }}</h3>

        <div v-if="editingBinding" class="space-y-4">
          <div class="flex flex-wrap gap-2">
            <span class="badge badge-primary">{{ getProgramName(editingBinding.program_id) }}</span>
            <span class="badge badge-outline">{{ editingBinding.id }}</span>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('bugBounty.workflowTemplates.enabled') }}</span>
              <input v-model="editForm.is_enabled" type="checkbox" class="checkbox checkbox-primary" />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('bugBounty.workflowTemplates.autoRunOnChange') }}</span>
              <input v-model="editForm.auto_run_on_change" type="checkbox" class="checkbox checkbox-primary" />
            </label>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.scope') }}</span></label>
            <select v-model="editForm.scope_id" class="select select-bordered">
              <option value="">{{ t('bugBounty.workflowTemplates.allScopes') }}</option>
              <option v-for="scope in editingBindingScopes" :key="scope.id" :value="scope.id">
                {{ formatScopeOption(scope) }}
              </option>
            </select>
            <label v-if="editingBindingScopes.length === 0" class="label">
              <span class="label-text-alt text-base-content/60">{{ t('bugBounty.workflowTemplates.noScopes') }}</span>
            </label>
          </div>

          <div class="divider">{{ t('bugBounty.workflowTemplates.triggerConditions') }}</div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.eventTypes') }}</span></label>
            <div class="grid grid-cols-2 gap-2 rounded-lg border border-base-300 bg-base-200/40 p-3">
              <label
                v-for="option in triggerEventTypeOptions"
                :key="option.value"
                class="flex items-center gap-2 cursor-pointer text-sm"
              >
                <input
                  type="checkbox"
                  class="checkbox checkbox-primary checkbox-sm"
                  :checked="editForm.trigger_event_types.includes(option.value)"
                  @change="toggleTriggerEventType(editForm.trigger_event_types, option.value, ($event.target as HTMLInputElement).checked)"
                />
                <span>{{ option.label }}</span>
              </label>
            </div>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.minSeverity') }}</span></label>
            <select v-model="editForm.min_severity" class="select select-bordered">
              <option value="">{{ t('common.all') }}</option>
              <option v-for="option in severityOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.assetTags') }}</span></label>
            <input
              v-model="editForm.asset_tags"
              type="text"
              class="input input-bordered"
              :placeholder="t('bugBounty.workflowTemplates.assetTagsPlaceholder')"
            />
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeEditBindingModal">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" :disabled="savingBinding" @click="saveEditedBinding">
            <span v-if="savingBinding" class="loading loading-spinner loading-sm mr-2"></span>
            {{ t('common.save') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeEditBindingModal"></div>
    </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'
import { dialog } from '../../composables/useDialog'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  programs: any[]
  selectedProgram?: any
}>()

const emit = defineEmits<{
  (e: 'view', template: any): void
}>()

// State
const loading = ref(false)
const templates = ref<any[]>([])
const bindings = ref<any[]>([])
const allBindings = ref<any[]>([]) // All bindings for all templates
const showCreateModal = ref(false)
const showBindModal = ref(false)
const showEditBindingModal = ref(false)
const selectedTemplate = ref<any>(null)
const editingBinding = ref<any>(null)
const savingBinding = ref(false)
const scopesByProgram = ref<Record<string, any[]>>({})
const workflowTemplateCategories = ['recon', 'discovery', 'monitoring', 'risk', 'api']
const triggerEventTypeOptions = [
  { value: 'asset_discovered', label: t('bugBounty.changeEvents.types.assetDiscovered') },
  { value: 'dns_change', label: t('bugBounty.changeEvents.types.dnsChange') },
  { value: 'certificate_change', label: t('bugBounty.changeEvents.types.certificateChange') },
  { value: 'content_change', label: t('bugBounty.changeEvents.types.contentChange') },
  { value: 'technology_change', label: t('bugBounty.changeEvents.types.technologyChange') },
  { value: 'api_change', label: t('bugBounty.changeEvents.types.apiChange') },
]
const severityOptions = [
  { value: 'low', label: t('bugBounty.severity.low') },
  { value: 'medium', label: t('bugBounty.severity.medium') },
  { value: 'high', label: t('bugBounty.severity.high') },
  { value: 'critical', label: t('bugBounty.severity.critical') },
]

const filter = reactive({
  category: '',
})

const createForm = reactive({
  name: '',
  category: 'recon',
  description: '',
  tags: '',
  estimated_duration_mins: 10,
})

const createLoading = ref(false)

const bindForm = reactive({
  program_ids: [] as string[],
  scope_id: '',
  auto_run_on_change: false,
  trigger_event_types: [] as string[],
  min_severity: '',
  asset_tags: '',
})

const editForm = reactive({
  scope_id: '',
  is_enabled: true,
  auto_run_on_change: false,
  trigger_event_types: [] as string[],
  min_severity: '',
  asset_tags: '',
})

const singleBindProgramId = computed(() =>
  bindForm.program_ids.length === 1 ? bindForm.program_ids[0] : '',
)

const singleBindProgramScopes = computed(() => {
  if (!singleBindProgramId.value) return []
  return scopesByProgram.value[singleBindProgramId.value] || []
})

const editingBindingScopes = computed(() => {
  const programId = editingBinding.value?.program_id
  if (!programId) return []
  return scopesByProgram.value[programId] || []
})

// Methods
const loadTemplates = async () => {
  try {
    loading.value = true
    templates.value = await invoke('bounty_list_workflow_templates', {
      category: filter.category || null,
      isBuiltIn: null,
    })
  } catch (error) {
    console.error('Failed to load templates:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loading.value = false
  }
}

const ensureBuiltinTemplates = async () => {
  try {
    await invoke('bounty_init_builtin_templates')
  } catch (error) {
    console.error('Failed to ensure builtin templates:', error)
  }
}

const loadBindings = async () => {
  if (!props.selectedProgram) {
    bindings.value = []
    return
  }
  try {
    bindings.value = await invoke('bounty_list_workflow_bindings', {
      programId: props.selectedProgram.id,
      scopeId: null,
      isEnabled: null,
    })
  } catch (error) {
    console.error('Failed to load bindings:', error)
  }
}

const loadScopesForProgram = async (programId?: string) => {
  if (!programId || scopesByProgram.value[programId]) return
  try {
    const scopes = await invoke<any[]>('bounty_list_scopes', {
      filter: {
        program_ids: [programId],
      },
    })
    scopesByProgram.value = {
      ...scopesByProgram.value,
      [programId]: scopes,
    }
  } catch (error) {
    console.error('Failed to load program scopes:', error)
  }
}

const loadAllBindings = async () => {
  try {
    allBindings.value = await invoke('bounty_list_workflow_bindings', {
      programId: null,
      scopeId: null,
      isEnabled: null,
    })
  } catch (error) {
    console.error('Failed to load all bindings:', error)
  }
}

const initBuiltinTemplates = async () => {
  try {
    const created = await invoke('bounty_init_builtin_templates')
    if ((created as any[]).length > 0) {
      toast.success(t('bugBounty.workflowTemplates.builtinCreated', { count: (created as any[]).length }))
    } else {
      toast.info(t('bugBounty.workflowTemplates.builtinExists'))
    }
    await loadTemplates()
  } catch (error) {
    console.error('Failed to init builtin templates:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  }
}

const viewTemplate = (template: any) => {
  emit('view', template)
}

const closeCreateModal = () => {
  showCreateModal.value = false
  resetCreateForm()
}

const resetCreateForm = () => {
  createForm.name = ''
  createForm.category = 'recon'
  createForm.description = ''
  createForm.tags = ''
  createForm.estimated_duration_mins = 10
}

const createTemplate = async () => {
  if (!createForm.name.trim()) return
  
  try {
    createLoading.value = true
    const tagsArray = createForm.tags
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0)
    
    await invoke('bounty_create_workflow_template', {
      request: {
        name: createForm.name.trim(),
        category: createForm.category,
        description: createForm.description.trim() || null,
        steps: [],
        tags: tagsArray.length > 0 ? tagsArray : null,
        estimated_duration_mins: createForm.estimated_duration_mins || null,
      }
    })
    
    toast.success(t('bugBounty.workflowTemplates.createSuccess'))
    closeCreateModal()
    await loadTemplates()
  } catch (error) {
    console.error('Failed to create template:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    createLoading.value = false
  }
}

const deleteTemplate = async (template: any) => {
  try {
    await invoke('bounty_delete_workflow_template', { id: template.id })
    toast.success(t('bugBounty.workflowTemplates.deleted'))
    await loadTemplates()
  } catch (error) {
    console.error('Failed to delete template:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const bindToProgram = (template: any) => {
  selectedTemplate.value = template
  resetBindForm()
  bindForm.program_ids = props.selectedProgram?.id ? [props.selectedProgram.id] : []
  showBindModal.value = true
}

const closeBindModal = () => {
  showBindModal.value = false
  selectedTemplate.value = null
  resetBindForm()
}

const resetBindForm = () => {
  bindForm.program_ids = []
  bindForm.scope_id = ''
  bindForm.auto_run_on_change = false
  bindForm.trigger_event_types = []
  bindForm.min_severity = ''
  bindForm.asset_tags = ''
}

const createBinding = async () => {
  if (!selectedTemplate.value || bindForm.program_ids.length === 0) return

  const targetProgramIds = [...new Set(bindForm.program_ids)]
  const existingProgramIds = targetProgramIds.filter(programId => allBindings.value.some(
    b => b.workflow_template_id === selectedTemplate.value.id && b.program_id === programId,
  ))
  const pendingProgramIds = targetProgramIds.filter(programId => !existingProgramIds.includes(programId))

  if (pendingProgramIds.length === 0) {
    toast.warning(t('bugBounty.workflowTemplates.bindingExists'))
    return
  }

  try {
    await Promise.all(pendingProgramIds.map(programId => invoke('bounty_create_workflow_binding', {
      request: {
        program_id: programId,
        scope_id: targetProgramIds.length === 1 ? normalizeScopeId(bindForm.scope_id) : null,
        workflow_template_id: selectedTemplate.value.id,
        is_enabled: true,
        auto_run_on_change: bindForm.auto_run_on_change,
        trigger_conditions: buildTriggerConditionsPayload(bindForm),
      },
    })))

    toast.success(t('bugBounty.workflowTemplates.bindingCreatedCount', { count: pendingProgramIds.length }))
    if (existingProgramIds.length > 0) {
      toast.info(t('bugBounty.workflowTemplates.bindingSkippedCount', { count: existingProgramIds.length }))
    }
    closeBindModal()
    await loadBindings()
    await loadAllBindings() // Reload all bindings to update template cards
  } catch (error) {
    console.error('Failed to create binding:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  }
}

const toggleBinding = async (binding: any) => {
  try {
    await invoke('bounty_update_workflow_binding', {
      id: binding.id,
      request: buildBindingUpdateRequest(binding, {
        is_enabled: !binding.is_enabled,
      }),
    })
    await loadBindings()
    await loadAllBindings()
  } catch (error) {
    console.error('Failed to update workflow binding:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const openEditBinding = async (binding: any) => {
  editingBinding.value = binding
  await loadScopesForProgram(binding.program_id)
  populateEditForm(binding)
  showEditBindingModal.value = true
}

const closeEditBindingModal = () => {
  showEditBindingModal.value = false
  editingBinding.value = null
  resetEditForm()
}

const resetEditForm = () => {
  editForm.scope_id = ''
  editForm.is_enabled = true
  editForm.auto_run_on_change = false
  editForm.trigger_event_types = []
  editForm.min_severity = ''
  editForm.asset_tags = ''
}

const populateEditForm = (binding: any) => {
  const conditions = parseTriggerConditions(binding.trigger_conditions_json)
  editForm.scope_id = binding.scope_id || ''
  editForm.is_enabled = binding.is_enabled
  editForm.auto_run_on_change = binding.auto_run_on_change
  editForm.trigger_event_types = [...conditions.event_types]
  editForm.min_severity = conditions.min_severity
  editForm.asset_tags = conditions.asset_tags
}

const saveEditedBinding = async () => {
  if (!editingBinding.value) return
  try {
    savingBinding.value = true
    await invoke('bounty_update_workflow_binding', {
      id: editingBinding.value.id,
      request: {
        scope_id: normalizeScopeId(editForm.scope_id),
        is_enabled: editForm.is_enabled,
        auto_run_on_change: editForm.auto_run_on_change,
        trigger_conditions: buildTriggerConditionsPayload(editForm),
        schedule_cron: editingBinding.value.schedule_cron || null,
      },
    })
    toast.success(t('bugBounty.workflowTemplates.bindingUpdated'))
    closeEditBindingModal()
    await loadBindings()
    await loadAllBindings()
  } catch (error) {
    console.error('Failed to save workflow binding:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  } finally {
    savingBinding.value = false
  }
}

const deleteBinding = async (binding: any) => {
  if (!(await dialog.confirm(t('bugBounty.workflowTemplates.confirmUnbind')))) return
  try {
    await invoke('bounty_delete_workflow_binding', { id: binding.id })
    toast.success(t('bugBounty.workflowTemplates.unbindSuccess'))
    await loadBindings()
    await loadAllBindings() // Reload all bindings to update template cards
  } catch (error) {
    console.error('Failed to delete binding:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

// Helpers
const getStepsCount = (template: any) => {
  try {
    const steps = JSON.parse(template.steps_json || '[]')
    return steps.length
  } catch {
    return 0
  }
}

const getTags = (template: any) => {
  try {
    return JSON.parse(template.tags_json || '[]')
  } catch {
    return []
  }
}

const getCategoryIcon = (category: string) => {
  const icons: Record<string, string> = {
    recon: 'fas fa-search text-info',
    discovery: 'fas fa-folder-open text-warning',
    monitoring: 'fas fa-satellite-dish text-secondary',
    risk: 'fas fa-shield-alt text-error',
    vuln: 'fas fa-shield-alt text-error',
    api: 'fas fa-plug text-primary',
  }
  return icons[category] || 'fas fa-cog'
}

const getCategoryLabel = (category: string) => {
  return t(`bugBounty.workflowTemplates.categories.${category}`)
}

const toggleBindProgram = (programId: string, checked: boolean) => {
  if (checked) {
    bindForm.program_ids = [...new Set([...bindForm.program_ids, programId])]
    return
  }

  bindForm.program_ids = bindForm.program_ids.filter(id => id !== programId)
}

const toggleTriggerEventType = (values: string[], eventType: string, checked: boolean) => {
  if (checked) {
    values.push(eventType)
    return
  }

  const index = values.indexOf(eventType)
  if (index >= 0) values.splice(index, 1)
}

const selectAllPrograms = () => {
  bindForm.program_ids = props.programs.map(program => program.id)
}

const clearSelectedPrograms = () => {
  bindForm.program_ids = []
}

const getTemplateName = (templateId: string) => {
  const template = templates.value.find(t => t.id === templateId)
  return template?.name || templateId
}

const getProgramName = (programId: string) => {
  const program = props.programs.find(p => p.id === programId)
  return program?.name || programId
}

const normalizeScopeId = (scopeId: string) => {
  const trimmed = scopeId.trim()
  return trimmed ? trimmed : null
}

const parseTriggerConditions = (conditionsJson?: string | null) => {
  if (!conditionsJson) {
    return {
      event_types: [] as string[],
      min_severity: '',
      asset_tags: '',
    }
  }

  try {
    const parsed = JSON.parse(conditionsJson)
    return {
      event_types: Array.isArray(parsed?.event_types) ? parsed.event_types : [],
      min_severity: parsed?.min_severity || '',
      asset_tags: Array.isArray(parsed?.asset_tags) ? parsed.asset_tags.join(', ') : '',
    }
  } catch {
    return {
      event_types: [] as string[],
      min_severity: '',
      asset_tags: '',
    }
  }
}

const buildTriggerConditionsPayload = (form: {
  trigger_event_types: string[]
  min_severity: string
  asset_tags: string
}) => {
  const payload: Record<string, unknown> = {}
  const eventTypes = [...new Set(form.trigger_event_types)].filter(Boolean)
  const assetTags = form.asset_tags
    .split(',')
    .map(tag => tag.trim())
    .filter(Boolean)

  if (eventTypes.length > 0) payload.event_types = eventTypes
  if (form.min_severity) payload.min_severity = form.min_severity
  if (assetTags.length > 0) payload.asset_tags = assetTags

  return Object.keys(payload).length > 0 ? payload : null
}

const buildBindingUpdateRequest = (binding: any, overrides: Record<string, any> = {}) => {
  const conditions = parseTriggerConditions(binding.trigger_conditions_json)
  return {
    scope_id: binding.scope_id || null,
    is_enabled: binding.is_enabled,
    auto_run_on_change: binding.auto_run_on_change,
    trigger_conditions: buildTriggerConditionsPayload({
      trigger_event_types: conditions.event_types,
      min_severity: conditions.min_severity,
      asset_tags: conditions.asset_tags,
    }),
    schedule_cron: binding.schedule_cron || null,
    ...overrides,
  }
}

const getScopeName = (programId: string, scopeId?: string | null) => {
  if (!scopeId) return ''
  const scopes = scopesByProgram.value[programId] || []
  const scope = scopes.find(item => item.id === scopeId)
  return scope ? formatScopeOption(scope) : scopeId
}

const formatScopeOption = (scope: any) => {
  const target = scope.target || scope.id
  return scope.description ? `${target} (${scope.description})` : target
}

const formatDate = (date: string) => {
  if (!date) return '-'
  return new Date(date).toLocaleDateString()
}

const getBoundPrograms = (templateId: string) => {
  const templateBindings = allBindings.value.filter(b => b.workflow_template_id === templateId)
  const programNames = templateBindings.map(b => {
    const program = props.programs.find(p => p.id === b.program_id)
    return program?.name || b.program_id
  })
  // Remove duplicates
  return [...new Set(programNames)]
}

const getBindingConditionSummaries = (binding: any) => {
  const conditions = parseTriggerConditions(binding.trigger_conditions_json)
  const summaries: string[] = []
  if (conditions.event_types.length > 0) {
    summaries.push(`${conditions.event_types.length} ${t('bugBounty.workflowTemplates.eventTypes')}`)
  }
  if (conditions.min_severity) {
    summaries.push(`${t('bugBounty.workflowTemplates.minSeverity')}: ${conditions.min_severity}`)
  }
  if (conditions.asset_tags) {
    summaries.push(conditions.asset_tags)
  }
  return summaries
}

// Lifecycle
onMounted(async () => {
  await ensureBuiltinTemplates()
  await loadTemplates()
  await loadAllBindings()
  if (props.selectedProgram) {
    await loadBindings()
    await loadScopesForProgram(props.selectedProgram.id)
  }
})

watch(() => props.selectedProgram?.id, async () => {
  await loadBindings()
  await loadScopesForProgram(props.selectedProgram?.id)
})

watch(singleBindProgramId, async programId => {
  bindForm.scope_id = ''
  await loadScopesForProgram(programId)
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
