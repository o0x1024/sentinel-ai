import { computed, type ComputedRef, type Ref } from 'vue'
import type { AgentMessage } from '@/types/agent'
import type {
  AgentTeamSession,
  TeamTask,
} from '@/types/agentTeam'
import {
  TEAM_ORCHESTRATION_PRESET_METAS,
  TEAM_RECOVERY_PRESETS,
} from './teamOrchestrationSupport'
import type {
  TeamOrchestrationPlan,
  TeamRecoveryPresetId,
  TeamRuntimeFailureMode,
  TeamRuntimeStepStat,
} from './teamOrchestrationTypes'

interface TeamSplitMemberOption {
  key: string
  label: string
  memberId?: string
  memberName?: string
  status?: string
}

const TEAM_SPLIT_ALL_MEMBER_KEY = '__all__'
const TEAM_RUNNING_STATES = new Set([
  'EXECUTING',
  'INITIALIZING',
  'PROPOSING',
  'CHALLENGING',
  'CONVERGENCE_CHECK',
  'REVISING',
  'DECIDING',
  'ARTIFACT_GENERATION',
])

const normalizeOptionalText = (value: unknown): string | undefined => {
  if (typeof value !== 'string') return undefined
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : undefined
}

const TEAM_MEMBER_STATUS_PRIORITY: Record<string, number> = {
  idle: 0,
  pending: 1,
  completed: 2,
  running: 3,
  blocked: 4,
  failed: 5,
}

const normalizeTeamMemberStatus = (value: unknown): string => {
  const normalized = String(value || '').trim().toLowerCase()
  if (!normalized) return 'idle'
  if (normalized.includes('fail') || normalized.includes('error')) return 'failed'
  if (normalized.includes('block')) return 'blocked'
  if (normalized.includes('run') || normalized.includes('execut')) return 'running'
  if (normalized.includes('done') || normalized.includes('complete')) return 'completed'
  if (normalized.includes('queue') || normalized.includes('pending')) return 'pending'
  return 'idle'
}

const mergeTeamMemberStatus = (current: string | undefined, candidate: string) => {
  const normalizedCurrent = normalizeTeamMemberStatus(current)
  const normalizedCandidate = normalizeTeamMemberStatus(candidate)
  return TEAM_MEMBER_STATUS_PRIORITY[normalizedCandidate] >= TEAM_MEMBER_STATUS_PRIORITY[normalizedCurrent]
    ? normalizedCandidate
    : normalizedCurrent
}

export const useAgentTeamViewState = (params: {
  activeTeamSessionId: Ref<string | null>
  agentIsExecuting: ComputedRef<boolean>
  isTeamScopedMainFlowMessage: (message: AgentMessage) => boolean
  messages: ComputedRef<AgentMessage[]>
  selectedTeamTaskId: Ref<string | null>
  teamModeEnabled: Ref<boolean>
  teamOrchestrationDraft: Ref<TeamOrchestrationPlan>
  teamSelectedOrchestrationPresetId: Ref<string | null>
  teamSelectedRecoveryPresetId: Ref<TeamRecoveryPresetId>
  teamSessionDetail: Ref<AgentTeamSession | null>
  teamSessionState: Ref<string>
  teamTasks: Ref<TeamTask[]>
}): {
  clearSelectedTeamTask: () => void
  formatTimestamp: (value?: string | null) => string
  resolveAgentName: (agentId?: string | null) => string
  selectedTeamTask: ComputedRef<TeamTask | null>
  selectedTeamTaskTitle: ComputedRef<string>
  teamBlackboardEntryBadgeClass: (entryType?: string | null) => string
  teamCurrentHumanInterventionTimeoutSecs: ComputedRef<number>
  teamCurrentMaxHumanInterventions: ComputedRef<number>
  teamCurrentNoHumanInputPolicy: ComputedRef<TeamRecoveryPresetId>
  teamFlattenedStepOptions: ComputedRef<Array<{ id: string; path: string; label: string }>>
  teamLastRuntimeStepPath: ComputedRef<string>
  teamMemberNameOptions: ComputedRef<string[]>
  teamOrchestrationPresets: ComputedRef<typeof TEAM_ORCHESTRATION_PRESET_METAS>
  teamOrchestrationRuntime: ComputedRef<Record<string, any>>
  teamRecoveryPresets: ComputedRef<typeof TEAM_RECOVERY_PRESETS>
  teamRuntimeBackendRecoverySuggestions: ComputedRef<string[]>
  teamRuntimeFailureModes: ComputedRef<TeamRuntimeFailureMode[]>
  teamRuntimeHotspots: ComputedRef<TeamRuntimeStepStat[]>
  teamRuntimeRecoverySuggestions: ComputedRef<string[]>
  teamRuntimeStepStats: ComputedRef<TeamRuntimeStepStat[]>
  teamRuntimeSuggestedResumeStepId: ComputedRef<string>
  teamRuntimeSummary: ComputedRef<{
    totalAttempts: number
    totalSuccess: number
    totalFailed: number
    slowestDurationMs: number
    slowestStepId: string
  }>
  teamSelectedOrchestrationPresetDescription: ComputedRef<string>
  teamSelectedRecoveryPresetDescription: ComputedRef<string>
  teamSplitMembers: ComputedRef<TeamSplitMemberOption[]>
  teamWorkspaceBadgeCount: ComputedRef<number>
  toggleSelectedTeamTask: (task: TeamTask) => void
  visibleMessages: ComputedRef<AgentMessage[]>
} => {
  const runtimeSpecAgentNameById = computed(() => {
    const out = new Map<string, string>()
    const rawAgents = (params.teamSessionDetail.value?.runtime_spec_v2 as any)?.agents
    if (!Array.isArray(rawAgents)) return out
    for (const item of rawAgents) {
      if (!item || typeof item !== 'object') continue
      const id = normalizeOptionalText((item as any).id)
      if (!id) continue
      const name = normalizeOptionalText((item as any).name)
      out.set(id, name || id)
    }
    return out
  })

  const isTeamRunActive = computed(() => {
    if (!params.teamModeEnabled.value || !params.activeTeamSessionId.value) return false
    const normalized = String(params.teamSessionState.value || '').trim().toUpperCase()
    return TEAM_RUNNING_STATES.has(normalized)
  })

  const teamSplitMembers = computed<TeamSplitMemberOption[]>(() => {
    const statusByMemberKey = new Map<string, string>()
    const applyStatus = (key: string | undefined, status: string) => {
      if (!key) return
      const current = statusByMemberKey.get(key)
      statusByMemberKey.set(key, mergeTeamMemberStatus(current, status))
    }

    for (const message of params.messages.value) {
      if (!params.isTeamScopedMainFlowMessage(message)) continue
      const memberId = normalizeOptionalText(message.metadata?.team_member_id)
      const memberName = normalizeOptionalText(message.metadata?.team_member_name)
      const statusFromMeta = normalizeTeamMemberStatus(message.metadata?.status)
      const statusFromType = message.type === 'error' ? 'failed' : 'idle'
      const merged = mergeTeamMemberStatus(statusFromMeta, statusFromType)
      applyStatus(memberId ? `id:${memberId}` : undefined, merged)
      applyStatus(memberName ? `name:${memberName}` : undefined, merged)
    }

    let globalStatus = isTeamRunActive.value ? 'running' : 'idle'
    for (const status of statusByMemberKey.values()) {
      globalStatus = mergeTeamMemberStatus(globalStatus, status)
    }

    const options: TeamSplitMemberOption[] = [
      {
        key: TEAM_SPLIT_ALL_MEMBER_KEY,
        label: '全局',
        status: globalStatus,
      },
    ]
    const seenKeys = new Set<string>([TEAM_SPLIT_ALL_MEMBER_KEY])

    const addMemberOption = (memberId?: string, memberName?: string, status?: string) => {
      if (memberId) {
        const key = `id:${memberId}`
        if (seenKeys.has(key)) return
        seenKeys.add(key)
        options.push({
          key,
          label: memberName || memberId,
          memberId,
          memberName,
          status: statusByMemberKey.get(key) || status,
        })
        return
      }
      if (!memberName) return
      const key = `name:${memberName}`
      if (seenKeys.has(key)) return
      seenKeys.add(key)
      options.push({
        key,
        label: memberName,
        memberName,
        status: statusByMemberKey.get(key) || status,
      })
    }

    for (const member of params.teamSessionDetail.value?.members || []) {
      addMemberOption(
        normalizeOptionalText(member.id),
        normalizeOptionalText(member.name),
        member.is_active ? 'running' : 'idle',
      )
    }

    for (const [agentId, agentName] of runtimeSpecAgentNameById.value.entries()) {
      addMemberOption(
        normalizeOptionalText(agentId),
        normalizeOptionalText(agentName),
        statusByMemberKey.get(`id:${agentId}`),
      )
    }

    for (const task of params.teamTasks.value || []) {
      const assigneeId = normalizeOptionalText(task.assignee_agent_id)
      if (!assigneeId) continue
      const matchedMember = (params.teamSessionDetail.value?.members || []).find((item) => item.id === assigneeId)
      const preferredName =
        normalizeOptionalText(matchedMember?.name) ||
        runtimeSpecAgentNameById.value.get(assigneeId) ||
        assigneeId
      addMemberOption(assigneeId, preferredName, statusByMemberKey.get(`id:${assigneeId}`))
    }

    for (const message of params.messages.value) {
      if (!params.isTeamScopedMainFlowMessage(message)) continue
      const memberId = normalizeOptionalText(message.metadata?.team_member_id)
      const memberName = normalizeOptionalText(message.metadata?.team_member_name)
      if (!memberId && !memberName) continue
      addMemberOption(memberId, memberName)
    }

    return options
  })

  const selectedTeamTask = computed(() =>
    params.teamTasks.value.find((task) => task.id === params.selectedTeamTaskId.value) || null,
  )

  const selectedTeamTaskMemberKey = computed(() => {
    const assigneeId = normalizeOptionalText(selectedTeamTask.value?.assignee_agent_id)
    if (!assigneeId) return TEAM_SPLIT_ALL_MEMBER_KEY
    const matched = teamSplitMembers.value.find((member) => member.memberId === assigneeId)
    if (matched?.key) return matched.key
    const byIdKey = `id:${assigneeId}`
    if (teamSplitMembers.value.some((member) => member.key === byIdKey)) return byIdKey
    return TEAM_SPLIT_ALL_MEMBER_KEY
  })

  const teamSplitMemberByKey = computed(() => {
    const out = new Map<string, TeamSplitMemberOption>()
    for (const member of teamSplitMembers.value) {
      out.set(member.key, member)
    }
    return out
  })

  const matchesSelectedTeamMember = (message: AgentMessage, member: TeamSplitMemberOption): boolean => {
    const metadata = message.metadata || {}
    const messageMemberId = String(metadata.team_member_id || '').trim()
    const messageMemberName = String(metadata.team_member_name || '').trim()
    if (member.memberId && messageMemberId === member.memberId) return true
    if (member.memberName && messageMemberName === member.memberName) return true
    return false
  }

  const isTeamMessageGlobalVisible = (message: AgentMessage): boolean => {
    const metadata = message.metadata || {}
    const kind = String(metadata.kind || '').trim().toLowerCase()
    const role = String(metadata.team_member_role || '').trim().toLowerCase()
    if (message.type === 'user' || message.type === 'system') return true
    if (kind === 'team_bridge' || kind === 'team_system' || kind === 'team_human_input') return true
    if (role === 'user' || role === 'system' || role === 'human') return true
    const memberId = String(metadata.team_member_id || '').trim()
    const memberName = String(metadata.team_member_name || '').trim()
    return !memberId && !memberName
  }

  const visibleMessages = computed<AgentMessage[]>(() => {
    const all = params.messages.value
    if (!params.teamModeEnabled.value) return all
    if (!params.selectedTeamTaskId.value) return all
    const memberKey = selectedTeamTaskMemberKey.value
    if (!memberKey || memberKey === TEAM_SPLIT_ALL_MEMBER_KEY) return all
    const member = teamSplitMemberByKey.value.get(memberKey)
    if (!member) return all
    return all.filter((message) => {
      if (!params.isTeamScopedMainFlowMessage(message)) return true
      if (isTeamMessageGlobalVisible(message)) return true
      return matchesSelectedTeamMember(message, member)
    })
  })

  const resolveAgentName = (agentId?: string | null) => {
    if (!agentId) return 'broadcast'
    const matched = (params.teamSessionDetail.value?.members || []).find((member) => member.id === agentId)
    const runtimeName = runtimeSpecAgentNameById.value.get(agentId)
    const displayName = (matched?.name || runtimeName || '').trim()
    if (!displayName || displayName === agentId) return agentId
    return `${agentId} (${displayName})`
  }

  const selectedTeamTaskTitle = computed(() => {
    const task = selectedTeamTask.value
    if (!task) return ''
    return task.title || resolveAgentName(task.assignee_agent_id)
  })

  const teamWorkspaceBadgeCount = computed(
    () => params.teamTasks.value.filter((task) => ['failed', 'blocked'].includes((task.status || '').toLowerCase())).length,
  )

  const clearSelectedTeamTask = () => {
    params.selectedTeamTaskId.value = null
  }

  const toggleSelectedTeamTask = (task: TeamTask) => {
    const nextId = task.id
    if (!nextId) {
      params.selectedTeamTaskId.value = null
      return
    }
    params.selectedTeamTaskId.value = params.selectedTeamTaskId.value === nextId ? null : nextId
  }

  const teamBlackboardEntryBadgeClass = (entryType?: string | null) => {
    const normalized = String(entryType || '').toLowerCase()
    if (normalized === 'task_output') return 'badge-success'
    if (normalized === 'task_error') return 'badge-error'
    if (normalized === 'task_start') return 'badge-info'
    if (normalized === 'plan') return 'badge-secondary'
    if (normalized === 'plan_fallback') return 'badge-warning'
    if (normalized === 'goal') return 'badge-accent'
    return 'badge-ghost'
  }

  const formatTimestamp = (value?: string | null) => {
    if (!value) return '—'
    const time = new Date(value).getTime()
    if (!Number.isFinite(time)) return value
    return new Date(time).toLocaleString()
  }

  const teamOrchestrationRuntime = computed<Record<string, any>>(() => {
    const raw = params.teamSessionDetail.value?.state_machine?.orchestration_runtime
    if (raw && typeof raw === 'object') return raw as Record<string, any>
    return {}
  })

  const teamMemberNameOptions = computed(() =>
    (params.teamSessionDetail.value?.members || [])
      .map((member) => member.name)
      .filter((name) => typeof name === 'string' && name.trim().length > 0),
  )

  const teamOrchestrationPresets = computed(() => TEAM_ORCHESTRATION_PRESET_METAS)
  const teamRecoveryPresets = computed(() => TEAM_RECOVERY_PRESETS)
  const teamSelectedOrchestrationPresetDescription = computed(() => {
    if (!params.teamSelectedOrchestrationPresetId.value) return ''
    return TEAM_ORCHESTRATION_PRESET_METAS.find((item) => item.id === params.teamSelectedOrchestrationPresetId.value)?.description || ''
  })
  const teamSelectedRecoveryPresetDescription = computed(() => {
    const selected = TEAM_RECOVERY_PRESETS.find((item) => item.id === params.teamSelectedRecoveryPresetId.value)
    return selected?.description || ''
  })

  const teamCurrentNoHumanInputPolicy = computed<TeamRecoveryPresetId>(() => {
    const fallback = 'balanced'
    const stateMachine = params.teamSessionDetail.value?.state_machine
    if (!stateMachine || typeof stateMachine !== 'object') return fallback
    const fromIntervention = (stateMachine as any)?.human_intervention?.policy
    const fromRoot = (stateMachine as any)?.no_human_input_policy
    const raw = typeof fromIntervention === 'string' ? fromIntervention : fromRoot
    if (raw === 'conservative' || raw === 'aggressive') return raw
    return fallback
  })

  const teamCurrentHumanInterventionTimeoutSecs = computed(() => {
    const stateMachine = params.teamSessionDetail.value?.state_machine
    const fromIntervention = Number((stateMachine as any)?.human_intervention?.timeout_secs)
    if (Number.isFinite(fromIntervention) && fromIntervention > 0) return Math.floor(fromIntervention)
    const fromRoot = Number((stateMachine as any)?.human_intervention_timeout_secs)
    if (Number.isFinite(fromRoot) && fromRoot > 0) return Math.floor(fromRoot)
    return 600
  })

  const teamCurrentMaxHumanInterventions = computed(() => {
    const stateMachine = params.teamSessionDetail.value?.state_machine
    const value = Number((stateMachine as any)?.max_human_interventions)
    if (Number.isFinite(value) && value > 0) return Math.floor(value)
    return 3
  })

  const teamFlattenedStepOptions = computed(() => {
    const options: Array<{ id: string; path: string; label: string }> = []
    const walk = (steps: TeamOrchestrationPlan['steps'], prefix: number[]) => {
      steps.forEach((step, idx) => {
        const path = [...prefix, idx]
        const pathLabel = path.map((part) => part + 1).join('.')
        const title = step.name?.trim() || step.phase?.trim() || step.type
        options.push({
          id: step.id,
          path: pathLabel,
          label: `${step.id} (${pathLabel}) · ${title}`,
        })
        if (Array.isArray(step.children) && step.children.length > 0) {
          walk(step.children, path)
        }
      })
    }
    walk(params.teamOrchestrationDraft.value.steps, [])
    return options
  })

  const teamStepPathById = computed(() => {
    const pathMap = new Map<string, string>()
    teamFlattenedStepOptions.value.forEach((item) => {
      if (!pathMap.has(item.id)) {
        pathMap.set(item.id, item.path)
      }
    })
    return pathMap
  })

  const teamLastRuntimeStepPath = computed(() => {
    const lastStepId = teamOrchestrationRuntime.value.last_step_id
    if (typeof lastStepId !== 'string' || !lastStepId.trim()) return '-'
    return teamStepPathById.value.get(lastStepId) || '-'
  })

  const teamRuntimeSummary = computed(() => {
    const raw = teamOrchestrationRuntime.value.summary
    const totalAttempts = Number(raw?.total_attempts ?? 0)
    const totalSuccess = Number(raw?.total_success ?? 0)
    const totalFailed = Number(raw?.total_failed ?? 0)
    const slowestDurationMs = Number(raw?.slowest_duration_ms ?? 0)
    const slowestStepId = typeof raw?.slowest_step_id === 'string' ? raw.slowest_step_id : ''
    return {
      totalAttempts: Number.isFinite(totalAttempts) ? Math.max(0, Math.floor(totalAttempts)) : 0,
      totalSuccess: Number.isFinite(totalSuccess) ? Math.max(0, Math.floor(totalSuccess)) : 0,
      totalFailed: Number.isFinite(totalFailed) ? Math.max(0, Math.floor(totalFailed)) : 0,
      slowestDurationMs: Number.isFinite(slowestDurationMs) ? Math.max(0, Math.floor(slowestDurationMs)) : 0,
      slowestStepId,
    }
  })

  const teamRuntimeSuggestedResumeStepId = computed(() => {
    const value = teamOrchestrationRuntime.value.suggested_resume_step_id
    return typeof value === 'string' ? value : ''
  })

  const teamRuntimeStepStats = computed<TeamRuntimeStepStat[]>(() => {
    const raw = teamOrchestrationRuntime.value.step_stats
    if (!raw || typeof raw !== 'object') return []
    return Object.entries(raw)
      .map(([stepId, value]) => {
        const v = value as Record<string, any>
        const toNum = (n: any) => {
          const parsed = Number(n ?? 0)
          return Number.isFinite(parsed) ? Math.max(0, Math.floor(parsed)) : 0
        }
        return {
          step_id: stepId,
          total_attempts: toNum(v.total_attempts),
          success_count: toNum(v.success_count),
          failure_count: toNum(v.failure_count),
          avg_duration_ms: toNum(v.avg_duration_ms),
          last_duration_ms: toNum(v.last_duration_ms),
          last_status: typeof v.last_status === 'string' ? v.last_status : '',
          last_error: typeof v.last_error === 'string' ? v.last_error : '',
        }
      })
      .sort((a, b) => b.failure_count - a.failure_count || b.avg_duration_ms - a.avg_duration_ms)
  })

  const teamRuntimeHotspots = computed(() => teamRuntimeStepStats.value.slice(0, 8))

  const teamRuntimeFailureModes = computed<TeamRuntimeFailureMode[]>(() => {
    const raw = teamOrchestrationRuntime.value.failure_modes
    if (!raw || typeof raw !== 'object') return []
    return Object.entries(raw)
      .map(([mode, value]) => {
        const v = value as Record<string, any>
        const countRaw = Number(v.count ?? 0)
        const count = Number.isFinite(countRaw) ? Math.max(0, Math.floor(countRaw)) : 0
        return {
          mode,
          count,
          latest_step_id: typeof v.latest_step_id === 'string' ? v.latest_step_id : '',
          latest_error: typeof v.latest_error === 'string' ? v.latest_error : '',
          hint: typeof v.hint === 'string' ? v.hint : '',
        }
      })
      .sort((a, b) => b.count - a.count)
  })

  const teamRuntimeBackendRecoverySuggestions = computed(() => {
    const raw = teamOrchestrationRuntime.value.recovery_suggestions
    if (!Array.isArray(raw)) return []
    return raw
      .filter((item) => typeof item === 'string')
      .map((item) => String(item).trim())
      .filter((item) => item.length > 0)
  })

  const teamRuntimeRecoverySuggestions = computed(() => {
    const hints: string[] = [...teamRuntimeBackendRecoverySuggestions.value]
    const seen = new Set(hints)
    const pushHint = (msg: string) => {
      const normalized = msg.trim()
      if (!normalized) return
      if (seen.has(normalized)) return
      seen.add(normalized)
      hints.push(normalized)
    }
    if (teamRuntimeSummary.value.totalFailed > 0 && teamRuntimeSuggestedResumeStepId.value) {
      pushHint(`优先从失败节点 ${teamRuntimeSuggestedResumeStepId.value} 恢复执行。`)
    }
    const frequentFailure = teamRuntimeStepStats.value.find((item) => item.failure_count >= 3)
    if (frequentFailure) {
      pushHint(`节点 ${frequentFailure.step_id} 连续失败较多，建议提高 backoff 或拆分任务。`)
    }
    if (teamRuntimeSummary.value.slowestDurationMs >= 120000 && teamRuntimeSummary.value.slowestStepId) {
      pushHint(`慢节点 ${teamRuntimeSummary.value.slowestStepId} 耗时较长，建议拆分或并行化。`)
    }
    if (hints.length === 0) {
      hints.push('当前执行稳定，可继续按既定编排运行。')
    }
    return hints
  })

  return {
    clearSelectedTeamTask,
    formatTimestamp,
    resolveAgentName,
    selectedTeamTask,
    selectedTeamTaskTitle,
    teamBlackboardEntryBadgeClass,
    teamCurrentHumanInterventionTimeoutSecs,
    teamCurrentMaxHumanInterventions,
    teamCurrentNoHumanInputPolicy,
    teamFlattenedStepOptions,
    teamLastRuntimeStepPath,
    teamMemberNameOptions,
    teamOrchestrationPresets,
    teamOrchestrationRuntime,
    teamRecoveryPresets,
    teamRuntimeBackendRecoverySuggestions,
    teamRuntimeFailureModes,
    teamRuntimeHotspots,
    teamRuntimeRecoverySuggestions,
    teamRuntimeStepStats,
    teamRuntimeSuggestedResumeStepId,
    teamRuntimeSummary,
    teamSelectedOrchestrationPresetDescription,
    teamSelectedRecoveryPresetDescription,
    teamSplitMembers,
    teamWorkspaceBadgeCount,
    toggleSelectedTeamTask,
    visibleMessages,
  }
}
