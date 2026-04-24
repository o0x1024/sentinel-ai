import { describe, expect, it } from 'vitest'
import {
  applyIntruderPayloadImportMode,
  buildIntruderPayloadPreview,
  buildIntruderPayloadTemplateApplicationSets,
  estimateIntruderRequestsAfterImport,
} from './intruderPayloadLibrarySupport'
import type { IntruderPayloadSet } from './types'

function createPayloadSet(overrides: Partial<IntruderPayloadSet> = {}): IntruderPayloadSet {
  return {
    id: 'set-1',
    name: 'Payload Set 1',
    payloadType: 'simpleList',
    payloadsText: 'alpha\nbeta',
    urlEncode: false,
    urlEncodeCharacters: '',
    dictionaryConfig: {
      sources: [],
      limit: 200,
      deduplicate: true,
    },
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
    filePath: '',
    bruteForceCharacterSet: 'abcdefghijklmnopqrstuvwxyz0123456789',
    bruteForceMinLength: 4,
    bruteForceMaxLength: 4,
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 0,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '',
    dateTo: '',
    dateStepDays: 1,
    dateFormat: 'yyyy-MM-dd',
    nullCount: 0,
    nullValue: '',
    usernameFirstNames: '',
    usernameLastNames: '',
    usernameFormats: '',
    ...overrides,
  }
}

describe('intruder payload library support', () => {
  it('applies import modes correctly', () => {
    expect(applyIntruderPayloadImportMode(['alpha'], ['beta'], 'append').nextItems).toEqual(['alpha', 'beta'])
    expect(applyIntruderPayloadImportMode(['alpha'], ['beta'], 'replace').nextItems).toEqual(['beta'])
    expect(applyIntruderPayloadImportMode(['alpha'], ['alpha', 'beta'], 'mergeDeduplicate').nextItems).toEqual(['alpha', 'beta'])
  })

  it('estimates requests after replacing a payload set', () => {
    const estimate = estimateIntruderRequestsAfterImport({
      attackType: 'sniper',
      positionsLength: 2,
      payloadSets: [createPayloadSet()],
      activePayloadSetId: 'set-1',
      nextPayloadsText: 'one\ntwo\nthree',
    })

    expect(estimate).toBe(6)
  })

  it('builds previews and template sets', () => {
    expect(buildIntruderPayloadPreview(['a', 'b', 'c'], 2)).toEqual(['a', 'b'])

    const sets = buildIntruderPayloadTemplateApplicationSets([
      { name: 'Users', sourceId: 'usernames' },
      { name: 'Passwords', sourceId: 'passwords' },
    ])
    expect(sets).toHaveLength(2)
    expect(sets[0]?.payloadsText.length).toBeGreaterThan(0)
  })
})
