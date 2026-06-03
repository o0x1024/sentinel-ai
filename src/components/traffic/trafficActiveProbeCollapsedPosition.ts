export interface ActiveProbeCollapsedPosition {
  left: number
  top: number
}

export const ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY = 'trafficAnalysis.activeProbe.collapsedPosition.v1'

const ACTIVE_PROBE_COLLAPSED_POSITION_MARGIN = 16

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function normalizeActiveProbeCollapsedPosition(
  value: unknown,
): ActiveProbeCollapsedPosition | null {
  if (!value || typeof value !== 'object') {
    return null
  }

  const candidate = value as Record<string, unknown>
  if (!isFiniteNumber(candidate.left) || !isFiniteNumber(candidate.top)) {
    return null
  }

  return {
    left: candidate.left,
    top: candidate.top,
  }
}

export function loadStoredActiveProbeCollapsedPosition(
  storageKey = ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY,
): ActiveProbeCollapsedPosition | null {
  if (typeof window === 'undefined') {
    return null
  }

  const raw = window.localStorage.getItem(storageKey)
  if (!raw) {
    return null
  }

  try {
    return normalizeActiveProbeCollapsedPosition(JSON.parse(raw))
  } catch {
    return null
  }
}

export function clampActiveProbeCollapsedPosition(options: {
  position: ActiveProbeCollapsedPosition
  viewportWidth: number
  viewportHeight: number
  width: number
  height: number
  margin?: number
}): ActiveProbeCollapsedPosition {
  const margin = options.margin ?? ACTIVE_PROBE_COLLAPSED_POSITION_MARGIN
  const maxLeft = Math.max(margin, options.viewportWidth - options.width - margin)
  const maxTop = Math.max(margin, options.viewportHeight - options.height - margin)

  return {
    left: Math.min(Math.max(options.position.left, margin), maxLeft),
    top: Math.min(Math.max(options.position.top, margin), maxTop),
  }
}
