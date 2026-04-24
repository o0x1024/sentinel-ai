import { nextTick, type Ref } from 'vue'
import type { HttpExchangeRequest } from '../../http/model'
import type {
  TrafficWorkbenchBasketCandidateInput,
  TrafficWorkbenchBasketItem,
  TrafficWorkbenchSource,
  TrafficWorkbenchToolSession,
} from '../../trafficWorkbenchTypes'
import type { TrafficComparerDraftRequestInput, TrafficComparePayload } from '../../transfers'
import type { useTrafficWorkbenchStore } from '../stores/useTrafficWorkbenchStore'
import {
  getBasketSource,
  getHistorySource,
  getInterceptSource,
  getToolSource,
} from '../services/sourceSupport'

type WorkbenchTool = TrafficWorkbenchToolSession['tool']
type TrafficWorkbenchStore = ReturnType<typeof useTrafficWorkbenchStore>

interface WorkbenchMainStageHandle {
  hasRepeater: () => boolean
  hasIntruder: () => boolean
  hasComparer: () => boolean
  openPreviewRequest: (request: HttpExchangeRequest) => void
  addComparison: (payload: TrafficComparePayload) => void
  addDraftRequest: (payload: TrafficComparerDraftRequestInput) => void
}

export function useTrafficWorkbenchActions(options: {
  workbenchState: TrafficWorkbenchStore
  mainStageRef: Ref<WorkbenchMainStageHandle | null>
  basketItems: Ref<TrafficWorkbenchBasketItem[]>
  addBasketRequest: (
    request: HttpExchangeRequest,
    source: TrafficWorkbenchSource,
    options?: { requestId?: number; title?: string },
  ) => boolean
  markSession: (tool: WorkbenchTool, source: TrafficWorkbenchSource | null) => void
  openWorkbenchTool: (tool: WorkbenchTool) => void
  closeInterceptDrawer: () => void
  openBasket: () => void
  pendingRepeaterDraftId: Ref<string | undefined>
  pendingRepeaterRequest: Ref<HttpExchangeRequest | undefined>
  pendingIntruderWorkspaceId: Ref<string | undefined>
  pendingIntruderRequest: Ref<HttpExchangeRequest | undefined>
  workbenchMetaTitleMap: Record<WorkbenchTool, string>
}) {
  const {
    workbenchState,
    mainStageRef,
    basketItems,
    addBasketRequest,
    markSession,
    openWorkbenchTool,
    closeInterceptDrawer,
    openBasket,
    pendingRepeaterDraftId,
    pendingRepeaterRequest,
    pendingIntruderWorkspaceId,
    pendingIntruderRequest,
    workbenchMetaTitleMap,
  } = options

  function focusDraftRecord(draftId: string) {
    const draft = workbenchState.drafts.drafts.value.find(item => item.id === draftId) ?? null
    workbenchState.drafts.selectDraft(draftId)
    workbenchState.selection.selectDraft(draft)
    if (mainStageRef.value?.hasRepeater()) {
      openWorkbenchTool('repeater')
      return
    }

    pendingRepeaterDraftId.value = draftId
    openWorkbenchTool('repeater')
    void nextTick(() => {
      pendingRepeaterDraftId.value = undefined
      pendingRepeaterRequest.value = undefined
    })
  }

  function pushRequestToRepeater(
    request: HttpExchangeRequest,
    source: TrafficWorkbenchSource,
    pushOptions?: { sourceSnapshotId?: string | null; title?: string },
  ) {
    const draft = workbenchState.drafts.createDraftFromExchangeRequest({
      request,
      source,
      sourceSnapshotId: pushOptions?.sourceSnapshotId ?? null,
      title: pushOptions?.title ?? (source.requestId ? `${source.label}` : request.endpoint.host),
    })
    markSession('repeater', source)
    focusDraftRecord(draft.id)
    return draft
  }

  function previewRequestInRepeater(request: HttpExchangeRequest) {
    if (mainStageRef.value?.hasRepeater()) {
      mainStageRef.value.openPreviewRequest(request)
      openWorkbenchTool('repeater')
      return
    }

    pendingRepeaterRequest.value = request
    openWorkbenchTool('repeater')
    void nextTick(() => {
      pendingRepeaterRequest.value = undefined
      pendingRepeaterDraftId.value = undefined
    })
  }

  function pushRequestToIntruder(
    request: HttpExchangeRequest,
    source: TrafficWorkbenchSource,
    pushOptions?: { title?: string },
  ) {
    const workspace = workbenchState.attack.createWorkspaceFromExchangeRequest({
      request,
      source,
      title: pushOptions?.title ?? (source.requestId ? `${source.label}` : request.endpoint.host),
    })
    markSession('intruder', source)
    workbenchState.attack.selectWorkspace(workspace.id)
    workbenchState.selection.selectAttackWorkspace(workspace)
    if (mainStageRef.value?.hasIntruder()) {
      openWorkbenchTool('intruder')
      return workspace
    }

    pendingIntruderWorkspaceId.value = workspace.id
    openWorkbenchTool('intruder')
    void nextTick(() => {
      pendingIntruderWorkspaceId.value = undefined
      pendingIntruderRequest.value = undefined
    })
    return workspace
  }

  function pushPayloadToComparer(payload: TrafficComparePayload, source: TrafficWorkbenchSource) {
    markSession('comparer', source)
    if (mainStageRef.value?.hasComparer()) {
      mainStageRef.value.addComparison(payload)
      openWorkbenchTool('comparer')
      return
    }

    openWorkbenchTool('comparer')
    requestAnimationFrame(() => {
      mainStageRef.value?.addComparison(payload)
    })
  }

  function pushDraftToComparer(payload: TrafficComparerDraftRequestInput, source: TrafficWorkbenchSource) {
    markSession('comparer', source)
    if (mainStageRef.value?.hasComparer()) {
      mainStageRef.value.addDraftRequest(payload)
      openWorkbenchTool('comparer')
      return
    }

    openWorkbenchTool('comparer')
    requestAnimationFrame(() => {
      mainStageRef.value?.addDraftRequest(payload)
    })
  }

  function selectDraftRecord(draftId: string) {
    focusDraftRecord(draftId)
  }

  function selectAttackWorkspaceRecord(workspaceId: string) {
    workbenchState.attack.selectWorkspace(workspaceId)
    workbenchState.selection.selectAttackWorkspace(
      workbenchState.attack.workspaces.value.find(workspace => workspace.id === workspaceId) ?? null,
    )
    openWorkbenchTool('intruder')
  }

  function handleCreateDraftFromHistory(request: HttpExchangeRequest) {
    pushRequestToRepeater(request, getHistorySource(request.sourceRequestId ?? null))
  }

  function handleCreateAttackWorkspaceFromHistory(request: HttpExchangeRequest) {
    pushRequestToIntruder(request, getHistorySource(request.sourceRequestId ?? null))
  }

  function handleOpenCompareFromHistory(payload: TrafficComparePayload) {
    pushPayloadToComparer(payload, getHistorySource())
  }

  function handleOpenDraftCompareFromHistory(payload: TrafficComparerDraftRequestInput) {
    pushDraftToComparer(payload, getHistorySource())
  }

  function handleCreateDraftFromIntercept(request: HttpExchangeRequest) {
    closeInterceptDrawer()
    pushRequestToRepeater(request, getInterceptSource())
  }

  function handleCreateAttackWorkspaceFromIntercept(request: HttpExchangeRequest) {
    closeInterceptDrawer()
    pushRequestToIntruder(request, getInterceptSource())
  }

  function handleOpenDraftCompareFromIntercept(payload: TrafficComparerDraftRequestInput) {
    closeInterceptDrawer()
    pushDraftToComparer(payload, getInterceptSource())
  }

  function handleCreateAttackWorkspaceFromRepeater(request: HttpExchangeRequest) {
    pushRequestToIntruder(request, getToolSource('repeater', workbenchMetaTitleMap.repeater))
  }

  function handleOpenCompareFromRepeater(payload: TrafficComparePayload) {
    pushPayloadToComparer(payload, getToolSource('repeater', workbenchMetaTitleMap.repeater))
  }

  function handleOpenDraftCompareFromRepeater(payload: TrafficComparerDraftRequestInput) {
    pushDraftToComparer(payload, getToolSource('repeater', workbenchMetaTitleMap.repeater))
  }

  function handleCreateDraftFromIntruder(request: HttpExchangeRequest) {
    pushRequestToRepeater(request, getToolSource('intruder', workbenchMetaTitleMap.intruder))
  }

  function handleOpenCompareFromIntruder(payload: TrafficComparePayload) {
    pushPayloadToComparer(payload, getToolSource('intruder', workbenchMetaTitleMap.intruder))
  }

  function handleOpenDraftCompareFromIntruder(payload: TrafficComparerDraftRequestInput) {
    pushDraftToComparer(payload, getToolSource('intruder', workbenchMetaTitleMap.intruder))
  }

  function handleCreateDraftFromComparer(request: HttpExchangeRequest) {
    pushRequestToRepeater(request, getToolSource('comparer', workbenchMetaTitleMap.comparer))
  }

  function addBasketCandidate(source: TrafficWorkbenchSource, payload: TrafficWorkbenchBasketCandidateInput) {
    addBasketRequest(payload.request, source, {
      requestId: payload.requestId,
      title: payload.title,
    })
  }

  function handleAddToBasketFromHistory(payload: TrafficWorkbenchBasketCandidateInput) {
    addBasketCandidate(getHistorySource(payload.requestId ?? null), payload)
    openBasket()
  }

  function handleAddToBasketFromIntercept(payload: TrafficWorkbenchBasketCandidateInput) {
    addBasketCandidate(getInterceptSource(), payload)
    openBasket()
  }

  function findBasketItem(id: string) {
    return basketItems.value.find(item => item.id === id) || null
  }

  function createDraftFromBasketItem(id: string) {
    const item = findBasketItem(id)
    if (!item) {
      return
    }
    pushRequestToRepeater(item.request, getBasketSource(item))
  }

  function createAttackWorkspaceFromBasketItem(id: string) {
    const item = findBasketItem(id)
    if (!item) {
      return
    }
    pushRequestToIntruder(item.request, getBasketSource(item))
  }

  function createDraftsForAllBasketItems() {
    basketItems.value.forEach(item => {
      pushRequestToRepeater(item.request, getBasketSource(item))
    })
  }

  function createAttackWorkspacesForAllBasketItems() {
    basketItems.value.forEach(item => {
      pushRequestToIntruder(item.request, getBasketSource(item))
    })
  }

  return {
    pushRequestToRepeater,
    pushRequestToIntruder,
    pushPayloadToComparer,
    pushDraftToComparer,
    selectDraftRecord,
    selectAttackWorkspaceRecord,
    handleCreateDraftFromHistory,
    handleCreateAttackWorkspaceFromHistory,
    handleOpenCompareFromHistory,
    handleOpenDraftCompareFromHistory,
    handleCreateDraftFromIntercept,
    handleCreateAttackWorkspaceFromIntercept,
    handleOpenDraftCompareFromIntercept,
    handleCreateAttackWorkspaceFromRepeater,
    handleOpenCompareFromRepeater,
    handleOpenDraftCompareFromRepeater,
    handleCreateDraftFromIntruder,
    handleOpenCompareFromIntruder,
    handleOpenDraftCompareFromIntruder,
    handleCreateDraftFromComparer,
    handleAddToBasketFromHistory,
    handleAddToBasketFromIntercept,
    createDraftFromBasketItem,
    createAttackWorkspaceFromBasketItem,
    createDraftsForAllBasketItems,
    createAttackWorkspacesForAllBasketItems,
    previewRequestInRepeater,
  }
}
