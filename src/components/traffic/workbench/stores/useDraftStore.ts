import { computed, ref } from 'vue'
import type { HttpEndpoint, HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { DraftRevision, DraftRevisionReason } from '../model/draftRevision'
import type { RequestDraft } from '../model/requestDraft'
import { createRequestDraftFromExchangeRequest } from '../services/draftFactory'
import { createWorkbenchEntityId } from '../services/id'

const draftsState = ref<RequestDraft[]>([])
const revisionsState = ref<Record<string, DraftRevision[]>>({})
const activeDraftIdState = ref<string | null>(null)

function findDraft(draftId: string) {
  return draftsState.value.find(draft => draft.id === draftId) ?? null
}

function createDraftFromExchangeRequest(options: {
  request: HttpExchangeRequest
  source: TrafficWorkbenchSource | null
  sourceSnapshotId?: string | null
  title?: string
}) {
  const { draft, initialRevision } = createRequestDraftFromExchangeRequest(options)
  draftsState.value = [...draftsState.value, draft]
  revisionsState.value = {
    ...revisionsState.value,
    [draft.id]: [initialRevision],
  }
  activeDraftIdState.value = draft.id
  return draft
}

function selectDraft(draftId: string | null) {
  activeDraftIdState.value = draftId
}

function appendRevision(draftId: string, reason: DraftRevisionReason) {
  const draft = findDraft(draftId)
  if (!draft) {
    return null
  }

  const revision: DraftRevision = {
    id: createWorkbenchEntityId('traffic-draft-revision'),
    draftId,
    endpoint: { ...draft.endpoint },
    rawRequest: draft.rawRequest,
    reason,
    createdAt: Date.now(),
  }

  revisionsState.value = {
    ...revisionsState.value,
    [draftId]: [...(revisionsState.value[draftId] || []), revision],
  }
  draftsState.value = draftsState.value.map(item =>
    item.id === draftId
      ? {
        ...item,
        activeRevisionId: revision.id,
        updatedAt: revision.createdAt,
      }
      : item,
  )
  return revision
}

function updateDraftRequest(draftId: string, rawRequest: string) {
  const now = Date.now()
  draftsState.value = draftsState.value.map(draft =>
    draft.id === draftId
      ? {
        ...draft,
        rawRequest,
        updatedAt: now,
      }
      : draft,
  )
}

function updateDraftEndpoint(draftId: string, endpoint: HttpEndpoint, mode: RequestDraft['endpointMode']) {
  const now = Date.now()
  draftsState.value = draftsState.value.map(draft =>
    draft.id === draftId
      ? {
        ...draft,
        endpoint: { ...endpoint },
        endpointMode: mode,
        updatedAt: now,
      }
      : draft,
  )
}

function pinDraft(draftId: string, pinned: boolean) {
  const now = Date.now()
  draftsState.value = draftsState.value.map(draft =>
    draft.id === draftId
      ? {
        ...draft,
        pinned,
        updatedAt: now,
      }
      : draft,
  )
}

function removeDraft(draftId: string) {
  draftsState.value = draftsState.value.filter(draft => draft.id !== draftId)
  const nextRevisions = { ...revisionsState.value }
  delete nextRevisions[draftId]
  revisionsState.value = nextRevisions

  if (activeDraftIdState.value === draftId) {
    activeDraftIdState.value = draftsState.value.at(-1)?.id ?? null
  }
}

function replaceState(
  drafts: RequestDraft[],
  revisions: Record<string, DraftRevision[]>,
  activeDraftId: string | null,
) {
  draftsState.value = drafts.map(draft => ({
    ...draft,
    endpoint: { ...draft.endpoint },
  }))
  revisionsState.value = Object.fromEntries(
    Object.entries(revisions).map(([draftId, items]) => [
      draftId,
      items.map(item => ({
        ...item,
        endpoint: { ...item.endpoint },
      })),
    ]),
  )
  activeDraftIdState.value = activeDraftId && draftsState.value.some(draft => draft.id === activeDraftId)
    ? activeDraftId
    : draftsState.value.at(-1)?.id ?? null
}

function resetDraftStore() {
  draftsState.value = []
  revisionsState.value = {}
  activeDraftIdState.value = null
}

export function useDraftStore() {
  const activeDraft = computed(() => findDraft(activeDraftIdState.value || ''))
  const activeDraftRevisions = computed(() =>
    activeDraft.value ? revisionsState.value[activeDraft.value.id] || [] : [],
  )

  return {
    drafts: draftsState,
    revisions: revisionsState,
    activeDraftId: activeDraftIdState,
    activeDraft,
    activeDraftRevisions,
    createDraftFromExchangeRequest,
    selectDraft,
    appendRevision,
    updateDraftRequest,
    updateDraftEndpoint,
    pinDraft,
    replaceState,
    removeDraft,
    resetDraftStore,
  }
}
