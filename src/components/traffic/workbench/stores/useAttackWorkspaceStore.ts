import { computed, ref } from 'vue'
import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { IntruderPosition, IntruderTarget } from '../../intruder/types'
import type { DraftRevision } from '../model/draftRevision'
import type { RequestDraft } from '../model/requestDraft'
import type { AttackWorkspace, AttackWorkspaceProgress, AttackWorkspaceRunState } from '../model/attackWorkspace'
import {
  createAttackWorkspaceFromDraft,
  createAttackWorkspaceFromExchangeRequest,
} from '../services/attackWorkspaceFactory'

const workspacesState = ref<AttackWorkspace[]>([])
const activeWorkspaceIdState = ref<string | null>(null)

function findWorkspace(workspaceId: string | null) {
  if (!workspaceId) {
    return null
  }
  return workspacesState.value.find(workspace => workspace.id === workspaceId) ?? null
}

function createWorkspaceFromExchangeRequest(options: {
  request: HttpExchangeRequest
  source: TrafficWorkbenchSource | null
  title?: string
}) {
  const workspace = createAttackWorkspaceFromExchangeRequest(options)
  workspacesState.value = [...workspacesState.value, workspace]
  activeWorkspaceIdState.value = workspace.id
  return workspace
}

function createWorkspaceFromDraft(options: { draft: RequestDraft; revision: DraftRevision }) {
  const workspace = createAttackWorkspaceFromDraft(options)
  workspacesState.value = [...workspacesState.value, workspace]
  activeWorkspaceIdState.value = workspace.id
  return workspace
}

function selectWorkspace(workspaceId: string | null) {
  activeWorkspaceIdState.value = workspaceId
}

function updateWorkspaceRequestText(workspaceId: string, requestText: string) {
  const now = Date.now()
  workspacesState.value = workspacesState.value.map(workspace =>
    workspace.id === workspaceId
      ? {
        ...workspace,
        requestText,
        updatedAt: now,
      }
      : workspace,
  )
}

function updateWorkspaceTarget(workspaceId: string, target: IntruderTarget) {
  const now = Date.now()
  workspacesState.value = workspacesState.value.map(workspace =>
    workspace.id === workspaceId
      ? {
        ...workspace,
        target: { ...target },
        updatedAt: now,
      }
      : workspace,
  )
}

function updateWorkspacePositions(workspaceId: string, positions: IntruderPosition[]) {
  const now = Date.now()
  workspacesState.value = workspacesState.value.map(workspace =>
    workspace.id === workspaceId
      ? {
        ...workspace,
        positions: [...positions],
        updatedAt: now,
      }
      : workspace,
  )
}

function updateWorkspaceTitle(workspaceId: string, title: string) {
  const nextTitle = title.trim()
  if (!nextTitle) {
    return
  }

  const now = Date.now()
  workspacesState.value = workspacesState.value.map(workspace =>
    workspace.id === workspaceId
      ? {
        ...workspace,
        title: nextTitle,
        updatedAt: now,
      }
      : workspace,
  )
}

function updateWorkspaceRuntime(
  workspaceId: string,
  runtime: {
    runState: AttackWorkspaceRunState
    resultCount: number
    progress: AttackWorkspaceProgress
  },
) {
  const now = Date.now()
  workspacesState.value = workspacesState.value.map(workspace =>
    workspace.id === workspaceId
      ? {
        ...workspace,
        runState: runtime.runState,
        resultCount: runtime.resultCount,
        progress: { ...runtime.progress },
        updatedAt: now,
      }
      : workspace,
  )
}

function replaceState(workspaces: AttackWorkspace[], activeWorkspaceId: string | null) {
  workspacesState.value = workspaces.map(workspace => ({
    ...workspace,
    target: { ...workspace.target },
    positions: [...workspace.positions],
    progress: { ...workspace.progress },
  }))
  activeWorkspaceIdState.value = activeWorkspaceId
    && workspacesState.value.some(workspace => workspace.id === activeWorkspaceId)
    ? activeWorkspaceId
    : workspacesState.value.at(-1)?.id ?? null
}

function removeWorkspace(workspaceId: string) {
  workspacesState.value = workspacesState.value.filter(workspace => workspace.id !== workspaceId)
  if (activeWorkspaceIdState.value === workspaceId) {
    activeWorkspaceIdState.value = workspacesState.value.at(-1)?.id ?? null
  }
}

function resetAttackWorkspaceStore() {
  workspacesState.value = []
  activeWorkspaceIdState.value = null
}

export function useAttackWorkspaceStore() {
  const activeWorkspace = computed(() => findWorkspace(activeWorkspaceIdState.value))

  return {
    workspaces: workspacesState,
    activeWorkspaceId: activeWorkspaceIdState,
    activeWorkspace,
    createWorkspaceFromExchangeRequest,
    createWorkspaceFromDraft,
    selectWorkspace,
    updateWorkspaceRequestText,
    updateWorkspaceTarget,
    updateWorkspacePositions,
    updateWorkspaceTitle,
    updateWorkspaceRuntime,
    replaceState,
    removeWorkspace,
    resetAttackWorkspaceStore,
  }
}
