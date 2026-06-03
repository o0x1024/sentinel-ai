import { beforeEach, describe, expect, it } from 'vitest'
import {
  clearImmersiveToolMinimized,
  immersiveMinimizedToolCount,
  immersiveMinimizedToolOrder,
  markImmersiveToolMinimized,
  resetImmersiveMinimizedToolTray,
} from './immersiveMinimizedToolTray'

describe('immersiveMinimizedToolTray', () => {
  beforeEach(() => {
    resetImmersiveMinimizedToolTray()
  })

  it('keeps the most recently minimized tool at the front', () => {
    markImmersiveToolMinimized('security-center')
    markImmersiveToolMinimized('traffic-assistant')

    expect(immersiveMinimizedToolOrder.value).toEqual([
      'traffic-assistant',
      'security-center',
    ])
    expect(immersiveMinimizedToolCount.value).toBe(2)
  })

  it('moves an already minimized tool back to the front when minimized again', () => {
    markImmersiveToolMinimized('security-center')
    markImmersiveToolMinimized('traffic-assistant')
    markImmersiveToolMinimized('security-center')

    expect(immersiveMinimizedToolOrder.value).toEqual([
      'security-center',
      'traffic-assistant',
    ])
  })

  it('removes tools from the tray when cleared', () => {
    markImmersiveToolMinimized('security-center')
    markImmersiveToolMinimized('traffic-assistant')

    clearImmersiveToolMinimized('traffic-assistant')

    expect(immersiveMinimizedToolOrder.value).toEqual(['security-center'])
    expect(immersiveMinimizedToolCount.value).toBe(1)
  })
})
