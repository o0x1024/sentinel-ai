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
    featureAccessStatus,
    refreshServiceConfigured,
    refreshRuntime,
    refreshCooldownSeconds,
    formatTimestamp,
    formatDuration,
  } = input

  const dialogTitle = hasLocalLicense ? '检查授权状态' : '服务端激活'
  const dialogSubtitle = !hasLocalLicense
    ? '连接授权服务，完成当前设备激活。'
    : !featureAccessStatus.ready
      ? '当前授权令牌不可用，系统会继续从服务端刷新。'
      : '当前授权已就绪，可在这里查看服务端授权状态。'
  const upgradeEntryLabel = !hasLocalLicense || !featureAccessStatus.ready
    ? '服务端激活'
    : '检查授权状态'

  let statusBadgeLabel = '待完成'
  let featureAccessTone = 'alert-info'

  if (isDebugAccess) {
    statusBadgeLabel = 'Debug'
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
    featureAccessStatus,
    formatTimestamp,
  })
  const featureAccessSummary = resolveFeatureAccessSummary({
    hasLocalLicense,
    isDebugAccess,
    featureAccessStatus,
    formatTimestamp,
  })
  const refreshRuntimeText = resolveRefreshRuntimeText({
    isDebugAccess,
    featureAccessStatus,
    refreshRuntime,
    refreshCooldownSeconds,
    formatTimestamp,
    formatDuration,
  })
  const refreshServiceHint = resolveRefreshServiceHint({
    isDebugAccess,
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
  featureAccessStatus,
  formatTimestamp,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  featureAccessStatus: FeatureAccessStatus
  formatTimestamp: (timestamp: number | null) => string
}) => {
  if (isDebugAccess) {
    return '当前为 debug 模式，已绕过 release 环境下的服务端激活限制。'
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
  featureAccessStatus,
  formatTimestamp,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  featureAccessStatus: FeatureAccessStatus
  formatTimestamp: (timestamp: number | null) => string
}) => {
  if (isDebugAccess) {
    return 'release 授权链路调试区。开发态不会因为服务端授权缺失而被拦截。'
  }

  if (!hasLocalLicense) {
    return '请先完成服务端激活。激活成功后会开放全部功能。'
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
  featureAccessStatus,
  refreshRuntime,
  refreshCooldownSeconds,
  formatTimestamp,
  formatDuration,
}: {
  isDebugAccess: boolean
  featureAccessStatus: FeatureAccessStatus
  refreshRuntime: LicenseActivationRefreshRuntime
  refreshCooldownSeconds: number
  formatTimestamp: (timestamp: number | null) => string
  formatDuration: (seconds: number) => string
}) => {
  if (isDebugAccess) {
    return 'debug 模式不会要求服务端授权。需要验证 release 限制时，可在管理员面板手工写入或从服务端刷新。'
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
  refreshServiceConfigured,
}: {
  isDebugAccess: boolean
  refreshServiceConfigured: boolean
}) => {
  if (isDebugAccess) {
    return 'release 授权服务配置由管理员统一维护。'
  }

  if (refreshServiceConfigured) {
    return '服务端激活已由管理员配置，可直接激活或等待后台自动刷新。'
  }

  return '服务端激活尚未配置。请让管理员在 设置 > 安全 > 高级功能权限同步管理 中配置刷新服务。'
}
