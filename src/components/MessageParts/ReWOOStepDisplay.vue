<template>
  <div class="rewoo-message-container space-y-4">
    <!-- Planning Phase -->
    <div v-if="planningData" class="rewoo-phase planning-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-lightbulb text-warning"></i>
          <span class="font-semibold">📋 规划阶段</span>
        </div>
        <span class="badge badge-sm badge-warning">Planning</span>
      </div>
      <div class="phase-content">
        <div class="plan-summary text-sm" v-html="renderMarkdown(planningData.summary)"></div>
        <div v-if="planningData.steps && planningData.steps.length > 0" class="mt-3">
          <div class="text-xs font-semibold mb-2 text-base-content/70">执行步骤：</div>
          <div class="steps-list space-y-1">
            <div 
              v-for="(step, idx) in planningData.steps" 
              :key="idx"
              class="step-item text-xs p-2 bg-base-200/50 rounded border-l-2 border-warning"
            >
              <span class="font-mono text-warning">{{ step.id }}</span> = 
              <span class="font-semibold">{{ step.tool }}</span>
              <span class="text-base-content/60 ml-1">- {{ step.description }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Execution Phase -->
    <div v-if="executionSteps && executionSteps.length > 0" class="rewoo-phase execution-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-cogs text-info"></i>
          <span class="font-semibold">🔧 执行阶段</span>
        </div>
        <span class="badge badge-sm badge-info">Execution</span>
      </div>
      <div class="phase-content space-y-3">
        <div 
          v-for="(step, idx) in executionSteps" 
          :key="idx"
          class="execution-step"
        >
          <details 
            class="collapse collapse-arrow bg-base-100 border rounded-lg"
            :class="getToolCallBorderClass(step)"
            :open="isToolCallInProgress(step)"
          >
            <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
              <div class="flex items-center gap-3">
                <!-- Status Icon -->
                <div class="flex-shrink-0">
                  <i :class="['text-sm', getToolCallIconClass(step)]"></i>
                </div>
                
                <!-- Tool Info -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-medium text-sm">Ran</span>
                    <code class="text-xs bg-base-200 px-2 py-0.5 rounded font-mono">{{ step.toolName }}</code>
                    <span v-if="step.status" class="badge badge-xs" :class="getActionStatusClass(step.status)">
                      {{ getActionStatusText(step.status) }}
                    </span>
                  </div>
                </div>
              </div>
            </summary>
            
            <div class="collapse-content px-4 pb-4">
              <div class="space-y-3">
                <!-- Parameters Section -->
                <div v-if="step.args" class="params-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-cog text-xs"></i>
                    PARAMETERS
                  </div>
                  <div class="bg-base-200/50 rounded-lg p-3 border border-base-300/30">
                    <div
                      v-for="(value, key) in formatParams(step.args)"
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
                <div v-if="step.result" class="response-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-arrow-left text-xs"></i>
                    RESPONSE
                  </div>
                  <div 
                    :class="[
                      'rounded-lg p-3 border',
                      hasObservationError(step.result)
                        ? 'bg-error/5 border-error/20'
                        : 'bg-success/5 border-success/20'
                    ]"
                  >
                    <pre class="text-xs whitespace-pre-wrap break-words font-mono text-base-content">{{ formatObservation(step.result) }}</pre>
                  </div>
                </div>

                <!-- Error Section -->
                <div v-if="step.error" class="error-section">
                  <div class="text-xs font-semibold text-error mb-2 flex items-center gap-1">
                    <i class="fas fa-exclamation-triangle text-xs"></i>
                    ERROR
                  </div>
                  <div class="rounded-lg p-3 border bg-error/5 border-error/20">
                    <pre class="text-xs whitespace-pre-wrap break-words text-error">{{ step.error }}</pre>
                  </div>
                </div>
              </div>
            </div>
          </details>
        </div>
      </div>
    </div>

    <!-- Solving Phase -->
    <div v-if="solvingData" class="rewoo-phase solving-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-check-circle text-success"></i>
          <span class="font-semibold">✅ 求解阶段</span>
        </div>
        <span class="badge badge-sm badge-success">Solving</span>
      </div>
      <div class="phase-content">
        <div class="final-answer prose prose-sm max-w-none" v-html="renderMarkdown(solvingData.answer)"></div>
        <div v-if="solvingData.meta" class="meta-info mt-3 text-xs text-base-content/60">
          {{ solvingData.meta }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { marked } from 'marked'

interface PlanningData {
  summary: string
  steps?: Array<{
    id: string
    tool: string
    description: string
  }>
}

interface ExecutionStep {
  toolName: string
  args?: any
  thinking?: string
  result?: any
  error?: string
  status?: 'running' | 'success' | 'failed' | 'pending' | 'error'
}

interface SolvingData {
  answer: string
  meta?: string
}

const props = defineProps<{
  planningData?: PlanningData
  executionSteps?: ExecutionStep[]
  solvingData?: SolvingData
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

const formatParams = (args: any) => {
  if (!args) return {}
  if (typeof args === 'object') {
    return args
  }
  try {
    return JSON.parse(args)
  } catch {
    return { value: args }
  }
}

const formatObservation = (obs: any) => {
  if (typeof obs === 'string') return obs
  try {
    return JSON.stringify(obs, null, 2)
  } catch {
    return String(obs)
  }
}

const isToolCallInProgress = (step: ExecutionStep) => {
  const status = step.status
  return status === 'running' || status === 'pending'
}

const getToolCallBorderClass = (step: ExecutionStep) => {
  const status = step.status
  if (status === 'failed' || status === 'error') {
    return 'border-error'
  }
  if (status === 'running') {
    return 'border-warning'
  }
  return 'border-success'
}

const getToolCallIconClass = (step: ExecutionStep) => {
  const status = step.status
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

const hasObservationError = (obs: any) => {
  if (typeof obs === 'string') {
    const lowerObs = obs.toLowerCase()
    return lowerObs.includes('error') || 
           lowerObs.includes('failed') || 
           lowerObs.includes('失败') ||
           lowerObs.includes('"success":false') ||
           lowerObs.includes('"success": false')
  }
  if (typeof obs === 'object' && obs !== null) {
    return obs.success === false || obs.error
  }
  return false
}
</script>

<style scoped>
.rewoo-message-container {
  width: 100%;
}

.rewoo-phase {
  border: 1px solid hsl(var(--bc) / 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
  background: hsl(var(--b1));
  transition: all 0.2s ease;
}

.rewoo-phase:hover {
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
  border-left: 3px solid hsl(var(--wa));
}

.execution-phase {
  border-left: 3px solid hsl(var(--in));
}

.solving-phase {
  border-left: 3px solid hsl(var(--su));
}

.execution-step {
  padding: 0.75rem;
  background: hsl(var(--b2) / 0.3);
  border-radius: 0.375rem;
  border: 1px solid hsl(var(--bc) / 0.1);
}

.step-header {
  cursor: pointer;
}

.step-number {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  background: hsl(var(--p));
  color: hsl(var(--pc));
  border-radius: 50%;
  font-size: 0.75rem;
  font-weight: bold;
}

.step-content {
  padding-left: 2rem;
}

.thinking-section,
.result-section,
.error-section {
  margin-top: 0.5rem;
}

.plan-summary {
  font-size: 0.875rem;
  line-height: 1.5;
}

.steps-list {
  max-height: 300px;
  overflow-y: auto;
}

.step-item {
  font-family: 'Courier New', monospace;
}

.final-answer {
  font-size: 0.875rem;
  line-height: 1.6;
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

.phase-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 0.5rem 0;
}

.phase-content :deep(th),
.phase-content :deep(td) {
  border: 1px solid hsl(var(--bc) / 0.2);
  padding: 0.5rem;
  text-align: left;
}

.phase-content :deep(th) {
  background: hsl(var(--b2));
  font-weight: 600;
}

.phase-content :deep(blockquote) {
  border-left: 3px solid hsl(var(--p));
  padding-left: 1rem;
  margin: 0.5rem 0;
  color: hsl(var(--bc) / 0.7);
}
</style>

