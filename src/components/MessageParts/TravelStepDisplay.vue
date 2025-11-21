<template>
  <div class="travel-step-display">
    <!-- 任务复杂度标识 -->
    <div v-if="taskComplexity" class="complexity-badge mb-3">
      <div class="badge" :class="getComplexityClass(taskComplexity)">
        <i :class="getComplexityIcon(taskComplexity)" class="mr-1"></i>
        {{ getComplexityText(taskComplexity) }}
      </div>
    </div>

    <!-- OODA 循环列表 -->
    <div v-for="(cycle, cycleIndex) in oodaCycles" :key="cycle.id || cycleIndex" class="ooda-cycle mb-4">
      <!-- 循环标题 -->
      <div class="cycle-header flex items-center gap-2 mb-2 p-3 bg-base-200/50 rounded-lg">
        <i class="fas fa-sync-alt text-primary"></i>
        <span class="font-semibold">OODA 循环 #{{ cycle.cycle_number }}</span>
        <span class="badge badge-sm" :class="getCycleStatusClass(cycle.status)">
          {{ getCycleStatusText(cycle.status) }}
        </span>
        <span v-if="cycle.completed_at" class="text-xs text-base-content/60 ml-auto">
          {{ formatDuration(cycle.started_at, cycle.completed_at) }}
        </span>
      </div>

      <!-- OODA 阶段 -->
      <div class="phases-container space-y-2">
        <div 
          v-for="(phaseExec, phaseIndex) in cycle.phase_history" 
          :key="phaseIndex"
          class="phase-execution"
        >
          <details 
            class="collapse collapse-arrow bg-base-100 border rounded-lg"
            :class="getPhaseBorderClass(phaseExec.status)"
            :open="phaseExec.status === 'Running'"
          >
            <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
              <div class="flex items-center gap-3">
                <!-- 阶段图标 -->
                <div class="flex-shrink-0">
                  <i :class="[getPhaseIcon(phaseExec.phase), 'text-sm', getPhaseIconColor(phaseExec.status)]"></i>
                </div>
                
                <!-- 阶段信息 -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-medium text-sm">{{ getPhaseText(phaseExec.phase) }}</span>
                    <span class="badge badge-xs" :class="getPhaseStatusClass(phaseExec.status)">
                      {{ getPhaseStatusText(phaseExec.status) }}
                    </span>
                    <span v-if="phaseExec.completed_at" class="text-xs text-base-content/60">
                      {{ formatDuration(phaseExec.started_at, phaseExec.completed_at) }}
                    </span>
                  </div>
                </div>
              </div>
            </summary>
            
            <div class="collapse-content px-4 pb-4">
              <div class="space-y-3">
                <!-- 思考过程 -->
                <div v-if="phaseExec.thinking && phaseExec.thinking.length > 0" class="thinking-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-brain text-xs"></i>
                    思考过程
                  </div>
                  <div class="space-y-1">
                    <div 
                      v-for="(think, thinkIndex) in phaseExec.thinking" 
                      :key="thinkIndex"
                      class="thinking-item p-2 rounded bg-base-200/30 text-xs"
                    >
                      {{ think.content }}
                    </div>
                  </div>
                </div>

                <!-- 内容输出 -->
                <div v-if="phaseExec.content && phaseExec.content.length > 0" class="content-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-file-alt text-xs"></i>
                    阶段内容
                  </div>
                  <div class="space-y-1">
                    <div 
                      v-for="(contentItem, contentIndex) in phaseExec.content" 
                      :key="contentIndex"
                      class="content-item p-2 rounded bg-base-200/30 text-xs whitespace-pre-wrap"
                    >
                      {{ contentItem.content }}
                    </div>
                  </div>
                </div>

                <!-- 护栏检查结果 -->
                <div v-if="phaseExec.guardrail_checks && phaseExec.guardrail_checks.length > 0" class="guardrail-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-shield-alt text-xs"></i>
                    护栏检查
                  </div>
                  <div class="space-y-1">
                    <div 
                      v-for="(check, checkIndex) in phaseExec.guardrail_checks" 
                      :key="checkIndex"
                      class="flex items-start gap-2 p-2 rounded bg-base-200/30"
                    >
                      <i :class="[getGuardrailIcon(check.result), 'text-xs mt-0.5']"></i>
                      <div class="flex-1 min-w-0">
                        <div class="text-xs font-medium">{{ check.rule_name }}</div>
                        <div class="text-xs text-base-content/70">{{ check.message }}</div>
                      </div>
                      <span class="badge badge-xs" :class="getSeverityClass(check.severity)">
                        {{ check.severity }}
                      </span>
                    </div>
                  </div>
                </div>

                <!-- 工具调用（ReAct 风格展示） -->
                <div v-if="phaseExec.tool_calls && phaseExec.tool_calls.length > 0" class=  "tool-calls-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-tools text-xs"></i>
                    工具调用
                  </div>
                  <div class="space-y-2">
                    <details
                      v-for="(toolCall, toolIndex) in phaseExec.tool_calls"
                      :key="toolIndex"
                      class="collapse collapse-arrow bg-base-100 border rounded-lg"
                      :class="getToolBorderClass(toolCall.status)"
                    >
                      <summary class="collapse-title min-h-0 py-2 px-3 cursor-pointer hover:bg-base-200/50 transition-colors">
                        <div class="flex items-center gap-3">
                          <div class="flex-shrink-0">
                            <i :class="['text-xs', getToolIconClass(toolCall.status)]"></i>
                          </div>
                          <div class="flex-1 min-w-0">
                            <div class="flex items-center gap-2 flex-wrap">
                              <span class="text-xs text-base-content/70">Ran</span>
                              <code class="text-xs bg-base-200 px-2 py-0.5 rounded font-mono">{{ toolCall.tool_name }}</code>
                              <span class="badge badge-xs" :class="getToolStatusClass(toolCall.status)">
                                {{ getToolStatusText(toolCall.status) }}
                              </span>
                            </div>
                          </div>
                        </div>
                      </summary>

                      <div class="collapse-content px-3 pb-3">
                        <div class="space-y-2">
                          <!-- 参数 -->
                          <div v-if="toolCall.args && Object.keys(toolCall.args).length > 0" class="params-section">
                            <div class="text-xs font-semibold text-base-content/70 mb-1 flex items-center gap-1">
                              <i class="fas fa-cog text-xs"></i>
                              PARAMETERS
                            </div>
                            <div class="bg-base-200/50 rounded-lg p-2 border border-base-300/30">
                              <div
                                v-for="(value, key) in toolCall.args"
                                :key="key"
                                class="flex items-start gap-2 py-0.5"
                              >
                                <span class="text-[11px] font-medium text-base-content/70 min-w-[80px]">
                                  {{ key }}
                                </span>
                                <span class="text-[11px] text-base-content font-mono break-all whitespace-pre-wrap">
                                  {{ typeof value === 'object' ? formatJson(value) : value }}
                                </span>
                              </div>
                            </div>
                          </div>

                          <!-- 响应结果 -->
                          <div v-if="toolCall.result" class="response-section">
                            <div class="text-xs font-semibold text-base-content/70 mb-1 flex items-center gap-1">
                              <i class="fas fa-arrow-left text-xs"></i>
                              RESPONSE
                            </div>
                            <div
                              :class="[
                                'rounded-lg p-2 border',
                                toolCall.status === 'Failed'
                                  ? 'bg-error/5 border-error/20'
                                  : 'bg-success/5 border-success/20',
                              ]"
                            >
                              <pre class="text-[11px] whitespace-pre-wrap break-words font-mono text-base-content max-h-40 overflow-auto">{{ formatJson(toolCall.result) }}</pre>
                            </div>
                          </div>
                        </div>
                      </div>
                    </details>
                  </div>
                </div>

                <!-- 阶段输出 -->
                <div v-if="phaseExec.output" class="output-section">
                  <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                    <i class="fas fa-arrow-right text-xs"></i>
                    阶段输出
                  </div>
                  <div class="bg-base-200/50 rounded-lg p-3">
                    <pre class="text-xs whitespace-pre-wrap break-words">{{ formatOutput(phaseExec.output, phaseExec.phase) }}</pre>
                  </div>
                </div>

                <!-- 错误信息 -->
                <div v-if="phaseExec.error" class="error-section">
                  <div class="alert alert-error text-xs">
                    <i class="fas fa-exclamation-triangle"></i>
                    <span>{{ phaseExec.error }}</span>
                  </div>
                </div>
              </div>
            </div>
          </details>
        </div>
      </div>

      <!-- 循环结果摘要 -->
      <div v-if="cycle.result" class="cycle-result mt-2 p-3 bg-accent/5 rounded-lg border border-accent/20">
        <div class="text-xs font-semibold text-base-content/70 mb-2">循环结果</div>
        <div class="text-sm prose prose-sm max-w-none" v-html="renderMarkdown(formatCycleResult(cycle.result))"></div>
      </div>

      <!-- 循环错误 -->
      <div v-if="cycle.error" class="cycle-error mt-2">
        <div class="alert alert-error text-sm">
          <i class="fas fa-times-circle"></i>
          <span>{{ cycle.error }}</span>
        </div>
      </div>
    </div>

    <!-- 执行指标 -->
    <div v-if="metrics" class="metrics-summary mt-4 p-3 bg-base-200/30 rounded-lg">
      <div class="text-xs font-semibold text-base-content/70 mb-2">执行统计</div>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
        <div class="stat-item">
          <span class="text-base-content/60">循环次数:</span>
          <span class="font-medium ml-1">{{ metrics.total_cycles }}</span>
        </div>
        <div class="stat-item">
          <span class="text-base-content/60">工具调用:</span>
          <span class="font-medium ml-1">{{ metrics.total_tool_calls }}</span>
        </div>
        <div class="stat-item">
          <span class="text-base-content/60">护栏检查:</span>
          <span class="font-medium ml-1">{{ metrics.guardrail_checks }}</span>
        </div>
        <div class="stat-item">
          <span class="text-base-content/60">总耗时:</span>
          <span class="font-medium ml-1">{{ formatMs(metrics.total_duration_ms) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'
import type { ChatMessage } from '../../types/chat'

interface TravelStepData {
  taskComplexity?: string
  oodaCycles?: any[]
  metrics?: any
}

const props = defineProps<{
  message?: ChatMessage
  stepData?: TravelStepData
}>()

const { renderMarkdown } = useMessageUtils()

const taskComplexity = computed(() => {
  if (props.stepData?.taskComplexity) return props.stepData.taskComplexity
  if (props.message?.travelData?.task_complexity) return props.message.travelData.task_complexity
  return null
})

const oodaCycles = computed(() => {
  console.log('[TravelStepDisplay] Computing oodaCycles, stepData:', props.stepData, 'message.travelData:', (props.message as any)?.travelData)
  
  if (props.stepData?.oodaCycles && props.stepData.oodaCycles.length > 0) {
    console.log('[TravelStepDisplay] Using stepData.oodaCycles:', props.stepData.oodaCycles)
    return props.stepData.oodaCycles
  }
  
  // 支持两种属性名：oodaCycles (新) 和 ooda_cycles (旧)
  const travelData = (props.message as any)?.travelData
  if (travelData) {
    const cycles = travelData.oodaCycles || travelData.ooda_cycles || []
    console.log('[TravelStepDisplay] Using message.travelData cycles:', cycles)
    return cycles
  }
  
  console.log('[TravelStepDisplay] No cycles found')
  return []
})

const metrics = computed(() => {
  if (props.stepData?.metrics) return props.stepData.metrics
  if (props.message?.travelData?.metrics) return props.message.travelData.metrics
  return null
})

const getComplexityClass = (complexity: string) => {
  const map: Record<string, string> = {
    Simple: 'badge-success',
    Medium: 'badge-warning',
    Complex: 'badge-error',
  }
  return map[complexity] || 'badge-ghost'
}

const getComplexityIcon = (complexity: string) => {
  const map: Record<string, string> = {
    Simple: 'fas fa-check-circle',
    Medium: 'fas fa-exclamation-circle',
    Complex: 'fas fa-brain',
  }
  return map[complexity] || 'fas fa-question-circle'
}

const getComplexityText = (complexity: string) => {
  const map: Record<string, string> = {
    Simple: '简单任务',
    Medium: '中等任务',
    Complex: '复杂任务',
  }
  return map[complexity] || complexity
}

const getCycleStatusClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error',
    RolledBack: 'badge-info',
    Cancelled: 'badge-ghost',
  }
  return map[status] || 'badge-ghost'
}

const getCycleStatusText = (status: string) => {
  const map: Record<string, string> = {
    Running: '运行中',
    Completed: '已完成',
    Failed: '失败',
    RolledBack: '已回退',
    Cancelled: '已取消',
  }
  return map[status] || status
}

const getPhaseIcon = (phase: string) => {
  const map: Record<string, string> = {
    Observe: 'fas fa-eye',
    Orient: 'fas fa-compass',
    Decide: 'fas fa-brain',
    Act: 'fas fa-bolt',
  }
  return map[phase] || 'fas fa-circle'
}

const getPhaseText = (phase: string) => {
  const map: Record<string, string> = {
    Observe: '观察 (Observe)',
    Orient: '定位 (Orient)',
    Decide: '决策 (Decide)',
    Act: '执行 (Act)',
  }
  return map[phase] || phase
}

const getPhaseIconColor = (status: string) => {
  const map: Record<string, string> = {
    Running: 'text-warning',
    Completed: 'text-success',
    Failed: 'text-error',
    Pending: 'text-base-content/40',
  }
  return map[status] || 'text-base-content'
}

const getPhaseBorderClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'border-warning',
    Completed: 'border-success',
    Failed: 'border-error',
    Pending: 'border-base-300',
  }
  return map[status] || 'border-base-300'
}

const getPhaseStatusClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error',
    Pending: 'badge-ghost',
    Skipped: 'badge-info',
    RolledBack: 'badge-info',
  }
  return map[status] || 'badge-ghost'
}

const getPhaseStatusText = (status: string) => {
  const map: Record<string, string> = {
    Running: '运行中',
    Completed: '已完成',
    Failed: '失败',
    Pending: '等待中',
    Skipped: '已跳过',
    RolledBack: '已回退',
  }
  return map[status] || status
}

const getGuardrailIcon = (result: string) => {
  const map: Record<string, string> = {
    Passed: 'fas fa-check-circle text-success',
    Warning: 'fas fa-exclamation-triangle text-warning',
    Failed: 'fas fa-times-circle text-error',
    Skipped: 'fas fa-minus-circle text-base-content/40',
  }
  return map[result] || 'fas fa-question-circle'
}

const getSeverityClass = (severity: string) => {
  const map: Record<string, string> = {
    Info: 'badge-info',
    Warning: 'badge-warning',
    Error: 'badge-error',
    Critical: 'badge-error',
  }
  return map[severity] || 'badge-ghost'
}

const getToolStatusClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error',
    Pending: 'badge-ghost',
    Timeout: 'badge-error',
  }
  return map[status] || 'badge-ghost'
}

const getToolStatusText = (status: string) => {
  const map: Record<string, string> = {
    Running: '运行中',
    Completed: '已完成',
    Failed: '失败',
    Pending: '等待中',
    Timeout: '超时',
  }
  return map[status] || status
}

const getToolBorderClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'border-warning',
    Completed: 'border-success',
    Failed: 'border-error',
    Pending: 'border-base-300',
    Timeout: 'border-error',
  }
  return map[status] || 'border-base-300'
}

const getToolIconClass = (status: string) => {
  const map: Record<string, string> = {
    Running: 'fas fa-spinner fa-spin text-warning',
    Completed: 'fas fa-check text-success',
    Failed: 'fas fa-times-circle text-error',
    Pending: 'fas fa-circle text-base-content/40',
    Timeout: 'fas fa-clock text-error',
  }
  return map[status] || 'fas fa-check text-success'
}

const formatJson = (obj: any) => {
  try {
    if (typeof obj === 'string') return obj
    return JSON.stringify(obj, null, 2)
  } catch {
    return String(obj)
  }
}

const formatOutput = (output: any, phase: string) => {
  try {
    if (typeof output === 'string') return output
    
    // 根据阶段类型格式化输出
    if (phase === 'Observe' && output.observations) {
      // 观察阶段结果
      const obs = output.observations
      return `收集到 ${Object.keys(obs).length} 项观察数据\n${JSON.stringify(obs, null, 2)}`
    }
    
    if (phase === 'Orient' && output.threats) {
      // 威胁分析结果
      return `威胁等级: ${output.threat_level}\n发现威胁: ${output.threats.length} 个\n漏洞: ${output.vulnerabilities?.length || 0} 个`
    }
    
    if (phase === 'Decide' && output.steps) {
      // 行动计划
      return `计划: ${output.name}\n步骤数: ${output.steps.length}\n预估时间: ${output.estimated_duration}s`
    }
    
    if (phase === 'Act' && output.execution_result) {
      // 执行结果
      return `执行完成\n${JSON.stringify(output.execution_result, null, 2)}`
    }
    
    return JSON.stringify(output, null, 2)
  } catch {
    return String(output)
  }
}

const formatCycleResult = (result: any) => {
  if (!result) return ''
  
  const parts: string[] = []
  
  if (result.observations && Object.keys(result.observations).length > 0) {
    parts.push(`**观察结果**: ${Object.keys(result.observations).length} 项`)
  }
  
  if (result.analysis) {
    parts.push(`**威胁分析**: ${result.analysis.threats?.length || 0} 个威胁`)
  }
  
  if (result.decision) {
    parts.push(`**决策计划**: ${result.decision.name}`)
  }
  
  if (result.execution_result) {
    parts.push(`**执行结果**: 已完成`)
  }
  
  return parts.join('\n\n')
}

const formatDuration = (start: any, end: any) => {
  try {
    const startMs = typeof start === 'object' && start.secs_since_epoch 
      ? start.secs_since_epoch * 1000 
      : new Date(start).getTime()
    const endMs = typeof end === 'object' && end.secs_since_epoch 
      ? end.secs_since_epoch * 1000 
      : new Date(end).getTime()
    const duration = endMs - startMs
    return formatMs(duration)
  } catch {
    return '-'
  }
}

const formatMs = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60000).toFixed(1)}min`
}
</script>

<style scoped>
.travel-step-display {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

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

.collapse:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.collapse-title {
  transition: background-color 0.15s ease;
}

pre {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--bc) / 0.2) transparent;
}

pre::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

pre::-webkit-scrollbar-track {
  background: transparent;
}

pre::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

pre::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}
</style>
