import type {
  PersistedReplayRunStore,
  PersistedTrafficAttackWorkspace,
  PersistedTrafficAttackWorkspaceStore,
  PersistedTrafficDraftRecord,
  PersistedTrafficDraftStore,
} from '@/api/trafficWorkbench'
import type { AttackWorkspace } from '../model/attackWorkspace'
import type { DraftRevision } from '../model/draftRevision'
import type { ReplayRun } from '../model/replayRun'
import type { RequestDraft } from '../model/requestDraft'

export function toPersistedDraftRecords(
  drafts: RequestDraft[],
  revisionsByDraftId: Record<string, DraftRevision[]>,
): PersistedTrafficDraftRecord[] {
  return drafts.map(draft => ({
    draft: {
      ...draft,
      endpoint: { ...draft.endpoint },
    },
    revisions: (revisionsByDraftId[draft.id] || []).map(revision => ({
      ...revision,
      endpoint: { ...revision.endpoint },
    })),
  }))
}

export function toPersistedAttackWorkspaces(
  workspaces: AttackWorkspace[],
): PersistedTrafficAttackWorkspace[] {
  return workspaces.map(workspace => ({
    id: workspace.id,
    title: workspace.title,
    source: workspace.source ? { ...workspace.source } : null,
    sourceDraftId: workspace.sourceDraftId,
    sourceDraftRevisionId: workspace.sourceDraftRevisionId,
    requestText: workspace.requestText,
    target: { ...workspace.target },
    positions: workspace.positions.map(position => ({ ...position })),
    runState: workspace.runState,
    resultCount: workspace.resultCount,
    progress: { ...workspace.progress },
    createdAt: workspace.createdAt,
    updatedAt: workspace.updatedAt,
  }))
}

export function buildPersistedDraftStore(
  drafts: RequestDraft[],
  revisionsByDraftId: Record<string, DraftRevision[]>,
  activeDraftId: string | null,
): PersistedTrafficDraftStore {
  return {
    activeDraftId,
    drafts: toPersistedDraftRecords(drafts, revisionsByDraftId),
  }
}

export function buildPersistedAttackWorkspaceStore(
  workspaces: AttackWorkspace[],
  activeWorkspaceId: string | null,
): PersistedTrafficAttackWorkspaceStore {
  return {
    activeWorkspaceId,
    workspaces: toPersistedAttackWorkspaces(workspaces),
  }
}

export function buildPersistedReplayRunStore(replayRuns: ReplayRun[]): PersistedReplayRunStore {
  return {
    replayRuns: replayRuns.map(run => ({
      ...run,
      response: run.response
        ? {
            ...run.response,
            headers: [...run.response.headers],
          }
        : null,
    })),
  }
}

export function restorePersistedDraftRecords(records: PersistedTrafficDraftRecord[]) {
  return {
    drafts: records.map(record => ({
      ...record.draft,
      endpoint: { ...record.draft.endpoint },
    })),
    revisionsByDraftId: Object.fromEntries(
      records.map(record => [
        record.draft.id,
        record.revisions.map(revision => ({
          ...revision,
          endpoint: { ...revision.endpoint },
        })),
      ]),
    ) as Record<string, DraftRevision[]>,
  }
}

export function buildAttackWorkspaceFromPersistence(workspace: PersistedTrafficAttackWorkspace) {
  return {
    ...workspace,
    target: { ...workspace.target },
    positions: workspace.positions.map(position => ({ ...position })),
    runState: workspace.runState === 'running' ? ('cancelled' as const) : workspace.runState,
    resultCount: workspace.resultCount,
    progress: { ...workspace.progress },
  }
}

export function buildReplayRunFromPersistence(run: ReplayRun): ReplayRun {
  return {
    ...run,
    state: run.state === 'running' ? 'cancelled' : run.state,
    response: run.response
      ? {
          ...run.response,
          headers: [...run.response.headers],
        }
      : null,
  }
}
