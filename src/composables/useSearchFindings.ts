import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Finding } from '@/components/SecurityCenter/vulnerabilityFindingTypes'

const SEARCH_FINDING_LIMIT = 30

const findings = ref<Finding[]>([])
const initialized = ref(false)
const isLoading = ref(false)
const unlisteners: UnlistenFn[] = []

async function refreshSearchFindings() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const response = await invoke<any>('list_findings', {
      limit: SEARCH_FINDING_LIMIT,
      offset: 0,
      severityFilter: null,
    })

    if (response?.success && Array.isArray(response.data)) {
      findings.value = response.data
        .map((item: any) => ({
          ...item,
          evidence: item.evidence || [],
        }))
        .filter((item: Finding) => Boolean(item?.id && item?.title))
      return
    }

    findings.value = []
  } catch (error) {
    console.error('[useSearchFindings] Failed to load findings:', error)
    findings.value = []
  } finally {
    isLoading.value = false
  }
}

async function initializeSearchFindings() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSearchFindings()

  unlisteners.push(
    await listen('scan:finding', () => {
      void refreshSearchFindings()
    }),
  )

  unlisteners.push(
    await listen('system-agent:verification-complete', () => {
      void refreshSearchFindings()
    }),
  )
}

const topFindings = computed(() => findings.value)

export function useSearchFindings() {
  return {
    findings: topFindings,
    isLoading,
    initializeSearchFindings,
    refreshSearchFindings,
  }
}
