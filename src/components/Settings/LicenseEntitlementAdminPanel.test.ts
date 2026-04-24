import { flushPromises, mount } from '@vue/test-utils'
import { readonly, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import LicenseEntitlementAdminPanel from './LicenseEntitlementAdminPanel.vue'

const getFeatureEntitlements = vi.fn()
const refreshFeatureEntitlements = vi.fn()
const getEntitlementRefreshConfig = vi.fn()
const saveEntitlementRefreshConfig = vi.fn()
const refreshEntitlementTokenFromServer = vi.fn()
const refreshEntitlementTokenStatus = vi.fn()
const storeEntitlementTokenFromAdmin = vi.fn()
const clearEntitlementTokenFromAdmin = vi.fn()

const tokenStatusState = ref({
  exists: false,
  valid: false,
  tier: null,
  license_id: null,
  feature_ids: [],
  issued_at: null,
  expires_at: null,
  expires_in_seconds: null,
  machine_id_match: false,
  error: null,
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
    t: (key: string, params?: Record<string, string>) =>
      params?.expiresAt ? `${key}:${params.expiresAt}` : key,
  }),
}))

vi.mock('@/services/featureEntitlements', () => ({
  getFeatureEntitlements: (...args: unknown[]) => getFeatureEntitlements(...args),
  refreshFeatureEntitlements: (...args: unknown[]) => refreshFeatureEntitlements(...args),
}))

vi.mock('@/services/entitlementRefresh', () => ({
  getEntitlementRefreshConfig: (...args: unknown[]) => getEntitlementRefreshConfig(...args),
  saveEntitlementRefreshConfig: (...args: unknown[]) => saveEntitlementRefreshConfig(...args),
  refreshEntitlementTokenFromServer: (...args: unknown[]) => refreshEntitlementTokenFromServer(...args),
}))

vi.mock('@/services/entitlementToken', () => ({
  refreshEntitlementTokenStatus: (...args: unknown[]) => refreshEntitlementTokenStatus(...args),
  useEntitlementTokenStatusState: () => readonly(tokenStatusState),
  getEntitlementTokenIssueMessage: vi.fn(() => '缺少 entitlement token'),
}))

vi.mock('@/services/entitlementTokenAdmin', () => ({
  storeEntitlementTokenFromAdmin: (...args: unknown[]) => storeEntitlementTokenFromAdmin(...args),
  clearEntitlementTokenFromAdmin: (...args: unknown[]) => clearEntitlementTokenFromAdmin(...args),
}))

vi.mock('@/services/entitlementRefreshState', () => ({
  markEntitlementRefreshFailure: vi.fn(),
  markEntitlementRefreshSuccess: vi.fn(),
  useEntitlementRefreshRuntimeState: () => readonly(refreshRuntimeState),
}))

describe('LicenseEntitlementAdminPanel', () => {
  beforeEach(() => {
    getFeatureEntitlements.mockReset()
    refreshFeatureEntitlements.mockReset()
    getEntitlementRefreshConfig.mockReset()
    saveEntitlementRefreshConfig.mockReset()
    refreshEntitlementTokenFromServer.mockReset()
    refreshEntitlementTokenStatus.mockReset()
    storeEntitlementTokenFromAdmin.mockReset()
    clearEntitlementTokenFromAdmin.mockReset()

    getFeatureEntitlements.mockResolvedValue({
      has_local_license: true,
      access_source: 'license_only',
    })
    refreshFeatureEntitlements.mockResolvedValue(undefined)
    refreshEntitlementTokenStatus.mockResolvedValue(tokenStatusState.value)
    getEntitlementRefreshConfig.mockResolvedValue({
      enabled: true,
      endpoint: 'https://license.example.com/api/entitlements/refresh',
      api_key: '',
      customer_id: 'tenant-1',
      timeout_secs: 15,
      api_key_configured: true,
    })
    saveEntitlementRefreshConfig.mockImplementation(async (config: Record<string, unknown>) => ({
      enabled: true,
      endpoint: String(config.endpoint),
      api_key: '',
      customer_id: String(config.customer_id),
      timeout_secs: Number(config.timeout_secs),
      api_key_configured: true,
    }))
    storeEntitlementTokenFromAdmin.mockResolvedValue({
      success: true,
      message: 'token stored',
    })
    clearEntitlementTokenFromAdmin.mockResolvedValue({
      success: true,
      message: 'token cleared',
    })
  })

  it('loads and saves refresh configuration from the admin panel', async () => {
    const wrapper = mount(LicenseEntitlementAdminPanel)
    await flushPromises()

    const endpoint = wrapper.get('input[type="url"]')
    await endpoint.setValue('https://license.example.com/api/entitlements/refresh/v2')
    await wrapper.get('button.btn-primary').trigger('click')
    await flushPromises()

    expect(saveEntitlementRefreshConfig).toHaveBeenCalledWith({
      enabled: true,
      endpoint: 'https://license.example.com/api/entitlements/refresh/v2',
      api_key: '',
      customer_id: 'tenant-1',
      timeout_secs: 15,
    })
    expect(wrapper.text()).toContain('settings.security.licenseAdmin.saveSuccess')

    wrapper.unmount()
  })

  it('keeps manual token tools in the admin panel only', async () => {
    const wrapper = mount(LicenseEntitlementAdminPanel)
    await flushPromises()

    await wrapper.get('textarea').setValue('signed-token')
    const buttons = wrapper.findAll('button')
    const storeButton = buttons.find(button => button.text().includes('settings.security.licenseAdmin.manualTokenStore'))
    expect(storeButton).toBeTruthy()

    await storeButton!.trigger('click')
    await flushPromises()

    expect(storeEntitlementTokenFromAdmin).toHaveBeenCalledWith('signed-token')
    expect(wrapper.text()).toContain('settings.security.licenseAdmin.manualTokenTitle')
    expect(wrapper.text()).toContain('token stored')

    wrapper.unmount()
  })
})
