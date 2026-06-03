import { describe, expect, it } from 'vitest'
import { applyPayloadProcessingRules } from './payloadProcessing'

describe('payload processing rules', () => {
  it('applies rules only when the contains condition matches', async () => {
    const output = await applyPayloadProcessingRules('admin-user', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'prefix',
        matchValue: '',
        replaceValue: 'pre-',
        conditionType: 'contains',
        conditionValue: 'admin',
        caseSensitive: false,
      },
      {
        id: 'rule-2',
        enabled: true,
        type: 'suffix',
        matchValue: '',
        replaceValue: '-tail',
        conditionType: 'contains',
        conditionValue: 'missing',
        caseSensitive: false,
      },
    ])

    expect(output).toBe('pre-admin-user')
  })

  it('supports case-insensitive regex replacement', async () => {
    const output = await applyPayloadProcessingRules('AdminADMIN', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'replaceRegex',
        matchValue: 'admin',
        replaceValue: 'user',
        conditionType: 'always',
        conditionValue: '',
        caseSensitive: false,
      },
    ])

    expect(output).toBe('useruser')
  })

  it('supports substring and base64 decode rules', async () => {
    const output = await applyPayloadProcessingRules('prefix-YWRtaW4=', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'substring',
        matchValue: '',
        replaceValue: '',
        substringStart: 7,
        substringLength: null,
      },
      {
        id: 'rule-2',
        enabled: true,
        type: 'decode',
        matchValue: '',
        replaceValue: '',
        codecType: 'base64',
      },
    ])

    expect(output).toBe('admin')
  })

  it('supports reverse substring from the tail of the payload', async () => {
    const output = await applyPayloadProcessingRules('abcdef', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'reverseSubstring',
        matchValue: '',
        replaceValue: '',
        substringStart: 1,
        substringLength: 3,
      },
    ])

    expect(output).toBe('cde')
  })

  it('supports sha256 hashing', async () => {
    const output = await applyPayloadProcessingRules('abc', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'hash',
        matchValue: '',
        replaceValue: '',
        hashAlgorithm: 'sha256',
      },
    ])

    expect(output).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
  })

  it('can append the raw payload after the processed payload', async () => {
    const output = await applyPayloadProcessingRules('abc', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'uppercase',
        matchValue: '',
        replaceValue: '',
      },
      {
        id: 'rule-2',
        enabled: true,
        type: 'addRawPayload',
        matchValue: '',
        replaceValue: '',
        rawPayloadPlacement: 'after',
      },
    ], {
      originalPayload: 'abc',
      baseValue: 'seed',
    })

    expect(output).toBe('ABCabc')
  })

  it('skips payloads when skip regex matches', async () => {
    const output = await applyPayloadProcessingRules('admin@example.com', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'skipRegex',
        matchValue: '@example\\.com$',
        replaceValue: '',
        caseSensitive: false,
      },
    ])

    expect(output).toBeNull()
  })

  it('replaces the {base} placeholder with the original position value', async () => {
    const output = await applyPayloadProcessingRules('prefix-{base}-suffix', [
      {
        id: 'rule-1',
        enabled: true,
        type: 'replaceBaseValue',
        matchValue: '',
        replaceValue: '',
      },
    ], {
      baseValue: 'admin',
      originalPayload: 'prefix-{base}-suffix',
    })

    expect(output).toBe('prefix-admin-suffix')
  })
})
