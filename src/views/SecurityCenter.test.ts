import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import {
  createMemoryHistory,
  createRouter,
  type RouteRecordRaw,
} from 'vue-router'
import { describe, expect, it, vi } from 'vitest'
import SecurityCenter from './SecurityCenter.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

vi.mock('../components/SecurityCenter/VulnerabilitiesPanel.vue', () => ({
  default: defineComponent({
    name: 'VulnerabilitiesPanel',
    setup() {
      return () => h('div', 'vulnerabilities-panel')
    },
  }),
}))

vi.mock('../components/SecurityCenter/LlmSecurityPanel.vue', () => ({
  default: defineComponent({
    name: 'LlmSecurityPanel',
    setup() {
      return () => h('div', 'llm-security-panel')
    },
  }),
}))

vi.mock('../components/SecurityCenter/SecurityWorkbenchPage.vue', () => ({
  default: defineComponent({
    name: 'SecurityWorkbenchPage',
    setup() {
      return () => h('div', 'security-workbench-panel')
    },
  }),
}))

async function mountSecurityCenter(initialPath: string) {
  const routes: RouteRecordRaw[] = [
    {
      path: '/security-center/workbench/:caseId?',
      name: 'SecurityWorkbench',
      component: SecurityCenter,
    },
    {
      path: '/security-center',
      name: 'SecurityCenter',
      component: SecurityCenter,
    },
    {
      path: '/vulnerabilities',
      name: 'Vulnerabilities',
      component: SecurityCenter,
    },
  ]

  const router = createRouter({
    history: createMemoryHistory(),
    routes,
  })

  await router.push(initialPath)
  await router.isReady()

  const wrapper = mount(SecurityCenter, {
    global: {
      plugins: [router],
      mocks: {
        $t: (key: string, fallback?: string) => fallback ?? key,
      },
    },
  })

  await nextTick()

  return { wrapper, router }
}

describe('SecurityCenter', () => {
  it('defaults /security-center to vulnerabilities tab', async () => {
    const { wrapper } = await mountSecurityCenter('/security-center')

    expect(wrapper.text()).toContain('vulnerabilities-panel')
    expect(wrapper.text()).not.toContain('security-workbench-panel')

    wrapper.unmount()
  })

  it('keeps /vulnerabilities on vulnerabilities tab', async () => {
    const { wrapper } = await mountSecurityCenter('/vulnerabilities')

    expect(wrapper.text()).toContain('vulnerabilities-panel')
    expect(wrapper.text()).not.toContain('security-workbench-panel')

    wrapper.unmount()
  })

  it('keeps workbench routes on workbench tab', async () => {
    const { wrapper } = await mountSecurityCenter('/security-center/workbench')

    expect(wrapper.text()).toContain('security-workbench-panel')
    expect(wrapper.text()).not.toContain('vulnerabilities-panel')

    wrapper.unmount()
  })
})
