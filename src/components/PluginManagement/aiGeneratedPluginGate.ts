import type { NewPluginMetadata } from './types'

export interface AiValidationIssue {
  code: string
  message: string
}

export interface StructuredAiValidationResult {
  errors: AiValidationIssue[]
  warnings: AiValidationIssue[]
}

export interface AiValidationSection {
  key: string
  title: string
  errors: AiValidationIssue[]
  warnings: AiValidationIssue[]
}

export interface AiValidationReport {
  title: string
  sections: AiValidationSection[]
}

export interface PluginCodeValidationResult {
  is_valid: boolean
  syntax_valid: boolean
  has_required_functions: boolean
  security_check_passed: boolean
  errors: string[]
  warnings: string[]
}

export interface PluginRuntimeSchemaValidationResult {
  success: boolean
  schema?: unknown
  schemas?: Record<string, unknown> | null
  error?: string | null
  issueCode?: string | null
  issue_code?: string | null
  warnings?: string[]
}

const hasPattern = (code: string, patterns: RegExp[]): boolean => patterns.some(pattern => pattern.test(code))

const pushIssue = (bucket: AiValidationIssue[], code: string, message: string) => {
  bucket.push({ code, message })
}

const formatRule = (passed: boolean, bucket: AiValidationIssue[], code: string, message: string) => {
  if (!passed) {
    pushIssue(bucket, code, message)
  }
}

export function validateAiGeneratedPluginCode(
  code: string,
  metadata: Pick<NewPluginMetadata, 'mainCategory' | 'category'>,
): StructuredAiValidationResult {
  const normalizedCode = code.replace(/\r\n/g, '\n')
  const errors: AiValidationIssue[] = []
  const warnings: AiValidationIssue[] = []

  formatRule(normalizedCode.trim().length > 0, errors, 'empty_generated_code', '生成结果为空')
  formatRule(!/```/.test(normalizedCode), errors, 'markdown_code_fence', '生成结果仍包含 Markdown 代码块标记')

  if (metadata.mainCategory === 'traffic') {
    formatRule(
      hasPattern(normalizedCode, [/export\s+async\s+function\s+scan_transaction\b/, /export\s+function\s+scan_transaction\b/]),
      errors,
      'scan_transaction_export',
      '缺少 `scan_transaction` 导出函数',
    )
    formatRule(
      /globalThis\.scan_transaction\s*=/.test(normalizedCode),
      errors,
      'scan_transaction_global_binding',
      '缺少 `globalThis.scan_transaction = scan_transaction` 绑定',
    )
    formatRule(
      /return\s+findings\b|return\s*\[\s*\]/.test(normalizedCode),
      warnings,
      'traffic_findings_return_path',
      '未明显看到 findings 返回路径',
    )
    return { errors, warnings }
  }

  formatRule(
    hasPattern(normalizedCode, [/export\s+function\s+get_input_schema\b/, /export\s+const\s+get_input_schema\b/]),
    errors,
    'get_input_schema_export',
    '缺少 `get_input_schema` 导出函数',
  )
  if (metadata.mainCategory === 'agent' || metadata.mainCategory === 'bounty') {
    formatRule(
      hasPattern(normalizedCode, [/export\s+function\s+get_output_schema\b/, /export\s+const\s+get_output_schema\b/]),
      errors,
      'get_output_schema_export',
      '缺少 `get_output_schema` 导出函数',
    )
    formatRule(
      /globalThis\.get_output_schema\s*=/.test(normalizedCode),
      errors,
      'get_output_schema_global_binding',
      '缺少 `globalThis.get_output_schema = get_output_schema` 绑定',
    )
  }
  formatRule(
    hasPattern(normalizedCode, [/export\s+async\s+function\s+analyze\b/, /export\s+function\s+analyze\b/]),
    errors,
    'analyze_export',
    '缺少 `analyze` 导出函数',
  )
  formatRule(
    /globalThis\.get_input_schema\s*=/.test(normalizedCode),
    errors,
    'get_input_schema_global_binding',
    '缺少 `globalThis.get_input_schema = get_input_schema` 绑定',
  )
  formatRule(
    /globalThis\.analyze\s*=/.test(normalizedCode),
    errors,
    'analyze_global_binding',
    '缺少 `globalThis.analyze = analyze` 绑定',
  )
  formatRule(/success\s*:/.test(normalizedCode), warnings, 'tool_output_success_field', '未明显看到 `ToolOutput.success` 字段')

  if (metadata.mainCategory === 'intruder') {
    if (metadata.category === 'payload_generator') {
      formatRule(/payloads\s*:/.test(normalizedCode), errors, 'payloads_return_shape', 'payload_generator 未明显返回 `data.payloads`')
    } else if (metadata.category === 'payload_processor') {
      formatRule(
        /payload\s*:|skip\s*:/.test(normalizedCode),
        errors,
        'payload_processor_return_shape',
        'payload_processor 未明显返回 `data.payload` 或 `data.skip`',
      )
    } else if (metadata.category === 'request_processor') {
      formatRule(/rawRequest\s*:/.test(normalizedCode), errors, 'raw_request_return_shape', 'request_processor 未明显返回 `data.rawRequest`')
      formatRule(/rawRequest\b/.test(normalizedCode), warnings, 'request_processor_raw_request_usage', 'request_processor 中未明显使用 `rawRequest` 输入')
    }
  }

  return { errors, warnings }
}

export function formatAiGeneratedPluginGateMessage(
  result: StructuredAiValidationResult,
  title = 'AI 生成结构校验未通过',
): string {
  const lines = [title]

  if (result.errors.length > 0) {
    lines.push('错误:')
    lines.push(...result.errors.map(issue => `- ${issue.message}`))
  }

  if (result.warnings.length > 0) {
    lines.push('提示:')
    lines.push(...result.warnings.map(issue => `- ${issue.message}`))
  }

  return lines.join('\n')
}

export function normalizePluginValidationResult(
  result: PluginCodeValidationResult,
  metadata: Pick<NewPluginMetadata, 'mainCategory' | 'category'>,
): PluginCodeValidationResult {
  if (metadata.mainCategory === 'traffic') {
    return result
  }

  const ignoredErrorPatterns = [
    /^Missing required function: scan_transaction$/,
  ]

  const ignoredWarningPatterns = [
    /^Recommended function\/API not found: op_emit_finding$/,
    /^Recommended function\/API not found: get_metadata$/,
  ]

  const errors = result.errors.filter(message => !ignoredErrorPatterns.some(pattern => pattern.test(message)))
  const warnings = result.warnings.filter(message => !ignoredWarningPatterns.some(pattern => pattern.test(message)))

  return {
    ...result,
    errors,
    warnings,
    has_required_functions: result.has_required_functions || errors.length !== result.errors.length,
    is_valid: errors.length === 0 && result.syntax_valid && result.security_check_passed,
  }
}

export function normalizePluginValidationIssues(
  result: PluginCodeValidationResult,
  metadata: Pick<NewPluginMetadata, 'mainCategory' | 'category'>,
): StructuredAiValidationResult {
  const normalized = normalizePluginValidationResult(result, metadata)
  const errors = normalized.errors.map(message => ({
    code: inferValidationIssueCode('static', message),
    message,
  }))
  const warnings = normalized.warnings.map(message => ({
    code: inferValidationIssueCode('static', message),
    message,
  }))

  return { errors, warnings }
}

export function combineAiValidationResults(
  ...results: Array<StructuredAiValidationResult>
): StructuredAiValidationResult {
  return {
    errors: results.flatMap(result => result.errors),
    warnings: results.flatMap(result => result.warnings),
  }
}

export function buildAiValidationReport(sections: AiValidationSection[]): AiValidationReport | null {
  const normalizedSections = sections.filter(section => section.errors.length > 0 || section.warnings.length > 0)
  return normalizedSections.length > 0
    ? {
        title: 'AI 生成校验结果',
        sections: normalizedSections,
      }
    : null
}

export function summarizeAiValidationReport(report: AiValidationReport | null): string {
  if (!report) return ''

  const errorCount = report.sections.reduce((sum, section) => sum + section.errors.length, 0)
  const warningCount = report.sections.reduce((sum, section) => sum + section.warnings.length, 0)
  const parts: string[] = []

  if (errorCount > 0) {
    parts.push(`${errorCount} 个错误`)
  }

  if (warningCount > 0) {
    parts.push(`${warningCount} 个提示`)
  }

  return parts.join('，')
}

export function normalizeRuntimeSchemaValidationResult(
  result: PluginRuntimeSchemaValidationResult,
  metadata: Pick<NewPluginMetadata, 'mainCategory' | 'category'>,
): StructuredAiValidationResult {
  if (metadata.mainCategory === 'traffic') {
    return { errors: [], warnings: [] }
  }

  if (!result.success) {
    return {
      errors: [{
        code: result.issueCode ?? result.issue_code ?? 'runtime_schema_execution_failed',
        message: `运行时 schema contract 校验失败: ${result.error || '未知错误'}`,
      }],
      warnings: [],
    }
  }

  const warnings = (result.warnings || []).map(message => ({
    code: inferValidationIssueCode('runtime-schema', message),
    message,
  }))
  const errors: AiValidationIssue[] = []
  const schema = result.schema

  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    pushIssue(errors, 'runtime_schema_shape', '运行时 `get_input_schema()` 未返回对象类型的 schema')
    return { errors, warnings }
  }

  if (!('type' in schema)) {
    pushIssue(warnings, 'runtime_schema_missing_type', '运行时 schema 未声明 `type` 字段')
  } else if ((schema as Record<string, unknown>).type !== 'object') {
    pushIssue(warnings, 'runtime_schema_root_type', '运行时 schema 的根 `type` 不是 `object`')
  }

  return { errors, warnings }
}

export function inferValidationIssueCode(
  sectionKey: string,
  message: string,
  explicitCode?: string | null,
): string {
  if (explicitCode) return explicitCode

  const normalized = `${sectionKey} ${message}`.toLowerCase()

  if (normalized.includes('markdown') || normalized.includes('```') || normalized.includes('代码块')) {
    return 'markdown_code_fence'
  }
  if (normalized.includes('scan_transaction')) {
    return normalized.includes('globalthis') ? 'scan_transaction_global_binding' : 'scan_transaction_export'
  }
  if (normalized.includes('get_input_schema')) {
    return normalized.includes('globalthis') ? 'get_input_schema_global_binding' : 'get_input_schema_export'
  }
  if (normalized.includes('get_output_schema')) {
    return normalized.includes('globalthis') ? 'get_output_schema_global_binding' : 'get_output_schema_export'
  }
  if (normalized.includes('analyze')) {
    return normalized.includes('globalthis') ? 'analyze_global_binding' : 'analyze_export'
  }
  if (normalized.includes('payloads')) {
    return 'payloads_return_shape'
  }
  if (normalized.includes('rawrequest')) {
    return normalized.includes('使用') || normalized.includes('input')
      ? 'request_processor_raw_request_usage'
      : 'raw_request_return_shape'
  }
  if (normalized.includes('payload') || normalized.includes('skip')) {
    return 'payload_processor_return_shape'
  }
  if (normalized.includes('schema')) {
    return 'runtime_schema_shape'
  }

  return `${sectionKey}_generic_issue`
}

export function toValidationIssues(
  sectionKey: string,
  messages: string[],
  explicitCode?: string | null,
): AiValidationIssue[] {
  return messages.map(message => ({
    code: inferValidationIssueCode(sectionKey, message, explicitCode),
    message,
  }))
}
