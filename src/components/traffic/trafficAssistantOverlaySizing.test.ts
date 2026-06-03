import { describe, expect, it } from 'vitest'
import {
  TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH,
  TRAFFIC_ASSISTANT_PANEL_MIN_WIDTH,
  clampTrafficAssistantPanelWidth,
  shouldUseTrafficAssistantDesktopPanel,
} from './trafficAssistantOverlaySizing'

describe('trafficAssistantOverlaySizing', () => {
  it('clamps width to minimum bound', () => {
    expect(clampTrafficAssistantPanelWidth(120, 1600)).toBe(TRAFFIC_ASSISTANT_PANEL_MIN_WIDTH)
  })

  it('clamps width to available stage width', () => {
    expect(clampTrafficAssistantPanelWidth(1200, 900)).toBe(868)
  })

  it('falls back to a sane stage width when input is invalid', () => {
    expect(clampTrafficAssistantPanelWidth(TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH, Number.NaN)).toBe(
      TRAFFIC_ASSISTANT_PANEL_DEFAULT_WIDTH,
    )
  })

  it('uses desktop panel mode only above the breakpoint', () => {
    expect(shouldUseTrafficAssistantDesktopPanel(1024)).toBe(false)
    expect(shouldUseTrafficAssistantDesktopPanel(1280)).toBe(true)
  })
})
