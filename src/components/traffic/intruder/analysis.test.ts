import { describe, expect, it } from 'vitest'
import {
  createDefaultGrepPayloadSettings,
  evaluateIntruderGrepExtracts,
  evaluateIntruderGrepMatches,
  evaluateIntruderPayloadReflections,
} from './analysis'

describe('intruder grep helpers', () => {
  it('evaluates grep matches', () => {
    const matches = evaluateIntruderGrepMatches('HTTP/1.1 200 OK\r\n\r\nhello admin', [
      {
        id: 'match-1',
        name: 'Admin',
        enabled: true,
        pattern: 'admin',
        patternType: 'literal',
        caseSensitive: false,
        excludeHeaders: false,
        invert: false,
      },
    ])

    expect(matches['match-1']).toBe(1)
  })

  it('counts regex matches in the response body only', () => {
    const matches = evaluateIntruderGrepMatches('X-Test: admin\r\n\r\nadmin admin', [
      {
        id: 'match-1',
        name: 'Admin',
        enabled: true,
        pattern: 'admin',
        patternType: 'regex',
        caseSensitive: false,
        excludeHeaders: true,
        invert: false,
      },
    ])

    expect(matches['match-1']).toBe(2)
  })

  it('extracts regex capture groups', () => {
    const extracts = evaluateIntruderGrepExtracts('Set-Cookie: session=abc123;', [
      {
        id: 'extract-1',
        name: 'Session',
        enabled: true,
        pattern: 'session=([^;]+)',
        groupIndex: 1,
        caseSensitive: false,
      },
    ])

    expect(extracts['extract-1']).toBe('abc123')
  })

  it('counts reflected payloads', () => {
    const count = evaluateIntruderPayloadReflections(
      'HTTP/1.1 200 OK\r\n\r\nHello admin and admin%40corp',
      ['admin', 'admin@corp'],
      {
        ...createDefaultGrepPayloadSettings(),
        enabled: true,
      },
    )

    expect(count).toBe(3)
  })
})
