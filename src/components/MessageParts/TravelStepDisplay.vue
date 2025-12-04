<template>
  <div class="travel-display">
    <!-- VisionExplorer 嵌入数据（如果有） -->
    <div v-if="visionIterations.length > 0" class="vision-embedded">
      <div class="vision-header">
        <i class="fas fa-eye"></i>
        <span>VisionExplorer 前置探索</span>
        <span class="badge badge-info">{{ visionIterations.length }} 迭代</span>
      </div>
      <template v-for="(iter, iterIdx) in visionIterations" :key="`vision-${iterIdx}`">
        <div class="vision-iteration">
          <div class="iteration-label">
            迭代 #{{ iter.iteration }}
            <span v-if="iter.url" class="url-text">{{ truncateUrl(iter.url) }}</span>
          </div>
          <template v-for="(phase, pIdx) in iter.phases" :key="`vp-${pIdx}`">
            <div v-if="phase.phase === 'analyze' && phase.analysis" class="vision-analysis">
              <span class="analysis-icon">🧠</span>
              <span class="analysis-text">{{ phase.analysis.page_analysis }}</span>
              <span v-if="phase.analysis.exploration_progress" class="progress-text">
                ({{ Math.round(phase.analysis.exploration_progress * 100) }}%)
              </span>
            </div>
            <div v-if="phase.phase === 'action' && phase.action" class="vision-action" :class="{ success: phase.action.success, failed: !phase.action.success }">
              <span class="action-icon">{{ phase.action.success ? '✅' : '❌' }}</span>
              <span class="action-type">{{ phase.action.action_type }}</span>
              <span v-if="phase.action.element_index !== undefined" class="element-idx">[{{ phase.action.element_index }}]</span>
              <span class="action-reason">- {{ phase.action.reason }}</span>
            </div>
          </template>
        </div>
      </template>
    </div>

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
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import { TravelMessageProcessor } from '../../composables/processors/TravelMessageProcessor'
import { VisionExplorerMessageProcessor } from '../../composables/processors/VisionExplorerMessageProcessor'
import type { TravelCycleDisplay, TravelActionDisplay } from '../../composables/processors/TravelMessageProcessor'
import type { VisionIterationDisplay } from '../../composables/processors/VisionExplorerMessageProcessor'
import type { ChatMessage } from '../../types/chat'
import type { OrderedMessageChunk } from '../../types/ordered-chat'

const { renderMarkdown } = useMessageUtils()

const props = defineProps<{
  message?: ChatMessage
  chunks?: OrderedMessageChunk[]
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
  // 优先从 message 中获取（历史消息）
  if (props.message) {
    const fromMessage = TravelMessageProcessor.buildCyclesFromMessage(props.message)
    if (fromMessage.length > 0) {
      return fromMessage
    }
  }
  // 从 chunks 中实时提取（流式消息）
  if (props.chunks && props.chunks.length > 0) {
    return TravelMessageProcessor.extractCyclesFromChunks(props.chunks)
  }
  return []
})

// 提取嵌入的 VisionExplorer 迭代数据
const visionIterations = computed((): VisionIterationDisplay[] => {
  // 优先从 message 中获取
  if (props.message && (props.message as any).visionIterations?.length > 0) {
    return (props.message as any).visionIterations
  }
  // 从 chunks 中实时提取
  if (props.chunks && props.chunks.length > 0) {
    return VisionExplorerMessageProcessor.extractIterationsFromChunks(props.chunks)
  }
  return []
})

// 截断 URL 显示
const truncateUrl = (url: string, maxLen = 50) => {
  if (url.length <= maxLen) return url
  return url.substring(0, maxLen - 3) + '...'
}

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

/* VisionExplorer 嵌入区域 */
.vision-embedded {
  border: 1px solid hsl(var(--in) / 0.2);
  border-radius: 8px;
  background: hsl(var(--in) / 0.03);
  padding: 0.75rem;
  margin-bottom: 0.5rem;
}

.vision-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  font-weight: 500;
  color: hsl(var(--in));
  margin-bottom: 0.75rem;
}

.vision-header i {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

.vision-iteration {
  padding: 0.5rem 0;
  border-bottom: 1px solid hsl(var(--bc) / 0.06);
}

.vision-iteration:last-child {
  border-bottom: none;
}

.iteration-label {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  font-weight: 500;
  color: hsl(var(--bc) / 0.7);
  margin-bottom: 0.375rem;
}

.url-text {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.5);
  margin-left: 0.5rem;
}

.vision-analysis {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: hsl(var(--bc) / 0.02);
  border-radius: 4px;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  margin-bottom: 0.25rem;
}

.analysis-icon {
  flex-shrink: 0;
}

.analysis-text {
  color: hsl(var(--bc) / 0.8);
  line-height: 1.5;
}

.progress-text {
  color: hsl(var(--bc) / 0.5);
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

.vision-action {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.5rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  margin-bottom: 0.125rem;
}

.vision-action.success {
  color: hsl(var(--su) / 0.9);
}

.vision-action.failed {
  color: hsl(var(--er) / 0.9);
}

.action-icon {
  flex-shrink: 0;
}

.action-type {
  font-weight: 500;
}

.element-idx {
  font-family: ui-monospace, monospace;
  background: hsl(var(--bc) / 0.08);
  padding: 0 0.25rem;
  border-radius: 2px;
}

.action-reason {
  color: hsl(var(--bc) / 0.6);
  font-style: italic;
}

/* 循环标题 */
.cycle-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: hsl(var(--bc) / 0.04);
  border-radius: 6px;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  font-weight: 500;
}

.cycle-header i {
  color: hsl(var(--p));
}

.badge {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
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
