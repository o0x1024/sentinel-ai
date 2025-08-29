<template>
  <div class="mt-3">
    <div class="bg-base-200/60 rounded-lg border border-base-300/50 overflow-hidden">
      <div class="p-3 border-b border-base-300/50 bg-base-100/50">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-success" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"/>
          </svg>
          <span class="font-medium text-sm">{{ t('aiAssistant.executionResult', '执行结果') }}</span>
          <div class="badge badge-outline badge-xs ml-auto" :class="getResultStatusClass(executionResult.status || 'completed')">
            {{ getResultStatusText(executionResult.status || 'completed') }}
          </div>
        </div>
      </div>
      
      <!-- Execution result content -->
      <div class="p-4">
        <div class="space-y-4">
          <!-- Summary section -->
          <div v-if="executionResult.summary" class="bg-base-100 p-3 rounded-lg border border-base-300/50">
            <div class="text-xs font-medium mb-2">执行摘要</div>
            <div class="text-sm" v-html="renderMarkdown(executionResult.summary)"></div>
          </div>
          
          <!-- Result data section -->
          <div v-if="hasResultData" class="bg-success/10 border border-success/20 rounded-lg p-3">
            <div class="text-xs font-medium text-success mb-2 flex items-center gap-1">
              <i class="fas fa-check-circle"></i>
              详细结果
            </div>
            <div class="text-sm text-base-content">
              <pre v-if="isJsonResult" class="text-xs overflow-x-auto overflow-y-auto max-h-60 max-w-full whitespace-pre-wrap word-wrap break-word word-break break-all bg-base-200/50 p-2 rounded border">{{ JSON.stringify(resultData, null, 2) }}</pre>
              <div v-else class="prose prose-sm max-w-none" v-html="renderMarkdown(resultData)"></div>
            </div>
          </div>
          
          <!-- Error section -->
          <div v-if="executionResult.error" class="bg-error/10 border border-error/20 rounded-lg p-3">
            <div class="text-xs font-medium text-error mb-2 flex items-center gap-1">
              <i class="fas fa-exclamation-triangle"></i>
              错误信息
            </div>
            <div class="text-sm text-error whitespace-pre-wrap">{{ executionResult.error }}</div>
          </div>
          
          <!-- Metadata section -->
          <div v-if="hasMetadata" class="bg-base-100 p-3 rounded-lg border border-base-300/50">
            <div class="text-xs font-medium mb-2 flex items-center justify-between">
              <span>执行元数据</span>
              <button @click="showMetadata = !showMetadata" class="btn btn-xs btn-ghost">
                <i :class="['fas', showMetadata ? 'fa-chevron-up' : 'fa-chevron-down']"></i>
              </button>
            </div>
            <div v-if="showMetadata" class="text-xs">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                <div v-if="executionResult.duration" class="flex items-center gap-2">
                  <span class="text-base-content/70">总耗时:</span>
                  <span>{{ formatDuration(executionResult.duration) }}</span>
                </div>
                <div v-if="executionResult.started_at" class="flex items-center gap-2">
                  <span class="text-base-content/70">开始时间:</span>
                  <span>{{ formatTimestamp(executionResult.started_at) }}</span>
                </div>
                <div v-if="executionResult.completed_at" class="flex items-center gap-2">
                  <span class="text-base-content/70">完成时间:</span>
                  <span>{{ formatTimestamp(executionResult.completed_at) }}</span>
                </div>
                <div v-if="executionResult.architecture" class="flex items-center gap-2">
                  <span class="text-base-content/70">执行架构:</span>
                  <span>{{ executionResult.architecture }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../../composables/useMessageUtils'

const props = defineProps<{
  executionResult: any
}>()

const { t } = useI18n()
const { 
  renderMarkdown, 
  formatTimestamp, 
  formatDuration, 
  getResultStatusClass, 
  getResultStatusText,
  getExecutionDetailedResult
} = useMessageUtils()

const showMetadata = ref(false)

// Computed properties for result data handling
const resultData = computed(() => {
  if (!props.executionResult) return null
  
  if (props.executionResult.result) {
    return props.executionResult.result
  }
  
  if (props.executionResult.data) {
    return props.executionResult.data
  }
  
  return null
})

const hasResultData = computed(() => resultData.value !== null)

const isJsonResult = computed(() => {
  if (!resultData.value) return false
  return typeof resultData.value === 'object'
})

const hasMetadata = computed(() => {
  return !!(
    props.executionResult?.duration || 
    props.executionResult?.started_at || 
    props.executionResult?.completed_at || 
    props.executionResult?.architecture
  )
})
</script>

<style scoped>
/* Comprehensive overflow prevention */
pre {
  white-space: pre-wrap !important;
  word-wrap: break-word !important;
  word-break: break-all !important;
  overflow-wrap: break-word !important;
  max-width: 100% !important;
  box-sizing: border-box !important;
}

/* Ensure all text content wraps properly */
* {
  max-width: 100%;
  box-sizing: border-box;
}
</style>
