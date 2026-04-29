import type { ProxyScopeRule } from './proxyConfigurationTypes'

function escapeRegexLiteral(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function buildExactHostRegex(hostname: string) {
  const normalizedHostname = hostname.trim().replace(/\.+$/g, '').toLowerCase()
  if (!normalizedHostname) {
    return ''
  }

  return `^${escapeRegexLiteral(normalizedHostname)}$`
}

export function buildProxyScopeRuleFromUrl(url: URL): ProxyScopeRule {
  return {
    enabled: true,
    protocol: url.protocol.replace(':', '').toLowerCase() || 'any',
    host_or_ip_range: buildExactHostRegex(url.hostname),
    port: url.port,
    file: url.pathname || '/',
  }
}
