import { describe, expect, it } from 'vitest'
import type { ProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'
import { isProxyRequestInScope } from './proxyScopeSupport'

const requestForUrl = (url: string): ProxyRequest => ({
  id: 1,
  url,
  host: new URL(url).hostname,
  scheme: new URL(url).protocol.replace(':', ''),
  method: 'GET',
  status_code: 200,
  response_size: 0,
  response_time: 0,
  timestamp: '2026-04-30T00:00:00.000Z',
})

const scopeRule = (rule: Partial<ProxyScopeRule>): ProxyScopeRule => ({
  enabled: true,
  protocol: 'any',
  host_or_ip_range: '',
  port: '',
  file: '',
  ...rule,
})

describe('isProxyRequestInScope', () => {
  it('allows all requests when no include rules are enabled', () => {
    expect(isProxyRequestInScope(
      requestForUrl('https://api.other.com/orders'),
      [
        scopeRule({
          enabled: false,
          host_or_ip_range: '^console\\.volcengine\\.com$',
        }),
      ],
      [],
    )).toBe(true)
  })

  it('honors exclude rules when no include rules are enabled', () => {
    const includeRules = [
      scopeRule({
        enabled: false,
        host_or_ip_range: '^console\\.volcengine\\.com$',
      }),
    ]
    const excludeRules = [
      scopeRule({
        host_or_ip_range: '^blocked\\.example\\.com$',
      }),
    ]

    expect(isProxyRequestInScope(
      requestForUrl('https://blocked.example.com/orders'),
      includeRules,
      excludeRules,
    )).toBe(false)
    expect(isProxyRequestInScope(
      requestForUrl('https://allowed.example.com/orders'),
      includeRules,
      excludeRules,
    )).toBe(true)
  })

  it('matches pasted-url exact host regex rules', () => {
    expect(isProxyRequestInScope(
      requestForUrl('https://lumi.console.volcengine.com/path'),
      [
        scopeRule({
          protocol: 'https',
          host_or_ip_range: '^lumi\\.console\\.volcengine\\.com$',
        }),
      ],
      [],
    )).toBe(true)
  })
})
