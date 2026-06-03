import type { Ref } from 'vue'

import type { AgentMessage } from '@/types/agent'
import type {
  TeamV4Memory,
  TeamV4RunBootstrap,
  TeamV4SpecialistAssignment,
} from '@/types/teamRuntime'
import type { AssistantProfileOption } from './assistantProfiles'
import { normalizeToolIdList, type UiToolConfigPayload } from './toolConfigRuntime'

export type TeamV4MonitorReview = {
  kind: TeamV4Memory['kind']
  summary: string
  evidence: string[]
  riskSignals: string[]
  confidence: number
  promoteToLongTerm: boolean
}

export type TeamV4OrchestratorRecoveryDecision = {
  action: 'retry' | 'reassign' | 'ask_user' | 'cancel'
  reason: string
  revisedInstruction?: string | null
  targetSpecialistId?: string | null
}

export class TeamV4WaitingHumanError extends Error {
  detail: string

  constructor(detail: string) {
    super(detail)
    this.name = 'TeamV4WaitingHumanError'
    this.detail = detail
  }
}

const TEAM_V4_MEMORY_KINDS = new Set<TeamV4Memory['kind']>([
  'evidence',
  'decision',
  'risk',
  'blocker',
  'checkpoint',
  'artifact_summary',
])

const TEAM_V4_RECOVERY_ACTIONS = new Set<TeamV4OrchestratorRecoveryDecision['action']>([
  'retry',
  'reassign',
  'ask_user',
  'cancel',
])

export const stringifyForTeamMemory = (value: unknown, maxLength = 1800): string => {
  const raw =
    typeof value === 'string'
      ? value
      : (() => {
          try {
            return JSON.stringify(value)
          } catch {
            return String(value ?? '')
          }
        })()
  const normalized = raw.trim()
  if (normalized.length <= maxLength) return normalized
  return `${normalized.slice(0, maxLength)}...`
}

export const isTeamV4CancellationError = (error: unknown): boolean => {
  const message = (error as any)?.toString?.() || String(error ?? '')
  const normalized = message.toLowerCase()
  return (
    normalized.includes('execution cancelled') ||
    normalized.includes('cancelled by user') ||
    normalized.includes('shell execution cancelled')
  )
}

export const isTeamV4WaitingHumanError = (error: unknown): error is TeamV4WaitingHumanError =>
  error instanceof TeamV4WaitingHumanError

export const buildTeamV4WaitingHumanError = (detail: string) =>
  new TeamV4WaitingHumanError(detail)

export const buildTeamV4ModelOnlyToolConfig = () => ({
  enabled: false,
  selection_strategy: { Manual: [] as string[] },
  max_tools: 1,
  preselected_tools: [] as string[],
  disabled_tools: [] as string[],
  allowed_tools: [] as string[],
})

export const buildTeamV4SpecialistProfileToolConfig = (
  profile: AssistantProfileOption,
  baseline: UiToolConfigPayload
): UiToolConfigPayload => {
  const preselectedTools = normalizeToolIdList(profile.defaultPreselectedTools)
  const manualTools = normalizeToolIdList(profile.defaultManualTools)
  return {
    ...baseline,
    enabled: profile.defaultToolsEnabled === true,
    selection_strategy: profile.defaultToolSelectionStrategy || baseline.selection_strategy,
    max_tools: Math.max(
      1,
      Math.floor(Number(profile.defaultMaxTools) || Number(baseline.max_tools) || 1)
    ),
    preselected_tools: preselectedTools,
    disabled_tools: normalizeToolIdList(profile.defaultDisabledTools),
    manual_tools: manualTools,
    allowed_tools: normalizeToolIdList([...preselectedTools, ...manualTools]),
  }
}

const parseTeamV4JsonObject = (raw: string, label: string): Record<string, any> => {
  const trimmed = raw.trim()
  const fenced = trimmed.match(/```(?:json)?\s*([\s\S]*?)```/i)
  const source = (fenced?.[1] || trimmed).trim()
  const start = source.indexOf('{')
  const end = source.lastIndexOf('}')
  if (start < 0 || end <= start) {
    throw new Error(`${label} did not return a JSON object.`)
  }
  try {
    const parsed = JSON.parse(source.slice(start, end + 1))
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('not an object')
    }
    return parsed as Record<string, any>
  } catch (error) {
    throw new Error(`${label} returned invalid JSON: ${error}`)
  }
}

const readTeamV4StringArray = (value: unknown, field: string): string[] => {
  if (!Array.isArray(value) || value.some(item => typeof item !== 'string')) {
    throw new Error(`Team v4 model output requires string[] field: ${field}.`)
  }
  return value.map(item => item.trim()).filter(Boolean)
}

export const parseTeamV4MonitorReview = (raw: string): TeamV4MonitorReview => {
  const parsed = parseTeamV4JsonObject(raw, 'Monitor review')
  const kind = parsed.kind
  if (typeof kind !== 'string' || !TEAM_V4_MEMORY_KINDS.has(kind as TeamV4Memory['kind'])) {
    throw new Error(`Monitor review returned invalid kind: ${String(kind)}.`)
  }
  if (typeof parsed.summary !== 'string' || !parsed.summary.trim()) {
    throw new Error('Monitor review requires a non-empty summary.')
  }
  const confidence = Number(parsed.confidence)
  if (!Number.isFinite(confidence) || confidence < 0 || confidence > 1) {
    throw new Error('Monitor review confidence must be between 0 and 1.')
  }
  if (typeof parsed.promoteToLongTerm !== 'boolean') {
    throw new Error('Monitor review requires boolean promoteToLongTerm.')
  }
  return {
    kind: kind as TeamV4Memory['kind'],
    summary: parsed.summary.trim(),
    evidence: readTeamV4StringArray(parsed.evidence, 'evidence'),
    riskSignals: readTeamV4StringArray(parsed.riskSignals, 'riskSignals'),
    confidence,
    promoteToLongTerm: parsed.promoteToLongTerm,
  }
}

export const parseTeamV4OrchestratorRecoveryDecision = (
  raw: string,
  availableSpecialistIds: Set<string>
): TeamV4OrchestratorRecoveryDecision => {
  const parsed = parseTeamV4JsonObject(raw, 'Orchestrator recovery decision')
  const action = parsed.action
  if (
    typeof action !== 'string' ||
    !TEAM_V4_RECOVERY_ACTIONS.has(action as TeamV4OrchestratorRecoveryDecision['action'])
  ) {
    throw new Error(`Orchestrator recovery decision returned invalid action: ${String(action)}.`)
  }
  if (typeof parsed.reason !== 'string' || !parsed.reason.trim()) {
    throw new Error('Orchestrator recovery decision requires a non-empty reason.')
  }
  const targetSpecialistId =
    typeof parsed.targetSpecialistId === 'string' && parsed.targetSpecialistId.trim()
      ? parsed.targetSpecialistId.trim()
      : null
  if (
    action === 'reassign' &&
    (!targetSpecialistId || !availableSpecialistIds.has(targetSpecialistId))
  ) {
    throw new Error(
      `Orchestrator recovery decision returned invalid targetSpecialistId: ${String(targetSpecialistId)}.`
    )
  }
  return {
    action: action as TeamV4OrchestratorRecoveryDecision['action'],
    reason: parsed.reason.trim(),
    revisedInstruction:
      typeof parsed.revisedInstruction === 'string' && parsed.revisedInstruction.trim()
        ? parsed.revisedInstruction.trim()
        : null,
    targetSpecialistId,
  }
}

export const buildMonitorMemoryContent = (params: {
  eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
  goal: string
  taskTitle?: string
  result?: unknown
  error?: string
}) => {
  if (params.eventType === 'specialist_execution_failed') {
    return [
      `Specialist failed while working on: ${params.taskTitle || params.goal}`,
      `Failure summary: ${stringifyForTeamMemory(params.error || 'unknown error', 1200)}`,
    ].join('\n')
  }
  return [
    `Specialist completed task: ${params.taskTitle || params.goal}`,
    `Result summary: ${stringifyForTeamMemory(params.result, 1600)}`,
  ].join('\n')
}

export const extractMonitorSignals = (params: {
  eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
  result?: unknown
  error?: string
}) => {
  const text =
    params.eventType === 'specialist_execution_failed'
      ? stringifyForTeamMemory(params.error || '', 4000)
      : stringifyForTeamMemory(params.result, 4000)
  const lines = text
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(Boolean)
  const riskKeywords = [
    'failed',
    'failure',
    'error',
    'blocked',
    'risk',
    'vulnerable',
    'exploit',
    'unsafe',
    'permission',
  ]
  const riskSignals = lines
    .filter(line => riskKeywords.some(keyword => line.toLowerCase().includes(keyword)))
    .slice(0, 5)
  const evidence = lines.filter(line => !riskSignals.includes(line)).slice(0, 5)
  const summary =
    lines[0] ||
    (params.eventType === 'specialist_execution_failed'
      ? 'Specialist failed.'
      : 'Specialist completed.')
  return {
    evidence,
    riskSignals,
    summary,
  }
}

export const buildTeamV4SpecialistTaskPrompt = (
  assignment: TeamV4SpecialistAssignment,
  goal: string,
  assignmentCount: number,
  inheritedProgress: Array<{ taskId: string; specialistId: string; summary: string }>,
  recoveryInstruction?: string | null
) => {
  if (assignmentCount === 1 && inheritedProgress.length === 0 && !recoveryInstruction) return goal
  const lines = [
    `Team Specialist Task: ${assignment.task.title}`,
    `User Goal: ${goal}`,
    `Assigned Specialist: ${assignment.specialist.name}`,
    `Task Instruction: ${recoveryInstruction || assignment.task.instruction}`,
    `Acceptance Criteria: ${assignment.task.acceptance_criteria || 'Orchestrator review'}`,
  ]
  if (inheritedProgress.length > 0) {
    lines.push(
      `Inherited Progress:\n${inheritedProgress
        .map((item, index) => `${index + 1}. ${item.summary}`)
        .join('\n')}`
    )
  }
  return lines.join('\n\n')
}

export const buildTeamV4MonitorReviewPrompt = (params: {
  eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
  goal: string
  taskTitle?: string
  result?: unknown
  error?: string
}) =>
  [
    'You are the Team Monitor. Review the specialist output as an independent quality gate.',
    'Return strict JSON only. Do not include markdown or prose outside JSON.',
    '',
    'Schema:',
    '{',
    '  "kind": "evidence | decision | risk | blocker | checkpoint | artifact_summary",',
    '  "summary": "one concise high-value summary",',
    '  "evidence": ["specific useful facts or artifacts"],',
    '  "riskSignals": ["risks, blockers, quality issues, or empty array"],',
    '  "confidence": 0.0,',
    '  "promoteToLongTerm": false',
    '}',
    '',
    `User goal: ${params.goal}`,
    `Task title: ${params.taskTitle || 'root task'}`,
    `Event type: ${params.eventType}`,
    `Specialist result: ${stringifyForTeamMemory(params.result, 5000)}`,
    `Specialist error: ${stringifyForTeamMemory(params.error || '', 2000)}`,
  ].join('\n')

export const buildTeamV4OrchestratorRecoveryPrompt = (params: {
  goal: string
  assignment: TeamV4SpecialistAssignment
  attemptIndex: number
  error: string
  allowAskUser?: boolean
  availableSpecialists: Array<{
    id: string
    name: string
    model?: string | null
    contextMode?: string | null
  }>
  completedProgress: Array<{ taskId: string; specialistId: string; summary: string }>
  continuationHint?: string | null
}) =>
  [
    'You are the Team Orchestrator. A specialist attempt failed. Decide the next recovery action.',
    'Return strict JSON only. Do not include markdown or prose outside JSON.',
    '',
    'Allowed actions:',
    '- retry: run the same specialist once more with a revised instruction.',
    '- reassign: run another available specialist with a revised instruction.',
    ...(params.allowAskUser === false
      ? []
      : ['- ask_user: stop and request user input because the missing information cannot be inferred.']),
    '- cancel: stop the team run because continuing is unsafe or invalid.',
    '',
    'Schema:',
    '{',
    `  "action": "${params.allowAskUser === false ? 'retry | reassign | cancel' : 'retry | reassign | ask_user | cancel'}",`,
    '  "reason": "why this action is correct",',
    '  "revisedInstruction": "instruction for retry/reassign, or null",',
    '  "targetSpecialistId": "required only for reassign, otherwise null"',
    '}',
    '',
    `User goal: ${params.goal}`,
    `Failed task: ${params.assignment.task.title}`,
    `Original instruction: ${params.assignment.task.instruction}`,
    `Failed specialist: ${params.assignment.specialist.id} (${params.assignment.specialist.name})`,
    `Attempt index: ${params.attemptIndex}`,
    `Error: ${params.error}`,
    `Available specialists: ${stringifyForTeamMemory(params.availableSpecialists, 3000)}`,
    `Completed progress: ${stringifyForTeamMemory(params.completedProgress, 3000)}`,
    ...(params.continuationHint ? [`Continuation hint: ${params.continuationHint}`] : []),
  ].join('\n')

export const resolveTeamV4ContextMode = (value: string | null | undefined) => {
  if (value === 'claude-like' || value === 'codex-like' || value === 'sentinel-like') {
    return value
  }
  throw new Error(`Invalid Team v4 specialist context mode: ${value || 'empty'}`)
}

export const readTeamV4ToolPolicyMatrix = (teamRun: TeamV4RunBootstrap) => {
  const matrix = teamRun.run.policy_json?.toolPolicyMatrix
  if (!matrix || typeof matrix !== 'object' || Array.isArray(matrix)) {
    throw new Error('Team v4 run is missing toolPolicyMatrix.')
  }
  return matrix as Record<string, any>
}

export const readTeamV4MaxSpecialists = (teamRun: TeamV4RunBootstrap, assignmentCount: number) => {
  const policy = teamRun.run.policy_json?.concurrencyPolicy
  if (!policy || typeof policy !== 'object' || Array.isArray(policy)) {
    throw new Error('Team v4 run is missing concurrencyPolicy.')
  }
  const raw = Number(policy.maxSpecialists ?? policy.max_specialists)
  if (!Number.isFinite(raw) || raw < 1) {
    throw new Error('Team v4 concurrencyPolicy.maxSpecialists must be at least 1.')
  }
  return Math.min(Math.floor(raw), assignmentCount)
}

export const readTeamV4MaxTasksPerSpecialist = (teamRun: TeamV4RunBootstrap) => {
  const policy = teamRun.run.policy_json?.concurrencyPolicy
  if (!policy || typeof policy !== 'object' || Array.isArray(policy)) {
    throw new Error('Team v4 run is missing concurrencyPolicy.')
  }
  const raw = Number(policy.maxTasksPerSpecialist ?? policy.max_tasks_per_specialist)
  if (!Number.isFinite(raw) || raw < 1) {
    throw new Error('Team v4 concurrencyPolicy.maxTasksPerSpecialist must be at least 1.')
  }
  return Math.floor(raw)
}

export const readTeamV4WaitingHumanTimeoutMs = (teamRun: TeamV4RunBootstrap) => {
  const raw = Number(teamRun.run.policy_json?.harnessPolicy?.waitingHumanTimeoutSecs)
  const seconds = Number.isFinite(raw) && raw >= 5 ? Math.floor(raw) : 60
  return seconds * 1000
}

export const buildTeamV4SpecialistExecutionId = (
  runId: string,
  taskId: string,
  specialistId: string,
  attemptIndex: number
) => `team-v4:${runId}:${taskId}:specialist:${specialistId}:attempt:${attemptIndex}`

export const buildTeamV4OrchestratorFinalContent = (
  specialistResults: Array<{
    specialistId: string
    taskId: string
    result: {
      response: string
    }
  }>
) => {
  const lines = ['Orchestrator summary: Team run completed.']
  specialistResults.forEach((item, index) => {
    lines.push('')
    lines.push(`Specialist ${index + 1} (${item.specialistId}) result:`)
    lines.push(stringifyForTeamMemory(item.result.response || 'No response content.', 2400))
  })
  return lines.join('\n')
}

export const appendTeamV4LocalMessage = (
  messages: Ref<AgentMessage[]>,
  content: string,
  metadata: Record<string, any>,
  type: AgentMessage['type'] = 'planning'
) => {
  messages.value.push({
    id: crypto.randomUUID(),
    type,
    content,
    timestamp: Date.now(),
    metadata,
  })
}

export const buildTeamV4InheritedProgress = (
  specialistResults: Array<{
    specialistId: string
    taskId: string
    result: {
      response: string
    }
  }>
) =>
  specialistResults.map(item => ({
    specialistId: item.specialistId,
    taskId: item.taskId,
    summary: stringifyForTeamMemory(
      item.result.response || 'Completed without response content.',
      600
    ),
  }))
