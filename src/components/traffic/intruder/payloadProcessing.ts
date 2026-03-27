import { createIntruderId } from './http'
import type {
  IntruderPayloadProcessingConditionType,
  IntruderPayloadProcessingRule,
  IntruderPayloadProcessingRuleType,
} from './types'

export function createDefaultPayloadProcessingRule(): IntruderPayloadProcessingRule {
  return {
    id: createIntruderId('payload-rule'),
    enabled: true,
    type: 'prefix',
    matchValue: '',
    replaceValue: '',
    caseSensitive: false,
    conditionType: 'always',
    conditionValue: '',
  }
}

export function applyPayloadProcessingRules(
  input: string,
  rules: IntruderPayloadProcessingRule[],
): string {
  return rules.reduce((currentValue, rule) => {
    if (!rule.enabled) return currentValue
    if (!matchesPayloadRuleCondition(currentValue, rule)) return currentValue

    switch (rule.type) {
      case 'prefix':
        return `${rule.replaceValue}${currentValue}`
      case 'suffix':
        return `${currentValue}${rule.replaceValue}`
      case 'replace':
        return currentValue.split(rule.matchValue).join(rule.replaceValue)
      case 'replaceRegex':
        try {
          return currentValue.replace(new RegExp(rule.matchValue, rule.caseSensitive ? 'g' : 'gi'), rule.replaceValue)
        } catch {
          return currentValue
        }
      case 'lowercase':
        return currentValue.toLowerCase()
      case 'uppercase':
        return currentValue.toUpperCase()
      case 'trim':
        return currentValue.trim()
      case 'base64':
        return btoa(unescape(encodeURIComponent(currentValue)))
      case 'urlEncode':
        return encodeURIComponent(currentValue)
      case 'reverse':
        return currentValue.split('').reverse().join('')
      case 'removeWhitespace':
        return currentValue.replace(/\s+/g, '')
      case 'repeat':
        return currentValue.repeat(Math.max(1, Number.parseInt(rule.replaceValue || '1', 10) || 1))
      case 'hexEncode':
        return Array.from(currentValue)
          .map((char) => char.charCodeAt(0).toString(16).padStart(2, '0'))
          .join('')
      default:
        return currentValue
    }
  }, input)
}

function matchesPayloadRuleCondition(input: string, rule: IntruderPayloadProcessingRule): boolean {
  const conditionType = rule.conditionType ?? 'always'
  const conditionValue = rule.conditionValue ?? ''

  if (conditionType === 'always' || !conditionValue) {
    return true
  }

  const source = rule.caseSensitive ? input : input.toLowerCase()
  const needle = rule.caseSensitive ? conditionValue : conditionValue.toLowerCase()

  switch (conditionType) {
    case 'contains':
      return source.includes(needle)
    case 'notContains':
      return !source.includes(needle)
    case 'regex':
      try {
        return new RegExp(conditionValue, rule.caseSensitive ? '' : 'i').test(input)
      } catch {
        return false
      }
    default:
      return true
  }
}

export function getPayloadProcessingRuleLabel(ruleType: IntruderPayloadProcessingRuleType): string {
  switch (ruleType) {
    case 'prefix':
      return 'Prefix'
    case 'suffix':
      return 'Suffix'
    case 'replace':
      return 'Replace'
    case 'replaceRegex':
      return 'Replace (regex)'
    case 'lowercase':
      return 'Lowercase'
    case 'uppercase':
      return 'Uppercase'
    case 'trim':
      return 'Trim'
    case 'base64':
      return 'Base64'
    case 'urlEncode':
      return 'URL encode'
    case 'reverse':
      return 'Reverse'
    case 'removeWhitespace':
      return 'Remove whitespace'
    case 'repeat':
      return 'Repeat'
    case 'hexEncode':
      return 'Hex encode'
    default:
      return 'Rule'
  }
}

export function getPayloadProcessingConditionLabel(conditionType: IntruderPayloadProcessingConditionType): string {
  switch (conditionType) {
    case 'contains':
      return 'If contains'
    case 'notContains':
      return 'If missing'
    case 'regex':
      return 'If regex matches'
    case 'always':
    default:
      return 'Always'
  }
}
