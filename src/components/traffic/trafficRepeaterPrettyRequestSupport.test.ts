import { describe, expect, it } from 'vitest'
import {
  convertRepeaterPrettyRequestToRaw,
  formatRepeaterPrettyRequest,
} from './trafficRepeaterPrettyRequestSupport'

describe('trafficRepeaterPrettyRequestSupport', () => {
  it('pretty prints JSON request bodies while normalizing line endings', () => {
    const rawRequest = [
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{"suspended":false}',
    ].join('\r\n')

    expect(formatRepeaterPrettyRequest(rawRequest)).toBe([
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{',
      '  "suspended":false',
      '}',
    ].join('\n'))
  })

  it('preserves user-authored pretty JSON bodies when converting back to raw', () => {
    const prettyRequest = [
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{',
      '  "suspended": false,',
      '  "role": "admin"',
      '}',
    ].join('\n')

    expect(convertRepeaterPrettyRequestToRaw(prettyRequest)).toBe([
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{',
      '  "suspended": false,',
      '  "role": "admin"',
      '}',
    ].join('\r\n'))
  })

  it('pretty prints multi-line json bodies while preserving source key order', () => {
    const rawRequest = [
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{ "suspended": false,',
      '"12313": "",',
      '"nested": { "2": true, "alpha": false } }',
    ].join('\r\n')

    expect(formatRepeaterPrettyRequest(rawRequest)).toBe([
      'POST /api HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      '',
      '{',
      '  "suspended":false,',
      '  "12313":"",',
      '  "nested":{',
      '    "2":true,',
      '    "alpha":false',
      '  }',
      '}',
    ].join('\n'))
  })

  it('keeps non-json bodies unchanged apart from line-ending normalization', () => {
    const prettyRequest = [
      'POST /submit HTTP/1.1',
      'Host: example.com',
      'Content-Type: text/plain',
      '',
      'alpha',
      'beta',
    ].join('\n')

    expect(convertRepeaterPrettyRequestToRaw(prettyRequest)).toBe([
      'POST /submit HTTP/1.1',
      'Host: example.com',
      'Content-Type: text/plain',
      '',
      'alpha',
      'beta',
    ].join('\r\n'))
  })
})
