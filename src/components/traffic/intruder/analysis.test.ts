import { describe, expect, it } from 'vitest'
import { evaluateIntruderGrepExtracts, evaluateIntruderGrepMatches } from './analysis'

describe('intruder grep helpers', () => {
  it('evaluates grep matches', () => {
    const matches = evaluateIntruderGrepMatches('HTTP/1.1 200 OK\r\n\r\nhello admin', [
      {
        id: 'match-1',
        name: 'Admin',
        enabled: true,
        pattern: 'admin',
        caseSensitive: false,
        invert: false,
      },
    ])

    expect(matches['match-1']).toBe(true)
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
})
