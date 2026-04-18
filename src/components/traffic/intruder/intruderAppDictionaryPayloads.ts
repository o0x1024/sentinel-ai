import {
  getIntruderDefaultDictionaryId,
  listIntruderDictionaryWords,
} from './intruderDictionaries'
import type {
  IntruderDictionaryPayloadConfig,
  IntruderDictionarySource,
} from './types'

const DEFAULT_DICTIONARY_PAYLOAD_LIMIT = 200

export function createDefaultIntruderDictionaryPayloadConfig(): IntruderDictionaryPayloadConfig {
  return {
    sources: [],
    limit: DEFAULT_DICTIONARY_PAYLOAD_LIMIT,
    deduplicate: true,
  }
}

export function normalizeIntruderDictionarySource(
  value?: Partial<IntruderDictionarySource> | null,
): IntruderDictionarySource | null {
  if (!value) {
    return null
  }

  if (value.type === 'default_dictionary') {
    const dictType = String(value.dictType || '').trim()
    if (!dictType) {
      return null
    }

    return {
      type: 'default_dictionary',
      dictType,
    }
  }

  const dictionaryId = String(value.dictionaryId || '').trim()
  if (!dictionaryId) {
    return null
  }

  const dictionaryName = String(value.dictionaryName || '').trim()
  return {
    type: 'dictionary',
    dictionaryId,
    dictionaryName: dictionaryName || undefined,
  }
}

export function normalizeIntruderDictionaryPayloadConfig(
  value?: Partial<IntruderDictionaryPayloadConfig> | null,
): IntruderDictionaryPayloadConfig {
  const defaults = createDefaultIntruderDictionaryPayloadConfig()
  const normalizedSources = Array.isArray(value?.sources)
    ? value.sources
      .map((source) => normalizeIntruderDictionarySource(source))
      .filter((source): source is IntruderDictionarySource => Boolean(source))
    : defaults.sources

  return {
    sources: deduplicateIntruderDictionarySources(normalizedSources),
    limit: Math.max(1, Number(value?.limit) || defaults.limit),
    deduplicate: value?.deduplicate !== false,
  }
}

export function deduplicateIntruderDictionarySources(
  sources: IntruderDictionarySource[],
): IntruderDictionarySource[] {
  const next: IntruderDictionarySource[] = []
  const seen = new Set<string>()

  for (const source of sources) {
    const normalized = normalizeIntruderDictionarySource(source)
    if (!normalized) {
      continue
    }

    const key = normalized.type === 'default_dictionary'
      ? `default:${normalized.dictType}`
      : `dictionary:${normalized.dictionaryId}`

    if (seen.has(key)) {
      continue
    }

    seen.add(key)
    next.push(normalized)
  }

  return next
}

export async function resolveIntruderDictionaryPayloads(
  config: IntruderDictionaryPayloadConfig,
): Promise<string[]> {
  const normalizedConfig = normalizeIntruderDictionaryPayloadConfig(config)
  if (!normalizedConfig.sources.length) {
    return []
  }

  const values = (
    await Promise.all(
      normalizedConfig.sources.map((source) => resolveIntruderDictionarySourceWords(source, normalizedConfig.limit)),
    )
  ).flat()

  const merged = normalizedConfig.deduplicate
    ? Array.from(new Set(values))
    : values

  return merged.slice(0, normalizedConfig.limit)
}

async function resolveIntruderDictionarySourceWords(
  source: IntruderDictionarySource,
  limit: number,
): Promise<string[]> {
  if (source.type === 'default_dictionary') {
    const dictType = source.dictType?.trim() || ''
    if (!dictType) {
      return []
    }

    const dictionaryId = await getIntruderDefaultDictionaryId(dictType)
    if (!dictionaryId) {
      throw new Error(`No default dictionary configured for type ${dictType}`)
    }

    return listIntruderDictionaryWords(dictionaryId, limit)
  }

  const dictionaryId = source.dictionaryId?.trim() || ''
  if (!dictionaryId) {
    return []
  }

  return listIntruderDictionaryWords(dictionaryId, limit)
}
