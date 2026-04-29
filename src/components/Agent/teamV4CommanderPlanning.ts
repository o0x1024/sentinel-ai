import { teamRuntimeApi } from '@/api/teamRuntime'
import type { TeamV4Agent, TeamV4RunBootstrap, TeamV4SolverAssignment } from '@/types/teamRuntime'
import type { AssistantProfileOption } from './assistantProfiles'
import {
  buildRuntimeToolConfigForTeamRole,
  normalizeToolIdList,
  type UiToolConfigPayload,
} from './toolConfigRuntime'

interface PlannedTask {
  key: string
  title: string
  instruction: string
  acceptanceCriteria: string
  solverId: string
  requiredTools: string[]
  dependsOnTaskKeys: string[]
  priority: number
}

interface SolverCapability {
  solver: TeamV4Agent
  profile: AssistantProfileOption
  tools: string[]
}

interface PlanTeamV4SolverAssignmentsParams {
  baselineToolConfig: UiToolConfigPayload
  getAssistantProfileOption: (profileId: string) => AssistantProfileOption | null
  goal: string
  runModelExecution: (input: {
    contextMode: 'claude-like' | 'codex-like' | 'sentinel-like'
    executionId: string
    model: string | null
    prompt: string
    roleLabel: string
    timeoutMs?: number
  }) => Promise<string>
  teamRun: TeamV4RunBootstrap
  teamToolPolicyMatrix: Record<string, any>
  webSearchEnabled: boolean
}

const parseJsonObject = (raw: string) => {
  const trimmed = raw.trim()
  const fenced = trimmed.match(/```(?:json)?\s*([\s\S]*?)```/i)
  const source = (fenced?.[1] || trimmed).trim()
  const start = source.indexOf('{')
  const end = source.lastIndexOf('}')
  if (start < 0 || end <= start) {
    throw new Error('Commander task plan did not return a JSON object.')
  }
  const parsed = JSON.parse(source.slice(start, end + 1))
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Commander task plan must be a JSON object.')
  }
  return parsed as Record<string, any>
}

const buildSolverProfileToolConfig = (
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

const collectSolverCapabilities = (params: PlanTeamV4SolverAssignmentsParams): SolverCapability[] =>
  params.teamRun.solvers.map((solver) => {
    const profileId = solver.profile_id?.trim()
    if (!profileId) throw new Error(`Team v4 solver ${solver.id} is missing profile_id.`)
    const profile = params.getAssistantProfileOption(profileId)
    if (!profile) throw new Error(`Team v4 solver profile not found: ${profileId}.`)
    const runtimeToolConfig = buildRuntimeToolConfigForTeamRole(
      buildSolverProfileToolConfig(profile, params.baselineToolConfig),
      params.teamToolPolicyMatrix,
      'solver',
      {
        webSearchEnabled: params.webSearchEnabled,
      },
    )
    return {
      solver,
      profile,
      tools: normalizeToolIdList(runtimeToolConfig.allowed_tools),
    }
  })

const buildPlanPrompt = (goal: string, capabilities: SolverCapability[]) => [
  'You are the Team Commander. Create a concrete multi-task execution graph for the solvers.',
  'Return strict JSON only. Do not include markdown or prose outside JSON.',
  '',
  'Rules:',
  '- Create 1 to 6 tasks.',
  '- Each task must have a unique key using lowercase letters, numbers, underscore, or hyphen.',
  '- Each task must choose exactly one solverId from the available solvers.',
  '- requiredTools must be a subset of the chosen solver availableTools.',
  '- dependsOnTaskKeys may reference only earlier task keys.',
  '- Use parallel independent tasks when possible.',
  '',
  'Schema:',
  '{',
  '  "tasks": [',
  '    {',
  '      "key": "short_key",',
  '      "title": "task title",',
  '      "instruction": "specific solver instruction",',
  '      "acceptanceCriteria": "how commander can judge completion",',
  '      "solverId": "solver id",',
  '      "requiredTools": ["tool_id"],',
  '      "dependsOnTaskKeys": ["earlier_key"],',
  '      "priority": 0',
  '    }',
  '  ]',
  '}',
  '',
  `User goal: ${goal}`,
  `Available solvers: ${JSON.stringify(capabilities.map((item) => ({
    solverId: item.solver.id,
    name: item.solver.name,
    profileId: item.solver.profile_id,
    role: item.profile.teamRole,
    contextMode: item.solver.context_mode,
    availableTools: item.tools,
  })))}`,
].join('\n')

const normalizeTaskKey = (value: unknown) => {
  if (typeof value !== 'string') throw new Error('Commander task key must be a string.')
  const key = value.trim()
  if (!/^[a-z0-9][a-z0-9_-]{1,40}$/.test(key)) {
    throw new Error(`Invalid Commander task key: ${key}.`)
  }
  if (key === 'root') {
    throw new Error('Commander task key root is reserved for the Team run container.')
  }
  return key
}

const readString = (value: unknown, field: string) => {
  if (typeof value !== 'string' || !value.trim()) {
    throw new Error(`Commander task requires non-empty string field: ${field}.`)
  }
  return value.trim()
}

const readStringArray = (value: unknown, field: string) => {
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw new Error(`Commander task requires string[] field: ${field}.`)
  }
  return value.map((item) => item.trim()).filter(Boolean)
}

const parsePlan = (raw: string, capabilities: SolverCapability[]): PlannedTask[] => {
  const parsed = parseJsonObject(raw)
  if (!Array.isArray(parsed.tasks) || parsed.tasks.length < 1 || parsed.tasks.length > 6) {
    throw new Error('Commander task plan must contain 1 to 6 tasks.')
  }
  const solverTools = new Map(capabilities.map((item) => [item.solver.id, new Set(item.tools)]))
  const seenKeys = new Set<string>()
  return parsed.tasks.map((item, index) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new Error(`Commander task ${index + 1} must be an object.`)
    }
    const task = item as Record<string, unknown>
    const key = normalizeTaskKey(task.key)
    if (seenKeys.has(key)) throw new Error(`Duplicate Commander task key: ${key}.`)
    const solverId = readString(task.solverId, 'solverId')
    const tools = solverTools.get(solverId)
    if (!tools) throw new Error(`Commander task ${key} references unknown solverId: ${solverId}.`)
    const requiredTools = normalizeToolIdList(readStringArray(task.requiredTools, 'requiredTools'))
    const missingTools = requiredTools.filter((tool) => !tools.has(tool))
    if (missingTools.length > 0) {
      throw new Error(`Commander task ${key} requires unavailable tools for ${solverId}: ${missingTools.join(', ')}.`)
    }
    const dependsOnTaskKeys = readStringArray(task.dependsOnTaskKeys, 'dependsOnTaskKeys')
    const invalidDependency = dependsOnTaskKeys.find((dependency) => !seenKeys.has(dependency))
    if (invalidDependency) {
      throw new Error(`Commander task ${key} has invalid dependency: ${invalidDependency}.`)
    }
    seenKeys.add(key)
    return {
      key,
      title: readString(task.title, 'title'),
      instruction: readString(task.instruction, 'instruction'),
      acceptanceCriteria: readString(task.acceptanceCriteria, 'acceptanceCriteria'),
      solverId,
      requiredTools,
      dependsOnTaskKeys,
      priority: Math.max(0, Math.floor(Number(task.priority) || index)),
    }
  })
}

export const planTeamV4SolverAssignments = async (
  params: PlanTeamV4SolverAssignmentsParams,
): Promise<TeamV4SolverAssignment[]> => {
  const capabilities = collectSolverCapabilities(params)
  const commanderContextMode = params.teamRun.commander.context_mode === 'codex-like'
    || params.teamRun.commander.context_mode === 'sentinel-like'
    ? params.teamRun.commander.context_mode
    : 'claude-like'
  const contextSnapshot = await teamRuntimeApi.createContextSnapshot(params.teamRun.run.id, {
    actorId: params.teamRun.commander.id,
    taskId: params.teamRun.rootTask.id,
    roleType: 'commander',
    sourceSequence: params.teamRun.events[params.teamRun.events.length - 1]?.sequence ?? null,
    policyJson: {
      planningMode: 'model_task_graph_solver_tool_matching',
    },
    sectionsJson: [
      {
        id: 'goal',
        title: 'User Goal',
        content: params.goal,
      },
      {
        id: 'solver_capabilities',
        title: 'Solver Capabilities',
        content: capabilities.map((item) => ({
          solverId: item.solver.id,
          name: item.solver.name,
          profileId: item.solver.profile_id,
          availableTools: item.tools,
        })),
      },
    ],
    tokenEstimate: 0,
  })
  const executionId = `team-v4:${params.teamRun.run.id}:commander-task-plan`
  const rawPlan = await params.runModelExecution({
    contextMode: commanderContextMode,
    executionId,
    model: params.teamRun.commander.model?.trim() || null,
    prompt: buildPlanPrompt(params.goal, capabilities),
    roleLabel: 'Commander task planning',
  })
  const plannedTasks = parsePlan(rawPlan, capabilities)
  const planEvent = await teamRuntimeApi.appendEvent(params.teamRun.run.id, {
    actorId: params.teamRun.commander.id,
    taskId: params.teamRun.rootTask.id,
    eventType: 'commander_task_graph_planned',
    visibility: 'workspace',
    payload: {
      contextSnapshotId: contextSnapshot.id,
      executionId,
      taskCount: plannedTasks.length,
      tasks: plannedTasks,
    },
  })
  const solverById = new Map(params.teamRun.solvers.map((solver) => [solver.id, solver]))
  const taskIdsByKey = new Map<string, string>()
  const leaseSecs = Math.max(30, Math.floor(Number(params.teamRun.run.policy_json?.harnessPolicy?.leaseSecs) || 600))
  const assignments: TeamV4SolverAssignment[] = []

  for (const plannedTask of plannedTasks) {
    const solver = solverById.get(plannedTask.solverId)
    if (!solver) throw new Error(`Planned task references missing solver: ${plannedTask.solverId}.`)
    const task = await teamRuntimeApi.createTask(params.teamRun.run.id, {
      parentTaskId: params.teamRun.rootTask.id,
      taskKey: plannedTask.key,
      title: plannedTask.title,
      instruction: plannedTask.instruction,
      priority: plannedTask.priority,
      assignedAgentId: solver.id,
      dependsOn: plannedTask.dependsOnTaskKeys.map((key) => taskIdsByKey.get(key) as string),
      acceptanceCriteria: plannedTask.acceptanceCriteria,
      metadata: {
        createdBy: params.teamRun.commander.id,
        dispatchMode: 'commander_task_graph',
        requiredTools: plannedTask.requiredTools,
        solverProfileId: solver.profile_id,
      },
    })
    taskIdsByKey.set(plannedTask.key, task.id)
    const context = await teamRuntimeApi.createContextSnapshot(params.teamRun.run.id, {
      actorId: solver.id,
      taskId: task.id,
      roleType: 'solver',
      sourceSequence: planEvent.sequence,
      policyJson: {
        inheritHistory: true,
        planningMode: 'commander_task_graph',
        requiredTools: plannedTask.requiredTools,
      },
      sectionsJson: [
        {
          id: 'goal',
          title: 'User Goal',
          content: params.goal,
        },
        {
          id: 'assignment',
          title: 'Commander Assignment',
          content: plannedTask,
        },
      ],
      tokenEstimate: 0,
    })
    const harnessRun = await teamRuntimeApi.startHarnessRun(params.teamRun.run.id, {
      actorId: solver.id,
      taskId: task.id,
      leaseSecs,
      metadata: {
        managedBy: 'harness',
        checkpointPolicy: 'event_sequence',
        planningMode: 'commander_task_graph',
      },
    })
    assignments.push({
      solver,
      task: {
        ...task,
        context_snapshot_id: context.id,
      },
      contextSnapshot: context,
      harnessRun,
    })
  }

  return assignments
}
