import { describe, expect, it } from 'vitest'
import {
  buildHelpCenterWindowUrl,
  buildIntruderResultsWindowUrl,
  resolveStandaloneBootstrapRoute,
} from './standalone'

describe('standalone route helpers', () => {
  it('builds intruder results window urls', () => {
    expect(buildIntruderResultsWindowUrl('workspace-1')).toBe(
      '/?standalone=intruder-results&workspaceId=workspace-1'
    )
  })

  it('builds help center window urls', () => {
    expect(buildHelpCenterWindowUrl()).toBe('/?standalone=help-center')
  })

  it('resolves intruder results bootstrap routes', () => {
    expect(
      resolveStandaloneBootstrapRoute({
        search: '?standalone=intruder-results&workspaceId=workspace-1',
        hash: '',
      })
    ).toBe('/intruder-results/workspace-1')
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
