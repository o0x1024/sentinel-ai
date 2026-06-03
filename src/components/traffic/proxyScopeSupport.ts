import type { ProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'

export function isProxyRequestInScope(
  request: ProxyRequest,
  includeRules: ProxyScopeRule[],
  excludeRules: ProxyScopeRule[],
) {
  const enabledIncludeRules = includeRules.filter(rule => rule.enabled)
  const url = safeParseUrl(request.url)
  if (!url) {
    return false
  }

  if (excludeRules.some(rule => rule.enabled && matchesProxyScopeRule(url, rule))) {
    return false
  }

  if (enabledIncludeRules.length === 0) {
    return true
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

  if (normalizedPattern === '*') {
    return true
  }

  const wildcardSuffix = normalizedPattern.match(/^\*\.([a-z0-9.-]+)$/)
  if (wildcardSuffix) {
    const suffix = wildcardSuffix[1]
    return normalizedHost === suffix || normalizedHost.endsWith(`.${suffix}`)
  }

  return exactOrRegexMatches(normalizedPattern, normalizedHost)
}

function matchesPort(url: URL, portPattern: string) {
  const normalizedPattern = portPattern.trim()
  if (!normalizedPattern) {
    return true
  }

  const effectivePort = url.port || (url.protocol === 'https:' ? '443' : '80')
  return exactOrRegexMatches(normalizedPattern, effectivePort)
}

function matchesPath(pathname: string, filePattern: string) {
  const normalizedPattern = filePattern.trim()
  if (!normalizedPattern) {
    return true
  }

  return exactOrRegexMatches(normalizedPattern.toLowerCase(), pathname.toLowerCase())
}

function exactOrRegexMatches(pattern: string, actual: string) {
  if (actual.toLowerCase() === pattern.toLowerCase()) {
    return true
  }

  try {
    return new RegExp(pattern, 'i').test(actual)
  } catch {
    return false
  }
}
