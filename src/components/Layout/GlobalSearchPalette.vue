<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-[1200]"
    >
      <button
        class="absolute inset-0 bg-base-content/20 backdrop-blur-sm"
        aria-label="Close global search"
        @click="closePalette"
      ></button>

      <div class="relative mx-auto mt-[10vh] w-[min(44rem,calc(100vw-2rem))]">
        <div
          class="overflow-hidden rounded-[1.75rem] border border-base-300 bg-base-100 shadow-2xl"
        >
          <div class="border-b border-base-300/70 px-4 py-4">
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                <i class="fas fa-search"></i>
              </div>
              <label class="flex min-w-0 flex-1 items-center gap-3">
                <input
                  ref="inputRef"
                  v-model="searchQuery"
                  type="text"
                  class="w-full bg-transparent text-base outline-none placeholder:text-base-content/40"
                  placeholder="搜索页面，或输入命令如 theme dark / finding critical"
                  @keydown="handleKeydown"
                />
              </label>
              <kbd class="kbd kbd-sm hidden sm:inline-flex">Esc</kbd>
            </div>
            <div
              v-if="parsedCommand"
              class="mt-4 flex flex-wrap items-center gap-2 rounded-2xl border border-primary/15 bg-primary/5 px-3 py-2 text-[11px]"
            >
              <span class="font-semibold text-primary">
                {{ parsedCommand.status === 'resolved' ? '命令已解析' : '命令待补全' }}
              </span>
              <span
                v-for="chip in parsedCommand.chips"
                :key="`${chip.key}-${chip.value}`"
                class="badge border-primary/20 bg-base-100 font-mono text-[11px] text-primary"
              >
                {{ chip.text }}
              </span>
              <span class="text-base-content/55">
                {{ parsedCommand.status === 'resolved' ? `已命中 ${parsedCommand.actionIds.length} 个动作` : '继续补充参数以缩小范围' }}
              </span>
            </div>
            <div v-if="commandSuggestions.length > 0" class="mt-3 flex flex-wrap items-center gap-2">
              <span class="text-[11px] font-semibold text-base-content/50">命令补全</span>
              <button
                v-for="(suggestion, index) in commandSuggestions"
                :key="suggestion.query"
                class="badge border font-mono text-[11px] transition-colors"
                :class="selectedCommandSuggestionIndex === index
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-base-300 bg-base-100 hover:border-primary hover:text-primary'"
                @mouseenter="selectCommandSuggestion(index)"
                @click="applyCommandSuggestion(suggestion.query)"
              >
                {{ suggestion.label }}
              </button>
              <span class="text-[11px] text-base-content/45">↑ ↓ 选择，Tab / Enter 接受</span>
            </div>
            <div class="mt-4 flex flex-wrap items-center gap-2">
              <button
                v-for="category in availableCategoryFilters"
                :key="category"
                class="badge border transition-colors"
                :class="activeCategory === category
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
                @click="selectCategory(category)"
              >
                {{ getSearchCategoryFilterLabel(category) }}
              </button>
            </div>
          </div>

          <div class="max-h-[62vh] overflow-y-auto p-3">
            <div v-if="showCommandHelp" class="space-y-4">
              <div class="px-2 text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
                命令帮助
              </div>
              <button
                v-for="item in commandHelpItems"
                :key="item.id"
                class="flex w-full items-start gap-3 rounded-2xl border border-base-300 px-4 py-4 text-left transition-colors hover:border-primary hover:bg-primary/5"
                @click="applyCommandSuggestion(item.examples[0])"
              >
                <div class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                  <i class="fas fa-terminal"></i>
                </div>
                <div class="min-w-0 flex-1 space-y-2">
                  <div class="flex items-center gap-2">
                    <span class="font-medium">{{ item.title }}</span>
                    <span class="badge badge-outline badge-sm font-mono">{{ item.syntax }}</span>
                  </div>
                  <p class="text-sm text-base-content/65">{{ item.description }}</p>
                  <div class="flex flex-wrap gap-2">
                    <span
                      v-for="example in item.examples"
                      :key="example"
                      class="badge badge-ghost badge-sm font-mono text-[11px]"
                    >
                      {{ example }}
                    </span>
                  </div>
                </div>
              </button>
            </div>

            <div v-else-if="!trimmedSearchQuery" class="space-y-4">
              <PinnedSearchShortcuts
                title="固定快捷入口"
                :shortcuts="pinnedShortcutEntries"
                @select="openEntry"
                @edit="openPinnedShortcutEditor"
                @toggle="togglePinnedEntry"
                @update-tags="updatePinnedEntryTags"
                @update-many-tags="updateManyPinnedEntryTags"
                @replace-tag="replacePinnedTag"
                @remove-tag="removePinnedTag"
                @remove-many="removePinnedEntries"
                @reorder="reorderPinnedEntries"
                @restore-history="restorePinnedShortcutHistory"
              />

              <div v-if="recentCommands.length > 0" class="space-y-2">
                <div class="flex items-center justify-between gap-3 px-2">
                  <div class="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
                    最近命令
                  </div>
                  <button
                    class="text-[11px] text-base-content/45 transition-colors hover:text-primary"
                    @click="clearPaletteRecentCommands"
                  >
                    清空
                  </button>
                </div>
                <div class="flex flex-wrap gap-2 px-2">
                  <button
                    v-for="item in recentCommands"
                    :key="item"
                    class="badge border-primary/20 bg-primary/5 font-mono text-[11px] text-primary transition-colors hover:border-primary hover:bg-primary/10"
                    @click="applyCommandSuggestion(item)"
                  >
                    {{ item }}
                  </button>
                </div>
              </div>

              <div v-if="recentSearches.length > 0" class="space-y-2">
                <div class="flex items-center justify-between gap-3 px-2">
                  <div class="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
                    最近搜索
                  </div>
                  <button
                    class="text-[11px] text-base-content/45 transition-colors hover:text-primary"
                    @click="clearPaletteRecentSearches"
                  >
                    清空
                  </button>
                </div>
                <div class="flex flex-wrap gap-2 px-2">
                  <button
                    v-for="item in recentSearches"
                    :key="item"
                    class="badge badge-outline badge-sm hover:border-primary hover:text-primary"
                    @click="applyRecentSearch(item)"
                  >
                    {{ item }}
                  </button>
                </div>
              </div>

              <div
                v-for="group in featuredGroups"
                :key="group.category"
                class="space-y-2"
              >
                <div class="px-2 text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
                  {{ group.label }}
                </div>
                <div
                  v-for="item in group.items"
                  :key="item.id"
                  class="flex items-start gap-3 rounded-2xl px-3 py-3 transition-colors"
                  :class="highlightedResult?.id === item.id ? 'bg-primary/10 text-primary' : 'hover:bg-base-200'"
                  @mouseenter="highlightResult(item.id)"
                >
                  <button class="flex min-w-0 flex-1 items-start gap-3 text-left" @click="openEntry(item)">
                    <div
                      class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl"
                      :class="highlightedResult?.id === item.id ? 'bg-primary/15 text-primary' : 'bg-base-200 text-base-content/70'"
                    >
                      <i :class="item.icon"></i>
                    </div>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <span class="truncate font-medium">{{ item.title }}</span>
                        <span class="badge badge-ghost badge-xs">{{ getSearchCategoryLabel(item.category) }}</span>
                      </div>
                      <p class="mt-1 line-clamp-2 text-sm text-base-content/65">{{ item.description }}</p>
                      <div class="mt-2 text-[11px] font-medium uppercase tracking-wider text-base-content/40">
                        {{ getSearchActionLabel(item) }}
                      </div>
                      <div v-if="item.aliases?.length" class="mt-2 flex flex-wrap items-center gap-1">
                        <span
                          v-for="alias in item.aliases.slice(0, 2)"
                          :key="`${item.id}-${alias}`"
                          class="badge badge-ghost badge-sm font-mono text-[11px]"
                        >
                          {{ alias }}
                        </span>
                      </div>
                    </div>
                  </button>
                  <SearchShortcutPinButton
                    :pinned="isPinnedEntry(item)"
                    @toggle="togglePinnedEntry(item)"
                  />
                </div>
              </div>
            </div>

            <div v-else-if="filteredResults.length === 0" class="px-4 py-12 text-center">
              <div class="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-base-200">
                <i class="fas fa-compass text-lg text-base-content/50"></i>
              </div>
              <h2 class="text-lg font-semibold">没有匹配结果</h2>
              <p class="mt-2 text-sm text-base-content/65">
                换个关键词试试，或者按 Tab 切到其他分组继续找。
              </p>
            </div>

            <div v-else class="space-y-5">
              <section
                v-for="group in groupedResults"
                :key="group.category"
                class="space-y-2"
              >
                <div class="flex items-center justify-between px-2">
                  <div class="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
                    {{ group.label }}
                  </div>
                  <div class="text-[11px] text-base-content/40">{{ group.items.length }} 项</div>
                </div>

                <div
                  v-for="result in group.items"
                  :key="result.id"
                  class="flex items-start gap-3 rounded-2xl px-3 py-3 transition-colors"
                  :class="highlightedResult?.id === result.id ? 'bg-primary/10 text-primary' : 'hover:bg-base-200'"
                  @mouseenter="highlightResult(result.id)"
                >
                  <button class="flex min-w-0 flex-1 items-start gap-3 text-left" @click="openEntry(result)">
                    <div
                      class="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl"
                      :class="highlightedResult?.id === result.id ? 'bg-primary/15 text-primary' : 'bg-base-200 text-base-content/70'"
                    >
                      <i :class="result.icon"></i>
                    </div>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <span class="truncate font-medium">
                          <template v-for="(part, index) in getHighlightedParts(result.title)" :key="`${result.id}-title-${index}`">
                            <mark v-if="part.matched" class="rounded bg-warning/30 px-0.5 text-inherit">{{ part.text }}</mark>
                            <span v-else>{{ part.text }}</span>
                          </template>
                        </span>
                        <span class="badge badge-ghost badge-xs">{{ getSearchCategoryLabel(result.category) }}</span>
                      </div>
                      <p class="mt-1 line-clamp-2 text-sm text-base-content/65">
                        <template v-for="(part, index) in getHighlightedParts(result.description)" :key="`${result.id}-description-${index}`">
                          <mark v-if="part.matched" class="rounded bg-warning/25 px-0.5 text-inherit">{{ part.text }}</mark>
                          <span v-else>{{ part.text }}</span>
                        </template>
                      </p>
                      <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px] text-base-content/45">
                        <span class="badge badge-ghost badge-sm">{{ result.path }}</span>
                        <span>{{ formatSearchKeywords(result.keywords) }}</span>
                        <span class="font-medium uppercase tracking-wider">{{ getSearchActionLabel(result) }}</span>
                      </div>
                      <div v-if="result.aliases?.length" class="mt-2 flex flex-wrap items-center gap-1">
                        <span
                          v-for="alias in result.aliases.slice(0, 2)"
                          :key="`${result.id}-${alias}`"
                          class="badge badge-ghost badge-sm font-mono text-[11px]"
                        >
                          {{ alias }}
                        </span>
                      </div>
                    </div>
                  </button>
                  <SearchShortcutPinButton
                    :pinned="isPinnedEntry(result)"
                    @toggle="togglePinnedEntry(result)"
                  />
                </div>
              </section>
            </div>
          </div>

          <div class="border-t border-base-300/70 bg-base-200/40 px-4 py-3">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <span class="text-[11px] text-base-content/45">Cmd/Ctrl + K 打开全局命令面板</span>
              <CommandPaletteFooterHints :hints="footerHints" />
            </div>
          </div>
        </div>
      </div>
    </div>

    <PinnedSearchShortcutEditor
      :visible="editingPinnedShortcut !== null"
      :shortcut="editingPinnedShortcut"
      :custom-title="editingPinnedShortcutSnapshot?.customTitle"
      :custom-description="editingPinnedShortcutSnapshot?.customDescription"
      :custom-tags="editingPinnedShortcutSnapshot?.customTags"
      :default-title="editingPinnedShortcutSnapshot?.title || editingPinnedShortcut?.title"
      :default-description="editingPinnedShortcutSnapshot?.description || editingPinnedShortcut?.description"
      @close="closePinnedShortcutEditor"
      @save="savePinnedShortcutEditor"
    />
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import CommandPaletteFooterHints from './CommandPaletteFooterHints.vue'
import PinnedSearchShortcuts from './PinnedSearchShortcuts.vue'
import PinnedSearchShortcutEditor from './PinnedSearchShortcutEditor.vue'
import SearchShortcutPinButton from './SearchShortcutPinButton.vue'
import { useSearchWorkspaceController } from '@/composables/useSearchWorkspaceController'
import {
  GLOBAL_SEARCH_CLOSE_EVENT,
  GLOBAL_SEARCH_OPEN_EVENT,
} from '@/services/globalSearchFocus'
import {
  type GlobalSearchEntry,
  type GlobalSearchResult,
  getSearchCategoryLabel,
} from '@/services/globalSearch'
import {
  formatSearchKeywords,
  getSearchActionLabel,
  getSearchCategoryFilterLabel,
} from '@/services/searchPresentation'
import { buildHighlightedParts } from '@/utils/searchHighlight'

const router = useRouter()
const inputRef = ref<HTMLInputElement | null>(null)
const visible = ref(false)
const searchQuery = ref('')
const {
  recentCommands,
  recentSearches,
  pinnedShortcutEntries,
  editingPinnedShortcut,
  editingPinnedShortcutSnapshot,
  selectedCommandSuggestionIndex,
  activeCategory,
  highlightedId,
  trimmedSearch: trimmedSearchQuery,
  showCommandHelp,
  commandHelpItems,
  parsedCommand,
  commandSuggestions,
  selectedCommandSuggestion,
  footerHints,
  featuredGroups,
  groupedFilteredResults: groupedResults,
  availableCategoryFilters,
  filteredResults,
  navigableEntries,
  highlightedResult,
  initializeDataSources,
  reloadWorkspaceState,
  resetHighlight,
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
  applyCommandSuggestion: applyWorkspaceCommandSuggestion,
  selectCommandSuggestion,
  moveCommandSuggestion,
  selectCategory,
  cycleCategory,
  moveHighlight,
  highlightResult,
  openEntry: openWorkspaceEntry,
  rememberCurrentQuery,
} = useSearchWorkspaceController({
  router,
  query: searchQuery,
  includeCloseHint: true,
})

const focusInput = () => {
  nextTick(() => {
    inputRef.value?.focus()
    inputRef.value?.select()
  })
}

const openPalette = async () => {
  visible.value = true
  searchQuery.value = ''
  activeCategory.value = 'all'
  reloadWorkspaceState()
  await initializeDataSources()
  resetHighlight()
  focusInput()
}

const closePalette = () => {
  visible.value = false
  searchQuery.value = ''
  highlightedId.value = null
  activeCategory.value = 'all'
  selectedCommandSuggestionIndex.value = -1
}

const clearPaletteRecentCommands = () => {
  clearWorkspaceRecentCommands()
}

const clearPaletteRecentSearches = () => {
  clearWorkspaceRecentSearches()
}

const openSearchPage = async () => {
  const query = trimmedSearchQuery.value
  if (!query) {
    return
  }

  rememberCurrentQuery()

  closePalette()
  await router.push({
    path: '/search',
    query: { q: query },
  })
}

const openEntry = async (entry: GlobalSearchEntry | GlobalSearchResult) =>
  openWorkspaceEntry(entry, { beforeOpen: closePalette })

const applyRecentSearch = async (query: string) => {
  searchQuery.value = query
  resetHighlight()
  await openSearchPage()
}

const applyCommandSuggestion = (query: string) => {
  applyWorkspaceCommandSuggestion(query)
  focusInput()
}


const performPrimaryAction = async () => {
  const query = trimmedSearchQuery.value
  const primaryResult = highlightedResult.value || navigableEntries.value[0] || null

  if (showCommandHelp.value) {
    return
  }

  if (primaryResult) {
    await openEntry(primaryResult)
    return
  }

  if (!query && recentSearches.value.length > 0) {
    await applyRecentSearch(recentSearches.value[0])
    return
  }

  if (!query) {
    return
  }

  await openSearchPage()
}

const handleKeydown = async (event: KeyboardEvent) => {
  if (event.key === 'Tab') {
    if (!event.shiftKey && commandSuggestions.value.length > 0) {
      event.preventDefault()
      applyCommandSuggestion(selectedCommandSuggestion.value?.query || commandSuggestions.value[0].query)
      return
    }

    event.preventDefault()
    cycleCategory(event.shiftKey ? -1 : 1)
    return
  }

  if (event.key === 'ArrowDown') {
    if (commandSuggestions.value.length > 0) {
      event.preventDefault()
      moveCommandSuggestion(1)
      return
    }

    event.preventDefault()
    moveHighlight(1)
    return
  }

  if (event.key === 'ArrowUp') {
    if (commandSuggestions.value.length > 0) {
      event.preventDefault()
      moveCommandSuggestion(-1)
      return
    }

    event.preventDefault()
    moveHighlight(-1)
    return
  }

  if (event.key === 'Escape') {
    event.preventDefault()
    closePalette()
    return
  }

  if (event.key === 'Enter') {
    if (commandSuggestions.value.length > 0) {
      event.preventDefault()
      applyCommandSuggestion(selectedCommandSuggestion.value?.query || commandSuggestions.value[0].query)
      return
    }

    event.preventDefault()
    await performPrimaryAction()
  }
}

const handleOpenEvent = () => {
  void openPalette()
}

const handleCloseEvent = () => {
  closePalette()
}

watch(availableCategoryFilters, (filters) => {
  if (!filters.includes(activeCategory.value)) {
    activeCategory.value = 'all'
  }
}, { immediate: true })

watch(navigableEntries, (items) => {
  if (!items.some(item => item.id === highlightedId.value)) {
    highlightedId.value = items[0]?.id || null
  }
}, { immediate: true })

watch(commandSuggestions, (suggestions) => {
  if (suggestions.length === 0) {
    selectedCommandSuggestionIndex.value = -1
    return
  }

  if (selectedCommandSuggestionIndex.value < 0 || selectedCommandSuggestionIndex.value >= suggestions.length) {
    selectedCommandSuggestionIndex.value = 0
  }
}, { immediate: true })

onMounted(() => {
  window.addEventListener(GLOBAL_SEARCH_OPEN_EVENT, handleOpenEvent)
  window.addEventListener(GLOBAL_SEARCH_CLOSE_EVENT, handleCloseEvent)
})

onUnmounted(() => {
  window.removeEventListener(GLOBAL_SEARCH_OPEN_EVENT, handleOpenEvent)
  window.removeEventListener(GLOBAL_SEARCH_CLOSE_EVENT, handleCloseEvent)
})

const getHighlightedParts = (text: string) => buildHighlightedParts(text, trimmedSearchQuery.value)
</script>
