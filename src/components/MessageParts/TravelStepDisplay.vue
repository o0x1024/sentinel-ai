<template>
  <div class="travel-display">
    <!-- 调试信息 -->
    <div v-if="debugMode" class="debug-info">
      <div>🔧 TravelStepDisplay rendered</div>
      <div>Chunks: {{ props.chunks?.length || 0 }}</div>
      <div>OODA Cycles: {{ cycles.length }}</div>
      <div>Vision Iterations: {{ visionIterations.length }}</div>
      <div>isExecuting: {{ isExecuting }}</div>
    </div>
    
    <!-- 嵌入的 VisionExplorer 迭代（在 OODA 循环之前显示） -->
    <div v-if="visionIterations.length > 0" class="vision-embedded">
      <div class="vision-header" @click="toggleVisionSection">
        <i class="fas fa-eye"></i>
        <span>VisionExplorer 前置探索</span>
        <span class="badge badge-info">{{ visionIterations.length }} 迭代</span>
        <i class="fas fa-chevron-right toggle-icon" :class="{ expanded: visionExpanded }"></i>
      </div>
      
      <div v-if="visionExpanded" class="vision-content">
      <template v-for="(iter, iterIdx) in visionIterations" :key="`vision-${iterIdx}`">
        <div class="vision-iteration">
          <div class="iteration-label">
              <span class="iteration-num">迭代 #{{ iter.iteration }}</span>
              <span v-if="iter.url" class="url-text" :title="iter.url">{{ truncateUrl(iter.url) }}</span>
              <span class="badge badge-sm" :class="getStatusClass(iter.status)">
                {{ getStatusText(iter.status) }}
              </span>
            </div>
            
            <template v-for="(phase, pIdx) in iter.phases" :key="`vp-${iterIdx}-${pIdx}`">
              <!-- 分析阶段 -->
              <div v-if="phase.phase === 'analyze' && phase.analysis" class="vision-phase analyze">
                <span class="phase-icon">🧠</span>
                <div class="phase-content">
                  <div class="analysis-text">{{ phase.analysis.page_analysis }}</div>
                  <div class="analysis-meta">
                    <span v-if="phase.analysis.exploration_progress" class="progress-badge">
                      {{ Math.round(phase.analysis.exploration_progress * 100) }}%
                    </span>
                    <span v-if="phase.analysis.estimated_apis?.length" class="apis-count">
                      {{ phase.analysis.estimated_apis.length }} API
                    </span>
                  </div>
                </div>
              </div>
              
              <!-- 操作阶段 -->
              <div v-if="phase.phase === 'action' && phase.action" 
                   class="vision-phase action" 
                   :class="{ success: phase.action.success, failed: !phase.action.success }">
                <span class="phase-icon">{{ phase.action.success ? '✅' : '❌' }}</span>
                <div class="phase-content">
              <span class="action-type">{{ phase.action.action_type }}</span>
              <span v-if="phase.action.element_index !== undefined" class="element-idx">[{{ phase.action.element_index }}]</span>
                  <span class="action-reason">{{ phase.action.reason }}</span>
                  <span v-if="phase.action.duration_ms" class="duration-badge">{{ phase.action.duration_ms }}ms</span>
                </div>
              </div>
              
              <!-- 错误 -->
              <div v-if="phase.error" class="vision-error">
                <i class="fas fa-exclamation-circle"></i>
                {{ phase.error }}
            </div>
          </template>
        </div>
      </template>
      </div>
    </div>

    <!-- OODA 循环列表 -->
    <template v-for="(cycle, cycleIdx) in cycles" :key="cycleIdx">
      <!-- 循环标题 -->
      <div class="cycle-header">
        <div class="cycle-title">
        <i class="fas fa-sync-alt"></i>
        <span>OODA 循环 #{{ cycle.cycle }}</span>
        </div>
        <span class="badge" :class="getStatusClass(cycle.status)">
          {{ getStatusText(cycle.status) }}
        </span>
      </div>

      <!-- 阶段列表 -->
      <template v-for="(phase, phaseIdx) in cycle.phases" :key="`${cycleIdx}-${phaseIdx}`">
        <!-- 阶段标题 -->
        <div class="phase-header" :class="`phase-${phase.phase.toLowerCase()}`">
          <i :class="getPhaseIcon(phase.phase)"></i>
          <span class="phase-name">{{ getPhaseName(phase.phase) }}</span>
          <span v-if="phase.reactIteration" class="iteration-badge">Iter #{{ phase.reactIteration }}</span>
          <span class="badge badge-sm" :class="getStatusClass(phase.status)">
            {{ getStatusText(phase.status) }}
          </span>
        </div>

        <!-- 思考内容 -->
        <div v-for="(thought, tIdx) in phase.thoughts" :key="`thought-${cycleIdx}-${phaseIdx}-${tIdx}`" 
             class="thought-block">
          <div class="prose prose-sm max-w-none" v-html="renderMarkdown(thought)"></div>
        </div>

        <!-- 工具调用卡片 -->
        <div v-for="(action, aIdx) in phase.actions" :key="`action-${cycleIdx}-${phaseIdx}-${aIdx}`" 
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
              <pre class="section-code" :class="{ error: hasError(action.result) }">{{ formatResult(action.result) }}</pre>
            </div>
          </div>
        </div>

        <!-- 阶段输出 -->
        <div v-if="phase.output" class="output-block">
          <div class="output-header">
            <i class="fas fa-arrow-right"></i>
            <span>输出</span>
          </div>
          <pre class="output-code">{{ formatJson(phase.output) }}</pre>
        </div>

        <!-- 阶段错误 -->
        <div v-if="phase.error" class="error-block">
          <i class="fas fa-exclamation-circle"></i>
          {{ phase.error }}
        </div>
      </template>
    </template>

    <!-- 执行统计 -->
    <div v-if="stats" class="stats-block">
      <div class="stats-header">
        <i class="fas fa-chart-bar"></i>
        <span>执行统计</span>
      </div>
      <div class="stats-grid">
        <div class="stat-item">
          <span class="stat-value">{{ stats.total_iterations }}</span>
          <span class="stat-label">迭代次数</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.tool_calls_count }}</span>
          <span class="stat-label">工具调用</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.successful_tool_calls }}</span>
          <span class="stat-label">成功</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.failed_tool_calls }}</span>
          <span class="stat-label">失败</span>
        </div>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="isExecuting && (cycles.length > 0 || visionIterations.length > 0)" class="loading-block">
      <span class="loading loading-dots loading-sm"></span>
      <span>执行中...</span>
    </div>
    
    <!-- 空状态 - 等待数据 -->
    <div v-if="isExecuting && cycles.length === 0 && visionIterations.length === 0" class="empty-state">
      <span class="loading loading-spinner loading-sm"></span>
      <span>等待执行数据...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import { TravelMessageProcessor } from '../../composables/processors/TravelMessageProcessor'
import type { 
  TravelCycleDisplay, 
  TravelActionDisplay,
  EmbeddedVisionIteration,
  TravelExecutionStats 
} from '../../composables/processors/TravelMessageProcessor'
import type { ChatMessage } from '../../types/chat'
import type { OrderedMessageChunk } from '../../types/ordered-chat'

const { renderMarkdown } = useMessageUtils()

const props = defineProps<{
  message?: ChatMessage
  chunks?: OrderedMessageChunk[]
  stepData?: any
  isExecuting?: boolean
}>()

// 展开状态
const expandedTools = ref<Set<string>>(new Set())
const visionExpanded = ref(true)
// 调试模式 - 开发时设为 true 可以看到调试信息
const debugMode = ref(true)

const toggleTool = (key: string) => {
  const newSet = new Set(expandedTools.value)
  if (newSet.has(key)) {
    newSet.delete(key)
  } else {
    newSet.add(key)
  }
  expandedTools.value = newSet
}

const toggleVisionSection = () => {
  visionExpanded.value = !visionExpanded.value
}

// 从 chunks 或 message 提取 OODA 循环
const cycles = computed((): TravelCycleDisplay[] => {
  // 流式消息：从 chunks 中实时提取
  if (props.chunks && props.chunks.length > 0) {
    const result = TravelMessageProcessor.extractCyclesFromChunks(props.chunks)
    console.log('[TravelStepDisplay] cycles extracted:', result.length, 'from', props.chunks.length, 'chunks')
    return result
  }
  // 历史消息：从 message 中获取已保存的数据
  if (props.message) {
    return TravelMessageProcessor.buildCyclesFromMessage(props.message)
  }
  return []
})

// 提取嵌入的 VisionExplorer 迭代数据
const visionIterations = computed((): EmbeddedVisionIteration[] => {
  // 流式消息
  if (props.chunks && props.chunks.length > 0) {
    const result = TravelMessageProcessor.extractEmbeddedVisionFromChunks(props.chunks)
    console.log('[TravelStepDisplay] visionIterations extracted:', result.length, 'from', props.chunks.length, 'chunks')
    // 详细日志：检查每个chunk的structured_data
    if (result.length === 0) {
      const metaChunks = props.chunks.filter(c => c.chunk_type === 'Meta')
      console.log('[TravelStepDisplay] Meta chunks:', metaChunks.length)
      metaChunks.forEach((c, i) => {
        console.log(`[TravelStepDisplay] Meta chunk ${i}:`, {
          stage: c.stage,
          architecture: c.architecture,
          has_sd: !!c.structured_data,
          sd_type: (c.structured_data as any)?.type
        })
      })
    }
    return result
  }
  // 历史消息
  if (props.message && (props.message as any).visionIterations?.length > 0) {
    return (props.message as any).visionIterations
  }
  return []
})

// 提取执行统计
const stats = computed((): TravelExecutionStats | null => {
  if (props.chunks && props.chunks.length > 0) {
    return TravelMessageProcessor.extractStatsFromChunks(props.chunks)
  }
  return null
})

// 截断 URL 显示
const truncateUrl = (url: string, maxLen = 40) => {
  try {
    const parsed = new URL(url)
    const display = parsed.hostname + parsed.pathname
    if (display.length > maxLen) {
      return display.substring(0, maxLen - 3) + '...'
    }
    return display
  } catch {
    if (url.length > maxLen) {
  return url.substring(0, maxLen - 3) + '...'
    }
    return url
  }
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
const getPhaseIcon = (phase: string) => TravelMessageProcessor.getPhaseIcon(phase)
const getPhaseName = (phase: string) => TravelMessageProcessor.getPhaseName(phase)

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

// 格式化结果（限制长度）
const formatResult = (result: any) => {
  const str = formatJson(result)
  if (str.length > 1000) {
    return str.substring(0, 1000) + '\n... (truncated)'
  }
  return str
}
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
  overflow: hidden;
}

.vision-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  font-weight: 500;
  color: hsl(var(--in));
  cursor: pointer;
  transition: background 0.15s;
}

.vision-header:hover {
  background: hsl(var(--in) / 0.05);
}

.vision-header i:first-child {
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.vision-content {
  border-top: 1px solid hsl(var(--in) / 0.1);
  padding: 0.5rem 0.75rem;
}

.vision-iteration {
  padding: 0.5rem 0;
  border-bottom: 1px solid hsl(var(--bc) / 0.06);
}

.vision-iteration:last-child {
  border-bottom: none;
}

.iteration-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  margin-bottom: 0.5rem;
}

.iteration-num {
  font-weight: 500;
  color: hsl(var(--bc) / 0.8);
}

.url-text {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.5);
  background: hsl(var(--bc) / 0.05);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vision-phase {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  margin-bottom: 0.25rem;
  border-radius: 6px;
}

.vision-phase.analyze {
  background: hsl(var(--bc) / 0.02);
}

.vision-phase.action {
  background: hsl(var(--bc) / 0.02);
}

.vision-phase.action.success {
  border-left: 2px solid hsl(var(--su) / 0.5);
}

.vision-phase.action.failed {
  border-left: 2px solid hsl(var(--er) / 0.5);
}

.phase-icon {
  flex-shrink: 0;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.phase-content {
  flex: 1;
  min-width: 0;
}

.analysis-text {
  color: hsl(var(--bc) / 0.8);
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  line-height: 1.5;
}

.analysis-meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.25rem;
}

.progress-badge {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  background: hsl(var(--su) / 0.15);
  color: hsl(var(--su));
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.apis-count {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.5);
}

.action-type {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  font-weight: 500;
  color: hsl(var(--bc) / 0.85);
  background: hsl(var(--bc) / 0.08);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.element-idx {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--p));
  font-weight: 600;
}

.action-reason {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.6);
  margin-left: 0.25rem;
}

.duration-badge {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.4);
  margin-left: auto;
}

.vision-error {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.5rem;
  background: hsl(var(--er) / 0.06);
  border-radius: 4px;
  color: hsl(var(--er));
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

/* 循环标题 */
.cycle-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.625rem 0.75rem;
  background: linear-gradient(135deg, hsl(var(--p) / 0.08), hsl(var(--s) / 0.05));
  border-radius: 8px;
  border: 1px solid hsl(var(--p) / 0.1);
}

.cycle-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  font-weight: 500;
}

.cycle-title i {
  color: hsl(var(--p));
}

.badge {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  padding: 0.125rem 0.5rem;
}

/* 阶段标题 */
.phase-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: hsl(var(--bc) / 0.03);
  border-radius: 6px;
  border-left: 3px solid hsl(var(--bc) / 0.2);
  margin-top: 0.5rem;
}

.phase-header.phase-observe { border-left-color: hsl(var(--in)); }
.phase-header.phase-orient { border-left-color: hsl(var(--wa)); }
.phase-header.phase-decide { border-left-color: hsl(var(--p)); }
.phase-header.phase-act { border-left-color: hsl(var(--su)); }

.phase-header i {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.6);
}

.phase-name {
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  font-weight: 500;
  color: hsl(var(--bc) / 0.85);
}

.iteration-badge {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.5);
  background: hsl(var(--bc) / 0.08);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.phase-header .badge {
  margin-left: auto;
}

/* Thought */
.thought-block {
  padding: 0.5rem 0.75rem;
  color: hsl(var(--bc) / 0.9);
  line-height: 1.65;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

/* 工具块 */
.tool-block {
  border-radius: 6px;
  border: 1px solid hsl(var(--bc) / 0.1);
  background: hsl(var(--bc) / 0.02);
  overflow: hidden;
  margin: 0.375rem 0;
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  cursor: pointer;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  transition: background 0.15s;
}

.tool-header:hover {
  background: hsl(var(--bc) / 0.04);
}

.tool-header i:first-child {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

.tool-status {
  color: hsl(var(--bc) / 0.5);
  font-size: calc(var(--font-size-base, 14px) * 0.75);
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
  max-height: 250px;
  overflow: auto;
}

.section-code.error {
  color: hsl(var(--er));
  background: hsl(var(--er) / 0.06);
}

/* 输出块 */
.output-block {
  margin: 0.375rem 0;
  border-radius: 6px;
  border: 1px solid hsl(var(--su) / 0.2);
  overflow: hidden;
}

.output-header {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  background: hsl(var(--su) / 0.05);
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  font-weight: 500;
  color: hsl(var(--su));
}

.output-code {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  padding: 0.5rem 0.75rem;
  margin: 0;
  background: hsl(var(--bc) / 0.02);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow: auto;
}

/* Error */
.error-block {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: hsl(var(--er) / 0.06);
  border-radius: 6px;
  color: hsl(var(--er));
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  margin: 0.375rem 0;
}

/* 统计块 */
.stats-block {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background: linear-gradient(135deg, hsl(var(--su) / 0.05), hsl(var(--p) / 0.03));
  border-radius: 8px;
  border: 1px solid hsl(var(--su) / 0.2);
}

.stats-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  font-weight: 500;
  color: hsl(var(--su));
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1rem;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.stat-value {
  font-size: calc(var(--font-size-base, 14px) * 1.25);
  font-weight: 700;
  color: hsl(var(--bc));
}

.stat-label {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  color: hsl(var(--bc) / 0.6);
}

/* Loading & Empty */
.loading-block,
.empty-state {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: hsl(var(--bc) / 0.5);
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  padding: 0.5rem;
}

/* 滚动条 */
.section-code::-webkit-scrollbar,
.output-code::-webkit-scrollbar {
  width: 4px;
  height: 4px;
}
.section-code::-webkit-scrollbar-thumb,
.output-code::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.15);
  border-radius: 2px;
}

/* 调试信息 */
.debug-info {
  background: hsl(var(--wa) / 0.1);
  border: 1px dashed hsl(var(--wa));
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--wa));
  margin-bottom: 0.5rem;
}

/* 响应式 */
@media (max-width: 640px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
