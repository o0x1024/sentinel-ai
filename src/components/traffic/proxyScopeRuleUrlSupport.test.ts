import { describe, expect, it } from 'vitest'
import { buildExactHostRegex, buildProxyScopeRuleFromUrl } from './proxyScopeRuleUrlSupport'

describe('buildExactHostRegex', () => {
  it('escapes regex metacharacters and anchors the hostname', () => {
    expect(buildExactHostRegex('console.volcengine.com')).toBe('^console\\.volcengine\\.com$')
  })

  it('normalizes case and trailing dots', () => {
    expect(buildExactHostRegex('Console.Volcengine.Com.')).toBe('^console\\.volcengine\\.com$')
  })
})

describe('buildProxyScopeRuleFromUrl', () => {
  it('builds a scope rule with exact host regex from a URL', () => {
    const rule = buildProxyScopeRuleFromUrl(
      new URL('https://console.volcengine.com/api/list?region=cn-beijing'),
    )

    expect(rule).toEqual({
      enabled: true,
      protocol: 'https',
      host_or_ip_range: '^console\\.volcengine\\.com$',
      port: '',
      file: '/api/list',
    })
  })

  it('preserves explicit ports and exact ip matching', () => {
    const rule = buildProxyScopeRuleFromUrl(new URL('http://10.0.0.8:8443/health'))

    expect(rule).toEqual({
      enabled: true,
      protocol: 'http',
      host_or_ip_range: '^10\\.0\\.0\\.8$',
      port: '8443',
      file: '/health',
    })
  })
})
