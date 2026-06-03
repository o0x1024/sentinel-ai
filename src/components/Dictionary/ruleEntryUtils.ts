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

  if (dictionaryType === 'fingerprint_rule' || dictionaryType === 'service_probe_rule') {
    const matcherCount = Array.isArray(metadata.matchers) ? metadata.matchers.length : 0
    const service = typeof metadata.service === 'string' ? metadata.service : ''
    const probeName = typeof metadata.probeName === 'string'
      ? metadata.probeName
      : typeof metadata.probe_name === 'string'
        ? metadata.probe_name
        : ''
    const summaryParts = [metadata.name || metadata.product || word.word]
    if (service) summaryParts.push(service)
    if (probeName) summaryParts.push(probeName)
    if (matcherCount > 0) summaryParts.push(`${matcherCount} matcher`)
    return summaryParts.join(' · ')
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

export function getRuleService(word: RuleWordLike): string {
  const metadata = parseRuleMetadata(word)
  return typeof metadata.service === 'string' ? metadata.service : ''
}

export function getRuleProbeName(word: RuleWordLike): string {
  const metadata = parseRuleMetadata(word)
  if (typeof metadata.probeName === 'string') return metadata.probeName
  return typeof metadata.probe_name === 'string' ? metadata.probe_name : ''
}
