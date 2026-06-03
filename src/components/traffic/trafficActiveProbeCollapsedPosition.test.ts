import { describe, expect, it, vi } from 'vitest'
import {
  ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY,
  clampActiveProbeCollapsedPosition,
  loadStoredActiveProbeCollapsedPosition,
} from './trafficActiveProbeCollapsedPosition'

describe('trafficActiveProbeCollapsedPosition', () => {
  it('clamps the collapsed button inside the viewport', () => {
    expect(clampActiveProbeCollapsedPosition({
      position: { left: -40, top: 900 },
      viewportWidth: 1280,
      viewportHeight: 800,
      width: 180,
      height: 52,
    })).toEqual({
      left: 16,
      top: 732,
    })
  })

  it('loads a valid stored position', () => {
    const getItem = vi.spyOn(Storage.prototype, 'getItem').mockReturnValue(JSON.stringify({
      left: 320,
      top: 128,
    }))

    expect(loadStoredActiveProbeCollapsedPosition()).toEqual({
      left: 320,
      top: 128,
    })

    getItem.mockRestore()
  })

  it('returns null when the stored position is invalid', () => {
    const getItem = vi.spyOn(Storage.prototype, 'getItem')
    getItem.mockImplementation((key: string) => {
      if (key === ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY) {
        return '{"left":"bad"}'
      }
      return null
    })

    expect(loadStoredActiveProbeCollapsedPosition()).toBeNull()

    getItem.mockRestore()
  })
})
