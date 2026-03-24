export interface RuleWordLike {
  word: string
  metadata?: string | null
}

export function parseRuleMetadata(word: RuleWordLike): Record<string, any> {
  if (!word.metadata) return {}
  try {
    return JSON.parse(word.metadata)
  } catch {
    return {}
  }
}

export function describeRuleEntry(word: RuleWordLike, dictionaryType?: string | null): string {
  const metadata = parseRuleMetadata(word)

  if (dictionaryType === 'sensitive_file') {
    return metadata.description || metadata.path || '敏感文件探测规则'
  }

  if (dictionaryType === 'fingerprint_rule') {
    const matcherCount = Array.isArray(metadata.matchers) ? metadata.matchers.length : 0
    return `${metadata.name || metadata.product || word.word}${matcherCount > 0 ? ` · ${matcherCount} matcher` : ''}`
  }

  if (dictionaryType === 'poc_rule') {
    const request = metadata.request || {}
    return `${metadata.name || metadata.finding_type || word.word}${request.path ? ` · ${request.method || 'GET'} ${request.path}` : ''}`
  }

  return word.word
}

export function getRuleSeverity(word: RuleWordLike): string {
  const metadata = parseRuleMetadata(word)
  return typeof metadata.severity === 'string' ? metadata.severity : ''
}

export function getRuleMatcherCount(word: RuleWordLike): number {
  const metadata = parseRuleMetadata(word)
  return Array.isArray(metadata.matchers) ? metadata.matchers.length : 0
}

export function isRuleEnabled(word: RuleWordLike): boolean {
  const metadata = parseRuleMetadata(word)
  return metadata.enabled !== false
}
