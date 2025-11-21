<template>
  <div class="react-step-display">
    <!-- Thought Section -->
    <div v-if="thought" class="thought-section mb-3">
      <div class="flex items-start gap-2 p-3 bg-base-200/50 rounded-lg border border-base-300/30">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-lightbulb text-warning text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-base-content/70 mb-1">Thought</div>
          <div class="text-sm text-base-content prose prose-sm max-w-none" v-html="renderMarkdown(thought)"></div>
        </div>
      </div>
    </div>

    <!-- Tool Call Section (Action + Observation combined) -->
    <div v-if="action" class="tool-call-section mb-3">
      <details 
        class="collapse collapse-arrow bg-base-100 border rounded-lg"
        :class="getToolCallBorderClass()"
        :open="isToolCallInProgress()"
      >
        <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
          <div class="flex items-center gap-3">
            <!-- Status Icon -->
            <div class="flex-shrink-0">
              <i 
                :class="[
                  'text-sm',
                  getToolCallIconClass()
                ]"
              ></i>
            </div>
            
            <!-- Tool Info -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-medium text-sm">Ran</span>
                <code class="text-xs bg-base-200 px-2 py-0.5 rounded font-mono">{{ action.tool }}</code>
                <span v-if="action.status" class="badge badge-xs" :class="getActionStatusClass(action.status)">
                  {{ getActionStatusText(action.status) }}
                </span>
              </div>
            </div>
          </div>
        </summary>
        
        <div class="collapse-content px-4 pb-4">
          <div class="space-y-3">
            <!-- Parameters Section -->
            <div v-if="action.args && Object.keys(action.args).length > 0" class="params-section">
              <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                <i class="fas fa-cog text-xs"></i>
                PARAMETERS
              </div>
              <div class="bg-base-200/50 rounded-lg p-3 border border-base-300/30">
                <div
                  v-for="(value, key) in action.args"
                  :key="key"
                  class="flex items-start gap-2 py-1"
                >
                  <span class="text-xs font-medium text-base-content/70 min-w-[100px]">
                    {{ key }}
                  </span>
                  <span class="text-xs text-base-content font-mono break-all whitespace-pre-wrap">
                    {{ typeof value === 'object' ? formatJson(value) : value }}
                  </span>
                </div>
              </div>
            </div>
            
            <!-- Response Section -->
            <div v-if="observation" class="response-section">
              <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                <i class="fas fa-arrow-left text-xs"></i>
                RESPONSE
              </div>
              <div 
                :class="[
                  'rounded-lg p-3 border',
                  hasObservationError(observation)
                    ? 'bg-error/5 border-error/20'
                    : 'bg-success/5 border-success/20'
                ]"
              >
                <pre class="text-xs whitespace-pre-wrap break-words font-mono text-base-content">{{ formatObservation(observation) }}</pre>
              </div>
            </div>
          </div>
        </div>
      </details>
    </div>

    <!-- Error Section -->
    <div v-if="error" class="error-section mb-3">
      <div class="flex items-start gap-2 p-3 bg-error/5 rounded-lg border border-error/20">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-exclamation-triangle text-error text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-error mb-1">Error</div>
          <div class="text-sm text-error whitespace-pre-wrap">{{ error }}</div>
        </div>
      </div>
    </div>

    <!-- Final Answer Section -->
    <div v-if="finalAnswer" class="final-answer-section">
      <div class="flex items-start gap-2 p-3 bg-accent/5 rounded-lg border border-accent/20">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-flag-checkered text-accent text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-base-content/70 mb-1">Final Answer</div>
          <div class="text-sm text-base-content prose prose-sm max-w-none" v-html="renderMarkdown(finalAnswer)"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import { ReActMessageProcessor } from '../../composables/processors/ReActMessageProcessor'
import type { ChatMessage } from '../../types/chat'
import type { ReActStepDisplay } from '../../types/react'

const props = defineProps<{
  message?: ChatMessage
  stepData?: {
    thought?: string
    action?: any
    observation?: any
    error?: string
    finalAnswer?: string
  }
}>()

const { renderMarkdown } = useMessageUtils()

/**
 * 构建 ReAct 步骤显示数据
 * 优先从 message.architectureMeta 中读取结构化数据
 * 其次从 message.reactSteps 中读取遗留格式
 * 最后使用 stepData 进行向后兼容
 */
const steps = computed(() => {
  if (props.message) {
    return ReActMessageProcessor.buildReActStepsFromMessage(props.message)
  }

  // 向后兼容：从 stepData 构造单步数组
  if (props.stepData) {
    return [{
      index: 0,
      thought: props.stepData.thought,
      action: props.stepData.action,
      observation: props.stepData.observation,
      error: props.stepData.error,
      finalAnswer: props.stepData.finalAnswer,
    } as ReActStepDisplay]
  }

  return []
})

// 获取当前步骤（如果有多个步骤，取第一个用于显示；否则返回 null）
const currentStep = computed(() => {
  return steps.value.length > 0 ? steps.value[0] : null
})

const thought = computed(() => currentStep.value?.thought)
const action = computed(() => currentStep.value?.action)
const observation = computed(() => currentStep.value?.observation)
const error = computed(() => currentStep.value?.error)
const finalAnswer = computed(() => currentStep.value?.finalAnswer)

const formatJson = (obj: any) => ReActMessageProcessor.formatJson(obj)

const formatObservation = (obs: any) => ReActMessageProcessor.formatObservation(obs)

const isToolCallInProgress = () => {
  if (!action.value) return false
  const status = action.value.status
  // 只有在运行中或待处理时才展开，完成、成功、失败或错误时都折叠
  return status === 'running' || status === 'pending'
}

const getToolCallBorderClass = () => {
  if (!action.value) return 'border-base-300'
  const status = action.value.status
  if (status === 'failed' || status === 'error') {
    return 'border-error'
  }
  if (status === 'running') {
    return 'border-warning'
  }
  return 'border-success'
}

const getToolCallIconClass = () => {
  if (!action.value) return 'fas fa-check text-success'
  const status = action.value.status
  if (status === 'failed' || status === 'error') {
    return 'fas fa-times-circle text-error'
  }
  if (status === 'running') {
    return 'fas fa-spinner fa-spin text-warning'
  }
  return 'fas fa-check text-success'
}

const getActionStatusClass = (status: string) => {
  const statusMap: Record<string, string> = {
    running: 'badge-warning',
    success: 'badge-success',
    completed: 'badge-success',
    failed: 'badge-error',
    error: 'badge-error',
  }
  return statusMap[status.toLowerCase()] || 'badge-ghost'
}

const getActionStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    running: '运行中',
    success: '成功',
    completed: '已完成',
    failed: '失败',
    error: '错误',
  }
  return textMap[status.toLowerCase()] || status
}

const hasObservationError = (obs: any) => ReActMessageProcessor.hasObservationError(obs)
</script>

<style scoped>
.react-step-display {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* DaisyUI collapse customization */
.collapse {
  border-width: 2px;
}

.collapse-title {
  padding-right: 3rem;
}

.collapse:not(.collapse-close) > .collapse-title:after {
  top: 50%;
  transform: translateY(-50%) rotate(0deg);
  transition: transform 0.2s ease;
}

.collapse[open] > .collapse-title:after {
  transform: translateY(-50%) rotate(90deg);
}

/* Custom scrollbar */
.response-section pre {
  max-height: 300px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--bc) / 0.2) transparent;
}

.response-section pre::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.response-section pre::-webkit-scrollbar-track {
  background: transparent;
}

.response-section pre::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

.response-section pre::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}

/* Hover effects */
.collapse:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* Parameters grid layout */
.params-section > div > div {
  display: flex;
  gap: 0.5rem;
}

/* Smooth transitions */
.collapse {
  transition: all 0.2s ease;
}

.collapse-title {
  transition: background-color 0.15s ease;
}
</style>
