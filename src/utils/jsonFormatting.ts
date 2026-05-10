const JSON_CODE_BLOCK_RE = /^```(?:json)?\s*([\s\S]*?)\s*```$/i
const MAX_NORMALIZE_DEPTH = 8

const isPlainObject = (value: unknown): value is Record<string, unknown> => {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

const extractJsonCandidate = (raw: unknown): string | null => {
  if (typeof raw !== 'string') return null
  const trimmed = raw.trim()
  if (!trimmed) return null

  const fencedMatch = trimmed.match(JSON_CODE_BLOCK_RE)
  if (fencedMatch?.[1]) {
    return fencedMatch[1].trim()
  }

  return trimmed
}

export const tryParseStructuredJson = (raw: unknown): unknown | null => {
  const candidate = extractJsonCandidate(raw)
  if (!candidate) return null
  if (!candidate.startsWith('{') && !candidate.startsWith('[')) return null

  try {
    return JSON.parse(candidate)
  } catch {
    return null
  }
}

const isTextEnvelope = (value: Record<string, unknown>) => {
  const text = value.text
  if (typeof text !== 'string') return false

  const type = typeof value.type === 'string' ? value.type.toLowerCase() : ''
  return ['text', 'output_text', 'input_text'].includes(type)
}

export const normalizeJsonDisplayValue = (value: unknown, depth = 0): unknown => {
  if (depth >= MAX_NORMALIZE_DEPTH) return value

  if (typeof value === 'string') {
    const parsed = tryParseStructuredJson(value)
    if (parsed === null) return value
    return normalizeJsonDisplayValue(parsed, depth + 1)
  }

  if (Array.isArray(value)) {
    const normalized = value.map((item) => normalizeJsonDisplayValue(item, depth + 1))
    if (value.length === 1 && isPlainObject(value[0]) && isTextEnvelope(value[0])) {
      return normalized[0]
    }
    return normalized
  }

  if (isPlainObject(value)) {
    if (isTextEnvelope(value)) {
      return normalizeJsonDisplayValue(value.text, depth + 1)
    }

    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, normalizeJsonDisplayValue(item, depth + 1)]),
    )
  }

  return value
}

export const formatJsonValueIfPossible = (value: unknown): string | null => {
  const normalized = normalizeJsonDisplayValue(value)
  if (typeof normalized === 'string') return null

  try {
    return JSON.stringify(normalized, null, 2)
  } catch {
    return null
  }
}

export const formatJsonStringIfPossible = (raw: unknown): string | null => {
  if (typeof raw !== 'string') return null
  return formatJsonValueIfPossible(raw)
}
