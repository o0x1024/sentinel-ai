import type { Evidence, Finding } from './vulnerabilityFindingTypes'
import { wb } from './securityWorkbenchLocale'

type VerificationMeta = {
  verificationOutcome?: string
  strategyUsed?: string
  mutated?: boolean
  baselineExchange?: WorkbenchEvidenceExchange | null
  replayExchange?: WorkbenchEvidenceExchange | null
}

type ObservationPayload = {
  riskType?: string
  confidence?: string
  signals?: string[]
  url?: string
  method?: string
  pathTemplate?: string | null
}

type ContextPayload = {
  riskType?: string
  confidence?: string
  signals?: string[]
  baselineRequest?: {
    method?: string
    url?: string
    requestHeaders?: string | null
    requestBody?: string | null
    responseStatus?: number | null
    responseHeaders?: string | null
    responseBody?: string | null
  }
}

export type WorkbenchEvidenceExchange = {
  requestMethod: string
  requestUrl: string
  requestHeaders?: string | null
  requestBody?: string | null
  responseStatus?: number | null
  responseHeaders?: string | null
  responseBody?: string | null
}

const parseJson = <T>(input?: string | null): T | null => {
  if (!input) return null
  try {
    return JSON.parse(input) as T
  } catch {
    return null
  }
}

const normalizePath = (rawUrl?: string | null) => {
  if (!rawUrl) return '/'
  try {
    const parsed = new URL(rawUrl)
    return parsed.pathname || '/'
  } catch {
    return rawUrl
  }
}

const joinSignals = (signals?: string[] | null) => {
  if (!signals?.length) return ''
  return signals.join(' · ')
}

const readMessage = (path: string, fallback: string) => {
  const value = wb(path)
  return value === path ? fallback : value
}

const normalizeRiskLabel = (riskType?: string | null) =>
  readMessage(`systemAgent.risk.${riskType || 'none'}`, riskType || wb('common.unknown'))

const normalizeConfidenceLabel = (confidence?: string | null) =>
  readMessage(
    `priority.${confidence === 'high' ? 'high' : confidence === 'medium' ? 'medium' : 'low'}`,
    confidence || wb('common.unknown')
  )

const normalizeOutcomeLabel = (outcome?: string | null) =>
  readMessage(
    `systemAgent.outcome.${outcome || 'verification_not_confirmed'}`,
    outcome || wb('common.unknown')
  )

const normalizeStrategyLabel = (strategy?: string | null) =>
  readMessage(
    `systemAgent.strategy.${strategy || 'manual_review'}`,
    strategy || wb('common.unknown')
  )

const normalizeMutatedLabel = (mutated?: boolean | null) =>
  mutated ? wb('common.yes') : wb('common.no')

const appendSignals = (base: string, signals?: string[] | null) => {
  const joined = joinSignals(signals)
  if (!joined) return base
  return `${base}\n${wb('systemAgent.evidence.signals', { signals: joined })}`
}

const buildObservationSummary = (evidence: Evidence) => {
  const payload = parseJson<ObservationPayload>(evidence.request_body)
  if (!payload) return evidence.evidence_snippet
  return appendSignals(
    wb('systemAgent.evidence.observationSummary', {
      method: payload.method || evidence.method,
      path: normalizePath(payload.pathTemplate || payload.url || evidence.url),
      risk: normalizeRiskLabel(payload.riskType),
      confidence: normalizeConfidenceLabel(payload.confidence),
    }),
    payload.signals
  )
}

const buildContextSummary = (evidence: Evidence) => {
  const payload = parseJson<ContextPayload>(evidence.response_body)
  if (!payload) return evidence.evidence_snippet
  return appendSignals(
    wb('systemAgent.evidence.contextSummary', {
      method: evidence.method,
      path: normalizePath(evidence.url),
      risk: normalizeRiskLabel(payload.riskType),
    }),
    payload.signals
  )
}

const buildVerificationSummary = (evidence: Evidence) => {
  const meta = parseJson<VerificationMeta>(evidence.response_headers)
  if (!meta) return evidence.evidence_snippet
  const key =
    meta.verificationOutcome === 'verification_confirmed'
      ? 'systemAgent.evidence.verificationConfirmed'
      : 'systemAgent.evidence.verificationNotConfirmed'
  return wb(key, {
    strategy: normalizeStrategyLabel(meta.strategyUsed),
    status: evidence.response_status ?? '-',
    outcome: normalizeOutcomeLabel(meta.verificationOutcome),
    mutated: normalizeMutatedLabel(meta.mutated),
  })
}

type SystemAgentLikeFinding = {
  plugin_id?: string
  pluginId?: string
  vuln_type?: string
  vulnType?: string
  confidence: string
  method?: string
  url: string
  evidence?: Evidence[]
  description: string
  title: string
}

const readPluginId = (finding: SystemAgentLikeFinding) =>
  finding.plugin_id || finding.pluginId || ''
const readRiskType = (finding: SystemAgentLikeFinding) =>
  finding.vuln_type || finding.vulnType || 'none'

const isSystemAgentFinding = (finding: SystemAgentLikeFinding) =>
  readPluginId(finding).startsWith('agent:')

export const getWorkbenchEvidenceLocationLabel = (location: string) => {
  if (location.startsWith('system_agent_')) {
    return wb(`systemAgent.location.${location}`)
  }
  return location
}

export const getWorkbenchEvidenceSnippet = (evidence: Evidence) => {
  switch (evidence.location) {
    case 'system_agent_observation':
      return buildObservationSummary(evidence)
    case 'system_agent_context':
      return buildContextSummary(evidence)
    case 'system_agent_verification':
      return buildVerificationSummary(evidence)
    default:
      return evidence.evidence_snippet
  }
}

export const formatWorkbenchRawPayload = (raw?: string | null) => {
  if (!raw) return ''
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

const buildDirectEvidenceExchange = (evidence: Evidence): WorkbenchEvidenceExchange | null => {
  const hasRequestDetails = Boolean(
    evidence.request_headers || evidence.request_body || evidence.url || evidence.method
  )
  const hasResponseDetails =
    evidence.response_status != null || Boolean(evidence.response_headers || evidence.response_body)
  if (!hasRequestDetails && !hasResponseDetails) return null
  return {
    requestMethod: evidence.method || 'GET',
    requestUrl: evidence.url,
    requestHeaders: evidence.request_headers || null,
    requestBody: evidence.request_body || null,
    responseStatus: evidence.response_status ?? null,
    responseHeaders: evidence.response_headers || null,
    responseBody: evidence.response_body || null,
  }
}

const buildContextEvidenceExchange = (evidence: Evidence): WorkbenchEvidenceExchange | null => {
  const payload =
    parseJson<ContextPayload>(evidence.response_body) ||
    parseJson<ContextPayload>(evidence.request_body)
  const baselineRequest = payload?.baselineRequest
  if (!baselineRequest) return buildDirectEvidenceExchange(evidence)
  return {
    requestMethod: baselineRequest.method || evidence.method || 'GET',
    requestUrl: baselineRequest.url || evidence.url,
    requestHeaders: baselineRequest.requestHeaders || null,
    requestBody: baselineRequest.requestBody || null,
    responseStatus: baselineRequest.responseStatus ?? evidence.response_status ?? null,
    responseHeaders: baselineRequest.responseHeaders || null,
    responseBody: baselineRequest.responseBody || null,
  }
}

export const getWorkbenchEvidenceExchange = (
  evidence: Evidence
): WorkbenchEvidenceExchange | null => {
  if (evidence.location === 'system_agent_verification') {
    return getWorkbenchVerificationReplayExchange(evidence) || buildDirectEvidenceExchange(evidence)
  }
  if (evidence.location === 'system_agent_context') {
    return buildContextEvidenceExchange(evidence)
  }
  return buildDirectEvidenceExchange(evidence)
}

export const getWorkbenchVerificationBaselineExchange = (
  evidence: Evidence
): WorkbenchEvidenceExchange | null => {
  const meta = parseJson<VerificationMeta>(evidence.response_headers)
  return meta?.baselineExchange || null
}

export const getWorkbenchVerificationReplayExchange = (
  evidence: Evidence
): WorkbenchEvidenceExchange | null => {
  const meta = parseJson<VerificationMeta>(evidence.response_headers)
  return meta?.replayExchange || null
}

export const hasWorkbenchEvidenceExchange = (evidence: Evidence) =>
  Boolean(getWorkbenchEvidenceExchange(evidence))

export const getWorkbenchFindingTitle = (finding: Finding | SystemAgentLikeFinding) => {
  if (!isSystemAgentFinding(finding)) return finding.title
  return wb('finding.title', {
    method: finding.method || 'GET',
    path: normalizePath(finding.url),
    risk: normalizeRiskLabel(readRiskType(finding)),
  })
}

export const getWorkbenchFindingDescription = (finding: Finding | SystemAgentLikeFinding) => {
  if (!isSystemAgentFinding(finding)) return finding.description

  const contextEvidence = (finding.evidence || []).find(
    item => item.location === 'system_agent_context'
  )
  if (contextEvidence) {
    return getWorkbenchEvidenceSnippet(contextEvidence)
  }

  const observationEvidence = (finding.evidence || []).find(
    item => item.location === 'system_agent_observation'
  )
  if (observationEvidence) {
    return getWorkbenchEvidenceSnippet(observationEvidence)
  }

  return wb('finding.fallbackDescription', {
    risk: normalizeRiskLabel(readRiskType(finding)),
    confidence: normalizeConfidenceLabel(finding.confidence),
  })
}
