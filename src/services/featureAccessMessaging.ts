import type { FeatureAccessStatus } from './featureAccessStatus'

export const getFeatureAccessIssueMessage = (status: FeatureAccessStatus) => {
  switch (status.issueCode) {
    case 'ok':
      return '高级功能权限有效'
    case 'missing':
      return '缺少高级功能权限'
    case 'expired':
      return '高级功能权限已过期'
    case 'machine_mismatch':
      return '高级功能权限与当前设备不匹配'
    case 'not_yet_valid':
      return '高级功能权限尚未生效'
    case 'invalid_signature':
      return '高级功能权限签名无效'
    case 'invalid_storage':
      return '本地高级功能权限缓存损坏'
    default:
      return status.backendError
        ? `高级功能权限不可用：${status.backendError}`
        : '高级功能权限不可用'
  }
}
