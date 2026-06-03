import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SearchPluginItem } from '@/services/searchEntryBuilders'

const plugins = ref<SearchPluginItem[]>([])
const initialized = ref(false)
const isLoading = ref(false)

export async function refreshSearchPlugins() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const response: any = await invoke('get_plugins_paginated', {
      page: 1,
      pageSize: 24,
      statusFilter: null,
      searchText: null,
      userId: null,
    })

    plugins.value = Array.isArray(response?.data?.data)
      ? response.data.data.filter((item: any) => item?.plugin_id)
      : []
  } catch (error) {
    console.error('[useSearchPlugins] Failed to load plugins:', error)
    plugins.value = []
  } finally {
    isLoading.value = false
  }
}

export async function initializeSearchPlugins() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSearchPlugins()
}

export function useSearchPlugins() {
  return {
    plugins: computed(() => plugins.value),
    isLoading,
    initializeSearchPlugins,
    refreshSearchPlugins,
  }
}
