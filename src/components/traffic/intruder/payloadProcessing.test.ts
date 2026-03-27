import { describe, expect, it } from 'vitest'
import { applyPayloadProcessingRules } from './payloadProcessing'

describe('payload processing rules', () => {
  it('applies rules only when the contains condition matches', () => {
    const output = applyPayloadProcessingRules('admin-user', [
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

  it('supports case-insensitive regex replacement', () => {
    const output = applyPayloadProcessingRules('AdminADMIN', [
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
})
