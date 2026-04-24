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

  const dialogTitle = hasLocalLicense ? '检查授权状态' : '完成升级'
  const dialogSubtitle = !hasLocalLicense
    ? '输入许可证密钥以完成当前设备的升级。'
    : !featureAccessStatus.ready
      ? '本地授权已完成，系统会继续自动补齐高级功能权限。'
      : '当前授权已就绪，可在这里查看高级功能同步状态。'
  const upgradeEntryLabel = !hasLocalLicense || !featureAccessStatus.ready
    ? '完成升级'
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
    return '当前为 debug 模式，已绕过 release 环境下的本地授权与高级功能权限限制。'
  }

  if (!hasLocalLicense) {
    return '当前未激活本地 license。release 环境下高级功能不可用。'
  }

  if (featureAccessStatus.ready) {
    const expiresAt = formatTimestamp(featureAccessStatus.expiresAt)
    return expiresAt
      ? `高级功能权限已就绪，过期时间：${expiresAt}`
      : '高级功能权限已就绪'
  }

  return `已激活本地 license，但${getFeatureAccessIssueMessage(featureAccessStatus)}。高价值功能仍会受限。`
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
    return 'release 授权链路调试区。开发态不会因为高级功能权限缺失而被拦截。'
  }

  if (!hasLocalLicense) {
    return '先完成许可证密钥激活，本地授权建立后才需要处理高级功能同步。'
  }

  if (featureAccessStatus.ready) {
    const expiresAt = formatTimestamp(featureAccessStatus.expiresAt)
    return expiresAt
      ? `高级功能同步已完成，权限过期时间：${expiresAt}`
      : '高级功能同步已完成。'
  }

  if (featureAccessStatus.exists) {
    return `本地 license 已激活，但当前高级功能权限不可用：${getFeatureAccessIssueMessage(featureAccessStatus)}`
  }

  return '本地 license 已激活。只有漏洞赏金、插件目录写入等高价值功能，才需要继续补齐高级功能权限。'
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
    return 'debug 模式不会要求高级功能权限。需要验证 release 限制时，可在管理员面板手工写入或从服务端刷新。'
  }

  if (featureAccessStatus.ready) {
    if (refreshRuntime.last_success_at) {
      return `最近一次自动同步成功时间：${formatTimestamp(refreshRuntime.last_success_at)}`
    }

    return '当前高级功能权限有效。'
  }

  if (refreshRuntime.next_retry_at) {
    return `自动刷新冷却中，下次重试${formatDuration(refreshCooldownSeconds)}。`
  }

  if (refreshRuntime.last_error) {
    return `最近一次自动刷新失败：${refreshRuntime.last_error}`
  }

  return '当前还没有自动刷新记录。'
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
    return '服务端自动同步已由管理员配置，可直接触发一次同步或等待后台自动刷新。'
  }

  return '服务端自动同步尚未配置。请让管理员在 设置 > 安全 > 高级功能权限同步管理 中配置刷新服务。'
}
