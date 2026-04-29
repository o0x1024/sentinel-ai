import type { HttpHeaderEntry } from './model'

export function cloneHeaderEntries(headers: HttpHeaderEntry[]): HttpHeaderEntry[] {
  return headers.map((header) => ({ ...header }))
}

export function normalizeHeaderEntries(headers: HttpHeaderEntry[]): HttpHeaderEntry[] {
  return headers
    .filter((header) => header.name.trim().length > 0)
    .map((header) => ({
      name: header.name.trim(),
      value: header.value,
    }))
}

export function findHeaderValue(headers: HttpHeaderEntry[], name: string): string | undefined {
  const target = name.trim().toLowerCase()
  return headers.find((header) => header.name.trim().toLowerCase() === target)?.value
}

export function filterHeaders(headers: HttpHeaderEntry[], name: string): HttpHeaderEntry[] {
  const target = name.trim().toLowerCase()
  return headers.filter((header) => header.name.trim().toLowerCase() !== target)
}

export function replaceOrAppendHeader(
  headers: HttpHeaderEntry[],
  name: string,
  value: string,
): HttpHeaderEntry[] {
  const target = name.trim().toLowerCase()
  const nextHeaders = cloneHeaderEntries(headers)
  const existingIndex = nextHeaders.findIndex((header) => header.name.trim().toLowerCase() === target)
  if (existingIndex >= 0) {
    nextHeaders[existingIndex] = {
      name: nextHeaders[existingIndex].name,
      value,
    }
    return nextHeaders
  }

  nextHeaders.push({ name, value })
  return nextHeaders
}

export function headerEntriesToRecord(headers: HttpHeaderEntry[]): Record<string, string> {
  return headers.reduce<Record<string, string>>((record, header) => {
    const existing = record[header.name]
    record[header.name] = existing ? `${existing}, ${header.value}` : header.value
    return record
  }, {})
}

export function parseStoredHeaderEntries(rawHeaders?: string): HttpHeaderEntry[] {
  if (!rawHeaders) return []

  try {
    const parsed = JSON.parse(rawHeaders)
    if (Array.isArray(parsed)) {
      return normalizeHeaderEntries(
        parsed
          .filter((entry) => entry && typeof entry === 'object')
          .map((entry) => ({
            name: String(entry.name ?? ''),
            value: String(entry.value ?? ''),
          })),
      )
    }

    if (parsed && typeof parsed === 'object') {
      return normalizeHeaderEntries(
        Object.entries(parsed).map(([name, value]) => ({
          name,
          value: String(value ?? ''),
        })),
      )
    }
  } catch {
    const parsedHeaders: HttpHeaderEntry[] = []

    for (const line of rawHeaders.split(/\r\n|\r|\n/).map((entry) => entry.trim()).filter((entry) => entry.length > 0)) {
      const separatorIndex = line.indexOf(':')
      if (separatorIndex <= 0) {
        const previousHeader = parsedHeaders[parsedHeaders.length - 1]
        if (previousHeader) {
          previousHeader.value = `${previousHeader.value}\n${line}`
        }
        continue
      }

      parsedHeaders.push({
        name: line.slice(0, separatorIndex).trim(),
        value: line.slice(separatorIndex + 1).trim(),
      })
    }

    return parsedHeaders
  }

  return []
}

export function serializeHeaderEntries(headers: HttpHeaderEntry[]): string {
  return JSON.stringify(normalizeHeaderEntries(headers))
}
