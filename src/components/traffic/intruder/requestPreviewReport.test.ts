import { describe, expect, it } from 'vitest'
import type { IntruderPayloadSourceSummaryEntry } from './intruderPayloadSourceSummary'
import { buildRequestPreviewDebugReport } from './requestPreviewReport'

describe('buildRequestPreviewDebugReport', () => {
  it('includes overall and per-processor sections', () => {
    const originalRequest = [
      'POST /login?mode=basic HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=1&sign=old',
    ].join('\r\n')
    const firstTraceRequest = [
      'POST /login?mode=advanced HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=1&sign=old',
    ].join('\r\n')
    const finalRequest = [
      'POST /login?mode=advanced HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/x-www-form-urlencoded',
      '',
      'user=alice&ts=2&sign=new',
    ].join('\r\n')
    const payloadSources: IntruderPayloadSourceSummaryEntry[] = [
      {
        payloadSetName: 'Payload Set 1',
        pluginId: 'intruder_dictionary_payload_generator',
        sources: [
          {
            type: 'dictionary',
            dictionaryId: 'DICT-1',
            dictionaryName: 'Common Users',
          },
          {
            type: 'default_dictionary',
            dictType: 'username',
          },
        ],
      },
    ]

    const report = buildRequestPreviewDebugReport({
      payloadSummary: 'user=alice',
      payloadSources,
      originalRequest,
      finalRequest,
      traces: [
        {
          pluginId: 'mode-switcher',
          requestText: firstTraceRequest,
          output: { mode: 'advanced' },
        },
        {
          pluginId: 'request-signer',
          requestText: finalRequest,
          output: { signature: 'new', canonical: 'ts=2&user=alice' },
        },
      ],
    })

    expect(report).toContain('Intruder Request Preview Debug Report')
    expect(report).toContain('Payload summary: user=alice')
    expect(report).toContain('Payload sources:')
    expect(report).toContain('Payload Set 1: Common Users (DICT-1), Default dictionary (username)')
    expect(report).toContain('Overall diff')
    expect(report).toContain('Processor trace')
    expect(report).toContain('1. mode-switcher')
    expect(report).toContain('2. request-signer')
    expect(report).toContain('Query parameters: 1 changed, 0 unchanged')
    expect(report).toContain('Form parameters: 2 changed, 1 unchanged')
    expect(report).toContain('"signature": "new"')
  })
})
