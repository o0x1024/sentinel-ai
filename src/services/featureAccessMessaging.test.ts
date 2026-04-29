import { describe, expect, it } from 'vitest'
import { getFeatureAccessIssueMessage } from './featureAccessMessaging'

describe('getFeatureAccessIssueMessage', () => {
  it('maps missing entitlement state to user-facing feature access wording', () => {
    expect(getFeatureAccessIssueMessage({
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
    })).toBe('缺少服务端授权')
  })

  it('preserves backend error detail while hiding entitlement terminology', () => {
    expect(getFeatureAccessIssueMessage({
      exists: true,
      ready: false,
      issueCode: 'invalid',
      tier: null,
      scopeIds: [],
      issuedAt: null,
      expiresAt: null,
      expiresInSeconds: null,
      deviceMatched: false,
      backendError: 'customer revoked',
    })).toBe('服务端授权不可用：customer revoked')
  })
})
