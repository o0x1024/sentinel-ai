import { beforeEach, describe, expect, it } from 'vitest'
import { attemptEntitlementAutoRefresh } from './entitlementAutoRefresh'
import { resetEntitlementRefreshRuntimeState, markEntitlementRefreshFailure } from './entitlementRefreshState'

const licenseOnlyEntitlements = {
  tier: 'licensed',
  is_licensed: false,
  has_local_license: true,
  access_source: 'license_only',
  has_valid_entitlement_token: false,
  entitlement_feature_ids: [],
  entitlement_expires_at: null,
  entitlement_license_id: null,
  can_access_all_plugins: false,
  can_access_bug_bounty: false,
  can_manage_plugin_catalog: false,
  can_add_plugins: false,
  can_edit_plugins: false,
  can_delete_plugins: false,
  can_install_plugins: false,
  can_review_plugins: false,
  allowed_plugin_ids: [],
}

const fullEntitlements = {
  ...licenseOnlyEntitlements,
  tier: 'pro',
  is_licensed: true,
  access_source: 'entitlement_token',
  has_valid_entitlement_token: true,
  can_access_all_plugins: true,
  can_access_bug_bounty: true,
  can_manage_plugin_catalog: true,
  can_add_plugins: true,
  can_edit_plugins: true,
  can_delete_plugins: true,
  can_install_plugins: true,
  can_review_plugins: true,
}

const missingTokenStatus = {
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
}

const validTokenStatus = {
  exists: true,
  valid: true,
  tier: 'pro',
  license_id: 'license-1',
  feature_ids: ['bug_bounty'],
  issued_at: 100,
  expires_at: 200,
  expires_in_seconds: 3600,
  machine_id_match: true,
  error: null,
}

describe('attemptEntitlementAutoRefresh', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    window.localStorage.clear()
    resetEntitlementRefreshRuntimeState()
  })

  it('refreshes entitlement token after local license activation', async () => {
    let entitlementsReads = 0
    let tokenReads = 0

    global.testUtils.mockInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_entitlements') {
        entitlementsReads += 1
        return Promise.resolve(entitlementsReads >= 2 ? fullEntitlements : licenseOnlyEntitlements)
      }

      if (command === 'get_entitlement_token_status') {
        tokenReads += 1
        return Promise.resolve(tokenReads >= 2 ? validTokenStatus : missingTokenStatus)
      }

      if (command === 'refresh_entitlement_token') {
        return Promise.resolve({
          success: true,
          configured: true,
          message: 'token synced',
          error_code: null,
          retry_after_secs: null,
          token_stored: true,
          status: validTokenStatus,
        })
      }

      throw new Error(`Unexpected command: ${command}`)
    })

    const outcome = await attemptEntitlementAutoRefresh({ force: true })

    expect(outcome).toEqual({
      status: 'success',
      message: 'token synced',
    })
    expect(global.testUtils.mockInvoke).toHaveBeenCalledWith('refresh_entitlement_token')
  })

  it('skips refresh requests during cooldown unless forced', async () => {
    markEntitlementRefreshFailure({
      message: 'previous failure',
      retryAfterSecs: 300,
      at: Math.floor(Date.now() / 1000),
    })

    global.testUtils.mockInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_entitlements') {
        return Promise.resolve(licenseOnlyEntitlements)
      }

      if (command === 'get_entitlement_token_status') {
        return Promise.resolve(missingTokenStatus)
      }

      if (command === 'refresh_entitlement_token') {
        throw new Error('refresh should not be called during cooldown')
      }

      throw new Error(`Unexpected command: ${command}`)
    })

    const outcome = await attemptEntitlementAutoRefresh()

    expect(outcome).toEqual({
      status: 'skipped',
      reason: 'cooldown',
    })
    expect(global.testUtils.mockInvoke).not.toHaveBeenCalledWith('refresh_entitlement_token')
  })
})
