import { describe, expect, it } from 'vitest'
import {
  buildTrafficWorkbenchGridRows,
  buildTrafficWorkbenchLeftColumnColumns,
  buildTrafficWorkbenchLeftColumnTemplate,
  buildTrafficWorkbenchGridTemplate,
  clampTrafficWorkbenchHistoryPanelWidth,
  clampTrafficWorkbenchSidebarHeight,
  getTrafficWorkbenchGridMode,
} from './useTrafficWorkbenchLayout'

describe('useTrafficWorkbenchLayout helpers', () => {
  it('switches grid mode at the expected breakpoints', () => {
    expect(getTrafficWorkbenchGridMode(1024)).toBe('stacked')
    expect(getTrafficWorkbenchGridMode(1280)).toBe('split')
    expect(getTrafficWorkbenchGridMode(1280, 'top-bottom')).toBe('top-bottom')
  })

  it('builds split grid templates', () => {
    expect(
      buildTrafficWorkbenchGridTemplate({
        mode: 'split',
        historyWidth: 640,
      }),
    ).toBe('640px 4px minmax(0, 1fr)')

    expect(
      buildTrafficWorkbenchGridTemplate({
        mode: 'stacked',
        historyWidth: 640,
      }),
    ).toBeUndefined()
  })

  it('builds top-bottom grid templates', () => {
    expect(
      buildTrafficWorkbenchGridRows({
        mode: 'top-bottom',
        topPanelHeight: 420,
      }),
    ).toBe('420px 4px minmax(0, 1fr)')

    expect(
      buildTrafficWorkbenchLeftColumnColumns({
        mode: 'top-bottom',
        sidebarWidth: 420,
      }),
    ).toBe('minmax(0, 1fr) 4px 420px')
  })

  it('builds split left column templates', () => {
    expect(
      buildTrafficWorkbenchLeftColumnTemplate({
        mode: 'split',
        sidebarHeight: 336,
      }),
    ).toBe('minmax(0, 1fr) 4px 336px')

    expect(
      buildTrafficWorkbenchLeftColumnTemplate({
        mode: 'stacked',
        sidebarHeight: 336,
      }),
    ).toBeUndefined()
  })

  it('clamps history width only to the available stage boundary', () => {
    expect(
      clampTrafficWorkbenchHistoryPanelWidth({
        width: 1200,
        stageWidth: 1600,
        mode: 'split',
      }),
    ).toBe(1200)

    expect(
      clampTrafficWorkbenchHistoryPanelWidth({
        width: -120,
        stageWidth: 1600,
        mode: 'split',
      }),
    ).toBe(0)
  })

  it('clamps sidebar height only to the available column boundary', () => {
    expect(
      clampTrafficWorkbenchSidebarHeight({
        height: 1000,
        columnHeight: 900,
      }),
    ).toBe(896)

    expect(
      clampTrafficWorkbenchSidebarHeight({
        height: -100,
        columnHeight: 900,
      }),
    ).toBe(0)
  })
})
