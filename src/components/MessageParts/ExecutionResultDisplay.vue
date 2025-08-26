<template>
  <div class="mt-4">
    <div class="collapse collapse-open bg-gradient-to-r from-success/10 to-info/10 border border-success/30 rounded-xl">
      <div class="collapse-title font-semibold flex items-center gap-2 text-success py-3">
        <i class="fas fa-chart-bar"></i>
        <span>{{ t('aiAssistant.executionResult', '执行结果') }}</span>
        <div class="badge badge-success badge-outline badge-sm ml-auto">
          {{ getResultStatusText(executionResult.status) }}
        </div>
      </div>
      <div class="collapse-content">
        <div class="space-y-4 pt-2">
          <!-- Basic stats -->
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 text-sm">
            <div class="bg-base-100 rounded-lg p-3 shadow-sm">
              <span class="font-medium text-base-content/70">状态</span>
              <div class="mt-1">
                <div class="badge badge-lg" :class="getResultStatusClass(executionResult.status)">
                  {{ getResultStatusText(executionResult.status) }}
                </div>
              </div>
            </div>
            <div class="bg-base-100 rounded-lg p-3 shadow-sm">
              <span class="font-medium text-base-content/70">耗时</span>
              <div class="mt-1 text-lg font-semibold text-base-content">
                {{ formatDuration(executionResult.duration || 0) }}
              </div>
            </div>
            <div class="bg-base-100 rounded-lg p-3 shadow-sm">
              <span class="font-medium text-base-content/70">完成任务</span>
              <div class="mt-1 text-lg font-semibold text-base-content">
                {{ executionResult.tasksCompleted || message.completedSteps || 0 }}/{{ executionResult.totalTasks || message.totalSteps || 0 }}
              </div>
            </div>
            <div class="bg-base-100 rounded-lg p-3 shadow-sm">
              <span class="font-medium text-base-content/70">架构</span>
              <div class="mt-1 text-lg font-semibold text-primary">{{ executionResult.architecture || 'Plan-Execute' }}</div>
            </div>
          </div>
          
          <!-- Detailed result -->
          <div v-if="getExecutionDetailedResult(message)" class="bg-base-100 rounded-lg p-4 shadow-sm">
            <h5 class="font-medium text-base-content mb-3 flex items-center gap-2">
              <i class="fas fa-clipboard-list text-info"></i>
              详细结果
            </h5>
            <div class="text-sm text-base-content/80">
              <div v-if="typeof getExecutionDetailedResult(message) === 'string'" 
                   class="prose prose-sm max-w-none"
                   v-html="renderMarkdown(getExecutionDetailedResult(message))">
              </div>
              <pre v-else class="bg-base-200 p-3 rounded text-xs overflow-x-auto overflow-y-auto max-h-40 max-w-full whitespace-pre-wrap word-wrap break-word word-break break-all border">{{ JSON.stringify(getExecutionDetailedResult(message), null, 2) }}</pre>
            </div>
          </div>
          
          <!-- Summary -->
          <div v-if="executionResult.summary" class="bg-base-100 rounded-lg p-4 shadow-sm">
            <h5 class="font-medium text-base-content mb-3 flex items-center gap-2">
              <i class="fas fa-file-alt text-info"></i>
              执行摘要
            </h5>
            <div class="text-sm text-base-content/80">{{ executionResult.summary }}</div>
          </div>
          
          <!-- Error -->
          <div v-if="executionResult.error" class="bg-error/10 border border-error/20 rounded-lg p-4">
            <h5 class="font-medium text-error mb-3 flex items-center gap-2">
              <i class="fas fa-exclamation-triangle"></i>
              错误信息
            </h5>
            <div class="text-sm text-error">{{ executionResult.error }}</div>
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
  executionResult: any
  message: any
}>()

const { t } = useI18n()
const { 
  getResultStatusClass, 
  getResultStatusText, 
  formatDuration,
  getExecutionDetailedResult,
  renderMarkdown
} = useMessageUtils()
</script>

<style scoped>
/* Comprehensive overflow prevention for execution results */
.collapse {
  max-width: 100%;
  overflow: hidden;
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

/* Grid responsiveness */
.grid {
  max-width: 100%;
}

.grid > * {
  min-width: 0;
  max-width: 100%;
}

/* Prose content wrapping */
.prose {
  max-width: 100% !important;
}

.prose * {
  max-width: 100%;
  word-wrap: break-word;
  word-break: break-word;
  overflow-wrap: break-word;
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
</style>