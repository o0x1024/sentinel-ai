import { describe, expect, it } from 'vitest'
import { createDefaultIntruderDictionaryPayloadConfig } from './intruderAppDictionaryPayloads'
import { countPayloadSet, expandPayloadSet } from './payloads'
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

describe('intruder payload expansion', () => {
  it('expands brute forcer payloads across a length range', () => {
    const payloadSet = createPayloadSet({
      payloadType: 'bruteForcer',
      bruteForceCharacterSet: 'ab',
      bruteForceMinLength: 1,
      bruteForceMaxLength: 2,
    })

    expect(countPayloadSet(payloadSet)).toBe(6)
    expect(expandPayloadSet(payloadSet)).toEqual(['a', 'b', 'aa', 'ab', 'ba', 'bb'])
  })

  it('limits brute forcer payload expansion when requested', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'bruteForcer',
        bruteForceCharacterSet: 'ab',
        bruteForceMinLength: 1,
        bruteForceMaxLength: 3,
      }),
      4,
    )

    expect(values).toEqual(['a', 'b', 'aa', 'ab'])
  })

  it('expands number payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'numbers',
        numberFrom: 7,
        numberTo: 9,
        numberStep: 1,
        numberPadWidth: 3,
      }),
    )

    expect(values).toEqual(['007', '008', '009'])
  })

  it('expands date payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'dates',
        dateFrom: '2026-03-01',
        dateTo: '2026-03-03',
        dateStepDays: 1,
        dateFormat: 'yyyyMMdd',
      }),
    )

    expect(values).toEqual(['20260301', '20260302', '20260303'])
  })

  it('expands character list payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'characterList',
        characterList: 'aabc',
      }),
    )

    expect(values).toEqual(['a', 'b', 'c'])
  })

  it('expands null payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'nullPayloads',
        nullCount: 3,
        nullValue: '',
      }),
    )

    expect(values).toEqual(['', '', ''])
  })

  it('expands character substitution payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'characterSubstitution',
        substitutionSource: 'admin',
        substitutionRules: 'a=@,4\ni=1',
      }),
    )

    expect(values).toContain('admin')
    expect(values).toContain('@dm1n')
    expect(values).toContain('4dm1n')
  })

  it('expands username generator payloads', () => {
    const values = expandPayloadSet(
      createPayloadSet({
        payloadType: 'usernameGenerator',
        usernameFirstNames: 'John',
        usernameLastNames: 'Smith',
        usernameFormats: '{first}.{last}\n{f}{last}',
      }),
    )

    expect(values).toEqual(['john.smith', 'jsmith'])
  })
})
