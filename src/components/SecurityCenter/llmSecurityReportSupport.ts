import type { LlmSuiteDefinition, LlmTestRunView } from '../../api/llmTest'

export type CaseResultEntry = {
  case_id: string
  verdict: string
  risk_level: string
  confidence: number
  latency_ms: number
  evidence_ref?: string
  executed_at?: string
  model_output?: Record<string, any>
  assertion_results?: Array<{ type: string; passed: boolean; reason?: string; score?: number }>
  owasp?: { id: string; title: string }
}

export const getSelectedRunCases = (run: LlmTestRunView | null) => {
  const summary = run?.results_summary as any
  const cases = summary?.cases
  return Array.isArray(cases) ? cases : []
}

export const getRunSummaryStats = (run: LlmTestRunView | null) => {
  const summary = run?.results_summary as any
  return {
    executed: Number(summary?.cases_executed || 0),
    passed: Number(summary?.cases_passed || 0),
    failed: Number(summary?.cases_failed || 0),
  }
}

export const getCaseMessagesFromSuites = (
  suites: LlmSuiteDefinition[],
  caseId: string,
): Array<{ role: string; content: string }> => {
  for (const suite of suites) {
    const found = suite.cases?.find((entry) => entry.case_id === caseId)
    if (!found) continue
    if (Array.isArray(found.messages)) {
      return found.messages
        .filter((message) => message?.role && typeof message?.content === 'string')
        .map((message) => ({ role: String(message.role), content: String(message.content) }))
    }
    if (found.user_prompt) {
      return [{ role: 'user', content: found.user_prompt }]
    }
  }
  return []
}

export const formatModelOutput = (output: Record<string, any> | undefined): string => {
  if (!output) return ''
  if (output.error) {
    const error = output.error
    if (typeof error === 'string') return `Error: ${error}`
    if (typeof error === 'object') {
      const message = error.message ?? error.msg ?? error.detail ?? error.description
      if (message) return `Error: ${message}`
      return `Error: ${JSON.stringify(error, null, 2)}`
    }
    return `Error: ${String(error)}`
  }
  if (output.content) return String(output.content)
  if (output.choices) {
    try {
      const choice = output.choices[0]
      return choice?.message?.content ?? choice?.text ?? JSON.stringify(output, null, 2)
    } catch {
      return JSON.stringify(output, null, 2)
    }
  }
  if (output.message) return String(output.message)
  if (output.text) return String(output.text)
  return JSON.stringify(output, null, 2)
}
