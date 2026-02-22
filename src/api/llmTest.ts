import { invoke } from '@tauri-apps/api/core'

export interface LlmTestTarget {
  app_id: string
  env: string
  endpoint: string
}

export interface LlmTestExecutionConfig {
  mode: string
  parallelism?: number
  timeout_ms?: number
  max_retries?: number
}

export interface LlmTestAuthConfig {
  type: string
  api_key_ref?: string
  api_key?: string
  header_name?: string
  bearer_token?: string
  username?: string
  password?: string
}

export interface LlmTestPolicyConfig {
  allow_tools?: string[]
  deny_tools?: string[]
  rate_limit_rps?: number
}

export interface LlmTestAdapterConfig {
  custom_headers?: Record<string, string>
  message_template?: string
  response_extract_path?: string
}

export interface CreateLlmTestRunRequest {
  suite_id: string
  suite_version?: string
  target: LlmTestTarget
  execution: LlmTestExecutionConfig
  auth?: LlmTestAuthConfig
  policy?: LlmTestPolicyConfig
  adapter?: LlmTestAdapterConfig
  metadata?: Record<string, string>
}

export interface LlmTestRunCreated {
  run_id: string
  status: string
  created_at: string
}

export interface LlmTestMessage {
  role: string
  content: string
}

export interface AssertionResult {
  type: string
  passed: boolean
  reason?: string
  score?: number
}

export interface ExecuteLlmTestCaseResponse {
  run_id: string
  case_id: string
  verdict: string
  risk_level: string
  confidence: number
  latency_ms: number
  model_output: unknown
  assertion_results: AssertionResult[]
  evidence_ref: string
}

export interface ExecuteLlmTestCaseRequest {
  input: {
    messages: LlmTestMessage[]
    attachments?: unknown[]
    context?: unknown
  }
  assertions?: Array<{
    type: string
    pattern?: string
    policy_id?: string
    threshold?: number
  }>
  owasp?: {
    id?: string
    title?: string
  }
  idempotency_key?: string
}

export interface ExecuteLlmTestBatchRequest {
  cases: Array<{
    case_id: string
    input: {
      messages: LlmTestMessage[]
      attachments?: unknown[]
      context?: unknown
    }
    assertions?: Array<{
      type: string
      pattern?: string
      policy_id?: string
      threshold?: number
    }>
    owasp?: {
      id?: string
      title?: string
    }
    idempotency_key?: string
  }>
  stop_on_failure?: boolean
}

export interface ExecuteLlmTestBatchResponse {
  run_id: string
  total_cases: number
  completed_cases: number
  failed_cases: number
  stopped_early: boolean
  results: ExecuteLlmTestCaseResponse[]
}

export interface LlmTestRunView {
  run_id: string
  status: string
  progress: number
  suite_id?: string
  suite_version?: string
  target: LlmTestTarget
  created_at: string
  started_at?: string
  completed_at?: string
  results_summary: unknown
  auth_config?: unknown
  adapter_config?: unknown
  execution_config?: Record<string, unknown>
}

export interface LlmTestListRunsRequest {
  limit?: number
  offset?: number
  status_filter?: 'Created' | 'Running' | 'Paused' | 'Completed' | 'Failed' | 'Cancelled'
}

export interface LlmTestResponse<T> {
  success: boolean
  data?: T
  message?: string
}

export interface LlmSuiteDefinition {
  id: string
  name: string
  version: string
  description?: string
  cases?: Array<{
    case_id: string
    owasp_id?: string
    owasp_title?: string
    user_prompt: string
    messages?: LlmTestMessage[]
    regex_not_match?: string
  }>
}

export async function llmTestCreateRun(
  request: CreateLlmTestRunRequest
): Promise<LlmTestResponse<LlmTestRunCreated>> {
  return await invoke('llm_test_create_run', { request })
}

export async function llmTestExecuteCase(
  runId: string,
  caseId: string,
  request: ExecuteLlmTestCaseRequest
): Promise<LlmTestResponse<ExecuteLlmTestCaseResponse>> {
  return await invoke('llm_test_execute_case', {
    runId,
    caseId,
    request,
  })
}

export async function llmTestExecuteCases(
  runId: string,
  request: ExecuteLlmTestBatchRequest
): Promise<LlmTestResponse<ExecuteLlmTestBatchResponse>> {
  return await invoke('llm_test_execute_cases', {
    runId,
    request,
  })
}

export async function llmTestGetRun(
  runId: string
): Promise<LlmTestResponse<LlmTestRunView>> {
  return await invoke('llm_test_get_run', { runId })
}

export async function llmTestListRuns(
  request: LlmTestListRunsRequest = {}
): Promise<LlmTestResponse<LlmTestRunView[]>> {
  return await invoke('llm_test_list_runs', { request })
}

export async function llmTestStopRun(
  runId: string,
  reason?: string
): Promise<LlmTestResponse<LlmTestRunView>> {
  return await invoke('llm_test_stop_run', {
    runId,
    request: reason ? { reason } : undefined,
  })
}

export async function llmTestResetRun(
  runId: string
): Promise<LlmTestResponse<LlmTestRunView>> {
  return await invoke('llm_test_reset_run', {
    runId,
    request: {
      scope: 'session',
      clear_memory: true,
      clear_cache: true,
    },
  })
}

export async function llmTestDeleteRun(
  runId: string
): Promise<LlmTestResponse<boolean>> {
  return await invoke('llm_test_delete_run', {
    runId,
  })
}

// ─── LLM Test Suite CRUD (dedicated table) ──────────────────────────

interface LlmTestSuiteRow {
  id: string
  name: string
  version: string
  description: string
  cases: string  // JSON string
}

/**
 * Load all LLM test suites from the dedicated `llm_test_suites` table.
 */
export async function loadLlmSuitesFromConfig(): Promise<LlmSuiteDefinition[]> {
  const resp = await invoke<LlmTestResponse<LlmTestSuiteRow[]>>('llm_test_list_suites')
  if (!resp?.success || !resp.data) {
    return []
  }
  return resp.data
    .filter(row => row.id && row.name)
    .map(row => {
      let cases: LlmSuiteDefinition['cases'] = []
      try {
        const parsed = JSON.parse(row.cases || '[]')
        if (Array.isArray(parsed)) {
          cases = parsed
            .filter((c: any) => {
              if (!c?.case_id) return false
              if (typeof c?.user_prompt === 'string' && c.user_prompt.trim().length > 0) return true
              if (!Array.isArray(c?.messages)) return false
              return c.messages.some((m: any) => m?.role && typeof m?.content === 'string')
            })
            .map((c: any) => ({
              case_id: String(c.case_id),
              owasp_id: c.owasp_id ? String(c.owasp_id) : '',
              owasp_title: c.owasp_title ? String(c.owasp_title) : '',
              user_prompt: c.user_prompt ? String(c.user_prompt) : '',
              messages: Array.isArray(c.messages)
                ? c.messages
                  .filter((m: any) => m?.role && typeof m?.content === 'string')
                  .map((m: any) => ({ role: String(m.role), content: String(m.content) }))
                : undefined,
              regex_not_match: c.regex_not_match ? String(c.regex_not_match) : '',
            }))
        }
      } catch {
        // ignore parse errors
      }
      return {
        id: row.id,
        name: row.name,
        version: row.version,
        description: row.description || '',
        cases,
      }
    })
}

/**
 * Save all LLM test suites. Each suite is upserted individually into `llm_test_suites`.
 */
export async function saveLlmSuitesToConfig(
  suites: LlmSuiteDefinition[]
): Promise<void> {
  for (const suite of suites) {
    const casesJson = JSON.stringify(
      Array.isArray(suite.cases)
        ? suite.cases.map(c => ({
          case_id: c.case_id,
          owasp_id: c.owasp_id || '',
          owasp_title: c.owasp_title || '',
          user_prompt: c.user_prompt,
          messages: Array.isArray(c.messages)
            ? c.messages.map(m => ({ role: m.role, content: m.content }))
            : undefined,
          regex_not_match: c.regex_not_match || '',
        }))
        : []
    )

    await invoke('llm_test_save_suite', {
      suite: {
        id: suite.id,
        name: suite.name,
        version: suite.version,
        description: suite.description || '',
        cases: casesJson,
      },
    })
  }
}

/**
 * Delete a single LLM test suite by id.
 */
export async function deleteLlmSuite(suiteId: string): Promise<void> {
  await invoke('llm_test_delete_suite', { suiteId })
}
