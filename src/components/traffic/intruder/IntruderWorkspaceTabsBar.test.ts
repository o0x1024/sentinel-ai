import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import IntruderWorkspaceTabsBar from './IntruderWorkspaceTabsBar.vue'

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('../trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('@/services/immersiveDrillMode', () => ({
  immersiveDrillModeEnabled: { value: false },
}))

describe('IntruderWorkspaceTabsBar', () => {
  it('emits clearAllWorkspaces when the clear-all button is clicked', async () => {
    const wrapper = mount(IntruderWorkspaceTabsBar, {
      props: {
        workspaces: [
          { id: 'workspace-1', name: 'example.com' },
          { id: 'workspace-2', name: 'api.example.com' },
        ],
        activeWorkspaceId: 'workspace-1',
      },
    })

    await wrapper.get('[data-testid="intruder-clear-all-workspaces"]').trigger('click')

    expect(wrapper.emitted('clearAllWorkspaces')).toHaveLength(1)
  })

  it('emits tab context menu actions for the targeted workspace', async () => {
    const wrapper = mount(IntruderWorkspaceTabsBar, {
      props: {
        workspaces: [
          { id: 'workspace-1', name: 'example.com' },
          { id: 'workspace-2', name: 'api.example.com' },
        ],
        activeWorkspaceId: 'workspace-1',
      },
      attachTo: document.body,
    })

    await wrapper.get('[data-testid="intruder-tab-1"]').trigger('contextmenu', { clientX: 40, clientY: 50 })
    await wrapper.get('[data-testid="traffic-context-deleteCurrent"]').trigger('click')
    expect(wrapper.emitted('closeWorkspace')?.[0]).toEqual(['workspace-2'])

    await wrapper.get('[data-testid="intruder-tab-0"]').trigger('contextmenu', { clientX: 40, clientY: 50 })
    await wrapper.get('[data-testid="traffic-context-deleteOthers"]').trigger('click')
    expect(wrapper.emitted('closeOtherWorkspaces')?.[0]).toEqual(['workspace-1'])

    await wrapper.get('[data-testid="intruder-tab-0"]').trigger('contextmenu', { clientX: 40, clientY: 50 })
    await wrapper.get('[data-testid="traffic-context-deleteAll"]').trigger('click')
    expect(wrapper.emitted('clearAllWorkspaces')).toHaveLength(1)
  })

  it('disables the clear-all button when there are no workspaces', () => {
    const wrapper = mount(IntruderWorkspaceTabsBar, {
      props: {
        workspaces: [],
        activeWorkspaceId: null,
      },
    })

    expect(wrapper.get('[data-testid="intruder-clear-all-workspaces"]').attributes('disabled')).toBeDefined()
  })
})
