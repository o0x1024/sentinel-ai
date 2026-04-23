import { computed, ref, type Ref } from 'vue'
import type { Router } from 'vue-router'
import { useCommandPaletteActions } from '@/composables/useCommandPaletteActions'
import { useNotificationCenter } from '@/composables/useNotificationCenter'
import { useToast } from '@/composables/useToast'
import { useGlobalSearch } from '@/composables/useGlobalSearch'
import { useSearchFindings } from '@/composables/useSearchFindings'
import { useSearchBountyKnowledge } from '@/composables/useSearchBountyKnowledge'
import { useSearchKnowledgeDocuments } from '@/composables/useSearchKnowledgeDocuments'
import { useSearchPlugins } from '@/composables/useSearchPlugins'
import { useSearchScanTasks } from '@/composables/useSearchScanTasks'
import { useSearchWorkflowIndex } from '@/composables/useSearchWorkflowIndex'
import { addRecentCommand, clearRecentCommands, loadRecentCommands } from '@/services/commandHistory'
import {
  buildCommandPaletteSearchEntries,
  parseCommandPaletteQuery,
  suggestCommandPaletteQueries,
} from '@/services/commandPaletteCommands'
import { buildCommandPaletteFooterHints } from '@/services/commandPaletteFooterHints'
import { getCommandPaletteHelpItems, isCommandHelpQuery } from '@/services/commandPaletteHelp'
import { getNextCommandSuggestionIndex } from '@/services/commandSuggestionNavigation'
import {
  dedupeGlobalSearchEntries,
  searchGlobalEntries,
  type GlobalSearchEntry,
  type GlobalSearchResult,
} from '@/services/globalSearch'
import {
  collectPinnedSearchShortcutBatchTargetLabels,
  describePinnedSearchShortcutBatchTargets,
  usePinnedSearchShortcutBatchHistory,
} from '@/services/pinnedSearchShortcutBatchHistory'
import {
  addPinnedSearchShortcutTagToMany,
  clonePinnedSearchShortcutSnapshots,
  loadPinnedSearchShortcuts,
  removePinnedSearchShortcutTagFromMany,
  removePinnedSearchShortcutTagAcrossAll,
  replacePinnedSearchShortcutTagAcrossAll,
  removePinnedSearchShortcuts,
  reorderPinnedSearchShortcuts,
  resolvePinnedSearchShortcutEntries,
  togglePinnedSearchShortcut,
  updatePinnedSearchShortcutMeta,
  type PinnedSearchShortcutSnapshot,
} from '@/services/pinnedSearchShortcuts'
import { addRecentSearch, clearRecentSearches, loadRecentSearches } from '@/services/searchHistory'
import {
  collectSearchCategoryFilters,
  filterSearchResultsByCategory,
  getNextSearchCategoryFilter,
  groupSearchResults,
  type SearchCategoryFilter,
} from '@/services/searchPresentation'
import { recordSearchAnalyticsEvent, useSearchAnalytics } from '@/services/searchAnalytics'

interface UseSearchWorkspaceControllerOptions {
  router: Router
  query: Ref<string>
  includeCloseHint?: boolean
}

function setLastWorkflowId(workflowId: string) {
  if (typeof window === 'undefined') {
    return
  }

  const normalizedWorkflowId = String(workflowId || '').trim()
  if (!normalizedWorkflowId) {
    return
  }

  window.localStorage.setItem('last_run_workflow_id', normalizedWorkflowId)
}

export function useSearchWorkspaceController(options: UseSearchWorkspaceControllerOptions) {
  const toast = useToast()
  const { pushEntry: pushPinnedShortcutBatchHistoryEntry, undoEntry: undoPinnedShortcutBatchHistoryEntry } =
    usePinnedSearchShortcutBatchHistory()
  const { summary: analyticsSummary, clearEvents: clearSearchAnalytics } = useSearchAnalytics()
  const { actions } = useCommandPaletteActions(options.router)
  const { items: notificationCenterItems, initializeNotificationCenter } = useNotificationCenter()
  const { findings: searchFindings, initializeSearchFindings } = useSearchFindings()
  const { bountyKnowledgeNotes, initializeSearchBountyKnowledge } = useSearchBountyKnowledge(options.query, { limit: 16 })
  const { scanTasks, initializeSearchScanTasks } = useSearchScanTasks()
  const { workflowDefinitions, workflowRuns, initializeSearchWorkflowIndex } = useSearchWorkflowIndex()
  const { knowledgeDocuments, initializeSearchKnowledgeDocuments } = useSearchKnowledgeDocuments()
  const { plugins, initializeSearchPlugins } = useSearchPlugins()
  const { entries } = useGlobalSearch({
    notifications: notificationCenterItems,
    findings: searchFindings,
    scanTasks,
    workflowDefinitions,
    workflowRuns,
    knowledgeDocuments,
    bountyKnowledgeNotes,
    plugins,
  })

  const recentCommands = ref<string[]>(loadRecentCommands())
  const recentSearches = ref<string[]>(loadRecentSearches())
  const pinnedShortcutSnapshots = ref<PinnedSearchShortcutSnapshot[]>(loadPinnedSearchShortcuts())
  const editingPinnedShortcut = ref<GlobalSearchEntry | null>(null)
  const selectedCommandSuggestionIndex = ref(-1)
  const activeCategory = ref<SearchCategoryFilter>('all')
  const highlightedId = ref<string | null>(null)

  const trimmedSearch = computed(() => options.query.value.trim())
  const showCommandHelp = computed(() => isCommandHelpQuery(trimmedSearch.value))
  const commandHelpItems = computed(() => getCommandPaletteHelpItems(trimmedSearch.value))
  const parsedCommand = computed(() => parseCommandPaletteQuery(trimmedSearch.value))
  const commandSuggestions = computed(() =>
    showCommandHelp.value || parsedCommand.value?.status === 'resolved'
      ? []
      : suggestCommandPaletteQueries(trimmedSearch.value).slice(0, 3),
  )
  const selectedCommandSuggestion = computed(
    () => commandSuggestions.value[selectedCommandSuggestionIndex.value] || commandSuggestions.value[0] || null,
  )
  const footerHints = computed(() =>
    buildCommandPaletteFooterHints({
      hasCommandSuggestions: commandSuggestions.value.length > 0,
      showCommandHelp: showCommandHelp.value,
      includeCloseHint: options.includeCloseHint,
    }),
  )
  const actionEntriesById = computed(() => new Map(actions.value.map(action => [action.id, action])))
  const availableShortcutEntries = computed(() =>
    dedupeGlobalSearchEntries([
      ...actions.value,
      ...entries.value,
    ]),
  )
  const availableShortcutEntriesById = computed(() =>
    new Map(availableShortcutEntries.value.map(entry => [entry.id, entry])),
  )
  const pinnedShortcutEntries = computed(() =>
    resolvePinnedSearchShortcutEntries(pinnedShortcutSnapshots.value, availableShortcutEntriesById.value),
  )
  const editingPinnedShortcutSnapshot = computed(() =>
    editingPinnedShortcut.value
      ? pinnedShortcutSnapshots.value.find(item => item.id === editingPinnedShortcut.value.id) || null
      : null,
  )
  const commandActionEntries = computed(() =>
    buildCommandPaletteSearchEntries(trimmedSearch.value, actionEntriesById.value),
  )
  const searchEntries = computed(() =>
    dedupeGlobalSearchEntries([
      ...commandActionEntries.value,
      ...actions.value,
      ...entries.value,
    ]),
  )
  const featuredEntries = computed(() => searchEntries.value.filter(entry => entry.featured).slice(0, 12))
  const results = computed(() => searchGlobalEntries(searchEntries.value, trimmedSearch.value, 24))
  const groupedResults = computed(() => groupSearchResults(results.value))
  const availableCategoryFilters = computed(() =>
    collectSearchCategoryFilters(trimmedSearch.value ? results.value : featuredEntries.value),
  )
  const filteredResults = computed(() => filterSearchResultsByCategory(results.value, activeCategory.value))
  const filteredFeaturedEntries = computed(() => filterSearchResultsByCategory(featuredEntries.value, activeCategory.value))
  const featuredGroups = computed(() => groupSearchResults(filteredFeaturedEntries.value).slice(0, 4))
  const groupedFilteredResults = computed(() => groupSearchResults(filteredResults.value))
  const navigableEntries = computed(() => (trimmedSearch.value ? filteredResults.value : filteredFeaturedEntries.value))
  const highlightedResult = computed(() => navigableEntries.value.find(result => result.id === highlightedId.value) || null)
  const pinnedShortcutIds = computed(() => new Set(pinnedShortcutSnapshots.value.map(item => item.id)))

  const initializeDataSources = async () => {
    await Promise.all([
      initializeNotificationCenter(options.router),
      initializeSearchFindings(),
      initializeSearchBountyKnowledge(),
      initializeSearchScanTasks(),
      initializeSearchWorkflowIndex(),
      initializeSearchKnowledgeDocuments(),
      initializeSearchPlugins(),
    ])
  }

  const reloadWorkspaceState = () => {
    recentCommands.value = loadRecentCommands()
    recentSearches.value = loadRecentSearches()
    pinnedShortcutSnapshots.value = loadPinnedSearchShortcuts()
  }

  const resetHighlight = () => {
    highlightedId.value = navigableEntries.value[0]?.id || null
  }

  const rememberCurrentQuery = (fallbackText?: string) => {
    const currentQuery = trimmedSearch.value || String(fallbackText || '').trim()
    if (!currentQuery || isCommandHelpQuery(currentQuery)) {
      return
    }

    if (parsedCommand.value?.status === 'resolved') {
      recentCommands.value = addRecentCommand(currentQuery)
      recordSearchAnalyticsEvent('command-execute', { query: currentQuery })
      return
    }

    recentSearches.value = addRecentSearch(currentQuery)
    recordSearchAnalyticsEvent('search-submit', { query: currentQuery })
  }

  const togglePinnedEntry = (entry: GlobalSearchEntry | GlobalSearchResult) => {
    pinnedShortcutSnapshots.value = togglePinnedSearchShortcut(entry)
  }

  const isPinnedEntry = (entry: GlobalSearchEntry | GlobalSearchResult) => pinnedShortcutIds.value.has(entry.id)

  const reorderPinnedEntries = (activeId: string, targetId: string) => {
    pinnedShortcutSnapshots.value = reorderPinnedSearchShortcuts(activeId, targetId)
  }

  const removePinnedEntries = (entryIds: string[]) => {
    const previousSnapshots = clonePinnedSearchShortcutSnapshots(pinnedShortcutSnapshots.value)
    const nextSnapshots = removePinnedSearchShortcuts(entryIds)
    pinnedShortcutSnapshots.value = nextSnapshots

    const historyEntry = pushPinnedShortcutBatchHistoryEntry({
      title: '批量取消固定',
      summary: `移除 ${describePinnedSearchShortcutBatchTargets(entryIds, previousSnapshots)}`,
      operationType: 'remove',
      targetLabels: collectPinnedSearchShortcutBatchTargetLabels(entryIds, previousSnapshots),
      previousSnapshots,
    })

    toast.show({
      type: 'success',
      message: `已取消固定 ${entryIds.length} 个入口`,
      duration: 5000,
      actionLabel: '撤销',
      onAction: () => {
        const restoredSnapshots = undoPinnedShortcutBatchHistoryEntry(historyEntry.id)
        if (restoredSnapshots) {
          pinnedShortcutSnapshots.value = clonePinnedSearchShortcutSnapshots(restoredSnapshots)
        }
      },
    })
  }

  const updatePinnedEntryTags = (payload: { id: string; customTags: string[] }) => {
    pinnedShortcutSnapshots.value = updatePinnedSearchShortcutMeta(payload.id, {
      customTags: payload.customTags,
    })
  }

  const updateManyPinnedEntryTags = (payload: { entryIds: string[]; tag: string; mode: 'add' | 'remove' }) => {
    const previousSnapshots = clonePinnedSearchShortcutSnapshots(pinnedShortcutSnapshots.value)
    const nextSnapshots = payload.mode === 'add'
      ? addPinnedSearchShortcutTagToMany(payload.entryIds, payload.tag)
      : removePinnedSearchShortcutTagFromMany(payload.entryIds, payload.tag)

    pinnedShortcutSnapshots.value = nextSnapshots

    const actionText = payload.mode === 'add' ? '追加标签' : '移除标签'
    const historyEntry = pushPinnedShortcutBatchHistoryEntry({
      title: `批量${actionText}`,
      summary: `${actionText} #${payload.tag} 到 ${describePinnedSearchShortcutBatchTargets(payload.entryIds, previousSnapshots)}`,
      operationType: payload.mode === 'add' ? 'tag-add' : 'tag-remove',
      targetLabels: collectPinnedSearchShortcutBatchTargetLabels(payload.entryIds, previousSnapshots),
      previousSnapshots,
    })

    toast.show({
      type: 'success',
      message: `${actionText} #${payload.tag}，已更新 ${payload.entryIds.length} 个固定入口`,
      duration: 5000,
      actionLabel: '撤销',
      onAction: () => {
        const restoredSnapshots = undoPinnedShortcutBatchHistoryEntry(historyEntry.id)
        if (restoredSnapshots) {
          pinnedShortcutSnapshots.value = clonePinnedSearchShortcutSnapshots(restoredSnapshots)
        }
      },
    })
  }

  const restorePinnedShortcutHistory = (snapshots: PinnedSearchShortcutSnapshot[]) => {
    pinnedShortcutSnapshots.value = clonePinnedSearchShortcutSnapshots(snapshots)
  }

  const replacePinnedTag = (payload: { currentTag: string; nextTag: string }) => {
    const previousSnapshots = clonePinnedSearchShortcutSnapshots(pinnedShortcutSnapshots.value)
    pinnedShortcutSnapshots.value = replacePinnedSearchShortcutTagAcrossAll(payload.currentTag, payload.nextTag)
    const targetLabels = previousSnapshots
      .filter(item => (item.customTags || []).includes(payload.currentTag))
      .map(item => item.customTitle || item.title)

    const historyEntry = pushPinnedShortcutBatchHistoryEntry({
      title: '批量重命名标签',
      summary: `将 #${payload.currentTag} 迁移为 #${payload.nextTag}`,
      operationType: 'tag-add',
      targetLabels,
      previousSnapshots,
    })

    toast.show({
      type: 'success',
      message: `已将 #${payload.currentTag} 迁移为 #${payload.nextTag}`,
      duration: 5000,
      actionLabel: '撤销',
      onAction: () => {
        const restoredSnapshots = undoPinnedShortcutBatchHistoryEntry(historyEntry.id)
        if (restoredSnapshots) {
          pinnedShortcutSnapshots.value = clonePinnedSearchShortcutSnapshots(restoredSnapshots)
        }
      },
    })
  }

  const removePinnedTag = (payload: { tag: string; replacementTag?: string }) => {
    const previousSnapshots = clonePinnedSearchShortcutSnapshots(pinnedShortcutSnapshots.value)
    pinnedShortcutSnapshots.value = removePinnedSearchShortcutTagAcrossAll(payload.tag, payload.replacementTag)
    const targetLabels = previousSnapshots
      .filter(item => (item.customTags || []).includes(payload.tag))
      .map(item => item.customTitle || item.title)

    const summary = payload.replacementTag
      ? `移除 #${payload.tag} 并迁移到 #${payload.replacementTag}`
      : `移除标签 #${payload.tag}`
    const historyEntry = pushPinnedShortcutBatchHistoryEntry({
      title: '批量整理标签',
      summary,
      operationType: 'tag-remove',
      targetLabels,
      previousSnapshots,
    })

    toast.show({
      type: 'success',
      message: payload.replacementTag
        ? `已移除 #${payload.tag}，并迁移到 #${payload.replacementTag}`
        : `已移除标签 #${payload.tag}`,
      duration: 5000,
      actionLabel: '撤销',
      onAction: () => {
        const restoredSnapshots = undoPinnedShortcutBatchHistoryEntry(historyEntry.id)
        if (restoredSnapshots) {
          pinnedShortcutSnapshots.value = clonePinnedSearchShortcutSnapshots(restoredSnapshots)
        }
      },
    })
  }

  const openPinnedShortcutEditor = (entry: GlobalSearchEntry) => {
    editingPinnedShortcut.value = entry
  }

  const closePinnedShortcutEditor = () => {
    editingPinnedShortcut.value = null
  }

  const savePinnedShortcutEditor = (
    payload: { id: string; customTitle: string; customDescription: string; customTags: string[] },
  ) => {
    pinnedShortcutSnapshots.value = updatePinnedSearchShortcutMeta(payload.id, {
      customTitle: payload.customTitle,
      customDescription: payload.customDescription,
      customTags: payload.customTags,
    })
    closePinnedShortcutEditor()
  }

  const clearWorkspaceRecentCommands = () => {
    recentCommands.value = clearRecentCommands()
  }

  const clearWorkspaceRecentSearches = () => {
    recentSearches.value = clearRecentSearches()
  }

  const applyCommandSuggestion = (query: string) => {
    options.query.value = query
    selectedCommandSuggestionIndex.value = -1
    resetHighlight()
  }

  const selectCommandSuggestion = (index: number) => {
    selectedCommandSuggestionIndex.value = index
  }

  const moveCommandSuggestion = (direction: 1 | -1) => {
    selectedCommandSuggestionIndex.value = getNextCommandSuggestionIndex(
      selectedCommandSuggestionIndex.value,
      commandSuggestions.value.length,
      direction,
    )
  }

  const selectCategory = (category: SearchCategoryFilter) => {
    activeCategory.value = category
  }

  const cycleCategory = (direction: 1 | -1) => {
    activeCategory.value = getNextSearchCategoryFilter(
      activeCategory.value,
      availableCategoryFilters.value,
      direction,
    )
  }

  const moveHighlight = (direction: 1 | -1) => {
    if (navigableEntries.value.length === 0) {
      highlightedId.value = null
      return
    }

    const currentIndex = navigableEntries.value.findIndex(result => result.id === highlightedId.value)
    const nextIndex = currentIndex < 0
      ? 0
      : (currentIndex + direction + navigableEntries.value.length) % navigableEntries.value.length
    highlightedId.value = navigableEntries.value[nextIndex]?.id || null
  }

  const highlightResult = (id: string) => {
    highlightedId.value = id
  }

  const openEntry = async (entry: GlobalSearchEntry | GlobalSearchResult, config?: { beforeOpen?: () => void }) => {
    const currentQuery = trimmedSearch.value
    rememberCurrentQuery(currentQuery || entry.title)
    recordSearchAnalyticsEvent('result-open', {
      query: currentQuery || entry.title,
      entryId: entry.id,
      entryTitle: entry.title,
      entryCategory: entry.category,
    })

    config?.beforeOpen?.()

    if (entry.category === 'workflow' && entry.query?.workflowId && !entry.query?.execution_id) {
      setLastWorkflowId(entry.query.workflowId)
    }

    if (typeof entry.execute === 'function') {
      await entry.execute()
      return
    }

    await options.router.push({
      path: entry.path,
      query: entry.query,
    })
  }

  return {
    analyticsSummary,
    clearSearchAnalytics,
    recentCommands,
    recentSearches,
    pinnedShortcutSnapshots,
    pinnedShortcutEntries,
    editingPinnedShortcut,
    editingPinnedShortcutSnapshot,
    selectedCommandSuggestionIndex,
    activeCategory,
    highlightedId,
    trimmedSearch,
    showCommandHelp,
    commandHelpItems,
    parsedCommand,
    commandSuggestions,
    selectedCommandSuggestion,
    footerHints,
    actions,
    entries,
    availableShortcutEntries,
    searchEntries,
    featuredEntries,
    results,
    groupedResults,
    availableCategoryFilters,
    filteredResults,
    filteredFeaturedEntries,
    featuredGroups,
    groupedFilteredResults,
    navigableEntries,
    highlightedResult,
    initializeDataSources,
    reloadWorkspaceState,
    resetHighlight,
    rememberCurrentQuery,
    togglePinnedEntry,
    isPinnedEntry,
    reorderPinnedEntries,
    removePinnedEntries,
    updatePinnedEntryTags,
    updateManyPinnedEntryTags,
    replacePinnedTag,
    removePinnedTag,
    restorePinnedShortcutHistory,
    openPinnedShortcutEditor,
    closePinnedShortcutEditor,
    savePinnedShortcutEditor,
    clearWorkspaceRecentCommands,
    clearWorkspaceRecentSearches,
    applyCommandSuggestion,
    selectCommandSuggestion,
    moveCommandSuggestion,
    selectCategory,
    cycleCategory,
    moveHighlight,
    highlightResult,
    openEntry: (entry: GlobalSearchEntry | GlobalSearchResult, config?: { beforeOpen?: () => void }) =>
      openEntry(entry, config),
  }
}
