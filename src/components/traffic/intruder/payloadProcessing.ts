import { createIntruderId } from './http'
import type {
  IntruderPayloadProcessingCodec,
  IntruderPayloadProcessingConditionType,
  IntruderPayloadProcessingHashAlgorithm,
  IntruderPayloadProcessingRawPayloadPlacement,
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
    substringStart: 0,
    substringLength: null,
    codecType: 'url',
    hashAlgorithm: 'sha256',
    rawPayloadPlacement: 'after',
  }
}

export interface IntruderPayloadProcessingContext {
  originalPayload: string
  baseValue: string
}

export async function applyPayloadProcessingRules(
  input: string,
  rules: IntruderPayloadProcessingRule[],
  context?: Partial<IntruderPayloadProcessingContext>,
): Promise<string | null> {
  let currentValue = input
  const resolvedContext: IntruderPayloadProcessingContext = {
    originalPayload: context?.originalPayload ?? input,
    baseValue: context?.baseValue ?? '',
  }

  for (const rule of rules) {
    if (!rule.enabled) continue
    if (!matchesPayloadRuleCondition(currentValue, rule)) continue

    switch (rule.type) {
      case 'prefix':
        currentValue = `${rule.replaceValue}${currentValue}`
        break
      case 'suffix':
        currentValue = `${currentValue}${rule.replaceValue}`
        break
      case 'replace':
        currentValue = currentValue.split(rule.matchValue).join(rule.replaceValue)
        break
      case 'replaceRegex':
        try {
          currentValue = currentValue.replace(new RegExp(rule.matchValue, rule.caseSensitive ? 'g' : 'gi'), rule.replaceValue)
        } catch {
          // Ignore invalid regular expressions and keep the current value.
        }
        break
      case 'substring':
        currentValue = applySubstringRule(currentValue, rule)
        break
      case 'reverseSubstring':
        currentValue = applyReverseSubstringRule(currentValue, rule)
        break
      case 'lowercase':
        currentValue = currentValue.toLowerCase()
        break
      case 'uppercase':
        currentValue = currentValue.toUpperCase()
        break
      case 'trim':
        currentValue = currentValue.trim()
        break
      case 'base64':
        currentValue = btoa(unescape(encodeURIComponent(currentValue)))
        break
      case 'urlEncode':
        currentValue = encodeURIComponent(currentValue)
        break
      case 'decode':
        currentValue = decodeValue(currentValue, rule.codecType ?? 'url')
        break
      case 'hash':
        currentValue = await hashValue(currentValue, rule.hashAlgorithm ?? 'sha256')
        break
      case 'addRawPayload':
        currentValue = appendRawPayload(currentValue, resolvedContext.originalPayload, rule.rawPayloadPlacement ?? 'after')
        break
      case 'replaceBaseValue':
        currentValue = currentValue.split('{base}').join(resolvedContext.baseValue)
        break
      case 'reverse':
        currentValue = currentValue.split('').reverse().join('')
        break
      case 'removeWhitespace':
        currentValue = currentValue.replace(/\s+/g, '')
        break
      case 'repeat':
        currentValue = currentValue.repeat(Math.max(1, Number.parseInt(rule.replaceValue || '1', 10) || 1))
        break
      case 'hexEncode':
        currentValue = Array.from(currentValue)
          .map((char) => char.charCodeAt(0).toString(16).padStart(2, '0'))
          .join('')
        break
      case 'skipRegex':
        if (matchesRegex(currentValue, rule.matchValue, rule.caseSensitive ?? false)) {
          return null
        }
        break
      default:
        break
    }
  }

  return currentValue
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
      return matchesRegex(input, conditionValue, rule.caseSensitive ?? false)
    default:
      return true
  }
}

function matchesRegex(input: string, pattern: string, caseSensitive: boolean): boolean {
  try {
    return new RegExp(pattern, caseSensitive ? '' : 'i').test(input)
  } catch {
    return false
  }
}

function applySubstringRule(value: string, rule: IntruderPayloadProcessingRule): string {
  const start = Math.max(0, Number.isFinite(rule.substringStart) ? Number(rule.substringStart) : 0)
  if (rule.substringLength == null) {
    return value.slice(start)
  }

  const length = Math.max(0, Number(rule.substringLength) || 0)
  return value.slice(start, start + length)
}

function applyReverseSubstringRule(value: string, rule: IntruderPayloadProcessingRule): string {
  const endOffset = Math.max(0, Number.isFinite(rule.substringStart) ? Number(rule.substringStart) : 0)
  const end = Math.max(0, value.length - endOffset)
  if (rule.substringLength == null) {
    return value.slice(0, end)
  }

  const length = Math.max(0, Number(rule.substringLength) || 0)
  const start = Math.max(0, end - length)
  return value.slice(start, end)
}

function decodeValue(value: string, codec: IntruderPayloadProcessingCodec): string {
  try {
    switch (codec) {
      case 'base64': {
        const binary = atob(value)
        const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0))
        return new TextDecoder().decode(bytes)
      }
      case 'hex': {
        const normalized = value.replace(/\s+/g, '')
        if (!normalized.length || normalized.length % 2 !== 0 || /[^0-9a-f]/i.test(normalized)) {
          return value
        }

        const bytes = new Uint8Array(normalized.length / 2)
        for (let index = 0; index < normalized.length; index += 2) {
          bytes[index / 2] = Number.parseInt(normalized.slice(index, index + 2), 16)
        }
        return new TextDecoder().decode(bytes)
      }
      case 'url':
      default:
        return decodeURIComponent(value)
    }
  } catch {
    return value
  }
}

async function hashValue(value: string, algorithm: IntruderPayloadProcessingHashAlgorithm): Promise<string> {
  const normalized = algorithm === 'sha1' ? 'SHA-1' : 'SHA-256'
  const data = new TextEncoder().encode(value)
  const hashBuffer = await crypto.subtle.digest(normalized, data)
  return Array.from(new Uint8Array(hashBuffer))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')
}

function appendRawPayload(
  currentValue: string,
  originalPayload: string,
  placement: IntruderPayloadProcessingRawPayloadPlacement,
): string {
  return placement === 'before'
    ? `${originalPayload}${currentValue}`
    : `${currentValue}${originalPayload}`
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
    case 'substring':
      return 'Substring'
    case 'reverseSubstring':
      return 'Reverse substring'
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
    case 'decode':
      return 'Decode'
    case 'hash':
      return 'Hash'
    case 'addRawPayload':
      return 'Add raw payload'
    case 'replaceBaseValue':
      return 'Replace placeholder with base value'
    case 'reverse':
      return 'Reverse'
    case 'removeWhitespace':
      return 'Remove whitespace'
    case 'repeat':
      return 'Repeat'
    case 'hexEncode':
      return 'Hex encode'
    case 'skipRegex':
      return 'Skip if regex matches'
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

export function getPayloadProcessingCodecLabel(codec: IntruderPayloadProcessingCodec): string {
  switch (codec) {
    case 'base64':
      return 'Base64'
    case 'hex':
      return 'Hex'
    case 'url':
    default:
      return 'URL'
  }
}

export function getPayloadProcessingHashLabel(algorithm: IntruderPayloadProcessingHashAlgorithm): string {
  switch (algorithm) {
    case 'sha1':
      return 'SHA-1'
    case 'sha256':
    default:
      return 'SHA-256'
  }
}

export function getPayloadProcessingRawPayloadPlacementLabel(placement: IntruderPayloadProcessingRawPayloadPlacement): string {
  switch (placement) {
    case 'before':
      return 'Before processed payload'
    case 'after':
    default:
      return 'After processed payload'
  }
}
