import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ShellPermissionRulePreview {
  rule: string
  reason_key: string
}

export interface ShellPermissionHistoryEntry {
  timestamp: string
  session_id: string
  execution_id?: string | null
  command: string
  decision: string
  allowed: boolean
  semantic_kind: string
  semantic_code: string
  semantic_summary_key: string
  semantic_reason_key?: string | null
  suggested_allow_rules: ShellPermissionRulePreview[]
  persisted_allow_rules: string[]
}

const normalizeHistoryEntry = (entry: ShellPermissionHistoryEntry): ShellPermissionHistoryEntry => {
  return {
    ...entry,
    suggested_allow_rules: Array.isArray(entry.suggested_allow_rules)
      ? entry.suggested_allow_rules
      : [],
    persisted_allow_rules: Array.isArray(entry.persisted_allow_rules)
      ? entry.persisted_allow_rules
      : [],
  }
}

const PAGE_SIZE = 20
const STORAGE_KEY = 'sentinel:settings:shell-permission-history-panel:v1'

export function useShellPermissionHistoryPanel() {
  const loading = ref(false)
  const history = ref<ShellPermissionHistoryEntry[]>([])
  const expandedEntries = ref<Record<string, boolean>>({})
  const currentLimit = ref(PAGE_SIZE)
  const hasMoreHistory = ref(false)
  const executionIdFilter = ref('')
  const commandSearchFilter = ref('')
  const selectedDate = ref('')
  const recentDays = ref(7)
  const decisionFilter = ref('all')
  const semanticKindFilter = ref('all')

  const entryKey = (entry: ShellPermissionHistoryEntry) =>
    `${entry.timestamp}-${entry.session_id}-${entry.command}`

  const isExpanded = (entry: ShellPermissionHistoryEntry) => {
    return !!expandedEntries.value[entryKey(entry)]
  }

  const toggleExpanded = (entry: ShellPermissionHistoryEntry) => {
    const key = entryKey(entry)
    expandedEntries.value[key] = !expandedEntries.value[key]
  }

  const visibleHistory = computed(() => {
    const commandQuery = commandSearchFilter.value.trim().toLowerCase()
    return history.value.filter((entry) => {
      const decisionMatches =
        decisionFilter.value === 'all' || entry.decision === decisionFilter.value
      const semanticMatches =
        semanticKindFilter.value === 'all' || entry.semantic_kind === semanticKindFilter.value
      const commandMatches =
        !commandQuery || entry.command.toLowerCase().includes(commandQuery)
      return decisionMatches && semanticMatches && commandMatches
    })
  })

  const persistPanelState = () => {
    if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
      return
    }

    const payload = {
      executionIdFilter: executionIdFilter.value,
      commandSearchFilter: commandSearchFilter.value,
      selectedDate: selectedDate.value,
      recentDays: recentDays.value,
      decisionFilter: decisionFilter.value,
      semanticKindFilter: semanticKindFilter.value,
      currentLimit: currentLimit.value,
      expandedEntries: expandedEntries.value,
    }

    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload))
    } catch (error) {
      console.warn('Failed to persist shell permission history panel state:', error)
    }
  }

  const restorePanelState = () => {
    if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
      return
    }

    try {
      const raw = window.localStorage.getItem(STORAGE_KEY)
      if (!raw) {
        return
      }

      const parsed = JSON.parse(raw) as Partial<{
        executionIdFilter: string
        commandSearchFilter: string
        selectedDate: string
        recentDays: number
        decisionFilter: string
        semanticKindFilter: string
        currentLimit: number
        expandedEntries: Record<string, boolean>
      }>

      executionIdFilter.value =
        typeof parsed.executionIdFilter === 'string' ? parsed.executionIdFilter : ''
      commandSearchFilter.value =
        typeof parsed.commandSearchFilter === 'string' ? parsed.commandSearchFilter : ''
      selectedDate.value = typeof parsed.selectedDate === 'string' ? parsed.selectedDate : ''
      recentDays.value =
        typeof parsed.recentDays === 'number' && Number.isFinite(parsed.recentDays)
          ? parsed.recentDays
          : 7
      decisionFilter.value =
        typeof parsed.decisionFilter === 'string' ? parsed.decisionFilter : 'all'
      semanticKindFilter.value =
        typeof parsed.semanticKindFilter === 'string' ? parsed.semanticKindFilter : 'all'
      currentLimit.value =
        typeof parsed.currentLimit === 'number' && Number.isFinite(parsed.currentLimit)
          ? Math.max(PAGE_SIZE, parsed.currentLimit)
          : PAGE_SIZE
      expandedEntries.value =
        parsed.expandedEntries && typeof parsed.expandedEntries === 'object'
          ? parsed.expandedEntries
          : {}
    } catch (error) {
      console.warn('Failed to restore shell permission history panel state:', error)
    }
  }

  const fetchHistory = async (options?: { preserveLimit?: boolean; preserveExpanded?: boolean }) => {
    const preserveLimit = !!options?.preserveLimit
    const preserveExpanded = !!options?.preserveExpanded
    if (!preserveLimit) {
      currentLimit.value = PAGE_SIZE
    }

    const requestedLimit = currentLimit.value
    loading.value = true
    try {
      const request = selectedDate.value
        ? {
            date: selectedDate.value,
            execution_id: executionIdFilter.value || null,
            limit: requestedLimit,
          }
        : {
            days: recentDays.value,
            execution_id: executionIdFilter.value || null,
            limit: requestedLimit,
          }
      const records = await invoke<ShellPermissionHistoryEntry[]>('get_shell_permission_history', {
        request,
      })
      history.value = records.map(normalizeHistoryEntry)
      hasMoreHistory.value = history.value.length >= requestedLimit
      if (!preserveExpanded) {
        expandedEntries.value = {}
      }
      persistPanelState()
    } catch (error) {
      console.error('Failed to load shell permission history:', error)
      history.value = []
      hasMoreHistory.value = false
    } finally {
      loading.value = false
    }
  }

  const loadMoreHistory = () => {
    currentLimit.value += PAGE_SIZE
    persistPanelState()
    fetchHistory({ preserveLimit: true, preserveExpanded: true })
  }

  const toggleExpandedAndPersist = (entry: ShellPermissionHistoryEntry) => {
    toggleExpanded(entry)
    persistPanelState()
  }

  const refreshHistory = () => {
    fetchHistory({ preserveLimit: true, preserveExpanded: true })
  }

  const applyFilters = () => {
    persistPanelState()
    fetchHistory({ preserveLimit: true, preserveExpanded: true })
  }

  const resetFilters = () => {
    executionIdFilter.value = ''
    commandSearchFilter.value = ''
    selectedDate.value = ''
    recentDays.value = 7
    decisionFilter.value = 'all'
    semanticKindFilter.value = 'all'
    currentLimit.value = PAGE_SIZE
    expandedEntries.value = {}
    persistPanelState()
    refreshHistory()
  }

  watch(
    [
      executionIdFilter,
      commandSearchFilter,
      selectedDate,
      recentDays,
      decisionFilter,
      semanticKindFilter,
    ],
    () => {
      persistPanelState()
    }
  )

  onMounted(() => {
    restorePanelState()
    refreshHistory()
  })

  return {
    loading,
    history,
    visibleHistory,
    hasMoreHistory,
    executionIdFilter,
    commandSearchFilter,
    selectedDate,
    recentDays,
    decisionFilter,
    semanticKindFilter,
    entryKey,
    isExpanded,
    loadMoreHistory,
    toggleExpandedAndPersist,
    refreshHistory,
    applyFilters,
    resetFilters,
  }
}
