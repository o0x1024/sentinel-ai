import type { Evidence } from './vulnerabilityFindingTypes'
import type {
  WorkbenchBehaviorStep,
  WorkbenchBehaviorStepKind,
  WorkbenchCase,
  WorkbenchConfidence,
  WorkbenchObjectAnalysis,
  WorkbenchObjectPoolGroup,
  WorkbenchObjectReference,
  WorkbenchObjectReferenceRole,
  WorkbenchSuggestionStrategy,
  WorkbenchTestSuggestion,
} from './securityWorkbenchTypes'
import { wb } from './securityWorkbenchLocale'

const COMMON_PATH_WORDS = new Set([
  'api',
  'v1',
  'v2',
  'detail',
  'list',
  'items',
  'item',
  'view',
  'get',
  'query',
  'search',
  'update',
  'create',
  'delete',
  'remove',
  'edit',
  'save',
  'submit',
  'confirm',
  'approve',
  'export',
  'download',
  'preview',
  'batch',
  'page',
])

const normalizeField = (value: string) => value.toLowerCase().replace(/[^a-z0-9]+/g, '')

const safeUrl = (input: string) => {
  try {
    return new URL(input, 'http://sentinel.local')
  } catch {
    return null
  }
}

const parseJson = (input?: string) => {
  if (!input) return null
  const trimmed = input.trim()
  if (!trimmed || (trimmed[0] !== '{' && trimmed[0] !== '[')) return null
  try {
    return JSON.parse(trimmed) as unknown
  } catch {
    return null
  }
}

const parseForm = (input?: string) => {
  if (!input || !input.includes('=')) return []
  return input
    .split('&')
    .map(item => {
      const [key, ...rest] = item.split('=')
      if (!key) return null
      return [decodeURIComponent(key), decodeURIComponent(rest.join('='))] as const
    })
    .filter((item): item is readonly [string, string] => Boolean(item))
}

const looksLikeObjectValue = (value: string) => {
  const trimmed = value.trim()
  if (!trimmed) return false
  if (trimmed.length < 2 || trimmed.length > 128) return false
  if (/^https?:\/\//i.test(trimmed)) return false
  if (/^[a-z]{2,12}$/i.test(trimmed)) return false
  if (/^\d{2,}$/.test(trimmed)) return true
  if (/^[0-9a-f]{8,}$/i.test(trimmed)) return true
  if (/^[A-Za-z0-9_-]{4,}$/.test(trimmed) && /[\d_-]/.test(trimmed)) return true
  if (/^[A-Z]{1,8}-[\w-]{2,}$/i.test(trimmed)) return true
  return false
}

const inferRole = (fieldPath: string): WorkbenchObjectReferenceRole => {
  const normalized = normalizeField(fieldPath)
  if (normalized.includes('tenant') || normalized.includes('workspace') || normalized.includes('org')) {
    return 'tenant'
  }
  if (
    normalized.includes('owner') ||
    normalized.includes('userid') ||
    normalized.includes('member') ||
    normalized.includes('assignee')
  ) {
    return 'owner'
  }
  if (normalized.includes('operator') || normalized.includes('actor') || normalized.includes('reviewer')) {
    return 'operator'
  }
  if (
    normalized.includes('id') ||
    normalized.includes('key') ||
    normalized.includes('code') ||
    normalized.includes('ref') ||
    normalized.includes('no')
  ) {
    return 'resource'
  }
  return 'unknown'
}

const inferConfidence = (
  source: WorkbenchObjectReference['source'],
  role: WorkbenchObjectReferenceRole,
  fieldPath: string,
): WorkbenchConfidence => {
  if (source === 'path' && role === 'resource') return 'high'
  if (role !== 'unknown' && normalizeField(fieldPath).length >= 2) return 'medium'
  return 'low'
}

const pushReference = (
  references: WorkbenchObjectReference[],
  evidenceId: string,
  source: WorkbenchObjectReference['source'],
  fieldPath: string,
  value: string,
) => {
  const trimmed = value.trim()
  if (!looksLikeObjectValue(trimmed)) return

  const role = inferRole(fieldPath)
  const normalizedField = normalizeField(fieldPath)
  references.push({
    id: `${evidenceId}:${source}:${fieldPath}:${trimmed}`,
    sourceEvidenceId: evidenceId,
    source,
    fieldPath,
    normalizedField,
    value: trimmed,
    role,
    confidence: inferConfidence(source, role, fieldPath),
  })
}

const flattenObject = (
  value: unknown,
  path: string,
  visitor: (fieldPath: string, raw: string) => void,
) => {
  if (Array.isArray(value)) {
    value.forEach((item, index) => {
      flattenObject(item, `${path}[${index}]`, visitor)
    })
    return
  }

  if (value && typeof value === 'object') {
    Object.entries(value as Record<string, unknown>).forEach(([key, nested]) => {
      flattenObject(nested, path ? `${path}.${key}` : key, visitor)
    })
    return
  }

  if (typeof value === 'string' || typeof value === 'number') {
    visitor(path, String(value))
  }
}

const extractReferencesFromEvidence = (evidence: Evidence): WorkbenchObjectReference[] => {
  const references: WorkbenchObjectReference[] = []
  const parsedUrl = safeUrl(evidence.url)

  if (parsedUrl) {
    const segments = parsedUrl.pathname.split('/').filter(Boolean)
    segments.forEach((segment, index) => {
      if (!looksLikeObjectValue(segment)) return
      const previous = segments[index - 1] || 'path'
      if (COMMON_PATH_WORDS.has(previous.toLowerCase())) return
      pushReference(references, evidence.id, 'path', `path.${previous}`, segment)
    })

    parsedUrl.searchParams.forEach((value, key) => {
      pushReference(references, evidence.id, 'query', `query.${key}`, value)
    })
  }

  parseForm(evidence.request_body).forEach(([key, value]) => {
    pushReference(references, evidence.id, 'request_body', `body.${key}`, value)
  })

  const requestJson = parseJson(evidence.request_body)
  if (requestJson) {
    flattenObject(requestJson, 'body', (fieldPath, raw) => {
      pushReference(references, evidence.id, 'request_body', fieldPath, raw)
    })
  }

  const responseJson = parseJson(evidence.response_body)
  if (responseJson) {
    flattenObject(responseJson, 'response', (fieldPath, raw) => {
      pushReference(references, evidence.id, 'response_body', fieldPath, raw)
    })
  }

  return references
}

const inferStepKind = (evidence: Evidence): { stepKind: WorkbenchBehaviorStepKind; reason: string } => {
  const url = `${evidence.method} ${evidence.url}`.toLowerCase()
  if (evidence.location === 'system_agent_verification') {
    return { stepKind: 'verification', reason: wb('analysis.inferenceReason.verification') }
  }
  if (url.includes('export') || url.includes('download') || url.includes('preview')) {
    return { stepKind: 'export', reason: wb('analysis.inferenceReason.export') }
  }
  if (evidence.method === 'DELETE' || url.includes('delete') || url.includes('remove')) {
    return { stepKind: 'delete', reason: wb('analysis.inferenceReason.delete') }
  }
  if (evidence.method === 'GET' && (url.includes('list') || url.includes('search') || url.includes('page='))) {
    return { stepKind: 'list', reason: wb('analysis.inferenceReason.list') }
  }
  if (evidence.method === 'GET') {
    return { stepKind: 'detail', reason: wb('analysis.inferenceReason.detail') }
  }
  if (evidence.method === 'POST' && (url.includes('create') || url.includes('add') || url.includes('new'))) {
    return { stepKind: 'create', reason: wb('analysis.inferenceReason.create') }
  }
  if (
    evidence.method === 'PUT' ||
    evidence.method === 'PATCH' ||
    url.includes('update') ||
    url.includes('edit') ||
    url.includes('save')
  ) {
    return { stepKind: 'update', reason: wb('analysis.inferenceReason.update') }
  }
  if (
    url.includes('approve') ||
    url.includes('confirm') ||
    url.includes('submit') ||
    url.includes('reject') ||
    url.includes('execute')
  ) {
    return { stepKind: 'action', reason: wb('analysis.inferenceReason.action') }
  }
  return { stepKind: 'unknown', reason: wb('analysis.inferenceReason.unknown') }
}

const buildBehaviorSteps = (evidences: Evidence[]): WorkbenchBehaviorStep[] =>
  [...evidences]
    .sort((left, right) => new Date(left.timestamp).getTime() - new Date(right.timestamp).getTime())
    .map(evidence => {
      const inference = inferStepKind(evidence)
      return {
        evidenceId: evidence.id,
        timestamp: evidence.timestamp,
        method: evidence.method,
        url: evidence.url,
        stepKind: inference.stepKind,
        reason: inference.reason,
      }
    })

const buildObjectPool = (references: WorkbenchObjectReference[]): WorkbenchObjectPoolGroup[] => {
  const groups = new Map<string, WorkbenchObjectReference[]>()
  references.forEach(reference => {
    const key = `${reference.role}:${reference.normalizedField || reference.fieldPath}`
    const next = groups.get(key) || []
    next.push(reference)
    groups.set(key, next)
  })

  return [...groups.entries()]
    .map(([key, items]) => {
      const uniqueValues = [...new Set(items.map(item => item.value))]
      const evidenceCount = new Set(items.map(item => item.sourceEvidenceId)).size
      const confidence = items.some(item => item.confidence === 'high')
        ? 'high'
        : items.some(item => item.confidence === 'medium')
          ? 'medium'
          : 'low'
      return {
        key,
        label: items[0]?.fieldPath || key,
        role: items[0]?.role || 'unknown',
        confidence,
        uniqueValues,
        referenceCount: items.length,
        evidenceCount,
        references: items,
      } satisfies WorkbenchObjectPoolGroup
    })
    .sort((left, right) => {
      const score = (group: WorkbenchObjectPoolGroup) =>
        group.role === 'resource' ? 30 : group.role === 'tenant' ? 20 : group.role === 'owner' ? 10 : 0
      return score(right) - score(left) || right.uniqueValues.length - left.uniqueValues.length
    })
}

const suggestionId = (strategy: WorkbenchSuggestionStrategy, targetEvidenceId: string, targetField: string) =>
  `${strategy}:${targetEvidenceId}:${targetField}`

const buildSuggestions = (
  baselineEvidenceId: string | null,
  objectPool: WorkbenchObjectPoolGroup[],
  evidences: Evidence[],
): WorkbenchTestSuggestion[] => {
  const suggestions: WorkbenchTestSuggestion[] = []
  const baselineEvidence = evidences.find(item => item.id === baselineEvidenceId) || evidences[0]
  if (!baselineEvidence) return suggestions

  const baselineRefs = extractReferencesFromEvidence(baselineEvidence)
  baselineRefs.forEach(reference => {
    const group = objectPool.find(
      item => item.role === reference.role && item.references.some(candidate => candidate.id === reference.id),
    )
    if (!group) return

    const candidateValues = group.uniqueValues.filter(value => value !== reference.value).slice(0, 5)
    if (candidateValues.length === 0) return

    let strategy: WorkbenchSuggestionStrategy = 'cross_replace'
    let severity: WorkbenchTestSuggestion['severity'] = 'medium'
    let title = wb('analysis.suggestionTitle.cross_replace', { field: reference.fieldPath })
    let why = wb('analysis.suggestionWhy.cross_replace')

    if (reference.role === 'owner') {
      strategy = 'owner_swap'
      severity = 'high'
      title = wb('analysis.suggestionTitle.owner_swap', { field: reference.fieldPath })
      why = wb('analysis.suggestionWhy.owner_swap')
    } else if (reference.role === 'tenant') {
      strategy = 'tenant_swap'
      severity = 'high'
      title = wb('analysis.suggestionTitle.tenant_swap', { field: reference.fieldPath })
      why = wb('analysis.suggestionWhy.tenant_swap')
    } else if (baselineEvidence.method === 'GET') {
      strategy = 'detail_probe'
      severity = 'high'
      title = wb('analysis.suggestionTitle.detail_probe', { field: reference.fieldPath })
      why = wb('analysis.suggestionWhy.detail_probe')
    }

    suggestions.push({
      id: suggestionId(strategy, baselineEvidence.id, reference.fieldPath),
      title,
      summary: `${baselineEvidence.method} ${baselineEvidence.url}`,
      severity,
      strategy,
      targetEvidenceId: baselineEvidence.id,
      targetField: reference.fieldPath,
      candidateValues,
      why,
    })
  })

  const exportEvidence = evidences.find(item =>
    `${item.method} ${item.url}`.toLowerCase().match(/export|download|preview/),
  )
  const resourcePool = objectPool.find(item => item.role === 'resource' && item.uniqueValues.length > 1)
  if (exportEvidence && resourcePool) {
    suggestions.push({
      id: suggestionId('export_check', exportEvidence.id, resourcePool.label),
      title: wb('analysis.suggestionTitle.export_check'),
      summary: `${exportEvidence.method} ${exportEvidence.url}`,
      severity: 'high',
      strategy: 'export_check',
      targetEvidenceId: exportEvidence.id,
      targetField: resourcePool.label,
      candidateValues: resourcePool.uniqueValues.slice(0, 5),
      why: wb('analysis.suggestionWhy.export_check'),
    })
  }

  return suggestions.slice(0, 8)
}

export const buildWorkbenchObjectAnalysis = (caseItem: WorkbenchCase): WorkbenchObjectAnalysis => {
  const evidences = caseItem.finding.evidence || []
  const references = evidences.flatMap(extractReferencesFromEvidence)
  const objectPool = buildObjectPool(references)
  return {
    behaviorSteps: buildBehaviorSteps(evidences),
    objectPool,
    suggestions: buildSuggestions(caseItem.baselineEvidenceId, objectPool, evidences),
  }
}

export const getBehaviorStepLabel = (kind: WorkbenchBehaviorStepKind) => {
  return wb(`analysis.behaviorStep.${kind === 'unknown' ? 'unknown' : kind}`)
}

export const getObjectRoleLabel = (role: WorkbenchObjectReferenceRole) => {
  return wb(`analysis.objectRole.${role === 'unknown' ? 'unknown' : role}`)
}

export const getConfidenceBadgeClass = (confidence: WorkbenchConfidence) => {
  switch (confidence) {
    case 'high':
      return 'badge-success'
    case 'medium':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

export const getSuggestionStrategyLabel = (strategy: WorkbenchSuggestionStrategy) => {
  return wb(`analysis.strategy.${strategy}`) || strategy
}
