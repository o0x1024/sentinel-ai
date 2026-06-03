import type {
  TrafficContextExtractionPreviewResponse,
  TrafficContextExtractionPreviewSample,
  TrafficContextPreviewFocus,
} from './trafficContextCandidateTypes'

export const TRAFFIC_CONTEXT_PREVIEW_FOCUS_MAX_AGE_MS = 2 * 60 * 60 * 1000

export function buildTrafficContextPreviewVersion(
  previewResult: TrafficContextExtractionPreviewResponse | null | undefined,
): string | null {
  if (!previewResult) {
    return null
  }

  return [
    previewResult.analyzedRequestCount,
    previewResult.changedRequestCount,
    previewResult.principalMatchDeltaCount,
    previewResult.resourceMatchDeltaCount,
    previewResult.authHeaderMatchDeltaCount,
    previewResult.authTokenMatchDeltaCount,
    previewResult.cookieMatchDeltaCount,
    previewResult.actionKindChangeCount,
    previewResult.samples.map(serializePreviewSampleVersion).join('|'),
  ].join('::')
}

export function isTrafficContextPreviewFocusFresh(
  focus: TrafficContextPreviewFocus | null | undefined,
  now = Date.now(),
): boolean {
  if (!focus?.updatedAt || !Number.isFinite(focus.updatedAt)) {
    return false
  }

  return now - focus.updatedAt <= TRAFFIC_CONTEXT_PREVIEW_FOCUS_MAX_AGE_MS
}

function serializePreviewSampleVersion(sample: TrafficContextExtractionPreviewSample): string {
  return [
    sample.requestId,
    sample.addedPrincipalKeys.join(','),
    sample.addedResourceKeys.join(','),
    sample.addedAuthHeaders.join(','),
    sample.addedAuthTokens.join(','),
    sample.addedCookieKeys.join(','),
    sample.actionKindBefore || '',
    sample.actionKindAfter || '',
  ].join(':')
}
