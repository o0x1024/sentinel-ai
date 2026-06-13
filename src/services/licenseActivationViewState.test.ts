import { describe, expect, it } from 'vitest'
import { buildLicenseActivationViewState } from './licenseActivationViewState'

const formatTimestamp = (timestamp: number | null) => (timestamp == null ? '' : `ts:${timestamp}`)

describe('licenseActivationViewState', () => {
  it('describes the missing local license path', () => {
    expect(buildLicenseActivationViewState({
      hasLocalLicense: false,
      isDebugAccess: false,
      formatTimestamp,
    })).toMatchObject({
      dialogTitle: '输入 License 激活',
      dialogSubtitle: '将管理员签发的 License 粘贴到下方，完成当前设备激活。',
      upgradeEntryLabel: '输入 License 激活',
      statusBadgeLabel: '未激活',
      featureAccessTone: 'alert-warning',
      featureAccessText: '当前未完成本地 License 激活。release 环境下付费功能不可用。',
    })
  })

  it('describes an active local license state', () => {
    expect(buildLicenseActivationViewState({
      hasLocalLicense: true,
      isDebugAccess: false,
      formatTimestamp,
    })).toMatchObject({
      dialogTitle: '授权状态',
      dialogSubtitle: '当前设备已完成本地 License 激活，可在这里查看授权状态。',
      upgradeEntryLabel: '查看授权状态',
      statusBadgeLabel: '已激活',
      featureAccessTone: 'alert-success',
      featureAccessText: '本地 License 已激活，当前设备可使用全部付费功能。',
      featureAccessSummary: '本地 License 有效，无需联网续期。',
    })
  })
})
