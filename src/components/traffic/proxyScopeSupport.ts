import type { ProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'

export function isProxyRequestInScope(
  request: ProxyRequest,
  includeRules: ProxyScopeRule[],
  excludeRules: ProxyScopeRule[],
) {
  const enabledIncludeRules = includeRules.filter(rule => rule.enabled)
  if (enabledIncludeRules.length === 0) {
    return false
  }

  const url = safeParseUrl(request.url)
  if (!url) {
    return false
  }

  if (excludeRules.some(rule => rule.enabled && matchesProxyScopeRule(url, rule))) {
    return false
  }

  return enabledIncludeRules.some(rule => matchesProxyScopeRule(url, rule))
}

export function matchesProxyScopeRule(url: URL, rule: ProxyScopeRule) {
  if (!matchesProtocol(url, rule.protocol)) {
    return false
  }

  if (!matchesHost(url.hostname, rule.host_or_ip_range)) {
    return false
  }

  if (!matchesPort(url, rule.port)) {
    return false
  }

  if (!matchesPath(url.pathname, rule.file)) {
    return false
  }

  return true
}

function safeParseUrl(value: string) {
  try {
    return new URL(value)
  } catch {
    return null
  }
}

function matchesProtocol(url: URL, protocol: string) {
  const normalizedRule = protocol.trim().toLowerCase()
  if (!normalizedRule || normalizedRule === 'any') {
    return true
  }

  return url.protocol.replace(':', '').toLowerCase() === normalizedRule
}

function matchesHost(hostname: string, pattern: string) {
  const normalizedPattern = pattern.trim().toLowerCase()
  if (!normalizedPattern) {
    return true
  }

  const normalizedHost = hostname.trim().toLowerCase()
  if (!normalizedHost) {
    return false
  }

  return wildcardPatternToRegExp(normalizedPattern).test(normalizedHost)
}

function matchesPort(url: URL, portPattern: string) {
  const normalizedPattern = portPattern.trim()
  if (!normalizedPattern) {
    return true
  }

  const effectivePort = url.port || (url.protocol === 'https:' ? '443' : '80')
  return wildcardPatternToRegExp(normalizedPattern).test(effectivePort)
}

function matchesPath(pathname: string, filePattern: string) {
  const normalizedPattern = filePattern.trim()
  if (!normalizedPattern) {
    return true
  }

  return wildcardPatternToRegExp(normalizedPattern.toLowerCase()).test(pathname.toLowerCase())
}

function wildcardPatternToRegExp(pattern: string) {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&')
  const source = `^${escaped.replace(/\*/g, '.*')}$`
  return new RegExp(source, 'i')
}
