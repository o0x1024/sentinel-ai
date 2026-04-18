import { describe, expect, it } from 'vitest'
import { createDefaultIntruderDictionaryPayloadConfig } from './intruderAppDictionaryPayloads'
import { extractIntruderPayloadSourceSummaryEntries } from './intruderPayloadSourceSummary'
import type { IntruderPayloadSet } from './types'

function createPayloadSet(overrides: Partial<IntruderPayloadSet> = {}): IntruderPayloadSet {
  return {
    id: 'set-1',
    name: 'Payload set 1',
    payloadType: 'simpleList',
    payloadsText: '',
    urlEncode: false,
    urlEncodeCharacters: String.raw`./\=<>?+&*;:"' {}|^#`,
    dictionaryConfig: createDefaultIntruderDictionaryPayloadConfig(),
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
    filePath: '',
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 0,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '2026-01-01',
    dateTo: '2026-01-03',
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

describe('intruder payload source summary', () => {
  it('extracts app dictionary sources from payload sets', () => {
    const entries = extractIntruderPayloadSourceSummaryEntries([
      createPayloadSet({
        payloadType: 'appDictionary',
        dictionaryConfig: {
          sources: [
            { type: 'dictionary', dictionaryId: 'DICT-1', dictionaryName: 'Users' },
            { type: 'default_dictionary', dictType: 'password' },
          ],
          limit: 100,
          deduplicate: true,
        },
      }),
    ])

    expect(entries).toHaveLength(1)
    expect(entries[0].pluginId).toBe('intruder.appDictionary')
    expect(entries[0].sources).toEqual([
      { type: 'dictionary', dictionaryId: 'DICT-1', dictionaryName: 'Users' },
      { type: 'default_dictionary', dictType: 'password' },
    ])
  })
})
