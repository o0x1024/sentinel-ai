import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SearchKnowledgeDocumentItem } from '@/services/searchEntryBuilders'

const knowledgeDocuments = ref<SearchKnowledgeDocumentItem[]>([])
const initialized = ref(false)
const isLoading = ref(false)

async function loadCollectionDocuments(collectionId: string) {
  const response = await invoke<any>('list_rag_documents_paginated', {
    collectionId,
    page: 1,
    pageSize: 6,
    searchQuery: null,
  })

  const documents = Array.isArray(response?.documents) ? response.documents : []
  return documents
    .filter((item: any) => item?.id)
    .map((item: any) => ({
      ...item,
      collection_id: collectionId,
      collection_name: item.collection_name || item.collectionName,
    })) as SearchKnowledgeDocumentItem[]
}

export async function refreshSearchKnowledgeDocuments() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const status = await invoke<any>('get_rag_status')
    const collections = Array.isArray(status?.collections) ? status.collections : []
    const activeCollections = collections
      .filter((collection: any) => collection?.id)
      .slice(0, 4)

    const documentGroups = await Promise.all(activeCollections.map((collection: any) =>
      loadCollectionDocuments(collection.id)
        .then(items => items.map(item => ({
          ...item,
          collection_name: collection.name || item.collection_name,
        })))
        .catch((error) => {
          console.warn('[useSearchKnowledgeDocuments] Failed to load collection documents:', collection?.id, error)
          return []
        }),
    ))

    knowledgeDocuments.value = documentGroups.flat()
  } catch (error) {
    console.error('[useSearchKnowledgeDocuments] Failed to load knowledge documents:', error)
    knowledgeDocuments.value = []
  } finally {
    isLoading.value = false
  }
}

export async function initializeSearchKnowledgeDocuments() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSearchKnowledgeDocuments()
}

export function useSearchKnowledgeDocuments() {
  return {
    knowledgeDocuments: computed(() => knowledgeDocuments.value),
    isLoading,
    initializeSearchKnowledgeDocuments,
    refreshSearchKnowledgeDocuments,
  }
}
