import { expect } from 'vitest'
import type { VueWrapper } from '@vue/test-utils'

export function getHttpSurfaces(wrapper: VueWrapper<any>) {
  return wrapper.findAll('[data-testid="http-surface"]')
}

export function expectHttpSurfaceModes(wrapper: VueWrapper<any>, modes: string[]) {
  const surfaces = getHttpSurfaces(wrapper)
  expect(surfaces).toHaveLength(modes.length)
  modes.forEach((mode, index) => {
    expect(surfaces[index]?.attributes('data-display-mode')).toBe(mode)
  })
}

export function expectNoHttpSurfaces(wrapper: VueWrapper<any>) {
  expect(getHttpSurfaces(wrapper)).toHaveLength(0)
}

export function expectReadonlyHttpSurface(
  wrapper: VueWrapper<any>,
  options: { mode: string; stateKey?: string; stateKeyIncludes?: string; index?: number },
) {
  const surface = options.stateKeyIncludes
    ? getHttpSurfaces(wrapper).find(node => node.attributes('data-state-key')?.includes(options.stateKeyIncludes))
    : getHttpSurfaces(wrapper)[options.index ?? 0]
  expect(surface).toBeTruthy()
  expect(surface?.attributes('data-readonly')).toBe('true')
  expect(surface?.attributes('data-display-mode')).toBe(options.mode)
  if (options.stateKey) {
    expect(surface?.attributes('data-state-key')).toBe(options.stateKey)
  }
}

export function expectTrafficReaderCount(wrapper: VueWrapper<any>, count: number) {
  expect(wrapper.findAll('[data-testid="traffic-reader"]')).toHaveLength(count)
}

export function expectNoTrafficReaders(wrapper: VueWrapper<any>) {
  expectTrafficReaderCount(wrapper, 0)
}

export function expectCodeDiffVisible(wrapper: VueWrapper<any>, visible: boolean) {
  expect(wrapper.find('[data-testid="code-diff-viewer"]').exists()).toBe(visible)
}
