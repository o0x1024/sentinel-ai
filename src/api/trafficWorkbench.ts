import { invoke } from '@tauri-apps/api/core'
import type { AttackWorkspace } from '@/components/traffic/workbench/model/attackWorkspace'
import type { DraftRevision } from '@/components/traffic/workbench/model/draftRevision'
import type { ReplayRun } from '@/components/traffic/workbench/model/replayRun'
import type { RequestDraft } from '@/components/traffic/workbench/model/requestDraft'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export interface PersistedTrafficDraftRecord {
  draft: RequestDraft
  revisions: DraftRevision[]
}

export interface PersistedTrafficDraftStore {
  activeDraftId: string | null
  drafts: PersistedTrafficDraftRecord[]
}

export type PersistedTrafficAttackWorkspace = Pick<
  AttackWorkspace,
  | 'id'
  | 'title'
  | 'source'
  | 'sourceDraftId'
  | 'sourceDraftRevisionId'
  | 'requestText'
  | 'target'
  | 'positions'
  | 'runState'
  | 'resultCount'
  | 'progress'
  | 'createdAt'
  | 'updatedAt'
>

export interface PersistedTrafficAttackWorkspaceStore {
  activeWorkspaceId: string | null
  workspaces: PersistedTrafficAttackWorkspace[]
}

export interface PersistedReplayRunStore {
  replayRuns: ReplayRun[]
}

export interface PersistedTrafficComparerStore {
  activeItemId: string | null
  items: unknown[]
  viewMode?: string | null
  compareRenderMode?: string | null
  pinnedBaseline?: unknown | null
  showDraftComposer?: boolean | null
  draftSequence?: number | null
  draft?: unknown | null
}

async function expectCommandData<T>(command: string, response: CommandResponse<T>) {
  if (!response.success) {
    throw new Error(response.error || `Failed to execute ${command}`)
  }
  if (response.data === undefined) {
    throw new Error(`Missing response data for ${command}`)
  }
  return response.data
}

export async function loadTrafficDraftStore(): Promise<PersistedTrafficDraftStore> {
  const response = await invoke<CommandResponse<PersistedTrafficDraftStore>>('load_traffic_draft_store')
  return expectCommandData('load_traffic_draft_store', response)
}

export async function saveTrafficDraftStore(store: PersistedTrafficDraftStore): Promise<void> {
  const response = await invoke<CommandResponse<null>>('save_traffic_draft_store', { store })
  await expectCommandData('save_traffic_draft_store', {
    ...response,
    data: response.data ?? null,
  })
}

export async function loadAttackWorkspaceStore(): Promise<PersistedTrafficAttackWorkspaceStore> {
  const response = await invoke<CommandResponse<PersistedTrafficAttackWorkspaceStore>>('load_attack_workspace_store')
  return expectCommandData('load_attack_workspace_store', response)
}

export async function saveAttackWorkspaceStore(store: PersistedTrafficAttackWorkspaceStore): Promise<void> {
  const response = await invoke<CommandResponse<null>>('save_attack_workspace_store', { store })
  await expectCommandData('save_attack_workspace_store', {
    ...response,
    data: response.data ?? null,
  })
}

export async function loadReplayRunStore(): Promise<PersistedReplayRunStore> {
  const response = await invoke<CommandResponse<PersistedReplayRunStore>>('load_replay_run_store')
  return expectCommandData('load_replay_run_store', response)
}

export async function saveReplayRunStore(store: PersistedReplayRunStore): Promise<void> {
  const response = await invoke<CommandResponse<null>>('save_replay_run_store', { store })
  await expectCommandData('save_replay_run_store', {
    ...response,
    data: response.data ?? null,
  })
}

export async function loadComparerStore(): Promise<PersistedTrafficComparerStore> {
  const response = await invoke<CommandResponse<PersistedTrafficComparerStore>>('load_comparer_store')
  return expectCommandData('load_comparer_store', response)
}

export async function saveComparerStore(store: PersistedTrafficComparerStore): Promise<void> {
  const response = await invoke<CommandResponse<null>>('save_comparer_store', { store })
  await expectCommandData('save_comparer_store', {
    ...response,
    data: response.data ?? null,
  })
}
