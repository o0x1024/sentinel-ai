import type { FilterRule } from './proxyInterceptTypes'

export function buildSavedInterceptFilterRule(filterRule: FilterRule) {
  const matchTypeMap: Record<string, string> = {
    domain: 'domain_name',
    url: 'url',
    method: 'http_method',
    fileExt: 'file_extension',
    header: 'any_header',
    status: 'status_code',
    contentType: 'content_type_header',
  }

  const relationshipMap: Record<string, string> = {
    matches: 'matches',
    notMatches: 'does_not_match',
    contains: 'matches',
    notContains: 'does_not_match',
  }

  return {
    enabled: true,
    operator: 'And',
    matchType: matchTypeMap[filterRule.matchType] || filterRule.matchType,
    relationship: filterRule.action === 'exclude'
      ? 'does_not_match'
      : relationshipMap[filterRule.relationship] || filterRule.relationship,
    condition: filterRule.condition,
  }
}
