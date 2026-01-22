<template>
  <div class="modal modal-open">
    <div
      ref="modalBoxEl"
      class="modal-box max-w-5xl h-[90vh] overflow-y-auto overflow-x-hidden overscroll-contain bg-base-100"
    >
      <!-- Header -->
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-3">
          <i :class="getCategoryIcon(template.category)" class="text-2xl"></i>
          <div>
            <h3 class="font-bold text-lg">{{ template.name }}</h3>
            <div class="flex items-center gap-2 text-sm text-base-content/60">
              <span class="badge badge-ghost badge-sm">{{ template.category }}</span>
              <span v-if="template.is_built_in" class="badge badge-info badge-sm">{{ t('bugBounty.workflowTemplates.builtIn') }}</span>
              <span v-if="template.estimated_duration_mins">
                <i class="fas fa-clock mr-1"></i>~{{ template.estimated_duration_mins }}m
              </span>
            </div>
          </div>
        </div>
        <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <!-- Tabs -->
      <div class="tabs tabs-boxed mb-4">
        <button type="button" class="tab" :class="{ 'tab-active': activeTab === 'overview' }" @click="activeTab = 'overview'">
          <i class="fas fa-info-circle mr-2"></i>{{ t('bugBounty.workflowTemplates.overview') }}
        </button>
        <button type="button" class="tab" :class="{ 'tab-active': activeTab === 'steps' }" @click="activeTab = 'steps'">
          <i class="fas fa-list mr-2"></i>{{ t('bugBounty.workflowTemplates.steps') }} ({{ steps.length }})
        </button>
        <button type="button" class="tab" :class="{ 'tab-active': activeTab === 'dataflow' }" @click="activeTab = 'dataflow'">
          <i class="fas fa-project-diagram mr-2"></i>{{ t('bugBounty.workflow.dataFlow') }}
        </button>
        <button type="button" class="tab" :class="{ 'tab-active': activeTab === 'run' }" @click="activeTab = 'run'">
          <i class="fas fa-play mr-2"></i>{{ t('bugBounty.workflowTemplates.run') }}
        </button>
        <button 
          v-if="executionId" 
          type="button" 
          class="tab" 
          :class="{ 'tab-active': activeTab === 'execution' }" 
          @click="activeTab = 'execution'"
        >
          <i class="fas fa-spinner mr-2" :class="{ 'fa-spin': executionStatus === 'running' }"></i>
          {{ t('bugBounty.workflow.execution') }}
          <span v-if="executionStatus === 'running'" class="badge badge-info badge-xs ml-1">{{ t('bugBounty.workflow.running') }}</span>
          <span v-else-if="executionStatus === 'completed'" class="badge badge-success badge-xs ml-1">{{ t('bugBounty.workflow.completed') }}</span>
          <span v-else-if="executionStatus === 'failed'" class="badge badge-error badge-xs ml-1">{{ t('bugBounty.workflow.failed') }}</span>
        </button>
      </div>

      <!-- Overview Tab -->
      <div v-show="activeTab === 'overview'" class="space-y-4">
        <div class="bg-base-200 rounded-lg p-4">
          <h4 class="font-medium mb-2">{{ t('bugBounty.workflowTemplates.description') }}</h4>
          <p class="text-base-content/70">{{ template.description || t('bugBounty.findingDetail.noDescription') }}</p>
        </div>

        <div v-if="getTags().length > 0" class="bg-base-200 rounded-lg p-4">
          <h4 class="font-medium mb-2">{{ t('bugBounty.workflowTemplates.tags') }}</h4>
          <div class="flex flex-wrap gap-2">
            <span v-for="tag in getTags()" :key="tag" class="badge badge-outline">{{ tag }}</span>
          </div>
        </div>

        <div class="bg-base-200 rounded-lg p-4">
          <h4 class="font-medium mb-2">{{ t('bugBounty.workflowTemplates.stepsSummary') }}</h4>
          <div v-if="steps.length === 0" class="text-base-content/50">
            {{ t('bugBounty.workflowTemplates.noSteps') }}
          </div>
          <div v-else class="space-y-2">
            <div v-for="(step, idx) in steps" :key="step.id" class="flex items-center gap-2">
              <span class="badge badge-primary badge-sm">{{ idx + 1 }}</span>
              <span class="font-medium">{{ step.name }}</span>
              <span class="text-xs text-base-content/50">{{ step.plugin_id || step.tool_name }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Steps Tab -->
      <div v-show="activeTab === 'steps'" class="space-y-4">
        <div class="flex justify-between items-center">
          <span class="text-sm text-base-content/60">
            {{ t('bugBounty.workflowTemplates.stepsHint') }}
          </span>
          <button class="btn btn-sm btn-primary" @click="showAddStepModal = true" :disabled="template.is_built_in">
            <i class="fas fa-plus mr-2"></i>{{ t('bugBounty.workflowTemplates.addStep') }}
          </button>
        </div>

        <div v-if="steps.length === 0" class="text-center py-8">
          <i class="fas fa-puzzle-piece text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.workflowTemplates.noSteps') }}</p>
          <button class="btn btn-primary btn-sm mt-4" @click="showAddStepModal = true" :disabled="template.is_built_in">
            {{ t('bugBounty.workflowTemplates.addFirstStep') }}
          </button>
        </div>

        <div v-else class="space-y-3">
          <div 
            v-for="(step, idx) in steps" 
            :key="step.id"
            class="bg-base-200 rounded-lg p-4"
          >
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-primary text-primary-content flex items-center justify-center font-bold">
                  {{ idx + 1 }}
                </div>
                <div>
                  <div class="font-medium">{{ step.name }}</div>
                  <div class="text-xs text-base-content/50">
                    <span class="badge badge-ghost badge-xs mr-1">{{ step.step_type }}</span>
                    {{ step.plugin_id || step.tool_name }}
                  </div>
                </div>
              </div>
              <div class="flex gap-1">
                <button 
                  class="btn btn-ghost btn-xs" 
                  @click="editStep(step)"
                  :disabled="template.is_built_in"
                >
                  <i class="fas fa-edit"></i>
                </button>
                <button 
                  class="btn btn-ghost btn-xs text-error" 
                  @click="removeStep(step.id)"
                  :disabled="template.is_built_in"
                >
                  <i class="fas fa-trash"></i>
                </button>
              </div>
            </div>

            <!-- Step Config Preview (collapsed by default) -->
            <div v-if="step.config && Object.keys(step.config).length > 0" class="mt-3">
              <div class="collapse collapse-arrow bg-base-300 rounded">
                <input type="checkbox" />
                <div class="collapse-title text-xs text-base-content/60 py-2 min-h-0">
                  {{ t('bugBounty.workflow.stepConfig') }} ({{ Object.keys(step.config).length }})
                </div>
                <div class="collapse-content">
                  <pre class="text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ JSON.stringify(step.config, null, 2) }}</pre>
                </div>
              </div>
            </div>

            <!-- Dependencies -->
            <div v-if="step.depends_on && step.depends_on.length > 0" class="mt-2">
              <span class="text-xs text-base-content/60">{{ t('bugBounty.workflowTemplates.dependsOn') }}: </span>
              <span v-for="dep in step.depends_on" :key="dep" class="badge badge-outline badge-xs mr-1">
                {{ getStepName(dep) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Data Flow Tab -->
      <div v-show="activeTab === 'dataflow'">
        <WorkflowDataFlow
          :steps="steps"
          :initial-inputs="{ domain: 'example.com' }"
        />
      </div>

      <!-- Run Tab -->
      <div v-show="activeTab === 'run'" class="space-y-4">
        <div class="alert alert-info">
          <i class="fas fa-info-circle"></i>
          <span>{{ t('bugBounty.workflowTemplates.runHint') }}</span>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.workflowTemplates.targetDomain') }}</span>
          </label>
          <input 
            v-model="runConfig.domain" 
            type="text" 
            class="input input-bordered" 
            placeholder="example.com"
          />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.program') }}</span>
          </label>
          <select v-model="runConfig.program_id" class="select select-bordered">
            <option value="">{{ t('bugBounty.form.selectProgram') }}</option>
            <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
        </div>

        <div class="flex justify-end">
          <button 
            class="btn btn-primary" 
            @click="runWorkflow"
            :disabled="!runConfig.domain || steps.length === 0 || running"
          >
            <span v-if="running" class="loading loading-spinner loading-sm mr-2"></span>
            <i v-else class="fas fa-play mr-2"></i>
            {{ t('bugBounty.workflowTemplates.runWorkflow') }}
          </button>
        </div>
      </div>

      <!-- Execution Tab -->
      <div v-show="activeTab === 'execution'" class="space-y-4">
        <WorkflowExecutionPanel
          v-if="executionId"
          :execution-id="executionId"
          :template="template"
          :steps="steps"
          :initial-inputs="executionInputs"
          @complete="onExecutionComplete"
          @error="onExecutionError"
        />
      </div>
    </div>
    <div class="modal-backdrop" @click="$emit('close')"></div>
  </div>

  <!-- Add/Edit Step Modal (teleported to avoid nesting/clipping) -->
  <Teleport to="body">
    <div v-if="showAddStepModal" class="modal modal-open z-[60]">
      <div class="modal-box max-w-2xl max-h-[85vh] overflow-y-auto bg-base-100" @click.stop>
        <h3 class="font-bold text-lg mb-4">
          {{ editingStep ? t('bugBounty.workflowTemplates.editStep') : t('bugBounty.workflowTemplates.addStep') }}
        </h3>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.stepName') }}</span></label>
          <input v-model="stepForm.name" type="text" class="input input-bordered" placeholder="e.g., Subdomain Enumeration" />
        </div>

        <div class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.pluginId') }}</span></label>
          <select v-model="stepForm.plugin_id" class="select select-bordered">
            <option value="">{{ t('bugBounty.workflowTemplates.selectPlugin') }}</option>
            <template v-if="availablePlugins.length > 0">
              <option v-for="plugin in availablePlugins" :key="plugin.id" :value="plugin.id">
                {{ plugin.name }}
              </option>
            </template>
            <template v-else>
              <!-- Fallback: built-in agent plugins when none loaded from DB -->
              <optgroup label="Recon">
                <option value="subdomain_enumerator">Subdomain Enumerator</option>
                <option value="http_prober">HTTP Prober</option>
                <option value="tech_fingerprinter">Tech Fingerprinter</option>
                <option value="port_monitor">Port Monitor</option>
                <option value="cert_monitor">Certificate Monitor</option>
              </optgroup>
              <optgroup label="Discovery">
                <option value="directory_bruteforcer">Directory Bruteforcer</option>
                <option value="js_analyzer">JS Analyzer</option>
                <option value="api_monitor">API Monitor</option>
                <option value="content_monitor">Content Monitor</option>
              </optgroup>
              <optgroup label="Vulnerability">
                <option value="xss_scanner">XSS Scanner</option>
                <option value="sql_injection_scanner">SQL Injection Scanner</option>
                <option value="ssrf_detector">SSRF Detector</option>
                <option value="open_redirect_detector">Open Redirect Detector</option>
                <option value="cors_misconfiguration">CORS Misconfiguration</option>
                <option value="subdomain_takeover">Subdomain Takeover</option>
                <option value="nextjs_rce_scanner">Next.js RCE Scanner</option>
              </optgroup>
              <optgroup label="Utility">
                <option value="hash_calculator">Hash Calculator</option>
                <option value="url_encoder">URL Encoder</option>
              </optgroup>
            </template>
          </select>
          <label v-if="availablePlugins.length === 0" class="label">
            <span class="label-text-alt text-warning">{{ t('bugBounty.workflowTemplates.noPluginsHint') }}</span>
          </label>
        </div>

        <div v-if="steps.length > 0" class="form-control mb-4">
          <label class="label"><span class="label-text">{{ t('bugBounty.workflowTemplates.dependsOn') }}</span></label>
          <div class="flex flex-wrap gap-2">
            <label v-for="step in steps.filter(s => s.id !== editingStep?.id)" :key="step.id" class="cursor-pointer flex items-center gap-2">
              <input
                type="checkbox"
                class="checkbox checkbox-sm checkbox-primary"
                :checked="stepForm.depends_on.includes(step.id)"
                @change="toggleDependency(step.id)"
              />
              <span class="text-sm">{{ step.name }}</span>
            </label>
          </div>
        </div>

        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.workflow.stepConfig') }} (JSON)</span>
            <span v-if="loadingSchema" class="loading loading-spinner loading-xs"></span>
          </label>
          <textarea
            v-model="stepForm.config_json"
            class="textarea textarea-bordered font-mono text-sm h-32"
            placeholder='{"timeout": 30, "threads": 10}'
            :disabled="loadingSchema"
          ></textarea>
          <label class="label">
            <span class="label-text-alt text-base-content/50">{{ t('bugBounty.workflowTemplates.configAutoFillHint') }}</span>
          </label>
        </div>

        <!-- Input Mappings -->
        <div v-if="getUpstreamSteps().length > 0" class="form-control mb-4">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.workflow.inputMappings') }}</span>
            <button type="button" class="btn btn-xs btn-ghost" @click="addInputMapping">
              <i class="fas fa-plus mr-1"></i>{{ t('common.add') }}
            </button>
          </label>
          <div v-if="stepForm.input_mappings.length === 0" class="text-sm text-base-content/50 py-2">
            {{ t('bugBounty.workflow.noMappingsHint') }}
          </div>
          <div v-for="(mapping, idx) in stepForm.input_mappings" :key="idx" 
               class="bg-base-200 rounded-lg p-3 mb-2">
            <div class="grid grid-cols-1 md:grid-cols-4 gap-2 items-end">
              <!-- Target Field -->
              <div>
                <label class="label py-0"><span class="label-text text-xs">{{ t('bugBounty.workflow.targetField') }}</span></label>
                <select v-model="mapping.target_field" class="select select-sm select-bordered w-full">
                  <option value="">{{ t('bugBounty.workflow.selectField') }}</option>
                  <option v-for="field in getCurrentInputFields()" :key="field.name" :value="field.name">
                    {{ field.name }} ({{ field.type }})
                  </option>
                </select>
              </div>
              
              <!-- Source Step -->
              <div>
                <label class="label py-0"><span class="label-text text-xs">{{ t('bugBounty.workflow.sourceStep') }}</span></label>
                <select v-model="mapping.source_step_id" class="select select-sm select-bordered w-full"
                        @change="onSourceStepChange(mapping)">
                  <option value="">{{ t('bugBounty.workflow.selectStep') }}</option>
                  <option v-for="step in getUpstreamSteps()" :key="step.id" :value="step.id">
                    {{ step.name }}
                  </option>
                </select>
              </div>
              
              <!-- Source Path -->
              <div>
                <label class="label py-0"><span class="label-text text-xs">{{ t('bugBounty.workflow.sourcePath') }}</span></label>
                <select v-model="mapping.source_path" class="select select-sm select-bordered w-full">
                  <option value="">{{ t('bugBounty.workflow.selectPath') }}</option>
                  <option v-for="path in getOutputPaths(mapping.source_step_id)" :key="path.path" :value="path.path">
                    {{ path.path }} ({{ path.type }})
                  </option>
                </select>
              </div>
              
              <!-- Transform + Delete -->
              <div class="flex gap-2">
                <div class="flex-1">
                  <label class="label py-0"><span class="label-text text-xs">{{ t('bugBounty.workflow.transform') }}</span></label>
                  <select v-model="mapping.transform" class="select select-sm select-bordered w-full">
                    <option value="">{{ t('bugBounty.workflow.noTransform') }}</option>
                    <option value="first">{{ t('bugBounty.workflow.transformFirst') }}</option>
                    <option value="flatten">{{ t('bugBounty.workflow.transformFlatten') }}</option>
                    <option value="map:url">map:url</option>
                    <option value="map:subdomain">map:subdomain</option>
                  </select>
                </div>
                <button type="button" class="btn btn-sm btn-ghost btn-error self-end" @click="removeInputMapping(idx)">
                  <i class="fas fa-trash"></i>
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeStepModal">{{ t('common.cancel') }}</button>
          <button
            class="btn btn-primary"
            @click="saveStep"
            :disabled="!stepForm.name || !stepForm.plugin_id"
          >
            {{ t('common.save') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeStepModal"></div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'
import WorkflowDataFlow from './WorkflowDataFlow.vue'
import WorkflowExecutionPanel from './WorkflowExecutionPanel.vue'

const { t } = useI18n()
const toast = useToast()

interface InputMapping {
  target_field: string
  source_step_id: string
  source_path: string
  transform?: string
}

interface WorkflowStep {
  id: string
  name: string
  step_type: string
  plugin_id?: string
  tool_name?: string
  config: any
  depends_on: string[]
  input_mappings?: InputMapping[]
}

const props = defineProps<{
  template: any
  programs: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

// State
const activeTab = ref('overview')
const steps = ref<WorkflowStep[]>([])
const showAddStepModal = ref(false)
const editingStep = ref<WorkflowStep | null>(null)
const availablePlugins = ref<any[]>([])
const running = ref(false)
const modalBoxEl = ref<HTMLElement | null>(null)

// Execution state
const executionId = ref<string | null>(null)
const executionStatus = ref<'pending' | 'running' | 'completed' | 'failed' | 'cancelled'>('pending')
const executionInputs = ref<any>(null)

const stepForm = reactive({
  name: '',
  plugin_id: '',
  depends_on: [] as string[],
  config_json: '{}',
  input_mappings: [] as InputMapping[],
})

const runConfig = reactive({
  domain: '',
  program_id: '',
})

const loadingSchema = ref(false)
const pluginSchemaCache = ref<Record<string, any>>({})

// Generate default config from JSON Schema (show all fields)
const generateDefaultFromSchema = (schema: any): any => {
  if (!schema || typeof schema !== 'object') return {}
  
  const properties = schema.properties || {}
  const requiredFields = schema.required || []
  const result: Record<string, any> = {}
  
  for (const [key, prop] of Object.entries(properties) as [string, any][]) {
    // Use default value if specified
    if (prop.default !== undefined) {
      result[key] = prop.default
      continue
    }
    
    // Generate value based on type (always show all fields)
    switch (prop.type) {
      case 'string':
        if (prop.enum && prop.enum.length > 0) {
          result[key] = prop.enum[0]
        } else if (requiredFields.includes(key)) {
          // Required string field - show empty placeholder
          result[key] = ''
        } else {
          result[key] = ''
        }
        break
      case 'number':
      case 'integer':
        if (prop.minimum !== undefined) {
          result[key] = prop.minimum
        } else if (key.includes('timeout')) {
          result[key] = 30
        } else if (key.includes('concurrency') || key.includes('threads')) {
          result[key] = 10
        } else if (key.includes('port')) {
          result[key] = 80
        } else if (key.includes('max') || key.includes('limit')) {
          result[key] = 100
        } else {
          result[key] = 0
        }
        break
      case 'boolean':
        result[key] = true
        break
      case 'array':
        if (prop.items?.enum) {
          // Show first few enum values as example
          result[key] = prop.items.enum.slice(0, 3)
        } else if (prop.items?.type === 'string') {
          result[key] = []
        } else if (prop.items?.type === 'number' || prop.items?.type === 'integer') {
          result[key] = []
        } else {
          result[key] = []
        }
        break
      case 'object':
        result[key] = {}
        break
      default:
        // Unknown type, show null
        result[key] = null
        break
    }
  }
  
  return result
}

// Fetch plugin input schema
const fetchPluginSchema = async (pluginId: string): Promise<any> => {
  // Check cache first
  if (pluginSchemaCache.value[pluginId]) {
    return pluginSchemaCache.value[pluginId]
  }
  
  try {
    const response = await invoke('get_plugin_input_schema', { pluginId }) as { success: boolean; data?: any }
    if (response.success && response.data) {
      pluginSchemaCache.value[pluginId] = response.data
      return response.data
    }
  } catch (error) {
    console.error(`Failed to fetch schema for plugin ${pluginId}:`, error)
  }
  
  return null
}

// Output schema cache
const pluginOutputSchemaCache = ref<Record<string, any>>({})

// Fetch plugin output schema
const fetchPluginOutputSchema = async (pluginId: string): Promise<any> => {
  // Check cache first
  if (pluginOutputSchemaCache.value[pluginId]) {
    return pluginOutputSchemaCache.value[pluginId]
  }
  
  try {
    const response = await invoke('get_plugin_output_schema', { pluginId }) as { success: boolean; data?: any }
    if (response.success && response.data) {
      pluginOutputSchemaCache.value[pluginId] = response.data
      return response.data
    }
  } catch (error) {
    console.error(`Failed to fetch output schema for plugin ${pluginId}:`, error)
  }
  
  return null
}

// Extract available paths from JSON Schema for input mapping UI
const extractSchemaPaths = (schema: any, prefix = '$'): Array<{path: string, type: string, description?: string}> => {
  const paths: Array<{path: string, type: string, description?: string}> = []
  
  if (!schema?.properties) return paths
  
  for (const [key, prop] of Object.entries(schema.properties) as [string, any][]) {
    const currentPath = `${prefix}.${key}`
    paths.push({ 
      path: currentPath, 
      type: prop.type || 'any',
      description: prop.description
    })
    
    if (prop.type === 'object' && prop.properties) {
      paths.push(...extractSchemaPaths(prop, currentPath))
    }
    if (prop.type === 'array') {
      paths.push({ path: `${currentPath}[*]`, type: 'array', description: `${prop.description || key} (all items)` })
      if (prop.items?.properties) {
        for (const [itemKey, itemProp] of Object.entries(prop.items.properties) as [string, any][]) {
          paths.push({ 
            path: `${currentPath}[*].${itemKey}`, 
            type: itemProp.type || 'any',
            description: itemProp.description
          })
        }
      }
    }
  }
  return paths
}

// Methods
const parseSteps = () => {
  try {
    steps.value = JSON.parse(props.template.steps_json || '[]')
  } catch {
    steps.value = []
  }
}

const getTags = () => {
  try {
    return JSON.parse(props.template.tags_json || '[]')
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

const getStepName = (stepId: string) => {
  const step = steps.value.find(s => s.id === stepId)
  return step?.name || stepId
}

const loadPlugins = async () => {
  try {
    // list_plugins returns CommandResponse<Vec<PluginRecord>>, extract .data
    const response = await invoke('list_plugins') as { success: boolean; data?: any[]; error?: string }
    if (response.success && Array.isArray(response.data)) {
      // Filter agent plugins (main_category === 'agent') and ensure valid metadata
      availablePlugins.value = response.data
        .filter(p => p?.metadata?.id && p?.metadata?.main_category === 'agent')
        .map(p => ({
          id: p.metadata.id,
          name: p.metadata.name || p.metadata.id,
          category: p.metadata.category,
          description: p.metadata.description,
        }))
    } else {
      availablePlugins.value = []
    }
  } catch (error) {
    console.error('Failed to load plugins:', error)
    availablePlugins.value = []
  }
}

const editStep = async (step: WorkflowStep) => {
  editingStep.value = step
  stepForm.name = step.name
  // Compatible with old data: use plugin_id or tool_name
  stepForm.plugin_id = step.plugin_id || step.tool_name || ''
  stepForm.depends_on = [...(step.depends_on || [])]
  stepForm.config_json = JSON.stringify(step.config || {}, null, 2)
  // Deep copy input_mappings to ensure they are editable
  stepForm.input_mappings = (step.input_mappings || []).map(m => ({
    target_field: m.target_field || '',
    source_step_id: m.source_step_id || '',
    source_path: m.source_path || '',
    transform: m.transform || ''
  }))
  
  // Load current plugin's input schema for target fields
  if (stepForm.plugin_id) {
    fetchPluginSchema(stepForm.plugin_id)
  }
  
  // Pre-load output schemas for upstream steps
  for (const depId of stepForm.depends_on) {
    const depStep = steps.value.find(s => s.id === depId)
    if (depStep?.plugin_id) {
      loadUpstreamOutputSchema(depStep.plugin_id)
    }
  }
  
  showAddStepModal.value = true
}

const closeStepModal = () => {
  showAddStepModal.value = false
  editingStep.value = null
  stepForm.name = ''
  stepForm.plugin_id = ''
  stepForm.depends_on = []
  stepForm.config_json = '{}'
  stepForm.input_mappings = []
}

const toggleDependency = (stepId: string) => {
  const idx = stepForm.depends_on.indexOf(stepId)
  if (idx >= 0) {
    stepForm.depends_on.splice(idx, 1)
  } else {
    stepForm.depends_on.push(stepId)
  }
}

// Input mapping helper methods
const upstreamOutputSchemas = ref<Record<string, any>>({})

const getUpstreamSteps = () => {
  // Return all steps that are before current step (based on depends_on or all previous steps)
  const currentStepId = editingStep.value?.id
  return steps.value.filter(s => s.id !== currentStepId && stepForm.depends_on.includes(s.id))
}

const getCurrentInputFields = () => {
  const pluginId = stepForm.plugin_id
  if (!pluginId) return []
  
  const schema = pluginSchemaCache.value[pluginId]
  if (!schema?.properties) return []
  
  return Object.entries(schema.properties).map(([name, prop]: [string, any]) => ({
    name,
    type: prop.type || 'any',
    description: prop.description
  }))
}

const getOutputPaths = (sourceStepId: string): Array<{path: string, type: string}> => {
  if (!sourceStepId) return []
  
  const step = steps.value.find(s => s.id === sourceStepId)
  if (!step?.plugin_id) return []
  
  const schema = upstreamOutputSchemas.value[step.plugin_id]
  if (!schema) {
    // Trigger async load
    loadUpstreamOutputSchema(step.plugin_id)
    return []
  }
  
  return extractSchemaPaths(schema)
}

const loadUpstreamOutputSchema = async (pluginId: string) => {
  if (upstreamOutputSchemas.value[pluginId]) return
  
  const schema = await fetchPluginOutputSchema(pluginId)
  if (schema) {
    upstreamOutputSchemas.value[pluginId] = schema
  }
}

const onSourceStepChange = (mapping: InputMapping) => {
  mapping.source_path = ''
  const step = steps.value.find(s => s.id === mapping.source_step_id)
  if (step?.plugin_id) {
    loadUpstreamOutputSchema(step.plugin_id)
  }
}

const addInputMapping = () => {
  stepForm.input_mappings.push({
    target_field: '',
    source_step_id: '',
    source_path: '',
    transform: ''
  })
}

const removeInputMapping = (idx: number) => {
  stepForm.input_mappings.splice(idx, 1)
}

const saveStep = async () => {
  let config = {}
  try {
    config = JSON.parse(stepForm.config_json || '{}')
  } catch {
    toast.error(t('bugBounty.workflowTemplates.invalidJson'))
    return
  }

  // Filter out incomplete mappings and clean up empty transform
  const validMappings = stepForm.input_mappings
    .filter(m => m.target_field && m.source_step_id && m.source_path)
    .map(m => ({
      target_field: m.target_field,
      source_step_id: m.source_step_id,
      source_path: m.source_path,
      transform: m.transform || undefined, // Convert empty string to undefined
    }))
  
  console.log('saveStep - stepForm.input_mappings:', stepForm.input_mappings)
  console.log('saveStep - validMappings:', validMappings)

  const newStep: WorkflowStep = {
    id: editingStep.value?.id || `step_${Date.now()}`,
    name: stepForm.name,
    step_type: 'plugin',
    plugin_id: stepForm.plugin_id,
    config,
    depends_on: stepForm.depends_on,
    input_mappings: validMappings.length > 0 ? validMappings : undefined,
  }

  if (editingStep.value) {
    const idx = steps.value.findIndex(s => s.id === editingStep.value!.id)
    if (idx >= 0) {
      steps.value[idx] = newStep
    }
  } else {
    steps.value.push(newStep)
  }

  await saveTemplate()
  closeStepModal()
}

const removeStep = async (stepId: string) => {
  if (!confirm(t('bugBounty.workflowTemplates.confirmDeleteStep'))) return
  
  steps.value = steps.value.filter(s => s.id !== stepId)
  // Remove from dependencies
  steps.value.forEach(s => {
    s.depends_on = s.depends_on.filter(d => d !== stepId)
  })
  await saveTemplate()
}

const saveTemplate = async () => {
  try {
    console.log('Saving steps with input_mappings:', JSON.stringify(steps.value, null, 2))
    await invoke('bounty_update_workflow_template', {
      id: props.template.id,
      request: {
        name: props.template.name,
        description: props.template.description,
        category: props.template.category,
        steps: steps.value,
        tags: getTags(),
        estimated_duration_mins: props.template.estimated_duration_mins,
      }
    })
    toast.success(t('bugBounty.workflowTemplates.saved'))
    emit('updated')
  } catch (error) {
    console.error('Failed to save template:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const runWorkflow = async () => {
  if (!runConfig.domain || steps.value.length === 0) return

  running.value = true
  try {
    const inputs = {
      domain: runConfig.domain,
      url: `https://${runConfig.domain}`,
    }
    // Backend returns execution_id as string directly
    const execId = await invoke('bounty_run_workflow_template', {
      templateId: props.template.id,
      programId: runConfig.program_id || null,
      inputs,
    }) as string
    
    // Store execution state
    executionId.value = execId
    executionStatus.value = 'running'
    executionInputs.value = inputs
    
    toast.success(t('bugBounty.workflowTemplates.runStarted'))
    
    // Switch to execution tab
    activeTab.value = 'execution'
  } catch (error) {
    console.error('Failed to run workflow:', error)
    toast.error(t('bugBounty.errors.runFailed'))
  } finally {
    running.value = false
  }
}

const onExecutionComplete = (result: any) => {
  executionStatus.value = 'completed'
  toast.success(t('bugBounty.workflow.executionCompleted'))
}

const onExecutionError = (error: any) => {
  executionStatus.value = 'failed'
  toast.error(t('bugBounty.workflow.executionFailed'))
}

// Lifecycle
onMounted(() => {
  parseSteps()
  loadPlugins()
})

// Re-parse steps when template data changes (e.g., after save and refresh)
watch(() => props.template.steps_json, () => {
  parseSteps()
}, { deep: true })

watch(activeTab, () => {
  // Prevent scroll jump/overscroll bounce causing a flash on tab switch.
  modalBoxEl.value?.scrollTo({ top: 0 })
})

// Auto-fill config when plugin is selected, and always load schema for input mapping
watch(() => stepForm.plugin_id, async (newPluginId) => {
  if (!newPluginId) return
  
  loadingSchema.value = true
  try {
    // Fetch schema from backend (always load for input mapping target fields)
    const schema = await fetchPluginSchema(newPluginId)
    
    // Only auto-fill config for new steps, not editing
    if (schema && !editingStep.value) {
      const defaultConfig = generateDefaultFromSchema(schema)
      if (Object.keys(defaultConfig).length > 0) {
        stepForm.config_json = JSON.stringify(defaultConfig, null, 2)
      }
    }
    
    // Auto-fill step name from plugin metadata or schema title (only for new steps)
    if (!stepForm.name && !editingStep.value) {
      const plugin = availablePlugins.value.find(p => p.id === newPluginId)
      if (plugin?.name) {
        stepForm.name = plugin.name
      } else {
        // Convert plugin_id to readable name: snake_case -> Title Case
        stepForm.name = newPluginId
          .split('_')
          .map(word => word.charAt(0).toUpperCase() + word.slice(1))
          .join(' ')
      }
    }
  } finally {
    loadingSchema.value = false
  }
})
</script>
