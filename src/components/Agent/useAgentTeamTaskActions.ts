import { ref, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'
import { agentTeamApi } from '@/api/agentTeam'
import type { TeamTask, TeamTaskCreateInput, TeamTaskReasonActionInput } from '@/types/agentTeam'
import { buildTeamTaskActionToastMessage, getTeamTaskClaimAgentId, getTeamTaskReleaseAgentId } from './teamTaskActionsSupport'
import { buildTeamTaskCreateRequest } from './teamTaskCreateSupport'

export const useAgentTeamTaskActions = (params: {
  activeTeamSessionId: Ref<string | null>
  loadTeamWorkspaceData: () => Promise<void>
}) => {
  const { t } = useI18n()
  const toast = useToast()

  const pendingTeamCreateTask = ref(false)
  const pendingTeamTaskActionTaskId = ref<string | null>(null)
  const pendingTeamTaskActionKind = ref<'claim' | 'release' | 'complete' | 'fail' | 'block' | null>(null)

  const showTeamTaskActionToast = (result: { success: boolean; message: string; reason?: string | null; next_step?: string | null }) => {
    const message = buildTeamTaskActionToastMessage(result)
    if (result.success) {
      toast.success(message, 2600)
      return
    }
    toast.warning(message, 3600)
  }

  const handleCreateTeamTask = async (input: TeamTaskCreateInput) => {
    const sessionId = String(params.activeTeamSessionId.value || '').trim()
    if (!sessionId) return

    pendingTeamCreateTask.value = true
    try {
      const result = await agentTeamApi.createTask(
        sessionId,
        buildTeamTaskCreateRequest(input),
      )
      showTeamTaskActionToast(result)
      await params.loadTeamWorkspaceData()
    } finally {
      pendingTeamCreateTask.value = false
    }
  }

  const runTeamTaskAction = async (
    kind: 'claim' | 'release' | 'complete' | 'fail' | 'block',
    task: TeamTask,
    reason?: string | null,
  ) => {
    const sessionId = String(params.activeTeamSessionId.value || '').trim()
    if (!sessionId) return
    if (kind === 'claim' || kind === 'release') {
      const actorId = kind === 'claim'
        ? getTeamTaskClaimAgentId(task)
        : getTeamTaskReleaseAgentId(task)
      if (!actorId) {
        toast.warning(
          kind === 'claim'
            ? t('agent.teamTaskActionClaimMissingActor')
            : t('agent.teamTaskActionReleaseMissingActor'),
          3200,
        )
        return
      }
    }

    pendingTeamTaskActionTaskId.value = task.id
    pendingTeamTaskActionKind.value = kind
    try {
      const result = kind === 'claim'
        ? await agentTeamApi.claimTask(sessionId, task.id, getTeamTaskClaimAgentId(task)!)
        : kind === 'release'
          ? await agentTeamApi.releaseTaskClaim(sessionId, task.id, getTeamTaskReleaseAgentId(task)!)
          : await agentTeamApi.updateTaskStatus(
            sessionId,
            task.id,
            kind === 'complete' ? 'completed' : (kind === 'fail' ? 'failed' : 'blocked'),
            kind === 'fail'
              ? ((reason || '').trim() || task.last_error || t('agent.teamTaskActionFailDefaultReason'))
              : kind === 'block'
                ? ((reason || '').trim() || task.last_error || t('agent.teamTaskActionBlockDefaultReason'))
                : null,
          )
      showTeamTaskActionToast(result)
      await params.loadTeamWorkspaceData()
    } finally {
      pendingTeamTaskActionTaskId.value = null
      pendingTeamTaskActionKind.value = null
    }
  }

  const handleClaimTeamTask = async (task: TeamTask) => {
    await runTeamTaskAction('claim', task)
  }

  const handleReleaseTeamTask = async (task: TeamTask) => {
    await runTeamTaskAction('release', task)
  }

  const handleCompleteTeamTask = async (task: TeamTask) => {
    await runTeamTaskAction('complete', task)
  }

  const handleFailTeamTask = async (input: TeamTaskReasonActionInput) => {
    await runTeamTaskAction('fail', input.task, input.reason)
  }

  const handleBlockTeamTask = async (input: TeamTaskReasonActionInput) => {
    await runTeamTaskAction('block', input.task, input.reason)
  }

  return {
    handleBlockTeamTask,
    handleClaimTeamTask,
    handleCompleteTeamTask,
    handleCreateTeamTask,
    handleFailTeamTask,
    handleReleaseTeamTask,
    pendingTeamCreateTask,
    pendingTeamTaskActionKind,
    pendingTeamTaskActionTaskId,
  }
}
