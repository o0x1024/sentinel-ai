import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SearchBountyKnowledgeItem } from '@/services/searchEntryBuilders'
import { BOUNTY_KNOWLEDGE_UPDATED_EVENT } from '@/services/bountyKnowledgeEvents'

interface UseSearchBountyKnowledgeOptions {
  limit?: number
}

export function useSearchBountyKnowledge(
  query: Ref<string>,
  options?: UseSearchBountyKnowledgeOptions,
) {
  const knowledgeNotes = ref<SearchBountyKnowledgeItem[]>([])
  const initialized = ref(false)
  const isLoading = ref(false)
  const limit = Math.min(Math.max(options?.limit ?? 12, 1), 24)
  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  const loadKnowledgeNotes = async () => {
    if (isLoading.value) {
      return
    }

    isLoading.value = true
    try {
      const trimmedQuery = query.value.trim()
      const response = trimmedQuery
        ? await invoke<SearchBountyKnowledgeItem[]>('bounty_search_knowledge_notes', {
            request: {
              query: trimmedQuery,
              limit,
              offset: 0,
            },
          })
        : await invoke<SearchBountyKnowledgeItem[]>('bounty_list_knowledge_notes', {
            filter: {
              limit,
              offset: 0,
            },
          })

      knowledgeNotes.value = Array.isArray(response) ? response : []
    } catch (error) {
      console.error('[useSearchBountyKnowledge] Failed to load bounty knowledge notes:', error)
      knowledgeNotes.value = []
    } finally {
      isLoading.value = false
    }
  }

  const scheduleReload = () => {
    if (!initialized.value) {
      return
    }

    if (debounceTimer) {
      clearTimeout(debounceTimer)
    }

    debounceTimer = setTimeout(() => {
      void loadKnowledgeNotes()
    }, 120)
  }

  const handleKnowledgeUpdated = () => {
    scheduleReload()
  }

  onMounted(() => {
    window.addEventListener(BOUNTY_KNOWLEDGE_UPDATED_EVENT, handleKnowledgeUpdated)
  })

  onBeforeUnmount(() => {
    window.removeEventListener(BOUNTY_KNOWLEDGE_UPDATED_EVENT, handleKnowledgeUpdated)
    if (debounceTimer) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }
  })

  watch(
    () => query.value.trim(),
    () => {
      scheduleReload()
    },
  )

  const initializeSearchBountyKnowledge = async () => {
    if (initialized.value) {
      return
    }

    initialized.value = true
    await loadKnowledgeNotes()
  }

  return {
    bountyKnowledgeNotes: computed(() => knowledgeNotes.value),
    isLoading,
    initializeSearchBountyKnowledge,
    refreshSearchBountyKnowledge: loadKnowledgeNotes,
  }
}
