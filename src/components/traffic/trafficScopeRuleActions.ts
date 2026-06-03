import type { ProxyConfigState, ProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'
import { buildProxyScopeRuleFromUrl } from './proxyScopeRuleUrlSupport'

export type TrafficScopeRuleMode = 'include' | 'exclude'

export interface ScopeRuleDialogOpenPayload {
  mode: TrafficScopeRuleMode
  rule: ProxyScopeRule
}

export type ScopeRuleDialogSavePayload = ScopeRuleDialogOpenPayload

export interface TrafficScopeRuleDialogHandle {
  open: (payload: ScopeRuleDialogOpenPayload) => void
}

export function cloneProxyScopeRule(rule: ProxyScopeRule): ProxyScopeRule {
  return {
    enabled: rule.enabled,
    protocol: rule.protocol,
    host_or_ip_range: rule.host_or_ip_range,
    port: rule.port,
    file: rule.file,
  }
}

export function buildScopeRuleFromProxyRequest(request: Pick<ProxyRequest, 'url'>): ProxyScopeRule {
  return buildProxyScopeRuleFromUrl(new URL(request.url))
}

export function appendScopeRuleToProxyConfig(
  config: ProxyConfigState,
  mode: TrafficScopeRuleMode,
  rule: ProxyScopeRule,
): ProxyConfigState {
  if (!Array.isArray(config.scope_include_rules) || !Array.isArray(config.scope_exclude_rules)) {
    throw new Error('Proxy scope rule lists are missing')
  }

  const includeRules = config.scope_include_rules.map(cloneProxyScopeRule)
  const excludeRules = config.scope_exclude_rules.map(cloneProxyScopeRule)
  const nextRule = cloneProxyScopeRule(rule)

  if (mode === 'include') {
    includeRules.push(nextRule)
  } else {
    excludeRules.push(nextRule)
  }

  return {
    ...config,
    scope_include_rules: includeRules,
    scope_exclude_rules: excludeRules,
  }
}
