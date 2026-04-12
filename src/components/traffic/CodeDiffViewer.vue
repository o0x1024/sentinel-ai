<template>
  <div class="code-diff-viewer h-full min-h-0 overflow-hidden rounded-lg border border-base-300 bg-base-100" :style="editorStyle">
    <div
      ref="containerRef"
      class="diff-editor-container min-h-0 overflow-hidden"
    ></div>
    <div class="editor-search-bar border-t border-base-300 bg-base-200/95">
      <div class="editor-search-main">
        <label class="input input-sm input-bordered flex items-center gap-2 w-full bg-base-100">
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
        <span class="badge badge-outline badge-sm text-[11px]">
          {{ searchActiveSideTitle }}: {{ activeSearchSideLabel }}
        </span>
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
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { EditorView } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { MergeView } from '@codemirror/merge'
import { drawSelection, highlightActiveLineGutter, keymap, lineNumbers } from '@codemirror/view'
import { SearchQuery, findNext, findPrevious, getSearchQuery, search, setSearchQuery } from '@codemirror/search'
import {
  getHttpCodeThemeExtensions,
  isDarkHttpEditorTheme,
} from '@/components/http-editor/httpEditorTheme'
import { getHttpEditorLanguageExtensions } from '@/components/http-editor/httpEditorHttpMode'
import {
  shouldHighlightTrafficMessageSyntax,
  useTrafficDisplaySettings,
  type TrafficMessageType,
} from './trafficDisplaySettings'

const props = withDefaults(defineProps<{
  leftText: string
  rightText: string
  messageType?: TrafficMessageType
  searchPlaceholder?: string
  searchNextTitle?: string
  searchPreviousTitle?: string
  searchCaseSensitiveTitle?: string
  searchRegexpTitle?: string
  searchActiveSideTitle?: string
  searchLeftLabel?: string
  searchRightLabel?: string
  searchClearTitle?: string
  searchNoMatchesText?: string
  searchInvalidRegexpText?: string
}>(), {
  leftText: '',
  rightText: '',
  messageType: 'generic',
  searchPlaceholder: 'Search',
  searchNextTitle: 'Next match',
  searchPreviousTitle: 'Previous match',
  searchCaseSensitiveTitle: 'Case sensitive',
  searchRegexpTitle: 'Regex',
  searchActiveSideTitle: 'Active side',
  searchLeftLabel: 'Left',
  searchRightLabel: 'Right',
  searchClearTitle: 'Clear search',
  searchNoMatchesText: 'No matches',
  searchInvalidRegexpText: 'Invalid regex',
})

const emit = defineEmits<{
  (e: 'contextmenu', event: MouseEvent): void
}>()

const containerRef = ref<HTMLDivElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const searchQuery = ref('')
const searchCaseSensitive = ref(false)
const searchRegexp = ref(false)
const totalSearchMatches = ref(0)
const activeSearchSide = ref<'a' | 'b'>('b')
let mergeView: MergeView | null = null
let themeObserver: MutationObserver | null = null
const { settings } = useTrafficDisplaySettings()
const editorStyle = computed(() => ({
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
}))
const hasInvalidSearchQuery = computed(() => {
  if (!searchQuery.value) return false
  return !buildSearchQuery().valid
})
const canNavigateSearchResults = computed(() => {
  if (!searchQuery.value) return false
  return totalSearchMatches.value > 0 && !hasInvalidSearchQuery.value
})
const activeSearchSideLabel = computed(() =>
  activeSearchSide.value === 'a' ? props.searchLeftLabel : props.searchRightLabel,
)
const searchStatusText = computed(() => {
  if (!searchQuery.value) return ''
  if (hasInvalidSearchQuery.value) return props.searchInvalidRegexpText
  if (totalSearchMatches.value === 0) return props.searchNoMatchesText
  return String(totalSearchMatches.value)
})

function getEditorThemeExtensions() {
  return getHttpCodeThemeExtensions(
    shouldHighlightTrafficMessageSyntax(props.messageType),
    isDarkHttpEditorTheme()
      ? {
          backgroundColor: '#111827',
          color: '#e5e7eb',
          gutterBackgroundColor: '#0f172a',
          gutterColor: '#64748b',
          gutterBorderRight: '1px solid #1e293b',
          activeLineBackgroundColor: 'transparent',
          activeLineGutterBackgroundColor: '#0f172a',
          selectionBackgroundColor: '#1f3a5f',
          caretColor: 'transparent',
        }
      : {
          backgroundColor: '#ffffff',
          color: '#1f2937',
          gutterBackgroundColor: '#f6f7f9',
          gutterColor: '#8b93a1',
          gutterBorderRight: '1px solid #d9dde4',
          activeLineBackgroundColor: 'transparent',
          activeLineGutterBackgroundColor: '#f6f7f9',
          selectionBackgroundColor: '#d7e8ff',
          caretColor: 'transparent',
        },
  )
}

function buildSearchQuery() {
  return new SearchQuery({
    search: searchQuery.value,
    caseSensitive: searchCaseSensitive.value,
    regexp: searchRegexp.value,
  })
}

const diffTheme = EditorView.theme({
  '&': {
    height: '100%',
  },
  '.cm-mergeView': {
    height: '100%',
  },
  '.cm-merge-a, .cm-merge-b': {
    height: '100%',
  },
  '.cm-editor': {
    height: '100%',
    fontSize: 'var(--traffic-editor-font-size, 13px)',
    fontVariantLigatures: 'none',
    cursor: 'text',
    userSelect: 'text',
  },
  '.cm-scroller': {
    overflow: 'auto',
    overscrollBehavior: 'contain',
    scrollbarGutter: 'stable',
    fontFamily: 'var(--traffic-editor-font-family)',
    fontSize: 'var(--traffic-editor-font-size)',
  },
  '.cm-content': {
    padding: '1px 0 4px',
    userSelect: 'text',
  },
  '.cm-line': {
    padding: '0 8px',
    minHeight: '13px',
    lineHeight: '13px',
  },
  '.cm-gutters': {
    minWidth: '3rem',
    userSelect: 'none',
  },
  '.cm-gutterElement': {
    minHeight: '13px',
    lineHeight: '13px',
    paddingRight: '0.65rem',
  },
  '&.cm-focused': {
    outline: 'none',
  },
})

function focusSearchInput() {
  requestAnimationFrame(() => {
    searchInput.value?.focus()
    searchInput.value?.select()
  })
}

function createSearchKeymap() {
  return keymap.of([
    {
      key: 'Mod-f',
      run: () => {
        focusSearchInput()
        return true
      },
    },
    {
      key: 'F3',
      run: () => findNextMatch(),
    },
    {
      key: 'Shift-F3',
      run: () => findPreviousMatch(),
    },
    {
      key: 'Mod-g',
      run: () => findNextMatch(),
    },
    {
      key: 'Shift-Mod-g',
      run: () => findPreviousMatch(),
    },
  ])
}

function getReadonlyExtensions(content: string) {
  const highlightEnabled = shouldHighlightTrafficMessageSyntax(props.messageType)
  return [
    lineNumbers(),
    drawSelection(),
    highlightActiveLineGutter(),
    createSearchKeymap(),
    search(),
    ...(highlightEnabled ? getHttpEditorLanguageExtensions(content) : []),
    ...getEditorThemeExtensions(),
    diffTheme,
    EditorState.readOnly.of(true),
    EditorView.editable.of(false),
    EditorView.lineWrapping,
  ]
}

function handleEditorContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function activateSearchSide(side: 'a' | 'b') {
  activeSearchSide.value = side
}

function countMatches(view: EditorView) {
  const query = getSearchQuery(view.state)
  if (!query.search || !query.valid) return 0
  const cursor = query.getCursor(view.state)
  let total = 0
  for (let next = cursor.next(); !next.done; next = cursor.next()) {
    total += 1
  }
  return total
}

function recountMatches() {
  if (!mergeView) {
    totalSearchMatches.value = 0
    return
  }
  totalSearchMatches.value = countMatches(mergeView.a) + countMatches(mergeView.b)
}

function applySearchState(navigateToMatch = false) {
  if (!mergeView) return
  const query = buildSearchQuery()
  mergeView.a.dispatch({ effects: setSearchQuery.of(query) })
  mergeView.b.dispatch({ effects: setSearchQuery.of(query) })
  if (navigateToMatch && query.search && query.valid) {
    const targetView = resolveActiveSearchView()
    if (targetView) findNext(targetView)
  }
  recountMatches()
}

function resolveActiveSearchView() {
  if (!mergeView) return null
  const primary = activeSearchSide.value === 'a' ? mergeView.a : mergeView.b
  const fallback = activeSearchSide.value === 'a' ? mergeView.b : mergeView.a
  return countMatches(primary) > 0 ? primary : countMatches(fallback) > 0 ? fallback : primary
}

function clearSearch() {
  searchQuery.value = ''
  applySearchState(false)
  const activeView = resolveActiveSearchView()
  activeView?.focus()
}

function findNextMatch() {
  const view = resolveActiveSearchView()
  if (!view || !canNavigateSearchResults.value) return false
  activateSearchSide(view === mergeView?.a ? 'a' : 'b')
  const handled = findNext(view)
  recountMatches()
  return handled
}

function findPreviousMatch() {
  const view = resolveActiveSearchView()
  if (!view || !canNavigateSearchResults.value) return false
  activateSearchSide(view === mergeView?.a ? 'a' : 'b')
  const handled = findPrevious(view)
  recountMatches()
  return handled
}

function handleSearchEnter(event: KeyboardEvent) {
  if (event.shiftKey) {
    findPreviousMatch()
    return
  }
  findNextMatch()
}

function toggleCaseSensitive() {
  searchCaseSensitive.value = !searchCaseSensitive.value
}

function toggleRegexp() {
  searchRegexp.value = !searchRegexp.value
}

function initMergeView() {
  if (!containerRef.value) return

  containerRef.value.innerHTML = ''
  if (mergeView) {
    mergeView.a.dom.removeEventListener('focusin', handleEditorAFocus)
    mergeView.b.dom.removeEventListener('focusin', handleEditorBFocus)
    mergeView.a.dom.removeEventListener('mousedown', handleEditorAMousedown)
    mergeView.b.dom.removeEventListener('mousedown', handleEditorBMouseDown)
    mergeView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    mergeView.destroy()
    mergeView = null
  }

  mergeView = new MergeView({
    a: {
      doc: props.leftText,
      extensions: getReadonlyExtensions(props.leftText),
    },
    b: {
      doc: props.rightText,
      extensions: getReadonlyExtensions(props.rightText),
    },
    parent: containerRef.value,
    collapseUnchanged: { margin: 3, minSize: 4 },
    orientation: 'a-b',
  })

  mergeView.a.dom.addEventListener('focusin', handleEditorAFocus)
  mergeView.b.dom.addEventListener('focusin', handleEditorBFocus)
  mergeView.a.dom.addEventListener('mousedown', handleEditorAMousedown)
  mergeView.b.dom.addEventListener('mousedown', handleEditorBMouseDown)
  mergeView.dom.addEventListener('contextmenu', handleEditorContextMenu, { capture: true })
  applySearchState(false)
  recountMatches()
}

function handleEditorAFocus() {
  activateSearchSide('a')
}

function handleEditorBFocus() {
  activateSearchSide('b')
}

function handleEditorAMousedown() {
  activateSearchSide('a')
}

function handleEditorBMouseDown() {
  activateSearchSide('b')
}

watch(searchQuery, () => {
  applySearchState(true)
})

watch(searchCaseSensitive, () => {
  applySearchState(true)
})

watch(searchRegexp, () => {
  applySearchState(true)
})

onMounted(() => {
  initMergeView()

  themeObserver = new MutationObserver(() => {
    initMergeView()
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})

watch(
  () => [props.leftText, props.rightText],
  () => {
    initMergeView()
  },
)

watch(
  () => [props.messageType, settings.value.highlightRequestSyntax, settings.value.highlightResponseSyntax],
  () => {
    initMergeView()
  },
)

onUnmounted(() => {
  if (mergeView) {
    mergeView.a.dom.removeEventListener('focusin', handleEditorAFocus)
    mergeView.b.dom.removeEventListener('focusin', handleEditorBFocus)
    mergeView.a.dom.removeEventListener('mousedown', handleEditorAMousedown)
    mergeView.b.dom.removeEventListener('mousedown', handleEditorBMouseDown)
    mergeView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    mergeView.destroy()
    mergeView = null
  }
  if (themeObserver) {
    themeObserver.disconnect()
  }
})
</script>

<style scoped>
.code-diff-viewer {
  display: flex;
  flex-direction: column;
}

.diff-editor-container {
  flex: 1 1 auto;
}

.editor-search-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  flex: 0 0 auto;
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

:deep(.cm-mergeView) {
  height: 100%;
}

:deep(.cm-merge-a),
:deep(.cm-merge-b) {
  flex: 1;
}

:deep(.cm-merge-spacer) {
  border-left: 1px solid oklch(var(--b3));
  border-right: 1px solid oklch(var(--b3));
}
</style>
