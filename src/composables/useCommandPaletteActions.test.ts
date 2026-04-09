import { describe, expect, it, vi } from 'vitest'
import { useCommandPaletteActions } from '@/composables/useCommandPaletteActions'

describe('useCommandPaletteActions', () => {
  it('provides executable palette actions', async () => {
    const push = vi.fn().mockResolvedValue(undefined)
    const router = { push } as any
    const { actions } = useCommandPaletteActions(router)

    const criticalAction = actions.value.find(item => item.id === 'action:critical-findings')
    expect(criticalAction?.category).toBe('action')
    expect(typeof criticalAction?.execute).toBe('function')
    expect(criticalAction?.aliases).toContain('finding critical')

    await criticalAction?.execute?.()
    expect(push).toHaveBeenCalledWith({
      path: '/security-center',
      query: {
        tab: 'vulnerabilities',
        severity: 'critical',
      },
    })
  })
})
