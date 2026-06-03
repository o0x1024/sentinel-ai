import { describe, expect, it } from 'vitest'
import {
  buildImmersiveFloatingPositionAnnouncement,
  describeImmersiveFloatingDockSide,
} from './immersiveFloatingA11y'

describe('immersiveFloatingA11y', () => {
  it('describes floating and docked states', () => {
    expect(describeImmersiveFloatingDockSide(null)).toBe('当前处于浮动位置')
    expect(describeImmersiveFloatingDockSide('left')).toBe('当前停靠在左侧边缘')
    expect(describeImmersiveFloatingDockSide('right')).toBe('当前停靠在右侧边缘')
    expect(describeImmersiveFloatingDockSide('top')).toBe('当前停靠在顶部边缘')
    expect(describeImmersiveFloatingDockSide('bottom')).toBe('当前停靠在底部边缘')
  })

  it('builds floating move announcements with rounded coordinates', () => {
    expect(
      buildImmersiveFloatingPositionAnnouncement('工具托盘', null, {
        x: 128.7,
        y: 45.2,
      }),
    ).toBe('工具托盘已移动到距左 129 像素，距上 45 像素。')
  })

  it('builds docked move announcements for every edge', () => {
    expect(
      buildImmersiveFloatingPositionAnnouncement('工具托盘', 'left', {
        x: 16,
        y: 200,
      }),
    ).toBe('工具托盘已停靠到左侧边缘。')
    expect(
      buildImmersiveFloatingPositionAnnouncement('工具托盘', 'right', {
        x: 16,
        y: 200,
      }),
    ).toBe('工具托盘已停靠到右侧边缘。')
    expect(
      buildImmersiveFloatingPositionAnnouncement('工具托盘', 'top', {
        x: 16,
        y: 200,
      }),
    ).toBe('工具托盘已停靠到顶部边缘。')
    expect(
      buildImmersiveFloatingPositionAnnouncement('工具托盘', 'bottom', {
        x: 16,
        y: 200,
      }),
    ).toBe('工具托盘已停靠到底部边缘。')
  })
})
