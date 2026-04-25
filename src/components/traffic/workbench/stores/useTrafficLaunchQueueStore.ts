import { ref } from 'vue'
import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficComparePayload } from '../../transfers'

export const TRAFFIC_LAUNCH_QUEUE_EVENT = 'traffic-workbench:launch-queue-updated'

type LaunchQueueState = {
  repeaterRequests: HttpExchangeRequest[]
  intruderRequests: HttpExchangeRequest[]
  comparePayloads: TrafficComparePayload[]
}

const launchQueueState = ref<LaunchQueueState>({
  repeaterRequests: [],
  intruderRequests: [],
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

function queueRepeaterRequest(request: HttpExchangeRequest) {
  launchQueueState.value = {
    ...launchQueueState.value,
    repeaterRequests: [...launchQueueState.value.repeaterRequests, cloneRequest(request)],
  }
}

function queueIntruderRequest(request: HttpExchangeRequest) {
  launchQueueState.value = {
    ...launchQueueState.value,
    intruderRequests: [...launchQueueState.value.intruderRequests, cloneRequest(request)],
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
    repeaterRequests: launchQueueState.value.repeaterRequests.map(cloneRequest),
    intruderRequests: launchQueueState.value.intruderRequests.map(cloneRequest),
    comparePayloads: launchQueueState.value.comparePayloads.map(cloneComparePayload),
  }
  launchQueueState.value = {
    repeaterRequests: [],
    intruderRequests: [],
    comparePayloads: [],
  }
  return snapshot
}

export function useTrafficLaunchQueueStore() {
  return {
    state: launchQueueState,
    queueRepeaterRequest,
    queueIntruderRequest,
    queueComparePayload,
    consumeLaunchQueue,
  }
}
