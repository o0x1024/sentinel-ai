export type MonitorSeedBinding = {
  seed_type: string
  input_key: string
}

export type SeedBindingInputKeyOption = {
  value: string
  label: string
  typeLabel: string
  description: string
}

export const MONITOR_SEED_TYPE_OPTIONS = [
  'root_domain',
  'domain',
  'favicon_hash',
  'brand_keyword',
  'org_name',
  'asn',
  'cname_keyword',
  'title_keyword',
  'body_keyword',
  'header_keyword',
]

const SEED_BINDING_EXCLUDED_INPUT_KEYS = new Set([
  'targets',
  'target_objects',
  'service_targets',
  'previousSnapshots',
  '__monitorExecution',
  'monitorProgress',
  'domain',
  'domains',
  'url',
  'urls',
  'dictionary',
  'dictionary_id',
  'serviceProbeEngine',
])

const SEED_BINDING_SECRET_PATTERNS = [
  /token/i,
  /secret/i,
  /password/i,
  /passwd/i,
  /pass$/i,
  /api[_-]?key/i,
  /[_-]key$/i,
]

const toCompactText = (value: unknown) =>
  String(value || '')
    .trim()
    .toLowerCase()

export const normalizeSeedBindings = (value: unknown): MonitorSeedBinding[] => {
  if (!Array.isArray(value)) {
    return []
  }

  const dedupe = new Set<string>()
  const bindings: MonitorSeedBinding[] = []

  for (const item of value) {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      continue
    }

    const seedType = toCompactText((item as Record<string, unknown>).seed_type)
    const inputKey = String((item as Record<string, unknown>).input_key || '').trim()
    if (!seedType || !inputKey) {
      continue
    }

    const key = `${seedType}::${inputKey}`
    if (dedupe.has(key)) {
      continue
    }

    dedupe.add(key)
    bindings.push({
      seed_type: seedType,
      input_key: inputKey,
    })
  }

  return bindings
}

export const stringifySeedBindings = (bindings: unknown) =>
  JSON.stringify(normalizeSeedBindings(bindings), null, 2)

export const parseSeedBindingsText = (text: string) => {
  const raw = text.trim()
  if (!raw) {
    return { bindings: [] as MonitorSeedBinding[], error: '' }
  }

  try {
    const parsed = JSON.parse(raw)
    return {
      bindings: normalizeSeedBindings(parsed),
      error: '',
    }
  } catch (error) {
    return {
      bindings: [] as MonitorSeedBinding[],
      error: error instanceof Error ? error.message : 'Invalid JSON',
    }
  }
}

export const humanizeSeedType = (value: string) =>
  String(value || '')
    .trim()
    .replace(/_/g, ' ')

export const formatSeedBindingLabel = (binding: MonitorSeedBinding) =>
  `${humanizeSeedType(binding.seed_type)} -> ${binding.input_key}`

const buildSchemaTypeLabel = (property: Record<string, any>) => {
  if (property.type === 'array' && property.items?.type) {
    return `array<${String(property.items.type)}>`
  }
  return String(property.type || 'json')
}

const isSecretLikeInputKey = (value: string) =>
  SEED_BINDING_SECRET_PATTERNS.some(pattern => pattern.test(value))

const isBindableSeedInputProperty = (name: string, property: unknown) => {
  if (!name || SEED_BINDING_EXCLUDED_INPUT_KEYS.has(name) || isSecretLikeInputKey(name)) {
    return false
  }

  if (!property || typeof property !== 'object' || Array.isArray(property)) {
    return false
  }

  const schema = property as Record<string, any>
  if (schema.type === 'string') {
    return true
  }

  return schema.type === 'array' && schema.items?.type === 'string' && !schema.items?.properties
}

export const extractSeedBindingInputKeyOptions = (schema: unknown): SeedBindingInputKeyOption[] => {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    return []
  }

  const properties = (schema as Record<string, any>).properties
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) {
    return []
  }

  return Object.entries(properties)
    .filter(([name, property]) => isBindableSeedInputProperty(name, property))
    .map(([name, property]) => {
      const details = property as Record<string, any>
      const typeLabel = buildSchemaTypeLabel(details)
      const description = String(details.description || '').trim()
      return {
        value: name,
        label: `${name} (${typeLabel})`,
        typeLabel,
        description,
      }
    })
}
