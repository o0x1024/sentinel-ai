import { describe, expect, it } from 'vitest'
import { buildLicenseActivationViewState } from './licenseActivationViewState'

const formatTimestamp = (timestamp: number | null) => (timestamp == null ? '' : `ts:${timestamp}`)
const formatDuration = (seconds: number) => `${seconds}s`

describe('licenseActivationViewState', () => {
  it('describes the missing local license path as a single upgrade flow', () => {
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
      dialogTitle: '完成升级',
      dialogSubtitle: '输入许可证密钥以完成当前设备的升级。',
      upgradeEntryLabel: '完成升级',
      statusBadgeLabel: '未激活',
      featureAccessTone: 'alert-warning',
      featureAccessText: '当前未激活本地 license。release 环境下高级功能不可用。',
    })
  })

  it('describes a ready feature access state without leaking backend token wording', () => {
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
      dialogSubtitle: '当前授权已就绪，可在这里查看高级功能同步状态。',
      upgradeEntryLabel: '检查授权状态',
      statusBadgeLabel: '已就绪',
      featureAccessTone: 'alert-success',
      featureAccessText: '高级功能权限已就绪，过期时间：ts:200',
      featureAccessSummary: '高级功能同步已完成，权限过期时间：ts:200',
      refreshRuntimeText: '最近一次自动同步成功时间：ts:120',
    })
  })

  it('surfaces refresh cooldowns and failures when local license exists but access is still incomplete', () => {
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
      dialogSubtitle: '本地授权已完成，系统会继续自动补齐高级功能权限。',
      upgradeEntryLabel: '完成升级',
      statusBadgeLabel: '待完成',
      featureAccessTone: 'alert-info',
      featureAccessText: '已激活本地 license，但高级功能权限已过期。高价值功能仍会受限。',
      featureAccessSummary: '本地 license 已激活，但当前高级功能权限不可用：高级功能权限已过期',
      refreshRuntimeText: '自动刷新冷却中，下次重试300s。',
      refreshServiceHint: '服务端自动同步尚未配置。请让管理员在 设置 > 安全 > 高级功能权限同步管理 中配置刷新服务。',
    })
  })
})
