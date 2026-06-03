import { describe, expect, it } from 'vitest'
import {
  getFeatureAccessAutoRefreshFailureMessage,
  getFeatureAccessReminderMessage,
  shouldResetFeatureAccessReminder,
} from './featureAccessAttention'

const missingFeatureAccess = {
  exists: false,
  ready: false,
  issueCode: 'missing' as const,
  tier: null,
  scopeIds: [],
  issuedAt: null,
  expiresAt: null,
  expiresInSeconds: null,
  deviceMatched: false,
  backendError: null,
}

describe('featureAccessAttention', () => {
  it('builds a warning reminder when server activation exists but authorization is unavailable', () => {
    expect(getFeatureAccessReminderMessage({
      hasLocalLicense: true,
      isDebugAccess: false,
      hasShownReminder: false,
      featureAccessStatus: missingFeatureAccess,
      cooldownSeconds: 180,
    })).toEqual({
      level: 'warning',
      message: '当前服务端授权不可用：缺少服务端授权，付费功能仍受限；自动重试将在3 分钟后。',
    })
  })

  it('builds an info reminder when server authorization is expiring soon', () => {
    expect(getFeatureAccessReminderMessage({
      hasLocalLicense: true,
      isDebugAccess: false,
      hasShownReminder: false,
      featureAccessStatus: {
        ...missingFeatureAccess,
        exists: true,
        ready: true,
        issueCode: 'ok',
        deviceMatched: true,
        expiresInSeconds: 60,
      },
      cooldownSeconds: 0,
      expiringSoonThresholdSeconds: 120,
    })).toEqual({
      level: 'info',
      message: '服务端授权即将过期，建议尽快完成续期。',
    })
  })

  it('only surfaces refresh failures on the first failure or customer revocation', () => {
    expect(getFeatureAccessAutoRefreshFailureMessage({
      outcome: {
        status: 'failure',
        configured: true,
        message: 'customer revoked',
        errorCode: 'customer_revoked',
        retryAfterSecs: 600,
      },
      consecutiveFailures: 3,
      cooldownSeconds: 600,
    })).toBe('服务端授权自动刷新失败：customer revoked；下次自动重试10 分钟后')

    expect(getFeatureAccessAutoRefreshFailureMessage({
      outcome: {
        status: 'failure',
        configured: true,
        message: 'temporary offline',
        errorCode: 'timeout',
        retryAfterSecs: 600,
      },
      consecutiveFailures: 3,
      cooldownSeconds: 600,
    })).toBeNull()
  })

  it('resets reminder state once server activation disappears, debug bypass is active, or access becomes ready', () => {
    expect(shouldResetFeatureAccessReminder({
      hasLocalLicense: false,
      isDebugAccess: false,
      featureAccessReady: false,
    })).toBe(true)

    expect(shouldResetFeatureAccessReminder({
      hasLocalLicense: true,
      isDebugAccess: true,
      featureAccessReady: false,
    })).toBe(true)

    expect(shouldResetFeatureAccessReminder({
      hasLocalLicense: true,
      isDebugAccess: false,
      featureAccessReady: true,
    })).toBe(true)

    expect(shouldResetFeatureAccessReminder({
      hasLocalLicense: true,
      isDebugAccess: false,
      featureAccessReady: false,
    })).toBe(false)
  })
})
