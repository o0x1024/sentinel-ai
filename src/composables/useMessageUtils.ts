import { remark } from 'remark';
import remarkHtml from 'remark-html';
import remarkGfm from 'remark-gfm';


export const useMessageUtils = () => {
  // Render markdown content
  const renderMarkdown = (content: string) => {
    return remark()
    .use(remarkHtml)
    .use(remarkGfm)
    .processSync(content).toString();
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