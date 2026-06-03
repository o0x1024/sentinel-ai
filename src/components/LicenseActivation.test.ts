import { flushPromises, mount } from '@vue/test-utils'
import { readonly, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import LicenseActivation from './LicenseActivation.vue'

const entitlementsState = ref({
  tier: 'licensed',
  is_licensed: false,
  has_local_license: true,
  access_source: 'server_activation',
  trial_active: false,
  trial_started_at: null,
  trial_expires_at: null,
  trial_remaining_seconds: null,
  trial_days_remaining: null,
  has_valid_entitlement_token: false,
  entitlement_feature_ids: [],
  entitlement_expires_at: null,
  entitlement_license_id: null,
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

const featureAccessStatusState = ref({
  exists: false,
  ready: false,
  issueCode: 'missing',
  tier: null,
  scopeIds: [],
  issuedAt: null,
  expiresAt: null,
  expiresInSeconds: null,
  deviceMatched: false,
  backendError: null,
})

const refreshRuntimeState = ref({
  last_attempt_at: null,
  last_success_at: null,
  last_failure_at: null,
  next_retry_at: null,
  consecutive_failures: 0,
  last_error: null,
  last_error_code: null,
})

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../services/featureEntitlements', () => ({
  refreshFeatureEntitlements: vi.fn(async () => entitlementsState.value),
  useFeatureEntitlementsState: () => readonly(entitlementsState),
}))

vi.mock('../services/featureAccessStatus', () => ({
  refreshFeatureAccessStatus: vi.fn(async () => featureAccessStatusState.value),
  useFeatureAccessStatusState: () => readonly(featureAccessStatusState),
}))

vi.mock('../services/entitlementRefresh', () => ({
  getEntitlementRefreshConfig: vi.fn(async () => ({
    enabled: false,
    endpoint: '',
    api_key: '',
    customer_id: '',
    timeout_secs: 15,
    api_key_configured: false,
  })),
  refreshEntitlementTokenFromServer: vi.fn(),
}))

vi.mock('../services/entitlementAutoRefresh', () => ({
  attemptEntitlementAutoRefresh: vi.fn(),
}))

vi.mock('../services/entitlementRefreshState', () => ({
  formatDurationLabel: vi.fn(() => '现在'),
  getEntitlementRefreshCooldownSeconds: vi.fn(() => 0),
  markEntitlementRefreshFailure: vi.fn(),
  markEntitlementRefreshSuccess: vi.fn(),
  useEntitlementRefreshRuntimeState: () => readonly(refreshRuntimeState),
}))

describe('LicenseActivation', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockInvoke.mockImplementation((command: string) => {
      if (command === 'get_license_info') {
        return Promise.resolve({
          machine_id: '84DE-A1B6-FCA0-88DD',
          is_licensed: true,
          needs_activation: false,
          trial_active: false,
          trial_started_at: null,
          trial_expires_at: null,
          trial_remaining_seconds: null,
          trial_days_remaining: null,
        })
      }

      throw new Error(`Unexpected command: ${command}`)
    })
  })

  it('shows username and activation key instead of admin refresh fields', async () => {
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

    expect(wrapper.text()).not.toContain('Refresh Endpoint')
    expect(wrapper.text()).not.toContain('Customer ID')
    expect(wrapper.text()).not.toContain('Refresh API Key')
    expect(wrapper.text()).not.toContain('Timeout (seconds)')
    expect(wrapper.text()).not.toContain('保存自动刷新配置')
    expect(wrapper.text()).not.toContain('写入 Token')
    expect(wrapper.text()).not.toContain('清除 Token')
    expect(wrapper.text()).toContain('用户名')
    expect(wrapper.text()).toContain('激活密钥')
    expect(wrapper.text()).not.toContain('服务端激活配置')

    wrapper.unmount()
  })
})
