import { parseIntruderPluginConfig } from './plugins'
import type { IntruderPayloadSet } from './types'

export interface IntruderPayloadSourceDescriptor {
  type: 'dictionary' | 'default_dictionary'
  dictionaryId?: string
  dictionaryName?: string
  dictType?: string
}

export interface IntruderPayloadSourceSummaryEntry {
  payloadSetName: string
  pluginId: string
  sources: IntruderPayloadSourceDescriptor[]
}

function normalizeLegacyDictionaryIds(value: unknown): IntruderPayloadSourceDescriptor[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.flatMap<IntruderPayloadSourceDescriptor>((item): IntruderPayloadSourceDescriptor[] => {
    if (typeof item !== 'string' || !item.trim()) {
      return []
    }

    const normalizedValue = item.trim()
    if (normalizedValue.startsWith('default:')) {
      const dictType = normalizedValue.slice('default:'.length).trim()
      return dictType ? [{ type: 'default_dictionary' as const, dictType }] : []
    }

    return [{ type: 'dictionary' as const, dictionaryId: normalizedValue }]
  })
}

function normalizeStructuredDictionarySources(value: unknown): IntruderPayloadSourceDescriptor[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.flatMap<IntruderPayloadSourceDescriptor>((item): IntruderPayloadSourceDescriptor[] => {
    if (!item || typeof item !== 'object') {
      return []
    }

    const source = item as Record<string, unknown>
    if (source.type === 'dictionary' && typeof source.dictionaryId === 'string' && source.dictionaryId.trim()) {
      return [{
        type: 'dictionary' as const,
        dictionaryId: source.dictionaryId.trim(),
        dictionaryName: typeof source.dictionaryName === 'string' && source.dictionaryName.trim()
          ? source.dictionaryName.trim()
          : undefined,
      }]
    }

    if (source.type === 'default_dictionary' && typeof source.dictType === 'string' && source.dictType.trim()) {
      return [{
        type: 'default_dictionary' as const,
        dictType: source.dictType.trim(),
      }]
    }

    return []
  })
}

export function extractIntruderPayloadSourceSummaryEntries(
  payloadSets: IntruderPayloadSet[],
): IntruderPayloadSourceSummaryEntry[] {
  return payloadSets.flatMap((payloadSet) => {
    if (payloadSet.payloadType !== 'extensionGenerated' || !payloadSet.pluginId.trim()) {
      return []
    }

    try {
      const config = parseIntruderPluginConfig(payloadSet.pluginConfig)
      const structuredSources = normalizeStructuredDictionarySources(config.dictionarySources)
      const legacySources = structuredSources.length === 0
        ? normalizeLegacyDictionaryIds(config.dictionaryIds)
        : []
      const sources = [...structuredSources, ...legacySources]

      if (sources.length === 0) {
        return []
      }

      return [{
        payloadSetName: payloadSet.name,
        pluginId: payloadSet.pluginId,
        sources,
      }]
    } catch {
      return []
    }
  })
}
