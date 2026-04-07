/**
 * Example Intruder payload generator plugin.
 * @plugin intruder_dictionary_payload_generator
 * @name Intruder Dictionary Payload Generator
 * @main_category intruder
 * @category payload_generator
 *
 * Import this file into Plugin Management, then set:
 * - main category: intruder
 * - category: payload_generator
 */

interface ToolInput {
  positions?: Array<{
    index?: number
    value?: string
  }>
  options?: {
    limit?: number
  }
  config?: {
    baseValues?: string[]
    dictionaryIds?: string[]
    dictionarySources?: Array<{
      type?: 'dictionary' | 'default_dictionary'
      dictionaryId?: string
      dictType?: string
      dictionaryName?: string
    }>
    prefixes?: string[]
    suffixes?: string[]
    includePositionValues?: boolean
    includeOriginalValues?: boolean
    dedupe?: boolean
    limit?: number
    dictionaryWordLimit?: number
  }
}

interface NormalizedDictionarySource {
  type: 'dictionary' | 'default_dictionary'
  dictionaryId?: string
  dictType?: string
}

interface ToolOutput {
  success: boolean
  data?: {
    payloads?: string[]
  }
  error?: string
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        properties: {
          baseValues: {
            type: 'array',
            items: { type: 'string' },
            description: 'Base payload values to expand',
            'x-dictionary': {
              mode: 'insert',
              allowInsert: true,
              allowReference: false,
              allowDefault: true,
              dictTypes: ['username', 'password', 'path', 'sql_injection_payload', 'xss_payload', 'custom'],
              maxWords: 500,
            },
            'x-ui-group': {
              key: 'base_values',
              label: 'Base Values',
              description: 'Choose the values used as the starting point for payload generation.',
            },
          },
          dictionarySources: {
            type: 'array',
            items: {
              type: 'object',
              properties: {
                type: { type: 'string' },
                dictionaryId: { type: 'string' },
                dictType: { type: 'string' },
                dictionaryName: { type: 'string' },
              },
            },
            description: 'Referenced dictionaries to merge into base payload values at runtime',
            'x-dictionary': {
              mode: 'reference',
              allowInsert: false,
              allowReference: true,
              allowDefault: true,
              storeAs: 'structuredSources',
              aliasFrom: 'dictionaryIds',
              dictTypes: ['username', 'password', 'path', 'sql_injection_payload', 'xss_payload', 'custom'],
            },
            'x-ui-group': 'base_values',
          },
          prefixes: {
            type: 'array',
            items: { type: 'string' },
            description: 'Prefixes to prepend to every base value',
            'x-ui-group': {
              key: 'expansion',
              label: 'Expansion',
              description: 'Generate derived payloads by combining prefixes and suffixes.',
            },
          },
          suffixes: {
            type: 'array',
            items: { type: 'string' },
            description: 'Suffixes to append to every base value',
            'x-ui-group': 'expansion',
          },
          includePositionValues: {
            type: 'boolean',
            default: true,
            description: 'Include current Intruder position values as base values',
            'x-ui-group': 'base_values',
          },
          includeOriginalValues: {
            type: 'boolean',
            default: true,
            description: 'Keep the original base values in the final output',
            'x-ui-group': {
              key: 'output',
              label: 'Output',
              description: 'Control how the final payload list is returned.',
            },
          },
          dedupe: {
            type: 'boolean',
            default: true,
            description: 'Remove duplicate payloads',
            'x-ui-group': 'output',
          },
          limit: {
            type: 'number',
            default: 200,
            description: 'Maximum number of generated payloads',
            'x-ui-group': 'output',
          },
          dictionaryWordLimit: {
            type: 'number',
            default: 200,
            description: 'Maximum number of words loaded from each referenced dictionary',
            'x-ui-group': 'output',
          },
        },
      },
    },
  }
}

function normalizeStringList(value: unknown) {
  if (!Array.isArray(value)) {
    return []
  }
  return value
    .filter((item): item is string => typeof item === 'string')
    .map((item) => item.trim())
    .filter(Boolean)
}

function normalizeDictionarySources(value: unknown): NormalizedDictionarySource[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.flatMap((item) => {
    if (!item || typeof item !== 'object') {
      return []
    }

    const source = item as Record<string, unknown>
    if (source.type === 'dictionary' && typeof source.dictionaryId === 'string' && source.dictionaryId.trim()) {
      return [{
        type: 'dictionary' as const,
        dictionaryId: source.dictionaryId.trim(),
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

function normalizeLegacyDictionaryIds(value: unknown): NormalizedDictionarySource[] {
  return normalizeStringList(value).map((item) => (
    item.startsWith('default:')
      ? { type: 'default_dictionary' as const, dictType: item.slice('default:'.length).trim() }
      : { type: 'dictionary' as const, dictionaryId: item }
  ))
}

function normalizeConfig(config?: ToolInput['config']) {
  const dictionarySources = normalizeDictionarySources(config?.dictionarySources)
  const legacyDictionarySources = dictionarySources.length === 0
    ? normalizeLegacyDictionaryIds(config?.dictionaryIds)
    : []

  return {
    baseValues: normalizeStringList(config?.baseValues),
    dictionarySources: [...dictionarySources, ...legacyDictionarySources],
    prefixes: normalizeStringList(config?.prefixes),
    suffixes: normalizeStringList(config?.suffixes),
    includePositionValues: config?.includePositionValues !== false,
    includeOriginalValues: config?.includeOriginalValues !== false,
    dedupe: config?.dedupe !== false,
    limit: Math.max(1, Number(config?.limit ?? 200)),
    dictionaryWordLimit: Math.max(1, Number(config?.dictionaryWordLimit ?? 200)),
  }
}

function collectPositionValues(positions?: ToolInput['positions']) {
  if (!Array.isArray(positions)) {
    return []
  }
  return positions
    .map((position) => (typeof position?.value === 'string' ? position.value.trim() : ''))
    .filter(Boolean)
}

function unique(values: string[]) {
  return Array.from(new Set(values))
}

async function loadDictionaryWords(dictionarySources: NormalizedDictionarySource[], perDictionaryLimit: number) {
  const dictionaryApi = globalThis.Sentinel?.Dictionary
  if (!dictionaryApi || dictionarySources.length === 0) {
    return []
  }

  const words = await Promise.all(
    dictionarySources.map(async (source) => {
      try {
        const resolvedDictionaryId = source.type === 'default_dictionary'
          ? await dictionaryApi.getDefaultId(source.dictType || '')
          : source.dictionaryId
        if (!resolvedDictionaryId) {
          return []
        }

        const result = await dictionaryApi.getWords(resolvedDictionaryId, perDictionaryLimit)
        return normalizeStringList(result)
      } catch {
        return []
      }
    }),
  )

  return words.flat()
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    const config = normalizeConfig(input?.config)
    const limit = Math.max(1, Number(input?.options?.limit ?? config.limit))
    const positionValues = config.includePositionValues ? collectPositionValues(input?.positions) : []
    const dictionaryValues = await loadDictionaryWords(config.dictionarySources, config.dictionaryWordLimit)
    const baseValues = [...config.baseValues, ...dictionaryValues, ...positionValues]
    const normalizedBaseValues = config.dedupe ? unique(baseValues) : baseValues

    const prefixes = config.prefixes.length > 0 ? config.prefixes : ['']
    const suffixes = config.suffixes.length > 0 ? config.suffixes : ['']
    const payloads: string[] = []

    if (config.includeOriginalValues) {
      payloads.push(...normalizedBaseValues)
    }

    normalizedBaseValues.forEach((baseValue) => {
      prefixes.forEach((prefix) => {
        suffixes.forEach((suffix) => {
          payloads.push(`${prefix}${baseValue}${suffix}`)
        })
      })
    })

    const finalPayloads = config.dedupe ? unique(payloads) : payloads

    return {
      success: true,
      data: {
        payloads: finalPayloads.slice(0, limit),
      },
    }
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

globalThis.get_input_schema = get_input_schema
globalThis.analyze = analyze
