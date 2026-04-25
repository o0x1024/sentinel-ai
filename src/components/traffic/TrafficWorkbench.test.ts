import { flushPromises, shallowMount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import TrafficWorkbench from './TrafficWorkbench.vue'
import {
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  toggleImmersiveTrafficProxySettings,
  toggleImmersiveTrafficPluginsPanel,
  toggleImmersiveTrafficBasket,
  toggleImmersiveTrafficInterceptDrawer,
  useImmersiveTrafficDockState,
} from './immersiveTrafficDockState'
import { setImmersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { useTrafficWorkbenchStore } from './workbench/stores/useTrafficWorkbenchStore'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('vue-i18n', async importOriginal => {
  const actual = await importOriginal<typeof import('vue-i18n')>()

  return {
    ...actual,
    useI18n: () => ({
      t: (key: string, fallback?: string) => fallback ?? key,
    }),
  }
})

describe('TrafficWorkbench', () => {
  beforeEach(() => {
    window.localStorage.clear()
    resetImmersiveTrafficDockState()
    setImmersiveDrillModeEnabled(false)
    const workbenchState = useTrafficWorkbenchStore()
    workbenchState.drafts.resetDraftStore()
    workbenchState.attack.resetAttackWorkspaceStore()
    workbenchState.replay.resetReplayStore()
    workbenchState.selection.clearSelection()
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'load_traffic_draft_store') {
        return {
          success: true,
          data: {
            activeDraftId: null,
            drafts: [],
          },
        } as never
      }
      if (command === 'load_attack_workspace_store') {
        return {
          success: true,
          data: {
            activeWorkspaceId: null,
            workspaces: [],
          },
        } as never
      }
      if (command === 'load_replay_run_store') {
        return {
          success: true,
          data: {
            replayRuns: [],
          },
        } as never
      }
      return {
        success: true,
        data: null,
      } as never
    })
  })

  afterEach(() => {
    resetImmersiveTrafficDockState()
    setImmersiveDrillModeEnabled(false)
  })

  it('defaults back to history when mounted in immersive drill mode', async () => {
    setImmersiveDrillModeEnabled(true)
    openImmersiveTrafficWorkbenchTool('intruder')
    toggleImmersiveTrafficProxySettings()
    toggleImmersiveTrafficPluginsPanel()

    const dockState = useImmersiveTrafficDockState()
    expect(dockState.proxySettingsOpen.value).toBe(true)
    expect(dockState.trafficPluginsOpen.value).toBe(true)

    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await nextTick()

    expect(dockState.workbenchOpen.value).toBe(false)
    expect(dockState.interceptDrawerOpen.value).toBe(false)
    expect(dockState.proxySettingsOpen.value).toBe(false)
    expect(dockState.trafficPluginsOpen.value).toBe(false)
    expect(dockState.basketOpen.value).toBe(false)

    wrapper.unmount()
  })

  it('hides immersive toolbar windows in layer order with Escape', async () => {
    setImmersiveDrillModeEnabled(true)
    const dockState = useImmersiveTrafficDockState()
    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await nextTick()

    openImmersiveTrafficWorkbenchTool('repeater')
    toggleImmersiveTrafficInterceptDrawer()
    toggleImmersiveTrafficBasket()
    toggleImmersiveTrafficProxySettings()
    toggleImmersiveTrafficPluginsPanel()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.trafficPluginsOpen.value).toBe(false)
    expect(dockState.proxySettingsOpen.value).toBe(true)
    expect(dockState.basketOpen.value).toBe(true)
    expect(dockState.interceptDrawerOpen.value).toBe(true)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.proxySettingsOpen.value).toBe(false)
    expect(dockState.basketOpen.value).toBe(true)
    expect(dockState.interceptDrawerOpen.value).toBe(true)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.basketOpen.value).toBe(false)
    expect(dockState.interceptDrawerOpen.value).toBe(true)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.interceptDrawerOpen.value).toBe(false)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.workbenchOpen.value).toBe(false)

    wrapper.unmount()
  })

  it('waits for persisted history before opening repeater after restart', async () => {
    let resolveDraftStore: (value: unknown) => void = () => {}
    let resolveAttackStore: (value: unknown) => void = () => {}
    let resolveReplayStore: (value: unknown) => void = () => {}
    const endpoint = { scheme: 'https', host: 'example.com', port: 443 }
    const rawRequest = 'GET /api/test HTTP/1.1\r\nHost: example.com\r\n\r\n'

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === 'load_traffic_draft_store') {
        return new Promise(resolve => { resolveDraftStore = resolve }) as never
      }
      if (command === 'load_attack_workspace_store') {
        return new Promise(resolve => { resolveAttackStore = resolve }) as never
      }
      if (command === 'load_replay_run_store') {
        return new Promise(resolve => { resolveReplayStore = resolve }) as never
      }
      return Promise.resolve({ success: true, data: null }) as never
    })

    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await nextTick()
    wrapper.findComponent({ name: 'TrafficWorkbenchSidebar' }).vm.$emit('openRepeater')
    await nextTick()

    const dockState = useImmersiveTrafficDockState()
    expect(dockState.workbenchOpen.value).toBe(false)

    resolveDraftStore({
      success: true,
      data: {
        activeDraftId: 'draft-1',
        drafts: [{
          draft: {
            id: 'draft-1',
            title: 'Persisted request',
            source: null,
            sourceSnapshotId: null,
            endpoint,
            endpointMode: 'auto',
            rawRequest,
            preferredView: 'raw',
            pinned: false,
            activeRevisionId: 'rev-1',
            createdAt: 1,
            updatedAt: 1,
          },
          revisions: [{
            id: 'rev-1',
            draftId: 'draft-1',
            endpoint,
            rawRequest,
            reason: 'clone',
            createdAt: 1,
          }],
        }],
      },
    })
    resolveAttackStore({ success: true, data: { activeWorkspaceId: null, workspaces: [] } })
    resolveReplayStore({ success: true, data: { replayRuns: [] } })
    await flushPromises()

    expect(useTrafficWorkbenchStore().drafts.drafts.value).toHaveLength(1)
    expect(dockState.workbenchOpen.value).toBe(true)
    expect(dockState.activeWorkbenchTool.value).toBe('repeater')

    wrapper.unmount()
  })

  it('shows repeater history count on the main workspace repeater button', async () => {
    const endpoint = { scheme: 'https', host: 'example.com', port: 443 }
    const rawRequest = 'GET /api/test HTTP/1.1\r\nHost: example.com\r\n\r\n'
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'load_traffic_draft_store') {
        return {
          success: true,
          data: {
            activeDraftId: 'draft-1',
            drafts: [{
              draft: {
                id: 'draft-1',
                title: 'Persisted request',
                source: null,
                sourceSnapshotId: null,
                endpoint,
                endpointMode: 'auto',
                rawRequest,
                preferredView: 'raw',
                pinned: false,
                activeRevisionId: 'rev-1',
                createdAt: 1,
                updatedAt: 1,
              },
              revisions: [{
                id: 'rev-1',
                draftId: 'draft-1',
                endpoint,
                rawRequest,
                reason: 'clone',
                createdAt: 1,
              }],
            }],
          },
        } as never
      }
      if (command === 'load_attack_workspace_store') {
        return { success: true, data: { activeWorkspaceId: null, workspaces: [] } } as never
      }
      if (command === 'load_replay_run_store') {
        return { success: true, data: { replayRuns: [] } } as never
      }
      return { success: true, data: null } as never
    })

    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await flushPromises()

    const mainStage = wrapper.findComponent({ name: 'TrafficWorkbenchMainStage' })
    const repeaterChip = mainStage.props('toolChips').find((chip: { tool: string }) => chip.tool === 'repeater')
    expect(repeaterChip).toMatchObject({ count: 1 })

    wrapper.unmount()
  })
})
