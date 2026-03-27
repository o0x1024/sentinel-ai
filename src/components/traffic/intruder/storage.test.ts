import { describe, expect, it } from 'vitest'
import { exportIntruderResultsCsv } from './storage'
import type { IntruderAttackResult } from './types'

function createResult(): IntruderAttackResult {
  return {
    id: 'result-1',
    index: 1,
    payloadSummary: 'P1=admin',
    payloadValues: ['admin'],
    statusCode: 200,
    responseLength: 512,
    wordCount: 10,
    lineCount: 4,
    responseTimeMs: 120,
    rawRequest: 'GET / HTTP/1.1',
    rawResponse: 'HTTP/1.1 200 OK',
    grepMatches: {},
    grepExtracts: {},
  }
}

describe('intruder result export', () => {
  it('exports columns in the selected order', () => {
    const csv = exportIntruderResultsCsv(
      [createResult()],
      [],
      [],
      ['statusCode', 'index', 'payloadSummary'],
    )

    expect(csv).toContain('Status,Request,Payload')
    expect(csv).toContain('200,0,P1=admin')
  })
})
