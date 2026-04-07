export interface ContextExtractionEntry {
  configuredKey: string
  matchedKey: string
  source: string
}

export interface ContextExtractionSummary {
  principalMatches: ContextExtractionEntry[]
  resourceMatches: ContextExtractionEntry[]
  authMatches: ContextExtractionEntry[]
  actionKind: string
  actionSource: string
  matchedAlias: string | null
}

export interface SemanticCandidateSummary {
  field: string
  source: string
  confidence: string
  reason: string
}

export interface SemanticActionSummary {
  kind: string
  source: string
  confidence: string
  reason: string
}

export interface SemanticAbstractionSummary {
  source: string
  signature: string | null
  principalCandidates: SemanticCandidateSummary[]
  resourceCandidates: SemanticCandidateSummary[]
  credentialCandidates: SemanticCandidateSummary[]
  stateCandidates: SemanticCandidateSummary[]
  actionCandidates: SemanticActionSummary[]
  notes: string[]
}

export function getContextExtractionSummaryFromPayload(
  payload: unknown
): ContextExtractionSummary | null {
  const extraction = asRecord(asRecord(payload)?.contextExtraction)
  if (!extraction) return null

  const actionInference = asRecord(extraction.actionInference)
  const principalMatches = parseContextExtractionEntries(extraction.principalMatches)
  const resourceMatches = parseContextExtractionEntries(extraction.resourceMatches)
  const authMatches = [
    ...parseContextExtractionEntries(extraction.authHeaderMatches),
    ...parseContextExtractionEntries(extraction.authTokenMatches),
    ...parseContextExtractionEntries(extraction.cookieMatches),
  ]

  const actionKind = typeof actionInference?.kind === 'string' ? actionInference.kind : ''
  const actionSource = typeof actionInference?.source === 'string' ? actionInference.source : ''
  const matchedAlias =
    typeof actionInference?.matchedAlias === 'string' ? actionInference.matchedAlias : null

  if (!principalMatches.length && !resourceMatches.length && !authMatches.length && !actionKind) {
    return null
  }

  return {
    principalMatches,
    resourceMatches,
    authMatches,
    actionKind: actionKind || '-',
    actionSource: actionSource || 'unknown',
    matchedAlias,
  }
}

export function getSemanticAbstractionSummaryFromPayload(
  payload: unknown
): SemanticAbstractionSummary | null {
  const abstraction = asRecord(asRecord(payload)?.semanticAbstraction)
  if (!abstraction) return null

  const principalCandidates = parseSemanticCandidates(abstraction.principalCandidates)
  const resourceCandidates = parseSemanticCandidates(abstraction.resourceCandidates)
  const credentialCandidates = parseSemanticCandidates(abstraction.credentialCandidates)
  const stateCandidates = parseSemanticCandidates(abstraction.stateCandidates)
  const actionCandidates = parseSemanticActionCandidates(abstraction.actionCandidates)
  const notes = parseStringArray(abstraction.notes)
  const source = typeof abstraction.source === 'string' ? abstraction.source : 'unknown'
  const signature = typeof abstraction.signature === 'string' ? abstraction.signature : null

  if (
    !principalCandidates.length &&
    !resourceCandidates.length &&
    !credentialCandidates.length &&
    !stateCandidates.length &&
    !actionCandidates.length &&
    !notes.length
  ) {
    return null
  }

  return {
    source,
    signature,
    principalCandidates,
    resourceCandidates,
    credentialCandidates,
    stateCandidates,
    actionCandidates,
    notes,
  }
}

export function formatContextExtractionEntry(entry: ContextExtractionEntry) {
  return `${entry.configuredKey} -> ${entry.matchedKey} @ ${entry.source}`
}

export function formatSemanticCandidateEntry(entry: SemanticCandidateSummary) {
  return `${entry.field} @ ${entry.source}`
}

export function formatSemanticActionEntry(entry: SemanticActionSummary) {
  return `${entry.kind} @ ${entry.source}`
}

export function semanticConfidenceBadgeClass(confidence: string) {
  if (confidence === 'high') return 'badge-success'
  if (confidence === 'medium') return 'badge-warning'
  if (confidence === 'low') return 'badge-ghost'
  return 'badge-outline'
}

function parseContextExtractionEntries(raw: unknown): ContextExtractionEntry[] {
  if (!Array.isArray(raw)) return []
  return raw
    .map(item => {
      const record = asRecord(item)
      if (!record) return null
      const configuredKey = typeof record.configuredKey === 'string' ? record.configuredKey : ''
      const matchedKey = typeof record.matchedKey === 'string' ? record.matchedKey : ''
      const source = typeof record.source === 'string' ? record.source : ''
      if (!configuredKey || !matchedKey || !source) return null
      return {
        configuredKey,
        matchedKey,
        source,
      }
    })
    .filter((item): item is ContextExtractionEntry => !!item)
}

function parseSemanticCandidates(raw: unknown): SemanticCandidateSummary[] {
  if (!Array.isArray(raw)) return []
  return raw
    .map(item => {
      const record = asRecord(item)
      if (!record) return null
      const field = typeof record.field === 'string' ? record.field : ''
      const source = typeof record.source === 'string' ? record.source : 'unknown'
      const confidence = typeof record.confidence === 'string' ? record.confidence : 'unknown'
      const reason = typeof record.reason === 'string' ? record.reason : ''
      if (!field) return null
      return {
        field,
        source,
        confidence,
        reason,
      }
    })
    .filter((item): item is SemanticCandidateSummary => !!item)
}

function parseSemanticActionCandidates(raw: unknown): SemanticActionSummary[] {
  if (!Array.isArray(raw)) return []
  return raw
    .map(item => {
      const record = asRecord(item)
      if (!record) return null
      const kind = typeof record.kind === 'string' ? record.kind : ''
      const source = typeof record.source === 'string' ? record.source : 'unknown'
      const confidence = typeof record.confidence === 'string' ? record.confidence : 'unknown'
      const reason = typeof record.reason === 'string' ? record.reason : ''
      if (!kind) return null
      return {
        kind,
        source,
        confidence,
        reason,
      }
    })
    .filter((item): item is SemanticActionSummary => !!item)
}

function parseStringArray(raw: unknown): string[] {
  if (!Array.isArray(raw)) return []
  return raw.filter((item): item is string => typeof item === 'string' && item.length > 0)
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}
