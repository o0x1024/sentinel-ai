import { describe, expect, it } from 'vitest'

import {
  formatJsonStringIfPossible,
  formatJsonValueIfPossible,
  normalizeJsonDisplayValue,
  tryParseStructuredJson,
} from './jsonFormatting'

describe('jsonFormatting', () => {
  it('formats raw json object strings', () => {
    expect(formatJsonStringIfPossible('{"ok":true,"items":[1,2]}')).toBe(
      '{\n  "ok": true,\n  "items": [\n    1,\n    2\n  ]\n}',
    )
  })

  it('formats fenced json strings', () => {
    expect(formatJsonStringIfPossible('```json\n{"ok":true}\n```')).toBe('{\n  "ok": true\n}')
  })

  it('ignores non-json text', () => {
    expect(formatJsonStringIfPossible('command finished successfully')).toBeNull()
  })

  it('parses json arrays but not plain scalar text', () => {
    expect(tryParseStructuredJson('[1,2,3]')).toEqual([1, 2, 3])
    expect(tryParseStructuredJson('true')).toBeNull()
  })

  it('unwraps tool-style text envelopes with nested json strings', () => {
    const raw =
      '[{"type":"text","text":"{\\"success\\":true,\\"critique\\":\\"review complete\\",\\"risk_level\\":\\"critical\\"}"}]'

    expect(formatJsonValueIfPossible(raw)).toBe(
      '{\n  "success": true,\n  "critique": "review complete",\n  "risk_level": "critical"\n}',
    )
  })

  it('normalizes nested json strings inside regular objects', () => {
    expect(
      normalizeJsonDisplayValue({
        outer: '{"ok":true}',
        keep: 'plain text',
      }),
    ).toEqual({
      outer: {
        ok: true,
      },
      keep: 'plain text',
    })
  })
})
