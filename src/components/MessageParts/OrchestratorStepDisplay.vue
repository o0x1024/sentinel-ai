<template>
  <div class="orchestrator-display">
    <!-- Session Summary -->
    <div v-if="sessionData" class="session-summary bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 rounded-lg p-4 mb-4 border border-purple-200 dark:border-purple-700">
      <div class="flex items-start justify-between mb-3">
        <div class="flex-1">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-lg font-semibold text-purple-700 dark:text-purple-300">
              🎯 {{ taskKindLabel }}
            </span>
            <span class="px-2 py-1 text-xs rounded-full bg-purple-100 dark:bg-purple-800 text-purple-700 dark:text-purple-200">
              {{ stageLabel }}
            </span>
          </div>
          <div class="text-sm text-gray-600 dark:text-gray-300 mb-2">
            <span class="font-medium">目标:</span> {{ sessionData.primaryTarget }}
          </div>
          <div class="text-sm text-gray-600 dark:text-gray-300">
            {{ sessionData.summary }}
          </div>
        </div>
      </div>
      
      <!-- Stats -->
      <div class="flex gap-4 text-sm">
        <div class="flex items-center gap-1">
          <span class="text-gray-500 dark:text-gray-400">步骤:</span>
          <span class="font-semibold text-gray-700 dark:text-gray-200">{{ sessionData.totalSteps }}</span>
        </div>
        <div class="flex items-center gap-1">
          <span class="text-gray-500 dark:text-gray-400">发现:</span>
          <span class="font-semibold text-gray-700 dark:text-gray-200">{{ sessionData.totalFindings }}</span>
        </div>
        <div v-if="sessionData.highRiskFindings > 0" class="flex items-center gap-1">
          <span class="text-gray-500 dark:text-gray-400">高危:</span>
          <span class="font-semibold text-red-600 dark:text-red-400">{{ sessionData.highRiskFindings }}</span>
        </div>
      </div>
    </div>

    <!-- Step Display -->
    <div v-if="stepData" class="step-display bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700 hover:border-purple-300 dark:hover:border-purple-600 transition-colors">
      <div class="flex items-start gap-3">
        <!-- Step Index & Status Icon -->
        <div class="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-lg" :class="getStepBgClass(stepData.status)">
          {{ statusIcon }}
        </div>
        
        <!-- Step Content -->
        <div class="flex-1 min-w-0">
          <!-- Header -->
          <div class="flex items-center gap-2 mb-2 flex-wrap">
            <span class="text-xs font-mono text-gray-500 dark:text-gray-400">
              #{{ stepData.index }}
            </span>
            <span class="px-2 py-0.5 text-xs rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-200">
              {{ subAgentLabel }}
            </span>
            <span class="px-2 py-0.5 text-xs rounded-full" :class="getRiskBgClass(stepData.riskImpact)">
              {{ stepData.riskImpact }}
            </span>
            <span class="text-xs" :class="statusColor">
              {{ getStatusLabel(stepData.status) }}
            </span>
          </div>
          
          <!-- Summary -->
          <div class="text-sm text-gray-700 dark:text-gray-200 mb-2">
            {{ stepData.shortSummary }}
          </div>
          
          <!-- Output (collapsible) -->
          <div v-if="stepData.output" class="mt-2">
            <button
              @click="toggleOutput"
              class="text-xs text-purple-600 dark:text-purple-400 hover:text-purple-700 dark:hover:text-purple-300 flex items-center gap-1"
            >
              <span>{{ showOutput ? '▼' : '▶' }}</span>
              <span>详细输出</span>
            </button>
            <div v-if="showOutput" class="mt-2 p-3 bg-gray-50 dark:bg-gray-900 rounded text-xs font-mono text-gray-600 dark:text-gray-300 whitespace-pre-wrap max-h-64 overflow-y-auto">
              {{ stepData.output }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useOrchestratorMessage } from '@/composables/useOrchestratorMessage'

const props = defineProps<{
  content: string
}>()

const {
  sessionData,
  stepData,
  taskKindLabel,
  stageLabel,
  subAgentLabel,
  statusColor,
  statusIcon,
} = useOrchestratorMessage(props.content)

const showOutput = ref(false)

const toggleOutput = () => {
  showOutput.value = !showOutput.value
}

const getStepBgClass = (status: string) => {
  const classes: Record<string, string> = {
    'pending': 'bg-gray-100 dark:bg-gray-700',
    'running': 'bg-blue-100 dark:bg-blue-900',
    'completed': 'bg-green-100 dark:bg-green-900',
    'failed': 'bg-red-100 dark:bg-red-900',
  }
  return classes[status] || 'bg-gray-100 dark:bg-gray-700'
}

const getRiskBgClass = (risk: string) => {
  const classes: Record<string, string> = {
    'None': 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300',
    'Info': 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-200',
    'Low': 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-200',
    'Medium': 'bg-yellow-100 dark:bg-yellow-900 text-yellow-700 dark:text-yellow-200',
    'High': 'bg-orange-100 dark:bg-orange-900 text-orange-700 dark:text-orange-200',
    'Critical': 'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200',
  }
  return classes[risk] || 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    'pending': '等待中',
    'running': '执行中',
    'completed': '已完成',
    'failed': '失败',
  }
  return labels[status] || status
}
</script>

<style scoped>
.orchestrator-display {
  width: 100%;
}

.session-summary {
  animation: slideIn 0.3s ease-out;
}

.step-display {
  animation: fadeIn 0.3s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
</style>

