<template>
  <div class="mt-3">
    <div class="bg-base-200/60 rounded-lg border border-base-300/50 overflow-hidden">
      <div class="p-3 border-b border-base-300/50 bg-base-100/50">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-primary" fill="currentColor" viewBox="0 0 20 20">
            <path d="M13 7H7v6h6V7z"/>
            <path fill-rule="evenodd" d="M7 2a1 1 0 012 0v1h2V2a1 1 0 112 0v1h2a2 2 0 012 2v2h1a1 1 0 110 2h-1v2h1a1 1 0 110 2h-1v2a2 2 0 01-2 2h-2v1a1 1 0 11-2 0v-1H9v1a1 1 0 11-2 0v-1H5a2 2 0 01-2-2v-2H2a1 1 0 110-2h1V9H2a1 1 0 010-2h1V5a2 2 0 012-2h2V2zM5 5h10v10H5V5z" clip-rule="evenodd"/>
          </svg>
          <span class="font-medium text-sm">{{ t('aiAssistant.toolExecutions', '工具调用') }}</span>
          <div class="badge badge-primary badge-outline badge-xs ml-auto">
            {{ toolExecutions.length }} 调用
          </div>
        </div>
      </div>
      
      <!-- Tool executions list -->
      <div class="space-y-1 p-2">
        <div 
          v-for="(execution, index) in toolExecutions" 
          :key="`tool-${index}`" 
          class="collapse collapse-arrow bg-base-100 border border-base-200 rounded-lg"
          :class="{ 
            'border-primary border-2': execution.status === 'running',
            'border-success border-2': execution.status === 'completed',
            'border-error border-2': execution.status === 'failed'
          }"
        >
          <input type="checkbox" class="collapse-checkbox" :id="`tool-${index}`" />
          <div class="collapse-title font-medium text-sm py-3 px-4 hover:bg-base-200/50 transition-colors">
            <div class="flex items-center gap-3">
              <div class="badge badge-primary badge-sm">{{ index + 1 }}</div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <span class="truncate">{{ execution.tool || execution.name }}</span>
                  <span class="badge badge-xs" :class="getToolStatusClass(execution.status)">
                    {{ getToolStatusText(execution.status) }}
                  </span>
                </div>
                
                <!-- Progress indicator for running tools -->
                <div v-if="execution.status === 'running'" class="mt-2">
                  <div class="flex items-center gap-2 text-xs">
                    <span class="text-base-content/60">执行中...</span>
                    <div class="flex-1 h-1 bg-base-200 rounded-full">
                      <div class="h-full bg-primary rounded-full transition-all duration-500" 
                           style="width: 100%; animation: progress-pulse 1.5s ease-in-out infinite;"></div>
                    </div>
                  </div>
                </div>
                
                <div v-if="execution.description" 
                     class="text-xs text-base-content/60 mt-1 truncate">{{ execution.description }}</div>
              </div>
            </div>
          </div>
          
          <div class="collapse-content px-4 pb-4">
            <div class="space-y-3">
              <!-- Basic info -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                <div v-if="execution.started_at" class="flex items-center gap-2">
                  <i class="fas fa-clock text-base-content/60"></i>
                  <span class="text-base-content/70">开始:</span>
                  <span>{{ formatTimestamp(execution.started_at) }}</span>
                </div>
                <div v-if="execution.completed_at" class="flex items-center gap-2">
                  <i class="fas fa-check-circle text-success"></i>
                  <span class="text-base-content/70">完成:</span>
                  <span>{{ formatTimestamp(execution.completed_at) }}</span>
                </div>
                <div v-if="execution.duration" class="flex items-center gap-2">
                  <i class="fas fa-stopwatch text-base-content/60"></i>
                  <span class="text-base-content/70">耗时:</span>
                  <span>{{ formatDuration(execution.duration) }}</span>
                </div>
              </div>
              
              <!-- Input params -->
              <div v-if="execution.input || execution.params" class="bg-base-200/80 border border-base-300/30 rounded-lg p-3">
                <div class="text-xs font-medium text-base-content/80 mb-2 flex items-center gap-1">
                  <i class="fas fa-arrow-right"></i>
                  输入参数
                </div>
                <div class="text-sm text-base-content">
                  <pre class="text-xs overflow-x-auto overflow-y-auto max-h-40 max-w-full whitespace-pre-wrap word-wrap break-word word-break break-all bg-base-200/50 p-2 rounded border">{{ JSON.stringify(execution.input || execution.params, null, 2) }}</pre>
                </div>
              </div>
              
              <!-- Results -->
              <div v-if="execution.result || execution.output" class="bg-success/10 border border-success/20 rounded-lg p-3">
                <div class="text-xs font-medium text-success mb-2 flex items-center gap-1">
                  <i class="fas fa-check-circle"></i>
                  执行结果
                </div>
                <div class="text-sm text-base-content">
                  <pre class="text-xs overflow-x-auto overflow-y-auto max-h-60 max-w-full whitespace-pre-wrap word-wrap break-word word-break break-all bg-base-200/50 p-2 rounded border">{{ formatToolResult(execution.result || execution.output) }}</pre>
                </div>
              </div>
              
              <!-- Errors -->
              <div v-if="execution.error" class="bg-error/10 border border-error/20 rounded-lg p-3">
                <div class="text-xs font-medium text-error mb-2 flex items-center gap-1">
                  <i class="fas fa-exclamation-triangle"></i>
                  错误信息
                </div>
                <div class="text-sm text-error whitespace-pre-wrap">{{ execution.error }}</div>
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
  toolExecutions: any[]
}>()

const { t } = useI18n()
const { 
  formatTimestamp, 
  formatDuration, 
  formatToolResult, 
  getToolStatusClass,
  getResultStatusText 
} = useMessageUtils()

// 使用 getResultStatusText 作为工具状态文本
const getToolStatusText = getResultStatusText
</script>

<style scoped>
/* Comprehensive overflow prevention for tool executions */
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
