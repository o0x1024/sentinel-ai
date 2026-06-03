export interface SecurityEvidenceSnippetEntry {
  key: string
  label: string
  value: string
}

const LABEL_MAP: Record<string, string> = {
  location: 'Location',
  technique: 'Technique',
  target_path: 'Target Path',
  reference_status: 'Reference Status',
  probe_status: 'Probe Status',
  probe: 'PoC',
  probe_value: 'PoC',
  payload: 'PoC',
  poc: 'PoC',
  sql_error: 'SQL Error',
  match: 'Matched Value',
  matched: 'Matched Value',
  matched_value: 'Matched Value',
  indicator: 'Indicator',
}

function normalizeEvidenceValue(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) return ''
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1).trim()
  }
  return trimmed
}

function toLabel(key: string) {
  const normalizedKey = key.trim().toLowerCase()
  const mapped = LABEL_MAP[normalizedKey]
  if (mapped) return mapped

  return normalizedKey
    .split('_')
    .filter(Boolean)
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ')
}

export function parseSecurityEvidenceSnippetFields(
  evidenceSnippet?: string | null
): Record<string, string> {
  if (!evidenceSnippet) {
    return {}
  }

  const fields: Record<string, string> = {}

  for (const segment of evidenceSnippet.split('|')) {
    const separatorIndex = segment.indexOf('=')
    if (separatorIndex === -1) {
      continue
    }

    const key = segment.slice(0, separatorIndex).trim().toLowerCase()
    const value = normalizeEvidenceValue(segment.slice(separatorIndex + 1))
    if (!key || !value) {
      continue
    }

    fields[key] = value
  }

  return fields
}

export function parseSecurityEvidenceSnippetEntries(
  evidenceSnippet?: string | null
): SecurityEvidenceSnippetEntry[] {
  const fields = parseSecurityEvidenceSnippetFields(evidenceSnippet)

  return Object.entries(fields).map(([key, value]) => ({
    key,
    label: toLabel(key),
    value,
  }))
}
