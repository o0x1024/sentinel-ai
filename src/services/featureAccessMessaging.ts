import type { FeatureAccessStatus } from './featureAccessStatus'

export const getFeatureAccessIssueMessage = (status: FeatureAccessStatus) => {
  switch (status.issueCode) {
    case 'ok':
      return '服务端授权有效'
    case 'missing':
      return '缺少服务端授权'
    case 'expired':
      return '服务端授权已过期'
    case 'machine_mismatch':
      return '服务端授权与当前设备不匹配'
    case 'not_yet_valid':
      return '服务端授权尚未生效'
    case 'invalid_signature':
      return '服务端授权签名无效'
    case 'invalid_storage':
      return '本地服务端授权缓存损坏'
    default:
      return status.backendError
        ? `服务端授权不可用：${status.backendError}`
        : '服务端授权不可用'
  }
}
