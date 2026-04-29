import { flushPromises, shallowMount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import BuiltinToolsTab from './BuiltinToolsTab.vue'

vi.mock('vue-router', () => ({
  useRoute: () => ({
    query: {},
  }),
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
  }),
}))

vi.mock('@/composables/useDialog', () => ({
  dialog: {
    toast: {
      success: vi.fn(),
      error: vi.fn(),
      info: vi.fn(),
    },
  },
}))

describe('BuiltinToolsTab', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockInvoke.mockImplementation((command: string) => {
      if (command === 'get_builtin_tools_with_status') {
        return Promise.resolve([])
      }
      if (command === 'list_durable_memory_diagnostics') {
        return Promise.resolve([])
      }
      if (command === 'get_durable_memory_diagnostics_by_ids') {
        return Promise.resolve([])
      }
      throw new Error(`Unexpected command: ${command}`)
    })
  })

  it('keeps source filter buttons visible in workflow mode', async () => {
    const wrapper = shallowMount(BuiltinToolsTab, {
      props: {
        sourceFilter: 'workflow',
        showContent: false,
        workflowCount: 3,
        pluginCount: 2,
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('工作流工具 (3)')
    expect(wrapper.text()).toContain('插件工具 (2)')
  })

  it('keeps source filter buttons visible in plugin mode', async () => {
    const wrapper = shallowMount(BuiltinToolsTab, {
      props: {
        sourceFilter: 'plugin',
        showContent: false,
        workflowCount: 3,
        pluginCount: 2,
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('工作流工具 (3)')
    expect(wrapper.text()).toContain('插件工具 (2)')
  })
})
