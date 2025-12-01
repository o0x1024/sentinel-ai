<template>
  <div class="travel-display">
    <!-- OODA 循环列表 -->
    <template v-for="(cycle, cycleIdx) in cycles" :key="cycleIdx">
      <!-- 循环标题 -->
      <div class="cycle-header">
        <i class="fas fa-sync-alt"></i>
        <span>OODA 循环 #{{ cycle.cycle }}</span>
        <span class="badge" :class="getStatusClass(cycle.status)">
          {{ getStatusText(cycle.status) }}
        </span>
      </div>

      <!-- 阶段列表 -->
      <template v-for="(phase, phaseIdx) in cycle.phases" :key="`${cycleIdx}-${phaseIdx}`">
        <!-- 思考内容 -->
        <div v-for="(thought, tIdx) in phase.thoughts" :key="`thought-${tIdx}`" class="thought-text">
          <div class="prose prose-sm max-w-none" v-html="renderMarkdown(thought)"></div>
        </div>

        <!-- 工具调用卡片 -->
        <div v-for="(action, aIdx) in phase.actions" :key="`action-${aIdx}`" 
             class="tool-block" :class="getToolClass(action)">
          <div class="tool-header" @click="toggleTool(`${cycleIdx}-${phaseIdx}-${aIdx}`)">
            <i :class="getToolStatusIcon(action)"></i>
            <span class="tool-status">{{ getToolStatusText(action) }}</span>
            <code class="tool-name">{{ action.tool }}</code>
            <i class="fas fa-chevron-right toggle-icon" 
               :class="{ expanded: expandedTools.has(`${cycleIdx}-${phaseIdx}-${aIdx}`) }"></i>
          </div>
          
          <div v-if="expandedTools.has(`${cycleIdx}-${phaseIdx}-${aIdx}`)" class="tool-body">
            <div v-if="hasArgs(action.args)" class="tool-section">
              <div class="section-title">Parameters</div>
              <pre class="section-code">{{ formatJson(action.args) }}</pre>
            </div>
            <div v-if="action.result !== undefined" class="tool-section">
              <div class="section-title">Result</div>
              <pre class="section-code" :class="{ error: hasError(action.result) }">{{ formatJson(action.result) }}</pre>
            </div>
          </div>
        </div>

        <!-- 阶段错误 -->
        <div v-if="phase.error" class="error-text">
          <i class="fas fa-exclamation-circle"></i>
          {{ phase.error }}
        </div>
      </template>
    </template>

    <!-- Loading -->
    <div v-if="isExecuting" class="loading-text">
      <span class="loading loading-dots loading-xs"></span>
      执行中...
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import { TravelMessageProcessor } from '../../composables/processors/TravelMessageProcessor'
import type { TravelCycleDisplay, TravelActionDisplay } from '../../composables/processors/TravelMessageProcessor'
import type { ChatMessage } from '../../types/chat'

const { renderMarkdown } = useMessageUtils()

const props = defineProps<{
  message?: ChatMessage
  stepData?: any
  isExecuting?: boolean
}>()

const expandedTools = ref<Set<string>>(new Set())

const toggleTool = (key: string) => {
  const newSet = new Set(expandedTools.value)
  if (newSet.has(key)) {
    newSet.delete(key)
  } else {
    newSet.add(key)
  }
  expandedTools.value = newSet
}

const cycles = computed((): TravelCycleDisplay[] => {
  if (props.message) {
    return TravelMessageProcessor.buildCyclesFromMessage(props.message)
  }
  return []
})

// 状态样式
const getStatusClass = (status: string) => {
  const map: Record<string, string> = {
    running: 'badge-warning',
    completed: 'badge-success',
    failed: 'badge-error',
  }
  return map[status] || 'badge-ghost'
}

const getStatusText = (status: string) => TravelMessageProcessor.getStatusText(status)

const getToolClass = (action: TravelActionDisplay) => {
  if (action.status === 'failed' || TravelMessageProcessor.hasError(action.result)) return 'tool-failed'
  if (action.status === 'completed') return 'tool-completed'
  return 'tool-running'
}

const getToolStatusIcon = (action: TravelActionDisplay) => {
  if (action.status === 'failed' || TravelMessageProcessor.hasError(action.result)) return 'fas fa-times-circle text-error'
  if (action.status === 'completed') return 'fas fa-check-circle text-success'
  return 'fas fa-circle-notch fa-spin text-warning'
}

const getToolStatusText = (action: TravelActionDisplay) => {
  if (action.status === 'failed' || TravelMessageProcessor.hasError(action.result)) return 'Failed'
  if (action.status === 'completed') return 'Ran'
  return 'Running'
}

// 数据处理
const hasArgs = (args: any) => args && typeof args === 'object' && Object.keys(args).length > 0
const hasError = (result: any) => TravelMessageProcessor.hasError(result)
const formatJson = (obj: any) => TravelMessageProcessor.formatJson(obj)
</script>

<style scoped>
.travel-display {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* 循环标题 */
.cycle-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: hsl(var(--bc) / 0.04);
  border-radius: 6px;
  font-size: 0.875rem;
  font-weight: 500;
}

.cycle-header i {
  color: hsl(var(--p));
}

.badge {
  font-size: 0.75rem;
  padding: 0.125rem 0.5rem;
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
  font-size: 0.8125rem;
}

.tool-header:hover {
  background: hsl(var(--bc) / 0.04);
}

.tool-header i:first-child {
  font-size: 0.75rem;
}

.tool-status {
  color: hsl(var(--bc) / 0.5);
}

.tool-name {
  font-family: ui-monospace, monospace;
  font-size: 0.8125rem;
  color: hsl(var(--bc) / 0.85);
  background: hsl(var(--bc) / 0.06);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.toggle-icon {
  margin-left: auto;
  font-size: 0.625rem;
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
  font-size: 0.6875rem;
  font-weight: 500;
  color: hsl(var(--bc) / 0.45);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin-bottom: 0.375rem;
}

.section-code {
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
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
  font-size: 0.8125rem;
}

/* Loading */
.loading-text {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: hsl(var(--bc) / 0.5);
  font-size: 0.8125rem;
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
