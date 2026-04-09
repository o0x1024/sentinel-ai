<template>
  <div class="page-content-padded space-y-6 pb-20">
    <div class="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
      <div class="space-y-2">
        <div class="badge badge-outline badge-primary">Global Search</div>
        <h1 class="text-3xl font-bold">全局搜索</h1>
        <p class="text-sm text-base-content/70 max-w-3xl">
          搜索页面、常用入口和快捷分类，也支持命令式输入，例如 `theme dark`、`finding critical`、`notify rules`。输入 `?` 可查看完整命令帮助。
        </p>
      </div>

      <form class="w-full lg:w-auto" @submit.prevent="applySearch">
        <label class="input input-bordered flex items-center gap-3 w-full lg:w-[28rem]">
          <i class="fas fa-search text-base-content/50"></i>
          <input
            ref="searchInputRef"
            v-model="searchInput"
            type="text"
            class="grow"
            placeholder="搜索页面，或输入命令如 theme dark / notify rules"
            @keydown="handleSearchInputKeydown"
          />
          <kbd class="kbd kbd-sm hidden sm:inline-flex">Enter</kbd>
        </label>
      </form>
    </div>

    <div
      v-if="parsedCommand"
      class="flex flex-wrap items-center gap-2 rounded-2xl border border-primary/15 bg-primary/5 px-4 py-3 text-xs"
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

    <div v-if="commandSuggestions.length > 0" class="flex flex-wrap items-center gap-2">
      <span class="text-xs font-semibold text-base-content/50">命令补全</span>
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
      <span class="text-xs text-base-content/50">
        点击或按 ↑ ↓ Enter / Tab 会直接替换当前命令
      </span>
    </div>

    <div class="rounded-2xl border border-base-300 bg-base-200/35 px-4 py-3">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <span class="text-xs text-base-content/50">输入 `?`、`help` 或 `commands` 查看命令帮助</span>
        <CommandPaletteFooterHints :hints="footerHints" />
      </div>
    </div>

    <div v-if="showCommandHelp" class="space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold">命令帮助</h2>
        <span class="text-xs text-base-content/55">点击示例会直接替换当前命令</span>
      </div>

      <div class="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
        <button
          v-for="item in commandHelpItems"
          :key="item.id"
          class="card border border-base-300 bg-base-100 text-left transition-colors hover:border-primary hover:bg-primary/5"
          @click="applyCommandSuggestion(item.examples[0])"
        >
          <div class="card-body gap-3">
            <div class="flex items-start gap-3">
              <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                <i class="fas fa-terminal"></i>
              </div>
              <div class="min-w-0">
                <div class="font-semibold">{{ item.title }}</div>
                <div class="mt-1 text-sm text-base-content/65">{{ item.description }}</div>
              </div>
            </div>
            <div class="badge badge-outline badge-sm font-mono w-fit">{{ item.syntax }}</div>
            <div class="flex flex-wrap gap-1">
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
    </div>

    <div v-else-if="!trimmedSearch" class="space-y-4">
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
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium text-base-content/75">最近命令</div>
          <button
            class="text-xs text-base-content/50 transition-colors hover:text-primary"
            @click="clearSearchPageRecentCommands"
          >
            清空
          </button>
        </div>
        <div class="flex flex-wrap gap-2">
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

      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold">推荐入口</h2>
        <span class="text-xs text-base-content/55">输入关键词后会按标题、描述和别名排序</span>
      </div>

      <div v-if="recentSearches.length > 0" class="space-y-2">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium text-base-content/75">最近搜索</div>
          <button
            class="text-xs text-base-content/50 transition-colors hover:text-primary"
            @click="clearSearchPageRecentSearches"
          >
            清空
          </button>
        </div>
        <div class="flex flex-wrap gap-2">
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

      <SearchAnalyticsPanel
        :summary="analyticsSummary"
        @clear="clearSearchAnalytics"
      />

      <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        <div
          v-for="entry in featuredEntries"
          :key="entry.id"
          class="card border border-base-300 bg-base-100 transition-colors hover:border-primary hover:bg-primary/5"
        >
          <div class="card-body gap-3">
            <div class="flex items-start justify-between gap-3">
              <button class="flex min-w-0 items-start gap-3 text-left" @click="openEntry(entry)">
                <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                  <i :class="entry.icon"></i>
                </div>
                <div class="min-w-0">
                  <div class="font-semibold">{{ entry.title }}</div>
                  <div class="mt-1 text-sm text-base-content/65">{{ entry.description }}</div>
                </div>
              </button>
              <SearchShortcutPinButton
                :pinned="isPinnedEntry(entry)"
                @toggle="togglePinnedEntry(entry)"
              />
            </div>
            <div class="text-xs text-base-content/50">{{ formatSearchKeywords(entry.keywords) }}</div>
            <div v-if="entry.aliases?.length" class="flex flex-wrap gap-1">
              <span
                v-for="alias in entry.aliases.slice(0, 2)"
                :key="`${entry.id}-${alias}`"
                class="badge badge-ghost badge-sm font-mono text-[11px]"
              >
                {{ alias }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="space-y-4">
      <div class="flex items-center justify-between gap-3">
        <div>
          <h2 class="text-lg font-semibold">搜索结果</h2>
          <p class="text-sm text-base-content/65">
            {{ results.length > 0 ? `共找到 ${results.length} 个结果` : '没有找到匹配结果' }}
          </p>
        </div>
        <button
          v-if="results.length > 0"
          class="btn btn-primary btn-sm"
          @click="openEntry(results[0])"
        >
          打开最佳结果
        </button>
      </div>

      <div v-if="results.length === 0" class="rounded-2xl border border-dashed border-base-300 px-6 py-12 text-center">
        <div class="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-base-200">
          <i class="fas fa-search text-lg text-base-content/60"></i>
        </div>
        <h3 class="text-lg font-semibold">没有找到匹配项</h3>
        <p class="mt-2 text-sm text-base-content/70">
          换个关键词试试，例如“漏洞”“工作流”“消息”“插件”或“性能”。
        </p>
      </div>

      <div v-else class="space-y-5">
        <section
          v-for="group in groupedResults"
          :key="group.category"
          class="space-y-3"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <h3 class="text-sm font-semibold uppercase tracking-wider text-base-content/55">
                {{ group.label }}
              </h3>
              <span class="badge badge-outline badge-sm">{{ group.items.length }}</span>
            </div>
            <span class="text-xs text-base-content/45">按相关度排序</span>
          </div>

          <div
            v-for="result in group.items"
            :key="result.id"
            class="card w-full border border-base-300 bg-base-100 transition-colors hover:border-primary hover:bg-primary/5"
          >
            <div class="card-body gap-4 sm:flex-row sm:items-center sm:justify-between">
              <button class="flex min-w-0 flex-1 items-start gap-3 text-left" @click="openEntry(result)">
                <div class="mt-1 flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-base-200 text-base-content/80">
                  <i :class="result.icon"></i>
                </div>

                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <h3 class="truncate text-base font-semibold">
                      <template v-for="(part, partIndex) in getHighlightedParts(result.title)" :key="`${result.id}-title-${partIndex}`">
                        <mark v-if="part.matched" class="rounded bg-warning/30 px-0.5 text-inherit">{{ part.text }}</mark>
                        <span v-else>{{ part.text }}</span>
                      </template>
                    </h3>
                    <span class="badge badge-outline badge-sm">
                      {{ getSearchCategoryLabel(result.category) }}
                    </span>
                  </div>
                  <p class="mt-2 text-sm leading-6 text-base-content/70">
                    <template v-for="(part, partIndex) in getHighlightedParts(result.description)" :key="`${result.id}-description-${partIndex}`">
                      <mark v-if="part.matched" class="rounded bg-warning/25 px-0.5 text-inherit">{{ part.text }}</mark>
                      <span v-else>{{ part.text }}</span>
                    </template>
                  </p>
                  <div class="mt-3 flex flex-wrap items-center gap-2 text-xs text-base-content/50">
                    <span class="badge badge-ghost badge-sm">{{ result.path }}</span>
                    <span>{{ formatSearchKeywords(result.keywords) }}</span>
                    <span class="font-medium text-base-content/45">{{ getSearchActionLabel(result) }}</span>
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

              <div class="flex shrink-0 items-center gap-2">
                <SearchShortcutPinButton
                  :pinned="isPinnedEntry(result)"
                  @toggle="togglePinnedEntry(result)"
                />
                <div class="text-xs text-base-content/45">
                  Score {{ result.score }}
                </div>
              </div>
            </div>
          </div>
        </section>
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
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import CommandPaletteFooterHints from '@/components/Layout/CommandPaletteFooterHints.vue'
import PinnedSearchShortcuts from '@/components/Layout/PinnedSearchShortcuts.vue'
import PinnedSearchShortcutEditor from '@/components/Layout/PinnedSearchShortcutEditor.vue'
import SearchAnalyticsPanel from '@/components/Layout/SearchAnalyticsPanel.vue'
import SearchShortcutPinButton from '@/components/Layout/SearchShortcutPinButton.vue'
import { useSearchWorkspaceController } from '@/composables/useSearchWorkspaceController'
import { GLOBAL_SEARCH_FOCUS_EVENT } from '@/services/globalSearchFocus'
import {
  getSearchCategoryLabel,
  type GlobalSearchEntry,
  type GlobalSearchResult,
} from '@/services/globalSearch'
import { formatSearchKeywords, getSearchActionLabel } from '@/services/searchPresentation'
import { buildHighlightedParts } from '@/utils/searchHighlight'

const route = useRoute()
const router = useRouter()

const getRouteQuery = () => (typeof route.query.q === 'string' ? route.query.q : '')

const searchInputRef = ref<HTMLInputElement | null>(null)
const searchInput = ref(getRouteQuery())
const {
  analyticsSummary,
  clearSearchAnalytics,
  recentCommands,
  recentSearches,
  pinnedShortcutEntries,
  editingPinnedShortcut,
  editingPinnedShortcutSnapshot,
  selectedCommandSuggestionIndex,
  trimmedSearch,
  showCommandHelp,
  commandHelpItems,
  parsedCommand,
  commandSuggestions,
  selectedCommandSuggestion,
  footerHints,
  featuredEntries,
  results,
  groupedResults,
  initializeDataSources,
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
  openEntry,
  rememberCurrentQuery,
} = useSearchWorkspaceController({
  router,
  query: searchInput,
})

const focusSearchInput = () => {
  nextTick(() => {
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  })
}

const handleGlobalSearchFocus = () => {
  if (route.path !== '/search') {
    return
  }

  focusSearchInput()
}

onMounted(() => {
  window.addEventListener(GLOBAL_SEARCH_FOCUS_EVENT, handleGlobalSearchFocus)
  void initializeDataSources()
})

onUnmounted(() => {
  window.removeEventListener(GLOBAL_SEARCH_FOCUS_EVENT, handleGlobalSearchFocus)
})

watch(
  () => route.query.q,
  value => {
    searchInput.value = typeof value === 'string' ? value : ''
  },
)

const applySearch = async () => {
  const q = trimmedSearch.value
  rememberCurrentQuery()
  await router.replace({
    path: '/search',
    query: q ? { q } : {},
  })
}

const applyRecentSearch = async (query: string) => {
  searchInput.value = query
  await applySearch()
}

const clearSearchPageRecentCommands = () => {
  clearWorkspaceRecentCommands()
}

const clearSearchPageRecentSearches = () => {
  clearWorkspaceRecentSearches()
}

const applyCommandSuggestion = async (query: string) => {
  applyWorkspaceCommandSuggestion(query)
  await applySearch()
  focusSearchInput()
}

const handleSearchInputKeydown = async (event: KeyboardEvent) => {
  if (commandSuggestions.value.length === 0) {
    return
  }

  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveCommandSuggestion(1)
    return
  }

  if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveCommandSuggestion(-1)
    return
  }

  if (event.key === 'Enter') {
    event.preventDefault()
    await applyCommandSuggestion(selectedCommandSuggestion.value?.query || commandSuggestions.value[0].query)
    return
  }

  if (event.key !== 'Tab' || event.shiftKey) {
    return
  }

  event.preventDefault()
  await applyCommandSuggestion(selectedCommandSuggestion.value?.query || commandSuggestions.value[0].query)
}

watch(commandSuggestions, (suggestions) => {
  if (suggestions.length === 0) {
    selectedCommandSuggestionIndex.value = -1
    return
  }

  if (selectedCommandSuggestionIndex.value < 0 || selectedCommandSuggestionIndex.value >= suggestions.length) {
    selectedCommandSuggestionIndex.value = 0
  }
}, { immediate: true })

const getHighlightedParts = (text: string) => buildHighlightedParts(text, trimmedSearch.value)
</script>
