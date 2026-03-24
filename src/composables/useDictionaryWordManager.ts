import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { dialog } from '@/composables/useDialog'
import {
  getRuleMatcherCount,
  getRuleSeverity,
  isRuleEnabled,
  parseRuleMetadata,
} from '@/components/Dictionary/ruleEntryUtils'

export interface ManagedDictionary {
  id: string
  name: string
  dict_type: string
}

export interface DictionaryWord {
  id: string
  dictionary_id: string
  word: string
  weight?: number | null
  category?: string | null
  metadata?: string | null
  created_at: string
}

export interface RuleEntryValue {
  id?: string
  word: string
  category?: string | null
  weight?: number | null
  metadata?: Record<string, any> | null
}

export interface BatchRuleEditPayload {
  categoryMode: 'keep' | 'set' | 'clear'
  category: string
  severityMode: 'keep' | 'set' | 'clear'
  severity: string
  addTags: string[]
  removeTags: string[]
  safeModeAction: 'keep' | 'enable' | 'disable'
  enabledAction: 'keep' | 'enable' | 'disable'
  matcherMode: 'keep' | 'append'
  appendMatcher?: {
    part: string
    type: string
    key?: string
    value?: string | number | Array<string | number>
  }
}

interface UseDictionaryWordManagerOptions {
  onDictionaryChanged: () => Promise<void>
  confirmClearMessage: string
}

const ROW_HEIGHT = 40
const LIST_HEIGHT = 384
const structuredDictionaryTypes = new Set(['sensitive_file', 'fingerprint_rule', 'poc_rule'])

export function useDictionaryWordManager(options: UseDictionaryWordManagerOptions) {
  const managingDictionary = ref<ManagedDictionary | null>(null)
  const showRuleEditor = ref(false)
  const showBatchRuleEditor = ref(false)
  const editingRuleEntry = ref<RuleEntryValue | null>(null)
  const showImportModal = ref(false)
  const importing = ref(false)
  const newWord = ref('')
  const searchQuery = ref('')
  const debouncedSearch = ref('')
  const virtualListRef = ref<{ scrollToTop?: () => void } | null>(null)
  const selectedWords = ref<string[]>([])
  const importText = ref('')
  const importMethod = ref('text')
  const selectedFile = ref<File | null>(null)
  const mergeMode = ref('append')
  const dictionaryWords = ref<DictionaryWord[]>([])
  const ruleCategoryFilter = ref('')
  const ruleSeverityFilter = ref('')
  const ruleMatcherFilter = ref('')
  const ruleEnabledFilter = ref('')
  const batchSize = ref(500)
  const isLoadingMore = ref(false)
  const hasMore = ref(true)

  const structuredCategoryOptions = computed(() => {
    const values = new Set<string>()
    for (const word of dictionaryWords.value) {
      if (word.category) values.add(word.category)
    }
    return Array.from(values).sort()
  })

  const structuredSeverityOptions = computed(() => {
    const values = new Set<string>()
    for (const word of dictionaryWords.value) {
      const severity = getRuleSeverity(word)
      if (severity) values.add(severity)
    }
    return ['critical', 'high', 'medium', 'low', 'info'].filter(value => values.has(value))
  })

  const isStructuredManagingDictionary = computed(() =>
    managingDictionary.value ? structuredDictionaryTypes.has(managingDictionary.value.dict_type) : false
  )

  const listItems = computed(() => {
    if (!isStructuredManagingDictionary.value) return dictionaryWords.value

    return dictionaryWords.value.filter(word => {
      if (ruleCategoryFilter.value && word.category !== ruleCategoryFilter.value) return false
      const severity = getRuleSeverity(word)
      if (ruleSeverityFilter.value && severity !== ruleSeverityFilter.value) return false
      const matcherCount = getRuleMatcherCount(word)
      if (ruleMatcherFilter.value === 'with_matchers' && matcherCount === 0) return false
      if (ruleMatcherFilter.value === 'without_matchers' && matcherCount > 0) return false
      const enabled = isRuleEnabled(word)
      if (ruleEnabledFilter.value === 'enabled' && !enabled) return false
      if (ruleEnabledFilter.value === 'disabled' && enabled) return false
      return true
    })
  })

  const currentRowHeight = computed(() => (isStructuredManagingDictionary.value ? 88 : ROW_HEIGHT))
  const selectedStructuredEntries = computed(() =>
    dictionaryWords.value.filter(word => selectedWords.value.includes(word.id))
  )

  function resetRuleFilters() {
    ruleCategoryFilter.value = ''
    ruleSeverityFilter.value = ''
    ruleMatcherFilter.value = ''
    ruleEnabledFilter.value = ''
  }

  async function manageDictionaryWords(dictionary: ManagedDictionary) {
    managingDictionary.value = dictionary
    resetRuleFilters()
    await resetAndLoadFirstPage()
  }

  async function viewDictionaryWords(dictionary: ManagedDictionary) {
    managingDictionary.value = dictionary
    resetRuleFilters()
    await resetAndLoadFirstPage()
  }

  async function resetAndLoadFirstPage() {
    if (!managingDictionary.value) return
    dictionaryWords.value = []
    selectedWords.value = []
    hasMore.value = true
    await loadMore()
  }

  async function loadMore() {
    if (!managingDictionary.value || isLoadingMore.value || !hasMore.value) return
    try {
      isLoadingMore.value = true
      const offset = dictionaryWords.value.length
      const limit = batchSize.value
      const pattern = debouncedSearch.value.trim() || null
      const result = await invoke('get_dictionary_words_paged', {
        dictionary_id: managingDictionary.value.id,
        offset,
        limit,
        pattern,
      }) as DictionaryWord[]

      const chunk = result || []
      if (chunk.length > 0) {
        dictionaryWords.value = dictionaryWords.value.concat(chunk)
      }
      hasMore.value = chunk.length === limit
    } catch (error) {
      console.error('Failed to load more dictionary words:', error)
    } finally {
      isLoadingMore.value = false
    }
  }

  function handleInfiniteScroll({ scrollTop, clientHeight, scrollHeight }: { scrollTop: number; clientHeight: number; scrollHeight: number }) {
    const threshold = 2 * ROW_HEIGHT
    if (scrollTop + clientHeight >= scrollHeight - threshold) {
      loadMore()
    }
  }

  async function addWord() {
    if (!newWord.value.trim() || !managingDictionary.value) return

    try {
      await invoke('add_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: [newWord.value.trim()],
      })
      newWord.value = ''
      await resetAndLoadFirstPage()
      await options.onDictionaryChanged()
    } catch (error) {
      console.error('Failed to add word:', error)
    }
  }

  function openRuleEditor(word?: DictionaryWord) {
    editingRuleEntry.value = word
      ? {
          id: word.id,
          word: word.word,
          category: word.category || null,
          weight: word.weight ?? 1,
          metadata: parseRuleMetadata(word),
        }
      : null
    showRuleEditor.value = true
  }

  function closeRuleEditor() {
    showRuleEditor.value = false
    editingRuleEntry.value = null
  }

  function closeBatchRuleEditor() {
    showBatchRuleEditor.value = false
  }

  async function saveRuleEntry(entry: RuleEntryValue) {
    if (!managingDictionary.value) return

    try {
      if (entry.id) {
        await invoke('update_dictionary_word', {
          word: {
            id: entry.id,
            dictionary_id: managingDictionary.value.id,
            word: entry.word,
            weight: entry.weight ?? 1,
            category: entry.category || null,
            metadata: entry.metadata ? JSON.stringify(entry.metadata) : null,
            created_at: dictionaryWords.value.find(item => item.id === entry.id)?.created_at || new Date().toISOString(),
          },
        })
      } else {
        await invoke('add_dictionary_entries', {
          dictionary_id: managingDictionary.value.id,
          entries: [{
            word: entry.word,
            weight: entry.weight ?? 1,
            category: entry.category || null,
            metadata: entry.metadata ?? null,
          }],
        })
      }

      closeRuleEditor()
      await resetAndLoadFirstPage()
      await options.onDictionaryChanged()
    } catch (error) {
      console.error('Failed to save rule entry:', error)
    }
  }

  async function removeWord(wordId: string) {
    try {
      const wordToRemove = dictionaryWords.value.find(w => w.id === wordId)
      if (wordToRemove && managingDictionary.value) {
        await invoke('remove_dictionary_words', {
          dictionary_id: managingDictionary.value.id,
          words: [wordToRemove.word],
        })
        await resetAndLoadFirstPage()
        await options.onDictionaryChanged()
      }
    } catch (error) {
      console.error('Failed to remove word:', error)
    }
  }

  async function removeSelectedWords() {
    if (selectedWords.value.length === 0) return

    try {
      const wordsToRemove = dictionaryWords.value
        .filter(w => selectedWords.value.includes(w.id))
        .map(w => w.word)

      if (wordsToRemove.length > 0 && managingDictionary.value) {
        await invoke('remove_dictionary_words', {
          dictionary_id: managingDictionary.value.id,
          words: wordsToRemove,
        })
        selectedWords.value = []
        await resetAndLoadFirstPage()
        await options.onDictionaryChanged()
      }
    } catch (error) {
      console.error('Failed to remove selected words:', error)
    }
  }

  async function applyBatchRuleEdit(payload: BatchRuleEditPayload) {
    if (!managingDictionary.value || selectedStructuredEntries.value.length === 0) return

    try {
      const updatedWords = selectedStructuredEntries.value.map(word => {
        const metadata = { ...parseRuleMetadata(word) }
        let nextCategory = word.category || null

        if (payload.categoryMode === 'set') {
          nextCategory = payload.category
        } else if (payload.categoryMode === 'clear') {
          nextCategory = null
        }

        if (payload.severityMode === 'set') {
          metadata.severity = payload.severity
        } else if (payload.severityMode === 'clear') {
          delete metadata.severity
        }

        if (payload.addTags.length > 0 || payload.removeTags.length > 0) {
          const currentTags = Array.isArray(metadata.tags)
            ? metadata.tags.filter((tag: unknown): tag is string => typeof tag === 'string')
            : []
          const mergedTags = Array.from(new Set([...currentTags, ...payload.addTags]))
            .filter(tag => !payload.removeTags.includes(tag))
          if (mergedTags.length > 0) {
            metadata.tags = mergedTags
          } else {
            delete metadata.tags
          }
        }

        if (payload.safeModeAction === 'enable') {
          metadata.safe_mode = true
        } else if (payload.safeModeAction === 'disable') {
          metadata.safe_mode = false
        }

        if (payload.enabledAction === 'enable') {
          metadata.enabled = true
        } else if (payload.enabledAction === 'disable') {
          metadata.enabled = false
        }

        if (payload.matcherMode === 'append' && payload.appendMatcher) {
          const currentMatchers = Array.isArray(metadata.matchers) ? metadata.matchers : []
          metadata.matchers = [...currentMatchers, payload.appendMatcher]
        }

        return {
          ...word,
          category: nextCategory,
          metadata: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : null,
        }
      })

      await invoke('update_dictionary_words_batch', {
        words: updatedWords,
      })

      showBatchRuleEditor.value = false
      selectedWords.value = []
      await resetAndLoadFirstPage()
      await options.onDictionaryChanged()
    } catch (error) {
      console.error('Failed to batch update rule entries:', error)
    }
  }

  async function exportSelectedRules() {
    if (!managingDictionary.value || selectedStructuredEntries.value.length === 0) return

    try {
      const fileName = `${managingDictionary.value.name.replace(/[^a-zA-Z0-9]/g, '_')}_selected_rules.json`
      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      })

      if (!filePath) return

      const payload = selectedStructuredEntries.value.map(word => ({
        word: word.word,
        weight: word.weight ?? null,
        category: word.category ?? null,
        metadata: parseRuleMetadata(word),
      }))

      await writeTextFile(filePath, JSON.stringify(payload, null, 2))
    } catch (error) {
      console.error('Failed to export selected rules:', error)
    }
  }

  async function clearDictionary() {
    const confirmed = await dialog.confirm(options.confirmClearMessage)
    if (!confirmed || !managingDictionary.value) return

    try {
      await invoke('clear_dictionary', { dictionary_id: managingDictionary.value.id })
      await resetAndLoadFirstPage()
      await options.onDictionaryChanged()
    } catch (error) {
      console.error('Failed to clear dictionary:', error)
    }
  }

  async function importWords() {
    importing.value = true
    try {
      let words: string[] = []
      let entries: any[] = []

      if (importMethod.value === 'text' && importText.value.trim()) {
        words = importText.value.split('\n')
          .map(word => word.trim())
          .filter(word => word.length > 0)
      } else if (importMethod.value === 'file' && selectedFile.value) {
        const content = await selectedFile.value.text()
        if (selectedFile.value.name.endsWith('.json')) {
          const data = JSON.parse(content)
          const payload = Array.isArray(data) ? data : data.words || []
          if (Array.isArray(payload) && payload.every(item => typeof item === 'string')) {
            words = payload
          } else if (Array.isArray(payload)) {
            entries = payload
              .filter(item => item && typeof item === 'object' && typeof item.word === 'string')
              .map(item => ({
                word: item.word,
                weight: typeof item.weight === 'number' ? item.weight : null,
                category: typeof item.category === 'string' ? item.category : null,
                metadata: item.metadata ?? null,
              }))
          }
        } else {
          words = content.split('\n')
            .map(word => word.trim())
            .filter(word => word.length > 0)
        }
      }

      if ((words.length > 0 || entries.length > 0) && managingDictionary.value) {
        if (entries.length > 0) {
          await invoke('add_dictionary_entries', {
            dictionary_id: managingDictionary.value.id,
            entries,
          })
        } else {
          await invoke('add_dictionary_words', {
            dictionary_id: managingDictionary.value.id,
            words,
          })
        }

        importText.value = ''
        selectedFile.value = null
        showImportModal.value = false
        await resetAndLoadFirstPage()
        await options.onDictionaryChanged()
      }
    } catch (error) {
      console.error('Failed to import words:', error)
    } finally {
      importing.value = false
    }
  }

  function handleFileSelect(event: Event) {
    const target = event.target as HTMLInputElement
    selectedFile.value = target.files?.[0] || null
  }

  function toggleSelectAll() {
    if (selectedWords.value.length === listItems.value.length) {
      selectedWords.value = []
    } else {
      selectedWords.value = listItems.value.map(word => word.id)
    }
  }

  function closeDictionaryWordsModal() {
    managingDictionary.value = null
    closeRuleEditor()
    closeBatchRuleEditor()
    dictionaryWords.value = []
    resetRuleFilters()
    selectedWords.value = []
    searchQuery.value = ''
    debouncedSearch.value = ''
    hasMore.value = true
  }

  let searchDebounceTimer: number | null = null
  watch(searchQuery, val => {
    if (searchDebounceTimer) {
      clearTimeout(searchDebounceTimer)
    }
    searchDebounceTimer = window.setTimeout(() => {
      debouncedSearch.value = val
      virtualListRef.value?.scrollToTop?.()
      if (managingDictionary.value) {
        hasMore.value = true
        resetAndLoadFirstPage()
      }
    }, 200)
  })

  return {
    LIST_HEIGHT,
    addWord,
    applyBatchRuleEdit,
    clearDictionary,
    closeBatchRuleEditor,
    closeDictionaryWordsModal,
    closeRuleEditor,
    currentRowHeight,
    debouncedSearch,
    dictionaryWords,
    editingRuleEntry,
    exportSelectedRules,
    handleFileSelect,
    handleInfiniteScroll,
    hasMore,
    importMethod,
    importText,
    importWords,
    importing,
    isLoadingMore,
    isStructuredManagingDictionary,
    listItems,
    manageDictionaryWords,
    managingDictionary,
    mergeMode,
    newWord,
    openRuleEditor,
    removeSelectedWords,
    removeWord,
    resetAndLoadFirstPage,
    ruleCategoryFilter,
    ruleEnabledFilter,
    ruleMatcherFilter,
    ruleSeverityFilter,
    searchQuery,
    selectedFile,
    selectedWords,
    showBatchRuleEditor,
    showImportModal,
    showRuleEditor,
    structuredCategoryOptions,
    structuredSeverityOptions,
    toggleSelectAll,
    viewDictionaryWords,
    virtualListRef,
    saveRuleEntry,
  }
}
