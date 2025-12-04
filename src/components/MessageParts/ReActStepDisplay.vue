<template>
  <div class="react-display">
    <template v-for="(step, idx) in steps" :key="step.id || idx">
      <!-- Thought -->
      <div v-if="step.thought" class="thought-text">
        <div class="prose prose-sm max-w-none" v-html="renderMarkdown(step.thought)"></div>
      </div>

      <!-- 工具调用卡片 -->
      <div v-if="step.action" class="tool-block" :class="getToolClass(step)">
        <div class="tool-header" @click="toggleTool(idx)">
          <i :class="getStatusIcon(step)"></i>
          <span class="tool-status">{{ getStatusText(step) }}</span>
          <code class="tool-name">{{ step.action.tool }}</code>
          <i class="fas fa-chevron-right toggle-icon" :class="{ expanded: expandedTools.has(idx) }"></i>
        </div>
        
        <div v-if="expandedTools.has(idx)" class="tool-body">
          <div v-if="hasArgs(step.action.args)" class="tool-section">
            <div class="section-title">Parameters</div>
            <pre class="section-code">{{ formatJson(step.action.args) }}</pre>
          </div>
          <div v-if="step.observation !== undefined" class="tool-section">
            <div class="section-title">Result</div>
            <pre class="section-code" :class="{ error: hasError(step.observation) }">{{ formatObservation(step.observation) }}</pre>
          </div>
        </div>
      </div>

      <!-- Error -->
      <div v-if="step.error" class="error-text">
        <i class="fas fa-exclamation-circle"></i>
        {{ step.error }}
      </div>

      <!-- Final Answer -->
      <div v-if="step.finalAnswer" class="answer-text">
        <div class="prose prose-sm max-w-none" v-html="renderMarkdown(step.finalAnswer)"></div>
      </div>
    </template>

    <!-- Loading -->
    <div v-if="isExecuting" class="loading-text">
      <span class="loading loading-dots loading-xs"></span>
      思考中...
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import { ReActMessageProcessor } from '../../composables/processors/ReActMessageProcessor'
import type { ChatMessage } from '../../types/chat'
import type { ReActStepDisplay } from '../../types/react'

const { renderMarkdown } = useMessageUtils()

const props = defineProps<{
  message?: ChatMessage
  stepData?: any
  showHeader?: boolean
  isExecuting?: boolean
}>()

const expandedTools = ref<Set<number>>(new Set())

const toggleTool = (idx: number) => {
  const newSet = new Set(expandedTools.value)
  if (newSet.has(idx)) {
    newSet.delete(idx)
  } else {
    newSet.add(idx)
  }
  expandedTools.value = newSet
}

const steps = computed((): ReActStepDisplay[] => {
  if (props.message) {
    return ReActMessageProcessor.buildReActStepsFromMessage(props.message)
  }
  if (props.stepData) {
    return [{ index: 0, ...props.stepData }]
  }
  return []
})

// 状态判断
const isCompleted = (step: ReActStepDisplay) => {
  return step.observation !== undefined || step.action?.status === 'completed'
}

const isFailed = (step: ReActStepDisplay) => {
  return step.action?.status === 'failed' || ReActMessageProcessor.hasObservationError(step.observation)
}

// 样式
const getToolClass = (step: ReActStepDisplay) => {
  if (isFailed(step)) return 'tool-failed'
  if (isCompleted(step)) return 'tool-completed'
  return 'tool-running'
}

const getStatusIcon = (step: ReActStepDisplay) => {
  if (isFailed(step)) return 'fas fa-times-circle text-error'
  if (isCompleted(step)) return 'fas fa-check-circle text-success'
  return 'fas fa-circle-notch fa-spin text-warning'
}

const getStatusText = (step: ReActStepDisplay) => {
  if (isFailed(step)) return 'Failed'
  if (isCompleted(step)) return 'Ran'
  return 'Running'
}

// 数据处理
const hasArgs = (args: any) => args && typeof args === 'object' && Object.keys(args).length > 0
const hasError = (obs: any) => ReActMessageProcessor.hasObservationError(obs)
const formatJson = (obj: any) => ReActMessageProcessor.formatJson(obj)
const formatObservation = (obs: any) => ReActMessageProcessor.formatObservation(obs)
</script>

<style scoped>
.react-display {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* Thought */
.thought-text {
  color: hsl(var(--bc) / 0.9);
  line-height: 1.65;
}

/* 工具块 */
.tool-block {
  border-radius: 6px;
  border: 1px solid hsl(var(--bc) / 0.1);
  background: hsl(var(--bc) / 0.02);
  overflow: hidden;
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  cursor: pointer;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
}

.tool-header:hover {
  background: hsl(var(--bc) / 0.04);
}

.tool-header i:first-child {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

.tool-status {
  color: hsl(var(--bc) / 0.5);
}

.tool-name {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  color: hsl(var(--bc) / 0.85);
  background: hsl(var(--bc) / 0.06);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.toggle-icon {
  margin-left: auto;
  font-size: calc(var(--font-size-base, 14px) * 0.625);
  color: hsl(var(--bc) / 0.3);
  transition: transform 0.15s;
}

.toggle-icon.expanded {
  transform: rotate(90deg);
}

/* 工具状态样式 */
.tool-running { border-color: hsl(var(--wa) / 0.3); }
.tool-completed { border-color: hsl(var(--su) / 0.2); }
.tool-failed { border-color: hsl(var(--er) / 0.3); }

/* 工具详情 */
.tool-body {
  border-top: 1px solid hsl(var(--bc) / 0.08);
  padding: 0.75rem;
  background: hsl(var(--bc) / 0.02);
}

.tool-section {
  margin-bottom: 0.75rem;
}
.tool-section:last-child {
  margin-bottom: 0;
}

.section-title {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  font-weight: 500;
  color: hsl(var(--bc) / 0.45);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin-bottom: 0.375rem;
}

.section-code {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  line-height: 1.5;
  color: hsl(var(--bc) / 0.8);
  background: hsl(var(--bc) / 0.04);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow: auto;
}

.section-code.error {
  color: hsl(var(--er));
  background: hsl(var(--er) / 0.06);
}

/* Error */
.error-text {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: hsl(var(--er) / 0.06);
  border-radius: 6px;
  color: hsl(var(--er));
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
}

/* Final Answer */
.answer-text {
  color: hsl(var(--bc));
  line-height: 1.65;
}

/* Loading */
.loading-text {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: hsl(var(--bc) / 0.5);
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
}

/* 滚动条 */
.section-code::-webkit-scrollbar {
  width: 4px;
  height: 4px;
}
.section-code::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.15);
  border-radius: 2px;
}
</style>
