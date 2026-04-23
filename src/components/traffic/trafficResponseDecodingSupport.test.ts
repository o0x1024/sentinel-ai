import { describe, expect, it } from 'vitest'
import {
  buildTrafficDisplayedRawResponse,
  resolveStoredTrafficResponseBodyText,
  resolveTrafficResponseBodyText,
} from './trafficResponseDecodingSupport'
import type { TrafficDisplaySettings } from './trafficDisplaySettings'

const defaultSettings: TrafficDisplaySettings = {
  fontFamily: 'monospace',
  fontSize: 13,
  highlightRequestSyntax: true,
  highlightResponseSyntax: true,
  wrapLongLines: true,
  showLineEndings: false,
  prettyPrintByDefault: true,
  showSendToRepeater: true,
  showSendToComparer: true,
  showSendToIntruder: true,
  charsetMode: 'auto',
  specificCharset: 'UTF-8',
}

describe('trafficResponseDecodingSupport', () => {
  it('decodes stored base64 bodies with a specific charset', () => {
    const body = resolveStoredTrafficResponseBodyText(
      '[BASE64]6Q==',
      JSON.stringify([{ name: 'Content-Type', value: 'text/plain; charset=windows-1252' }]),
      {
        ...defaultSettings,
        charsetMode: 'specific',
        specificCharset: 'windows-1252',
      },
    )

    expect(body).toBe('é')
  })

  it('renders raw bytes without charset decoding when requested', () => {
    const body = resolveTrafficResponseBodyText(
      '',
      'text/plain; charset=utf-8',
      {
        ...defaultSettings,
        charsetMode: 'rawBytes',
      },
      '6Q==',
    )

    expect(body.charCodeAt(0)).toBe(0xe9)
  })

  it('rebuilds raw responses from replay metadata using decoded body bytes', () => {
    const raw = buildTrafficDisplayedRawResponse({
      statusCode: 200,
      versionObserved: 'HTTP/2',
      statusText: 'OK',
      headers: [{ name: 'Content-Type', value: 'text/plain; charset=windows-1252' }],
      bodyText: '',
      bodyBytesBase64: '6Q==',
    }, {
      ...defaultSettings,
      charsetMode: 'specific',
      specificCharset: 'windows-1252',
    })

    expect(raw).toBe('HTTP/2 200 OK\r\nContent-Type: text/plain; charset=windows-1252\r\n\r\né')
  })
})
