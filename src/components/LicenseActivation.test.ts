import { flushPromises, mount } from '@vue/test-utils'
import { readonly, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import LicenseActivation from './LicenseActivation.vue'

const entitlementsState = ref({
  tier: 'free',
  is_licensed: false,
  has_local_license: false,
  access_source: 'free',
  can_access_all_plugins: false,
  can_access_bug_bounty: false,
  can_access_bot_console: false,
  can_manage_plugin_catalog: false,
  can_add_plugins: false,
  can_edit_plugins: false,
  can_delete_plugins: false,
  can_install_plugins: false,
  can_review_plugins: false,
  allowed_plugin_ids: [],
})

vi.mock('../services/featureEntitlements', () => ({
  refreshFeatureEntitlements: vi.fn(async () => entitlementsState.value),
  useFeatureEntitlementsState: () => readonly(entitlementsState),
}))

describe('LicenseActivation', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockInvoke.mockImplementation((command: string) => {
      if (command === 'get_license_info') {
        return Promise.resolve({
          machine_id: '84DE-A1B6-FCA0-88DD',
          is_licensed: false,
          needs_activation: true,
        })
      }

      throw new Error(`Unexpected command: ${command}`)
    })
  })

  it('shows local license activation fields instead of remote card activation', async () => {
    const wrapper = mount(LicenseActivation, {
      global: {
        stubs: {
          teleport: true,
        },
      },
    })

    await flushPromises()
    await wrapper.get('button.btn-warning.btn-sm').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('License')
    expect(wrapper.text()).toContain('当前设备')
    expect(wrapper.text()).not.toContain('用户名')
    expect(wrapper.text()).not.toContain('激活密钥')
    expect(wrapper.text()).not.toContain('服务端授权')
    expect(wrapper.text()).not.toContain('立即刷新授权')

    wrapper.unmount()
  })
})
