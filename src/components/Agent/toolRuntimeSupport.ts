export interface ToolRuntimeMetadata {
  executionEnvironment: string
  workingDir: string
  containerRef: string | null
}

const MAX_PARSE_DEPTH = 4

const normalizeRuntimeCandidate = (value: unknown): ToolRuntimeMetadata | null => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null

  const record = value as Record<string, unknown>
  const executionEnvironment =
    typeof record.execution_environment === 'string' ? record.execution_environment.trim() : ''
  const workingDir = typeof record.working_dir === 'string' ? record.working_dir.trim() : ''
  const containerRef =
    typeof record.container_ref === 'string' && record.container_ref.trim().length > 0
      ? record.container_ref.trim()
      : null

  if (!executionEnvironment || !workingDir) return null

  return {
    executionEnvironment,
    workingDir,
    containerRef,
  }
}

const parseRuntimePayload = (value: unknown, depth = 0): ToolRuntimeMetadata | null => {
  if (depth > MAX_PARSE_DEPTH || value === null || value === undefined) return null

  const direct = normalizeRuntimeCandidate(value)
  if (direct) return direct

  if (typeof value === 'string') {
    try {
      return parseRuntimePayload(JSON.parse(value), depth + 1)
    } catch {
      return null
    }
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      const parsed = parseRuntimePayload(item, depth + 1)
      if (parsed) return parsed
    }
    return null
  }

  if (typeof value !== 'object') return null

  const record = value as Record<string, unknown>
  if ('runtime' in record) {
    const parsedRuntime = parseRuntimePayload(record.runtime, depth + 1)
    if (parsedRuntime) return parsedRuntime
  }

  if (typeof record.text === 'string') {
    const parsedText = parseRuntimePayload(record.text, depth + 1)
    if (parsedText) return parsedText
  }

  return null
}

export const extractToolRuntimeMetadata = (value: unknown): ToolRuntimeMetadata | null => {
  return parseRuntimePayload(value)
}
