import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SearchScanTaskItem } from '@/services/searchEntryBuilders'

const scanTasks = ref<SearchScanTaskItem[]>([])
const initialized = ref(false)
const isLoading = ref(false)

export async function refreshSearchScanTasks() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const response = await invoke<any[]>('get_scan_tasks')
    scanTasks.value = Array.isArray(response)
      ? response.filter(item => item?.id)
      : []
  } catch (error) {
    console.error('[useSearchScanTasks] Failed to load scan tasks:', error)
    scanTasks.value = []
  } finally {
    isLoading.value = false
  }
}

export async function initializeSearchScanTasks() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSearchScanTasks()
}

export function useSearchScanTasks() {
  return {
    scanTasks: computed(() => scanTasks.value),
    isLoading,
    initializeSearchScanTasks,
    refreshSearchScanTasks,
  }
}
