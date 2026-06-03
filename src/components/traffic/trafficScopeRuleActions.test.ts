import { describe, expect, it } from 'vitest'
import {
  appendScopeRuleToProxyConfig,
  buildScopeRuleFromProxyRequest,
  cloneProxyScopeRule,
} from './trafficScopeRuleActions'
import { createDefaultProxyConfig, type ProxyScopeRule } from './proxyConfigurationTypes'

const baseRule: ProxyScopeRule = {
  enabled: true,
  protocol: 'https',
  host_or_ip_range: '^example\\.com$',
  port: '',
  file: '/api',
}

describe('buildScopeRuleFromProxyRequest', () => {
  it('creates an editable exact-host scope rule from a history URL', () => {
    expect(buildScopeRuleFromProxyRequest({
      url: 'https://Api.Example.com:8443/v1/users?id=1',
    })).toEqual({
      enabled: true,
      protocol: 'https',
      host_or_ip_range: '^api\\.example\\.com$',
      port: '8443',
      file: '/v1/users',
    })
  })
})

describe('appendScopeRuleToProxyConfig', () => {
  it('appends include scope rules without mutating the source config', () => {
    const source = createDefaultProxyConfig()
    const next = appendScopeRuleToProxyConfig(source, 'include', baseRule)

    expect(source.scope_include_rules).toEqual([])
    expect(next.scope_include_rules).toEqual([baseRule])
    expect(next.scope_exclude_rules).toEqual([])
  })

  it('appends exclude scope rules and clones rule objects', () => {
    const source = createDefaultProxyConfig()
    const next = appendScopeRuleToProxyConfig(source, 'exclude', baseRule)

    expect(next.scope_exclude_rules).toEqual([baseRule])
    expect(next.scope_exclude_rules[0]).not.toBe(baseRule)
    expect(cloneProxyScopeRule(next.scope_exclude_rules[0])).toEqual(baseRule)
  })
})
