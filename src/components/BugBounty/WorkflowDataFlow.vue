<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="font-semibold flex items-center gap-2">
        <i class="fas fa-project-diagram text-primary"></i>
        {{ t('bugBounty.workflow.dataFlowVisualization') }}
      </h3>
      <div class="flex items-center gap-2">
        <button 
          class="btn btn-ghost btn-xs"
          :class="{ 'btn-active': showLabels }"
          @click="showLabels = !showLabels"
        >
          <i class="fas fa-tags"></i>
        </button>
        <button 
          class="btn btn-ghost btn-xs"
          @click="expandAll = !expandAll"
        >
          <i :class="expandAll ? 'fas fa-compress-alt' : 'fas fa-expand-alt'"></i>
        </button>
      </div>
    </div>

    <!-- Flow Diagram -->
    <div class="relative bg-base-200 rounded-lg p-4 overflow-auto">
      <div class="flex flex-col gap-6">
        <!-- Workflow Input (Initial Inputs) -->
        <div class="flex items-center justify-center">
          <div class="bg-primary/20 border-2 border-primary border-dashed rounded-lg px-4 py-2">
            <div class="text-sm font-medium text-primary flex items-center gap-2">
              <i class="fas fa-play-circle"></i>
              {{ t('bugBounty.workflow.initialInput') }}
            </div>
            <div v-if="initialInputs" class="text-xs text-base-content/60 mt-1">
              {{ Object.keys(initialInputs).join(', ') || 'domain, url...' }}
            </div>
          </div>
        </div>

        <!-- Connection Line -->
        <div class="flex justify-center">
          <div class="w-0.5 h-6 bg-base-content/20"></div>
        </div>

        <!-- Steps with Data Flow -->
        <div 
          v-for="(layer, layerIdx) in stepLayers" 
          :key="layerIdx"
          class="space-y-4"
        >
          <!-- Layer Steps (Parallel) -->
          <div class="flex flex-wrap justify-center gap-4">
            <div 
              v-for="step in layer" 
              :key="step.id"
              class="relative"
            >
              <!-- Step Card -->
              <div 
                class="bg-base-100 rounded-lg shadow-sm border-2 transition-all w-64"
                :class="getStepBorderClass(step)"
                @click="selectStep(step)"
              >
                <div class="p-3">
                  <!-- Step Header -->
                  <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center gap-2">
                      <i :class="getStepIcon(step)" class="text-lg"></i>
                      <span class="font-medium text-sm">{{ step.name }}</span>
                    </div>
                    <span v-if="getStepStatus(step)" :class="getStatusBadge(step)">
                      {{ getStepStatus(step) }}
                    </span>
                  </div>

                  <!-- Plugin/Tool ID -->
                  <div class="text-xs text-base-content/50 mb-2">
                    {{ step.plugin_id || step.tool_name || step.step_type }}
                  </div>

                  <!-- Input Ports -->
                  <div v-if="showLabels && getStepPorts(step).inputs.length > 0" class="mb-2">
                    <div class="flex flex-wrap gap-1">
                      <span 
                        v-for="port in getStepPorts(step).inputs" 
                        :key="port.name"
                        class="badge badge-outline badge-xs"
                        :class="getPortColorClass(port.expected_artifact_type, 'border')"
                      >
                        <i class="fas fa-sign-in-alt mr-1 text-[8px]"></i>
                        {{ port.name }}
                      </span>
                    </div>
                  </div>

                  <!-- Output Ports -->
                  <div v-if="showLabels && getStepPorts(step).outputs.length > 0">
                    <div class="flex flex-wrap gap-1">
                      <span 
                        v-for="port in getStepPorts(step).outputs" 
                        :key="port.name"
                        class="badge badge-xs"
                        :class="getPortColorClass(port.artifact_type, 'bg')"
                      >
                        <i class="fas fa-sign-out-alt mr-1 text-[8px]"></i>
                        {{ port.name }}
                      </span>
                    </div>
                  </div>

                  <!-- Artifact Summary -->
                  <div v-if="getStepArtifacts(step).length > 0" class="mt-2 pt-2 border-t border-base-300">
                    <div class="flex flex-wrap gap-1">
                      <span 
                        v-for="artifact in getStepArtifacts(step)" 
                        :key="artifact.type"
                        class="badge badge-ghost badge-xs"
                      >
                        {{ artifact.type }}: {{ artifact.count }}
                      </span>
                    </div>
                  </div>
                </div>

                <!-- Upstream Connections Indicator -->
                <div 
                  v-if="step.depends_on.length > 0"
                  class="absolute -top-3 left-1/2 transform -translate-x-1/2"
                >
                  <div class="w-6 h-6 rounded-full bg-base-200 border-2 border-base-content/20 flex items-center justify-center text-xs">
                    {{ step.depends_on.length }}
                  </div>
                </div>
              </div>

              <!-- Connection Lines to Dependencies -->
              <svg 
                v-if="step.depends_on.length > 0 && layerIdx > 0"
                class="absolute -top-10 left-0 w-full h-10 pointer-events-none"
                :viewBox="`0 0 256 40`"
              >
                <path 
                  v-for="(dep, depIdx) in step.depends_on"
                  :key="dep"
                  :d="getConnectionPath(step, dep, depIdx)"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  class="text-base-content/30"
                  marker-end="url(#arrowhead)"
                />
              </svg>
            </div>
          </div>

          <!-- Inter-layer Connection -->
          <div v-if="layerIdx < stepLayers.length - 1" class="flex justify-center">
            <div class="w-0.5 h-6 bg-base-content/20"></div>
          </div>
        </div>

        <!-- Workflow Output -->
        <div class="flex items-center justify-center">
          <div class="bg-success/20 border-2 border-success border-dashed rounded-lg px-4 py-2">
            <div class="text-sm font-medium text-success flex items-center gap-2">
              <i class="fas fa-flag-checkered"></i>
              {{ t('bugBounty.workflow.output') }}
            </div>
            <div v-if="totalArtifacts" class="text-xs text-base-content/60 mt-1">
              {{ t('bugBounty.workflow.totalArtifacts') }}: {{ totalArtifacts }}
            </div>
          </div>
        </div>
      </div>

      <!-- SVG Defs for Arrow Marker -->
      <svg class="absolute w-0 h-0">
        <defs>
          <marker
            id="arrowhead"
            markerWidth="10"
            markerHeight="7"
            refX="10"
            refY="3.5"
            orient="auto"
          >
            <polygon points="0 0, 10 3.5, 0 7" fill="currentColor" class="text-base-content/30" />
          </marker>
        </defs>
      </svg>
    </div>

    <!-- Legend -->
    <div class="flex flex-wrap items-center gap-4 text-xs text-base-content/60">
      <span class="flex items-center gap-1">
        <span class="w-3 h-3 rounded bg-primary"></span>
        {{ t('bugBounty.workflow.artifactTypes.subdomains') }}
      </span>
      <span class="flex items-center gap-1">
        <span class="w-3 h-3 rounded bg-success"></span>
        {{ t('bugBounty.workflow.artifactTypes.liveHosts') }}
      </span>
      <span class="flex items-center gap-1">
        <span class="w-3 h-3 rounded bg-secondary"></span>
        {{ t('bugBounty.workflow.artifactTypes.technologies') }}
      </span>
      <span class="flex items-center gap-1">
        <span class="w-3 h-3 rounded bg-error"></span>
        {{ t('bugBounty.workflow.artifactTypes.findings') }}
      </span>
      <span class="flex items-center gap-1">
        <span class="w-3 h-3 rounded bg-info"></span>
        {{ t('bugBounty.workflow.artifactTypes.directories') }}
      </span>
    </div>

    <!-- Selected Step Detail -->
    <div v-if="selectedStep" class="border-t pt-4">
      <WorkflowStepDetail
        :step="selectedStep"
        :execution-id="executionId"
        :step-result="getStepResult(selectedStep)"
        :show-execution-details="!!executionId"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import WorkflowStepDetail from './WorkflowStepDetail.vue'

const { t } = useI18n()

interface WorkflowStep {
  id: string
  name: string
  step_type: string
  plugin_id?: string
  tool_name?: string
  config: any
  depends_on: string[]
}

interface PluginPortInfo {
  plugin_id: string
  output_ports: { name: string; artifact_type: string }[]
  input_params: { name: string; expected_artifact_type: string; extract_path?: string; required: boolean }[]
}

const props = defineProps<{
  steps: WorkflowStep[]
  initialInputs?: any
  executionId?: string
  stepResults?: Record<string, any>
}>()

const emit = defineEmits<{
  (e: 'step-select', step: WorkflowStep): void
}>()

// State
const showLabels = ref(true)
const expandAll = ref(false)
const selectedStep = ref<WorkflowStep | null>(null)
const pluginPorts = ref<Record<string, PluginPortInfo>>({})
const stepArtifactSummary = ref<Record<string, { type: string; count: number }[]>>({})

// Computed
const stepLayers = computed(() => {
  const layers: WorkflowStep[][] = []
  const processed = new Set<string>()
  const stepMap = new Map(props.steps.map(s => [s.id, s]))

  const canProcess = (step: WorkflowStep) => {
    return step.depends_on.every(dep => processed.has(dep))
  }

  let remaining = [...props.steps]
  while (remaining.length > 0) {
    const layer = remaining.filter(canProcess)
    if (layer.length === 0) {
      // Circular dependency or orphaned steps - add remaining
      layers.push(remaining)
      break
    }
    layers.push(layer)
    layer.forEach(s => processed.add(s.id))
    remaining = remaining.filter(s => !processed.has(s.id))
  }

  return layers
})

const totalArtifacts = computed(() => {
  let total = 0
  for (const summary of Object.values(stepArtifactSummary.value)) {
    for (const item of summary) {
      total += item.count
    }
  }
  return total
})

// Methods
const loadAllPluginPorts = async () => {
  try {
    const allPorts = await invoke<PluginPortInfo[]>('bounty_list_plugin_ports')
    for (const port of allPorts) {
      pluginPorts.value[port.plugin_id] = port
    }
  } catch (error) {
    console.error('Failed to load plugin ports:', error)
  }
}

const selectStep = (step: WorkflowStep) => {
  selectedStep.value = step
  emit('step-select', step)
}

const getStepPorts = (step: WorkflowStep) => {
  const pluginId = step.plugin_id || step.tool_name || ''
  const ports = pluginPorts.value[pluginId]
  return {
    inputs: ports?.input_params || [],
    outputs: ports?.output_ports || [],
  }
}

const getStepIcon = (step: WorkflowStep) => {
  const type = step.step_type
  if (type === 'plugin') return 'fas fa-puzzle-piece text-primary'
  if (type === 'tool') return 'fas fa-wrench text-info'
  if (type === 'condition') return 'fas fa-code-branch text-warning'
  if (type === 'parallel') return 'fas fa-columns text-accent'
  return 'fas fa-cog'
}

const getStepBorderClass = (step: WorkflowStep) => {
  const status = getStepStatus(step)
  if (status === 'completed') return 'border-success'
  if (status === 'running') return 'border-info animate-pulse'
  if (status === 'failed') return 'border-error'
  if (selectedStep.value?.id === step.id) return 'border-primary'
  return 'border-base-300'
}

const getStepStatus = (step: WorkflowStep): string => {
  if (!props.stepResults) return ''
  const result = props.stepResults[step.id]
  if (!result) return 'pending'
  if (result.error) return 'failed'
  if (result.success === false) return 'failed'
  return 'completed'
}

const getStatusBadge = (step: WorkflowStep) => {
  const status = getStepStatus(step)
  switch (status) {
    case 'completed': return 'badge badge-success badge-xs'
    case 'running': return 'badge badge-info badge-xs'
    case 'failed': return 'badge badge-error badge-xs'
    default: return 'badge badge-ghost badge-xs'
  }
}

const getStepResult = (step: WorkflowStep) => {
  return props.stepResults?.[step.id]
}

const getStepArtifacts = (step: WorkflowStep) => {
  return stepArtifactSummary.value[step.id] || []
}

const getPortColorClass = (artifactType: string, variant: 'bg' | 'border') => {
  const colors: Record<string, Record<string, string>> = {
    subdomains: { bg: 'badge-primary', border: 'border-primary' },
    live_hosts: { bg: 'badge-success', border: 'border-success' },
    technologies: { bg: 'badge-secondary', border: 'border-secondary' },
    finding: { bg: 'badge-error', border: 'border-error' },
    findings: { bg: 'badge-error', border: 'border-error' },
    directories: { bg: 'badge-info', border: 'border-info' },
    endpoints: { bg: 'badge-accent', border: 'border-accent' },
    secrets: { bg: 'badge-warning', border: 'border-warning' },
    raw_data: { bg: 'badge-ghost', border: 'border-base-content/30' },
  }
  return colors[artifactType]?.[variant] || (variant === 'bg' ? 'badge-ghost' : 'border-base-content/30')
}

const getConnectionPath = (step: WorkflowStep, depId: string, depIdx: number) => {
  // Simple vertical line for now
  const x = 128
  return `M ${x} 0 L ${x} 40`
}

const processStepResults = async () => {
  if (!props.stepResults) return

  for (const step of props.steps) {
    const result = props.stepResults[step.id]
    if (!result) continue

    try {
      const processed = await invoke<any>('bounty_process_step_output', {
        request: {
          execution_id: props.executionId || 'preview',
          step_id: step.id,
          step_name: step.name,
          plugin_id: step.plugin_id,
          raw_output: result,
        }
      })

      stepArtifactSummary.value[step.id] = processed.artifacts.map((a: any) => ({
        type: a.artifact_type,
        count: a.count || 1,
      }))
    } catch (error) {
      console.error(`Failed to process step ${step.id} results:`, error)
    }
  }
}

// Lifecycle
onMounted(async () => {
  await loadAllPluginPorts()
  if (props.stepResults) {
    await processStepResults()
  }
})

watch(() => props.stepResults, async () => {
  if (props.stepResults) {
    await processStepResults()
  }
}, { deep: true })
</script>
