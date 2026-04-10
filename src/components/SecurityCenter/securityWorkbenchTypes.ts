import type { Evidence, Finding } from './vulnerabilityFindingTypes'

export type WorkbenchCaseStatus =
  | 'new'
  | 'investigating'
  | 'awaiting_verification'
  | 'verified'
  | 'false_positive'
  | 'archived'

export type WorkbenchNoteKind =
  | 'observation'
  | 'conclusion'
  | 'false_positive_reason'
  | 'remediation_note'
  | 'replay_note'

export type WorkbenchConfidence = 'high' | 'medium' | 'low'

export interface WorkbenchFindingSnapshot {
  id: string
  title: string
  vulnType: string
  severity: string
  confidence: WorkbenchConfidence
  status: string
  pluginId: string
  url: string
  method?: string
  description: string
  createdAt: string
  updatedAt: string
  firstSeenAt: string
  lastSeenAt: string
  evidence: Evidence[]
}

export interface WorkbenchCase {
  id: string
  findingId: string
  title: string
  status: WorkbenchCaseStatus
  currentConclusion: string
  priority: 'low' | 'medium' | 'high'
  baselineEvidenceId: string | null
  createdAt: string
  updatedAt: string
  lastActivityAt: string
  finding: WorkbenchFindingSnapshot
}

export interface WorkbenchNote {
  id: string
  caseId: string
  kind: WorkbenchNoteKind
  body: string
  createdAt: string
  author: string
}

export interface WorkbenchCaseListItem extends WorkbenchCase {
  noteCount: number
}

export interface WorkbenchCaseListResult {
  items: WorkbenchCaseListItem[]
  total: number
  page: number
  pageSize: number
}

export interface WorkbenchCaseDetailResult {
  caseItem: WorkbenchCase
  notes: WorkbenchNote[]
  activities: WorkbenchActivity[]
  executionDrafts: WorkbenchExecutionDraft[]
  executionRuns: WorkbenchExecutionRun[]
  verifierRuns: WorkbenchVerifierRun[]
  assessmentSuggestion: WorkbenchAssessmentSuggestion | null
}

export interface WorkbenchSystemAgentStatus {
  profileId: string
  capability: string
  enabled: boolean
  autoMode: boolean
  allowActiveReplay: boolean
  shadowMode: boolean
  scopeHosts: string[]
}

export interface WorkbenchCaseListQuery {
  search?: string
  status?: WorkbenchCaseStatus | ''
  page?: number
  pageSize?: number
}

export interface WorkbenchCasePatch {
  status?: WorkbenchCaseStatus
  currentConclusion?: string
  priority?: 'low' | 'medium' | 'high'
  baselineEvidenceId?: string | null
}

export interface WorkbenchDeleteCasesResult {
  deletedCaseIds: string[]
  deletedCaseCount: number
  deletedNoteCount: number
  deletedActivityCount: number
  deletedExecutionDraftCount: number
  deletedExecutionRunCount: number
}

export interface WorkbenchFindingSyncResult {
  caseId: string
  findingId: string
  caseStatus: WorkbenchCaseStatus
  previousFindingStatus: string
  nextFindingStatus: string
}

export type WorkbenchActivityKind = 'finding_sync' | 'suggestion_sync' | 'draft_execution'

export type WorkbenchActivityStateSnapshot = Record<string, unknown>

export interface WorkbenchActivity {
  id: string
  caseId: string
  kind: WorkbenchActivityKind
  title: string
  summary: string
  before?: WorkbenchActivityStateSnapshot | null
  after?: WorkbenchActivityStateSnapshot | null
  createdAt: string
  actor: string
}

export type WorkbenchObjectReferenceSource = 'path' | 'query' | 'request_body' | 'response_body'
export type WorkbenchObjectReferenceRole = 'resource' | 'owner' | 'tenant' | 'operator' | 'unknown'

export interface WorkbenchObjectReference {
  id: string
  sourceEvidenceId: string
  source: WorkbenchObjectReferenceSource
  fieldPath: string
  normalizedField: string
  value: string
  role: WorkbenchObjectReferenceRole
  confidence: WorkbenchConfidence
}

export interface WorkbenchObjectPoolGroup {
  key: string
  label: string
  role: WorkbenchObjectReferenceRole
  confidence: WorkbenchConfidence
  uniqueValues: string[]
  referenceCount: number
  evidenceCount: number
  references: WorkbenchObjectReference[]
}

export type WorkbenchBehaviorStepKind =
  | 'list'
  | 'detail'
  | 'create'
  | 'update'
  | 'delete'
  | 'export'
  | 'action'
  | 'verification'
  | 'unknown'

export interface WorkbenchBehaviorStep {
  evidenceId: string
  timestamp: string
  method: string
  url: string
  stepKind: WorkbenchBehaviorStepKind
  reason: string
}

export type WorkbenchSuggestionStrategy =
  | 'cross_replace'
  | 'owner_swap'
  | 'tenant_swap'
  | 'export_check'
  | 'detail_probe'

export interface WorkbenchTestSuggestion {
  id: string
  title: string
  summary: string
  severity: 'high' | 'medium' | 'low'
  strategy: WorkbenchSuggestionStrategy
  targetEvidenceId: string
  targetField: string
  candidateValues: string[]
  why: string
}

export interface WorkbenchObjectAnalysis {
  behaviorSteps: WorkbenchBehaviorStep[]
  objectPool: WorkbenchObjectPoolGroup[]
  suggestions: WorkbenchTestSuggestion[]
}

export interface WorkbenchReplayPlanStep {
  id: string
  title: string
  detail: string
}

export interface WorkbenchReplayPlanStopCondition {
  id: string
  detail: string
}

export interface WorkbenchReplayPlan {
  id: string
  title: string
  summary: string
  strategy: WorkbenchSuggestionStrategy
  severity: 'high' | 'medium' | 'low'
  readOnly: boolean
  targetEvidenceId: string
  targetField: string
  targetMethod: string
  targetUrl: string
  candidateValues: string[]
  steps: WorkbenchReplayPlanStep[]
  stopConditions: WorkbenchReplayPlanStopCondition[]
  rationale: string
}

export type WorkbenchExecutionDraftStatus = 'draft' | 'ready' | 'paused' | 'archived'

export interface WorkbenchExecutionDraft {
  id: string
  caseId: string
  planId: string
  title: string
  status: WorkbenchExecutionDraftStatus
  readOnly: boolean
  severity: 'high' | 'medium' | 'low'
  targetEvidenceId: string
  targetField: string
  targetMethod: string
  targetUrl: string
  candidateValues: string[]
  steps: WorkbenchReplayPlanStep[]
  stopConditions: WorkbenchReplayPlanStopCondition[]
  rationale: string
  createdAt: string
  updatedAt: string
}

export type WorkbenchExecutionRunStatus = 'completed' | 'blocked' | 'failed'
export type WorkbenchExecutionAttemptOutcome = 'changed' | 'same' | 'blocked' | 'not_found' | 'error'
export type WorkbenchExecutionSimilarityLevel = 'high' | 'medium' | 'low'

export interface WorkbenchExecutionDiff {
  matchedStatus: boolean
  matchedBody: boolean
  similarityLevel: WorkbenchExecutionSimilarityLevel
  baselineStatus: number | null
  baselineLength: number
  responseLength: number
  changedSignals: string[]
}

export interface WorkbenchExecutionAttempt {
  id: string
  candidateValue: string
  mutatedUrl: string
  responseStatus: number | null
  responseSnippet: string
  outcome: WorkbenchExecutionAttemptOutcome
  diff: WorkbenchExecutionDiff
}

export interface WorkbenchExecutionRun {
  id: string
  caseId: string
  draftId: string
  status: WorkbenchExecutionRunStatus
  method: string
  baselineUrl: string
  targetField: string
  summary: string
  attempts: WorkbenchExecutionAttempt[]
  startedAt: string
  finishedAt: string
}

export interface WorkbenchVerifierRun {
  id: string
  status: string
  triggerEvent?: string | null
  strategy?: string | null
  verified: boolean
  responseStatus?: number | null
  summary: string
  evidenceId?: string | null
  errorMessage?: string | null
  startedAt: string
  finishedAt?: string | null
}

export interface WorkbenchAssessmentSuggestion {
  title: string
  summary: string
  suggestedStatus: WorkbenchCaseStatus
  suggestedConclusion: string
  confidence: WorkbenchConfidence
  signals: string[]
}

export const normalizeWorkbenchConfidence = (
  confidence: string | null | undefined,
): WorkbenchConfidence => {
  if (confidence === 'high' || confidence === 'medium' || confidence === 'low') {
    return confidence
  }
  return 'low'
}

export const buildWorkbenchFindingSnapshot = (finding: Finding): WorkbenchFindingSnapshot => ({
  id: finding.id,
  title: finding.title,
  vulnType: finding.vuln_type,
  severity: finding.severity,
  confidence: normalizeWorkbenchConfidence(finding.confidence),
  status: finding.status,
  pluginId: finding.plugin_id,
  url: finding.url,
  method: finding.method,
  description: finding.description,
  createdAt: finding.created_at,
  updatedAt: finding.updated_at,
  firstSeenAt: finding.first_seen_at,
  lastSeenAt: finding.last_seen_at,
  evidence: finding.evidence || [],
})
