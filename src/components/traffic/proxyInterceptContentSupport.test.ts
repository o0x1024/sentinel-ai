import { describe, expect, it } from 'vitest'
import {
  getInterceptItemKey,
  resolveInterceptForwardModifiedContent,
} from './proxyInterceptContentSupport'
import type { InterceptedItem } from './proxyInterceptSupport'

describe('proxyInterceptContentSupport', () => {
  it('does not send modified content when the editor text is unchanged', () => {
    expect(resolveInterceptForwardModifiedContent('GET / HTTP/1.1\n', 'GET / HTTP/1.1\n')).toBeUndefined()
  })

  it('sends modified content only when the editor text differs from the original intercept text', () => {
    expect(resolveInterceptForwardModifiedContent('GET /admin HTTP/1.1\n', 'GET / HTTP/1.1\n')).toBe('GET /admin HTTP/1.1\n')
  })

  it('builds stable original-content keys per intercepted item type and id', () => {
    const item = {
      type: 'request',
      data: {
        id: 'req-1',
        timestamp: 1,
        method: 'GET',
        url: 'https://example.com/',
        path: '/',
        protocol: 'HTTP/1.1',
        headers: {},
        body: '',
      },
    } satisfies InterceptedItem

    expect(getInterceptItemKey(item)).toBe('request:req-1')
  })
})
