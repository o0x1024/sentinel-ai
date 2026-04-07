import {
  createIntruderDefaultDictionaryReference,
  isIntruderDefaultDictionaryReference,
  parseIntruderDefaultDictionaryReference,
  type IntruderDictionarySummary,
} from './intruderDictionaries'

export interface IntruderStructuredDictionarySource {
  type: 'dictionary' | 'default_dictionary'
  dictionaryId?: string
  dictionaryName?: string
  dictType?: string
}

export function decodeIntruderStructuredDictionarySources(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.flatMap((item) => {
    if (!item || typeof item !== 'object') {
      return []
    }

    const source = item as IntruderStructuredDictionarySource
    if (source.type === 'dictionary' && typeof source.dictionaryId === 'string' && source.dictionaryId.trim()) {
      return [source.dictionaryId.trim()]
    }

    if (source.type === 'default_dictionary' && typeof source.dictType === 'string' && source.dictType.trim()) {
      return [createIntruderDefaultDictionaryReference(source.dictType.trim())]
    }

    return []
  })
}

export function encodeIntruderStructuredDictionarySources(
  values: string[],
  dictionarySummaryById: Record<string, IntruderDictionarySummary> = {},
): IntruderStructuredDictionarySource[] {
  return values
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => {
      const defaultDictType = parseIntruderDefaultDictionaryReference(item)
      if (defaultDictType) {
        return {
          type: 'default_dictionary' as const,
          dictType: defaultDictType,
        }
      }

      const dictionary = dictionarySummaryById[item]
      return {
        type: 'dictionary' as const,
        dictionaryId: item,
        dictionaryName: dictionary?.name || undefined,
      }
    })
}

export function isIntruderStructuredDictionarySourceArray(value: unknown): boolean {
  return Array.isArray(value) && value.every((item) => {
    if (!item || typeof item !== 'object') {
      return false
    }

    const source = item as IntruderStructuredDictionarySource
    if (source.type === 'dictionary') {
      return typeof source.dictionaryId === 'string' && source.dictionaryId.trim().length > 0
    }
    if (source.type === 'default_dictionary') {
      return typeof source.dictType === 'string' && source.dictType.trim().length > 0
    }
    return false
  })
}

export function normalizeIntruderDictionaryReferenceValues(values: string[]): string[] {
  return Array.from(
    new Set(
      values
        .map((item) => item.trim())
        .filter((item) => item && (isIntruderDefaultDictionaryReference(item) || item.length > 0)),
    ),
  )
}
