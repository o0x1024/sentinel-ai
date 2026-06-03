import { describe, expect, it } from 'vitest'

import { locateValidationIssue } from './validationIssueLocator'

describe('locateValidationIssue', () => {
  it('locates get_input_schema issues', () => {
    const code = `
      export function get_input_schema() { return { type: 'object' } }
      export async function analyze() { return { success: true, data: {} } }
    `

    const target = locateValidationIssue(
      code,
      'runtime-schema',
      'runtime_schema_execution_failed',
      '运行时 get_input_schema 校验失败: Function not found',
    )

    expect(target).not.toBeNull()
    expect(code.slice(target!.from, target!.to)).toContain('get_input_schema')
  })

  it('locates request_processor rawRequest issues', () => {
    const code = `
      export async function analyze(input) {
        return { success: true, data: { rawRequest: input.rawRequest } }
      }
    `

    const target = locateValidationIssue(
      code,
      'runtime-schema',
      'raw_request_return_shape',
      'request_processor 未明显返回 `data.rawRequest`',
    )

    expect(target).not.toBeNull()
    expect(code.slice(target!.from, target!.to)).toContain('rawRequest')
  })

  it('falls back to first non-whitespace character', () => {
    const code = '\n\nconst x = 1'
    const target = locateValidationIssue(code, 'unknown', 'unknown_issue', '未知问题')

    expect(target).toEqual({ from: 2, to: 3 })
  })
})
