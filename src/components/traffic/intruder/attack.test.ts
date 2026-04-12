import { describe, expect, it } from 'vitest'
import {
  autoMarkIntruderPositions,
  buildIntruderAttackPlan,
  clearIntruderMarkers,
  encodeSelectedPayloadCharacters,
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
    urlEncodeCharacters: String.raw`./\=<>?+&*;:"' {}|^#`,
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
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
    const positions = extractIntruderPositions('GET /?q=$admin$&role=$user$ HTTP/1.1')
    expect(positions).toHaveLength(2)
    expect(positions[0].value).toBe('admin')
    expect(positions[1].value).toBe('user')
  })

  it('wraps selected text with intruder markers', () => {
    const result = wrapSelectionWithMarkers('username=admin', 9, 14)
    expect(result).toBe('username=$admin$')
  })

  it('auto marks query string values', () => {
    const request = 'GET /search?q=test&lang=zh HTTP/1.1\r\nHost: example.com\r\n\r\n'
    const marked = autoMarkIntruderPositions(request)
    expect(marked).toContain('q=$test$')
    expect(marked).toContain('lang=$zh$')
  })

  it('builds sniper attack requests', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$test$&role=$user$ HTTP/1.1',
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

  it('encodes only selected payload characters', () => {
    const encoded = encodeSelectedPayloadCharacters('a+b/c=', '+/=')
    expect(encoded).toBe('a%2Bb%2Fc%3D')
  })

  it('truncates cluster bomb plans above the limit', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'POST /login HTTP/1.1\r\nHost: example.com\r\n\r\nusername=$admin$&password=$pass$',
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
    expect(clearIntruderMarkers('a$b$c')).toBe('abc')
  })

  it('keeps supporting legacy section markers', () => {
    const positions = extractIntruderPositions('GET /?q=§admin§ HTTP/1.1')
    expect(positions).toHaveLength(1)
    expect(positions[0].value).toBe('admin')
    expect(clearIntruderMarkers('before §admin§ after')).toBe('before admin after')
  })

  it('applies payload processing rules before substitution', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$test$ HTTP/1.1',
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

  it('skips payloads removed by processing rules', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$test$ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({ payloadsText: 'admin\nroot' }),
      ],
      payloadProcessingRules: [
        {
          id: 'rule-1',
          enabled: true,
          type: 'skipRegex',
          matchValue: '^admin$',
          replaceValue: '',
          caseSensitive: false,
        },
      ],
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=root')
  })

  it('replaces {base} using the original value of each attack position', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$admin$&role=$user$ HTTP/1.1',
      attackType: 'batteringRam',
      payloadSets: [
        createPayloadSet({ payloadsText: 'pre-{base}-post' }),
      ],
      payloadProcessingRules: [
        {
          id: 'rule-1',
          enabled: true,
          type: 'replaceBaseValue',
          matchValue: '',
          replaceValue: '',
        },
      ],
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=pre-admin-post')
    expect(plan.requests[0].requestText).toContain('role=pre-user-post')
  })

  it('can include both hashed and raw payload forms in one request', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$seed$ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({ payloadsText: 'abc' }),
      ],
      payloadProcessingRules: [
        {
          id: 'rule-1',
          enabled: true,
          type: 'hash',
          matchValue: '',
          replaceValue: '',
          hashAlgorithm: 'sha1',
        },
        {
          id: 'rule-2',
          enabled: true,
          type: 'addRawPayload',
          matchValue: '',
          replaceValue: '',
          rawPayloadPlacement: 'after',
        },
      ],
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=a9993e364706816aba3e25717850c26c9cd0d89dabc')
  })

  it('applies selective URL encoding to the final payload', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$seed$ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({
          payloadsText: 'a+b/c=',
          urlEncode: true,
          urlEncodeCharacters: '+/=',
        }),
      ],
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=a%2Bb%2Fc%3D')
  })

  it('uses async resolver for extension-generated payloads', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$test$ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({
          payloadType: 'extensionGenerated',
          pluginId: 'intruder.demo',
        }),
      ],
      payloadResolver: async () => ['alpha', 'beta'],
      maxRequests: 10,
    })

    expect(plan.totalGenerated).toBe(2)
    expect(plan.requests).toHaveLength(2)
    expect(plan.requests[0].requestText).toContain('q=alpha')
    expect(plan.requests[1].requestText).toContain('q=beta')
  })

  it('runs payload plugin processors after built-in payload rules', async () => {
    const plan = await buildIntruderAttackPlan({
      template: 'GET /?q=$test$ HTTP/1.1',
      attackType: 'sniper',
      payloadSets: [
        createPayloadSet({ payloadsText: 'admin' }),
      ],
      payloadProcessingRules: [
        {
          id: 'rule-1',
          enabled: true,
          type: 'uppercase',
          matchValue: '',
          replaceValue: '',
        },
      ],
      payloadPluginProcessor: async (payload) => `${payload}-plugin`,
      maxRequests: 10,
    })

    expect(plan.requests).toHaveLength(1)
    expect(plan.requests[0].requestText).toContain('q=ADMIN-plugin')
  })
})
