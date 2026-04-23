import { invoke } from '@tauri-apps/api/core'
import { readonly, ref } from 'vue'

export interface EntitlementTokenStatus {
  exists: boolean
  valid: boolean
  tier: string | null
  license_id: string | null
  feature_ids: readonly string[]
  issued_at: number | null
  expires_at: number | null
  expires_in_seconds: number | null
  machine_id_match: boolean
  error: string | null
}

export type EntitlementTokenIssueCode =
  | 'missing'
  | 'expired'
  | 'machine_mismatch'
  | 'not_yet_valid'
  | 'invalid_signature'
  | 'invalid_storage'
  | 'invalid'
  | 'ok'

const entitlementTokenStatusState = ref<EntitlementTokenStatus>({
  exists: false,
  valid: false,
  tier: null,
  license_id: null,
  feature_ids: [],
  issued_at: null,
  expires_at: null,
  expires_in_seconds: null,
  machine_id_match: false,
  error: null,
})

let statusPromise: Promise<EntitlementTokenStatus> | null = null

const applyTokenStatus = (value: EntitlementTokenStatus) => {
  entitlementTokenStatusState.value = value
  window.dispatchEvent(new CustomEvent('sentinel:entitlement-token-updated', { detail: value }))
  return value
}

const loadTokenStatus = async (force = false): Promise<EntitlementTokenStatus> => {
  if (!force && statusPromise) {
    return statusPromise
  }

  statusPromise = invoke<EntitlementTokenStatus>('get_entitlement_token_status')
    .then(applyTokenStatus)
    .catch(error => {
      console.error('Failed to load entitlement token status:', error)
      return entitlementTokenStatusState.value
    })
    .finally(() => {
      statusPromise = null
    })

  return statusPromise
}

export const getEntitlementTokenStatus = async () => loadTokenStatus(false)

export const refreshEntitlementTokenStatus = async () => loadTokenStatus(true)

export const useEntitlementTokenStatusState = () => readonly(entitlementTokenStatusState)

export const isEntitlementTokenExpiringSoon = (
  status: EntitlementTokenStatus,
  thresholdSeconds = 24 * 60 * 60,
) => {
  if (!status.valid || status.expires_in_seconds == null) {
    return false
  }

  return status.expires_in_seconds <= thresholdSeconds
}

export const getEntitlementTokenIssueCode = (
  status: EntitlementTokenStatus,
): EntitlementTokenIssueCode => {
  if (status.valid) {
    return 'ok'
  }

  if (!status.exists) {
    return 'missing'
  }

  const error = status.error?.toLowerCase() ?? ''
  if (error.includes('expired')) {
    return 'expired'
  }
  if (error.includes('machine mismatch')) {
    return 'machine_mismatch'
  }
  if (error.includes('not yet valid')) {
    return 'not_yet_valid'
  }
  if (error.includes('signature')) {
    return 'invalid_signature'
  }
  if (error.includes('decrypt')) {
    return 'invalid_storage'
  }

  return 'invalid'
}

export const getEntitlementTokenIssueMessage = (status: EntitlementTokenStatus) => {
  switch (getEntitlementTokenIssueCode(status)) {
    case 'ok':
      return 'entitlement token 有效'
    case 'missing':
      return '缺少 entitlement token'
    case 'expired':
      return 'entitlement token 已过期'
    case 'machine_mismatch':
      return 'entitlement token 与当前设备不匹配'
    case 'not_yet_valid':
      return 'entitlement token 尚未生效'
    case 'invalid_signature':
      return 'entitlement token 签名无效'
    case 'invalid_storage':
      return '本地 entitlement token 缓存损坏'
    default:
      return status.error ? `entitlement token 不可用：${status.error}` : 'entitlement token 不可用'
  }
}
