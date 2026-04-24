import { beforeEach, describe, expect, it } from 'vitest'
import {
  getIntruderBuiltInPayloadListDefinition,
  getIntruderPayloadQualityStats,
  matchesIntruderBuiltInPayloadList,
  normalizeIntruderPayloadSearchText,
  rankIntruderBuiltInPayloadLists,
} from './intruderBuiltInPayloadCatalog'

describe('intruder built-in payload catalog', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('normalizes payload library search text', () => {
    expect(normalizeIntruderPayloadSearchText('User-Name 01')).toBe('username01')
  })

  it('matches aliases and payload content during search', () => {
    const definition = getIntruderBuiltInPayloadListDefinition('phones-cn')
    expect(definition).not.toBeNull()
    expect(matchesIntruderBuiltInPayloadList(definition!, 'mobile')).toBe(true)
    expect(matchesIntruderBuiltInPayloadList(definition!, '13800138000')).toBe(true)
  })

  it('ranks account lists for login-oriented positions', () => {
    const ranked = rankIntruderBuiltInPayloadLists({
      attackType: 'pitchfork',
      requestText: 'POST /login HTTP/1.1\r\nHost: example.com\r\n\r\nusername=$admin$&password=$secret$',
      positionValues: ['admin'],
    })

    expect(ranked.slice(0, 3)).toContain('usernames')
  })

  it('computes payload quality stats', () => {
    const stats = getIntruderPayloadQualityStats(['admin', 'admin', '123456', ' 张三 '])

    expect(stats.total).toBe(4)
    expect(stats.unique).toBe(3)
    expect(stats.duplicates).toBe(1)
    expect(stats.weakRatio).toBeGreaterThan(0)
    expect(stats.dirtyCount).toBe(1)
    expect(stats.charsetDistribution.withUnicode).toBe(1)
  })
})
