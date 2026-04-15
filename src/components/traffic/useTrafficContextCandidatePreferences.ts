import { computed, ref } from 'vue'

import type {
  TrafficContextPreviewFocus,
  TrafficContextPreviewKind,
} from './trafficContextCandidateTypes'

const TRAFFIC_CONTEXT_CANDIDATE_PREFERENCES_KEY = 'trafficContextCandidatePreferences.previewFocus'
const VALID_PREVIEW_KINDS: TrafficContextPreviewKind[] = [
  'principal',
  'resource',
  'authHeader',
  'authToken',
  'cookie',
  'action',
]

const preferredPreviewFocusState = ref<TrafficContextPreviewFocus>(loadStoredPreferredPreviewFocus())

const preferredPreviewFocus = computed<TrafficContextPreviewFocus>({
  get: () => preferredPreviewFocusState.value,
  set: (value) => {
    const normalized = normalizePreferredPreviewFocus(value)
    preferredPreviewFocusState.value = normalized
    persistPreferredPreviewFocus(normalized)
  },
})

export function useTrafficContextCandidatePreferences() {
  return {
    preferredPreviewFocus,
  }
}

function loadStoredPreferredPreviewFocus(): TrafficContextPreviewFocus {
  if (typeof window === 'undefined') {
    return buildEmptyPreviewFocus()
  }

  const raw = window.localStorage.getItem(TRAFFIC_CONTEXT_CANDIDATE_PREFERENCES_KEY)
  if (!raw) {
    return buildEmptyPreviewFocus()
  }

  try {
    return normalizePreferredPreviewFocus(JSON.parse(raw))
  } catch {
    return buildEmptyPreviewFocus()
  }
}

function persistPreferredPreviewFocus(value: TrafficContextPreviewFocus) {
  if (typeof window === 'undefined') {
    return
  }

  if (!value.kind && !value.requestId) {
    window.localStorage.removeItem(TRAFFIC_CONTEXT_CANDIDATE_PREFERENCES_KEY)
    return
  }

  window.localStorage.setItem(TRAFFIC_CONTEXT_CANDIDATE_PREFERENCES_KEY, JSON.stringify(value))
}

function normalizePreferredPreviewKind(value: string | null | undefined): TrafficContextPreviewKind | null {
  if (!value) {
    return null
  }

  return VALID_PREVIEW_KINDS.includes(value as TrafficContextPreviewKind)
    ? value as TrafficContextPreviewKind
    : null
}

function normalizePreferredPreviewFocus(value: unknown): TrafficContextPreviewFocus {
  const input = (value && typeof value === 'object') ? value as Partial<TrafficContextPreviewFocus> : {}
  const requestId = Number.isFinite(input.requestId) ? Number(input.requestId) : null
  const updatedAt = Number.isFinite(input.updatedAt) ? Number(input.updatedAt) : null
  const previewVersion = typeof input.previewVersion === 'string' && input.previewVersion.trim().length > 0
    ? input.previewVersion
    : null
  return {
    requestId,
    kind: normalizePreferredPreviewKind(input.kind),
    updatedAt,
    previewVersion,
  }
}

function buildEmptyPreviewFocus(): TrafficContextPreviewFocus {
  return {
    requestId: null,
    kind: null,
    updatedAt: null,
    previewVersion: null,
  }
}
