import { refreshFeatureEntitlements } from './featureEntitlements'
import { refreshEntitlementTokenFromServer } from './entitlementRefresh'
import {
  canAttemptEntitlementAutoRefresh,
  markEntitlementRefreshAttempt,
  markEntitlementRefreshFailure,
  markEntitlementRefreshSuccess,
} from './entitlementRefreshState'
import {
  getEntitlementTokenStatus,
  isEntitlementTokenExpiringSoon,
  refreshEntitlementTokenStatus,
} from './entitlementToken'

export interface EntitlementAutoRefreshOptions {
  force?: boolean
  expiringSoonThresholdSeconds?: number
}

export type EntitlementAutoRefreshOutcome =
  | {
      status: 'skipped'
      reason: 'debug' | 'cooldown' | 'not_needed'
    }
  | {
      status: 'success'
      message: string
    }
  | {
      status: 'failure'
      configured: boolean
      message: string
      errorCode: string | null
      retryAfterSecs: number | null
    }

const DEFAULT_EXPIRING_SOON_THRESHOLD_SECONDS = 6 * 60 * 60

export async function attemptEntitlementAutoRefresh(
  options: EntitlementAutoRefreshOptions = {},
): Promise<EntitlementAutoRefreshOutcome> {
  const entitlements = await refreshFeatureEntitlements()

  if (entitlements.access_source === 'debug') {
    return { status: 'skipped', reason: 'debug' }
  }

  const tokenStatus = await getEntitlementTokenStatus()
  const thresholdSeconds =
    options.expiringSoonThresholdSeconds ?? DEFAULT_EXPIRING_SOON_THRESHOLD_SECONDS
  const needsRefresh =
    !tokenStatus.valid || isEntitlementTokenExpiringSoon(tokenStatus, thresholdSeconds)

  if (!needsRefresh) {
    return { status: 'skipped', reason: 'not_needed' }
  }

  if (!options.force && !canAttemptEntitlementAutoRefresh()) {
    return { status: 'skipped', reason: 'cooldown' }
  }

  try {
    markEntitlementRefreshAttempt()
    const result = await refreshEntitlementTokenFromServer()

    if (result.success) {
      markEntitlementRefreshSuccess()
      await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus()])
      return {
        status: 'success',
        message: result.message,
      }
    }

    if (result.configured) {
      markEntitlementRefreshFailure({
        message: result.message,
        errorCode: result.error_code,
        retryAfterSecs: result.retry_after_secs,
      })
    }

    await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus()])
    return {
      status: 'failure',
      configured: result.configured,
      message: result.message,
      errorCode: result.error_code,
      retryAfterSecs: result.retry_after_secs,
    }
  } catch (error) {
    const message = String(error)
    markEntitlementRefreshFailure({
      message,
      errorCode: 'auto_refresh_exception',
      retryAfterSecs: 300,
    })
    await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus()])
    return {
      status: 'failure',
      configured: true,
      message,
      errorCode: 'auto_refresh_exception',
      retryAfterSecs: 300,
    }
  }
}
