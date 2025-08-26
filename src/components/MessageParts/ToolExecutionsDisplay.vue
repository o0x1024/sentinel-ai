<template>
  <div class="mt-4">
    <div class="collapse collapse-arrow bg-base-300/50 rounded-xl border border-base-300">
      <input type="checkbox" class="collapse-checkbox" />
      <div class="collapse-title font-semibold flex items-center gap-2 py-3">
        <i class="fas fa-tools text-info"></i>
        <span>{{ t('aiAssistant.toolExecution', '工具执行') }}</span>
        <div class="badge badge-info badge-outline badge-sm ml-auto">{{ toolExecutions.length }} 工具</div>
        
        <!-- Streaming indicator -->
        <div v-if="isStreaming" class="flex items-center gap-1 text-xs text-primary">
          <i class="fas fa-spinner fa-spin"></i>
          <span>执行中...</span>
        </div>
      </div>
      <div class="collapse-content">
        <!-- Current step indicator -->
        <div v-if="isStreaming && currentStep" 
             class="flex items-center gap-2 mb-3 text-sm text-primary bg-primary/10 px-3 py-2 rounded-lg">
          <i class="fas fa-cog fa-spin"></i>
          <span>{{ currentStep }}</span>
        </div>
        
        <div class="space-y-3 pt-2">
          <div v-for="tool in toolExecutions" :key="tool.id" 
               class="collapse collapse-arrow bg-base-100 rounded-lg shadow-sm"
               :class="{ 'border-primary border-2': tool.status === 'executing' }">
            <input type="checkbox" class="collapse-checkbox" />
            <div class="collapse-title text-sm font-medium flex items-center gap-2 py-3">
              <i class="fas fa-cog" :class="{
                'text-warning animate-spin': tool.status === 'executing' || tool.status === 'running',
                'text-success': tool.status === 'completed',
                'text-error': tool.status === 'failed',
                'text-base-content/50': tool.status === 'pending'
              }"></i>
              <span class="flex-1">{{ tool.name }}</span>
              
              <!-- Progress indicator for executing tools -->
              <div v-if="tool.status === 'executing' && tool.progress !== undefined" 
                   class="text-xs text-primary mr-2">
                {{ tool.progress }}%
              </div>
              
              <div class="badge badge-xs" :class="getToolStatusClass(tool.status)">
                {{ getToolStatusText(tool.status) }}
              </div>
            </div>
            <div class="collapse-content">
              <div v-if="tool.result" class="bg-base-200 rounded-lg p-3 text-xs font-mono overflow-hidden">
                <pre class="whitespace-pre-wrap word-wrap break-word word-break break-all overflow-wrap break-word max-w-full overflow-x-auto overflow-y-auto max-h-40"><code>{{ formatToolResult(tool.result) }}</code></pre>
              </div>
              <div v-if="tool.error" class="alert alert-error mt-3 text-sm">
                <i class="fas fa-exclamation-triangle"></i>
                <span>{{ tool.error }}</span>
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
  isStreaming?: boolean
  currentStep?: string
}>()

const { t } = useI18n()
const { getToolStatusClass, formatToolResult } = useMessageUtils()

const getToolStatusText = (status: string) => {
  const statusMap: Record<string, string> = {
    'pending': '待执行',
    'executing': '执行中',
    'running': '执行中',
    'completed': '已完成',
    'failed': '失败'
  }
  return statusMap[status] || status
}
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
  max-width: 100%;
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

code {
  word-wrap: break-word;
  word-break: break-all;
  overflow-wrap: break-word;
  max-width: 100%;
}

/* Tool result container */
.font-mono {
  max-width: 100%;
  overflow: hidden;
}

/* Ensure all text content wraps properly */
* {
  max-width: 100%;
  box-sizing: border-box;
}
</style>