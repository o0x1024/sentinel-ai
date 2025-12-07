import { remark } from 'remark';
import remarkHtml from 'remark-html';
import remarkGfm from 'remark-gfm';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';


export const useMessageUtils = () => {
  // 提取平衡的 JSON 对象字符串
  const extractJsonObjects = (content: string): string[] => {
    const results: string[] = []
    let depth = 0
    let start = -1
    
    for (let i = 0; i < content.length; i++) {
      const char = content[i]
      if (char === '{') {
        if (depth === 0) start = i
        depth++
      } else if (char === '}') {
        depth--
        if (depth === 0 && start !== -1) {
          results.push(content.substring(start, i + 1))
          start = -1
        }
      }
    }
    
    return results
  }

  // 尝试解析并格式化执行计划 JSON（支持多个计划）
  const tryFormatPlanJson = (content: string): string | null => {
    const trimmed = content.trim()
    
    // 提取所有 JSON 对象
    const jsonStrings = extractJsonObjects(trimmed)
    
    if (jsonStrings.length === 0) return null
    
    const formattedPlans: string[] = []
    
    for (const jsonStr of jsonStrings) {
      try {
        const json = JSON.parse(jsonStr)
        
        // 检查是否是执行计划格式
        if (json.goal && Array.isArray(json.steps)) {
          formattedPlans.push(formatPlanToMarkdown(json))
        }
      } catch {
        // 忽略解析失败的
      }
    }
    
    if (formattedPlans.length === 0) return null
    
    // 如果有多个计划，用分隔线分开
    if (formattedPlans.length > 1) {
      return formattedPlans.join('\n\n---\n\n')
    }
    
    return formattedPlans[0]
  }

  // 状态图标映射
  const statusIcons: Record<string, string> = {
    'pending': '⏳',
    'running': '🔄',
    'completed': '✅',
    'failed': '❌',
    'skipped': '⏭️',
    'replanned': '🔁'
  }

  // 格式化任务列表为树形结构
  const formatTaskTree = (task: any, indent: string = '', isLast: boolean = true): string[] => {
    const lines: string[] = []
    const icon = statusIcons[task.status] || '⏳'
    const progress = task.progress !== undefined ? ` (${task.progress}%)` : ''
    const prefix = indent + (isLast ? '└── ' : '├── ')
    const childIndent = indent + (isLast ? '    ' : '│   ')
    
    lines.push(`${prefix}[${task.id}] ${icon} ${task.name}${progress}`)
    
    if (task.children && task.children.length > 0) {
      task.children.forEach((child: any, index: number) => {
        const isChildLast = index === task.children.length - 1
        lines.push(...formatTaskTree(child, childIndent, isChildLast))
      })
    }
    
    return lines
  }

  // 将执行计划转换为 Markdown 格式
  const formatPlanToMarkdown = (plan: any): string => {
    const lines: string[] = []
    
    // 目标
    lines.push(`## 🎯 任务目标\n`)
    lines.push(`${plan.goal}\n`)
    
    // 复杂度
    if (plan.complexity) {
      const complexityMap: Record<string, string> = {
        'simple': '🟢 简单',
        'medium': '🟡 中等',
        'complex': '🔴 复杂'
      }
      lines.push(`**复杂度**: ${complexityMap[plan.complexity] || plan.complexity}\n`)
    }
    
    // 任务列表（新格式）
    if (plan.task_list) {
      lines.push(`\n## 📋 任务列表\n`)
      lines.push('```')
      lines.push(`📋 任务进度 (总进度: ${plan.task_list.progress || 0}%)`)
      if (plan.task_list.children && plan.task_list.children.length > 0) {
        plan.task_list.children.forEach((task: any, index: number) => {
          const isLast = index === plan.task_list.children.length - 1
          lines.push(...formatTaskTree(task, '', isLast))
        })
      }
      lines.push('```\n')
    }
    
    // 执行步骤
    if (plan.steps && plan.steps.length > 0) {
      lines.push(`\n## 🔧 执行步骤\n`)
      
      for (const step of plan.steps) {
        const stepId = step.id || '?'
        const taskId = step.task_id ? ` [任务 ${step.task_id}]` : ''
        const deps = step.depends_on?.length ? ` ← 依赖 [${step.depends_on.join(', ')}]` : ''
        const time = step.estimated_time ? ` ⏱️ ~${step.estimated_time}s` : ''
        
        lines.push(`### 步骤 ${stepId}${taskId}${deps}${time}\n`)
        lines.push(`**${step.description || '执行操作'}**\n`)
        lines.push(`- 🔧 工具: \`${step.tool}\``)
        
        if (step.params && Object.keys(step.params).length > 0) {
          lines.push(`- 📝 参数:`)
          for (const [key, value] of Object.entries(step.params)) {
            const displayValue = typeof value === 'string' ? value : JSON.stringify(value)
            lines.push(`  - \`${key}\`: ${displayValue}`)
          }
        }
        
        // 子步骤
        if (step.sub_steps && step.sub_steps.length > 0) {
          lines.push(`- 📑 子步骤:`)
          for (const subStep of step.sub_steps) {
            lines.push(`  - [${subStep.id}] ${subStep.description} → \`${subStep.tool}\``)
          }
        }
        
        lines.push('')
      }
    }
    
    // 预期结果
    if (plan.expected_outcome) {
      lines.push(`\n## ✅ 预期结果\n`)
      lines.push(`${plan.expected_outcome}\n`)
    }
    
    // 备选方案
    if (plan.fallback_plan) {
      lines.push(`\n## 🔄 备选方案\n`)
      lines.push(`${plan.fallback_plan}\n`)
    }
    
    return lines.join('\n')
  }

  // Render markdown content
  const renderMarkdown = (content: string) => {
    // 先尝试格式化执行计划 JSON
    const formattedPlan = tryFormatPlanJson(content)
    if (formattedPlan) {
      content = formattedPlan
    }
    
    // 预处理：
    // 1) 将多重换行规范化
    // 2) 将 [SOURCE n] 转换为可点击锚点的上标链接
    let preprocessed = content.replace(/\n{2,}/g, '\n\n')

    // 将 [SOURCE n] 替换为 <sup><a href="#source-n">[n]</a></sup>
    preprocessed = preprocessed.replace(/\[SOURCE\s+(\d+)\]/g, (_m, n: string) => {
      const num = String(n)
      return `<sup><a href="#source-${num}" class="source-anchor">[${num}]<\/a><\/sup>`
    })

    return remark()
      .use(remarkGfm)
      .use(remarkDirective)
      .use(remarkFrontmatter)
      .use(remarkHtml, { sanitize: false, allowDangerousHtml: true })
      .processSync(preprocessed)
      .toString()
  };

  // Format time display
  const formatTime = (timestamp: Date) => {
    return timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }

  // Format timestamp from number to readable format
  const formatTimestamp = (timestamp: number) => {
    if (!timestamp) return '-'
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  }

  // Format duration
  const formatDuration = (milliseconds: number) => {
    if (milliseconds < 1000) {
      return `${milliseconds}ms`
    }
    const seconds = Math.floor(milliseconds / 1000)
    if (seconds < 60) {
      return `${seconds}秒`
    }
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = seconds % 60
    return `${minutes}分${remainingSeconds}秒`
  }

  // Format tool result
  const formatToolResult = (result: any) => {
    if (typeof result === 'string') return result
    return JSON.stringify(result, null, 2)
  }

  // Get step status class for styling
  const getStepStatusClass = (status: string) => {
    switch (status) {
      case 'pending': return 'badge-ghost'
      case 'executing':
      case 'running': return 'badge-warning'
      case 'completed': return 'badge-success'
      case 'failed': return 'badge-error'
      default: return 'badge-ghost'
    }
  }

  // Get step status text
  const getStepStatusText = (status: string) => {
    switch (status) {
      case 'pending': return '待执行'
      case 'executing': return '执行中'
      case 'running': return '运行中'
      case 'completed': return '已完成'
      case 'failed': return '失败'
      default: return status
    }
  }

  // Get tool status class
  const getToolStatusClass = (status: string) => {
    switch (status) {
      case 'running': return 'badge-warning'
      case 'completed': return 'badge-success'
      case 'failed': return 'badge-error'
      default: return 'badge-ghost'
    }
  }

  // Get result status class
  const getResultStatusClass = (status: string) => {
    switch (status) {
      case 'success':
      case 'completed': return 'badge-success'
      case 'failure':
      case 'failed': return 'badge-error'
      case 'partial':
      case 'running': return 'badge-warning'
      default: return 'badge-ghost'
    }
  }

  // Get result status text
  const getResultStatusText = (status: string) => {
    switch (status) {
      case 'success': return '成功'
      case 'completed': return '已完成'
      case 'failure': return '失败'
      case 'failed': return '失败'
      case 'partial': return '部分完成'
      case 'running': return '运行中'
      default: return status
    }
  }

  // Get architecture badge class
  const getArchBadgeClass = (status: string) => {
    switch (status) {
      case 'stable': return 'badge-success'
      case 'beta': return 'badge-warning'
      case 'experimental': return 'badge-info'
      case 'ai-powered': return 'badge-accent'
      default: return 'badge-ghost'
    }
  }

  // Get architecture badge text
  const getArchBadgeText = (status: string) => {
    switch (status) {
      case 'stable': return 'STABLE'
      case 'beta': return 'BETA'
      case 'experimental': return 'EXPERIMENTAL'
      case 'ai-powered': return 'AI'
      default: return status?.toUpperCase?.() || 'N/A'
    }
  }

  // Get step result data
  const getStepResultData = (step: any) => {
    return step.result_data || step.result || null
  }

  // Get step reasoning result
  const getStepReasoningResult = (step: any) => {
    const resultData = getStepResultData(step)
    if (!resultData) return null
    
    if (typeof resultData === 'object' && resultData.reasoning_result) {
      return resultData.reasoning_result
    }
    
    if (typeof resultData === 'string') {
      try {
        const parsed = JSON.parse(resultData)
        if (parsed.reasoning_result) {
          return parsed.reasoning_result
        }
      } catch (e) {
        // If parsing fails, return null
      }
    }
    
    return null
  }

  // Get execution detailed result
  const getExecutionDetailedResult = (message: any) => {
    // Try to get from the last step's reasoning_result
    if (message.executionPlan?.steps && message.executionPlan.steps.length > 0) {
      const lastStep = message.executionPlan.steps[message.executionPlan.steps.length - 1]
      const reasoningResult = getStepReasoningResult(lastStep)
      if (reasoningResult) {
        return reasoningResult
      }
    }
    
    // Then try from execution result
    if (message.executionResult) {
      if (message.executionResult.result) {
        return message.executionResult.result
      }
      if (message.executionResult.data) {
        return message.executionResult.data
      }
    }
    
    return null
  }

  // Convert backend step status to frontend status
  const getStepStatusFromBackend = (backendStatus: string) => {
    const statusMap: Record<string, string> = {
      'Pending': 'pending',
      'Running': 'executing',
      'Executing': 'executing',
      'Completed': 'completed',
      'Failed': 'failed',
      'Skipped': 'skipped',
      'Cancelled': 'failed'
    }
    return statusMap[backendStatus] || backendStatus?.toLowerCase()
  }

  return {
    renderMarkdown,
    formatTime,
    formatTimestamp,
    formatDuration,
    formatToolResult,
    getStepStatusClass,
    getStepStatusText,
    getToolStatusClass,
    getResultStatusClass,
    getResultStatusText,
    getArchBadgeClass,
    getArchBadgeText,
    getStepResultData,
    getStepReasoningResult,
    getExecutionDetailedResult,
    getStepStatusFromBackend
  }
}