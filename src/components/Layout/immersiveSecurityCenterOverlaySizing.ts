export const IMMERSIVE_SECURITY_CENTER_WIDTH_STORAGE_KEY = 'sentinel:immersive-security-center-width:v1'
export const IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH = 1040
export const IMMERSIVE_SECURITY_CENTER_MIN_WIDTH = 820
export const IMMERSIVE_SECURITY_CENTER_DESKTOP_BREAKPOINT = 1280
export const IMMERSIVE_SECURITY_CENTER_STAGE_GAP = 32

export function readImmersiveSecurityCenterWidth() {
  if (typeof window === 'undefined') {
    return IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH
  }

  const stored = Number(window.localStorage.getItem(IMMERSIVE_SECURITY_CENTER_WIDTH_STORAGE_KEY))
  if (!Number.isFinite(stored) || stored <= 0) {
    return IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH
  }

  return stored
}

export function persistImmersiveSecurityCenterWidth(width: number) {
  if (typeof window === 'undefined' || !Number.isFinite(width) || width <= 0) {
    return
  }

  window.localStorage.setItem(IMMERSIVE_SECURITY_CENTER_WIDTH_STORAGE_KEY, String(width))
}

export function clampImmersiveSecurityCenterWidth(width: number, stageWidth: number) {
  const normalizedStageWidth = Number.isFinite(stageWidth) && stageWidth > 0
    ? stageWidth
    : IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH + IMMERSIVE_SECURITY_CENTER_STAGE_GAP
  const maxWidth = Math.max(
    IMMERSIVE_SECURITY_CENTER_MIN_WIDTH,
    normalizedStageWidth - IMMERSIVE_SECURITY_CENTER_STAGE_GAP,
  )

  return Math.min(
    Math.max(IMMERSIVE_SECURITY_CENTER_MIN_WIDTH, width),
    maxWidth,
  )
}

export function shouldUseImmersiveSecurityCenterDesktopPanel(stageWidth: number) {
  return stageWidth > IMMERSIVE_SECURITY_CENTER_DESKTOP_BREAKPOINT
}
