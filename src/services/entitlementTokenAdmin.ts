import { invoke } from '@tauri-apps/api/core'

export interface EntitlementTokenAdminResult {
  success: boolean
  message: string
}

export const storeEntitlementTokenFromAdmin = async (token: string) =>
  invoke<EntitlementTokenAdminResult>('store_entitlement_token', { token })

export const clearEntitlementTokenFromAdmin = async () =>
  invoke<EntitlementTokenAdminResult>('clear_entitlement_token')
