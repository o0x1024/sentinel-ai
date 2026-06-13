import { getFeatureEntitlements, type AppFeatureEntitlements } from './featureEntitlements'

interface LicenseActivationViewStateInput {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  formatTimestamp: (timestamp: number | null) => string
}

export interface LicenseActivationViewState {
  dialogTitle: string
  dialogSubtitle: string
  upgradeEntryLabel: string
  statusBadgeLabel: string
  featureAccessTone: string
  featureAccessText: string
  featureAccessSummary: string
}

export const buildLicenseActivationViewState = (
  input: LicenseActivationViewStateInput,
): LicenseActivationViewState => {
  const {
    hasLocalLicense,
    isDebugAccess,
  } = input

  const dialogTitle = hasLocalLicense
    ? '授权状态'
    : '输入 License 激活'
  const dialogSubtitle = !hasLocalLicense
    ? '将管理员签发的 License 粘贴到下方，完成当前设备激活。'
    : '当前设备已完成本地 License 激活，可在这里查看授权状态。'
  const upgradeEntryLabel = hasLocalLicense ? '查看授权状态' : '输入 License 激活'

  let statusBadgeLabel = '待激活'
  let featureAccessTone = 'alert-info'

  if (isDebugAccess) {
    statusBadgeLabel = 'Debug'
    featureAccessTone = 'alert-success'
  } else if (!hasLocalLicense) {
    statusBadgeLabel = '未激活'
    featureAccessTone = 'alert-warning'
  } else {
    statusBadgeLabel = '已激活'
    featureAccessTone = 'alert-success'
  }

  const featureAccessText = resolveFeatureAccessText({
    hasLocalLicense,
    isDebugAccess,
  })
  const featureAccessSummary = resolveFeatureAccessSummary({
    hasLocalLicense,
    isDebugAccess,
  })

  return {
    dialogTitle,
    dialogSubtitle,
    upgradeEntryLabel,
    statusBadgeLabel,
    featureAccessTone,
    featureAccessText,
    featureAccessSummary,
  }
}

const resolveFeatureAccessText = ({
  hasLocalLicense,
  isDebugAccess,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
}) => {
  if (isDebugAccess) {
    return '当前为 debug 模式，已绕过 release 环境下的本地 License 限制。'
  }

  if (!hasLocalLicense) {
    return '当前未完成本地 License 激活。release 环境下付费功能不可用。'
  }

  return '本地 License 已激活，当前设备可使用全部付费功能。'
}

const resolveFeatureAccessSummary = ({
  hasLocalLicense,
  isDebugAccess,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
}) => {
  if (isDebugAccess) {
    return 'release 授权链路调试区。开发态不会因为 License 缺失而被拦截。'
  }

  if (!hasLocalLicense) {
    return '请先复制设备 ID 给管理员签发 License，再粘贴到激活框完成绑定。'
  }

  return '本地 License 有效，无需联网续期。'
}

export type { AppFeatureEntitlements }

export { getFeatureEntitlements }
