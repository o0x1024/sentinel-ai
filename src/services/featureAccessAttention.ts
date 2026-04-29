import type { EntitlementAutoRefreshOutcome } from './entitlementAutoRefresh'
import { getFeatureAccessIssueMessage } from './featureAccessMessaging'
import {
  isFeatureAccessExpiringSoon,
  type FeatureAccessStatus,
} from './featureAccessStatus'

interface FeatureAccessReminderInput {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  hasShownReminder: boolean
  featureAccessStatus: FeatureAccessStatus
  cooldownSeconds: number
  expiringSoonThresholdSeconds?: number
}

interface FeatureAccessAutoRefreshFailureInput {
  outcome: EntitlementAutoRefreshOutcome
  consecutiveFailures: number
  cooldownSeconds: number
}

export interface FeatureAccessAttentionMessage {
  level: 'warning' | 'info'
  message: string
}

export const getFeatureAccessReminderMessage = ({
  hasLocalLicense,
  isDebugAccess,
  hasShownReminder,
  featureAccessStatus,
  cooldownSeconds,
  expiringSoonThresholdSeconds,
}: FeatureAccessReminderInput): FeatureAccessAttentionMessage | null => {
  if (!hasLocalLicense || isDebugAccess || hasShownReminder) {
    return null
  }

  if (!featureAccessStatus.ready) {
    const suffix =
      cooldownSeconds > 0 ? `；自动重试将在${formatCooldownLabel(cooldownSeconds)}` : ''
    return {
      level: 'warning',
      message: `当前服务端授权不可用：${getFeatureAccessIssueMessage(featureAccessStatus)}，付费功能仍受限${suffix}。`,
    }
  }

  if (isFeatureAccessExpiringSoon(featureAccessStatus, expiringSoonThresholdSeconds)) {
    return {
      level: 'info',
      message: '服务端授权即将过期，建议尽快完成续期。',
    }
  }

  return null
}

export const getFeatureAccessAutoRefreshFailureMessage = ({
  outcome,
  consecutiveFailures,
  cooldownSeconds,
}: FeatureAccessAutoRefreshFailureInput) => {
  if (
    !(
      outcome.status === 'failure' &&
      outcome.configured &&
      (consecutiveFailures <= 1 || outcome.errorCode === 'customer_revoked')
    )
  ) {
    return null
  }

  const suffix =
    cooldownSeconds > 0 ? `；下次自动重试${formatCooldownLabel(cooldownSeconds)}` : ''
  return `服务端授权自动刷新失败：${outcome.message}${suffix}`
}

export const shouldResetFeatureAccessReminder = ({
  hasLocalLicense,
  isDebugAccess,
  featureAccessReady,
}: {
  hasLocalLicense: boolean
  isDebugAccess: boolean
  featureAccessReady: boolean
}) => !hasLocalLicense || isDebugAccess || featureAccessReady

const formatCooldownLabel = (seconds: number) => {
  if (seconds <= 0) {
    return '现在'
  }

  if (seconds < 60) {
    return `${seconds} 秒后`
  }

  if (seconds < 3600) {
    return `${Math.ceil(seconds / 60)} 分钟后`
  }

  return `${Math.ceil(seconds / 3600)} 小时后`
}
