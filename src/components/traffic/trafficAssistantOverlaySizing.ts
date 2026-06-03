export const TRAFFIC_ASSISTANT_PANEL_WIDTH_STORAGE_KEY = 'sentinel:traffic-assistant-panel-width:v1'
export const TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH = 576
export const TRAFFIC_ASSISTANT_PANEL_MIN_WIDTH = 420
export const TRAFFIC_ASSISTANT_PANEL_DESKTOP_BREAKPOINT = 1024
export const TRAFFIC_ASSISTANT_PANEL_STAGE_GAP = 32

export function readTrafficAssistantPanelWidth() {
  if (typeof window === 'undefined') {
    return TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH
  }

  const stored = Number(window.localStorage.getItem(TRAFFIC_ASSISTANT_PANEL_WIDTH_STORAGE_KEY))
  if (!Number.isFinite(stored) || stored <= 0) {
    return TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH
  }

  return stored
}

export function persistTrafficAssistantPanelWidth(width: number) {
  if (typeof window === 'undefined' || !Number.isFinite(width) || width <= 0) {
    return
  }

  window.localStorage.setItem(TRAFFIC_ASSISTANT_PANEL_WIDTH_STORAGE_KEY, String(width))
}

export function clampTrafficAssistantPanelWidth(width: number, stageWidth: number) {
  const normalizedStageWidth = Number.isFinite(stageWidth) && stageWidth > 0
    ? stageWidth
    : TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH + TRAFFIC_ASSISTANT_PANEL_STAGE_GAP
  const maxWidth = Math.max(
    TRAFFIC_ASSISTANT_PANEL_MIN_WIDTH,
    normalizedStageWidth - TRAFFIC_ASSISTANT_PANEL_STAGE_GAP,
  )

  return Math.min(
    Math.max(TRAFFIC_ASSISTANT_PANEL_MIN_WIDTH, width),
    maxWidth,
  )
}

export function shouldUseTrafficAssistantDesktopPanel(stageWidth: number) {
  return stageWidth > TRAFFIC_ASSISTANT_PANEL_DESKTOP_BREAKPOINT
}
