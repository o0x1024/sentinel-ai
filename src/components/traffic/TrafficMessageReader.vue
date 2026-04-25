<template>
  <div
    class="traffic-message-reader"
    :class="[readerThemeMode, { 'with-search-bar': showSearchBar }]"
    :style="readerStyle"
  >
    <textarea
      ref="textArea"
      class="traffic-message-reader-textarea"
      :value="modelValue"
      :wrap="textareaWrapMode"
      readonly
      spellcheck="false"
      @contextmenu="handleContextMenu"
      @scroll="handleScroll"
      @select="updateSearchMetrics"
    ></textarea>
    <div v-if="showSearchBar" class="editor-search-bar">
      <div class="editor-search-main">
        <label class="input input-sm flex items-center gap-2 w-full">
          <i class="fas fa-search text-base-content/50 text-xs"></i>
          <input
            ref="searchInput"
            v-model="searchQuery"
            type="text"
            class="grow"
            :placeholder="searchPlaceholder"
            @keydown.enter.prevent="handleSearchEnter"
            @keydown.esc.stop="clearSearch"
          />
          <span v-if="searchQuery" class="text-[11px] text-base-content/60 whitespace-nowrap">
            {{ searchStatusText }}
          </span>
        </label>
      </div>
      <div class="editor-search-actions">
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          :class="{ 'btn-active': searchCaseSensitive }"
          :title="searchCaseSensitiveTitle"
          @click="toggleCaseSensitive"
        >
          Aa
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          :class="{ 'btn-active': searchRegexp }"
          :title="searchRegexpTitle"
          @click="toggleRegexp"
        >
          .*
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          :disabled="!canNavigateSearchResults"
          :title="searchPreviousTitle"
          @click="findPreviousMatch"
        >
          <i class="fas fa-chevron-up"></i>
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          :disabled="!canNavigateSearchResults"
          :title="searchNextTitle"
          @click="findNextMatch"
        >
          <i class="fas fa-chevron-down"></i>
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          :disabled="!searchQuery"
          :title="searchClearTitle"
          @click="clearSearch"
        >
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onActivated, onDeactivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { isDarkHttpEditorTheme } from '@/components/http-editor/httpEditorTheme'
import { setSessionStorageItem } from '@/utils/browserStorage'
import {
  TRAFFIC_MESSAGE_TEXT_LINE_HEIGHT,
  useTrafficDisplaySettings,
} from './trafficDisplaySettings'

const scrollStateCache = new Map<string, { top: number; left: number }>()
const MAX_SCROLL_STATE_CACHE_SIZE = 100
const STORAGE_PREFIX = 'sentinel:traffic-message-reader-scroll:'

type SearchMatch = { from: number; to: number }

// Keep this component intentionally narrow: plain readonly text, search, and scroll state.
// HTTP-rich viewing should stay on HttpMessageSurface to avoid reintroducing dual-layer text rendering.

const props = withDefaults(defineProps<{
  modelValue: string
  height?: string
  customContextMenu?: boolean
  stateKey?: string
  showSearchBar?: boolean
  searchPlaceholder?: string
  searchNextTitle?: string
  searchPreviousTitle?: string
  searchCaseSensitiveTitle?: string
  searchRegexpTitle?: string
  searchClearTitle?: string
  searchNoMatchesText?: string
  searchInvalidRegexpText?: string
}>(), {
  modelValue: '',
  height: '100%',
  customContextMenu: false,
  stateKey: '',
  showSearchBar: false,
  searchPlaceholder: 'Search',
  searchNextTitle: 'Next match',
  searchPreviousTitle: 'Previous match',
  searchCaseSensitiveTitle: 'Case sensitive',
  searchRegexpTitle: 'Regex',
  searchClearTitle: 'Clear search',
  searchNoMatchesText: 'No matches',
  searchInvalidRegexpText: 'Invalid regex',
})

const emit = defineEmits<{
  (e: 'contextmenu', event: MouseEvent): void
}>()

const textArea = ref<HTMLTextAreaElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const searchQuery = ref('')
const searchCaseSensitive = ref(false)
const searchRegexp = ref(false)
const totalSearchMatches = ref(0)
const activeSearchMatch = ref(0)
const readerThemeMode = ref<'burp-light' | 'burp-dark'>(isDarkHttpEditorTheme() ? 'burp-dark' : 'burp-light')
const { settings } = useTrafficDisplaySettings()
let themeObserver: MutationObserver | null = null

const readerStyle = computed(() => ({
  height: props.height,
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
  '--traffic-editor-line-height': String(TRAFFIC_MESSAGE_TEXT_LINE_HEIGHT),
}))

const textareaWrapMode = computed<'soft' | 'off'>(() =>
  settings.value.wrapLongLines ? 'soft' : 'off',
)

function getSearchMatches(content: string): SearchMatch[] {
  if (!searchQuery.value) return []

  if (searchRegexp.value) {
    const flags = `g${searchCaseSensitive.value ? '' : 'i'}`
    const regex = new RegExp(searchQuery.value, flags)
    const matches: SearchMatch[] = []
    let match: RegExpExecArray | null

    while ((match = regex.exec(content)) !== null) {
      const value = match[0]
      matches.push({ from: match.index, to: match.index + value.length })
      if (value.length === 0) {
        regex.lastIndex += 1
      }
    }

    return matches
  }

  const needle = searchCaseSensitive.value ? searchQuery.value : searchQuery.value.toLowerCase()
  const haystack = searchCaseSensitive.value ? content : content.toLowerCase()
  const matches: SearchMatch[] = []
  let offset = 0

  while (offset <= haystack.length) {
    const foundIndex = haystack.indexOf(needle, offset)
    if (foundIndex === -1) break
    matches.push({ from: foundIndex, to: foundIndex + needle.length })
    offset = foundIndex + Math.max(needle.length, 1)
  }

  return matches
}

const hasInvalidSearchQuery = computed(() => {
  if (!searchQuery.value || !props.showSearchBar) return false
  try {
    void getSearchMatches(props.modelValue)
    return false
  } catch {
    return true
  }
})

const canNavigateSearchResults = computed(() =>
  Boolean(searchQuery.value) && totalSearchMatches.value > 0 && !hasInvalidSearchQuery.value,
)

const searchStatusText = computed(() => {
  if (!searchQuery.value) return ''
  if (hasInvalidSearchQuery.value) return props.searchInvalidRegexpText
  if (totalSearchMatches.value === 0) return props.searchNoMatchesText
  return `${activeSearchMatch.value || 1}/${totalSearchMatches.value}`
})

function updateSearchMetrics() {
  if (!props.showSearchBar || !searchQuery.value) {
    totalSearchMatches.value = 0
    activeSearchMatch.value = 0
    return
  }

  try {
    const matches = getSearchMatches(props.modelValue)
    totalSearchMatches.value = matches.length

    if (!textArea.value || matches.length === 0) {
      activeSearchMatch.value = 0
      return
    }

    const start = textArea.value.selectionStart
    const end = textArea.value.selectionEnd
    const index = matches.findIndex(match => match.from === start && match.to === end)
    activeSearchMatch.value = index >= 0 ? index + 1 : 1
  } catch {
    totalSearchMatches.value = 0
    activeSearchMatch.value = 0
  }
}

function focusSearchInput() {
  if (!props.showSearchBar) return
  requestAnimationFrame(() => {
    searchInput.value?.focus()
    searchInput.value?.select()
  })
}

function applySelection(from: number, to: number) {
  if (!textArea.value) return
  textArea.value.focus()
  textArea.value.setSelectionRange(from, to)
  updateSearchMetrics()
}

function navigateToMatch(direction: 'next' | 'previous') {
  if (!canNavigateSearchResults.value || !textArea.value) return false

  const matches = getSearchMatches(props.modelValue)
  if (matches.length === 0) return false

  const selectionStart = textArea.value.selectionStart
  const selectionEnd = textArea.value.selectionEnd
  const currentIndex = matches.findIndex(match => match.from === selectionStart && match.to === selectionEnd)

  let targetIndex = 0
  if (direction === 'next') {
    if (currentIndex >= 0) {
      targetIndex = (currentIndex + 1) % matches.length
    } else {
      targetIndex = matches.findIndex(match => match.from >= selectionEnd)
      if (targetIndex === -1) targetIndex = 0
    }
  } else if (currentIndex >= 0) {
    targetIndex = (currentIndex - 1 + matches.length) % matches.length
  } else {
    const reverseIndex = [...matches].reverse().findIndex(match => match.to <= selectionStart)
    targetIndex = reverseIndex === -1 ? matches.length - 1 : matches.length - 1 - reverseIndex
  }

  const target = matches[targetIndex]
  applySelection(target.from, target.to)
  return true
}

function findNextMatch() {
  return navigateToMatch('next')
}

function findPreviousMatch() {
  return navigateToMatch('previous')
}

function clearSearch() {
  searchQuery.value = ''
  updateSearchMetrics()
  textArea.value?.focus()
}

function toggleCaseSensitive() {
  searchCaseSensitive.value = !searchCaseSensitive.value
}

function toggleRegexp() {
  searchRegexp.value = !searchRegexp.value
}

function handleSearchEnter(event: KeyboardEvent) {
  if (event.shiftKey) {
    findPreviousMatch()
    return
  }
  findNextMatch()
}

function handleContextMenu(event: MouseEvent) {
  if (!props.customContextMenu) return
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function loadPersistedScrollState(stateKey: string) {
  try {
    const persisted = window.sessionStorage.getItem(`${STORAGE_PREFIX}${stateKey}`)
    if (!persisted) return null
    const parsed = JSON.parse(persisted)
    if (!Number.isFinite(parsed?.top) || !Number.isFinite(parsed?.left)) return null
    const state = { top: parsed.top, left: parsed.left }
    scrollStateCache.set(stateKey, state)
    return state
  } catch {
    return null
  }
}

function saveScrollStateForKey(stateKey: string) {
  if (!stateKey || !textArea.value) return

  const state = {
    top: textArea.value.scrollTop,
    left: textArea.value.scrollLeft,
  }
  if (scrollStateCache.has(stateKey)) {
    scrollStateCache.delete(stateKey)
  }
  scrollStateCache.set(stateKey, state)
  while (scrollStateCache.size > MAX_SCROLL_STATE_CACHE_SIZE) {
    const oldestKey = scrollStateCache.keys().next().value
    if (!oldestKey) break
    scrollStateCache.delete(oldestKey)
  }
  setSessionStorageItem(`${STORAGE_PREFIX}${stateKey}`, JSON.stringify(state))
}

function saveScrollState() {
  saveScrollStateForKey(props.stateKey)
}

function handleScroll() {
  saveScrollState()
}

function restoreScrollState() {
  if (!props.stateKey || !textArea.value) return
  const state = scrollStateCache.get(props.stateKey) ?? loadPersistedScrollState(props.stateKey)
  if (!state) return
  textArea.value.scrollTop = state.top
  textArea.value.scrollLeft = state.left
}

watch([searchQuery, searchCaseSensitive, searchRegexp], () => {
  updateSearchMetrics()
})

watch(() => props.modelValue, async () => {
  await nextTick()
  updateSearchMetrics()
  restoreScrollState()
})

watch(() => props.stateKey, (newKey, oldKey) => {
  if (oldKey && oldKey !== newKey) {
    saveScrollStateForKey(oldKey)
  }
  requestAnimationFrame(() => {
    restoreScrollState()
  })
})

onMounted(() => {
  restoreScrollState()
  updateSearchMetrics()

  themeObserver = new MutationObserver(() => {
    readerThemeMode.value = isDarkHttpEditorTheme() ? 'burp-dark' : 'burp-light'
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})

onDeactivated(() => {
  saveScrollState()
})

onActivated(() => {
  restoreScrollState()
})

onUnmounted(() => {
  saveScrollState()
  if (themeObserver) {
    themeObserver.disconnect()
    themeObserver = null
  }
})

defineExpose({
  focus: () => textArea.value?.focus(),
  focusSearch: focusSearchInput,
  getContent: () => props.modelValue,
  getSelectionRange: () => ({
    from: textArea.value?.selectionStart ?? 0,
    to: textArea.value?.selectionEnd ?? 0,
  }),
  selectAll: () => {
    if (!textArea.value) return
    textArea.value.focus()
    textArea.value.select()
    updateSearchMetrics()
  },
  setSelection: (from: number, to: number) => {
    applySelection(from, to)
  },
})
</script>

<style scoped>
.traffic-message-reader {
  width: 100%;
  height: v-bind(height);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
  border: 1px solid #d4d4d4;
  background: #ffffff;
}

.traffic-message-reader.burp-light {
  --traffic-selection-bg: rgb(207 227 255 / 0.9);
  --traffic-selection-fg: #111111;
}

.traffic-message-reader.burp-dark {
  --traffic-selection-bg: rgb(38 79 120 / 0.8);
  --traffic-selection-fg: #e6edf3;
}

.traffic-message-reader-textarea {
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  border: 0;
  margin: 0;
  padding: 0.75rem 0.9rem;
  resize: none;
  background: transparent;
  color: inherit;
  outline: none;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  font-family: var(--traffic-editor-font-family);
  font-size: var(--traffic-editor-font-size, 13px);
  line-height: var(--traffic-editor-line-height, 1.3);
  font-variant-ligatures: none;
}

.traffic-message-reader-textarea::selection {
  background-color: var(--traffic-selection-bg, rgb(207 227 255 / 0.9));
  color: var(--traffic-selection-fg, #111111);
  -webkit-text-fill-color: var(--traffic-selection-fg, #111111);
}

.editor-search-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  flex: 0 0 auto;
  border-top: 1px solid #d4d4d4;
  background: #f3f3f3;
}

.editor-search-main {
  flex: 1 1 auto;
  min-width: 0;
}

.editor-search-actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.traffic-message-reader :deep(.input) {
  border-color: #c8c8c8;
  background: #ffffff;
  box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.04);
}

.traffic-message-reader :deep(.btn) {
  border-color: #cfcfcf;
  background: linear-gradient(180deg, #ffffff 0%, #f1f1f1 100%);
  color: #444444;
}

.traffic-message-reader :deep(.btn.btn-active) {
  border-color: #a9bfdc;
  background: linear-gradient(180deg, #e9f2ff 0%, #d5e7ff 100%);
  color: #1f4d9a;
}
</style>
