import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import {
  createMemoryHistory,
  createRouter,
  type RouteRecordRaw,
} from 'vue-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ImmersiveSecurityCenterOverlay from './ImmersiveSecurityCenterOverlay.vue'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterSidebarOpen,
  openImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

vi.mock('@/components/SecurityCenter/SecurityCenterImmersiveSidebar.vue', () => ({
  default: defineComponent({
    name: 'SecurityCenterImmersiveSidebar',
    setup() {
      return () => h('div', 'sidebar')
    },
  }),
}))

vi.mock('@/views/SecurityCenter.vue', () => ({
  default: defineComponent({
    name: 'SecurityCenter',
    inheritAttrs: false,
    setup(_, { attrs }) {
      return () => h('div', { ...attrs, class: ['security-center', attrs.class] }, 'security-center')
    },
  }),
}))

function createTestRouter() {
  const TrafficPage = defineComponent({
    name: 'TrafficPage',
    setup() {
      return () => h('div', 'traffic')
    },
  })

  const routes: RouteRecordRaw[] = [
    {
      path: '/traffic',
      component: TrafficPage,
    },
    {
      path: '/security-center',
      component: TrafficPage,
    },
  ]

  return createRouter({
    history: createMemoryHistory(),
    routes,
  })
}

describe('ImmersiveSecurityCenterOverlay', () => {
  beforeEach(() => {
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    vi.stubGlobal('ResizeObserver', class {
      observe() {}
      disconnect() {}
    })
  })

  afterEach(() => {
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    vi.unstubAllGlobals()
  })

  it('hides the immersive security center sidebar on Escape', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()

    openImmersiveSecurityCenterSidebar('/traffic')

    const wrapper = mount(ImmersiveSecurityCenterOverlay, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()

    expect(immersiveSecurityCenterSidebarOpen.value).toBe(false)

    wrapper.unmount()
  })

  it('renders the workspace body with a shrinkable content column', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()

    openImmersiveSecurityCenterSidebar('/traffic')

    const wrapper = mount(ImmersiveSecurityCenterOverlay, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()

    const contentColumn = wrapper.find('.min-w-0.flex.flex-1.flex-col.overflow-hidden')
    expect(contentColumn.exists()).toBe(true)
    expect(wrapper.find('.security-center.min-h-0.flex-1').exists()).toBe(true)

    wrapper.unmount()
  })
})
