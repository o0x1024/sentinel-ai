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
import { planTeamV4SolverAssignments } from './teamV4CommanderPlanning'
import { startTeamV4SolverActivityProgress } from './teamV4SolverActivityProgress'
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
  TeamV4SolverAssignment,
} from '@/types/teamRuntime'

type TeamV4ObserverReview = {
  kind: TeamV4Memory['kind']
  summary: string
  evidence: string[]
  riskSignals: string[]
  confidence: number
  promoteToLongTerm: boolean
}

type TeamV4CommanderRecoveryDecision = {
  action: 'retry' | 'reassign' | 'ask_user' | 'cancel'
  reason: string
  revisedInstruction?: string | null
  targetSolverId?: string | null
}

const TEAM_V4_MEMORY_KINDS = new Set<TeamV4Memory['kind']>([
  'evidence',
  'decision',
  'risk',
  'blocker',
  'checkpoint',
  'artifact_summary',
])

const TEAM_V4_RECOVERY_ACTIONS = new Set<TeamV4CommanderRecoveryDecision['action']>([
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

const buildTeamV4ModelOnlyToolConfig = () => ({
  enabled: false,
  selection_strategy: { Manual: [] as string[] },
  max_tools: 1,
  fixed_tools: [] as string[],
  disabled_tools: [] as string[],
  allowed_tools: [] as string[],
})

const buildTeamV4SolverProfileToolConfig = (
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

const parseTeamV4ObserverReview = (raw: string): TeamV4ObserverReview => {
  const parsed = parseTeamV4JsonObject(raw, 'Observer review')
  const kind = parsed.kind
  if (typeof kind !== 'string' || !TEAM_V4_MEMORY_KINDS.has(kind as TeamV4Memory['kind'])) {
    throw new Error(`Observer review returned invalid kind: ${String(kind)}.`)
  }
  if (typeof parsed.summary !== 'string' || !parsed.summary.trim()) {
    throw new Error('Observer review requires a non-empty summary.')
  }
  const confidence = Number(parsed.confidence)
  if (!Number.isFinite(confidence) || confidence < 0 || confidence > 1) {
    throw new Error('Observer review confidence must be between 0 and 1.')
  }
  if (typeof parsed.promoteToLongTerm !== 'boolean') {
    throw new Error('Observer review requires boolean promoteToLongTerm.')
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

const parseTeamV4CommanderRecoveryDecision = (
  raw: string,
  availableSolverIds: Set<string>,
): TeamV4CommanderRecoveryDecision => {
  const parsed = parseTeamV4JsonObject(raw, 'Commander recovery decision')
  const action = parsed.action
  if (typeof action !== 'string' || !TEAM_V4_RECOVERY_ACTIONS.has(action as TeamV4CommanderRecoveryDecision['action'])) {
    throw new Error(`Commander recovery decision returned invalid action: ${String(action)}.`)
  }
  if (typeof parsed.reason !== 'string' || !parsed.reason.trim()) {
    throw new Error('Commander recovery decision requires a non-empty reason.')
  }
  const targetSolverId = typeof parsed.targetSolverId === 'string' && parsed.targetSolverId.trim()
    ? parsed.targetSolverId.trim()
    : null
  if (action === 'reassign' && (!targetSolverId || !availableSolverIds.has(targetSolverId))) {
    throw new Error(`Commander recovery decision returned invalid targetSolverId: ${String(targetSolverId)}.`)
  }
  return {
    action: action as TeamV4CommanderRecoveryDecision['action'],
    reason: parsed.reason.trim(),
    revisedInstruction: typeof parsed.revisedInstruction === 'string' && parsed.revisedInstruction.trim()
      ? parsed.revisedInstruction.trim()
      : null,
    targetSolverId,
  }
}

const buildObserverMemoryContent = (params: {
  eventType: 'solver_execution_completed' | 'solver_execution_failed'
  goal: string
  taskTitle?: string
  result?: unknown
  error?: string
}) => {
  if (params.eventType === 'solver_execution_failed') {
    return [
      `Solver failed while working on: ${params.taskTitle || params.goal}`,
      `Failure summary: ${stringifyForTeamMemory(params.error || 'unknown error', 1200)}`,
    ].join('\n')
  }
  return [
    `Solver completed task: ${params.taskTitle || params.goal}`,
    `Result summary: ${stringifyForTeamMemory(params.result, 1600)}`,
  ].join('\n')
}

const extractObserverSignals = (params: {
  eventType: 'solver_execution_completed' | 'solver_execution_failed'
  result?: unknown
  error?: string
}) => {
  const text = params.eventType === 'solver_execution_failed'
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
  const summary = lines[0] || (params.eventType === 'solver_execution_failed' ? 'Solver failed.' : 'Solver completed.')
  return {
    evidence,
    riskSignals,
    summary,
  }
}

const buildTeamV4SolverTaskPrompt = (
  assignment: TeamV4SolverAssignment,
  goal: string,
  assignmentCount: number,
  inheritedProgress: Array<{ taskId: string; solverId: string; summary: string }>,
  recoveryInstruction?: string | null,
) => {
  if (assignmentCount === 1 && inheritedProgress.length === 0 && !recoveryInstruction) return goal
  const lines = [
    `Team Solver Task: ${assignment.task.title}`,
    `User Goal: ${goal}`,
    `Assigned Solver: ${assignment.solver.name}`,
    `Task Instruction: ${recoveryInstruction || assignment.task.instruction}`,
    `Acceptance Criteria: ${assignment.task.acceptance_criteria || 'Commander review'}`,
  ]
  if (inheritedProgress.length > 0) {
    lines.push(`Inherited Progress:\n${inheritedProgress
      .map((item, index) => `${index + 1}. ${item.summary}`)
      .join('\n')}`)
  }
  return lines.join('\n\n')
}

const buildTeamV4ObserverReviewPrompt = (params: {
  eventType: 'solver_execution_completed' | 'solver_execution_failed'
  goal: string
  taskTitle?: string
  result?: unknown
  error?: string
}) => [
  'You are the Team Observer. Review the solver output as an independent quality gate.',
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
  `Solver result: ${stringifyForTeamMemory(params.result, 5000)}`,
  `Solver error: ${stringifyForTeamMemory(params.error || '', 2000)}`,
].join('\n')

const buildTeamV4CommanderRecoveryPrompt = (params: {
  goal: string
  assignment: TeamV4SolverAssignment
  attemptIndex: number
  error: string
  availableSolvers: Array<{ id: string; name: string; model?: string | null; contextMode?: string | null }>
  completedProgress: Array<{ taskId: string; solverId: string; summary: string }>
}) => [
  'You are the Team Commander. A solver attempt failed. Decide the next recovery action.',
  'Return strict JSON only. Do not include markdown or prose outside JSON.',
  '',
  'Allowed actions:',
  '- retry: run the same solver once more with a revised instruction.',
  '- reassign: run another available solver with a revised instruction.',
  '- ask_user: stop and request user input because the missing information cannot be inferred.',
  '- cancel: stop the team run because continuing is unsafe or invalid.',
  '',
  'Schema:',
  '{',
  '  "action": "retry | reassign | ask_user | cancel",',
  '  "reason": "why this action is correct",',
  '  "revisedInstruction": "instruction for retry/reassign, or null",',
  '  "targetSolverId": "required only for reassign, otherwise null"',
  '}',
  '',
  `User goal: ${params.goal}`,
  `Failed task: ${params.assignment.task.title}`,
  `Original instruction: ${params.assignment.task.instruction}`,
  `Failed solver: ${params.assignment.solver.id} (${params.assignment.solver.name})`,
  `Attempt index: ${params.attemptIndex}`,
  `Error: ${params.error}`,
  `Available solvers: ${stringifyForTeamMemory(params.availableSolvers, 3000)}`,
  `Completed progress: ${stringifyForTeamMemory(params.completedProgress, 3000)}`,
].join('\n')

const resolveTeamV4ContextMode = (value: string | null | undefined) => {
  if (value === 'claude-like' || value === 'codex-like' || value === 'sentinel-like') {
    return value
  }
  throw new Error(`Invalid Team v4 solver context mode: ${value || 'empty'}`)
}

const readTeamV4ToolPolicyMatrix = (teamRun: TeamV4RunBootstrap) => {
  const matrix = teamRun.run.policy_json?.toolPolicyMatrix
  if (!matrix || typeof matrix !== 'object' || Array.isArray(matrix)) {
    throw new Error('Team v4 run is missing toolPolicyMatrix.')
  }
  return matrix as Record<string, any>
}

const readTeamV4MaxSolvers = (teamRun: TeamV4RunBootstrap, assignmentCount: number) => {
  const policy = teamRun.run.policy_json?.concurrencyPolicy
  if (!policy || typeof policy !== 'object' || Array.isArray(policy)) {
    throw new Error('Team v4 run is missing concurrencyPolicy.')
  }
  const raw = Number(policy.maxSolvers ?? policy.max_solvers)
  if (!Number.isFinite(raw) || raw < 1) {
    throw new Error('Team v4 concurrencyPolicy.maxSolvers must be at least 1.')
  }
  return Math.min(Math.floor(raw), assignmentCount)
}

const buildTeamV4SolverExecutionId = (
  runId: string,
  taskId: string,
  solverId: string,
  attemptIndex: number,
) => `team-v4:${runId}:${taskId}:solver:${solverId}:attempt:${attemptIndex}`

const buildTeamV4CommanderFinalContent = (solverResults: Array<{
  solverId: string
  taskId: string
  result: {
    response: string
  }
}>) => {
  const lines = ['Commander summary: Team run completed.']
  solverResults.forEach((item, index) => {
    lines.push('')
    lines.push(`Solver ${index + 1} (${item.solverId}) result:`)
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

const buildTeamV4InheritedProgress = (solverResults: Array<{
  solverId: string
  taskId: string
  result: {
    response: string
  }
}>) =>
  solverResults.map((item) => ({
    solverId: item.solverId,
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

  const recordTeamV4ObserverMemory = async (input: {
    event: TeamV4Event
    eventType: 'solver_execution_completed' | 'solver_execution_failed'
    goal: string
    result?: unknown
    error?: string
    solverId: string
    taskId: string
    taskTitle?: string
    teamRun: TeamV4RunBootstrap
  }) => {
    const signals = extractObserverSignals({
      eventType: input.eventType,
      result: input.result,
      error: input.error,
    })
    const observerContextMode = resolveTeamV4ContextMode(input.teamRun.observer.context_mode)
    const observerModel = input.teamRun.observer.model?.trim() || null
    const observerContextSnapshot = await teamRuntimeApi.createContextSnapshot(input.teamRun.run.id, {
      actorId: input.teamRun.observer.id,
      taskId: input.taskId,
      roleType: 'observer',
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
    const observerExecutionId = [
      'team-v4',
      input.teamRun.run.id,
      input.taskId,
      'observer-review',
      input.event.sequence,
    ].join(':')
    const observerReviewRaw = await runTeamV4ModelExecution({
      contextMode: observerContextMode,
      executionId: observerExecutionId,
      model: observerModel,
      prompt: buildTeamV4ObserverReviewPrompt({
        eventType: input.eventType,
        goal: input.goal,
        taskTitle: input.taskTitle,
        result: input.result,
        error: input.error,
      }),
      roleLabel: 'Observer review',
    })
    const observerReview = parseTeamV4ObserverReview(observerReviewRaw)
    const reviewEvent = await teamRuntimeApi.appendEvent(input.teamRun.run.id, {
      actorId: input.teamRun.observer.id,
      taskId: input.taskId,
      eventType: 'observer_model_review_completed',
      visibility: 'workspace',
      payload: {
        contextSnapshotId: observerContextSnapshot.id,
        executionId: observerExecutionId,
        review: observerReview,
        sourceEventId: input.event.id,
        sourceEventType: input.event.event_type,
      },
    })
    const baseContent = buildObserverMemoryContent({
      eventType: input.eventType,
      goal: input.goal,
      taskTitle: input.taskTitle,
      result: input.result,
      error: input.error,
    })
    const content = [
      `Observer summary: ${observerReview.summary}`,
      baseContent,
      observerReview.evidence.length
        ? `Evidence:\n${observerReview.evidence.map((item, index) => `${index + 1}. ${item}`).join('\n')}`
        : '',
      observerReview.riskSignals.length
        ? `Risk signals:\n${observerReview.riskSignals.map((item, index) => `${index + 1}. ${item}`).join('\n')}`
        : '',
    ].filter(Boolean).join('\n\n')
    const memory = await teamRuntimeApi.createMemoryCandidate(input.teamRun.run.id, {
      taskId: input.taskId,
      kind: observerReview.kind,
      content,
      confidence: observerReview.confidence,
      sourceEventIds: [input.event.id, reviewEvent.id],
      metadata: {
        curationSchema: 'team_v4_observer_model_v1',
        evidence: observerReview.evidence,
        modelContextSnapshotId: observerContextSnapshot.id,
        modelExecutionId: observerExecutionId,
        modelReview: observerReview,
        observerId: input.teamRun.observer.id,
        riskSignals: observerReview.riskSignals,
        ruleSignals: signals,
        solverId: input.solverId,
        summary: observerReview.summary,
        sourceEventType: input.event.event_type,
        taskTitle: input.taskTitle || null,
        gate: 'observer_candidate',
      },
    })
    await teamRuntimeApi.acceptMemory(input.teamRun.run.id, memory.id, observerReview.promoteToLongTerm)
  }

  const requestTeamV4CommanderRecoveryDecision = async (input: {
    assignment: TeamV4SolverAssignment
    attemptIndex: number
    error: string
    goal: string
    solverResults: Array<{
      solverId: string
      taskId: string
      result: {
        response: string
      }
    }>
    teamRun: TeamV4RunBootstrap
  }) => {
    const commanderContextMode = resolveTeamV4ContextMode(input.teamRun.commander.context_mode)
    const commanderModel = input.teamRun.commander.model?.trim() || null
    const completedProgress = buildTeamV4InheritedProgress(input.solverResults)
    const contextSnapshot = await teamRuntimeApi.createContextSnapshot(input.teamRun.run.id, {
      actorId: input.teamRun.commander.id,
      taskId: input.assignment.task.id,
      roleType: 'commander',
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
            solverId: input.assignment.solver.id,
            solverName: input.assignment.solver.name,
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
      'commander-replan',
      input.attemptIndex,
    ].join(':')
    const availableSolvers = input.teamRun.solvers.map((solver) => ({
      id: solver.id,
      name: solver.name,
      model: solver.model,
      contextMode: solver.context_mode,
    }))
    const rawDecision = await runTeamV4ModelExecution({
      contextMode: commanderContextMode,
      executionId,
      model: commanderModel,
      prompt: buildTeamV4CommanderRecoveryPrompt({
        assignment: input.assignment,
        attemptIndex: input.attemptIndex,
        availableSolvers,
        completedProgress,
        error: input.error,
        goal: input.goal,
      }),
      roleLabel: 'Commander recovery',
    })
    const decision = parseTeamV4CommanderRecoveryDecision(
      rawDecision,
      new Set(input.teamRun.solvers.map((solver) => solver.id)),
    )
    await teamRuntimeApi.appendEvent(input.teamRun.run.id, {
      actorId: input.teamRun.commander.id,
      taskId: input.assignment.task.id,
      eventType: 'commander_replan_completed',
      visibility: 'workspace',
      payload: {
        contextSnapshotId: contextSnapshot.id,
        decision,
        error: input.error,
        executionId,
        failedSolverId: input.assignment.solver.id,
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
          const assignments = await planTeamV4SolverAssignments({
            baselineToolConfig: params.buildToolConfig(),
            getAssistantProfileOption: params.getAssistantProfileOption,
            goal: fullTask,
            runModelExecution: runTeamV4ModelExecution,
            teamRun,
            teamToolPolicyMatrix,
            webSearchEnabled: params.webSearchEnabled.value,
          })
          if (!assignments.length) {
            throw new Error('Team v4 requires at least one solver assignment.')
          }
          const maxSolvers = readTeamV4MaxSolvers(teamRun, assignments.length)
          await teamRuntimeApi.appendEvent(teamRun.run.id, {
            actorId: teamRun.commander.id,
            taskId: teamRun.rootTask.id,
            eventType: 'solver_scheduler_started',
            visibility: 'workspace',
            payload: {
              assignmentCount: assignments.length,
              planningMode: 'commander_task_graph',
              maxSolvers,
            },
          })
          appendTeamV4LocalMessage(
            params.agentMessages,
            `Commander planned ${assignments.length} task(s) and started Solver scheduling, max ${maxSolvers} concurrent Solver(s).`,
            {
              kind: 'team_v4_scheduler_started',
              team_member_id: teamRun.commander.id,
              team_member_name: teamRun.commander.name,
              team_member_role: 'commander',
              team_session_id: teamRun.run.id,
              team_task_record_id: teamRun.rootTask.id,
            },
          )
          await params.refreshTeamV4Workspace?.()
          const solverResults: Array<{
            solverId: string
            taskId: string
            result: {
              messageId: unknown
              outcome: AgentExecutionFinishedEvent['outcome']
              response: string
            }
          }> = []

          const runSolverAssignment = async (
            assignment: TeamV4SolverAssignment,
            assignmentIndex: number,
          ) => {
            const runSolverAttempt = async (
              effectiveAssignment: TeamV4SolverAssignment,
              attemptIndex: number,
              recoveryInstruction?: string | null,
            ) => {
              const inheritedProgress = buildTeamV4InheritedProgress(solverResults)
              const inheritanceSnapshot = await teamRuntimeApi.createContextSnapshot(teamRun.run.id, {
                actorId: effectiveAssignment.solver.id,
                taskId: effectiveAssignment.task.id,
                roleType: 'solver',
                sourceSequence: teamRun.events[teamRun.events.length - 1]?.sequence ?? null,
                policyJson: {
                  inheritHistory: true,
                  inheritanceMode: 'execution_time_completed_solver_progress',
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
                    title: 'Commander Assignment',
                    content: {
                      solverId: effectiveAssignment.solver.id,
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
              const solverExecutionId = buildTeamV4SolverExecutionId(
                teamRun.run.id,
                effectiveAssignment.task.id,
                effectiveAssignment.solver.id,
                attemptIndex,
              )
              const solverContextMode = resolveTeamV4ContextMode(effectiveAssignment.solver.context_mode)
              const solverModel = effectiveAssignment.solver.model?.trim() || null
              const solverProfileId = effectiveAssignment.solver.profile_id?.trim()
              if (!solverProfileId) {
                throw new Error(`Team v4 solver ${effectiveAssignment.solver.id} is missing profile_id.`)
              }
              const solverProfile = params.getAssistantProfileOption(solverProfileId)
              if (!solverProfile) {
                throw new Error(`Team v4 solver profile not found: ${solverProfileId}.`)
              }
              const solverProfileToolConfig = buildTeamV4SolverProfileToolConfig(
                solverProfile,
                params.buildToolConfig(),
              )
              const solverRuntimeToolConfig = buildRuntimeToolConfigForTeamRole(
                solverProfileToolConfig,
                teamToolPolicyMatrix,
                'solver',
                {
                  webSearchEnabled: params.webSearchEnabled.value,
                },
              )
              const startedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                actorId: effectiveAssignment.solver.id,
                taskId: effectiveAssignment.task.id,
                eventType: 'solver_execution_started',
                visibility: 'workspace',
                payload: {
                  model: solverModel,
                  contextMode: solverContextMode,
                  executionId: solverExecutionId,
                  assignmentIndex,
                  attemptIndex,
                  recoveryInstruction: recoveryInstruction || null,
                  inheritanceSnapshotId: inheritanceSnapshot.id,
                  inheritedProgressCount: inheritedProgress.length,
                  taskKey: effectiveAssignment.task.task_key,
                  toolPolicy: {
                    profileId: solverProfileId,
                    selectionStrategy: solverRuntimeToolConfig.selection_strategy,
                    maxTools: solverRuntimeToolConfig.max_tools,
                    fixedTools: solverRuntimeToolConfig.fixed_tools,
                    disabledTools: solverRuntimeToolConfig.disabled_tools,
                    allowedTools: solverRuntimeToolConfig.allowed_tools,
                  },
                },
              })
              appendTeamV4LocalMessage(
                params.agentMessages,
                `Solver started: ${effectiveAssignment.solver.name} -> ${effectiveAssignment.task.title}`,
                {
                  kind: 'team_v4_solver_started',
                  team_member_id: effectiveAssignment.solver.id,
                  team_member_name: effectiveAssignment.solver.name,
                  team_member_role: 'solver',
                  team_session_id: teamRun.run.id,
                  team_task_record_id: effectiveAssignment.task.id,
                  team_task_key: effectiveAssignment.task.task_key,
                  team_sequence: startedEvent.sequence,
                },
              )
              const solverStartedAt = Date.now()
              const solverProgressMessageId = crypto.randomUUID()
              const solverProgressPrefix =
                `${effectiveAssignment.solver.name} -> ${effectiveAssignment.task.title}`
              const updateSolverProgressMessage = (status: 'running' | 'completed' | 'failed') => {
                const elapsedSeconds = Math.max(0, Math.round((Date.now() - solverStartedAt) / 1000))
                const statusLabel = status === 'running'
                  ? `Solver running: ${solverProgressPrefix} (${elapsedSeconds}s)`
                  : `Solver ${status}: ${solverProgressPrefix} (${elapsedSeconds}s)`
                const existing = params.agentMessages.value.find((item) => item.id === solverProgressMessageId)
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
                  id: solverProgressMessageId,
                  type: 'planning',
                  content: statusLabel,
                  timestamp: Date.now(),
                  metadata: {
                    kind: 'team_v4_solver_progress',
                    status,
                    duration_ms: elapsedSeconds * 1000,
                    team_member_id: effectiveAssignment.solver.id,
                    team_member_name: effectiveAssignment.solver.name,
                    team_member_role: 'solver',
                    team_session_id: teamRun.run.id,
                    team_task_record_id: effectiveAssignment.task.id,
                    team_task_key: effectiveAssignment.task.task_key,
                  },
                })
              }
              updateSolverProgressMessage('running')
              const solverActivityProgress = startTeamV4SolverActivityProgress({
                executionId: solverExecutionId,
                messages: params.agentMessages,
                runId: teamRun.run.id,
                solverId: effectiveAssignment.solver.id,
                solverName: effectiveAssignment.solver.name,
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
                    updateSolverProgressMessage('running')
                    return params.refreshTeamV4Workspace?.()
                  })
                  .catch((error) => {
                    console.warn(
                      '[useAgentConversationFlow] Team v4 harness heartbeat failed:',
                      error,
                    )
                  })
              }, 30_000)

              let solverFinishedSuccessfully = false
              try {
                const solverTaskPrompt = buildTeamV4SolverTaskPrompt(
                  effectiveAssignment,
                  fullTask,
                  assignments.length,
                  inheritedProgress,
                  recoveryInstruction,
                )
                const executionFinished = watchAgentExecutionFinished(solverExecutionId, 360_000)
                let messageId: unknown
                try {
                  activeTeamV4ExecutionIds.add(solverExecutionId)
                  messageId = await executeConversationTaskSupport({
                    assistantContextMode: solverContextMode,
                    assistantSelectedModel: solverModel,
                    conversationId: solverExecutionId,
                    defaultConversationTitle: params.getUnnamedConversationTitle(),
                    displayContent,
                    enableRag: params.ragEnabled.value,
                    enableTenthManRule: params.tenthManEnabled.value,
                    firstMessage: task,
                    forceTasks: params.forceTasks,
                    fullTask: solverTaskPrompt,
                    maybeAutoRenameConversation: (renameParams) => {
                      void maybeAutoRenameConversationByFirstMessage(renameParams)
                    },
                    onConversationListRefresh: params.loadConversationList,
                    onCurrentConversationTitleChange: (title) => {
                      params.currentConversationTitle.value = title
                    },
                    persistMessages: false,
                    runAgentExecute: (request) => invoke('agent_execute', request),
                    runtimeToolConfig: solverRuntimeToolConfig,
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
                  throw new Error(finished.error || finished.message || 'Solver execution failed.')
                }
                solverFinishedSuccessfully = true
                const result = {
                  messageId,
                  outcome: finished.outcome,
                  response: finished.response || finished.message || '',
                }
                activeTeamV4ExecutionIds.delete(solverExecutionId)
                const completedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: effectiveAssignment.solver.id,
                  taskId: effectiveAssignment.task.id,
                  eventType: 'solver_execution_completed',
                  visibility: 'user',
                  payload: {
                    result,
                    executionId: solverExecutionId,
                    attemptIndex,
                    taskKey: effectiveAssignment.task.task_key,
                  },
                })
                await teamRuntimeApi.checkpointHarnessRun(
                  effectiveAssignment.harnessRun.id,
                  completedEvent.sequence,
                )
                await recordTeamV4ObserverMemory({
                  event: completedEvent,
                  eventType: 'solver_execution_completed',
                  goal: fullTask,
                  result,
                  solverId: effectiveAssignment.solver.id,
                  taskId: effectiveAssignment.task.id,
                  taskTitle: effectiveAssignment.task.title,
                  teamRun,
                })
                updateSolverProgressMessage('completed')
                solverActivityProgress.complete()
                await params.refreshTeamV4Workspace?.()
                return {
                  solverId: effectiveAssignment.solver.id,
                  taskId: effectiveAssignment.task.id,
                  result,
                }
              } catch (solverError: any) {
                activeTeamV4ExecutionIds.delete(solverExecutionId)
                const solverErrorMsg = solverError?.toString?.() || String(solverError)
                if (solverFinishedSuccessfully) {
                  await teamRuntimeApi.appendEvent(teamRun.run.id, {
                    actorId: teamRun.observer.id,
                    taskId: effectiveAssignment.task.id,
                    eventType: 'observer_model_review_failed',
                    visibility: 'user',
                    payload: {
                      error: solverErrorMsg,
                      executionId: solverExecutionId,
                      attemptIndex,
                      solverId: effectiveAssignment.solver.id,
                      taskKey: effectiveAssignment.task.task_key,
                    },
                  })
                  await params.refreshTeamV4Workspace?.()
                  throw solverError
                }
                const failedEvent = await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: effectiveAssignment.solver.id,
                  taskId: effectiveAssignment.task.id,
                  eventType: 'solver_execution_failed',
                  visibility: 'user',
                  payload: {
                    error: solverErrorMsg,
                    executionId: solverExecutionId,
                    attemptIndex,
                    taskKey: effectiveAssignment.task.task_key,
                  },
                })
                await teamRuntimeApi.checkpointHarnessRun(
                  effectiveAssignment.harnessRun.id,
                  failedEvent.sequence,
                )
                await recordTeamV4ObserverMemory({
                  error: solverErrorMsg,
                  event: failedEvent,
                  eventType: 'solver_execution_failed',
                  goal: fullTask,
                  solverId: effectiveAssignment.solver.id,
                  taskId: effectiveAssignment.task.id,
                  taskTitle: effectiveAssignment.task.title,
                  teamRun,
                })
                updateSolverProgressMessage('failed')
                solverActivityProgress.fail(solverErrorMsg)
                await params.refreshTeamV4Workspace?.()
                throw solverError
              } finally {
                window.clearInterval(heartbeatTimer)
                solverActivityProgress.dispose()
              }
            }

            try {
              return await runSolverAttempt(assignment, 0)
            } catch (firstError: any) {
              const firstErrorMsg = firstError?.toString?.() || String(firstError)
              let decision: TeamV4CommanderRecoveryDecision
              try {
                decision = await requestTeamV4CommanderRecoveryDecision({
                  assignment,
                  attemptIndex: 0,
                  error: firstErrorMsg,
                  goal: fullTask,
                  solverResults,
                  teamRun,
                })
              } catch (replanError: any) {
                const replanErrorMsg = replanError?.toString?.() || String(replanError)
                await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: teamRun.commander.id,
                  taskId: assignment.task.id,
                  eventType: 'commander_recovery_decision',
                  visibility: 'user',
                  payload: {
                    decision: 'cancel_after_replan_failure',
                    failedSolverId: assignment.solver.id,
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
                actorId: teamRun.commander.id,
                taskId: assignment.task.id,
                eventType: 'commander_recovery_decision',
                visibility: 'user',
                payload: {
                  decision: decision.action,
                  failedSolverId: assignment.solver.id,
                  failedTaskId: assignment.task.id,
                  modelReason: decision.reason,
                  revisedInstruction: decision.revisedInstruction || null,
                  targetSolverId: decision.targetSolverId || null,
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
                throw new Error(`Commander ${decision.action} decision requires revisedInstruction.`)
              }

              const recoveryAssignment = decision.action === 'reassign'
                ? {
                    ...assignment,
                    solver: teamRun.solvers.find((solver) => solver.id === decision.targetSolverId)!,
                  }
                : assignment

              try {
                return await runSolverAttempt(recoveryAssignment, 1, decision.revisedInstruction)
              } catch (recoveryError: any) {
                await teamRuntimeApi.appendEvent(teamRun.run.id, {
                  actorId: teamRun.commander.id,
                  taskId: assignment.task.id,
                  eventType: 'commander_recovery_decision',
                  visibility: 'user',
                  payload: {
                    decision: 'cancel_after_recovery_failure',
                    failedSolverId: recoveryAssignment.solver.id,
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

          solverResults.push(
            ...await runTeamV4AssignmentsWithDependencies(assignments, maxSolvers, runSolverAssignment),
          )
          await teamRuntimeApi.appendEvent(teamRun.run.id, {
            actorId: teamRun.commander.id,
            taskId: teamRun.rootTask.id,
            eventType: 'solver_scheduler_completed',
            visibility: 'workspace',
            payload: {
              completedAssignments: solverResults.length,
              maxSolvers,
            },
          })

          await teamRuntimeApi.updateRunState(teamRun.run.id, 'completed')
          await params.refreshTeamV4Workspace?.()
          const commanderFinalMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'final' as any,
            content: buildTeamV4CommanderFinalContent(solverResults),
            timestamp: Date.now(),
            metadata: {
              kind: 'team_v4_commander_final',
              team_member_id: teamRun.commander.id,
              team_member_name: teamRun.commander.name,
              team_member_role: 'commander',
              team_session_id: teamRun.run.id,
              team_task_record_id: teamRun.rootTask.id,
            },
          }
          params.agentMessages.value.push(commanderFinalMessage)
          await invoke('save_ai_message', {
            request: {
              id: commanderFinalMessage.id,
              conversation_id: ensuredConversationId,
              role: 'assistant',
              content: commanderFinalMessage.content,
              metadata: commanderFinalMessage.metadata ?? null,
              architecture_type: 'team_v4',
              architecture_meta: JSON.stringify({
                team_run_id: teamRun.run.id,
                role: 'commander',
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
              solverResults,
            },
          })
        } catch (error: any) {
          const errorMsg = error?.toString?.() || String(error)
          const runId = params.activeTeamV4RunId.value
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
