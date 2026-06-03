import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import { createRawRequestFromSource, extractTargetFromRequest } from '../../intruder/http'
import { extractIntruderPositions } from '../../intruder/attack'
import type { AttackWorkspace } from '../model/attackWorkspace'
import type { DraftRevision } from '../model/draftRevision'
import type { RequestDraft } from '../model/requestDraft'
import { createWorkbenchEntityId } from './id'

export function createAttackWorkspaceFromExchangeRequest(options: {
  request: HttpExchangeRequest
  source: TrafficWorkbenchSource | null
  title?: string
}): AttackWorkspace {
  const requestText = createRawRequestFromSource(options.request)
  const target = extractTargetFromRequest(requestText, options.request.absoluteUrl)
  const now = Date.now()

  return {
    id: createWorkbenchEntityId('traffic-attack-workspace'),
    title: options.title?.trim() || target.host || options.request.absoluteUrl,
    source: options.source,
    sourceDraftId: null,
    sourceDraftRevisionId: null,
    requestText,
    target,
    positions: extractIntruderPositions(requestText),
    runState: 'idle',
    resultCount: 0,
    progress: {
      total: 0,
      completed: 0,
      failed: 0,
      active: 0,
      truncated: false,
    },
    createdAt: now,
    updatedAt: now,
  }
}

export function createAttackWorkspaceFromDraft(options: {
  draft: RequestDraft
  revision: DraftRevision
}): AttackWorkspace {
  const target = extractTargetFromRequest(options.revision.rawRequest)
  const now = Date.now()

  return {
    id: createWorkbenchEntityId('traffic-attack-workspace'),
    title: options.draft.title,
    source: options.draft.source,
    sourceDraftId: options.draft.id,
    sourceDraftRevisionId: options.revision.id,
    requestText: options.revision.rawRequest,
    target,
    positions: extractIntruderPositions(options.revision.rawRequest),
    runState: 'idle',
    resultCount: 0,
    progress: {
      total: 0,
      completed: 0,
      failed: 0,
      active: 0,
      truncated: false,
    },
    createdAt: now,
    updatedAt: now,
  }
}
