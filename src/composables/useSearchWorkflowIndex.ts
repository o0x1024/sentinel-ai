import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  SearchWorkflowDefinitionItem,
  SearchWorkflowRunItem,
} from '@/services/searchEntryBuilders'

const workflowDefinitions = ref<SearchWorkflowDefinitionItem[]>([])
const workflowRuns = ref<SearchWorkflowRunItem[]>([])
const initialized = ref(false)
const isLoading = ref(false)

export async function refreshSearchWorkflowIndex() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const [definitions, runs] = await Promise.all([
      invoke<any[]>('list_workflow_definitions', { isTemplate: false }),
      invoke<any[]>('list_workflow_runs'),
    ])

    workflowDefinitions.value = Array.isArray(definitions)
      ? definitions.filter(item => item?.id)
      : []
    workflowRuns.value = Array.isArray(runs)
      ? runs.filter(item => item?.id)
      : []
  } catch (error) {
    console.error('[useSearchWorkflowIndex] Failed to load workflows:', error)
    workflowDefinitions.value = []
    workflowRuns.value = []
  } finally {
    isLoading.value = false
  }
}

export async function initializeSearchWorkflowIndex() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSearchWorkflowIndex()
}

export function useSearchWorkflowIndex() {
  return {
    workflowDefinitions: computed(() => workflowDefinitions.value),
    workflowRuns: computed(() => workflowRuns.value),
    isLoading,
    initializeSearchWorkflowIndex,
    refreshSearchWorkflowIndex,
  }
}
