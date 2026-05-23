import { invoke } from '@tauri-apps/api/core'

export interface EntitlementRefreshConfig {
  enabled: boolean
  endpoint: string
  api_key: string
  customer_id: string
  timeout_secs: number
  api_key_configured: boolean
}

export interface EntitlementRefreshResult {
  success: boolean
  configured: boolean
  message: string
  error_code: string | null
  retry_after_secs: number | null
  token_stored: boolean
  status: {
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
}

export const getEntitlementRefreshConfig = async () =>
  invoke<EntitlementRefreshConfig>('get_entitlement_refresh_config')

export const saveEntitlementRefreshConfig = async (config: {
  enabled: boolean
  endpoint: string
  api_key: string
  customer_id: string
  timeout_secs?: number
}) => invoke<EntitlementRefreshConfig>('save_entitlement_refresh_config', { config })

export const refreshEntitlementTokenFromServer = async () =>
  invoke<EntitlementRefreshResult>('refresh_entitlement_token')

export const activateWithLicenseCard = async (input: {
  username: string
  activation_key: string
}) => invoke<EntitlementRefreshResult>('activate_with_license_card', { input })
