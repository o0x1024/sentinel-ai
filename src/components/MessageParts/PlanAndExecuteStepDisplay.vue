<template>
  <div class="plan-execute-container space-y-4">
    <!-- Planning Phase -->
    <div v-if="planningData" class="plan-execute-phase planning-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-tasks text-primary"></i>
          <span class="font-semibold">📝 规划阶段 (Planning)</span>
        </div>
        <span class="badge badge-sm badge-primary">Plan Generation</span>
      </div>
      <div class="phase-content">
        <div class="plan-summary text-sm mb-3" v-html="renderMarkdown(planningData.summary)"></div>
        
        <!-- Plan Steps -->
        <div v-if="planningData.steps && planningData.steps.length > 0" class="mt-3">
          <div class="text-xs font-semibold mb-2 text-base-content/70">执行步骤（共 {{ planningData.steps.length }} 步）：</div>
          <div class="steps-list space-y-2">
            <div 
              v-for="(step, idx) in planningData.steps" 
              :key="idx"
              class="step-card"
            >
              <div class="flex items-start gap-3">
                <div class="step-number">{{ idx + 1 }}</div>
                <div class="flex-1 min-w-0">
                  <div class="font-semibold text-sm">{{ step.name }}</div>
                  <div class="text-xs text-base-content/60 mt-1">{{ step.description }}</div>
                  <div v-if="step.tool" class="mt-2">
                    <code class="text-xs bg-base-300 px-2 py-0.5 rounded">{{ step.tool }}</code>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Risk Assessment -->
        <div v-if="planningData.risk_assessment" class="mt-3 p-3 bg-warning/10 rounded-lg border-l-2 border-warning">
          <div class="text-xs font-semibold text-base-content/70 mb-2">风险评估：</div>
          <div class="text-xs text-base-content/80">
            <div class="mb-1">整体风险等级: <span class="badge badge-xs" :class="getRiskLevelClass(planningData.risk_assessment.overall_risk)">{{ planningData.risk_assessment.overall_risk }}</span></div>
            <div v-if="planningData.risk_assessment.risk_items && planningData.risk_assessment.risk_items.length > 0" class="mt-2">
              <div class="font-semibold mb-1">风险项：</div>
              <ul class="list-disc list-inside space-y-1">
                <li v-for="(item, idx) in planningData.risk_assessment.risk_items" :key="idx">
                  {{ item.description }} - <span class="badge badge-xs" :class="getRiskLevelClass(item.level)">{{ item.level }}</span>
                </li>
              </ul>
            </div>
          </div>
        </div>
        
        <!-- Resource Requirements -->
        <div v-if="planningData.resource_requirements" class="mt-3 p-3 bg-info/10 rounded-lg border-l-2 border-info">
          <div class="text-xs font-semibold text-base-content/70 mb-2">资源需求：</div>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div v-if="planningData.resource_requirements.estimated_time">
              <i class="fas fa-clock"></i> 预计时间: {{ planningData.resource_requirements.estimated_time }}s
            </div>
            <div v-if="planningData.resource_requirements.required_tools && planningData.resource_requirements.required_tools.length > 0">
              <i class="fas fa-wrench"></i> 所需工具: {{ planningData.resource_requirements.required_tools.length }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Execution Phase -->
    <div v-if="executionData && executionData.steps && executionData.steps.length > 0" class="plan-execute-phase execution-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-cog text-info"></i>
          <span class="font-semibold">⚙️ 执行阶段 (Execution)</span>
        </div>
        <span class="badge badge-sm badge-info">Sequential Processing</span>
      </div>
      <div class="phase-content space-y-3">
        <!-- Execution Steps -->
        <div 
          v-for="(step, idx) in executionData.steps" 
          :key="idx"
          class="execution-step"
        >
          <details 
            class="collapse collapse-arrow bg-base-100 border rounded-lg"
            :class="getStepBorderClass(step)"
            :open="isStepInProgress(step)"
          >
            <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
              <div class="flex items-center gap-3">
                <!-- Status Icon -->
                <div class="flex-shrink-0">
                  <i :class="['text-sm', getStepIconClass(step)]"></i>
                </div>
                
                <!-- Step Info -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-mono text-xs text-primary">步骤 {{ idx + 1 }}</span>
                    <span class="font-medium text-sm">{{ step.name }}</span>
                    <span v-if="step.status" class="badge badge-xs" :class="getStepStatusClass(step.status)">
                      {{ getStepStatusText(step.status) }}
                    </span>
                  </div>
                  <div v-if="step.description" class="text-xs text-base-content/60 mt-1">
                    {{ step.description }}
                  </div>
                </div>
                
                <!-- Duration -->
                <div v-if="step.duration_ms" class="text-xs text-base-content/50">
                  {{ step.duration_ms }}ms
                </div>
              </div>
            </summary>
            
            <div class="collapse-content px-4 pb-4">
              <div class="space-y-3">
                <!-- Step Type -->
                <div v-if="step.step_type" class="step-type-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-tag text-xs"></i>
                    TYPE
                  </div>
                  <span class="badge badge-sm badge-outline">{{ step.step_type }}</span>
                </div>
                
                <!-- Tool Config -->
                <div v-if="step.tool_config" class="tool-config-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-wrench text-xs"></i>
                    TOOL
                  </div>
                  <div class="bg-base-200/50 rounded-lg p-3 border border-base-300/30">
                    <div class="text-xs">
                      <div class="font-medium mb-1">{{ step.tool_config.tool_name }}</div>
                      <div v-if="step.tool_config.parameters && Object.keys(formatParams(step.tool_config.parameters)).length > 0" class="mt-2">
                        <div class="font-semibold mb-1">参数：</div>
                        <div
                          v-for="(value, key) in formatParams(step.tool_config.parameters)"
                          :key="key"
                          class="flex items-start gap-2 py-1"
                        >
                          <span class="font-medium text-base-content/70 min-w-[100px]">
                            {{ key }}
                          </span>
                          <span class="text-base-content font-mono break-all whitespace-pre-wrap">
                            {{ typeof value === 'object' ? formatJson(value) : value }}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                
                <!-- Result Section -->
                <div v-if="step.result" class="result-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-arrow-left text-xs"></i>
                    RESULT
                  </div>
                  
                  <!-- Markdown Result for AiReasoning -->
                  <div 
                    v-if="step.step_type === 'AiReasoning'"
                    :class="[
                      'rounded-lg p-3 border',
                      hasResultError(step.result)
                        ? 'bg-error/5 border-error/20'
                        : 'bg-success/5 border-success/20'
                    ]"
                  >
                    <div class="prose prose-sm max-w-none text-xs" v-html="renderMarkdown(step.result)"></div>
                  </div>

                  <!-- Preformatted Result for others (ToolCall) -->
                  <div 
                    v-else
                    :class="[
                      'rounded-lg p-3 border',
                      hasResultError(step.result)
                        ? 'bg-error/5 border-error/20'
                        : 'bg-success/5 border-success/20'
                    ]"
                  >
                    <pre class="text-xs whitespace-pre-wrap break-words font-mono text-base-content">{{ formatResult(step.result) }}</pre>
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
                
                <!-- Retry Info -->
                <div v-if="step.retry_count && step.retry_count > 0" class="retry-info text-xs text-base-content/60">
                  <i class="fas fa-redo"></i> 重试次数: {{ step.retry_count }}
                </div>
              </div>
            </div>
          </details>
        </div>
      </div>
    </div>

    <!-- Replanning Phase -->
    <div v-if="replanningData" class="plan-execute-phase replanning-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-sync text-warning"></i>
          <span class="font-semibold">🔄 重新规划 (Replanning)</span>
        </div>
        <span class="badge badge-sm badge-warning">Adaptive Planning</span>
      </div>
      <div class="phase-content">
        <div v-if="replanningData.trigger" class="trigger-section mb-3">
          <div class="text-xs font-semibold text-base-content/70 mb-2">触发原因：</div>
          <div class="p-3 rounded-lg bg-warning/10 border-l-2 border-warning">
            <div class="text-sm">{{ replanningData.trigger }}</div>
          </div>
        </div>
        
        <div v-if="replanningData.new_plan" class="new-plan-section">
          <div class="text-xs font-semibold text-base-content/70 mb-2">新计划：</div>
          <div class="text-sm" v-html="renderMarkdown(replanningData.new_plan)"></div>
        </div>
      </div>
    </div>

    <!-- Summary Phase -->
    <div v-if="summaryData" class="plan-execute-phase summary-phase">
      <div class="phase-header">
        <div class="flex items-center gap-2">
          <i class="fas fa-check-circle text-success"></i>
          <span class="font-semibold">✅ 执行摘要 (Summary)</span>
        </div>
        <span class="badge badge-sm badge-success">Completed</span>
      </div>
      <div class="phase-content">
        <div v-if="summaryData.response" class="response-section mb-3">
          <div class="text-sm prose prose-sm max-w-none" v-html="renderMarkdown(summaryData.response)"></div>
        </div>
        
        <div class="summary-stats grid grid-cols-2 md:grid-cols-4 gap-3">
          <div class="stat-card">
            <div class="stat-label">总步骤数</div>
            <div class="stat-value">{{ summaryData.total_steps }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">成功步骤</div>
            <div class="stat-value text-success">{{ summaryData.completed_steps }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">失败步骤</div>
            <div class="stat-value text-error">{{ summaryData.failed_steps }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">总耗时</div>
            <div class="stat-value">{{ summaryData.total_duration_ms }}ms</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { marked } from 'marked'

interface PlanningStep {
  name: string
  description: string
  tool?: string
  step_type?: string
}

interface RiskItem {
  description: string
  level: string
  impact?: string
  probability?: number
}

interface RiskAssessment {
  overall_risk: string
  risk_items?: RiskItem[]
  mitigation_strategies?: string[]
}

interface ResourceRequirements {
  estimated_time?: number
  required_tools?: string[]
  memory_mb?: number
  cpu_cores?: number
}

interface PlanningData {
  summary: string
  steps?: PlanningStep[]
  risk_assessment?: RiskAssessment
  resource_requirements?: ResourceRequirements
  confidence?: number
}

interface ToolConfig {
  tool_name: string
  parameters?: any
}

interface ExecutionStep {
  name: string
  description?: string
  step_type?: string
  tool_config?: ToolConfig
  result?: any
  error?: string
  status?: 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Skipped' | 'Blocked'
  duration_ms?: number
  retry_count?: number
}

interface ExecutionData {
  steps: ExecutionStep[]
  current_step?: number
}

interface ReplanningData {
  trigger?: string
  new_plan?: string
  reason?: string
}

interface SummaryData {
  response?: string
  total_steps: number
  completed_steps: number
  failed_steps: number
  total_duration_ms: number
}

const props = defineProps<{
  planningData?: PlanningData
  executionData?: ExecutionData
  replanningData?: ReplanningData
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

const getRiskLevelClass = (level: string) => {
  const levelMap: Record<string, string> = {
    Low: 'badge-success',
    Medium: 'badge-warning',
    High: 'badge-error',
    Critical: 'badge-error',
  }
  return levelMap[level] || 'badge-ghost'
}

const isStepInProgress = (step: ExecutionStep) => {
  return step.status === 'Running'
}

const getStepBorderClass = (step: ExecutionStep) => {
  const status = step.status
  if (status === 'Failed') {
    return 'border-error'
  }
  if (status === 'Running') {
    return 'border-warning'
  }
  if (status === 'Completed') {
    return 'border-success'
  }
  return 'border-base-300'
}

const getStepIconClass = (step: ExecutionStep) => {
  const status = step.status
  if (status === 'Failed') {
    return 'fas fa-times-circle text-error'
  }
  if (status === 'Running') {
    return 'fas fa-spinner fa-spin text-warning'
  }
  if (status === 'Completed') {
    return 'fas fa-check-circle text-success'
  }
  if (status === 'Skipped') {
    return 'fas fa-forward text-base-content/50'
  }
  if (status === 'Blocked') {
    return 'fas fa-ban text-error'
  }
  return 'fas fa-circle text-base-content/30'
}

const getStepStatusClass = (status: string) => {
  const statusMap: Record<string, string> = {
    Pending: 'badge-ghost',
    Running: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error',
    Skipped: 'badge-ghost',
    Blocked: 'badge-error',
  }
  return statusMap[status] || 'badge-ghost'
}

const getStepStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    Pending: '等待中',
    Running: '执行中',
    Completed: '已完成',
    Failed: '失败',
    Skipped: '已跳过',
    Blocked: '已阻塞',
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
.plan-execute-container {
  width: 100%;
}

.plan-execute-phase {
  border: 1px solid hsl(var(--bc) / 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
  background: hsl(var(--b1));
  transition: all 0.2s ease;
}

.plan-execute-phase:hover {
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

.replanning-phase {
  border-left: 3px solid hsl(var(--wa));
}

.summary-phase {
  border-left: 3px solid hsl(var(--su));
}

/* Steps List */
.steps-list {
  max-height: 400px;
  overflow-y: auto;
}

.step-card {
  padding: 0.75rem;
  background: hsl(var(--b2) / 0.3);
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--bc) / 0.1);
  border-left: 3px solid hsl(var(--p));
  transition: all 0.2s ease;
}

.step-card:hover {
  border-color: hsl(var(--p) / 0.5);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
}

.step-number {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 2rem;
  height: 2rem;
  background: hsl(var(--p) / 0.2);
  border: 1px solid hsl(var(--p) / 0.3);
  border-radius: 50%;
  color: hsl(var(--p));
  font-weight: 700;
  font-size: 0.875rem;
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

