<template>
  <div class="llm-compiler-container space-y-4">
    <!-- Planning Phase -->
    <div v-if="planningData" class="llm-compiler-phase planning-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-project-diagram text-primary"></i>
          <span class="font-semibold">📊 规划阶段 (Planning)</span>
        </div>
        <span class="badge badge-sm badge-primary">DAG Generation</span>
      </div>
      <div class="phase-content">
        <div class="plan-summary text-sm" v-html="renderMarkdown(planningData.summary)"></div>
        
        <!-- DAG Tasks -->
        <div v-if="planningData.tasks && planningData.tasks.length > 0" class="mt-3">
          <div class="text-xs font-semibold mb-2 text-base-content/70">任务节点（共 {{ planningData.tasks.length }} 个）：</div>
          <div class="dag-tasks-grid">
            <div 
              v-for="(task, idx) in planningData.tasks" 
              :key="idx"
              class="dag-task-card"
              :class="getTaskCardClass(task)"
            >
              <div class="flex items-start gap-2">
                <div class="task-id-badge">
                  <span class="font-mono text-xs">{{ task.id }}</span>
                </div>
                <div class="flex-1 min-w-0">
                  <div class="font-semibold text-sm">{{ task.name }}</div>
                  <div class="text-xs text-base-content/60 mt-1">{{ task.description }}</div>
                  <div class="flex items-center gap-2 mt-2 flex-wrap">
                    <code class="text-xs bg-base-300 px-2 py-0.5 rounded">{{ task.tool }}</code>
                    <span v-if="task.dependencies && task.dependencies.length > 0" class="text-xs text-base-content/50">
                      <i class="fas fa-link text-xs"></i> 依赖: {{ task.dependencies.join(', ') }}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Execution Strategy -->
        <div v-if="planningData.strategy" class="mt-3 p-3 bg-base-200/50 rounded-lg border-l-2 border-primary">
          <div class="text-xs font-semibold text-base-content/70 mb-1">执行策略：</div>
          <div class="text-xs text-base-content/80">{{ planningData.strategy }}</div>
        </div>
      </div>
    </div>

    <!-- Execution Phase -->
    <div v-if="executionData && executionData.rounds && executionData.rounds.length > 0" class="llm-compiler-phase execution-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-play-circle text-info"></i>
          <span class="font-semibold">⚡ 执行阶段 (Execution)</span>
        </div>
        <span class="badge badge-sm badge-info">Parallel Processing</span>
      </div>
      <div class="phase-content space-y-4">
        <!-- Execution Rounds -->
        <div 
          v-for="(round, roundIdx) in executionData.rounds" 
          :key="roundIdx"
          class="execution-round"
        >
          <div class="round-header">
            <div class="flex items-center gap-2">
              <span class="round-badge">Round {{ round.round }}</span>
              <span class="text-xs text-base-content/60">
                {{ round.tasks.length }} 个任务并行执行
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span v-if="round.duration_ms" class="text-xs text-base-content/50">
                <i class="fas fa-clock"></i> {{ round.duration_ms }}ms
              </span>
            </div>
          </div>
          
          <!-- Tasks in this round -->
          <div class="round-tasks space-y-2">
            <div 
              v-for="(task, taskIdx) in round.tasks" 
              :key="taskIdx"
              class="execution-task"
            >
              <details 
                class="collapse collapse-arrow bg-base-100 border rounded-lg"
                :class="getTaskBorderClass(task)"
                :open="isTaskInProgress(task)"
              >
                <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
                  <div class="flex items-center gap-3">
                    <!-- Status Icon -->
                    <div class="flex-shrink-0">
                      <i :class="['text-sm', getTaskIconClass(task)]"></i>
                    </div>
                    
                    <!-- Task Info -->
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 flex-wrap">
                        <span class="font-mono text-xs text-primary">{{ task.task_id }}</span>
                        <span class="font-medium text-sm">{{ task.name || task.tool }}</span>
                        <code class="text-xs bg-base-200 px-2 py-0.5 rounded font-mono">{{ task.tool }}</code>
                        <span v-if="task.status" class="badge badge-xs" :class="getTaskStatusClass(task.status)">
                          {{ getTaskStatusText(task.status) }}
                        </span>
                      </div>
                    </div>
                    
                    <!-- Duration -->
                    <div v-if="task.duration_ms" class="text-xs text-base-content/50">
                      {{ task.duration_ms }}ms
                    </div>
                  </div>
                </summary>
                
                <div class="collapse-content px-4 pb-4">
                  <div class="space-y-3">
                    <!-- Inputs Section -->
                    <div v-if="task.inputs" class="inputs-section">
                      <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                        <i class="fas fa-arrow-right text-xs"></i>
                        INPUTS
                      </div>
                      <div class="bg-base-200/50 rounded-lg p-3 border border-base-300/30">
                        <div
                          v-for="(value, key) in formatParams(task.inputs)"
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
                    
                    <!-- Dependencies Section -->
                    <div v-if="task.dependencies && task.dependencies.length > 0" class="dependencies-section">
                      <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                        <i class="fas fa-link text-xs"></i>
                        DEPENDENCIES
                      </div>
                      <div class="flex flex-wrap gap-1">
                        <span 
                          v-for="dep in task.dependencies" 
                          :key="dep"
                          class="badge badge-sm badge-outline"
                        >
                          {{ dep }}
                        </span>
                      </div>
                    </div>
                    
                    <!-- Result Section -->
                    <div v-if="task.result" class="result-section">
                      <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                        <i class="fas fa-arrow-left text-xs"></i>
                        RESULT
                      </div>
                      <div 
                        :class="[
                          'rounded-lg p-3 border',
                          hasResultError(task.result)
                            ? 'bg-error/5 border-error/20'
                            : 'bg-success/5 border-success/20'
                        ]"
                      >
                        <pre class="text-xs whitespace-pre-wrap break-words font-mono text-base-content">{{ formatResult(task.result) }}</pre>
                      </div>
                    </div>

                    <!-- Error Section -->
                    <div v-if="task.error" class="error-section">
                      <div class="text-xs font-semibold text-error mb-2 flex items-center gap-1">
                        <i class="fas fa-exclamation-triangle text-xs"></i>
                        ERROR
                      </div>
                      <div class="rounded-lg p-3 border bg-error/5 border-error/20">
                        <pre class="text-xs whitespace-pre-wrap break-words text-error">{{ task.error }}</pre>
                      </div>
                    </div>
                  </div>
                </div>
              </details>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Joiner Phase -->
    <div v-if="joinerData" class="llm-compiler-phase joiner-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-brain text-warning"></i>
          <span class="font-semibold">🧠 决策阶段 (Joiner)</span>
        </div>
        <span class="badge badge-sm badge-warning">Intelligent Decision</span>
      </div>
      <div class="phase-content">
        <div v-if="joinerData.decision" class="decision-section mb-3">
          <div class="text-xs font-semibold text-base-content/70 mb-2">决策结果：</div>
          <div 
            class="p-3 rounded-lg border-l-4"
            :class="joinerData.decision === 'complete' ? 'bg-success/10 border-success' : 'bg-warning/10 border-warning'"
          >
            <div class="flex items-center gap-2 mb-2">
              <i :class="joinerData.decision === 'complete' ? 'fas fa-check-circle text-success' : 'fas fa-sync text-warning'"></i>
              <span class="font-semibold text-sm">
                {{ joinerData.decision === 'complete' ? '✓ 完成执行' : '→ 继续执行' }}
              </span>
            </div>
            <div class="text-sm" v-html="renderMarkdown(joinerData.response || joinerData.feedback || '')"></div>
          </div>
        </div>
        
        <div v-if="joinerData.meta" class="meta-info text-xs text-base-content/60">
          {{ joinerData.meta }}
        </div>
      </div>
    </div>

    <!-- Summary Phase -->
    <div v-if="summaryData" class="llm-compiler-phase summary-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-chart-line text-success"></i>
          <span class="font-semibold">📈 执行摘要 (Summary)</span>
        </div>
        <span class="badge badge-sm badge-success">Completed</span>
      </div>
      <div class="phase-content">
        <div class="summary-stats grid grid-cols-2 md:grid-cols-4 gap-3 mb-3">
          <div class="stat-card">
            <div class="stat-label">总任务数</div>
            <div class="stat-value">{{ summaryData.total_tasks }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">成功任务</div>
            <div class="stat-value text-success">{{ summaryData.successful_tasks }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">失败任务</div>
            <div class="stat-value text-error">{{ summaryData.failed_tasks }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">总耗时</div>
            <div class="stat-value">{{ summaryData.total_duration_ms }}ms</div>
          </div>
        </div>
        
        <div v-if="summaryData.replanning_count > 0" class="replanning-info text-xs text-base-content/60">
          <i class="fas fa-redo"></i> 重规划次数: {{ summaryData.replanning_count }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { marked } from 'marked'

interface PlanningTask {
  id: string
  name: string
  description: string
  tool: string
  inputs?: any
  dependencies?: string[]
  reason?: string
}

interface PlanningData {
  summary: string
  tasks?: PlanningTask[]
  strategy?: string
}

interface ExecutionTask {
  task_id: string
  name?: string
  tool: string
  inputs?: any
  dependencies?: string[]
  result?: any
  error?: string
  status?: 'Pending' | 'Ready' | 'Running' | 'Completed' | 'Failed' | 'Cancelled' | 'Retrying'
  duration_ms?: number
}

interface ExecutionRound {
  round: number
  tasks: ExecutionTask[]
  duration_ms?: number
}

interface ExecutionData {
  rounds: ExecutionRound[]
}

interface JoinerData {
  decision?: 'complete' | 'continue'
  response?: string
  feedback?: string
  meta?: string
}

interface SummaryData {
  total_tasks: number
  successful_tasks: number
  failed_tasks: number
  total_duration_ms: number
  replanning_count: number
}

const props = defineProps<{
  planningData?: PlanningData
  executionData?: ExecutionData
  joinerData?: JoinerData
  summaryData?: SummaryData
}>()

const renderMarkdown = (content: string): string => {
  if (!content) return ''
  try {
    return marked.parse(content) as string
  } catch (e) {
    console.error('Markdown parse error:', e)
    return content
  }
}

const formatJson = (obj: any) => {
  try {
    return JSON.stringify(obj, null, 2)
  } catch {
    return String(obj)
  }
}

const formatParams = (params: any) => {
  if (!params) return {}
  if (typeof params === 'object') {
    return params
  }
  try {
    return JSON.parse(params)
  } catch {
    return { value: params }
  }
}

const formatResult = (result: any) => {
  if (typeof result === 'string') return result
  try {
    return JSON.stringify(result, null, 2)
  } catch {
    return String(result)
  }
}

const getTaskCardClass = (task: PlanningTask) => {
  if (task.dependencies && task.dependencies.length > 0) {
    return 'has-dependencies'
  }
  return 'no-dependencies'
}

const isTaskInProgress = (task: ExecutionTask) => {
  return task.status === 'Running' || task.status === 'Retrying'
}

const getTaskBorderClass = (task: ExecutionTask) => {
  const status = task.status
  if (status === 'Failed' || status === 'Cancelled') {
    return 'border-error'
  }
  if (status === 'Running' || status === 'Retrying') {
    return 'border-warning'
  }
  if (status === 'Completed') {
    return 'border-success'
  }
  return 'border-base-300'
}

const getTaskIconClass = (task: ExecutionTask) => {
  const status = task.status
  if (status === 'Failed' || status === 'Cancelled') {
    return 'fas fa-times-circle text-error'
  }
  if (status === 'Running' || status === 'Retrying') {
    return 'fas fa-spinner fa-spin text-warning'
  }
  if (status === 'Completed') {
    return 'fas fa-check-circle text-success'
  }
  return 'fas fa-circle text-base-content/30'
}

const getTaskStatusClass = (status: string) => {
  const statusMap: Record<string, string> = {
    Pending: 'badge-ghost',
    Ready: 'badge-info',
    Running: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error',
    Cancelled: 'badge-error',
    Retrying: 'badge-warning',
  }
  return statusMap[status] || 'badge-ghost'
}

const getTaskStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    Pending: '等待中',
    Ready: '就绪',
    Running: '执行中',
    Completed: '已完成',
    Failed: '失败',
    Cancelled: '已取消',
    Retrying: '重试中',
  }
  return textMap[status] || status
}

const hasResultError = (result: any) => {
  if (typeof result === 'string') {
    const lowerResult = result.toLowerCase()
    return lowerResult.includes('error') || 
           lowerResult.includes('failed') || 
           lowerResult.includes('失败') ||
           lowerResult.includes('"success":false') ||
           lowerResult.includes('"success": false')
  }
  if (typeof result === 'object' && result !== null) {
    return result.success === false || result.error
  }
  return false
}
</script>

<style scoped>
.llm-compiler-container {
  width: 100%;
}

.llm-compiler-phase {
  border: 1px solid hsl(var(--bc) / 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
  background: hsl(var(--b1));
  transition: all 0.2s ease;
}

.llm-compiler-phase:hover {
  border-color: hsl(var(--bc) / 0.2);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.phase-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: hsl(var(--b2) / 0.5);
  border-bottom: 1px solid hsl(var(--bc) / 0.1);
}

.phase-content {
  padding: 1rem;
}

.planning-phase {
  border-left: 3px solid hsl(var(--p));
}

.execution-phase {
  border-left: 3px solid hsl(var(--in));
}

.joiner-phase {
  border-left: 3px solid hsl(var(--wa));
}

.summary-phase {
  border-left: 3px solid hsl(var(--su));
}

/* DAG Tasks Grid */
.dag-tasks-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 0.75rem;
}

.dag-task-card {
  padding: 0.75rem;
  background: hsl(var(--b2) / 0.3);
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--bc) / 0.1);
  transition: all 0.2s ease;
}

.dag-task-card:hover {
  border-color: hsl(var(--p) / 0.5);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
}

.dag-task-card.has-dependencies {
  border-left: 3px solid hsl(var(--wa));
}

.dag-task-card.no-dependencies {
  border-left: 3px solid hsl(var(--su));
}

.task-id-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 2.5rem;
  height: 2rem;
  background: hsl(var(--p) / 0.2);
  border: 1px solid hsl(var(--p) / 0.3);
  border-radius: 0.375rem;
  color: hsl(var(--p));
  font-weight: 600;
}

/* Execution Rounds */
.execution-round {
  padding: 0.75rem;
  background: hsl(var(--b2) / 0.2);
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--bc) / 0.1);
}

.round-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid hsl(var(--bc) / 0.1);
}

.round-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.75rem;
  background: hsl(var(--in) / 0.2);
  border: 1px solid hsl(var(--in) / 0.3);
  border-radius: 0.375rem;
  color: hsl(var(--in));
  font-size: 0.75rem;
  font-weight: 600;
}

.round-tasks {
  padding-left: 0.5rem;
}

/* Summary Stats */
.summary-stats {
  margin-top: 0.75rem;
}

.stat-card {
  padding: 0.75rem;
  background: hsl(var(--b2) / 0.3);
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--bc) / 0.1);
  text-align: center;
}

.stat-label {
  font-size: 0.75rem;
  color: hsl(var(--bc) / 0.6);
  margin-bottom: 0.25rem;
}

.stat-value {
  font-size: 1.25rem;
  font-weight: 700;
  color: hsl(var(--bc));
}

/* Markdown样式优化 */
.phase-content :deep(h1),
.phase-content :deep(h2),
.phase-content :deep(h3) {
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  font-weight: 600;
}

.phase-content :deep(ul),
.phase-content :deep(ol) {
  margin-left: 1.5rem;
  margin-top: 0.5rem;
  margin-bottom: 0.5rem;
}

.phase-content :deep(code) {
  background: hsl(var(--b3));
  padding: 0.125rem 0.25rem;
  border-radius: 0.25rem;
  font-size: 0.875em;
}

.phase-content :deep(pre) {
  background: hsl(var(--b3));
  padding: 0.75rem;
  border-radius: 0.375rem;
  overflow-x: auto;
  margin: 0.5rem 0;
}

.phase-content :deep(pre code) {
  background: none;
  padding: 0;
}
</style>

