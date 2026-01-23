<template>
  <div v-if="visible" class="modal modal-open">
    <div class="modal-box max-w-3xl">
      <h3 class="font-bold text-lg mb-4">
        <i class="fas fa-search mr-2"></i>
        {{ t('bugBounty.monitor.discoverAssets') }}
      </h3>

      <div v-if="!discoverResult" class="space-y-4">
        <div class="alert alert-info">
          <i class="fas fa-info-circle"></i>
          <span>{{ t('bugBounty.monitor.discoverAssetsHint') }}</span>
        </div>

        <!-- Program Selection (if not pre-selected) -->
        <div v-if="!selectedProgram" class="form-control">
          <label class="label"><span class="label-text">{{ t('bugBounty.program.selectProgram') }} *</span></label>
          <select v-model="form.program_id" class="select select-bordered" :disabled="loadingPrograms">
            <option value="">{{ loadingPrograms ? t('common.loading') : t('bugBounty.program.selectProgramPlaceholder') }}</option>
            <option v-for="program in availablePrograms" :key="program.id" :value="program.id">
              {{ program.name }}
            </option>
          </select>
        </div>
        
        <!-- Show selected program info -->
        <div v-else class="alert alert-success text-sm">
          <i class="fas fa-trophy"></i>
          <div>
            <div class="font-semibold">{{ t('bugBounty.program.selectedProgram') }}</div>
            <div>{{ selectedProgram.name }}</div>
          </div>
        </div>

        <!-- Loading Schema -->
        <div v-if="loadingSchema && form.plugin_id" class="alert alert-info text-xs">
          <span class="loading loading-spinner loading-sm"></span>
          <span>{{ t('common.loading') }} Schema...</span>
        </div>

        <!-- Plugin Schema Info -->
        <div v-else-if="pluginInputSchema && form.plugin_id" class="alert alert-success text-xs">
          <i class="fas fa-check-circle"></i>
          <div class="flex-1">
            <div class="font-semibold">{{ t('bugBounty.monitor.pluginRequirements') }}</div>
            <ul class="list-disc list-inside mt-1 space-y-1">
              <li v-for="(prop, key) in pluginInputSchema.properties" :key="key">
                <strong>{{ key }}</strong>: {{ prop.description || key }}
                <span v-if="prop.required" class="text-error ml-1">({{ t('bugBounty.monitor.required') }})</span>
                <span v-if="prop.default !== undefined" class="text-base-content/60 ml-1">
                  ({{ t('bugBounty.monitor.default') }}: {{ prop.default }})
                </span>
              </li>
            </ul>
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">{{ t('bugBounty.monitor.selectPlugin') }} *</span></label>
          <select v-model="form.plugin_id" class="select select-bordered" :disabled="loadingPlugins">
            <option value="">{{ loadingPlugins ? t('common.loading') : t('bugBounty.monitor.selectPluginPlaceholder') }}</option>
            <optgroup v-for="category in pluginCategories" :key="category.name" :label="category.label">
              <option v-for="plugin in category.plugins" :key="plugin.name" :value="plugin.name">
                {{ plugin.name }}
              </option>
            </optgroup>
          </select>
          <label v-if="form.plugin_id && selectedPlugin" class="label">
            <span class="label-text-alt text-base-content/60">
              <i :class="getCategoryIcon(selectedPlugin.category)" class="mr-1"></i>
              {{ selectedPlugin.description || selectedPlugin.name }}
            </span>
          </label>
        </div>

          <!-- Dynamic input based on plugin Schema -->
          <div v-if="pluginInputSchema?.properties" class="space-y-4">
            <div v-for="(prop, key) in pluginInputSchema.properties" :key="key" class="form-control">
              <label class="label">
                <span class="label-text">
                  {{ prop.description || key }}
                  <span v-if="pluginInputSchema.required?.includes(key)" class="text-error"> *</span>
                  <span v-if="prop.default !== undefined" class="text-base-content/60 text-xs ml-2">
                    ({{ t('bugBounty.monitor.default') }}: {{ typeof prop.default === 'object' ? JSON.stringify(prop.default) : prop.default }})
                  </span>
                </span>
              </label>
              
              <!-- Boolean input -->
              <input 
                v-if="prop.type === 'boolean'"
                v-model="dynamicFormInputs[key]"
                type="checkbox"
                class="checkbox checkbox-primary"
              />
              
              <!-- Number input -->
              <input 
                v-else-if="prop.type === 'integer' || prop.type === 'number'"
                v-model.number="dynamicFormInputs[key]"
                type="number"
                class="input input-bordered"
                :placeholder="prop.default?.toString() || ''"
              />
              
              <!-- String textarea for long text -->
              <textarea 
                v-else-if="prop.type === 'string' && (String(key).includes('content') || String(key).includes('body') || String(key).includes('data'))"
                v-model="dynamicFormInputs[key]"
                class="textarea textarea-bordered h-24"
                :placeholder="prop.default?.toString() || ''"
              ></textarea>
              
              <!-- String input (default) -->
              <input 
                v-else
                v-model="dynamicFormInputs[key]"
                type="text"
                class="input input-bordered"
                :placeholder="prop.default?.toString() || ''"
              />
            </div>
          </div>

        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text">
              <i class="fas fa-database mr-2"></i>
              {{ t('bugBounty.monitor.autoImportAssets') }}
            </span>
            <input type="checkbox" v-model="form.auto_import" class="checkbox checkbox-primary" />
          </label>
          <label class="label">
            <span class="label-text-alt">{{ t('bugBounty.monitor.autoImportHint') }}</span>
          </label>
        </div>
      </div>

      <div v-else class="space-y-4">
        <!-- Discovery Results -->
        <div class="alert" :class="discoverResult.success ? 'alert-success' : 'alert-error'">
          <i :class="discoverResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
          <div>
            <div class="font-semibold">
              {{ discoverResult.success ? t('bugBounty.monitor.discoverySuccess') : t('bugBounty.monitor.discoveryFailed') }}
            </div>
            <div v-if="discoverResult.error" class="text-sm">{{ discoverResult.error }}</div>
          </div>
        </div>

        <div v-if="discoverResult.success" class="stats stats-vertical lg:stats-horizontal shadow w-full">
          <div class="stat">
            <div class="stat-title">{{ t('bugBounty.monitor.assetsDiscovered') }}</div>
            <div class="stat-value text-primary">{{ discoverResult.assets_discovered }}</div>
          </div>
          <div class="stat">
            <div class="stat-title">{{ t('bugBounty.monitor.assetsImported') }}</div>
            <div class="stat-value text-success">{{ discoverResult.assets_imported }}</div>
          </div>
          <div class="stat">
            <div class="stat-title">{{ t('bugBounty.monitor.eventsCreated') }}</div>
            <div class="stat-value text-warning">{{ discoverResult.events_created }}</div>
          </div>
        </div>

        <div v-if="discoverResult.plugin_output" class="collapse collapse-arrow bg-base-200">
          <input type="checkbox" /> 
          <div class="collapse-title font-medium">
            <i class="fas fa-code mr-2"></i>
            {{ t('bugBounty.monitor.pluginOutput') }}
          </div>
          <div class="collapse-content">
            <pre class="text-xs overflow-auto max-h-96 bg-base-300 p-4 rounded">{{ JSON.stringify(discoverResult.plugin_output, null, 2) }}</pre>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="handleClose">
          {{ discoverResult ? t('common.close') : t('common.cancel') }}
        </button>
        <button 
          v-if="!discoverResult"
          class="btn btn-primary" 
          @click="executeDiscovery" 
          :disabled="!isFormValid || discovering"
        >
          <span v-if="discovering" class="loading loading-spinner loading-sm mr-2"></span>
          <i v-else class="fas fa-search mr-2"></i>
          {{ t('bugBounty.monitor.startDiscovery') }}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" @click="handleClose"></div>
  </div>
</template>

<script setup lang="ts">
import { reactive, computed, watch, ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  visible: boolean
  selectedProgram?: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'success', result: any): void
}>()

const discovering = ref(false)
const discoverResult = ref<any>(null)
const loadingPlugins = ref(false)
const loadingPrograms = ref(false)
const availablePlugins = ref<any[]>([])
const availablePrograms = ref<any[]>([])
const pluginInputSchema = ref<any>(null)
const loadingSchema = ref(false)

interface PluginCategory {
  name: string
  label: string
  plugins: any[]
}

interface DiscoveryResult {
  success: boolean
  assets_discovered: number
  assets_imported: number
  events_created: number
  plugin_output?: any
  error?: string
}

interface PluginSchemaProperty {
  description?: string
  required?: boolean
  default?: any
  type?: string
}

const form = reactive({
  program_id: '',
  plugin_id: '',
  domain: '',
  url: '',
  urls: '',
  auto_import: true,
})

// Dynamic form inputs based on plugin schema
const dynamicFormInputs = ref<Record<string, any>>({})

const pluginCategories = computed<PluginCategory[]>(() => {
  const categories: { [key: string]: PluginCategory } = {
    recon: { name: 'recon', label: t('bugBounty.workflowTemplates.categories.recon'), plugins: [] },
  }

  availablePlugins.value.forEach(plugin => {
    const category = plugin.category?.toLowerCase() || 'recon'
    if (categories[category]) {
      categories[category].plugins.push(plugin)
    } else {
      // Fallback to recon category for any unrecognized category
      categories.recon.plugins.push(plugin)
    }
  })

  return Object.values(categories).filter(cat => cat.plugins.length > 0)
})

const selectedPlugin = computed(() => {
  return availablePlugins.value.find(p => p.name === form.plugin_id)
})

const isFormValid = computed(() => {
  // Check if program is selected (either from props or form)
  const programId = props.selectedProgram?.id || form.program_id
  if (!programId) return false
  
  if (!form.plugin_id) return false
  
  // If Schema is loaded, validate based on Schema
  if (pluginInputSchema.value?.required) {
    const required = pluginInputSchema.value.required
    
    // Check all required fields are filled in dynamicFormInputs
    for (const field of required) {
      const value = dynamicFormInputs.value[field]
      // Check if value exists and is not empty string
      if (value === undefined || value === null || value === '') {
        return false
      }
    }
    
    return true
  }
  
  // Fallback: if plugin is selected, it's valid
  return true
  
  return true
})

const getCategoryIcon = (category: string) => {
  const icons: { [key: string]: string } = {
    recon: 'fas fa-search',
    scanning: 'fas fa-radar',
    exploitation: 'fas fa-bug',
    monitoring: 'fas fa-eye',
    other: 'fas fa-tools',
  }
  return icons[category] || icons.other
}

const needsDomainInput = (plugin: any) => {
  const name = plugin.name?.toLowerCase() || ''
  const category = plugin.category?.toLowerCase() || ''
  
  return name.includes('subdomain') || 
         name.includes('enum') ||
         name.includes('dns') ||
         category === 'recon'
}

const needsUrlsInput = (plugin: any) => {
  const name = plugin.name?.toLowerCase() || ''
  
  return name.includes('http') || 
         name.includes('prober') ||
         name.includes('probe')
}

const loadPrograms = async () => {
  try {
    loadingPrograms.value = true
    const response = await invoke('bounty_list_programs') as any
    // bounty_list_programs returns an array directly, not { programs: [...] }
    availablePrograms.value = Array.isArray(response) ? response : []
    console.log('[DiscoverAssets] Loaded programs:', availablePrograms.value.length)
  } catch (error) {
    console.error('[DiscoverAssets] Failed to load programs:', error)
  } finally {
    loadingPrograms.value = false
  }
}

const loadPlugins = async () => {
  try {
    loadingPlugins.value = true
    
    // Try to load from tool server first
    const response = await invoke('list_tool_server_tools') as any[]
    
    // Filter plugins suitable for asset discovery - only show recon tools
    availablePlugins.value = response.filter(tool => {
      const category = tool.category?.toLowerCase() || ''
      
      // Only include recon (reconnaissance/discovery) tools
      return category === 'recon'
    })
    
    // Sort by category and name
    availablePlugins.value.sort((a, b) => {
      if (a.category !== b.category) {
        return (a.category || 'other').localeCompare(b.category || 'other')
      }
      return (a.name || a.id).localeCompare(b.name || b.id)
    })
    
  } catch (error) {
    console.error('[DiscoverAssets] Failed to load plugins:', error)
    // Fallback to hardcoded list if loading fails
    availablePlugins.value = [
      { id: 'subdomain_enumerator', name: 'Subdomain Enumerator', category: 'recon', description: 'Enumerate subdomains using multiple sources' },
      { id: 'http_prober', name: 'HTTP Prober', category: 'recon', description: 'Probe HTTP/HTTPS endpoints' },
      { id: 'port_monitor', name: 'Port Monitor', category: 'monitoring', description: 'Monitor open ports' },
    ]
  } finally {
    loadingPlugins.value = false
  }
}

const executeDiscovery = async () => {
  // Get program ID from props or form
  const programId = props.selectedProgram?.id || form.program_id
  if (!programId) {
    toast.error(t('bugBounty.monitor.selectProgramFirst'))
    return
  }
  
  try {
    discovering.value = true
    
    // Prepare plugin input based on schema or plugin characteristics
    let pluginInput: any = {}
    const plugin = selectedPlugin.value
    
    if (!plugin) {
      throw new Error('Plugin not found')
    }
    
    // Use dynamicFormInputs based on schema
    if (pluginInputSchema.value?.properties) {
      // Copy all user-provided inputs from dynamicFormInputs
      pluginInput = { ...dynamicFormInputs.value }
      
      // Add default values for fields not provided by user
      const schema = pluginInputSchema.value.properties
      for (const [key, prop] of Object.entries(schema)) {
        const property = prop as PluginSchemaProperty
        if (property.default !== undefined && (pluginInput[key] === undefined || pluginInput[key] === '')) {
          pluginInput[key] = property.default
        }
      }
    } else {
      // Fallback: if no schema, use empty object
      pluginInput = {}
    }
    
    const result = await invoke<DiscoveryResult>('monitor_discover_and_import_assets', {
      request: {
        program_id: programId,
        scope_id: null,
        plugin_id: form.plugin_id,
        plugin_input: pluginInput,
        auto_import: form.auto_import,
      }
    })
    
    discoverResult.value = result
    
    if (result.success) {
      if (result.assets_imported > 0) {
        toast.success(t('bugBounty.monitor.assetsImportedSuccess', { count: result.assets_imported }))
        emit('success', result)
      } else {
        toast.info(t('bugBounty.monitor.noNewAssets'))
      }
    } else {
      toast.error(result.error || t('bugBounty.errors.operationFailed'))
    }
  } catch (error: any) {
    console.error('Failed to discover assets:', error)
    toast.error(error || t('bugBounty.errors.operationFailed'))
    discoverResult.value = {
      success: false,
      error: error?.toString() || 'Unknown error',
      assets_discovered: 0,
      assets_imported: 0,
      events_created: 0,
    }
  } finally {
    discovering.value = false
  }
}

const handleClose = () => {
  emit('close')
}

const resetForm = () => {
  form.program_id = ''
  form.plugin_id = ''
  form.domain = ''
  form.url = ''
  form.urls = ''
  form.auto_import = true
  discoverResult.value = null
  pluginInputSchema.value = null
  dynamicFormInputs.value = {}
}

watch(() => props.visible, (val) => {
  if (val) {
    resetForm()
    if (availablePlugins.value.length === 0) {
      loadPlugins()
    }
    // Load programs if not pre-selected
    if (!props.selectedProgram && availablePrograms.value.length === 0) {
      loadPrograms()
    }
  }
})

// Watch plugin selection to load schema and adjust input fields
watch(() => form.plugin_id, async (pluginName) => {
  if (!pluginName) {
    pluginInputSchema.value = null
    return
  }
  
  const plugin = availablePlugins.value.find(p => p.name === pluginName)
  if (!plugin) return
  
  // Load plugin input schema from ToolServer
  try {
    loadingSchema.value = true
    const schema = await invoke('get_tool_input_schema', { toolId: pluginName }) as any
    
    if (schema && schema.properties) {
      pluginInputSchema.value = schema
      
      // Initialize dynamicFormInputs with default values
      dynamicFormInputs.value = {}
      for (const [key, prop] of Object.entries(schema.properties)) {
        const property = prop as PluginSchemaProperty
        if (property.default !== undefined) {
          dynamicFormInputs.value[key] = property.default
        }
      }
    }
  } catch (error) {
    console.error('Failed to load plugin schema:', error)
  } finally {
    loadingSchema.value = false
  }
})

onMounted(() => {
  loadPlugins()
  if (!props.selectedProgram) {
    loadPrograms()
  }
})
</script>
