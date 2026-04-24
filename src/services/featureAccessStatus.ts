import { computed } from 'vue'
import {
  getEntitlementTokenStatus,
  getEntitlementTokenIssueCode,
  isEntitlementTokenExpiringSoon,
  refreshEntitlementTokenStatus,
  type EntitlementTokenIssueCode,
  type EntitlementTokenStatus,
  useEntitlementTokenStatusState,
} from './entitlementToken'

export interface FeatureAccessStatus {
  exists: boolean
  ready: boolean
  issueCode: EntitlementTokenIssueCode
  tier: string | null
  scopeIds: readonly string[]
  issuedAt: number | null
  expiresAt: number | null
  expiresInSeconds: number | null
  deviceMatched: boolean
  backendError: string | null
}

export const mapEntitlementToFeatureAccessStatus = (
  status: EntitlementTokenStatus,
): FeatureAccessStatus => ({
  exists: status.exists,
  ready: status.valid,
  issueCode: getEntitlementTokenIssueCode(status),
  tier: status.tier,
  scopeIds: status.feature_ids,
  issuedAt: status.issued_at,
  expiresAt: status.expires_at,
  expiresInSeconds: status.expires_in_seconds,
  deviceMatched: status.machine_id_match,
  backendError: status.error,
})

export const getFeatureAccessStatus = async () =>
  mapEntitlementToFeatureAccessStatus(await getEntitlementTokenStatus())

export const refreshFeatureAccessStatus = async () =>
  mapEntitlementToFeatureAccessStatus(await refreshEntitlementTokenStatus())

export const useFeatureAccessStatusState = () =>
  computed(() => mapEntitlementToFeatureAccessStatus(useEntitlementTokenStatusState().value))

export const isFeatureAccessExpiringSoon = (
  status: FeatureAccessStatus,
  thresholdSeconds = 24 * 60 * 60,
) =>
  isEntitlementTokenExpiringSoon(
    {
      exists: status.exists,
      valid: status.ready,
      tier: status.tier,
      license_id: null,
      feature_ids: status.scopeIds,
      issued_at: status.issuedAt,
      expires_at: status.expiresAt,
      expires_in_seconds: status.expiresInSeconds,
      machine_id_match: status.deviceMatched,
      error: status.backendError,
    },
    thresholdSeconds,
  )
