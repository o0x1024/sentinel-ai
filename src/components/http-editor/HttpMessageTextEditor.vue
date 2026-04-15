<template>
  <div
    class="http-code-editor"
    :class="{
      'fullscreen': fullscreen,
      'readonly-mode': readonly,
      'pretty-mode': displayMode === 'pretty',
      'raw-mode': displayMode === 'raw',
      'with-search-bar': showSearchBar,
    }"
    :style="editorStyle"
  >
    <div class="editor-display-toolbar">
      <button
        type="button"
        class="btn btn-ghost btn-xs editor-display-button"
        :class="{ 'btn-active': settings.showLineEndings }"
        :title="lineEndingToggleTitle"
        @click="toggleLineEndingIndicators"
      >
        <span class="editor-display-button-label">↵</span>
      </button>
      <button
        type="button"
        class="btn btn-ghost btn-xs editor-display-button"
        :class="{ 'btn-active': settings.wrapLongLines }"
        :title="lineWrapToggleTitle"
        @click="toggleLineWrap"
      >
        <i class="fas fa-text-width"></i>
      </button>
    </div>
    <div ref="editorContainer" class="editor-container"></div>
    <div v-if="showSearchBar" class="editor-search-bar border-t border-base-300 bg-base-200/95">
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
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { EditorState, Compartment } from '@codemirror/state'
import { drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars, keymap, lineNumbers } from '@codemirror/view'
import { defaultKeymap, indentWithTab, history, undo, redo } from '@codemirror/commands'
import { SearchQuery, findNext, findPrevious, getSearchQuery, search, setSearchQuery } from '@codemirror/search'
import { useI18n } from 'vue-i18n'
import { getHttpCodeThemeExtensions, isDarkHttpEditorTheme } from './httpEditorTheme'
import { computeMinimalTextChange } from './httpEditorContentSync'
import { createLineEndingIndicatorExtension, getDetectedLineEndingLabel } from './httpEditorDisplayExtensions'
import { getHttpEditorLanguageExtensions, getHttpLanguageSignature } from './httpEditorHttpMode'
import { shouldHighlightTrafficMessageSyntax, useTrafficDisplaySettings, type TrafficMessageType } from '@/components/traffic/trafficDisplaySettings'
import { getIntruderMarkerEditorExtensions } from '@/components/traffic/intruder/intruderMarkerEditorExtension'

const scrollStateCache = new Map<string, { top: number; left: number }>()
const MAX_SCROLL_STATE_CACHE_SIZE = 100

const props = withDefaults(defineProps<{
  modelValue: string
  readonly?: boolean
  height?: string
  fullscreen?: boolean
  customContextMenu?: boolean
  placeholder?: string
  messageType?: TrafficMessageType
  displayMode?: 'pretty' | 'raw'
  stateKey?: string
  markerMode?: 'none' | 'intruder'
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
  readonly: false,
  height: '100%',
  fullscreen: false,
  customContextMenu: false,
  placeholder: '',
  messageType: 'generic',
  displayMode: 'raw',
  stateKey: '',
  markerMode: 'none',
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
  (e: 'update:modelValue', value: string): void
  (e: 'contextmenu', event: MouseEvent): void
}>()

const editorContainer = ref<HTMLDivElement>()
const searchInput = ref<HTMLInputElement>()
const searchQuery = ref('')
const searchCaseSensitive = ref(false)
const searchRegexp = ref(false)
const totalSearchMatches = ref(0)
const activeSearchMatch = ref(0)
let editorView: EditorView | null = null
let currentLanguageSignature = ''
let currentThemeSignature = ''
let currentMarkerSignature = ''
let currentReadonlyAccessibilitySignature = ''
let currentSearchExtensionSignature = ''
let contextMenuListenerAttached = false
const readOnlyCompartment = new Compartment()
const editableCompartment = new Compartment()
const lineWrapCompartment = new Compartment()
const lineEndingCompartment = new Compartment()
const languageCompartment = new Compartment()
const themeCompartment = new Compartment()
const markerCompartment = new Compartment()
const readonlyAccessibilityCompartment = new Compartment()
const searchCompartment = new Compartment()
const { t } = useI18n()
const { settings } = useTrafficDisplaySettings()
const editorStyle = computed(() => ({
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
}))
const lineEndingToggleTitle = computed(() => (
  settings.value.showLineEndings
    ? t('trafficAnalysis.httpEditor.toolbar.hideLineEndings')
    : t('trafficAnalysis.httpEditor.toolbar.showLineEndings')
))
const lineWrapToggleTitle = computed(() => (
  settings.value.wrapLongLines
    ? t('trafficAnalysis.httpEditor.toolbar.disableLineWrap')
    : t('trafficAnalysis.httpEditor.toolbar.enableLineWrap')
))
const hasInvalidSearchQuery = computed(() => {
  if (!searchQuery.value || !props.showSearchBar) return false
  return !buildSearchQuery().valid
})
const canNavigateSearchResults = computed(() => {
  if (!searchQuery.value) return false
  return totalSearchMatches.value > 0 && !hasInvalidSearchQuery.value
})
const searchStatusText = computed(() => {
  if (!searchQuery.value) return ''
  if (hasInvalidSearchQuery.value) return props.searchInvalidRegexpText
  if (totalSearchMatches.value === 0) return props.searchNoMatchesText
  return `${activeSearchMatch.value || 1}/${totalSearchMatches.value}`
})

function getThemeExtensions() {
  const highlightEnabled = shouldHighlightTrafficMessageSyntax(props.messageType)
  return getHttpCodeThemeExtensions(highlightEnabled, isDarkHttpEditorTheme()
    ? {
        backgroundColor: '#111827',
        color: '#e5e7eb',
        gutterBackgroundColor: '#0f172a',
        gutterColor: '#64748b',
        gutterBorderRight: '1px solid #1e293b',
        activeLineBackgroundColor: 'transparent',
        activeLineGutterBackgroundColor: '#0f172a',
        selectionBackgroundColor: '#1f3a5f',
        caretColor: props.readonly ? 'transparent' : '#e5e7eb',
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
        caretColor: props.readonly ? 'transparent' : '#1f2937',
      })
}

function getThemeSignature() {
  return [
    props.messageType,
    shouldHighlightTrafficMessageSyntax(props.messageType) ? 'highlight' : 'plain',
    props.readonly ? 'readonly' : 'editable',
    isDarkHttpEditorTheme() ? 'dark' : 'light',
  ].join(':')
}

function getLanguageSignature(content: string) {
  if (!shouldHighlightTrafficMessageSyntax(props.messageType)) {
    return `${props.messageType}:plain`
  }
  return `${props.messageType}:${getHttpLanguageSignature(content)}`
}

function getLanguageExtensions(content: string) {
  if (!shouldHighlightTrafficMessageSyntax(props.messageType)) return []
  return getHttpEditorLanguageExtensions(content)
}

function getMarkerExtensions() {
  return props.markerMode === 'intruder' ? getIntruderMarkerEditorExtensions() : []
}

function getSearchExtensions() {
  return props.showSearchBar ? [search()] : []
}

function getReadonlyAccessibilityExtensions() {
  return props.readonly
    ? [EditorView.contentAttributes.of({ tabindex: '0' })]
    : []
}

function getEditorContent() {
  return editorView?.state.doc.toString() ?? props.modelValue
}

function getLineWrapExtension() {
  return settings.value.wrapLongLines ? EditorView.lineWrapping : []
}

function getLineEndingExtension(content: string) {
  return settings.value.showLineEndings
    ? createLineEndingIndicatorExtension(getDetectedLineEndingLabel(content))
    : []
}

function getBaseExtensions() {
  const sharedKeymap = keymap.of([
    {
      key: 'Mod-a',
      run: () => {
        if (!editorView) return false
        editorView.dispatch({
          selection: { anchor: 0, head: editorView.state.doc.length },
        })
        editorView.focus()
        return true
      },
    },
    {
      key: 'Mod-f',
      run: () => {
        if (!props.showSearchBar) return false
        focusSearchInput()
        return true
      },
    },
    {
      key: 'F3',
      run: () => {
        if (!props.showSearchBar) return false
        return findNextMatch()
      },
    },
    {
      key: 'Shift-F3',
      run: () => {
        if (!props.showSearchBar) return false
        return findPreviousMatch()
      },
    },
    {
      key: 'Mod-g',
      run: () => {
        if (!props.showSearchBar) return false
        return findNextMatch()
      },
    },
    {
      key: 'Shift-Mod-g',
      run: () => {
        if (!props.showSearchBar) return false
        return findPreviousMatch()
      },
    },
  ])
  const baseEditorExtensions = [
    lineNumbers(),
    drawSelection(),
    highlightSpecialChars(),
    highlightActiveLineGutter(),
    highlightActiveLine(),
  ]

  return [
    sharedKeymap,
    ...baseEditorExtensions,
    history(),
    keymap.of([...defaultKeymap, indentWithTab]),
  ]
}

function handleEditorContextMenu(event: MouseEvent) {
  if (!props.customContextMenu) return
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function updateContextMenuBinding() {
  if (!editorView) return
  if (props.customContextMenu && !contextMenuListenerAttached) {
    editorView.dom.addEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    contextMenuListenerAttached = true
    return
  }
  if (!props.customContextMenu && contextMenuListenerAttached) {
    editorView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    contextMenuListenerAttached = false
  }
}

function saveScrollStateForKey(stateKey: string) {
  if (!stateKey || !editorView) return
  const scroller = editorView.scrollDOM
  if (scrollStateCache.has(stateKey)) {
    scrollStateCache.delete(stateKey)
  }
  scrollStateCache.set(stateKey, {
    top: scroller.scrollTop,
    left: scroller.scrollLeft,
  })
  while (scrollStateCache.size > MAX_SCROLL_STATE_CACHE_SIZE) {
    const oldestKey = scrollStateCache.keys().next().value
    if (!oldestKey) break
    scrollStateCache.delete(oldestKey)
  }
}

function saveScrollState() {
  saveScrollStateForKey(props.stateKey)
}

function restoreScrollState() {
  if (!props.stateKey || !editorView) return
  const state = scrollStateCache.get(props.stateKey)
  if (!state) return
  const scroller = editorView.scrollDOM
  scroller.scrollTop = state.top
  scroller.scrollLeft = state.left
}

function buildSearchQuery() {
  return new SearchQuery({
    search: searchQuery.value,
    caseSensitive: searchCaseSensitive.value,
    regexp: searchRegexp.value,
  })
}

function syncLocalSearchState() {
  if (!editorView || !props.showSearchBar) return
  const query = getSearchQuery(editorView.state)
  if (searchQuery.value !== query.search) searchQuery.value = query.search
  if (searchCaseSensitive.value !== query.caseSensitive) searchCaseSensitive.value = query.caseSensitive
  if (searchRegexp.value !== query.regexp) searchRegexp.value = query.regexp
}

function updateSearchMetrics() {
  if (!editorView || !props.showSearchBar) {
    totalSearchMatches.value = 0
    activeSearchMatch.value = 0
    return
  }

  const query = getSearchQuery(editorView.state)
  if (!query.search || !query.valid) {
    totalSearchMatches.value = 0
    activeSearchMatch.value = 0
    return
  }

  const cursor = query.getCursor(editorView.state)
  const selection = editorView.state.selection.main
  let total = 0
  let active = 0

  for (let next = cursor.next(); !next.done; next = cursor.next()) {
    total += 1
    const match = next.value
    const overlapsSelection = (selection.from === match.from && selection.to === match.to)
      || (selection.from === selection.to && selection.from >= match.from && selection.from <= match.to)
      || (selection.from < match.to && selection.to > match.from)

    if (overlapsSelection && active === 0) {
      active = total
    }
  }

  totalSearchMatches.value = total
  activeSearchMatch.value = total === 0 ? 0 : active || 1
}

function applySearchState(navigateToMatch = false) {
  if (!editorView || !props.showSearchBar) return
  const query = buildSearchQuery()
  editorView.dispatch({
    effects: setSearchQuery.of(query),
  })
  if (navigateToMatch && query.search && query.valid) {
    findNext(editorView)
  }
  syncLocalSearchState()
  updateSearchMetrics()
}

function focusSearchInput() {
  if (!props.showSearchBar) return
  requestAnimationFrame(() => {
    searchInput.value?.focus()
    searchInput.value?.select()
  })
}

function clearSearch() {
  if (!props.showSearchBar) return
  searchQuery.value = ''
  applySearchState(false)
  editorView?.focus()
}

function findNextMatch() {
  if (!editorView || !canNavigateSearchResults.value) return false
  const handled = findNext(editorView)
  updateSearchMetrics()
  return handled
}

function findPreviousMatch() {
  if (!editorView || !canNavigateSearchResults.value) return false
  const handled = findPrevious(editorView)
  updateSearchMetrics()
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

function initEditor() {
  if (!editorContainer.value) return
  let previousSelection: { anchor: number; head: number } | null = null
  let previousFocused = false
  const initialContent = props.modelValue

  if (editorView) {
    previousSelection = {
      anchor: editorView.state.selection.main.anchor,
      head: editorView.state.selection.main.head,
    }
    previousFocused = editorView.hasFocus
    saveScrollState()
    if (props.customContextMenu) {
      editorView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    }
    editorView.scrollDOM.removeEventListener('scroll', saveScrollState)
    editorView.destroy()
    editorView = null
  }

  editorContainer.value.innerHTML = ''
  currentLanguageSignature = getLanguageSignature(initialContent)
  currentThemeSignature = getThemeSignature()
  currentMarkerSignature = props.markerMode
  currentReadonlyAccessibilitySignature = props.readonly ? 'readonly' : 'editable'
  currentSearchExtensionSignature = props.showSearchBar ? 'enabled' : 'disabled'
  contextMenuListenerAttached = false

  const state = EditorState.create({
    doc: initialContent,
    extensions: [
      ...getBaseExtensions(),
      searchCompartment.of(getSearchExtensions()),
      languageCompartment.of(getLanguageExtensions(initialContent)),
      themeCompartment.of(getThemeExtensions()),
      markerCompartment.of(getMarkerExtensions()),
      readonlyAccessibilityCompartment.of(getReadonlyAccessibilityExtensions()),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !props.readonly) {
          emit('update:modelValue', update.state.doc.toString())
        }
        if (
          props.showSearchBar
          && (
            update.selectionSet
            || update.docChanged
            || update.transactions.some(tr => tr.effects.some(effect => effect.is(setSearchQuery)))
          )
        ) {
          syncLocalSearchState()
          updateSearchMetrics()
        }
      }),
      readOnlyCompartment.of(EditorState.readOnly.of(props.readonly)),
      editableCompartment.of(EditorView.editable.of(!props.readonly)),
      lineWrapCompartment.of(getLineWrapExtension()),
      lineEndingCompartment.of(getLineEndingExtension(initialContent)),
    ],
  })

  editorView = new EditorView({
    state,
    parent: editorContainer.value,
  })

  updateContextMenuBinding()
  editorView.scrollDOM.addEventListener('scroll', saveScrollState, { passive: true })
  requestAnimationFrame(() => {
    if (previousSelection && editorView) {
      const docLength = editorView.state.doc.length
      editorView.dispatch({
        selection: {
          anchor: Math.min(previousSelection.anchor, docLength),
          head: Math.min(previousSelection.head, docLength),
        },
      })
    }
    if (previousFocused) {
      editorView?.focus()
    }
    restoreScrollState()
    syncLocalSearchState()
    updateSearchMetrics()
  })
}

function syncContentAndLanguage(content: string) {
  if (!editorView) return
  const currentContent = editorView.state.doc.toString()
  const nextLanguageSignature = getLanguageSignature(content)
  const effects = nextLanguageSignature !== currentLanguageSignature
    ? [languageCompartment.reconfigure(getLanguageExtensions(content))]
    : []

  currentLanguageSignature = nextLanguageSignature

  const minimalChange = computeMinimalTextChange(currentContent, content)
  if (!minimalChange) {
    if (effects.length) {
      editorView.dispatch({ effects })
    }
    return
  }

  editorView.dispatch({
    changes: minimalChange,
    effects,
  })
}

function updateReadonly(readonly: boolean) {
  if (!editorView) return
  editorView.dispatch({
    effects: [
      readOnlyCompartment.reconfigure(EditorState.readOnly.of(readonly)),
      editableCompartment.reconfigure(EditorView.editable.of(!readonly)),
    ],
  })
}

function updateLineWrap(enabled: boolean) {
  if (!editorView) return
  editorView.dispatch({
    effects: lineWrapCompartment.reconfigure(enabled ? EditorView.lineWrapping : []),
  })
}

function updateLineEndings(enabled: boolean) {
  if (!editorView) return
  editorView.dispatch({
    effects: lineEndingCompartment.reconfigure(
      enabled
        ? createLineEndingIndicatorExtension(getDetectedLineEndingLabel(getEditorContent()))
        : [],
    ),
  })
}

function updateTheme() {
  if (!editorView) return
  const nextSignature = getThemeSignature()
  if (nextSignature === currentThemeSignature) return
  currentThemeSignature = nextSignature
  editorView.dispatch({
    effects: themeCompartment.reconfigure(getThemeExtensions()),
  })
}

function updateMarker() {
  if (!editorView) return
  const nextSignature = props.markerMode
  if (nextSignature === currentMarkerSignature) return
  currentMarkerSignature = nextSignature
  editorView.dispatch({
    effects: markerCompartment.reconfigure(getMarkerExtensions()),
  })
}

function updateReadonlyAccessibility() {
  if (!editorView) return
  const nextSignature = props.readonly ? 'readonly' : 'editable'
  if (nextSignature === currentReadonlyAccessibilitySignature) return
  currentReadonlyAccessibilitySignature = nextSignature
  editorView.dispatch({
    effects: readonlyAccessibilityCompartment.reconfigure(getReadonlyAccessibilityExtensions()),
  })
}

function updateSearchExtension() {
  if (!editorView) return
  const nextSignature = props.showSearchBar ? 'enabled' : 'disabled'
  if (nextSignature === currentSearchExtensionSignature) return
  currentSearchExtensionSignature = nextSignature
  editorView.dispatch({
    effects: searchCompartment.reconfigure(getSearchExtensions()),
  })
  if (!props.showSearchBar) {
    totalSearchMatches.value = 0
    activeSearchMatch.value = 0
    return
  }
  syncLocalSearchState()
  updateSearchMetrics()
}

function toggleLineWrap() {
  settings.value.wrapLongLines = !settings.value.wrapLongLines
}

function toggleLineEndingIndicators() {
  settings.value.showLineEndings = !settings.value.showLineEndings
}

defineExpose({
  focus: () => editorView?.focus(),
  focusSearch: focusSearchInput,
  getContent: () => editorView?.state.doc.toString() || '',
  getSelectionRange: () => {
    if (!editorView) return { from: 0, to: 0 }
    const main = editorView.state.selection.main
    return { from: main.from, to: main.to }
  },
  undo: () => editorView && undo(editorView),
  redo: () => editorView && redo(editorView),
  selectAll: () => {
    if (!editorView) return
    editorView.dispatch({
      selection: { anchor: 0, head: editorView.state.doc.length },
    })
    editorView.focus()
  },
  setSelection: (from: number, to: number) => {
    if (!editorView) return
    editorView.dispatch({
      selection: { anchor: from, head: to },
    })
    editorView.focus()
  },
})

watch(searchQuery, () => {
  if (!props.showSearchBar) return
  applySearchState(false)
})

watch(searchCaseSensitive, () => {
  if (!props.showSearchBar) return
  applySearchState(false)
})

watch(searchRegexp, () => {
  if (!props.showSearchBar) return
  applySearchState(false)
})

watch(() => props.modelValue, (newVal) => {
  syncContentAndLanguage(newVal)
  updateLineEndings(settings.value.showLineEndings)
  updateTheme()
  updateSearchMetrics()
})

watch(() => props.readonly, (newVal) => {
  updateReadonly(newVal)
  updateReadonlyAccessibility()
  updateTheme()
})

watch(() => props.stateKey, (newKey, oldKey) => {
  if (oldKey && oldKey !== newKey) {
    saveScrollStateForKey(oldKey)
  }

  requestAnimationFrame(() => {
    restoreScrollState()
  })
})

watch(() => props.showSearchBar, () => {
  updateSearchExtension()
})

watch(() => props.customContextMenu, () => {
  updateContextMenuBinding()
})

watch(
  () => [
    props.messageType,
    props.markerMode,
    settings.value.highlightRequestSyntax,
    settings.value.highlightResponseSyntax,
  ],
  () => {
    const content = getEditorContent()
    syncContentAndLanguage(content)
    updateMarker()
    updateTheme()
  },
)

watch(() => settings.value.wrapLongLines, (enabled) => {
  updateLineWrap(enabled)
})

watch(() => settings.value.showLineEndings, (enabled) => {
  updateLineEndings(enabled)
})

let themeObserver: MutationObserver | null = null

onMounted(async () => {
  await nextTick()
  initEditor()

  themeObserver = new MutationObserver(() => {
    updateTheme()
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})

onUnmounted(() => {
  saveScrollState()
  if (editorView) {
    if (contextMenuListenerAttached) {
      editorView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    }
    editorView.scrollDOM.removeEventListener('scroll', saveScrollState)
    editorView.destroy()
    editorView = null
    contextMenuListenerAttached = false
  }
  if (themeObserver) {
    themeObserver.disconnect()
  }
})
</script>

<style scoped>
.http-code-editor {
  width: 100%;
  height: v-bind(height);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
}

.http-code-editor.fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
}

.editor-container {
  width: 100%;
  height: 100%;
  flex: 1 1 auto;
  min-height: 0;
}

.editor-display-toolbar {
  position: absolute;
  top: 0.35rem;
  right: 0.45rem;
  z-index: 4;
  display: flex;
  gap: 0.25rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 120ms ease;
}

.http-code-editor:hover .editor-display-toolbar,
.http-code-editor:focus-within .editor-display-toolbar {
  opacity: 1;
  pointer-events: auto;
}

.editor-display-button {
  min-width: 1.85rem;
  height: 1.65rem;
  padding: 0 0.45rem;
  border: 1px solid oklch(var(--b3) / 0.85);
  background: oklch(var(--b1) / 0.95);
  box-shadow: 0 4px 14px oklch(0 0 0 / 0.08);
}

.editor-display-button-label {
  font-size: 0.8rem;
  line-height: 1;
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

:deep(.cm-editor) {
  height: 100%;
  font-size: var(--traffic-editor-font-size, 13px);
  font-variant-ligatures: none;
}

:deep(.cm-scroller) {
  overflow: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

:deep(.cm-content) {
  user-select: text;
}

:deep(.cm-line) {
  padding: 0 8px;
}

.http-code-editor.pretty-mode :deep(.cm-line) {
  padding: 0 10px 0 8px;
}

:deep(.cm-gutters) {
  min-width: 3rem;
  user-select: none;
}

:deep(.cm-gutterElement) {
  padding-right: 0.65rem;
}

:deep(.cm-gutter-lint) {
  width: 0;
}

:deep(.cm-editor.cm-focused) {
  outline: none;
}

:deep(.cm-editor) {
  cursor: text;
  user-select: text;
}

:deep(.cm-selectionLayer .cm-selectionBackground) {
  background-color: oklch(var(--p) / 0.2) !important;
  border-radius: 0;
}

:deep(.cm-content ::selection) {
  background-color: transparent;
}

:deep(.cm-line-ending-indicator) {
  display: inline-block;
  margin-left: 0.2rem;
  color: oklch(var(--bc) / 0.32);
  font-size: 0.72em;
  font-weight: 600;
  letter-spacing: 0.02em;
  pointer-events: none;
  user-select: none;
}
</style>
