type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue }

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function formatJsonValue(value: JsonValue, depth: number): string {
  if (Array.isArray(value)) {
    if (value.length === 0) return '[]'
    const lines = value.map((item) => `${indent(depth + 1)}${formatJsonValue(item, depth + 1)}`)
    return `[\n${lines.join(',\n')}\n${indent(depth)}]`
  }

  if (value && typeof value === 'object') {
    const entries = Object.entries(value)
    if (entries.length === 0) return '{}'
    const lines = entries.map(
      ([key, item]) => `${indent(depth + 1)}${JSON.stringify(key)}:${formatJsonValue(item, depth + 1)}`,
    )
    return `{\n${lines.join(',\n')}\n${indent(depth)}}`
  }

  return JSON.stringify(value) ?? 'null'
}

export function formatTrafficJsonBody(body: string): string {
  if (!body) return ''

  const trimmed = body.trim()
  if (!trimmed) return body

  try {
    return formatJsonValue(JSON.parse(trimmed) as JsonValue, 0)
  } catch {
    return body
  }
}
