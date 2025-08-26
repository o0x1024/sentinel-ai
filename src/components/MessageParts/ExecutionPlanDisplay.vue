<template>
  <div class="mt-3">
    <div class="bg-base-200/60 rounded-lg border border-base-300/50 overflow-hidden">
      <div class="p-3 border-b border-base-300/50 bg-base-100/50">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-accent" fill="currentColor" viewBox="0 0 20 20">
            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
          <span class="font-medium text-sm">{{ t('aiAssistant.executionPlan', '执行计划') }}</span>
          <div class="badge badge-accent badge-outline badge-xs ml-auto">
            {{ executionPlan.steps?.length || 0 }} 步骤
          </div>
        </div>
        
        <!-- Progress bar -->
        <div v-if="isStreaming || executionProgress !== undefined" 
             class="flex items-center gap-2 mt-2">
          <div class="text-xs text-base-content/60">进度:</div>
          <div class="flex-1 h-1.5 bg-base-300 rounded-full overflow-hidden">
            <div class="h-full bg-gradient-to-r from-primary to-accent transition-all duration-500 ease-out" 
                 :style="{ width: `${executionProgress || 0}%` }"
                 :class="{ 'animate-pulse': isStreaming && (executionProgress || 0) < 100 }"></div>
          </div>
          <div class="text-xs text-base-content/60 min-w-max font-mono">
            {{ Math.round(executionProgress || 0) }}%
          </div>
        </div>
        
        <!-- Current step indicator -->
        <div v-if="isStreaming && currentStep" 
             class="flex items-center gap-2 mt-2 text-xs text-base-content/70 bg-primary/5 px-2 py-1 rounded">
          <svg class="w-3 h-3 text-primary animate-spin" fill="currentColor" viewBox="0 0 20 20">
            <path d="M4 2a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2V4a2 2 0 00-2-2H4z"/>
          </svg>
          <span>正在执行: {{ currentStep }}</span>
        </div>
      </div>
      
      <!-- Steps list -->
      <div class="space-y-1 p-2">
        <div v-for="(step, index) in executionPlan.steps" :key="step.id" 
             class="collapse collapse-arrow bg-base-100 border border-base-200 rounded-lg"
             :class="{ 
               'border-primary border-2': step.status === 'executing' || step.status === 'running'
             }">
          <input type="checkbox" class="collapse-checkbox" :id="`step-${step.id}`" />
          <div class="collapse-title font-medium text-sm py-3 px-4 hover:bg-base-200/50 transition-colors">
            <div class="flex items-center gap-3">
              <div class="badge badge-primary badge-sm">{{ index + 1 }}</div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <span class="truncate">{{ step.name || step.description }}</span>
                  <i v-if="step.status === 'executing' || step.status === 'running'" 
                     class="fas fa-spinner fa-spin text-primary text-xs"></i>
                  <i v-else-if="step.status === 'completed'" 
                     class="fas fa-check-circle text-success text-xs"></i>
                  <i v-else-if="step.status === 'failed'" 
                     class="fas fa-times-circle text-error text-xs"></i>
                </div>
                
                <!-- Progress indicator for executing steps -->
                <div v-if="step.status === 'executing' || step.status === 'running'" class="mt-2">
                  <div class="flex items-center gap-2 text-xs">
                    <span class="text-base-content/60">执行中...</span>
                    <div class="flex-1 h-1 bg-base-200 rounded-full">
                      <div class="h-full bg-primary rounded-full transition-all duration-500" 
                           style="width: 100%; animation: progress-pulse 1.5s ease-in-out infinite;"></div>
                    </div>
                  </div>
                </div>
                
                <div v-if="step.description && step.name !== step.description" 
                     class="text-xs text-base-content/60 mt-1 truncate">{{ step.description }}</div>
              </div>
            </div>
          </div>
          
          <div class="collapse-content px-4 pb-4">
            <div class="space-y-3">
              <!-- Basic info -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                <div v-if="step.started_at" class="flex items-center gap-2">
                  <i class="fas fa-clock text-base-content/60"></i>
                  <span class="text-base-content/70">开始:</span>
                  <span>{{ formatTimestamp(step.started_at) }}</span>
                </div>
                <div v-if="step.completed_at" class="flex items-center gap-2">
                  <i class="fas fa-check-circle text-success"></i>
                  <span class="text-base-content/70">完成:</span>
                  <span>{{ formatTimestamp(step.completed_at) }}</span>
                </div>
              </div>
              
              <!-- Results -->
              <div v-if="step.result_data" class="bg-success/10 border border-success/20 rounded-lg p-3">
                <div class="text-xs font-medium text-success mb-2 flex items-center gap-1">
                  <i class="fas fa-check-circle"></i>
                  执行结果
                </div>
                <div class="text-sm text-base-content">
                  <pre class="text-xs overflow-x-auto overflow-y-auto max-h-60 max-w-full whitespace-pre-wrap word-wrap break-word word-break break-all bg-base-200/50 p-2 rounded border">{{ JSON.stringify(step.result_data, null, 2) }}</pre>
                </div>
              </div>
              
              <!-- Errors -->
              <div v-if="step.error" class="bg-error/10 border border-error/20 rounded-lg p-3">
                <div class="text-xs font-medium text-error mb-2 flex items-center gap-1">
                  <i class="fas fa-exclamation-triangle"></i>
                  错误信息
                </div>
                <div class="text-sm text-error whitespace-pre-wrap">{{ step.error }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../../composables/useMessageUtils'

const props = defineProps<{
  executionPlan: {
    steps?: any[]
    name?: string
  }
  executionProgress?: number
  currentStep?: string
  isStreaming?: boolean
}>()

const { t } = useI18n()
const { formatTimestamp } = useMessageUtils()
</script>

<style scoped>
/* Comprehensive overflow prevention for execution plan */
.collapse {
  max-width: 100%;
  overflow: hidden;
}

.collapse-title {
  word-wrap: break-word;
  word-break: break-word;
  overflow-wrap: break-word;
}

.collapse-content {
  max-width: 100%;
  overflow: hidden;
}

/* Pre and code element formatting */
pre {
  white-space: pre-wrap !important;
  word-wrap: break-word !important;
  word-break: break-all !important;
  overflow-wrap: break-word !important;
  max-width: 100% !important;
  box-sizing: border-box !important;
}

/* JSON content specific styling */
.json-content {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  white-space: pre-wrap;
  word-wrap: break-word;
  word-break: break-all;
  overflow-wrap: break-word;
  max-width: 100%;
}

/* Ensure all text content wraps properly */
* {
  max-width: 100%;
  box-sizing: border-box;
}

/* Progress animation */
@keyframes progress-pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}
</style>