import { describe, expect, it } from 'vitest'
import {
  buildHelpCenterWindowUrl,
  resolveStandaloneBootstrapRoute,
} from './standalone'

describe('standalone route helpers', () => {
  it('builds help center window urls', () => {
    expect(buildHelpCenterWindowUrl()).toBe('/?standalone=help-center')
  })

  it('resolves help center bootstrap routes', () => {
    expect(
      resolveStandaloneBootstrapRoute({
        search: '?standalone=help-center',
        hash: '',
      })
    ).toBe('/help-center')
  })

  it('resolves help center hash bootstrap routes', () => {
    expect(
      resolveStandaloneBootstrapRoute({
        search: '',
        hash: '#/help-center',
      })
    ).toBe('/help-center')
  })
})
