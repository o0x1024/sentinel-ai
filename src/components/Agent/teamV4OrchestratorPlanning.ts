import { teamRuntimeApi } from '@/api/teamRuntime'
import type { TeamV4Agent, TeamV4RunBootstrap, TeamV4SpecialistAssignment } from '@/types/teamRuntime'
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
  specialistId: string
  requiredTools: string[]
  dependsOnTaskKeys: string[]
  priority: number
}

interface SpecialistCapability {
  specialist: TeamV4Agent
  profile: AssistantProfileOption
  tools: string[]
}

interface PlanTeamV4SpecialistAssignmentsParams {
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
    throw new Error('Orchestrator task plan did not return a JSON object.')
  }
  const parsed = JSON.parse(source.slice(start, end + 1))
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Orchestrator task plan must be a JSON object.')
  }
  return parsed as Record<string, any>
}

const buildSpecialistProfileToolConfig = (
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

const collectSpecialistCapabilities = (params: PlanTeamV4SpecialistAssignmentsParams): SpecialistCapability[] =>
  params.teamRun.specialists.map((specialist) => {
    const profileId = specialist.profile_id?.trim()
    if (!profileId) throw new Error(`Team v4 specialist ${specialist.id} is missing profile_id.`)
    const profile = params.getAssistantProfileOption(profileId)
    if (!profile) throw new Error(`Team v4 specialist profile not found: ${profileId}.`)
    const runtimeToolConfig = buildRuntimeToolConfigForTeamRole(
      buildSpecialistProfileToolConfig(profile, params.baselineToolConfig),
      params.teamToolPolicyMatrix,
      'specialist',
      {
        webSearchEnabled: params.webSearchEnabled,
      },
    )
    return {
      specialist,
      profile,
      tools: normalizeToolIdList(runtimeToolConfig.allowed_tools),
    }
  })

const buildPlanPrompt = (goal: string, capabilities: SpecialistCapability[]) => [
  'You are the Team Orchestrator. Create a concrete multi-task execution graph for the specialists.',
  'Return strict JSON only. Do not include markdown or prose outside JSON.',
  '',
  'Rules:',
  '- Create 1 to 6 tasks.',
  '- Each task must have a unique key using lowercase letters, numbers, underscore, or hyphen.',
  '- Each task must choose exactly one specialistId from the available specialists.',
  '- requiredTools must be a subset of the chosen specialist availableTools.',
  '- dependsOnTaskKeys may reference only earlier task keys.',
  '- Use parallel independent tasks when possible.',
  '',
  'Schema:',
  '{',
  '  "tasks": [',
  '    {',
  '      "key": "short_key",',
  '      "title": "task title",',
  '      "instruction": "specific specialist instruction",',
  '      "acceptanceCriteria": "how orchestrator can judge completion",',
  '      "specialistId": "specialist id",',
  '      "requiredTools": ["tool_id"],',
  '      "dependsOnTaskKeys": ["earlier_key"],',
  '      "priority": 0',
  '    }',
  '  ]',
  '}',
  '',
  `User goal: ${goal}`,
  `Available specialists: ${JSON.stringify(capabilities.map((item) => ({
    specialistId: item.specialist.id,
    name: item.specialist.name,
    profileId: item.specialist.profile_id,
    role: item.profile.teamRole,
    contextMode: item.specialist.context_mode,
    availableTools: item.tools,
  })))}`,
].join('\n')

const normalizeTaskKey = (value: unknown) => {
  if (typeof value !== 'string') throw new Error('Orchestrator task key must be a string.')
  const key = value.trim()
  if (!/^[a-z0-9][a-z0-9_-]{1,40}$/.test(key)) {
    throw new Error(`Invalid Orchestrator task key: ${key}.`)
  }
  if (key === 'root') {
    throw new Error('Orchestrator task key root is reserved for the Team run container.')
  }
  return key
}

const readString = (value: unknown, field: string) => {
  if (typeof value !== 'string' || !value.trim()) {
    throw new Error(`Orchestrator task requires non-empty string field: ${field}.`)
  }
  return value.trim()
}

const readStringArray = (value: unknown, field: string) => {
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw new Error(`Orchestrator task requires string[] field: ${field}.`)
  }
  return value.map((item) => item.trim()).filter(Boolean)
}

const parsePlan = (raw: string, capabilities: SpecialistCapability[]): PlannedTask[] => {
  const parsed = parseJsonObject(raw)
  if (!Array.isArray(parsed.tasks) || parsed.tasks.length < 1 || parsed.tasks.length > 6) {
    throw new Error('Orchestrator task plan must contain 1 to 6 tasks.')
  }
  const specialistTools = new Map(capabilities.map((item) => [item.specialist.id, new Set(item.tools)]))
  const seenKeys = new Set<string>()
  return parsed.tasks.map((item, index) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new Error(`Orchestrator task ${index + 1} must be an object.`)
    }
    const task = item as Record<string, unknown>
    const key = normalizeTaskKey(task.key)
    if (seenKeys.has(key)) throw new Error(`Duplicate Orchestrator task key: ${key}.`)
    const specialistId = readString(task.specialistId, 'specialistId')
    const tools = specialistTools.get(specialistId)
    if (!tools) throw new Error(`Orchestrator task ${key} references unknown specialistId: ${specialistId}.`)
    const requiredTools = normalizeToolIdList(readStringArray(task.requiredTools, 'requiredTools'))
    const missingTools = requiredTools.filter((tool) => !tools.has(tool))
    if (missingTools.length > 0) {
      throw new Error(`Orchestrator task ${key} requires unavailable tools for ${specialistId}: ${missingTools.join(', ')}.`)
    }
    const dependsOnTaskKeys = readStringArray(task.dependsOnTaskKeys, 'dependsOnTaskKeys')
    const invalidDependency = dependsOnTaskKeys.find((dependency) => !seenKeys.has(dependency))
    if (invalidDependency) {
      throw new Error(`Orchestrator task ${key} has invalid dependency: ${invalidDependency}.`)
    }
    seenKeys.add(key)
    return {
      key,
      title: readString(task.title, 'title'),
      instruction: readString(task.instruction, 'instruction'),
      acceptanceCriteria: readString(task.acceptanceCriteria, 'acceptanceCriteria'),
      specialistId,
      requiredTools,
      dependsOnTaskKeys,
      priority: Math.max(0, Math.floor(Number(task.priority) || index)),
    }
  })
}

export const planTeamV4SpecialistAssignments = async (
  params: PlanTeamV4SpecialistAssignmentsParams,
): Promise<TeamV4SpecialistAssignment[]> => {
  const capabilities = collectSpecialistCapabilities(params)
  const orchestratorContextMode = params.teamRun.orchestrator.context_mode === 'codex-like'
    || params.teamRun.orchestrator.context_mode === 'sentinel-like'
    ? params.teamRun.orchestrator.context_mode
    : 'claude-like'
  const contextSnapshot = await teamRuntimeApi.createContextSnapshot(params.teamRun.run.id, {
    actorId: params.teamRun.orchestrator.id,
    taskId: params.teamRun.rootTask.id,
    roleType: 'orchestrator',
    sourceSequence: params.teamRun.events[params.teamRun.events.length - 1]?.sequence ?? null,
    policyJson: {
      planningMode: 'model_task_graph_specialist_tool_matching',
    },
    sectionsJson: [
      {
        id: 'goal',
        title: 'User Goal',
        content: params.goal,
      },
      {
        id: 'specialist_capabilities',
        title: 'Specialist Capabilities',
        content: capabilities.map((item) => ({
          specialistId: item.specialist.id,
          name: item.specialist.name,
          profileId: item.specialist.profile_id,
          availableTools: item.tools,
        })),
      },
    ],
    tokenEstimate: 0,
  })
  const executionId = `team-v4:${params.teamRun.run.id}:orchestrator-task-plan`
  const rawPlan = await params.runModelExecution({
    contextMode: orchestratorContextMode,
    executionId,
    model: params.teamRun.orchestrator.model?.trim() || null,
    prompt: buildPlanPrompt(params.goal, capabilities),
    roleLabel: 'Orchestrator task planning',
  })
  const plannedTasks = parsePlan(rawPlan, capabilities)
  const planEvent = await teamRuntimeApi.appendEvent(params.teamRun.run.id, {
    actorId: params.teamRun.orchestrator.id,
    taskId: params.teamRun.rootTask.id,
    eventType: 'orchestrator_task_graph_planned',
    visibility: 'workspace',
    payload: {
      contextSnapshotId: contextSnapshot.id,
      executionId,
      taskCount: plannedTasks.length,
      tasks: plannedTasks,
    },
  })
  const specialistById = new Map(params.teamRun.specialists.map((specialist) => [specialist.id, specialist]))
  const taskIdsByKey = new Map<string, string>()
  const leaseSecs = Math.max(30, Math.floor(Number(params.teamRun.run.policy_json?.harnessPolicy?.leaseSecs) || 600))
  const assignments: TeamV4SpecialistAssignment[] = []

  for (const plannedTask of plannedTasks) {
    const specialist = specialistById.get(plannedTask.specialistId)
    if (!specialist) throw new Error(`Planned task references missing specialist: ${plannedTask.specialistId}.`)
    const task = await teamRuntimeApi.createTask(params.teamRun.run.id, {
      parentTaskId: params.teamRun.rootTask.id,
      taskKey: plannedTask.key,
      title: plannedTask.title,
      instruction: plannedTask.instruction,
      priority: plannedTask.priority,
      assignedAgentId: specialist.id,
      dependsOn: plannedTask.dependsOnTaskKeys.map((key) => taskIdsByKey.get(key) as string),
      acceptanceCriteria: plannedTask.acceptanceCriteria,
      metadata: {
        createdBy: params.teamRun.orchestrator.id,
        dispatchMode: 'orchestrator_task_graph',
        requiredTools: plannedTask.requiredTools,
        specialistProfileId: specialist.profile_id,
      },
    })
    taskIdsByKey.set(plannedTask.key, task.id)
    const context = await teamRuntimeApi.createContextSnapshot(params.teamRun.run.id, {
      actorId: specialist.id,
      taskId: task.id,
      roleType: 'specialist',
      sourceSequence: planEvent.sequence,
      policyJson: {
        inheritHistory: true,
        planningMode: 'orchestrator_task_graph',
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
          title: 'Orchestrator Assignment',
          content: plannedTask,
        },
      ],
      tokenEstimate: 0,
    })
    const harnessRun = await teamRuntimeApi.startHarnessRun(params.teamRun.run.id, {
      actorId: specialist.id,
      taskId: task.id,
      leaseSecs,
      metadata: {
        managedBy: 'harness',
        checkpointPolicy: 'event_sequence',
        planningMode: 'orchestrator_task_graph',
      },
    })
    assignments.push({
      specialist,
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
