import { invoke } from '@tauri-apps/api/core'
import { readonly, ref } from 'vue'

export interface AppFeatureEntitlements {
  tier: 'free' | 'pro' | string
  is_licensed: boolean
  has_local_license: boolean
  access_source: string
  can_access_all_plugins: boolean
  can_access_bug_bounty: boolean
  can_access_bot_console: boolean
  can_manage_plugin_catalog: boolean
  can_add_plugins: boolean
  can_edit_plugins: boolean
  can_delete_plugins: boolean
  can_install_plugins: boolean
  can_review_plugins: boolean
  allowed_plugin_ids: string[]
}

const entitlementsState = ref<AppFeatureEntitlements>({
  tier: 'free',
  is_licensed: false,
  has_local_license: false,
  access_source: 'free',
  can_access_all_plugins: false,
  can_access_bug_bounty: false,
  can_access_bot_console: false,
  can_manage_plugin_catalog: false,
  can_add_plugins: false,
  can_edit_plugins: false,
  can_delete_plugins: false,
  can_install_plugins: false,
  can_review_plugins: false,
  allowed_plugin_ids: [],
})

let entitlementsPromise: Promise<AppFeatureEntitlements> | null = null

const applyEntitlements = (value: AppFeatureEntitlements) => {
  entitlementsState.value = value
  window.dispatchEvent(new CustomEvent('sentinel:entitlements-updated', { detail: value }))
  return value
}

const loadEntitlements = async (force = false): Promise<AppFeatureEntitlements> => {
  if (!force && entitlementsPromise) {
    return entitlementsPromise
  }

  entitlementsPromise = invoke<AppFeatureEntitlements>('get_app_entitlements')
    .then(applyEntitlements)
    .catch(error => {
      console.error('Failed to load app entitlements:', error)
      return entitlementsState.value
    })
    .finally(() => {
      entitlementsPromise = null
    })

  return entitlementsPromise
}

export const getFeatureEntitlements = async () => loadEntitlements(false)

export const refreshFeatureEntitlements = async () => loadEntitlements(true)

export const useFeatureEntitlementsState = () => readonly(entitlementsState)
