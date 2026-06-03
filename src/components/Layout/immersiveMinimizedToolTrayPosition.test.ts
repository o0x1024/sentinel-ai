import { describe, expect, it } from 'vitest'
import {
  applyImmersiveMinimizedToolTrayMagneticAttraction,
  buildDefaultImmersiveMinimizedToolTrayPosition,
  clampImmersiveMinimizedToolTrayPosition,
  doImmersiveTrayRectsOverlap,
  resolveImmersiveMinimizedToolTrayDockSide,
  resolveImmersiveMinimizedToolTraySnapEdges,
  resolveImmersiveMinimizedToolTrayPosition,
  snapImmersiveMinimizedToolTrayPosition,
} from './immersiveMinimizedToolTrayPosition'

describe('immersiveMinimizedToolTrayPosition', () => {
  it('builds a bottom-right default position inside the viewport', () => {
    expect(
      buildDefaultImmersiveMinimizedToolTrayPosition(
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toEqual({ x: 1164, y: 828 })
  })

  it('clamps tray position to the viewport edges', () => {
    expect(
      clampImmersiveMinimizedToolTrayPosition(
        { x: -20, y: 1000 },
        { width: 1280, height: 720 },
        { width: 240, height: 64 },
      ),
    ).toEqual({ x: 16, y: 640 })
  })

  it('detects overlapping rectangles', () => {
    expect(
      doImmersiveTrayRectsOverlap(
        { x: 100, y: 100, width: 240, height: 64 },
        { x: 200, y: 120, width: 60, height: 180 },
      ),
    ).toBe(true)
    expect(
      doImmersiveTrayRectsOverlap(
        { x: 100, y: 100, width: 240, height: 64 },
        { x: 360, y: 120, width: 60, height: 180 },
      ),
    ).toBe(false)
  })

  it('moves the tray away from the dock when the preferred position overlaps', () => {
    expect(
      resolveImmersiveMinimizedToolTrayPosition(
        { x: 1110, y: 740 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
        { x: 1180, y: 620, width: 52, height: 240 },
      ),
    ).toEqual({ x: 904, y: 740 })
  })

  it('detects which viewport edges should snap', () => {
    expect(
      resolveImmersiveMinimizedToolTraySnapEdges(
        { x: 18, y: 824 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toEqual({
      left: true,
      right: false,
      top: false,
      bottom: true,
    })
  })

  it('snaps a tray position to the nearest eligible edges', () => {
    expect(
      snapImmersiveMinimizedToolTrayPosition(
        { x: 18, y: 824 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toEqual({ x: 16, y: 828 })
  })

  it('prefers horizontal docking sides when snapped to a corner', () => {
    expect(
      resolveImmersiveMinimizedToolTrayDockSide(
        { x: 18, y: 824 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toBe('left')
    expect(
      resolveImmersiveMinimizedToolTrayDockSide(
        { x: 1162, y: 824 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toBe('right')
  })

  it('applies eased magnetic attraction when close to viewport edges', () => {
    expect(
      applyImmersiveMinimizedToolTrayMagneticAttraction(
        { x: 32, y: 810 },
        { width: 1440, height: 900 },
        { width: 260, height: 56 },
      ),
    ).toEqual({
      x: 28.251239669421487,
      y: 813.9863119834711,
    })
  })
})
