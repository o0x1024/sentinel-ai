import { createRawRequestFromHttpExchangeRequest } from '../../http/parser'
import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { DraftRevision } from '../model/draftRevision'
import type { RequestDraft } from '../model/requestDraft'
import { createWorkbenchEntityId } from './id'

export function createRequestDraftFromExchangeRequest(options: {
  request: HttpExchangeRequest
  source: TrafficWorkbenchSource | null
  sourceSnapshotId?: string | null
  title?: string
}): {
  draft: RequestDraft
  initialRevision: DraftRevision
} {
  const draftId = createWorkbenchEntityId('traffic-draft')
  const revisionId = createWorkbenchEntityId('traffic-draft-revision')
  const now = Date.now()
  const rawRequest = createRawRequestFromHttpExchangeRequest(options.request)
  const title = options.title?.trim() || options.request.endpoint.host || options.request.absoluteUrl

  return {
    draft: {
      id: draftId,
      title,
      source: options.source,
      sourceSnapshotId: options.sourceSnapshotId ?? null,
      endpoint: { ...options.request.endpoint },
      endpointMode: 'auto',
      rawRequest,
      preferredView: options.request.preferredRequestView === 'raw' ? 'raw' : 'pretty',
      pinned: false,
      activeRevisionId: revisionId,
      createdAt: now,
      updatedAt: now,
    },
    initialRevision: {
      id: revisionId,
      draftId,
      endpoint: { ...options.request.endpoint },
      rawRequest,
      reason: 'clone',
      createdAt: now,
    },
  }
}
