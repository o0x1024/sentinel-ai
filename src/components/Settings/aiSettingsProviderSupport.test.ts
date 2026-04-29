import { describe, expect, it } from 'vitest'

import {
  buildExtraHeadersFromInputRows,
  createExtraHeaderInputRows,
  formatExtraBodyJson,
  parseExtraBodyJson,
} from './aiSettingsProviderSupport'

describe('aiSettingsProviderSupport', () => {
  it('creates editable header rows from an extra_headers object', () => {
    expect(createExtraHeaderInputRows({
      Authorization: 'Bearer token',
      'X-Trace-Id': 'trace-1',
    })).toEqual([
      { key: 'Authorization', value: 'Bearer token' },
      { key: 'X-Trace-Id', value: 'trace-1' },
    ])
  })

  it('builds extra_headers from key-value rows and ignores blank rows', () => {
    expect(buildExtraHeadersFromInputRows([
      { key: ' Authorization ', value: 'Bearer token' },
      { key: '', value: '' },
      { key: 'X-Empty-Value', value: '' },
    ])).toEqual({
      ok: true,
      headers: {
        Authorization: 'Bearer token',
        'X-Empty-Value': '',
      },
    })
  })

  it('rejects a value without a header name', () => {
    expect(buildExtraHeadersFromInputRows([{ key: '', value: 'Bearer token' }])).toEqual({
      ok: false,
      reason: 'missing-key',
    })
  })

  it('rejects duplicate header names case-insensitively', () => {
    expect(buildExtraHeadersFromInputRows([
      { key: 'Authorization', value: 'Bearer one' },
      { key: 'authorization', value: 'Bearer two' },
    ])).toEqual({
      ok: false,
      reason: 'duplicate-key',
      key: 'authorization',
    })
  })

  it('parses extra_body JSON objects', () => {
    expect(parseExtraBodyJson('{"enable_thinking": false}')).toEqual({
      ok: true,
      body: { enable_thinking: false },
    })
  })

  it('rejects non-object extra_body JSON', () => {
    expect(parseExtraBodyJson('[1, 2]')).toEqual({
      ok: false,
      reason: 'not-object',
    })
  })

  it('formats extra_body JSON objects for editing', () => {
    expect(formatExtraBodyJson({ top_k: 20 })).toBe('{\n  "top_k": 20\n}')
  })
})
