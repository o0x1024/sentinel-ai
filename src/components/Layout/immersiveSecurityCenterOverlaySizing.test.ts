import { describe, expect, it } from 'vitest'
import {
  IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH,
  IMMERSIVE_SECURITY_CENTER_MIN_WIDTH,
  clampImmersiveSecurityCenterWidth,
  shouldUseImmersiveSecurityCenterDesktopPanel,
} from './immersiveSecurityCenterOverlaySizing'

describe('immersiveSecurityCenterOverlaySizing', () => {
  it('clamps width to minimum bound', () => {
    expect(clampImmersiveSecurityCenterWidth(320, 1800)).toBe(IMMERSIVE_SECURITY_CENTER_MIN_WIDTH)
  })

  it('clamps width to available stage width', () => {
    expect(clampImmersiveSecurityCenterWidth(1800, 1200)).toBe(1168)
  })

  it('falls back to default width when stage width is invalid', () => {
    expect(clampImmersiveSecurityCenterWidth(IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH, Number.NaN)).toBe(
      IMMERSIVE_SECURITY_CENTER_DEFAULT_WIDTH,
    )
  })

  it('uses floating desktop mode only above the breakpoint', () => {
    expect(shouldUseImmersiveSecurityCenterDesktopPanel(1280)).toBe(false)
    expect(shouldUseImmersiveSecurityCenterDesktopPanel(1440)).toBe(true)
  })
})
