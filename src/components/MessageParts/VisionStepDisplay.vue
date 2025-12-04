<template>
  <div class="vision-display">
    <!-- 迭代列表 -->
    <template v-for="(iter, iterIdx) in iterations" :key="iterIdx">
      <!-- 迭代标题 -->
      <div class="iteration-header">
        <i class="fas fa-eye"></i>
        <span>迭代 #{{ iter.iteration }}</span>
        <span v-if="iter.url" class="url-badge" :title="iter.url">
          {{ truncateUrl(iter.url) }}
        </span>
        <span class="badge" :class="getStatusClass(iter.status)">
          {{ getStatusText(iter.status) }}
        </span>
      </div>

      <!-- 阶段列表 -->
      <template v-for="(phase, phaseIdx) in iter.phases" :key="`${iterIdx}-${phaseIdx}`">
        <!-- 截图阶段 -->
        <div v-if="phase.phase === 'screenshot'" class="phase-block screenshot">
          <div class="phase-header">
            <span class="phase-icon">📸</span>
            <span>截图完成</span>
            <span v-if="iter.title" class="page-title">{{ iter.title }}</span>
          </div>
        </div>

        <!-- 分析阶段 -->
        <div v-if="phase.phase === 'analyze' && phase.analysis" class="phase-block analyze">
          <div class="phase-header">
            <span class="phase-icon">🧠</span>
            <span>VLM 分析</span>
            <span class="progress-badge">{{ formatProgress(phase.analysis.exploration_progress) }}</span>
          </div>
          <div class="analysis-content">
            <div class="analysis-text">{{ phase.analysis.page_analysis }}</div>
            <div v-if="phase.analysis.estimated_apis?.length" class="apis-tag">
              <span class="api-label">预估 API:</span>
              <code v-for="(api, i) in phase.analysis.estimated_apis" :key="i" class="api-item">{{ api }}</code>
            </div>
          </div>
        </div>

        <!-- 操作阶段 -->
        <div v-if="phase.phase === 'action' && phase.action" 
             class="phase-block action" :class="getActionClass(phase.action)">
          <div class="phase-header" @click="toggleAction(`${iterIdx}-${phaseIdx}`)">
            <span class="phase-icon">{{ getActionIcon(phase.action) }}</span>
            <span class="action-type">{{ phase.action.action_type }}</span>
            <span v-if="phase.action.element_index !== undefined" class="element-badge">[{{ phase.action.element_index }}]</span>
            <span v-if="phase.action.duration_ms" class="duration-badge">{{ phase.action.duration_ms }}ms</span>
            <i class="fas fa-chevron-right toggle-icon" 
               :class="{ expanded: expandedActions.has(`${iterIdx}-${phaseIdx}`) }"></i>
          </div>
          
          <div v-if="expandedActions.has(`${iterIdx}-${phaseIdx}`)" class="action-body">
            <div v-if="phase.action.value" class="action-section">
              <div class="section-title">值</div>
              <code class="section-value">{{ phase.action.value }}</code>
            </div>
            <div class="action-section">
              <div class="section-title">原因</div>
              <div class="section-text">{{ phase.action.reason }}</div>
            </div>
          </div>
        </div>

        <!-- 错误 -->
        <div v-if="phase.error" class="error-text">
          <i class="fas fa-exclamation-circle"></i>
          {{ phase.error }}
        </div>
      </template>
    </template>

    <!-- 统计信息 -->
    <div v-if="stats" class="stats-block">
      <div class="stats-header">
        <i class="fas fa-chart-bar"></i>
        <span>探索统计</span>
      </div>
      <div class="stats-grid">
        <div class="stat-item">
          <span class="stat-value">{{ stats.total_iterations }}</span>
          <span class="stat-label">迭代次数</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.pages_visited }}</span>
          <span class="stat-label">访问页面</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.apis_discovered }}</span>
          <span class="stat-label">发现 API</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ stats.elements_interacted }}</span>
          <span class="stat-label">交互元素</span>
        </div>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="isExecuting" class="loading-text">
      <span class="loading loading-dots loading-xs"></span>
      <span>探索中...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { VisionExplorerMessageProcessor } from '../../composables/processors/VisionExplorerMessageProcessor'
import type { VisionIterationDisplay, VisionAction, VisionExplorationStats } from '../../composables/processors/VisionExplorerMessageProcessor'
import type { ChatMessage } from '../../types/chat'
import type { OrderedMessageChunk } from '../../types/ordered-chat'

const props = defineProps<{
  message?: ChatMessage
  chunks?: OrderedMessageChunk[]
  isExecuting?: boolean
}>()

const expandedActions = ref<Set<string>>(new Set())

const toggleAction = (key: string) => {
  const newSet = new Set(expandedActions.value)
  if (newSet.has(key)) {
    newSet.delete(key)
  } else {
    newSet.add(key)
  }
  expandedActions.value = newSet
}

const iterations = computed((): VisionIterationDisplay[] => {
  if (props.chunks && props.chunks.length > 0) {
    return VisionExplorerMessageProcessor.extractIterationsFromChunks(props.chunks)
  }
  if (props.message) {
    return VisionExplorerMessageProcessor.buildIterationsFromMessage(props.message)
  }
  return []
})

const stats = computed((): VisionExplorationStats | null => {
  if (props.chunks && props.chunks.length > 0) {
    return VisionExplorerMessageProcessor.extractStatsFromChunks(props.chunks)
  }
  return null
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

const getStatusText = (status: string) => {
  const map: Record<string, string> = {
    running: '进行中',
    completed: '完成',
    failed: '失败',
  }
  return map[status] || status
}

const getActionClass = (action: VisionAction) => {
  if (!action.success) return 'action-failed'
  return 'action-completed'
}

const getActionIcon = (action: VisionAction) => {
  if (!action.success) return '❌'
  switch (action.action_type) {
    case 'click_by_index': return '👆'
    case 'fill_by_index': return '✏️'
    case 'scroll': return '📜'
    case 'navigate': return '🔗'
    case 'screenshot': return '📸'
    case 'set_status': return '🏁'
    default: return '⚙️'
  }
}

const formatProgress = (progress: number) => {
  return `${Math.round(progress * 100)}%`
}

const truncateUrl = (url: string) => {
  try {
    const parsed = new URL(url)
    const path = parsed.pathname
    if (path.length > 30) {
      return parsed.hostname + path.substring(0, 27) + '...'
    }
    return parsed.hostname + path
  } catch {
    if (url.length > 40) {
      return url.substring(0, 37) + '...'
    }
    return url
  }
}
</script>

<style scoped>
.vision-display {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* 迭代标题 */
.iteration-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: linear-gradient(135deg, hsl(var(--p) / 0.08), hsl(var(--s) / 0.05));
  border-radius: 8px;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  font-weight: 500;
}

.iteration-header i {
  color: hsl(var(--p));
}

.url-badge {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.6);
  background: hsl(var(--bc) / 0.05);
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  padding: 0.125rem 0.5rem;
  margin-left: auto;
}

/* 阶段块 */
.phase-block {
  border-radius: 8px;
  border: 1px solid hsl(var(--bc) / 0.1);
  background: hsl(var(--bc) / 0.02);
  overflow: hidden;
}

.phase-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
}

.phase-icon {
  font-size: calc(var(--font-size-base, 14px) * 1);
}

.page-title {
  color: hsl(var(--bc) / 0.6);
  font-size: calc(var(--font-size-base, 14px) * 0.75);
}

.progress-badge {
  background: linear-gradient(135deg, hsl(var(--su) / 0.2), hsl(var(--p) / 0.1));
  color: hsl(var(--su));
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  font-weight: 600;
}

/* 分析内容 */
.analysis-content {
  padding: 0.5rem 0.75rem;
  border-top: 1px solid hsl(var(--bc) / 0.05);
}

.analysis-text {
  color: hsl(var(--bc) / 0.85);
  line-height: 1.6;
  margin-bottom: 0.5rem;
}

.apis-tag {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.375rem;
}

.api-label {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.5);
}

.api-item {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  background: hsl(var(--p) / 0.1);
  color: hsl(var(--p));
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

/* 操作块 */
.phase-block.action .phase-header {
  cursor: pointer;
}

.phase-block.action .phase-header:hover {
  background: hsl(var(--bc) / 0.04);
}

.action-type {
  font-family: ui-monospace, monospace;
  color: hsl(var(--bc) / 0.85);
  background: hsl(var(--bc) / 0.06);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.element-badge {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--p));
  font-weight: 600;
}

.duration-badge {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.5);
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

/* 操作状态样式 */
.action-completed { border-color: hsl(var(--su) / 0.2); }
.action-failed { border-color: hsl(var(--er) / 0.3); }

/* 操作详情 */
.action-body {
  border-top: 1px solid hsl(var(--bc) / 0.08);
  padding: 0.75rem;
  background: hsl(var(--bc) / 0.02);
}

.action-section {
  margin-bottom: 0.5rem;
}
.action-section:last-child {
  margin-bottom: 0;
}

.section-title {
  font-size: calc(var(--font-size-base, 14px) * 0.6875);
  font-weight: 500;
  color: hsl(var(--bc) / 0.45);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin-bottom: 0.25rem;
}

.section-value {
  font-family: ui-monospace, monospace;
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  color: hsl(var(--p));
  background: hsl(var(--p) / 0.08);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  display: inline-block;
}

.section-text {
  color: hsl(var(--bc) / 0.8);
  font-size: calc(var(--font-size-base, 14px) * 0.8125);
  line-height: 1.5;
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
  font-size: calc(var(--font-size-base, 14px) * 1.5);
  font-weight: 700;
  color: hsl(var(--bc));
}

.stat-label {
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: hsl(var(--bc) / 0.6);
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
  padding: 0.5rem;
}

/* 响应式 */
@media (max-width: 640px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>

