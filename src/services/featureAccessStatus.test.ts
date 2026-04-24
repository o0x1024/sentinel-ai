import { computed, ref } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import {
  isFeatureAccessExpiringSoon,
  mapEntitlementToFeatureAccessStatus,
  useFeatureAccessStatusState,
} from './featureAccessStatus'

const entitlementState = ref({
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
})

vi.mock('./entitlementToken', () => ({
  getEntitlementTokenStatus: vi.fn(),
  getEntitlementTokenIssueCode: vi.fn((status: { exists: boolean; valid: boolean; error: string | null }) => {
    if (status.valid) {
      return 'ok'
    }

    if (!status.exists) {
      return 'missing'
    }

    return 'invalid'
  }),
  isEntitlementTokenExpiringSoon: vi.fn((status: { valid: boolean; expires_in_seconds: number | null }, thresholdSeconds = 24 * 60 * 60) =>
    Boolean(status.valid && status.expires_in_seconds != null && status.expires_in_seconds <= thresholdSeconds),
  ),
  refreshEntitlementTokenStatus: vi.fn(),
  useEntitlementTokenStatusState: () => computed(() => entitlementState.value),
}))

describe('featureAccessStatus', () => {
  it('maps raw entitlement status into UI-facing feature access status', () => {
    expect(mapEntitlementToFeatureAccessStatus(entitlementState.value)).toEqual({
      exists: true,
      ready: true,
      issueCode: 'ok',
      tier: 'pro',
      scopeIds: ['bug_bounty'],
      issuedAt: 100,
      expiresAt: 200,
      expiresInSeconds: 3600,
      deviceMatched: true,
      backendError: null,
    })
  })

  it('exposes computed UI state and expiring-soon checks', () => {
    const state = useFeatureAccessStatusState()
    expect(state.value.ready).toBe(true)
    expect(isFeatureAccessExpiringSoon(state.value, 4000)).toBe(true)
  })
})
