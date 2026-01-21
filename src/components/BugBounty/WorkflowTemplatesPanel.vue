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
              <option value="recon">{{ t('bugBounty.workflowTemplates.categories.recon') }}</option>
              <option value="discovery">{{ t('bugBounty.workflowTemplates.categories.discovery') }}</option>
              <option value="vuln">{{ t('bugBounty.workflowTemplates.categories.vuln') }}</option>
              <option value="api">{{ t('bugBounty.workflowTemplates.categories.api') }}</option>
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
                    <span class="badge badge-ghost badge-xs">{{ template.category }}</span>
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
            <div class="flex items-center gap-3">
              <div class="form-control">
                <input 
                  type="checkbox" 
                  class="toggle toggle-sm toggle-primary" 
                  :checked="binding.is_enabled"
                  @change="toggleBinding(binding)"
                />
              </div>
              <div>
                <div class="font-medium">{{ getTemplateName(binding.workflow_template_id) }}</div>
                <div class="text-xs text-base-content/60">
                  <span v-if="binding.auto_run_on_change" class="badge badge-success badge-xs mr-1">
                    {{ t('bugBounty.workflowTemplates.autoRun') }}
                  </span>
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
              <button class="btn btn-ghost btn-xs text-error" @click="deleteBinding(binding)">
                <i class="fas fa-unlink"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Bind to Program Modal -->
    <div v-if="showBindModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.workflowTemplates.bindToProgram') }}</h3>
        
        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.form.program') }}</span></label>
          <select v-model="bindForm.program_id" class="select select-bordered">
            <option value="">{{ t('bugBounty.form.selectProgram') }}</option>
            <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
        </div>

        <div class="form-control mb-4">
          <label class="label cursor-pointer">
            <span class="label-text">{{ t('bugBounty.workflowTemplates.autoRunOnChange') }}</span>
            <input type="checkbox" v-model="bindForm.auto_run_on_change" class="checkbox checkbox-primary" />
          </label>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="showBindModal = false">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" @click="createBinding" :disabled="!bindForm.program_id">
            {{ t('bugBounty.workflowTemplates.bind') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

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
const showCreateModal = ref(false)
const showBindModal = ref(false)
const selectedTemplate = ref<any>(null)

const filter = reactive({
  category: '',
})

const bindForm = reactive({
  program_id: '',
  auto_run_on_change: false,
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

const loadBindings = async () => {
  if (!props.selectedProgram) return
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

const deleteTemplate = async (template: any) => {
  if (!confirm(t('bugBounty.workflowTemplates.confirmDelete'))) return
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
  bindForm.program_id = props.selectedProgram?.id || ''
  bindForm.auto_run_on_change = false
  showBindModal.value = true
}

const createBinding = async () => {
  if (!selectedTemplate.value || !bindForm.program_id) return
  try {
    await invoke('bounty_create_workflow_binding', {
      request: {
        program_id: bindForm.program_id,
        workflow_template_id: selectedTemplate.value.id,
        is_enabled: true,
        auto_run_on_change: bindForm.auto_run_on_change,
      }
    })
    toast.success(t('bugBounty.workflowTemplates.bindingCreated'))
    showBindModal.value = false
    await loadBindings()
  } catch (error) {
    console.error('Failed to create binding:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  }
}

const toggleBinding = async (binding: any) => {
  // TODO: Update binding enabled state
  toast.info(t('bugBounty.comingSoon'))
}

const deleteBinding = async (binding: any) => {
  if (!confirm(t('bugBounty.workflowTemplates.confirmUnbind'))) return
  try {
    await invoke('bounty_delete_workflow_binding', { id: binding.id })
    toast.success(t('bugBounty.workflowTemplates.unbindSuccess'))
    await loadBindings()
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
    vuln: 'fas fa-shield-alt text-error',
    api: 'fas fa-plug text-primary',
  }
  return icons[category] || 'fas fa-cog'
}

const getTemplateName = (templateId: string) => {
  const template = templates.value.find(t => t.id === templateId)
  return template?.name || templateId
}

const formatDate = (date: string) => {
  if (!date) return '-'
  return new Date(date).toLocaleDateString()
}

// Lifecycle
onMounted(async () => {
  await loadTemplates()
  if (props.selectedProgram) {
    await loadBindings()
  }
})
</script>
