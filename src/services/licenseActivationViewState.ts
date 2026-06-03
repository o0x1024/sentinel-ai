import { getFeatureAccessIssueMessage } from './featureAccessMessaging'
import type { FeatureAccessStatus } from './featureAccessStatus'

interface LicenseActivationRefreshRuntime {
  last_success_at: number | null
  next_retry_at: number | null
  last_error: string | null
}

interface LicenseActivationViewStateInput {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  isTrialAccess: boolean
  trialExpiresAt: number | null
  trialDaysRemaining: number | null
  featureAccessStatus: FeatureAccessStatus
  refreshServiceConfigured: boolean
  refreshRuntime: LicenseActivationRefreshRuntime
  refreshCooldownSeconds: number
  formatTimestamp: (timestamp: number | null) => string
  formatDuration: (seconds: number) => string
}

export interface LicenseActivationViewState {
  dialogTitle: string
  dialogSubtitle: string
  upgradeEntryLabel: string
  statusBadgeLabel: string
  featureAccessTone: string
  featureAccessText: string
  featureAccessSummary: string
  refreshRuntimeText: string
  refreshServiceHint: string
}

export const buildLicenseActivationViewState = (
  input: LicenseActivationViewStateInput,
): LicenseActivationViewState => {
  const {
    hasLocalLicense,
    isDebugAccess,
    isTrialAccess,
    trialExpiresAt,
    trialDaysRemaining,
    featureAccessStatus,
    refreshServiceConfigured,
    refreshRuntime,
    refreshCooldownSeconds,
    formatTimestamp,
    formatDuration,
  } = input

  const dialogTitle = isTrialAccess ? '试用授权状态' : hasLocalLicense ? '检查授权状态' : '输入卡密激活'
  const dialogSubtitle = isTrialAccess
    ? '当前设备处于 7 天试用期，试用结束后需要输入用户名和卡密继续使用付费功能。'
    : !hasLocalLicense
    ? '输入管理员分配的用户名和激活密钥，完成当前设备激活。'
    : !featureAccessStatus.ready
      ? '当前授权令牌不可用，系统会继续从服务端刷新。'
      : '当前授权已就绪，可在这里查看服务端授权状态。'
  const upgradeEntryLabel = !hasLocalLicense || !featureAccessStatus.ready
    ? '输入卡密激活'
    : '检查授权状态'

  let statusBadgeLabel = '待完成'
  let featureAccessTone = 'alert-info'

  if (isDebugAccess) {
    statusBadgeLabel = 'Debug'
    featureAccessTone = 'alert-success'
  } else if (isTrialAccess) {
    statusBadgeLabel = trialDaysRemaining == null ? '试用中' : `试用 ${trialDaysRemaining} 天`
    featureAccessTone = 'alert-success'
  } else if (!hasLocalLicense) {
    statusBadgeLabel = '未激活'
    featureAccessTone = 'alert-warning'
  } else if (featureAccessStatus.ready) {
    statusBadgeLabel = '已就绪'
    featureAccessTone = 'alert-success'
  }

  const featureAccessText = resolveFeatureAccessText({
    hasLocalLicense,
    isDebugAccess,
    isTrialAccess,
    trialExpiresAt,
    trialDaysRemaining,
    featureAccessStatus,
    formatTimestamp,
  })
  const featureAccessSummary = resolveFeatureAccessSummary({
    hasLocalLicense,
    isDebugAccess,
    isTrialAccess,
    trialExpiresAt,
    trialDaysRemaining,
    featureAccessStatus,
    formatTimestamp,
  })
  const refreshRuntimeText = resolveRefreshRuntimeText({
    isDebugAccess,
    isTrialAccess,
    featureAccessStatus,
    refreshRuntime,
    refreshCooldownSeconds,
    formatTimestamp,
    formatDuration,
  })
  const refreshServiceHint = resolveRefreshServiceHint({
    isDebugAccess,
    isTrialAccess,
    refreshServiceConfigured,
  })

  return {
    dialogTitle,
    dialogSubtitle,
    upgradeEntryLabel,
    statusBadgeLabel,
    featureAccessTone,
    featureAccessText,
    featureAccessSummary,
    refreshRuntimeText,
    refreshServiceHint,
  }
}

const resolveFeatureAccessText = ({
  hasLocalLicense,
  isDebugAccess,
  isTrialAccess,
  trialExpiresAt,
  trialDaysRemaining,
  featureAccessStatus,
  formatTimestamp,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  isTrialAccess: boolean
  trialExpiresAt: number | null
  trialDaysRemaining: number | null
  featureAccessStatus: FeatureAccessStatus
  formatTimestamp: (timestamp: number | null) => string
}) => {
  if (isDebugAccess) {
    return '当前为 debug 模式，已绕过 release 环境下的服务端激活限制。'
  }

  if (isTrialAccess) {
    const expiresAt = formatTimestamp(trialExpiresAt)
    const remaining = trialDaysRemaining == null ? '' : `，剩余 ${trialDaysRemaining} 天`
    return expiresAt
      ? `当前为 7 天试用期，付费功能暂可用${remaining}。试用截止：${expiresAt}`
      : `当前为 7 天试用期，付费功能暂可用${remaining}。`
  }

  if (!hasLocalLicense) {
    return '当前未完成服务端激活。release 环境下付费功能不可用。'
  }

  if (featureAccessStatus.ready) {
    const expiresAt = formatTimestamp(featureAccessStatus.expiresAt)
    return expiresAt
      ? `服务端授权已就绪，过期时间：${expiresAt}`
      : '服务端授权已就绪'
  }

  return `服务端授权不可用：${getFeatureAccessIssueMessage(featureAccessStatus)}。付费功能仍会受限。`
}

const resolveFeatureAccessSummary = ({
  hasLocalLicense,
  isDebugAccess,
  isTrialAccess,
  trialExpiresAt,
  trialDaysRemaining,
  featureAccessStatus,
  formatTimestamp,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  isTrialAccess: boolean
  trialExpiresAt: number | null
  trialDaysRemaining: number | null
  featureAccessStatus: FeatureAccessStatus
  formatTimestamp: (timestamp: number | null) => string
}) => {
  if (isDebugAccess) {
    return 'release 授权链路调试区。开发态不会因为服务端授权缺失而被拦截。'
  }

  if (isTrialAccess) {
    const expiresAt = formatTimestamp(trialExpiresAt)
    const remaining = trialDaysRemaining == null ? '' : `剩余 ${trialDaysRemaining} 天。`
    return expiresAt
      ? `试用授权有效，${remaining}试用截止：${expiresAt}`
      : `试用授权有效。${remaining}`
  }

  if (!hasLocalLicense) {
    return '请先输入用户名和卡密完成激活。激活成功后会开放全部功能。'
  }

  if (featureAccessStatus.ready) {
    const expiresAt = formatTimestamp(featureAccessStatus.expiresAt)
    return expiresAt
      ? `服务端授权有效，过期时间：${expiresAt}`
      : '服务端授权有效。'
  }

  if (featureAccessStatus.exists) {
    return `当前服务端授权不可用：${getFeatureAccessIssueMessage(featureAccessStatus)}`
  }

  return '当前设备尚未保存有效的服务端授权令牌。'
}

const resolveRefreshRuntimeText = ({
  isDebugAccess,
  isTrialAccess,
  featureAccessStatus,
  refreshRuntime,
  refreshCooldownSeconds,
  formatTimestamp,
  formatDuration,
}: {
  isDebugAccess: boolean
  isTrialAccess: boolean
  featureAccessStatus: FeatureAccessStatus
  refreshRuntime: LicenseActivationRefreshRuntime
  refreshCooldownSeconds: number
  formatTimestamp: (timestamp: number | null) => string
  formatDuration: (seconds: number) => string
}) => {
  if (isDebugAccess) {
    return 'debug 模式不会要求服务端授权。需要验证 release 限制时，可在管理员面板手工写入或从服务端刷新。'
  }

  if (isTrialAccess) {
    return '试用期内不会自动续期。试用结束后输入管理员分配的用户名和卡密完成正式激活。'
  }

  if (featureAccessStatus.ready) {
    if (refreshRuntime.last_success_at) {
      return `最近一次自动续期成功时间：${formatTimestamp(refreshRuntime.last_success_at)}`
    }

    return '当前服务端授权有效。'
  }

  if (refreshRuntime.next_retry_at) {
    return `自动刷新冷却中，下次重试${formatDuration(refreshCooldownSeconds)}。`
  }

  if (refreshRuntime.last_error) {
    return `最近一次自动刷新失败：${refreshRuntime.last_error}`
  }

  return '当前还没有自动续期记录。'
}

const resolveRefreshServiceHint = ({
  isDebugAccess,
  isTrialAccess,
  refreshServiceConfigured,
}: {
  isDebugAccess: boolean
  isTrialAccess: boolean
  refreshServiceConfigured: boolean
}) => {
  if (isDebugAccess) {
    return 'release 授权服务由卡密平台统一维护。'
  }

  if (isTrialAccess) {
    return '试用期结束后，卡密平台会绑定当前设备并开放正式授权。'
  }

  if (refreshServiceConfigured) {
    return '输入管理员提供的用户名和卡密即可激活当前设备。'
  }

  return '当前授权服务不可用。请联系管理员确认卡密平台状态。'
}
