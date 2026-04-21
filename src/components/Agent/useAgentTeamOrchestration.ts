import { computed, ref, type ComputedRef, type Ref } from 'vue'
import { agentTeamApi } from '@/api/agentTeam'
import {
  buildOrchestrationPresetPlan,
  defaultOrchestrationPlan,
  getAllTeamAgentSteps,
  moveTeamStepByPath,
  nestTeamStep,
  normalizeTeamOrchestrationPlan,
  parseTeamOrchestrationPlanInput as parseTeamOrchestrationPlanInputSupport,
  promoteTeamStep,
  serializeTeamOrchestrationPlan,
  TEAM_RECOVERY_PRESETS,
} from './teamOrchestrationSupport'
import type {
  TeamOrchestrationPlan,
  TeamOrchestrationPresetId,
  TeamOrchestrationStep,
  TeamRecoveryPresetId,
  TeamStepMovePayload,
} from './teamOrchestrationTypes'

export const useAgentTeamOrchestration = (params: {
  activeTeamSessionId: Ref<string | null>
  isTeamRunActive: ComputedRef<boolean>
  loadTeamWorkspaceData: () => Promise<void>
  runTeamExecutionFromWorkspace: (message: string) => Promise<void>
  teamOrchestrationDraft: Ref<TeamOrchestrationPlan>
  teamCurrentNoHumanInputPolicy: ComputedRef<TeamRecoveryPresetId>
  teamMemberNameOptions: ComputedRef<string[]>
  teamSelectedOrchestrationPresetId: Ref<TeamOrchestrationPresetId | null>
  teamSelectedRecoveryPresetId: Ref<TeamRecoveryPresetId>
  teamSessionDetail: Ref<any | null>
}) => {
  const teamOrchestrationPlanText = ref('{\n  "version": 1,\n  "steps": []\n}')
  const teamPlanDirty = ref(false)
  const teamPlanSaving = ref(false)
  const teamPlanError = ref<string | null>(null)
  const teamPlanSuccess = ref<string | null>(null)
  const teamResumeStepId = ref('')
  const teamRecoveryPresetApplying = ref(false)

  const updateTeamOrchestrationTextFromDraft = () => {
    teamOrchestrationPlanText.value = serializeTeamOrchestrationPlan(params.teamOrchestrationDraft.value)
  }

  const syncTeamOrchestrationEditorFromSession = (force = false) => {
    const plan = params.teamSessionDetail.value?.orchestration_plan ?? defaultOrchestrationPlan()
    if (teamPlanDirty.value && !force) return
    const normalized = normalizeTeamOrchestrationPlan(plan)
    params.teamOrchestrationDraft.value = normalized
    teamOrchestrationPlanText.value = serializeTeamOrchestrationPlan(normalized)
    teamPlanDirty.value = false
    teamPlanError.value = null
    if (force || !teamResumeStepId.value.trim()) {
      const lastStepId = params.teamSessionDetail.value?.state_machine?.orchestration_runtime?.last_step_id
      teamResumeStepId.value = typeof lastStepId === 'string' ? lastStepId : ''
    }
    params.teamSelectedOrchestrationPresetId.value = null
    params.teamSelectedRecoveryPresetId.value = params.teamCurrentNoHumanInputPolicy.value
  }

  const markTeamVisualPlanDirty = () => {
    updateTeamOrchestrationTextFromDraft()
    teamPlanDirty.value = true
    teamPlanError.value = null
    teamPlanSuccess.value = null
  }

  const handleTeamOrchestrationInput = (event: Event) => {
    const target = event.target as HTMLTextAreaElement
    teamOrchestrationPlanText.value = target.value
    try {
      const parsed = JSON.parse(target.value)
      params.teamOrchestrationDraft.value = normalizeTeamOrchestrationPlan(parsed)
    } catch {
      // Keep text as source when JSON is temporarily invalid during editing.
    }
    teamPlanDirty.value = true
    teamPlanError.value = null
    teamPlanSuccess.value = null
  }

  const handleTeamReloadOrchestrationPlan = () => {
    syncTeamOrchestrationEditorFromSession(true)
    teamPlanSuccess.value = '已从会话重新载入编排计划。'
  }

  const handleTeamVisualStepsUpdated = (steps: TeamOrchestrationStep[]) => {
    params.teamOrchestrationDraft.value.steps = steps
    markTeamVisualPlanDirty()
  }

  const handleTeamApplyOrchestrationPreset = (presetId: TeamOrchestrationPresetId) => {
    const presetPlan = buildOrchestrationPresetPlan({
      memberOptions: params.teamMemberNameOptions.value,
      presetId,
      version: Math.max(1, Number(params.teamOrchestrationDraft.value.version || 1)),
    })
    const normalized = normalizeTeamOrchestrationPlan(presetPlan)
    params.teamOrchestrationDraft.value = normalized
    updateTeamOrchestrationTextFromDraft()
    teamPlanDirty.value = true
    teamPlanError.value = null
    params.teamSelectedOrchestrationPresetId.value = presetId

    const missingMemberCount = getAllTeamAgentSteps(normalized.steps)
      .filter((step) => !step.member || !step.member.trim())
      .length
    if (missingMemberCount > 0) {
      teamPlanSuccess.value = `已应用预设（${missingMemberCount} 个节点未匹配 Agent，请手动选择后保存）。`
    } else {
      teamPlanSuccess.value = '已应用编排预设，请保存后运行。'
    }
  }

  const handleTeamApplyRecoveryPreset = async (presetId: TeamRecoveryPresetId) => {
    const preset = TEAM_RECOVERY_PRESETS.find((item) => item.id === presetId)
    if (!preset) return

    teamPlanError.value = null
    teamPlanSuccess.value = null
    params.teamSelectedRecoveryPresetId.value = presetId

    const agentSteps = getAllTeamAgentSteps(params.teamOrchestrationDraft.value.steps)
    for (const step of agentSteps) {
      step.retry = {
        max_attempts: preset.max_attempts,
        backoff_ms: preset.backoff_ms,
      }
    }
    if (agentSteps.length > 0) {
      markTeamVisualPlanDirty()
    }

    if (!params.activeTeamSessionId.value) {
      teamPlanSuccess.value = '已应用恢复策略 preset（会话未激活，仅更新本地编排草稿）。'
      return
    }

    const currentStateMachine = params.teamSessionDetail.value?.state_machine && typeof params.teamSessionDetail.value.state_machine === 'object'
      ? params.teamSessionDetail.value.state_machine
      : {}
    const currentIntervention = (currentStateMachine as any)?.human_intervention && typeof (currentStateMachine as any).human_intervention === 'object'
      ? (currentStateMachine as any).human_intervention
      : {}

    const nextStateMachine = {
      ...currentStateMachine,
      no_human_input_policy: preset.no_human_input_policy,
      human_intervention_timeout_secs: preset.human_intervention_timeout_secs,
      max_human_interventions: preset.max_human_interventions,
      human_intervention: {
        ...currentIntervention,
        policy: preset.no_human_input_policy,
        timeout_secs: preset.human_intervention_timeout_secs,
      },
    }

    teamRecoveryPresetApplying.value = true
    try {
      await agentTeamApi.updateSession(params.activeTeamSessionId.value, {
        state_machine: nextStateMachine,
      })
      if (params.teamSessionDetail.value) {
        params.teamSessionDetail.value = {
          ...params.teamSessionDetail.value,
          state_machine: nextStateMachine,
        }
      }
      teamPlanSuccess.value = '已应用恢复策略 preset，并同步会话恢复配置。'
    } catch (e: any) {
      teamPlanError.value = e?.message || String(e)
    } finally {
      teamRecoveryPresetApplying.value = false
    }
  }

  const handleTeamMoveStepByPath = (payload: TeamStepMovePayload) => {
    const changed = moveTeamStepByPath(params.teamOrchestrationDraft.value.steps, payload)
    if (!changed) {
      markTeamVisualPlanDirty()
      return
    }
    markTeamVisualPlanDirty()
  }

  const handleTeamPromoteStep = (path: number[]) => {
    if (!promoteTeamStep(params.teamOrchestrationDraft.value.steps, path)) return
    markTeamVisualPlanDirty()
  }

  const handleTeamNestStep = (path: number[]) => {
    if (!nestTeamStep(params.teamOrchestrationDraft.value.steps, path)) return
    markTeamVisualPlanDirty()
  }

  const parseTeamOrchestrationPlanInput = (): any => {
    const { jsonValue, normalized } = parseTeamOrchestrationPlanInputSupport(teamOrchestrationPlanText.value)
    params.teamOrchestrationDraft.value = normalized
    return jsonValue
  }

  const handleTeamSaveOrchestrationPlan = async () => {
    if (!params.activeTeamSessionId.value) return
    teamPlanSaving.value = true
    teamPlanError.value = null
    teamPlanSuccess.value = null
    try {
      parseTeamOrchestrationPlanInput()
      teamPlanError.value = '会话编排直改入口已下线。'
    } catch (e: any) {
      teamPlanError.value = e?.message || String(e)
    } finally {
      teamPlanSaving.value = false
    }
  }

  const handleTeamStartRunWithPlan = async () => {
    if (!params.activeTeamSessionId.value || params.isTeamRunActive.value) return
    teamPlanError.value = null
    teamPlanSuccess.value = null
    try {
      if (teamPlanDirty.value) {
        await handleTeamSaveOrchestrationPlan()
        if (teamPlanError.value) return
      }
      await params.runTeamExecutionFromWorkspace('[Team] 已按当前编排计划启动执行。')
      await params.loadTeamWorkspaceData()
    } catch (e: any) {
      teamPlanError.value = e?.message || String(e)
    }
  }

  const handleTeamRetryRun = async () => {
    if (!params.activeTeamSessionId.value || params.isTeamRunActive.value) return
    teamPlanError.value = null
    teamPlanSuccess.value = null
    try {
      if (teamPlanDirty.value) {
        await handleTeamSaveOrchestrationPlan()
        if (teamPlanError.value) return
      }
      await params.runTeamExecutionFromWorkspace('[Team] 已触发重试运行。')
      await params.loadTeamWorkspaceData()
    } catch (e: any) {
      teamPlanError.value = e?.message || String(e)
    }
  }

  const handleTeamResumeFromStep = async () => {
    if (!params.activeTeamSessionId.value || params.isTeamRunActive.value) return
    teamPlanError.value = null
    teamPlanSuccess.value = null
    try {
      const stepId = teamResumeStepId.value.trim()
      if (!stepId) {
        throw new Error('请先填写要恢复的 step_id。')
      }
      const currentStateMachine = params.teamSessionDetail.value?.state_machine && typeof params.teamSessionDetail.value.state_machine === 'object'
        ? params.teamSessionDetail.value.state_machine
        : {}
      const currentRuntime = currentStateMachine?.orchestration_runtime && typeof currentStateMachine.orchestration_runtime === 'object'
        ? currentStateMachine.orchestration_runtime
        : {}
      await agentTeamApi.updateSession(params.activeTeamSessionId.value, {
        state_machine: {
          ...currentStateMachine,
          orchestration_runtime: {
            ...currentRuntime,
            resume_from_step_id: stepId,
          },
        },
      })
      await params.runTeamExecutionFromWorkspace(`[Team] 已从 step '${stepId}' 发起恢复执行。`)
      await params.loadTeamWorkspaceData()
    } catch (e: any) {
      teamPlanError.value = e?.message || String(e)
    }
  }

  const handleTeamFillResumeStep = (stepId: string) => {
    const normalized = String(stepId || '').trim()
    if (!normalized) return
    teamResumeStepId.value = normalized
    teamPlanError.value = null
    teamPlanSuccess.value = `已选择恢复节点：${normalized}`
  }

  return {
    handleTeamApplyOrchestrationPreset,
    handleTeamApplyRecoveryPreset,
    handleTeamFillResumeStep,
    handleTeamMoveStepByPath,
    handleTeamNestStep,
    handleTeamOrchestrationInput,
    handleTeamPromoteStep,
    handleTeamReloadOrchestrationPlan,
    handleTeamResumeFromStep,
    handleTeamRetryRun,
    handleTeamSaveOrchestrationPlan,
    handleTeamStartRunWithPlan,
    handleTeamVisualStepsUpdated,
    parseTeamOrchestrationPlanInput,
    syncTeamOrchestrationEditorFromSession,
    teamOrchestrationDraft: params.teamOrchestrationDraft,
    teamOrchestrationPlanText,
    teamPlanDirty,
    teamPlanError,
    teamPlanSaving,
    teamPlanSuccess,
    teamRecoveryPresetApplying,
    teamResumeStepId,
    teamSelectedOrchestrationPresetId: params.teamSelectedOrchestrationPresetId,
    teamSelectedRecoveryPresetId: params.teamSelectedRecoveryPresetId,
  }
}
