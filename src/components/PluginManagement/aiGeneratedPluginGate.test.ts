import { describe, expect, it } from 'vitest'

import {
  combineAiValidationResults,
  formatAiGeneratedPluginGateMessage,
  normalizePluginValidationIssues,
  type PluginCodeValidationResult,
  normalizeRuntimeSchemaValidationResult,
  validateAiGeneratedPluginCode,
} from './aiGeneratedPluginGate'

describe('validateAiGeneratedPluginCode', () => {
  it('accepts a minimal intruder payload generator skeleton', () => {
    const result = validateAiGeneratedPluginCode(
      `
      export function get_input_schema() { return { type: 'object' } }
      export async function analyze() {
        return { success: true, data: { payloads: ['admin'] } }
      }
      globalThis.get_input_schema = get_input_schema
      globalThis.analyze = analyze
      `,
      { mainCategory: 'intruder', category: 'payload_generator' },
    )

    expect(result.errors).toEqual([])
  })

  it('flags missing request_processor rawRequest contract', () => {
    const result = validateAiGeneratedPluginCode(
      `
      export function get_input_schema() { return { type: 'object' } }
      export async function analyze() {
        return { success: true, data: {} }
      }
      globalThis.get_input_schema = get_input_schema
      globalThis.analyze = analyze
      `,
      { mainCategory: 'intruder', category: 'request_processor' },
    )

    expect(result.errors.map(issue => issue.message)).toContain('request_processor 未明显返回 `data.rawRequest`')
  })

  it('formats gate messages with errors and warnings', () => {
    const message = formatAiGeneratedPluginGateMessage({
      errors: [{ code: 'analyze_export', message: '缺少 `analyze` 导出函数' }],
      warnings: [{ code: 'tool_output_success_field', message: '未明显看到 `ToolOutput.success` 字段' }],
    })

    expect(message).toContain('AI 生成结构校验未通过')
    expect(message).toContain('错误:')
    expect(message).toContain('提示:')
  })

  it('normalizes traffic-only validator rules for intruder plugins', () => {
    const result = normalizePluginValidationIssues({
      is_valid: false,
      syntax_valid: true,
      has_required_functions: false,
      security_check_passed: true,
      errors: ['Missing required function: scan_transaction'],
      warnings: ['Recommended function/API not found: get_metadata'],
    }, { mainCategory: 'intruder', category: 'request_processor' })

    expect(result.errors).toEqual([])
    expect(result.warnings).toEqual([])
  })

  it('combines gate and static validation feedback', () => {
    const staticValidation: PluginCodeValidationResult = {
      is_valid: false,
      syntax_valid: false,
      has_required_functions: true,
      security_check_passed: true,
      errors: ['TypeScript syntax validation failed'],
      warnings: ['Syntax validation skipped: deno unavailable'],
    }

    const result = combineAiValidationResults(
      { errors: [{ code: 'analyze_export', message: '缺少 `analyze` 导出函数' }], warnings: [] },
      normalizePluginValidationIssues(staticValidation, { mainCategory: 'agent', category: 'utility' }),
    )

    expect(result.errors.map(issue => issue.message)).toEqual([
      '缺少 `analyze` 导出函数',
      'TypeScript syntax validation failed',
    ])
    expect(result.warnings.map(issue => issue.message)).toEqual(['Syntax validation skipped: deno unavailable'])
  })

  it('reports runtime schema execution failures for intruder plugins', () => {
    const result = normalizeRuntimeSchemaValidationResult({
      success: false,
      error: 'Function not found: get_input_schema',
    }, { mainCategory: 'intruder', category: 'payload_generator' })

    expect(result.errors).toEqual([{
      code: 'runtime_schema_execution_failed',
      message: '运行时 get_input_schema 校验失败: Function not found: get_input_schema',
    }])
  })

  it('warns when runtime schema root type is not object', () => {
    const result = normalizeRuntimeSchemaValidationResult({
      success: true,
      schema: { type: 'string' },
      warnings: [],
    }, { mainCategory: 'agent', category: 'utility' })

    expect(result.errors).toEqual([])
    expect(result.warnings.map(issue => issue.message)).toContain('运行时 schema 的根 `type` 不是 `object`')
  })
})
