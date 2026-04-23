interface EvidenceHighlightBuckets {
  requestTerms: string[]
  responseTerms: string[]
}

export interface SecurityEvidenceSnippetFields {
  location?: string
  technique?: string
  targetPath?: string
  referenceStatus?: string
  probeStatus?: string
}

const REQUEST_TERM_KEYS = new Set(['probe', 'probe_value', 'payload', 'poc'])
const RESPONSE_TERM_KEYS = new Set(['sql_error', 'match', 'matched', 'matched_value', 'indicator'])

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function normalizeEvidenceValue(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) return ''
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"'))
    || (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1).trim()
  }
  return trimmed
}

function uniqueTerms(terms: string[]): string[] {
  return [...new Set(terms.map(term => term.trim()).filter(Boolean))]
}

function parseEvidenceSnippetFields(evidenceSnippet?: string | null): Record<string, string> {
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

function collectMatchRanges(content: string, terms: string[]) {
  const ranges: Array<{ start: number; end: number }> = []
  const lowerContent = content.toLowerCase()
  const orderedTerms = uniqueTerms(terms).sort((left, right) => right.length - left.length)

  for (const term of orderedTerms) {
    const lowerTerm = term.toLowerCase()
    let fromIndex = 0

    while (fromIndex < lowerContent.length) {
      const matchIndex = lowerContent.indexOf(lowerTerm, fromIndex)
      if (matchIndex === -1) {
        break
      }

      const nextRange = {
        start: matchIndex,
        end: matchIndex + lowerTerm.length,
      }
      const overlaps = ranges.some(range => nextRange.start < range.end && nextRange.end > range.start)
      if (!overlaps) {
        ranges.push(nextRange)
      }
      fromIndex = matchIndex + lowerTerm.length
    }
  }

  return ranges.sort((left, right) => left.start - right.start)
}

function renderHighlightedHtml(
  content: string,
  terms: string[],
  tone: 'request' | 'response',
): string {
  if (!content) return ''

  const ranges = collectMatchRanges(content, terms)
  if (ranges.length === 0) {
    return escapeHtml(content)
  }

  let cursor = 0
  let html = ''

  for (const range of ranges) {
    html += escapeHtml(content.slice(cursor, range.start))
    html += `<mark class="security-evidence-hit security-evidence-hit--${tone}">${escapeHtml(content.slice(range.start, range.end))}</mark>`
    cursor = range.end
  }

  html += escapeHtml(content.slice(cursor))
  return html
}

export function extractSecurityEvidenceHighlightBuckets(evidenceSnippet?: string | null): EvidenceHighlightBuckets {
  const requestTerms: string[] = []
  const responseTerms: string[] = []
  const fields = parseEvidenceSnippetFields(evidenceSnippet)

  for (const [key, value] of Object.entries(fields)) {
    if (REQUEST_TERM_KEYS.has(key)) {
      requestTerms.push(value)
      continue
    }

    if (RESPONSE_TERM_KEYS.has(key)) {
      responseTerms.push(value)
    }
  }

  return {
    requestTerms: uniqueTerms(requestTerms),
    responseTerms: uniqueTerms(responseTerms),
  }
}

export function extractSecurityEvidenceSnippetFields(
  evidenceSnippet?: string | null,
): SecurityEvidenceSnippetFields {
  const fields = parseEvidenceSnippetFields(evidenceSnippet)

  return {
    location: fields.location,
    technique: fields.technique,
    targetPath: fields.target_path,
    referenceStatus: fields.reference_status,
    probeStatus: fields.probe_status,
  }
}

export function buildSecurityEvidenceRequestHtml(
  rawRequest: string,
  evidenceSnippet?: string | null,
): string {
  const { requestTerms } = extractSecurityEvidenceHighlightBuckets(evidenceSnippet)
  return renderHighlightedHtml(rawRequest, requestTerms, 'request')
}

export function buildSecurityEvidenceResponseHtml(
  rawResponse: string,
  evidenceSnippet?: string | null,
): string {
  const { responseTerms } = extractSecurityEvidenceHighlightBuckets(evidenceSnippet)
  return renderHighlightedHtml(rawResponse, responseTerms, 'response')
}
