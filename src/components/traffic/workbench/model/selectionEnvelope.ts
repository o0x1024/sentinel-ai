import type { HttpEndpoint, HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { AttackWorkspace } from './attackWorkspace'
import type { HistorySnapshot } from './historySnapshot'
import type { ReplayRun } from './replayRun'
import type { RequestDraft } from './requestDraft'

export type TrafficSelectionKind =
  | 'history-snapshot'
  | 'draft'
  | 'replay-run'
  | 'attack-workspace'

interface TrafficSelectionEnvelopeBase {
  id: string
  kind: TrafficSelectionKind
  title: string
  source: TrafficWorkbenchSource | null
}

export interface HistorySnapshotSelectionEnvelope extends TrafficSelectionEnvelopeBase {
  kind: 'history-snapshot'
  requestId: number | null
  dbRequestId: number | null
  trafficRequestId: string | null
  variant: HistorySnapshot['variant']
  request: HttpExchangeRequest
}

export interface DraftSelectionEnvelope extends TrafficSelectionEnvelopeBase {
  kind: 'draft'
  draftId: string
  sourceSnapshotId: string | null
  endpoint: HttpEndpoint
  rawRequest: string
}

export interface ReplayRunSelectionEnvelope extends TrafficSelectionEnvelopeBase {
  kind: 'replay-run'
  draftId: string
  draftRevisionId: string
  run: ReplayRun
}

export interface AttackWorkspaceSelectionEnvelope extends TrafficSelectionEnvelopeBase {
  kind: 'attack-workspace'
  workspaceId: string
  sourceDraftId: string | null
  sourceDraftRevisionId: string | null
  requestText: string
}

export type TrafficSelectionEnvelope =
  | HistorySnapshotSelectionEnvelope
  | DraftSelectionEnvelope
  | ReplayRunSelectionEnvelope
  | AttackWorkspaceSelectionEnvelope

export function createHistorySnapshotSelectionEnvelope(
  snapshot: HistorySnapshot,
): HistorySnapshotSelectionEnvelope {
  return {
    id: snapshot.id,
    kind: 'history-snapshot',
    title: snapshot.source.label,
    source: snapshot.source,
    requestId: snapshot.requestId,
    dbRequestId: snapshot.dbRequestId,
    trafficRequestId: snapshot.trafficRequestId,
    variant: snapshot.variant,
    request: snapshot.request,
  }
}

export function createDraftSelectionEnvelope(draft: RequestDraft): DraftSelectionEnvelope {
  return {
    id: draft.id,
    kind: 'draft',
    title: draft.title,
    source: draft.source,
    draftId: draft.id,
    sourceSnapshotId: draft.sourceSnapshotId,
    endpoint: { ...draft.endpoint },
    rawRequest: draft.rawRequest,
  }
}

export function createReplayRunSelectionEnvelope(
  run: ReplayRun,
  draft: RequestDraft | null,
): ReplayRunSelectionEnvelope {
  return {
    id: run.id,
    kind: 'replay-run',
    title: draft?.title || run.id,
    source: draft?.source ?? null,
    draftId: run.draftId,
    draftRevisionId: run.draftRevisionId,
    run,
  }
}

export function createAttackWorkspaceSelectionEnvelope(
  workspace: AttackWorkspace,
): AttackWorkspaceSelectionEnvelope {
  return {
    id: workspace.id,
    kind: 'attack-workspace',
    title: workspace.title,
    source: workspace.source,
    workspaceId: workspace.id,
    sourceDraftId: workspace.sourceDraftId,
    sourceDraftRevisionId: workspace.sourceDraftRevisionId,
    requestText: workspace.requestText,
  }
}
