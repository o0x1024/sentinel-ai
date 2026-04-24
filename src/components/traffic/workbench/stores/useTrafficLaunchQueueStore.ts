import { ref } from 'vue'
import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficComparePayload } from '../../transfers'

type LaunchQueueState = {
  draftRequests: HttpExchangeRequest[]
  attackWorkspaceRequests: HttpExchangeRequest[]
  comparePayloads: TrafficComparePayload[]
}

const launchQueueState = ref<LaunchQueueState>({
  draftRequests: [],
  attackWorkspaceRequests: [],
  comparePayloads: [],
})

function cloneRequest(request: HttpExchangeRequest): HttpExchangeRequest {
  return {
    endpoint: {
      ...request.endpoint,
    },
    absoluteUrl: request.absoluteUrl,
    sourceRequestId: request.sourceRequestId ?? null,
    preferredRequestView: request.preferredRequestView,
    request: {
      method: request.request.method,
      target: request.request.target,
      versionPreference: request.request.versionPreference,
      headers: request.request.headers.map(header => ({ ...header })),
      bodyText: request.request.bodyText,
    },
  }
}

function cloneComparePayload(payload: TrafficComparePayload): TrafficComparePayload {
  return {
    ...payload,
    leftMeta: payload.leftMeta
      ? {
          ...payload.leftMeta,
          repeaterRequest: payload.leftMeta.repeaterRequest
            ? cloneRequest(payload.leftMeta.repeaterRequest)
            : undefined,
        }
      : undefined,
    rightMeta: payload.rightMeta
      ? {
          ...payload.rightMeta,
          repeaterRequest: payload.rightMeta.repeaterRequest
            ? cloneRequest(payload.rightMeta.repeaterRequest)
            : undefined,
        }
      : undefined,
    compareMeta: payload.compareMeta
      ? { ...payload.compareMeta }
      : undefined,
  }
}

function queueDraftRequest(request: HttpExchangeRequest) {
  launchQueueState.value = {
    ...launchQueueState.value,
    draftRequests: [...launchQueueState.value.draftRequests, cloneRequest(request)],
  }
}

function queueAttackWorkspaceRequest(request: HttpExchangeRequest) {
  launchQueueState.value = {
    ...launchQueueState.value,
    attackWorkspaceRequests: [...launchQueueState.value.attackWorkspaceRequests, cloneRequest(request)],
  }
}

function queueComparePayload(payload: TrafficComparePayload) {
  launchQueueState.value = {
    ...launchQueueState.value,
    comparePayloads: [...launchQueueState.value.comparePayloads, cloneComparePayload(payload)],
  }
}

function consumeLaunchQueue(): LaunchQueueState {
  const snapshot = {
    draftRequests: launchQueueState.value.draftRequests.map(cloneRequest),
    attackWorkspaceRequests: launchQueueState.value.attackWorkspaceRequests.map(cloneRequest),
    comparePayloads: launchQueueState.value.comparePayloads.map(cloneComparePayload),
  }
  launchQueueState.value = {
    draftRequests: [],
    attackWorkspaceRequests: [],
    comparePayloads: [],
  }
  return snapshot
}

export function useTrafficLaunchQueueStore() {
  return {
    state: launchQueueState,
    queueDraftRequest,
    queueAttackWorkspaceRequest,
    queueComparePayload,
    consumeLaunchQueue,
  }
}
