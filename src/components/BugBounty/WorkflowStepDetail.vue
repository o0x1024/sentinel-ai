<template>
  <div class="space-y-4">
    <!-- Step Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-lg flex items-center justify-center" :class="stepTypeClass">
          <i :class="stepTypeIcon"></i>
        </div>
        <div>
          <h3 class="font-semibold">{{ step.name }}</h3>
          <span class="text-xs text-base-content/60">{{ step.plugin_id || step.tool_name || step.step_type }}</span>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span v-if="executionStatus" :class="statusBadgeClass">
          {{ executionStatus }}
        </span>
        <span v-if="retryCount > 0" class="badge badge-warning badge-sm">
          {{ t('bugBounty.workflow.retries') }}: {{ retryCount }}
        </span>
      </div>
    </div>

    <!-- Port Visualization -->
    <div class="grid grid-cols-2 gap-4">
      <!-- Input Ports -->
      <div class="bg-base-200 rounded-lg p-3">
        <h4 class="text-sm font-medium mb-2 flex items-center gap-2">
          <i class="fas fa-sign-in-alt text-info"></i>
          {{ t('bugBounty.workflow.inputPorts') }}
        </h4>
        <div v-if="inputPorts.length === 0" class="text-xs text-base-content/50">
          {{ t('bugBounty.workflow.noInputs') }}
        </div>
        <div v-else class="space-y-2">
          <div 
            v-for="port in inputPorts" 
            :key="port.name"
            class="flex items-center justify-between bg-base-100 rounded px-2 py-1"
          >
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full" :class="getArtifactTypeColor(port.expected_artifact_type)"></span>
              <span class="text-sm font-mono">{{ port.name }}</span>
            </div>
            <div class="flex items-center gap-1">
              <span class="badge badge-ghost badge-xs">{{ port.expected_artifact_type }}</span>
              <span v-if="port.required" class="text-error text-xs">*</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Output Ports -->
      <div class="bg-base-200 rounded-lg p-3">
        <h4 class="text-sm font-medium mb-2 flex items-center gap-2">
          <i class="fas fa-sign-out-alt text-success"></i>
          {{ t('bugBounty.workflow.outputPorts') }}
        </h4>
        <div v-if="outputPorts.length === 0" class="text-xs text-base-content/50">
          {{ t('bugBounty.workflow.noOutputs') }}
        </div>
        <div v-else class="space-y-2">
          <div 
            v-for="port in outputPorts" 
            :key="port.name"
            class="flex items-center justify-between bg-base-100 rounded px-2 py-1"
          >
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full" :class="getArtifactTypeColor(port.artifact_type)"></span>
              <span class="text-sm font-mono">{{ port.name }}</span>
            </div>
            <span class="badge badge-ghost badge-xs">{{ port.artifact_type }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Data Flow Connections -->
    <div v-if="upstreamConnections.length > 0" class="bg-base-200 rounded-lg p-3">
      <h4 class="text-sm font-medium mb-2 flex items-center gap-2">
        <i class="fas fa-project-diagram text-primary"></i>
        {{ t('bugBounty.workflow.dataFlow') }}
      </h4>
      <div class="space-y-2">
        <div 
          v-for="conn in upstreamConnections" 
          :key="conn.from_step"
          class="flex items-center gap-2 text-sm"
        >
          <span class="badge badge-outline badge-sm">{{ conn.from_step }}</span>
          <i class="fas fa-long-arrow-alt-right text-base-content/40"></i>
          <span class="font-mono text-xs bg-base-100 px-2 py-0.5 rounded">{{ conn.extract_path || '*' }}</span>
          <i class="fas fa-long-arrow-alt-right text-base-content/40"></i>
          <span class="badge badge-primary badge-sm">{{ conn.to_param }}</span>
        </div>
      </div>
    </div>

    <!-- Artifact Preview -->
    <div v-if="artifacts.length > 0" class="bg-base-200 rounded-lg p-3">
      <h4 class="text-sm font-medium mb-2 flex items-center gap-2">
        <i class="fas fa-cubes text-warning"></i>
        {{ t('bugBounty.workflow.artifacts') }}
        <span class="badge badge-sm">{{ artifacts.length }}</span>
      </h4>
      <div class="space-y-2">
        <div 
          v-for="artifact in artifacts" 
          :key="artifact.id"
          class="bg-base-100 rounded-lg p-2"
        >
          <div class="flex items-center justify-between mb-1">
            <div class="flex items-center gap-2">
              <i :class="getArtifactIcon(artifact.artifact_type)"></i>
              <span class="font-medium text-sm">{{ artifact.artifact_type }}</span>
            </div>
            <span v-if="artifact.count" class="badge badge-ghost badge-xs">
              {{ artifact.count }} {{ t('bugBounty.workflow.items') }}
            </span>
          </div>
          <!-- Preview based on type -->
          <div class="text-xs text-base-content/70 mt-1">
            <template v-if="artifact.artifact_type === 'subdomains'">
              <div class="flex flex-wrap gap-1">
                <span 
                  v-for="(sub, idx) in getPreviewItems(artifact, 'subdomains', 5)" 
                  :key="idx"
                  class="badge badge-outline badge-xs"
                >
                  {{ sub.subdomain || sub }}
                </span>
                <span v-if="artifact.count > 5" class="text-base-content/50">
                  +{{ artifact.count - 5 }} more
                </span>
              </div>
            </template>
            <template v-else-if="artifact.artifact_type === 'live_hosts'">
              <div class="space-y-0.5">
                <div 
                  v-for="(host, idx) in getPreviewItems(artifact, 'hosts', 3)" 
                  :key="idx"
                  class="flex items-center gap-2"
                >
                  <span :class="getStatusCodeClass(host.status_code)">{{ host.status_code }}</span>
                  <span class="truncate">{{ host.url }}</span>
                </div>
              </div>
            </template>
            <template v-else-if="artifact.artifact_type === 'finding'">
              <div class="flex items-center gap-2">
                <span :class="getSeverityClass(artifact.data?.severity)">{{ artifact.data?.severity || 'medium' }}</span>
                <span class="truncate">{{ artifact.data?.title }}</span>
              </div>
            </template>
            <template v-else>
              <pre class="bg-base-300 p-2 rounded text-xs overflow-auto max-h-20">{{ JSON.stringify(artifact.data, null, 2).slice(0, 200) }}...</pre>
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Step Config -->
    <div class="collapse collapse-arrow bg-base-200">
      <input type="checkbox" />
      <div class="collapse-title text-sm font-medium">
        <i class="fas fa-cog mr-2"></i>{{ t('bugBounty.workflow.stepConfig') }}
      </div>
      <div class="collapse-content">
        <pre class="bg-base-300 p-2 rounded text-xs overflow-auto max-h-40">{{ JSON.stringify(step.config, null, 2) }}</pre>
      </div>
    </div>

    <!-- Retry/Rate Limit Status -->
    <div v-if="showExecutionDetails" class="flex items-center gap-4 text-xs text-base-content/60">
      <div v-if="rateLimitStatus" class="flex items-center gap-1">
        <i class="fas fa-tachometer-alt"></i>
        <span>{{ t('bugBounty.workflow.rateLimit') }}: {{ rateLimitStatus.global_available }}/{{ rateLimitStatus.global_limit }}</span>
      </div>
      <div v-if="duration" class="flex items-center gap-1">
        <i class="fas fa-clock"></i>
        <span>{{ duration }}ms</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()

interface PortDef {
  name: string
  artifact_type: string
}

interface InputParamDef {
  name: string
  expected_artifact_type: string
  extract_path?: string
  required: boolean
}

interface DataFlowConnection {
  from_step: string
  to_param: string
  extract_path?: string
}

interface Artifact {
  id: string
  artifact_type: string
  data: any
  count?: number
}

const props = defineProps<{
  step: {
    id: string
    name: string
    step_type: string
    plugin_id?: string
    tool_name?: string
    config: any
    depends_on: string[]
  }
  executionId?: string
  stepResult?: any
  showExecutionDetails?: boolean
}>()

// State
const inputPorts = ref<InputParamDef[]>([])
const outputPorts = ref<PortDef[]>([])
const artifacts = ref<Artifact[]>([])
const rateLimitStatus = ref<any>(null)
const executionStatus = ref<string>('')
const retryCount = ref(0)
const duration = ref<number | null>(null)

// Computed
const stepTypeClass = computed(() => {
  const type = props.step.step_type
  if (type === 'plugin') return 'bg-primary/20 text-primary'
  if (type === 'tool') return 'bg-info/20 text-info'
  if (type === 'condition') return 'bg-warning/20 text-warning'
  return 'bg-base-300'
})

const stepTypeIcon = computed(() => {
  const type = props.step.step_type
  if (type === 'plugin') return 'fas fa-puzzle-piece'
  if (type === 'tool') return 'fas fa-wrench'
  if (type === 'condition') return 'fas fa-code-branch'
  return 'fas fa-cog'
})

const statusBadgeClass = computed(() => {
  switch (executionStatus.value) {
    case 'completed': return 'badge badge-success badge-sm'
    case 'running': return 'badge badge-info badge-sm'
    case 'failed': return 'badge badge-error badge-sm'
    case 'pending': return 'badge badge-ghost badge-sm'
    default: return 'badge badge-ghost badge-sm'
  }
})

const upstreamConnections = computed(() => {
  const connections: DataFlowConnection[] = []
  for (const depStep of props.step.depends_on) {
    for (const port of inputPorts.value) {
      if (port.extract_path) {
        connections.push({
          from_step: depStep,
          to_param: port.name,
          extract_path: port.extract_path,
        })
      }
    }
  }
  return connections
})

// Methods
const loadPortInfo = async () => {
  const pluginId = props.step.plugin_id || props.step.tool_name
  if (!pluginId) return

  try {
    const portInfo = await invoke<any>('bounty_get_plugin_ports', { pluginId })
    if (portInfo) {
      inputPorts.value = portInfo.input_params || []
      outputPorts.value = portInfo.output_ports || []
    }
  } catch (error) {
    console.error('Failed to load port info:', error)
  }
}

const loadRateLimitStatus = async () => {
  if (!props.showExecutionDetails) return
  try {
    rateLimitStatus.value = await invoke('bounty_get_rate_limiter_stats')
  } catch (error) {
    console.error('Failed to load rate limit status:', error)
  }
}

const processStepResult = async () => {
  if (!props.stepResult) return

  try {
    const result = await invoke<any>('bounty_process_step_output', {
      request: {
        execution_id: props.executionId || 'preview',
        step_id: props.step.id,
        step_name: props.step.name,
        plugin_id: props.step.plugin_id,
        raw_output: props.stepResult,
      }
    })
    artifacts.value = result.artifacts || []
  } catch (error) {
    console.error('Failed to process step result:', error)
  }
}

const getArtifactTypeColor = (type: string) => {
  const colors: Record<string, string> = {
    finding: 'bg-error',
    evidence: 'bg-warning',
    asset: 'bg-info',
    subdomains: 'bg-primary',
    live_hosts: 'bg-success',
    technologies: 'bg-secondary',
    endpoints: 'bg-accent',
    secrets: 'bg-error',
    directories: 'bg-info',
    raw_data: 'bg-base-content/30',
  }
  return colors[type] || 'bg-base-content/30'
}

const getArtifactIcon = (type: string) => {
  const icons: Record<string, string> = {
    finding: 'fas fa-bug text-error',
    evidence: 'fas fa-file-alt text-warning',
    asset: 'fas fa-server text-info',
    subdomains: 'fas fa-sitemap text-primary',
    live_hosts: 'fas fa-globe text-success',
    technologies: 'fas fa-microchip text-secondary',
    endpoints: 'fas fa-link text-accent',
    secrets: 'fas fa-key text-error',
    directories: 'fas fa-folder text-info',
    raw_data: 'fas fa-database text-base-content/50',
  }
  return icons[type] || 'fas fa-cube'
}

const getPreviewItems = (artifact: Artifact, field: string, limit: number) => {
  const data = artifact.data?.[field] || artifact.data
  if (Array.isArray(data)) {
    return data.slice(0, limit)
  }
  return []
}

const getStatusCodeClass = (code: number) => {
  if (code >= 200 && code < 300) return 'badge badge-success badge-xs'
  if (code >= 300 && code < 400) return 'badge badge-warning badge-xs'
  if (code >= 400) return 'badge badge-error badge-xs'
  return 'badge badge-ghost badge-xs'
}

const getSeverityClass = (severity: string) => {
  const classes: Record<string, string> = {
    critical: 'badge badge-error badge-xs',
    high: 'badge badge-warning badge-xs',
    medium: 'badge badge-info badge-xs',
    low: 'badge badge-success badge-xs',
    info: 'badge badge-ghost badge-xs',
  }
  return classes[severity?.toLowerCase()] || 'badge badge-ghost badge-xs'
}

// Lifecycle
onMounted(async () => {
  await loadPortInfo()
  if (props.showExecutionDetails) {
    await loadRateLimitStatus()
  }
  if (props.stepResult) {
    await processStepResult()
  }
})

watch(() => props.stepResult, async () => {
  if (props.stepResult) {
    await processStepResult()
  }
})
</script>
