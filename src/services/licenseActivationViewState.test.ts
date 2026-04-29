import { describe, expect, it } from 'vitest'
import { buildLicenseActivationViewState } from './licenseActivationViewState'

const formatTimestamp = (timestamp: number | null) => (timestamp == null ? '' : `ts:${timestamp}`)
const formatDuration = (seconds: number) => `${seconds}s`

describe('licenseActivationViewState', () => {
  it('describes the missing server activation path as a single activation flow', () => {
    expect(buildLicenseActivationViewState({
      hasLocalLicense: false,
      isDebugAccess: false,
      featureAccessStatus: {
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
      },
      refreshServiceConfigured: false,
      refreshRuntime: {
        last_success_at: null,
        next_retry_at: null,
        last_error: null,
      },
      refreshCooldownSeconds: 0,
      formatTimestamp,
      formatDuration,
    })).toMatchObject({
      dialogTitle: '服务端激活',
      dialogSubtitle: '连接授权服务，完成当前设备激活。',
      upgradeEntryLabel: '服务端激活',
      statusBadgeLabel: '未激活',
      featureAccessTone: 'alert-warning',
      featureAccessText: '当前未完成服务端激活。release 环境下付费功能不可用。',
    })
  })

  it('describes a ready server authorization state', () => {
    expect(buildLicenseActivationViewState({
      hasLocalLicense: true,
      isDebugAccess: false,
      featureAccessStatus: {
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
      },
      refreshServiceConfigured: true,
      refreshRuntime: {
        last_success_at: 120,
        next_retry_at: null,
        last_error: null,
      },
      refreshCooldownSeconds: 0,
      formatTimestamp,
      formatDuration,
    })).toMatchObject({
      dialogTitle: '检查授权状态',
      dialogSubtitle: '当前授权已就绪，可在这里查看服务端授权状态。',
      upgradeEntryLabel: '检查授权状态',
      statusBadgeLabel: '已就绪',
      featureAccessTone: 'alert-success',
      featureAccessText: '服务端授权已就绪，过期时间：ts:200',
      featureAccessSummary: '服务端授权有效，过期时间：ts:200',
      refreshRuntimeText: '最近一次自动续期成功时间：ts:120',
    })
  })

  it('surfaces refresh cooldowns and failures when server authorization is incomplete', () => {
    expect(buildLicenseActivationViewState({
      hasLocalLicense: true,
      isDebugAccess: false,
      featureAccessStatus: {
        exists: true,
        ready: false,
        issueCode: 'expired',
        tier: null,
        scopeIds: [],
        issuedAt: null,
        expiresAt: null,
        expiresInSeconds: null,
        deviceMatched: true,
        backendError: null,
      },
      refreshServiceConfigured: false,
      refreshRuntime: {
        last_success_at: null,
        next_retry_at: 300,
        last_error: 'customer revoked',
      },
      refreshCooldownSeconds: 300,
      formatTimestamp,
      formatDuration,
    })).toMatchObject({
      dialogTitle: '检查授权状态',
      dialogSubtitle: '当前授权令牌不可用，系统会继续从服务端刷新。',
      upgradeEntryLabel: '服务端激活',
      statusBadgeLabel: '待完成',
      featureAccessTone: 'alert-info',
      featureAccessText: '服务端授权不可用：服务端授权已过期。付费功能仍会受限。',
      featureAccessSummary: '当前服务端授权不可用：服务端授权已过期',
      refreshRuntimeText: '自动刷新冷却中，下次重试300s。',
      refreshServiceHint: '服务端激活尚未配置。请让管理员在 设置 > 安全 > 高级功能权限同步管理 中配置刷新服务。',
    })
  })
})
