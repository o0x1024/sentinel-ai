import { flushPromises, mount } from '@vue/test-utils'
import {
  KeepAlive,
  computed,
  defineComponent,
  h,
  onActivated,
  onDeactivated,
} from 'vue'
import {
  RouterView,
  createMemoryHistory,
  createRouter,
  useRoute,
  type RouteRecordRaw,
} from 'vue-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

const lifecycleLog: string[] = []

const SecurityCenterMock = defineComponent({
  name: 'SecurityCenter',
  setup() {
    onActivated(() => {
      lifecycleLog.push('security:activated')
    })
    onDeactivated(() => {
      lifecycleLog.push('security:deactivated')
    })

    return () => h('div', { 'data-testid': 'security-center' }, 'security-center')
  },
})

const TrafficAnalysisMock = defineComponent({
  name: 'TrafficAnalysis',
  setup() {
    onActivated(() => {
      lifecycleLog.push('traffic:activated')
    })
    onDeactivated(() => {
      lifecycleLog.push('traffic:deactivated')
    })

    return () => h('div', { 'data-testid': 'traffic-analysis' }, 'traffic-analysis')
  },
})

const AppShellHarness = defineComponent({
  name: 'AppShellHarness',
  setup() {
    const route = useRoute()
    const showImmersiveSecurityCenterWorkspace = computed(() =>
      route.path.startsWith('/security-center'),
    )

    return () =>
      h(RouterView, null, {
        default: ({ Component }) =>
          h(
            KeepAlive,
            { include: ['SecurityCenter', 'TrafficAnalysis'] },
            () =>
              Component
                ? h(Component, {
                    class: 'min-h-full',
                    style: showImmersiveSecurityCenterWorkspace.value ? 'display: none;' : undefined,
                  })
                : null,
          ),
      })
  },
})

function createTestRouter() {
  const routes: RouteRecordRaw[] = [
    {
      path: '/security-center',
      name: 'SecurityCenter',
      component: SecurityCenterMock,
    },
    {
      path: '/traffic',
      name: 'TrafficAnalysis',
      component: TrafficAnalysisMock,
    },
  ]

  return createRouter({
    history: createMemoryHistory(),
    routes,
  })
}

describe('App keep-alive routing', () => {
  afterEach(() => {
    lifecycleLog.length = 0
    vi.restoreAllMocks()
  })

  it('opens traffic analysis from an immersive security center state without vue runtime errors', async () => {
    const router = createTestRouter()
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    router.push('/security-center')
    await router.isReady()

    const wrapper = mount(AppShellHarness, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await flushPromises()
    expect(wrapper.find('[data-testid="security-center"]').exists()).toBe(true)

    await expect(router.push('/traffic')).resolves.toBeUndefined()
    await flushPromises()

    expect(wrapper.find('[data-testid="traffic-analysis"]').exists()).toBe(true)
    expect(lifecycleLog).toContain('security:deactivated')
    expect(lifecycleLog).toContain('traffic:activated')
    expect(consoleErrorSpy).not.toHaveBeenCalled()

    wrapper.unmount()
  })
})
