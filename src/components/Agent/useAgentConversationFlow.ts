import { nextTick, type ComputedRef, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { agentTeamApi } from '@/api/agentTeam'
import type { AgentMessage } from '@/types/agent'
import { maybeAutoRenameConversationByFirstMessage } from './agentConversationTitleSupport'
import type { PersistedConversationMessageRow } from './agentConversationHistorySupport'
import { loadConversationHistory as loadConversationHistorySupport } from './agentConversationLoadSupport'
import {
  ensureConversationForExecution as ensureConversationForExecutionSupport,
  executeConversationTask as executeConversationTaskSupport,
  takeOverConversationExecution as takeOverConversationExecutionSupport,
} from './agentConversationExecutionSupport'
import {
  clearConversationSession,
  createConversationSession,
  loadLatestConversationSession,
  selectConversationSession,
} from './agentConversationSessionSupport'
import {
  buildMessageReplaySnapshot,
  deleteConversationTailForReplay,
} from './agentMessageReplaySupport'
import type { AiConversationDetail, AiConversationSummary } from './conversationTypes'
import type { AgentExecutionFinishedEvent, PersistedAgentExecutionState } from './executionState'
import { normalizeTeamHumanInputContent, shouldSuppressTeamMirrorNoiseMessage } from './agentTeamMessageSupport'
import type { AssistantModelOption } from './agentDraftTypes'
import { buildVisionModelUnsupportedError } from './agentVisionErrorSupport'
import { prepareSubmission } from './agentSubmissionSupport'
import { runTeamV4AssignmentsWithDependencies } from './teamV4AssignmentScheduler'
import { planTeamV4SpecialistAssignments } from './teamV4OrchestratorPlanning'
import { startTeamV4SpecialistActivityProgress } from './teamV4SpecialistActivityProgress'
import {
  buildRuntimeToolConfigForExecution,
  buildRuntimeToolConfigForTeamRole,
  normalizeToolIdList,
  type UiToolConfigPayload,
} from './toolConfigRuntime'
import type { AssistantProfileOption } from './assistantProfiles'
import { teamRuntimeApi } from '@/api/teamRuntime'
import type {
  TeamV4Event,
  TeamV4Memory,
  TeamV4RunBootstrap,
  TeamV4SpecialistAssignment,
} from '@/types/teamRuntime'

type TeamV4MonitorReview = {
  kind: TeamV4Memory['kind']
  summary: string
  evidence: string[]
  riskSignals: string[]
  confidence: number
  promoteToLongTerm: boolean
}

type TeamV4OrchestratorRecoveryDecision = {
  action: 'retry' | 'reassign' | 'ask_user' | 'cancel'
  reason: string
  revisedInstruction?: string | null
  targetSpecialistId?: string | null
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

const stringifyForTeamMemory = (value: unknown, maxLength = 1800): string => {
  const raw = typeof value === 'string'
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

const isTeamV4CancellationError = (error: unknown): boolean => {
  const message = (error as any)?.toString?.() || String(error ?? '')
  const normalized = message.toLowerCase()
  return normalized.includes('execution cancelled')
    || normalized.includes('cancelled by user')
    || normalized.includes('shell execution cancelled')
}

const buildTeamV4ModelOnlyToolConfig = () => ({
  enabled: false,
  selection_strategy: { Manual: [] as string[] },
  max_tools: 1,
  fixed_tools: [] as string[],
  disabled_tools: [] as string[],
  allowed_tools: [] as string[],
})

const buildTeamV4SpecialistProfileToolConfig = (
  profile: AssistantProfileOption,
  baseline: UiToolConfigPayload,
): UiToolConfigPayload => {
  const fixedTools = normalizeToolIdList(profile.defaultFixedTools)
  const manualTools = normalizeToolIdList(profile.defaultManualTools)
  return {
    ...baseline,
    enabled: profile.defaultToolsEnabled === true,
    selection_strategy: profile.defaultToolSelectionStrategy || baseline.selection_strategy,
    max_tools: Math.max(1, Math.floor(Number(profile.defaultMaxTools) || Number(baseline.max_tools) || 1)),
    fixed_tools: fixedTools,
    disabled_tools: normalizeToolIdList(profile.defaultDisabledTools),
    manual_tools: manualTools,
    allowed_tools: normalizeToolIdList([...fixedTools, ...manualTools]),
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
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw new Error(`Team v4 model output requires string[] field: ${field}.`)
  }
  return value.map((item) => item.trim()).filter(Boolean)
}

const parseTeamV4MonitorReview = (raw: string): TeamV4MonitorReview => {
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

const parseTeamV4OrchestratorRecoveryDecision = (
  raw: string,
  availableSpecialistIds: Set<string>,
): TeamV4OrchestratorRecoveryDecision => {
  const parsed = parseTeamV4JsonObject(raw, 'Orchestrator recovery decision')
  const action = parsed.action
  if (typeof action !== 'string' || !TEAM_V4_RECOVERY_ACTIONS.has(action as TeamV4OrchestratorRecoveryDecision['action'])) {
    throw new Error(`Orchestrator recovery decision returned invalid action: ${String(action)}.`)
  }
  if (typeof parsed.reason !== 'string' || !parsed.reason.trim()) {
    throw new Error('Orchestrator recovery decision requires a non-empty reason.')
  }
  const targetSpecialistId = typeof parsed.targetSpecialistId === 'string' && parsed.targetSpecialistId.trim()
    ? parsed.targetSpecialistId.trim()
    : null
  if (action === 'reassign' && (!targetSpecialistId || !availableSpecialistIds.has(targetSpecialistId))) {
    throw new Error(`Orchestrator recovery decision returned invalid targetSpecialistId: ${String(targetSpecialistId)}.`)
  }
  return {
    action: action as TeamV4OrchestratorRecoveryDecision['action'],
    reason: parsed.reason.trim(),
    revisedInstruction: typeof parsed.revisedInstruction === 'string' && parsed.revisedInstruction.trim()
      ? parsed.revisedInstruction.trim()
      : null,
    targetSpecialistId,
  }
}

const buildMonitorMemoryContent = (params: {
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

const extractMonitorSignals = (params: {
  eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
  result?: unknown
  error?: string
}) => {
  const text = params.eventType === 'specialist_execution_failed'
    ? stringifyForTeamMemory(params.error || '', 4000)
    : stringifyForTeamMemory(params.result, 4000)
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
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
    .filter((line) => riskKeywords.some((keyword) => line.toLowerCase().includes(keyword)))
    .slice(0, 5)
  const evidence = lines
    .filter((line) => !riskSignals.includes(line))
    .slice(0, 5)
  const summary = lines[0] || (params.eventType === 'specialist_execution_failed' ? 'Specialist failed.' : 'Specialist completed.')
  return {
    evidence,
    riskSignals,
    summary,
  }
}

const buildTeamV4SpecialistTaskPrompt = (
  assignment: TeamV4SpecialistAssignment,
  goal: string,
  assignmentCount: number,
  inheritedProgress: Array<{ taskId: string; specialistId: string; summary: string }>,
  recoveryInstruction?: string | null,
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
    lines.push(`Inherited Progress:\n${inheritedProgress
      .map((item, index) => `${index + 1}. ${item.summary}`)
      .join('\n')}`)
  }
  return lines.join('\n\n')
}

const buildTeamV4MonitorReviewPrompt = (params: {
  eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
  goal: string
  taskTitle?: string
  result?: unknown
  error?: string
}) => [
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

const buildTeamV4OrchestratorRecoveryPrompt = (params: {
  goal: string
  assignment: TeamV4SpecialistAssignment
  attemptIndex: number
  error: string
  availableSpecialists: Array<{ id: string; name: string; model?: string | null; contextMode?: string | null }>
  completedProgress: Array<{ taskId: string; specialistId: string; summary: string }>
}) => [
  'You are the Team Orchestrator. A specialist attempt failed. Decide the next recovery action.',
  'Return strict JSON only. Do not include markdown or prose outside JSON.',
  '',
  'Allowed actions:',
  '- retry: run the same specialist once more with a revised instruction.',
  '- reassign: run another available specialist with a revised instruction.',
  '- ask_user: stop and request user input because the missing information cannot be inferred.',
  '- cancel: stop the team run because continuing is unsafe or invalid.',
  '',
  'Schema:',
  '{',
  '  "action": "retry | reassign | ask_user | cancel",',
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
].join('\n')

const resolveTeamV4ContextMode = (value: string | null | undefined) => {
  if (value === 'claude-like' || value === 'codex-like' || value === 'sentinel-like') {
    return value
  }
  throw new Error(`Invalid Team v4 specialist context mode: ${value || 'empty'}`)
}

const readTeamV4ToolPolicyMatrix = (teamRun: TeamV4RunBootstrap) => {
  const matrix = teamRun.run.policy_json?.toolPolicyMatrix
  if (!matrix || typeof matrix !== 'object' || Array.isArray(matrix)) {
    throw new Error('Team v4 run is missing toolPolicyMatrix.')
  }
  return matrix as Record<string, any>
}

const readTeamV4MaxSpecialists = (teamRun: TeamV4RunBootstrap, assignmentCount: number) => {
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

const buildTeamV4SpecialistExecutionId = (
  runId: string,
  taskId: string,
  specialistId: string,
  attemptIndex: number,
) => `team-v4:${runId}:${taskId}:specialist:${specialistId}:attempt:${attemptIndex}`

const buildTeamV4OrchestratorFinalContent = (specialistResults: Array<{
  specialistId: string
  taskId: string
  result: {
    response: string
  }
}>) => {
  const lines = ['Orchestrator summary: Team run completed.']
  specialistResults.forEach((item, index) => {
    lines.push('')
    lines.push(`Specialist ${index + 1} (${item.specialistId}) result:`)
    lines.push(stringifyForTeamMemory(item.result.response || 'No response content.', 2400))
  })
  return lines.join('\n')
}

const appendTeamV4LocalMessage = (
  messages: Ref<AgentMessage[]>,
  content: string,
  metadata: Record<string, any>,
  type: AgentMessage['type'] = 'planning',
) => {
  messages.value.push({
    id: crypto.randomUUID(),
    type,
    content,
    timestamp: Date.now(),
    metadata,
  })
}

const buildTeamV4InheritedProgress = (specialistResults: Array<{
  specialistId: string
  taskId: string
  result: {
    response: string
  }
}>) =>
  specialistResults.map((item) => ({
    specialistId: item.specialistId,
    taskId: item.taskId,
    summary: stringifyForTeamMemory(item.result.response || 'Completed without response content.', 600),
  }))

const watchAgentExecutionFinished = (executionId: string, timeoutMs: number) => {
  let settled = false
  let unlisten: UnlistenFn | null = null
  let timeoutHandle: number | null = null

  const cleanup = () => {
    if (timeoutHandle) {
      window.clearTimeout(timeoutHandle)
      timeoutHandle = null
    }
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  const promise = new Promise<AgentExecutionFinishedEvent>((resolve, reject) => {
    timeoutHandle = window.setTimeout(() => {
      if (settled) return
      settled = true
      cleanup()
      reject(new Error(`Timed out waiting for agent execution: ${executionId}`))
    }, timeoutMs)

    listen<AgentExecutionFinishedEvent>('agent:execution_finished', (event) => {
      const payload = event.payload
      if (!payload || payload.execution_id !== executionId || settled) return
      settled = true
      cleanup()
      resolve(payload)
    }).then((dispose) => {
      if (settled) {
        dispose()
        return
      }
      unlisten = dispose
    }).catch((error) => {
      if (settled) return
      settled = true
      cleanup()
      reject(error)
    })
  })

  return {
    cancel: cleanup,
    promise,
  }
}

export const useAgentConversationFlow = (params: {
  activeTeamSessionId: Ref<string | null>
  activeTeamV4RunId: Ref<string | null>
  agentMessages: Ref<AgentMessage[]>
  agentStreamingContent: Ref<string>
  agentSubagents: Ref<any[]>
  assistantModelOptions: Ref<AssistantModelOption[]>
  assistantContextMode: Ref<'claude-like' | 'codex-like' | 'sentinel-like'>
  assistantExecutionMode: Ref<'single' | 'parallel'>
  assistantParallelJudgeModel: Ref<string>
  assistantParallelSelectedModels: Ref<string[]>
  assistantSelectedModel: Ref<string>
  buildToolConfig: () => UiToolConfigPayload
  clearAgentMessages: () => void
  clearDraftArtifacts: () => void
  clearTasksForCurrentContext: () => void
  closeConversationDrawer: () => void
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  conversationId: Ref<string | null>
  currentConversationTitle: Ref<string>
  emitComplete: (payload: any) => void
  emitError: (message: string) => void
  emitSubmit: (task: string) => void
  ensureConversationForTeamSession: () => Promise<any>
  executionIdProp?: string | null
  forceTasks: boolean
  getFailedToClearConversationLabel: () => string
  getFailedToStopExecutionLabel: () => string
  getNewConversationTitle: () => string
  getToolCallCompletedLabel: () => string
  getUnnamedConversationTitle: () => string
  getAssistantProfileOption: (profileId: string) => AssistantProfileOption | null
  handleStopTeamState: (nextState: string) => void
  hydrateTaskHistory: (conversationId: string) => Promise<void>
  historyLoadToken: Ref<number>
  inputValue: Ref<string>
  isHistoryLoading: Ref<boolean>
  isExecuting: ComputedRef<boolean>
  isTeamModeEnabled: Ref<boolean>
  isToolConfigEnabled: Ref<boolean>
  loadConversationList: () => void
  loadSubagentRuns: (parentExecutionId: string, loadToken?: number) => Promise<void>
  localError: Ref<string | null>
  pendingAttachments: Ref<any[]>
  processedDocuments: Ref<any[]>
  ragEnabled: Ref<boolean>
  referencedAssets: Ref<any[]>
  referencedFiles: Ref<any[]>
  referencedMessages: Ref<any[]>
  referencedTraffic: Ref<any[]>
  refreshTeamV4Workspace?: () => Promise<void>
  resetTerminal: () => void
  restoreArtifactsFromMessage: (message: AgentMessage) => void
  routeTeamMessage: (content: string) => Promise<any>
  scrollMessageViewportToBottom: () => void
  setMirroredConversationMessageIds: (ids: Set<string>) => void
  setPendingDocumentAttachments: (documents: any[]) => void
  startTeamV4AssistantRun: (content: string) => Promise<TeamV4RunBootstrap>
  startTeamExecutionRun: (bridgeMessage?: string) => Promise<void>
  stopAgentExecutionState: () => void
  stopTeamV4Run: () => Promise<void>
  submitInFlight: Ref<boolean>
  syncActiveTeamSession: () => Promise<void>
  syncTeamMessagesToMainFlow: (sessionId?: string | null) => Promise<void>
  teamModeEnabled: Ref<boolean>
  tenthManEnabled: Ref<boolean>
  webSearchEnabled: Ref<boolean>
}) => {
  const getUnsupportedVisionModelError = (usedAttachments: unknown[]): string | null => {
    if (usedAttachments.length === 0) return null
    const selectedModelKey = params.assistantSelectedModel.value.trim()
    const selectedModelOption = params.assistantModelOptions.value.find(
      (item) => item.value === selectedModelKey,
    )
    if (selectedModelOption?.visionCapability !== 'unsupported') {
      return null
    }
    const [provider = '', ...modelParts] = selectedModelKey.split('/')
    return buildVisionModelUnsupportedError(provider, modelParts.join('/'))
  }

  const handleConversationExecutionStateUpdate = (payload: AgentExecutionFinishedEvent) => {
    if (payload.execution_id !== params.conversationId.value) return
    params.conversationExecutionState.value = {
      ...payload,
      completed_at: new Date().toISOString(),
    }
  }

  const activeTeamV4ExecutionIds = new Set<string>()

  const runTeamV4ModelExecution = async (input: {
    contextMode: 'claude-like' | 'codex-like' | 'sentinel-like'
    executionId: string
    model: string | null
    prompt: string
    roleLabel: string
    timeoutMs?: number
  }) => {
    const executionFinished = watchAgentExecutionFinished(
      input.executionId,
      input.timeoutMs ?? 360_000,
    )
    let executionStarted = false
    let finished: AgentExecutionFinishedEvent
    try {
      activeTeamV4ExecutionIds.add(input.executionId)
      await executeConversationTaskSupport({
        assistantContextMode: input.contextMode,
        assistantSelectedModel: input.model,
        conversationId: input.executionId,
        defaultConversationTitle: params.getUnnamedConversationTitle(),
        displayContent: undefined,
        enableRag: false,
        enableTenthManRule: false,
        firstMessage: input.prompt,
        forceTasks: false,
        fullTask: input.prompt,
        maybeAutoRenameConversation: (renameParams) => {
          void maybeAutoRenameConversationByFirstMessage(renameParams)
        },
        onConversationListRefresh: params.loadConversationList,
        onCurrentConversationTitleChange: (title) => {
          params.currentConversationTitle.value = title
        },
        persistMessages: false,
        runAgentExecute: (request) => invoke('agent_execute', request),
        runtimeToolConfig: buildTeamV4ModelOnlyToolConfig(),
        skipAutoRename: true,
        usedAssets: [],
        usedAttachments: [],
        usedDocuments: [],
        usedFiles: [],
        usedMessages: [],
        usedTraffic: [],
      })
      executionStarted = true
      finished = await executionFinished.promise
    } catch (startError) {
      if (!executionStarted) {
        executionFinished.cancel()
      }
      throw startError
    } finally {
      activeTeamV4ExecutionIds.delete(input.executionId)
    }

    if (!finished.success) {
      throw new Error(finished.error || finished.message || `${input.roleLabel} model execution failed.`)
    }
    const response = (finished.response || finished.message || '').trim()
    if (!response) {
      throw new Error(`${input.roleLabel} model execution returned an empty response.`)
    }
    return response
  }

  const hydrateTaskHistorySafely = async (conversationId: string) => {
    try {
      await params.hydrateTaskHistory(conversationId)
    } catch (error) {
      console.warn(
        `[useAgentConversationFlow] Failed to hydrate task history for ${conversationId}:`,
        error,
      )
    }
  }

  const recordTeamV4MonitorMemory = async (input: {
    event: TeamV4Event
    eventType: 'specialist_execution_completed' | 'specialist_execution_failed'
    goal: string
    result?: unknown
    error?: string
    specialistId: string
    taskId: string
    taskTitle?: string
    teamRun: TeamV4RunBootstrap
  }) => {
    const signals = extractMonitorSignals({
      eventType: input.eventType,
      result: input.result,
      error: input.error,
    })
    const monitorContextMode = resolveTeamV4ContextMode(input.teamRun.monitor.context_mode)
    const monitorModel = input.teamRun.monitor.model?.trim() || null
    const monitorContextSnapshot = await teamRuntimeApi.createContextSnapshot(input.teamRun.run.id, {
      actorId: input.teamRun.monitor.id,
      taskId: input.taskId,
      roleType: 'monitor',
      sourceSequence: input.event.sequence,
      policyJson: {
        reviewMode: 'model_level_structured_review',
        sourceEventType: input.event.event_type,
      },
      sectionsJson: [
        {
          id: 'goal',
          title: 'User Goal',
          content: input.goal,
        },
        {
          id: 'source_event',
          title: 'Source Event',
          content: {
            eventId: input.event.id,
            eventType: input.event.event_type,
            payload: input.event.payload,
          },
        },
        {
          id: 'rule_signals',
          title: 'Rule Signals',
          content: signals,
        },
      ],
      tokenEstimate: 0,
    })
    const monitorExecutionId = [
      'team-v4',
      input.teamRun.run.id,
      input.taskId,
      'monitor-review',
      input.event.sequence,
    ].join(':')
    const monitorReviewRaw = await runTeamV4ModelExecution({
      contextMode: monitorContextMode,
      executionId: monitorExecutionId,
      model: monitorModel,
      prompt: buildTeamV4MonitorReviewPrompt({
        eventType: input.eventType,
        goal: input.goal,
        taskTitle: input.taskTitle,
        result: input.result,
        error: input.error,
      }),
      roleLabel: 'Monitor review',
    })
    const monitorReview = parseTeamV4MonitorReview(monitorReviewRaw)
    const reviewEvent = await teamRuntimeApi.appendEvent(input.teamRun.run.id, {
      actorId: input.teamRun.monitor.id,
      taskId: input.taskId,
      eventType: 'monitor_model_review_completed',
      visibility: 'workspace',
      payload: {
        contextSnapshotId: monitorContextSnapshot.id,
        executionId: monitorExecutionId,
        review: monitorReview,
        sourceEventId: input.event.id,
        sourceEventType: input.event.event_type,
      },
    })
    const baseContent = buildMonitorMemoryContent({
      eventType: input.eventType,
      goal: input.goal,
      taskTitle: input.taskTitle,
      result: input.result,
      error: input.error,
    })
    const content = [
      `Monitor summary: ${monitorReview.summary}`,
      baseContent,
      monitorReview.evidence.length
        ? `Evidence:\n${monitorReview.evidence.map((item, index) => `${index + 1}. ${item}`).join('\n')}`
        : '',
      monitorReview.riskSignals.length
        ? `Risk signals:\n${monitorReview.riskSignals.map((item, index) => `${index + 1}. ${item}`).join('\n')}`
        : '',
    ].filter(Boolean).join('\n\n')
    const memory = await teamRuntimeApi.createMemoryCandidate(input.teamRun.run.id, {
      taskId: input.taskId,
      kind: monitorReview.kind,
      content,
      confidence: monitorReview.confidence,
      sourceEventIds: [input.event.id, reviewEvent.id],
      metadata: {
        curationSchema: 'team_v4_monitor_model_v1',
        evidence: monitorReview.evidence,
        modelContextSnapshotId: monitorContextSnapshot.id,
        modelExecutionId: monitorExecutionId,
        modelReview: monitorReview,
        monitorId: input.teamRun.monitor.id,
        riskSignals: monitorReview.riskSignals,
        ruleSignals: signals,
        specialistId: input.specialistId,
        summary: monitorReview.summary,
        sourceEventType: input.event.event_type,
        taskTitle: input.taskTitle || null,
        gate: 'monitor_candidate',
      },
    })
    await teamRuntimeApi.acceptMemory(input.teamRun.run.id, memory.id, monitorReview.promoteToLongTerm)
  }

  const requestTeamV4OrchestratorRecoveryDecision = async (input: {
    assignment: TeamV4SpecialistAssignment
    attemptIndex: number
    error: string
    goal: string
    specialistResults: Array<{
      specialistId: string
      taskId: string
      result: {
        response: string
      }
    }>
    teamRun: TeamV4RunBootstrap
  }) => {
    const orchestratorContextMode = resolveTeamV4ContextMode(input.teamRun.orchestrator.context_mode)
    const orchestratorModel = input.teamRun.orchestrator.model?.trim() || null
    const completedProgress = buildTeamV4InheritedProgress(input.specialistResults)
    const contextSnapshot = await teamRuntimeApi.createContextSnapshot(input.teamRun.run.id, {
      actorId: input.teamRun.orchestrator.id,
      taskId: input.assignment.task.id,
      roleType: 'orchestrator',
      sourceSequence: null,
      policyJson: {
        recoveryMode: 'model_driven_retry_reassign',
        attemptIndex: input.attemptIndex,
      },
      sectionsJson: [
        {
          id: 'goal',
          title: 'User Goal',
          content: input.goal,
        },
        {
          id: 'failed_assignment',
          title: 'Failed Assignment',
          content: {
            specialistId: input.assignment.specialist.id,
            specialistName: input.assignment.specialist.name,
            taskId: input.assignment.task.id,
            taskTitle: input.assignment.task.title,
            instruction: input.assignment.task.instruction,
            error: input.error,
          },
        },
        {
          id: 'completed_progress',
          title: 'Completed Progress',
          content: completedProgress,
        },
      ],
      tokenEstimate: 0,
    })
    const executionId = [
      'team-v4',
      input.teamRun.run.id,
      input.assignment.task.id,
      'orchestrator-replan',
      input.attemptIndex,
    ].join(':')
    const availableSpecialists = input.teamRun.specialists.map((specialist) => ({
      id: specialist.id,
      name: specialist.name,
      model: specialist.model,
      contextMode: specialist.context_mode,
    }))
    const rawDecision = await runTeamV4ModelExecution({
      contextMode: orchestratorContextMode,
      executionId,
      model: orchestratorModel,
      prompt: buildTeamV4OrchestratorRecoveryPrompt({
        assignment: input.assignment,
        attemptIndex: input.attemptIndex,
        availableSpecialists,
        completedProgress,
        error: input.error,
        goal: input.goal,
      }),
      roleLabel: 'Orchestrator recovery',
    })
    const decision = parseTeamV4OrchestratorRecoveryDecision(
      rawDecision,
      new Set(input.teamRun.specialists.map((specialist) => specialist.id)),
    )
    await teamRuntimeApi.appendEvent(input.teamRun.run.id, {
      actorId: input.teamRun.orchestrator.id,
      taskId: input.assignment.task.id,
      eventType: 'orchestrator_replan_completed',
      visibility: 'workspace',
      payload: {
        contextSnapshotId: contextSnapshot.id,
        decision,
        error: input.error,
        executionId,
        failedSpecialistId: input.assignment.specialist.id,
        failedTaskId: input.assignment.task.id,
      },
    })
    return decision
  }

  const applyReplaySnapshot = (message: AgentMessage) => {
    const snapshot = buildMessageReplaySnapshot({
      message,
      messages: params.agentMessages.value,
      subagents: params.agentSubagents.value,
    })
    if (!snapshot) {
      console.error('[useAgentConversationFlow] Message not found')
      return null
    }
    params.agentMessages.value = snapshot.messagesToKeep
    params.agentSubagents.value = snapshot.subagentsToKeep
    return snapshot
  }

  const deleteConversationTailForMessageReplay = async (
    message: AgentMessage,
    messageTimestamp: number,
    logLabel: string,
  ) => {
    await deleteConversationTailForReplay({
      conversationId: params.conversationId.value,
      deleteAiMessage: async (messageId) => {
        await invoke('delete_ai_message', { messageId })
      },
      deleteAiMessagesAfter: async (conversationId, messageId) =>
        invoke<number>('delete_ai_messages_after', { conversationId, messageId }),
      deleteSubagentRunsAfter: async (parentExecutionId, afterTimestampMs) =>
        invoke<number>('delete_subagent_runs_after', { parentExecutionId, afterTimestampMs }),
      logLabel,
      messageId: message.id,
      messageTimestamp,
    })
  }

  const handleClearConversation = async () => {
    try {
      if (!params.conversationId.value) {
        console.log('[useAgentConversationFlow] No conversation to clear')
        return
      }
      const cleared = await clearConversationSession({
        clearConversationMessages: async (conversationId) => {
          await invoke('clear_conversation_messages', { conversationId })
        },
        conversationId: params.conversationId.value,
        loadConversationList: params.loadConversationList,
        onConversationCleared: () => {
          params.clearAgentMessages()
          params.conversationExecutionState.value = null
          params.clearDraftArtifacts()
          params.inputValue.value = ''
        },
      })
      if (cleared) {
        console.log('[useAgentConversationFlow] Conversation cleared successfully')
      }
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to clear conversation:', error)
      params.localError.value = `${params.getFailedToClearConversationLabel()}: ${error}`
    }
  }

  const handleStop = async () => {
    console.log('[useAgentConversationFlow] Stop requested for conversation:', params.conversationId.value)

    if (params.teamModeEnabled.value && params.activeTeamV4RunId.value) {
      try {
        const executionIds = Array.from(activeTeamV4ExecutionIds)
        for (const executionId of executionIds) {
          await invoke('cancel_ai_stream', {
            conversationId: executionId,
          })
        }
        if (params.conversationId.value && executionIds.length === 0) {
          await invoke('cancel_ai_stream', {
            conversationId: params.conversationId.value,
          })
        }
        await params.stopTeamV4Run()
        params.stopAgentExecutionState()
      } catch (error) {
        console.error('[useAgentConversationFlow] Failed to stop Team v4 execution:', error)
        params.localError.value = `${params.getFailedToStopExecutionLabel()}: ${error}`
      }
      return
    }

    if (params.teamModeEnabled.value && params.activeTeamSessionId.value) {
      try {
        await agentTeamApi.stopRun(params.activeTeamSessionId.value)
        params.handleStopTeamState('FAILED')
      } catch (error) {
        console.error('[useAgentConversationFlow] Failed to stop team execution:', error)
        params.localError.value = `${params.getFailedToStopExecutionLabel()}: ${error}`
      }
      return
    }

    if (!params.conversationId.value) {
      console.warn('[useAgentConversationFlow] No conversation ID to stop')
      return
    }

    try {
      await invoke('cancel_ai_stream', {
        conversationId: params.conversationId.value,
      })
      params.stopAgentExecutionState()
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to stop execution:', error)
      params.localError.value = `${params.getFailedToStopExecutionLabel()}: ${error}`
    }
  }

  const handleResendMessage = async (message: AgentMessage) => {
    if (params.isExecuting.value) return
    const snapshot = applyReplaySnapshot(message)
    if (!snapshot) return
    await deleteConversationTailForMessageReplay(message, snapshot.messageTimestamp, 'original')
    params.restoreArtifactsFromMessage(message)
    params.clearTasksForCurrentContext()
    params.inputValue.value = params.teamModeEnabled.value
      ? normalizeTeamHumanInputContent(message.content)
      : message.content
    await handleSubmit()
  }

  const handleEditMessage = async (message: AgentMessage, newContent: string) => {
    if (params.isExecuting.value) return
    const snapshot = applyReplaySnapshot(message)
    if (!snapshot) return
    await deleteConversationTailForMessageReplay(message, snapshot.messageTimestamp, 'edited')
    params.restoreArtifactsFromMessage(message)
    params.clearTasksForCurrentContext()
    params.inputValue.value = params.teamModeEnabled.value
      ? normalizeTeamHumanInputContent(newContent)
      : newContent
    await handleSubmit()
  }

  const loadConversationHistory = async (conversationId: string) => {
    const currentLoadToken = ++params.historyLoadToken.value
    params.isHistoryLoading.value = true

    try {
      await loadConversationHistorySupport({
        buildToolCallCompletedLabel: params.getToolCallCompletedLabel,
        clearMessages: params.clearAgentMessages,
        conversationId,
        getConversation: (targetConversationId) =>
          invoke<AiConversationDetail | null>('get_ai_conversation', {
            conversationId: targetConversationId,
          }),
        getMessages: (targetConversationId) =>
          invoke<PersistedConversationMessageRow[]>('get_ai_messages_by_conversation', {
            conversationId: targetConversationId,
          }),
        isStale: () => currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId,
        loadSubagentRuns: async () => {
          await params.loadSubagentRuns(conversationId, currentLoadToken)
        },
        log: (message, ...args) => {
          console.log(message, ...args)
        },
        onConversationLoaded: ({ executionState, title }) => {
          params.currentConversationTitle.value = title
          params.conversationExecutionState.value = executionState || null
        },
        onEmptyHistoryLoaded: async () => {
          await hydrateTaskHistorySafely(conversationId)
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          params.setMirroredConversationMessageIds(new Set())
          await params.syncActiveTeamSession()
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          if (params.activeTeamSessionId.value) {
            await params.syncTeamMessagesToMainFlow(params.activeTeamSessionId.value)
            if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          }
        },
        onMessagesLoaded: async ({ messageCount, mirroredConversationMessageIds, timeline }) => {
          params.agentMessages.value = timeline
          await hydrateTaskHistorySafely(conversationId)
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          params.setMirroredConversationMessageIds(mirroredConversationMessageIds)
          await params.syncActiveTeamSession()
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          if (params.activeTeamSessionId.value) {
            await params.syncTeamMessagesToMainFlow(params.activeTeamSessionId.value)
            if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          }
          console.log('[useAgentConversationFlow] Loaded', messageCount, 'messages from conversation:', conversationId)
          nextTick(() => {
            params.scrollMessageViewportToBottom()
          })
        },
        onLoadFailed: (error) => {
          console.error('[useAgentConversationFlow] Failed to load conversation history:', error)
        },
        shouldSuppressTeamMirrorNoiseMessage,
        unnamedConversationTitle: params.getUnnamedConversationTitle(),
      })
    } finally {
      if (currentLoadToken === params.historyLoadToken.value) {
        params.isHistoryLoading.value = false
      }
    }
  }

  const handleSelectConversation = async (conversationId: string) => {
    await selectConversationSession({
      closeConversationDrawer: params.closeConversationDrawer,
      conversationId,
      isStillActive: (currentId) => params.conversationId.value === currentId,
      loadConversationHistory,
      onConversationSelected: (selectedId) => {
        params.conversationId.value = selectedId
      },
      resetTerminal: params.resetTerminal,
    })
  }

  const handleCreateConversation = async (newConversationId?: string) => {
    if (newConversationId) {
      await handleSelectConversation(newConversationId)
      return
    }

    try {
      await createConversationSession({
        createConversation: async (request) =>
          invoke<string>('create_ai_conversation', { request }),
        getConversationTitle: params.getNewConversationTitle,
        getDisplayTitle: params.getUnnamedConversationTitle,
        loadConversationList: params.loadConversationList,
        onConversationCreated: (conversationId, title) => {
          params.conversationId.value = conversationId
          params.currentConversationTitle.value = title
          params.conversationExecutionState.value = null
          params.clearAgentMessages()
          params.resetTerminal()
        },
      })
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to create conversation:', error)
    }
  }

  const handleSubmit = async () => {
    if (params.submitInFlight.value) return
    const task = params.inputValue.value.trim()
    if (!task) return

    params.submitInFlight.value = true
    params.localError.value = null

    try {
      if (params.teamModeEnabled.value) {
        try {
          if (params.isExecuting.value && params.conversationId.value) {
            await handleStop()
            await new Promise((resolve) => setTimeout(resolve, 300))
          }

          const {
            displayContent,
            fullTask,
            usedAssets,
            usedAttachments,
            usedDocuments,
            usedFiles,
            usedMessages,
            usedTraffic,
          } = prepareSubmission({
            clearDraftState: () => {
              params.inputValue.value = ''
              params.clearDraftArtifacts()
            },
            pendingAttachments: params.pendingAttachments.value,
            processedDocuments: params.processedDocuments.value,
            referencedAssets: params.referencedAssets.value,
            referencedFiles: params.referencedFiles.value,
            referencedMessages: params.referencedMessages.value,
            referencedTraffic: params.referencedTraffic.value,
            setPendingDocumentAttachments: (documents) => {
              params.setPendingDocumentAttachments(documents)
            },
            task,
            toAssetContextItems: (assets) => assets,
            toFileContextItems: (files) => files,
            toMessageContextItems: (messages) => messages,
            toTrafficContextItems: (traffic) => traffic,
          })

          const teamVisionError = getUnsupportedVisionModelError(usedAttachments)
          if (teamVisionError) {
            params.localError.value = teamVisionError
            params.emitError(teamVisionError)
            return
          }

          nextTick(() => {
            params.scrollMessageViewportToBottom()
          })

          params.emitSubmit(fullTask)
          const ensuredConversationId = await ensureConversationForExecutionSupport({
            conversationId: params.conversationId.value,
            createConversation: async (request) =>
              invoke<string>('create_ai_conversation', { request }),
            getConversationTitle: params.getNewConversationTitle,
            getDisplayTitle: params.getUnnamedConversationTitle,
            loadConversationList: params.loadConversationList,
            onConversationReady: (conversationId, title) => {
              params.conversationId.value = conversationId
              params.currentConversationTitle.value = title
              params.conversationExecutionState.value = null
            },
          })
          if (!ensuredConversationId) {
            throw new Error('Conversation ID is required for Team v4 execution.')
          }

          const teamRun = await params.startTeamV4AssistantRun(fullTask)
          const teamToolPolicyMatrix = readTeamV4ToolPolicyMatrix(teamRun)
          const assignments = await planTeamV4SpecialistAssignments({
            baselineToolConfig: params.buildToolConfig(),
            getAssistantProfileOption: params.getAssistantProfileOption,
            goal: fullTask,
            runModelExecution: runTeamV4ModelExecution,
            teamRun,
            teamToolPolicyMatrix,
            webSearchEnabled: params.webSearchEnabled.value,
          })
          if (!assignments.length) {
            throw new Error('Team v4 requires at least one specialist assignment.')
          }
          const maxSpecialists = readTeamV4MaxSpecialists(teamRun, assignments.length)
          await teamRuntimeApi.appendEvent(teamRun.run.id, {
            actorId: teamRun.orchestrator.id,
            taskId: teamRun.rootTask.id,
            eventType: 'specialist_scheduler_started',
            visibility: 'workspace',
            payload: {
              assignmentCount: assignments.length,
              planningMode: 'orchestrator_task_graph',
              maxSpecialists,
            },
          })
          appendTeamV4LocalMessage(
            params.agentMessages,
            `Orchestrator planned ${assignments.length} task(s) and started Specialist scheduling, max ${maxSpecialists} concurrent Specialist(s).`,
            {
              kind: 'team_v4_scheduler_started',
              team_member_id: teamRun.orchestrator.id,
              team_member_name: teamRun.orchestrator.name,
              team_member_role: 'orchestrator',
              team_session_id: teamRun.run.id,
              team_task_record_id: teamRun.rootTask.id,
            },
          )
          await params.refreshTeamV4Workspace?.()
          const specialistResults: Array<{
            specialistId: string
            taskId: string
            result: {
              messageId: unknown
              outcome: AgentExecutionFinishedEvent['outcome']
              response: string
            }
          }> = []

          const runSpecialistAssignment = async (
            assignment: TeamV4SpecialistAssignment,
            assignmentIndex: number,
          ) => {
            const runSpecialistAttempt = async (
              effectiveAssignment: TeamV4SpecialistAssignment,
              attemptIndex: number,
              recoveryInstruction?: string | null,
            ) => {
              const inheritedProgress = buildTeamV4InheritedProgress(specialistResults)
              const inheritanceSnapshot = await teamRuntimeApi.createContextSnapshot(teamRun.run.id, {
                actorId: effectiveAssignment.specialist.id,
                taskId: effectiveAssignment.task.id,
                roleType: 'specialist',
                sourceSequence: teamRun.events[teamRun.events.length - 1]?.sequence ?? null,
                policyJson: {
                  inheritHistory: true,
                  inheritanceMode: 'execution_time_completed_specialist_progress',
                  assignmentIndex,
                  attemptIndex,
                  recoveryInstruction: recoveryInstruction || null,
                },
                sectionsJson: [
                  {
                    id: 'goal',
                    title: 'User Goal',
                    content: fullTask,
                  },
                  {
                    id: 'assignment',
                    title: 'Orchestrator Assignment',
                    content: {
                      specialistId: effectiveAssignment.specialist.id,
                      taskId: effectiveAssignment.task.id,
                      taskKey: effectiveAssignment.task.task_key,
                      title: effectiveAssignment.task.title,
                      instruction: recoveryInstruction || effectiveAssignment.task.instruction,
                      originalInstruction: effectiveAssignment.task.instruction,
                    },
                  },
                  {
                    id: 'inherited_progress',
                    title: 'Inherited Progress',
                    content: inheritedProgress,
                  },
                ],
                tokenEstimate: 0,
              })
              const specialistExecutionId = buildTeamV4SpecialistExecutionId(
                teamRun.run.id,
                effectiveAssignment.task.id,
                effectiveAssignment.specialist.id,
                attemptIndex,
              )
              const specialistContextMode = resolveTeamV4ContextMode(effectiveAssignment.specialist.context_mode)
              const specialistModel = effectiveAssignment.specialist.model?.trim() || null
              const specialistProfileId = effectiveAssignment.specialist.profile_id?.trim()
              if (!specialistProfileId) {
                throw new Error(`Team v4 specialist ${effectiveAssignment.specialist.id} is missing profile_id.`)
              }
              const specialistProfile = params.getAssistantProfileOption(specialistProfileId)
              if (!specialistProfile) {
                throw new Error(`Team v4 specialist profile not found: ${specialistProfileId}.`)
              }
              const specialistProfileToolConfig = buildTeamV4SpecialistProfileToolConfig(
                specialistProfile,
                params.buildToolConfig(),
              )
              const specialistRuntimeToolConfig = buildRuntimeToolConfigForTeamRole(
                specialistProfileToolConfig,
                teamToolPolicyMatrix,
                'specialist',
                {
                  webSearchEnabled: params.webSearchEnabled.value,
                },
              )
              const startedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                actorId: effectiveAssignment.specialist.id,
                taskId: effectiveAssignment.task.id,
                eventType: 'specialist_execution_started',
                visibility: 'workspace',
                payload: {
                  model: specialistModel,
                  contextMode: specialistContextMode,
                  executionId: specialistExecutionId,
                  assignmentIndex,
                  attemptIndex,
                  recoveryInstruction: recoveryInstruction || null,
                  inheritanceSnapshotId: inheritanceSnapshot.id,
                  inheritedProgressCount: inheritedProgress.length,
                  taskKey: effectiveAssignment.task.task_key,
                  toolPolicy: {
                    profileId: specialistProfileId,
                    selectionStrategy: specialistRuntimeToolConfig.selection_strategy,
                    maxTools: specialistRuntimeToolConfig.max_tools,
                    fixedTools: specialistRuntimeToolConfig.fixed_tools,
                    disabledTools: specialistRuntimeToolConfig.disabled_tools,
                    allowedTools: specialistRuntimeToolConfig.allowed_tools,
                  },
                },
              })
              appendTeamV4LocalMessage(
                params.agentMessages,
                `Specialist started: ${effectiveAssignment.specialist.name} -> ${effectiveAssignment.task.title}`,
                {
                  kind: 'team_v4_specialist_started',
                  team_member_id: effectiveAssignment.specialist.id,
                  team_member_name: effectiveAssignment.specialist.name,
                  team_member_role: 'specialist',
                  team_session_id: teamRun.run.id,
                  team_task_record_id: effectiveAssignment.task.id,
                  team_task_key: effectiveAssignment.task.task_key,
                  team_sequence: startedEvent.sequence,
                },
              )
              const specialistStartedAt = Date.now()
              const specialistProgressMessageId = crypto.randomUUID()
              const specialistProgressPrefix =
                `${effectiveAssignment.specialist.name} -> ${effectiveAssignment.task.title}`
              const updateSpecialistProgressMessage = (status: 'running' | 'completed' | 'failed' | 'cancelled') => {
                const elapsedSeconds = Math.max(0, Math.round((Date.now() - specialistStartedAt) / 1000))
                const statusLabel = status === 'running'
                  ? `Specialist running: ${specialistProgressPrefix} (${elapsedSeconds}s)`
                  : `Specialist ${status}: ${specialistProgressPrefix} (${elapsedSeconds}s)`
                const existing = params.agentMessages.value.find((item) => item.id === specialistProgressMessageId)
                if (existing) {
                  existing.content = statusLabel
                  existing.timestamp = Date.now()
                  existing.metadata = {
                    ...existing.metadata,
                    status,
                    duration_ms: elapsedSeconds * 1000,
                  }
                  return
                }
                params.agentMessages.value.push({
                  id: specialistProgressMessageId,
                  type: 'planning',
                  content: statusLabel,
                  timestamp: Date.now(),
                  metadata: {
                    kind: 'team_v4_specialist_progress',
                    status,
                    duration_ms: elapsedSeconds * 1000,
                    team_member_id: effectiveAssignment.specialist.id,
                    team_member_name: effectiveAssignment.specialist.name,
                    team_member_role: 'specialist',
                    team_session_id: teamRun.run.id,
                    team_task_record_id: effectiveAssignment.task.id,
                    team_task_key: effectiveAssignment.task.task_key,
                  },
                })
              }
              updateSpecialistProgressMessage('running')
              const specialistActivityProgress = startTeamV4SpecialistActivityProgress({
                executionId: specialistExecutionId,
                messages: params.agentMessages,
                runId: teamRun.run.id,
                specialistId: effectiveAssignment.specialist.id,
                specialistName: effectiveAssignment.specialist.name,
                taskId: effectiveAssignment.task.id,
                taskKey: effectiveAssignment.task.task_key,
                taskTitle: effectiveAssignment.task.title,
                scrollToBottom: params.scrollMessageViewportToBottom,
              })
              await teamRuntimeApi.checkpointHarnessRun(
                effectiveAssignment.harnessRun.id,
                startedEvent.sequence,
              )
              await params.refreshTeamV4Workspace?.()
              const heartbeatTimer = window.setInterval(() => {
                void teamRuntimeApi
                  .heartbeatHarnessRun(effectiveAssignment.harnessRun.id, 600)
                  .then(() => {
                    updateSpecialistProgressMessage('running')
                    return params.refreshTeamV4Workspace?.()
                  })
                  .catch((error) => {
                    console.warn(
                      '[useAgentConversationFlow] Team v4 harness heartbeat failed:',
                      error,
                    )
                  })
              }, 30_000)

              let specialistFinishedSuccessfully = false
              try {
                const specialistTaskPrompt = buildTeamV4SpecialistTaskPrompt(
                  effectiveAssignment,
                  fullTask,
                  assignments.length,
                  inheritedProgress,
                  recoveryInstruction,
                )
                const executionFinished = watchAgentExecutionFinished(specialistExecutionId, 360_000)
                let messageId: unknown
                try {
                  activeTeamV4ExecutionIds.add(specialistExecutionId)
                  messageId = await executeConversationTaskSupport({
                    assistantContextMode: specialistContextMode,
                    assistantSelectedModel: specialistModel,
                    conversationId: specialistExecutionId,
                    defaultConversationTitle: params.getUnnamedConversationTitle(),
                    displayContent,
                    enableRag: params.ragEnabled.value,
                    enableTenthManRule: params.tenthManEnabled.value,
                    firstMessage: task,
                    forceTasks: params.forceTasks,
                    fullTask: specialistTaskPrompt,
                    maybeAutoRenameConversation: (renameParams) => {
                      void maybeAutoRenameConversationByFirstMessage(renameParams)
                    },
                    onConversationListRefresh: params.loadConversationList,
                    onCurrentConversationTitleChange: (title) => {
                      params.currentConversationTitle.value = title
                    },
                    persistMessages: false,
                    runAgentExecute: (request) => invoke('agent_execute', request),
                    runtimeToolConfig: specialistRuntimeToolConfig,
                    skipAutoRename: true,
                    usedAssets,
                    usedAttachments,
                    usedDocuments,
                    usedFiles,
                    usedMessages,
                    usedTraffic,
                  })
                } catch (startError) {
                  executionFinished.cancel()
                  throw startError
                }
                const finished = await executionFinished.promise
                if (!finished.success) {
                  throw new Error(finished.error || finished.message || 'Specialist execution failed.')
                }
                specialistFinishedSuccessfully = true
                const result = {
                  messageId,
                  outcome: finished.outcome,
                  response: finished.response || finished.message || '',
                }
                activeTeamV4ExecutionIds.delete(specialistExecutionId)
                const completedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: effectiveAssignment.specialist.id,
                  taskId: effectiveAssignment.task.id,
                  eventType: 'specialist_execution_completed',
                  visibility: 'user',
                  payload: {
                    result,
                    executionId: specialistExecutionId,
                    attemptIndex,
                    taskKey: effectiveAssignment.task.task_key,
                  },
                })
                await teamRuntimeApi.checkpointHarnessRun(
                  effectiveAssignment.harnessRun.id,
                  completedEvent.sequence,
                )
                await recordTeamV4MonitorMemory({
                  event: completedEvent,
                  eventType: 'specialist_execution_completed',
                  goal: fullTask,
                  result,
                  specialistId: effectiveAssignment.specialist.id,
                  taskId: effectiveAssignment.task.id,
                  taskTitle: effectiveAssignment.task.title,
                  teamRun,
                })
                updateSpecialistProgressMessage('completed')
                specialistActivityProgress.complete()
                await params.refreshTeamV4Workspace?.()
                return {
                  specialistId: effectiveAssignment.specialist.id,
                  taskId: effectiveAssignment.task.id,
                  result,
                }
              } catch (specialistError: any) {
                activeTeamV4ExecutionIds.delete(specialistExecutionId)
                const specialistErrorMsg = specialistError?.toString?.() || String(specialistError)
                if (isTeamV4CancellationError(specialistError)) {
                  await teamRuntimeApi.updateRunState(teamRun.run.id, 'cancelled')
                  updateSpecialistProgressMessage('cancelled')
                  specialistActivityProgress.fail('Execution cancelled by user')
                  await params.refreshTeamV4Workspace?.()
                  throw specialistError
                }
                if (specialistFinishedSuccessfully) {
                  await teamRuntimeApi.appendEvent(teamRun.run.id, {
                    actorId: teamRun.monitor.id,
                    taskId: effectiveAssignment.task.id,
                    eventType: 'monitor_model_review_failed',
                    visibility: 'user',
                    payload: {
                      error: specialistErrorMsg,
                      executionId: specialistExecutionId,
                      attemptIndex,
                      specialistId: effectiveAssignment.specialist.id,
                      taskKey: effectiveAssignment.task.task_key,
                    },
                  })
                  await params.refreshTeamV4Workspace?.()
                  throw specialistError
                }
                const failedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: effectiveAssignment.specialist.id,
                  taskId: effectiveAssignment.task.id,
                  eventType: 'specialist_execution_failed',
                  visibility: 'user',
                  payload: {
                    error: specialistErrorMsg,
                    executionId: specialistExecutionId,
                    attemptIndex,
                    taskKey: effectiveAssignment.task.task_key,
                  },
                })
                await teamRuntimeApi.checkpointHarnessRun(
                  effectiveAssignment.harnessRun.id,
                  failedEvent.sequence,
                )
                await recordTeamV4MonitorMemory({
                  error: specialistErrorMsg,
                  event: failedEvent,
                  eventType: 'specialist_execution_failed',
                  goal: fullTask,
                  specialistId: effectiveAssignment.specialist.id,
                  taskId: effectiveAssignment.task.id,
                  taskTitle: effectiveAssignment.task.title,
                  teamRun,
                })
                updateSpecialistProgressMessage('failed')
                specialistActivityProgress.fail(specialistErrorMsg)
                await params.refreshTeamV4Workspace?.()
                throw specialistError
              } finally {
                window.clearInterval(heartbeatTimer)
                specialistActivityProgress.dispose()
              }
            }

            try {
              return await runSpecialistAttempt(assignment, 0)
            } catch (firstError: any) {
              if (isTeamV4CancellationError(firstError)) {
                throw firstError
              }
              const firstErrorMsg = firstError?.toString?.() || String(firstError)
              let decision: TeamV4OrchestratorRecoveryDecision
              try {
                decision = await requestTeamV4OrchestratorRecoveryDecision({
                  assignment,
                  attemptIndex: 0,
                  error: firstErrorMsg,
                  goal: fullTask,
                  specialistResults,
                  teamRun,
                })
              } catch (replanError: any) {
                const replanErrorMsg = replanError?.toString?.() || String(replanError)
                await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: teamRun.orchestrator.id,
                  taskId: assignment.task.id,
                  eventType: 'orchestrator_recovery_decision',
                  visibility: 'user',
                  payload: {
                    decision: 'cancel_after_replan_failure',
                    failedSpecialistId: assignment.specialist.id,
                    failedTaskId: assignment.task.id,
                    reason: replanErrorMsg,
                    originalError: firstErrorMsg,
                  },
                })
                await teamRuntimeApi.updateRunState(teamRun.run.id, 'failed')
                await params.refreshTeamV4Workspace?.()
                throw replanError
              }

              await teamRuntimeApi.appendEvent(teamRun.run.id, {
                actorId: teamRun.orchestrator.id,
                taskId: assignment.task.id,
                eventType: 'orchestrator_recovery_decision',
                visibility: 'user',
                payload: {
                  decision: decision.action,
                  failedSpecialistId: assignment.specialist.id,
                  failedTaskId: assignment.task.id,
                  modelReason: decision.reason,
                  revisedInstruction: decision.revisedInstruction || null,
                  targetSpecialistId: decision.targetSpecialistId || null,
                },
              })

              if (decision.action === 'ask_user') {
                await teamRuntimeApi.updateRunState(teamRun.run.id, 'waiting_human')
                await params.refreshTeamV4Workspace?.()
                throw new Error(`Team v4 requires user input: ${decision.reason}`)
              }

              if (decision.action === 'cancel') {
                await teamRuntimeApi.updateRunState(teamRun.run.id, 'failed')
                await params.refreshTeamV4Workspace?.()
                throw firstError
              }

              if (!decision.revisedInstruction) {
                await teamRuntimeApi.updateRunState(teamRun.run.id, 'failed')
                await params.refreshTeamV4Workspace?.()
                throw new Error(`Orchestrator ${decision.action} decision requires revisedInstruction.`)
              }

              const recoveryAssignment = decision.action === 'reassign'
                ? {
                    ...assignment,
                    specialist: teamRun.specialists.find((specialist) => specialist.id === decision.targetSpecialistId)!,
                  }
                : assignment

              try {
                return await runSpecialistAttempt(recoveryAssignment, 1, decision.revisedInstruction)
              } catch (recoveryError: any) {
                await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: teamRun.orchestrator.id,
                  taskId: assignment.task.id,
                  eventType: 'orchestrator_recovery_decision',
                  visibility: 'user',
                  payload: {
                    decision: 'cancel_after_recovery_failure',
                    failedSpecialistId: recoveryAssignment.specialist.id,
                    failedTaskId: recoveryAssignment.task.id,
                    reason: recoveryError?.toString?.() || String(recoveryError),
                    previousDecision: decision,
                  },
                })
                await teamRuntimeApi.updateRunState(teamRun.run.id, 'failed')
                await params.refreshTeamV4Workspace?.()
                throw recoveryError
              }
            }
          }

          specialistResults.push(
            ...await runTeamV4AssignmentsWithDependencies(assignments, maxSpecialists, runSpecialistAssignment),
          )
          await teamRuntimeApi.appendEvent(teamRun.run.id, {
            actorId: teamRun.orchestrator.id,
            taskId: teamRun.rootTask.id,
            eventType: 'specialist_scheduler_completed',
            visibility: 'workspace',
            payload: {
              completedAssignments: specialistResults.length,
              maxSpecialists,
            },
          })

          await teamRuntimeApi.updateRunState(teamRun.run.id, 'completed')
          await params.refreshTeamV4Workspace?.()
          const orchestratorFinalMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'final' as any,
            content: buildTeamV4OrchestratorFinalContent(specialistResults),
            timestamp: Date.now(),
            metadata: {
              kind: 'team_v4_orchestrator_final',
              team_member_id: teamRun.orchestrator.id,
              team_member_name: teamRun.orchestrator.name,
              team_member_role: 'orchestrator',
              team_session_id: teamRun.run.id,
              team_task_record_id: teamRun.rootTask.id,
            },
          }
          params.agentMessages.value.push(orchestratorFinalMessage)
          await invoke('save_ai_message', {
            request: {
              id: orchestratorFinalMessage.id,
              conversation_id: ensuredConversationId,
              role: 'assistant',
              content: orchestratorFinalMessage.content,
              metadata: orchestratorFinalMessage.metadata ?? null,
              architecture_type: 'team_v4',
              architecture_meta: JSON.stringify({
                team_run_id: teamRun.run.id,
                role: 'orchestrator',
              }),
              structured_data: null,
            },
          })
          nextTick(() => {
            params.scrollMessageViewportToBottom()
          })
          params.emitComplete({
            mode: 'team',
            architecture_version: 'team_v4',
            run_id: teamRun.run.id,
            root_task_id: teamRun.rootTask.id,
            execution_id: ensuredConversationId,
            result: {
              specialistResults,
            },
          })
        } catch (error: any) {
          const errorMsg = error?.toString?.() || String(error)
          const runId = params.activeTeamV4RunId.value
          if (isTeamV4CancellationError(error)) {
            if (runId) {
              try {
                await teamRuntimeApi.updateRunState(runId, 'cancelled')
                await params.refreshTeamV4Workspace?.()
              } catch (eventError) {
                console.warn('[useAgentConversationFlow] Failed to persist Team v4 cancellation:', eventError)
              }
            }
            appendTeamV4LocalMessage(
              params.agentMessages,
              'Team run cancelled by user.',
              {
                kind: 'team_v4_runtime_cancelled',
                team_session_id: runId || null,
              },
              'planning' as AgentMessage['type'],
            )
            params.stopAgentExecutionState()
            return
          }
          if (runId) {
            try {
              await teamRuntimeApi.appendEvent(runId, {
                actorId: null,
                taskId: null,
                eventType: 'team_runtime_failed',
                visibility: 'user',
                payload: {
                  error: errorMsg,
                },
              })
              await teamRuntimeApi.updateRunState(runId, 'failed')
              await params.refreshTeamV4Workspace?.()
            } catch (eventError) {
              console.warn('[useAgentConversationFlow] Failed to persist Team v4 failure:', eventError)
            }
          }
          appendTeamV4LocalMessage(
            params.agentMessages,
            `Team run failed before completion.\n\n${errorMsg}`,
            {
              kind: 'team_v4_runtime_error',
              team_session_id: runId || null,
            },
            'error' as AgentMessage['type'],
          )
          params.localError.value = errorMsg
          params.emitError(errorMsg)
        }
        return
      }

      await takeOverConversationExecutionSupport({
        appendPartialAssistantMessage: (message) => {
          params.agentMessages.value.push({
            id: message.id,
            type: 'final' as any,
            content: message.content,
            timestamp: message.timestamp,
          })
        },
        conversationId: params.conversationId.value,
        createMessageId: () => crypto.randomUUID(),
        currentTime: () => Date.now(),
        isExecuting: params.isExecuting.value,
        savePartialAssistantMessage: async (request) => {
          await invoke('save_ai_message', { request })
        },
        stopExecution: async () => {
          await handleStop()
        },
        streamingContent: params.agentStreamingContent.value,
        waitForStop: async () => {
          await new Promise((resolve) => setTimeout(resolve, 500))
        },
      })

      const {
        displayContent,
        fullTask,
        usedAssets,
        usedAttachments,
        usedDocuments,
        usedFiles,
        usedMessages,
        usedTraffic,
      } = prepareSubmission({
        clearDraftState: () => {
          params.inputValue.value = ''
          params.clearDraftArtifacts()
        },
        pendingAttachments: params.pendingAttachments.value,
        processedDocuments: params.processedDocuments.value,
        referencedAssets: params.referencedAssets.value,
        referencedFiles: params.referencedFiles.value,
        referencedMessages: params.referencedMessages.value,
        referencedTraffic: params.referencedTraffic.value,
        setPendingDocumentAttachments: (documents) => {
          params.setPendingDocumentAttachments(documents)
        },
        task,
        toAssetContextItems: (assets) => assets,
        toFileContextItems: (files) => files,
        toMessageContextItems: (messages) => messages,
        toTrafficContextItems: (traffic) => traffic,
      })

      const visionError = getUnsupportedVisionModelError(usedAttachments)
      if (visionError) {
        params.localError.value = visionError
        params.emitError(visionError)
        return
      }

      nextTick(() => {
        params.scrollMessageViewportToBottom()
      })
      params.emitSubmit(fullTask)

      try {
        const ensuredConversationId = await ensureConversationForExecutionSupport({
          conversationId: params.conversationId.value,
          createConversation: async (request) =>
            invoke<string>('create_ai_conversation', { request }),
          getConversationTitle: params.getNewConversationTitle,
          getDisplayTitle: params.getUnnamedConversationTitle,
          loadConversationList: params.loadConversationList,
          onConversationReady: (conversationId, title) => {
            params.conversationId.value = conversationId
            params.currentConversationTitle.value = title
            params.conversationExecutionState.value = null
          },
        })
        if (!ensuredConversationId) {
          throw new Error('Conversation ID is required for agent execution.')
        }

        const result = await executeConversationTaskSupport({
          assistantContextMode: params.assistantContextMode.value,
          assistantExecutionMode: params.assistantExecutionMode.value,
          assistantParallelJudgeModel: params.assistantParallelJudgeModel.value,
          assistantParallelSelectedModels: params.assistantParallelSelectedModels.value,
          assistantSelectedModel: params.assistantSelectedModel.value,
          conversationId: ensuredConversationId,
          defaultConversationTitle: params.getUnnamedConversationTitle(),
          displayContent,
          enableRag: params.ragEnabled.value,
          enableTenthManRule: params.tenthManEnabled.value,
          firstMessage: task,
          forceTasks: params.forceTasks,
          fullTask,
          maybeAutoRenameConversation: (renameParams) => {
            void maybeAutoRenameConversationByFirstMessage(renameParams)
          },
          onConversationListRefresh: params.loadConversationList,
          onCurrentConversationTitleChange: (title) => {
            params.currentConversationTitle.value = title
          },
          runAgentExecute: (request) => invoke('agent_execute', request),
          runAgentExecuteParallel: (request) => invoke('agent_execute_parallel', request),
          runtimeToolConfig: buildRuntimeToolConfigForExecution(params.buildToolConfig(), {
            webSearchEnabled: params.webSearchEnabled.value,
          }),
          usedAssets,
          usedAttachments,
          usedDocuments,
          usedFiles,
          usedMessages,
          usedTraffic,
        })

        params.emitComplete(result)
      } catch (error: any) {
        const errorMsg = error.toString()
        params.localError.value = errorMsg
        params.emitError(errorMsg)
      }
    } finally {
      params.submitInFlight.value = false
    }
  }

  const loadLatestConversation = async () => {
    try {
      const latest = await loadLatestConversationSession({
        currentConversationId: params.conversationId.value,
        getConversations: () => invoke<AiConversationSummary[]>('get_ai_conversations'),
        loadConversationHistory,
        onConversationLoaded: (conversation) => {
          params.conversationId.value = conversation.id
          params.currentConversationTitle.value = conversation.title || params.getUnnamedConversationTitle()
        },
      })
      if (latest) {
        console.log('[useAgentConversationFlow] Loaded latest conversation:', latest.id)
      }
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to load latest conversation:', error)
    }
  }

  return {
    handleClearConversation,
    handleConversationExecutionStateUpdate,
    handleCreateConversation,
    handleEditMessage,
    handleResendMessage,
    handleSelectConversation,
    handleStop,
    handleSubmit,
    loadConversationHistory,
    loadLatestConversation,
  }
}
