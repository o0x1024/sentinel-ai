import { describe, expect, it } from 'vitest'
import {
  autoMarkIntruderPositions,
  buildIntruderAttackPlan,
  clearIntruderMarkers,
  extractIntruderPositions,
  wrapSelectionWithMarkers,
} from './attack'
import type { IntruderPayloadSet } from './types'

function createPayloadSet(overrides: Partial<IntruderPayloadSet> = {}): IntruderPayloadSet {
  return {
    id: 'set-1',
    name: 'Payload set 1',
    payloadType: 'simpleList',
    payloadsText: '',
    urlEncode: false,
    filePath: '',
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 100,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '2026-01-01',
    dateTo: '2026-01-07',
    dateStepDays: 1,
    dateFormat: 'yyyy-MM-dd',
    nullCount: 0,
    nullValue: '',
    usernameFirstNames: '',
    usernameLastNames: '',
    usernameFormats: '{first}.{last}',
    ...overrides,
  }
}

describe('intruder attack helpers', () => {
  it('extracts marker positions', () => {
    const positions = extractIntruderPositions('GET /?q=§admin§&role=§user§ HTTP/1.1')
    expect(positions).toHaveLength(2)
    expect(positions[0].value).toBe('admin')
    expect(positions[1].value).toBe('user')
  })

  it('wraps selected text with intruder markers', () => {
    const result = wrapSelectionWithMarkers('username=admin', 9, 14)
    expect(result).toBe('username=§admin§')
  })

  it('auto marks query string values', () => {
    const request = 'GET /search?q=test&lang=zh HTTP/1.1\r\nHost: example.com\r\n\r\n'
    const marked = autoMarkIntruderPositions(request)
    expect(marked).toContain('q=§test§')
    expect(marked).toContain('lang=§zh§')
  })

  it('builds sniper attack requests', () => {
    const plan = buildIntruderAttackPlan({
      template: 'GET /?q=§test§&role=§user§ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({ payloadsText: 'admin\nroot' }),
      ],
      maxRequests: 20,
    })

    expect(plan.requests).toHaveLength(4)
    expect(plan.requests[0].requestText).toContain('q=admin')
    expect(plan.requests[2].requestText).toContain('role=admin')
  })

  it('truncates cluster bomb plans above the limit', () => {
    const plan = buildIntruderAttackPlan({
      template: 'POST /login HTTP/1.1\r\nHost: example.com\r\n\r\nusername=§admin§&password=§pass§',
      attackType: 'clusterBomb',
      payloadSets: [
        createPayloadSet({ id: 'set-1', payloadsText: 'a\nb\nc' }),
        createPayloadSet({ id: 'set-2', name: 'Payload set 2', payloadsText: '1\n2\n3' }),
      ],
      maxRequests: 4,
    })

    expect(plan.totalGenerated).toBe(9)
    expect(plan.requests).toHaveLength(4)
    expect(plan.truncated).toBe(true)
  })

  it('clears markers cleanly', () => {
    expect(clearIntruderMarkers('a§b§c')).toBe('abc')
  })

  it('applies payload processing rules before substitution', () => {
    const plan = buildIntruderAttackPlan({
      template: 'GET /?q=§test§ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({ payloadsText: 'admin' }),
      ],
      payloadProcessingRules: [
        {
          id: 'rule-1',
          enabled: true,
          type: 'prefix',
          matchValue: '',
          replaceValue: 'pre-',
        },
        {
          id: 'rule-2',
          enabled: true,
          type: 'uppercase',
          matchValue: '',
          replaceValue: '',
        },
      ],
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=PRE-ADMIN')
  })
})
