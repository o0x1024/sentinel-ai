<template>
  <div
    class="http-code-editor"
    :class="[
      editorThemeMode,
      {
        'fullscreen': fullscreen,
        'readonly-mode': readonly,
        'pretty-mode': displayMode === 'pretty',
        'raw-mode': displayMode === 'raw',
        'with-search-bar': showSearchBar,
      },
    ]"
    :style="editorStyle"
  >
    <div v-if="showDisplayToolbar" class="editor-topbar">
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
    </div>
    <div ref="editorContainer" class="editor-container"></div>
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
import {
  computed,
  nextTick,
  onActivated,
  onDeactivated,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from 'vue'
import { EditorState, Compartment } from '@codemirror/state'
import { EditorView, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars, keymap, lineNumbers } from '@codemirror/view'
import { defaultKeymap, indentWithTab, history, undo, redo } from '@codemirror/commands'
import { SearchQuery, findNext, findPrevious, getSearchQuery, search, setSearchQuery } from '@codemirror/search'
import { useI18n } from 'vue-i18n'
import { getHttpCodeThemeExtensions, isDarkHttpEditorTheme } from './httpEditorTheme'
import { computeMinimalTextChange } from './httpEditorContentSync'
import { createLineEndingIndicatorExtension, getDetectedLineEndingLabel } from './httpEditorDisplayExtensions'
import { createSearchHighlightExtension } from './httpEditorSearchHighlight'
import { getHttpEditorLanguageExtensions, getHttpLanguageSignature } from './httpEditorHttpMode'
import { useHttpEditorSyntaxWarmup } from './useHttpEditorSyntaxWarmup'
import {
  shouldHighlightTrafficMessageSyntax,
  TRAFFIC_MESSAGE_TEXT_LINE_HEIGHT,
  useTrafficDisplaySettings,
  type TrafficMessageType,
} from '@/components/traffic/trafficDisplaySettings'
import { getIntruderMarkerEditorExtensions } from '@/components/traffic/intruder/intruderMarkerEditorExtension'

const scrollStateCache = new Map<string, { top: number; left: number }>()
const MAX_SCROLL_STATE_CACHE_SIZE = 100
const HTTP_EDITOR_SCROLL_STATE_STORAGE_PREFIX = 'sentinel:http-editor-scroll:'

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
  showDisplayToolbar?: boolean
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
  showDisplayToolbar: true,
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
let pendingScrollRestoreState: { top: number; left: number } | null = null
let pendingScrollRestoreFrames = 0
let pendingScrollRestoreAnimationFrame: number | null = null
const editorThemeMode = ref<'burp-light' | 'burp-dark'>(isDarkHttpEditorTheme() ? 'burp-dark' : 'burp-light')
const readOnlyCompartment = new Compartment()
const editableCompartment = new Compartment()
const lineWrapCompartment = new Compartment()
const lineEndingCompartment = new Compartment()
const languageCompartment = new Compartment()
const themeCompartment = new Compartment()
const markerCompartment = new Compartment()
const readonlyAccessibilityCompartment = new Compartment()
const searchCompartment = new Compartment()
const searchHighlightCompartment = new Compartment()
const { t } = useI18n()
const { settings } = useTrafficDisplaySettings()
const editorStyle = computed(() => ({
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
  '--traffic-editor-line-height': String(TRAFFIC_MESSAGE_TEXT_LINE_HEIGHT),
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
const {
  forceVisibleSyntaxHighlight,
  cancelPendingSyntaxForce,
  observeEditorContainer,
  disconnectEditorResizeObserver,
} = useHttpEditorSyntaxWarmup({
  getEditorView: () => editorView,
  getEditorContainer: () => editorContainer.value,
  getMessageType: () => props.messageType,
})
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
        backgroundColor: '#1f2329',
        color: '#e6edf3',
        gutterBackgroundColor: '#181b20',
        gutterColor: '#8b949e',
        gutterBorderRight: '1px solid #30363d',
        activeLineBackgroundColor: '#22272e',
        activeLineGutterBackgroundColor: '#22272e',
        selectionBackgroundColor: '#264f78',
        caretColor: props.readonly ? 'transparent' : '#e6edf3',
      }
    : {
        backgroundColor: '#ffffff',
        color: '#111111',
        gutterBackgroundColor: '#f3f3f3',
        gutterColor: '#707070',
        gutterBorderRight: '1px solid #d4d4d4',
        activeLineBackgroundColor: '#fffdf5',
        activeLineGutterBackgroundColor: '#ececec',
        selectionBackgroundColor: '#cfe3ff',
        caretColor: props.readonly ? 'transparent' : '#111111',
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

function getSearchHighlightExtensions() {
  if (!props.showSearchBar) {
    return []
  }

  return createSearchHighlightExtension(buildSearchQuery())
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
    highlightSpecialChars(),
  ]

  if (!props.readonly) {
    baseEditorExtensions.push(
      highlightActiveLineGutter(),
      highlightActiveLine(),
    )
  }

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
  if (
    !scroller.isConnected
    || scroller.clientHeight <= 0
    || scroller.scrollHeight <= 0
  ) {
    return
  }
  const scrollState = {
    top: scroller.scrollTop,
    left: scroller.scrollLeft,
  }
  if (scrollStateCache.has(stateKey)) {
    scrollStateCache.delete(stateKey)
  }
  scrollStateCache.set(stateKey, scrollState)
  while (scrollStateCache.size > MAX_SCROLL_STATE_CACHE_SIZE) {
    const oldestKey = scrollStateCache.keys().next().value
    if (!oldestKey) break
    scrollStateCache.delete(oldestKey)
  }

  try {
    window.sessionStorage.setItem(
      `${HTTP_EDITOR_SCROLL_STATE_STORAGE_PREFIX}${stateKey}`,
      JSON.stringify(scrollState),
    )
  } catch {
    // Ignore storage quota or privacy-mode failures and keep in-memory fallback.
  }
}

function saveScrollState() {
  saveScrollStateForKey(props.stateKey)
}

function restoreScrollState() {
  if (!props.stateKey || !editorView) return
  const state = scrollStateCache.get(props.stateKey) ?? loadPersistedScrollState(props.stateKey)
  if (!state) return
  const scroller = editorView.scrollDOM
  scroller.scrollTop = state.top
  scroller.scrollLeft = state.left
}

function loadPersistedScrollState(stateKey: string) {
  try {
    const persisted = window.sessionStorage.getItem(`${HTTP_EDITOR_SCROLL_STATE_STORAGE_PREFIX}${stateKey}`)
    if (!persisted) return null
    const parsed = JSON.parse(persisted)
    if (!Number.isFinite(parsed?.top) || !Number.isFinite(parsed?.left)) {
      return null
    }
    const state = {
      top: parsed.top,
      left: parsed.left,
    }
    scrollStateCache.set(stateKey, state)
    return state
  } catch {
    return null
  }
}

function restoreScrollStateWithRetries(retries = 4) {
  if (!props.stateKey) {
    return
  }

  const state = scrollStateCache.get(props.stateKey) ?? loadPersistedScrollState(props.stateKey)
  if (!state) {
    return
  }

  pendingScrollRestoreState = state
  pendingScrollRestoreFrames = Math.max(retries, 1)

  if (pendingScrollRestoreAnimationFrame !== null) {
    cancelAnimationFrame(pendingScrollRestoreAnimationFrame)
  }

  const applyPendingScrollRestore = () => {
    if (!editorView || !pendingScrollRestoreState) {
      pendingScrollRestoreAnimationFrame = null
      return
    }

    const scroller = editorView.scrollDOM
    scroller.scrollTop = pendingScrollRestoreState.top
    scroller.scrollLeft = pendingScrollRestoreState.left

    pendingScrollRestoreFrames -= 1
    if (pendingScrollRestoreFrames <= 0) {
      pendingScrollRestoreAnimationFrame = null
      pendingScrollRestoreState = null
      return
    }

    pendingScrollRestoreAnimationFrame = requestAnimationFrame(applyPendingScrollRestore)
  }

  pendingScrollRestoreAnimationFrame = requestAnimationFrame(applyPendingScrollRestore)
}

function cancelPendingScrollRestore() {
  if (pendingScrollRestoreAnimationFrame !== null) {
    cancelAnimationFrame(pendingScrollRestoreAnimationFrame)
    pendingScrollRestoreAnimationFrame = null
  }
  pendingScrollRestoreState = null
  pendingScrollRestoreFrames = 0
}

function destroyEditorView() {
  if (!editorView) {
    return
  }

  cancelPendingScrollRestore()
  cancelPendingSyntaxForce()

  saveScrollState()
  if (contextMenuListenerAttached) {
    editorView.dom.removeEventListener('contextmenu', handleEditorContextMenu, { capture: true })
    contextMenuListenerAttached = false
  }
  editorView.scrollDOM.removeEventListener('scroll', saveScrollState)
  editorView.destroy()
  editorView = null
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
    effects: [
      setSearchQuery.of(query),
      searchHighlightCompartment.reconfigure(getSearchHighlightExtensions()),
    ],
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
    destroyEditorView()
  }

  editorContainer.value.innerHTML = ''
  editorThemeMode.value = isDarkHttpEditorTheme() ? 'burp-dark' : 'burp-light'
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
      searchHighlightCompartment.of(getSearchHighlightExtensions()),
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

  forceVisibleSyntaxHighlight(8)
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
      forceVisibleSyntaxHighlight()
    }
    return
  }

  editorView.dispatch({
    changes: minimalChange,
    effects,
  })
  forceVisibleSyntaxHighlight()
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
  forceVisibleSyntaxHighlight()
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
  editorThemeMode.value = isDarkHttpEditorTheme() ? 'burp-dark' : 'burp-light'
  const nextSignature = getThemeSignature()
  if (nextSignature === currentThemeSignature) return
  currentThemeSignature = nextSignature
  editorView.dispatch({
    effects: themeCompartment.reconfigure(getThemeExtensions()),
  })
  forceVisibleSyntaxHighlight()
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
    effects: [
      searchCompartment.reconfigure(getSearchExtensions()),
      searchHighlightCompartment.reconfigure(getSearchHighlightExtensions()),
    ],
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

  restoreScrollStateWithRetries(24)
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
  observeEditorContainer()

  themeObserver = new MutationObserver(() => {
    updateTheme()
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
  restoreScrollStateWithRetries(24)
})

onUnmounted(() => {
  destroyEditorView()
  if (themeObserver) {
    themeObserver.disconnect()
  }
  disconnectEditorResizeObserver()
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
  border: 1px solid #d4d4d4;
  background: #ffffff;
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

.editor-topbar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  flex: 0 0 auto;
  min-height: 2.2rem;
  padding: 0.35rem 0.45rem 0.2rem;
}

.editor-display-toolbar {
  display: flex;
  gap: 0.25rem;
}

.editor-display-button {
  min-width: 1.85rem;
  height: 1.65rem;
  padding: 0 0.45rem;
  border: 1px solid #cfcfcf;
  background: linear-gradient(180deg, #ffffff 0%, #f1f1f1 100%);
  color: #4a4a4a;
  box-shadow: 0 1px 1px rgb(0 0 0 / 0.05);
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

.http-code-editor.readonly-mode .editor-search-bar {
  gap: 0.375rem;
  padding: 0.3rem 0.4rem;
}

.http-code-editor.readonly-mode .editor-search-actions {
  gap: 0.2rem;
}

.http-code-editor.readonly-mode .editor-search-bar :deep(.input.input-sm) {
  min-height: 1.8rem;
  height: 1.8rem;
  padding-inline: 0.55rem;
}

.http-code-editor.readonly-mode .editor-search-bar :deep(.btn.btn-xs) {
  min-height: 1.7rem;
  height: 1.7rem;
  padding-inline: 0.45rem;
}

.http-code-editor.readonly-mode .editor-search-bar :deep(.btn.btn-xs i) {
  font-size: 0.72rem;
}

:deep(.cm-editor) {
  height: 100%;
  font-size: var(--traffic-editor-font-size, 13px);
  line-height: var(--traffic-editor-line-height, 1.3);
  font-variant-ligatures: none;
}

:deep(.cm-scroller) {
  overflow: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

:deep(.cm-content) {
  user-select: text;
  line-height: var(--traffic-editor-line-height, 1.3);
}

:deep(.cm-gutterElement) {
  line-height: var(--traffic-editor-line-height, 1.3);
  padding-right: 0.65rem;
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

:deep(.cm-lineNumbers .cm-gutterElement) {
  font-variant-numeric: tabular-nums;
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

:deep(.cm-content ::selection) {
  background-color: var(--traffic-selection-bg, #cfe3ff);
}

:deep(.cm-line-ending-indicator) {
  display: inline-block;
  margin-left: 0.2rem;
  color: #9aa0a6;
  font-size: 0.72em;
  font-weight: 600;
  letter-spacing: 0.02em;
  pointer-events: none;
  user-select: none;
}

.http-code-editor.burp-light .editor-search-bar :deep(.input) {
  border-color: #c8c8c8;
  background: #ffffff;
  box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.04);
}

.http-code-editor.burp-light {
  --traffic-selection-bg: rgb(207 227 255 / 0.9);
}

.http-code-editor.burp-light .editor-search-bar :deep(.btn) {
  border-color: #cfcfcf;
  background: linear-gradient(180deg, #ffffff 0%, #f1f1f1 100%);
  color: #444444;
}

.http-code-editor.burp-light .editor-search-bar :deep(.btn.btn-active) {
  border-color: #a9bfdc;
  background: linear-gradient(180deg, #e9f2ff 0%, #d5e7ff 100%);
  color: #1f4d9a;
}

.http-code-editor.burp-light .editor-topbar {
  border-bottom: 1px solid #e6e6e6;
  background: linear-gradient(180deg, #fafafa 0%, #f3f3f3 100%);
}

.http-code-editor.burp-light.readonly-mode .editor-search-bar {
  border-top-color: #e6e6e6;
  background: linear-gradient(180deg, #f8f8f8 0%, #f2f2f2 100%);
}

.http-code-editor.burp-light.readonly-mode .editor-search-bar :deep(.input) {
  box-shadow: none;
}

.http-code-editor.burp-light.readonly-mode .editor-search-bar :deep(.btn) {
  box-shadow: none;
}

.http-code-editor.burp-dark {
  border-color: #30363d;
  background: #1f2329;
  --traffic-selection-bg: rgb(38 79 120 / 0.8);
}

.http-code-editor.burp-dark .editor-display-button {
  border-color: #3d444d;
  background: linear-gradient(180deg, #2d333b 0%, #252b32 100%);
  color: #d0d7de;
  box-shadow: none;
}

.http-code-editor.burp-dark .editor-search-bar {
  border-top-color: #30363d;
  background: #181b20;
}

.http-code-editor.burp-dark .editor-topbar {
  border-bottom: 1px solid #30363d;
  background: linear-gradient(180deg, #22272e 0%, #1b2026 100%);
}

.http-code-editor.burp-dark.readonly-mode .editor-search-bar {
  border-top-color: #2b3138;
  background: linear-gradient(180deg, #1c2128 0%, #171b20 100%);
}

.http-code-editor.burp-dark.readonly-mode .editor-search-bar :deep(.input) {
  background: #1f252c;
}

.http-code-editor.burp-dark .editor-search-bar :deep(.input) {
  border-color: #3d444d;
  background: #22272e;
  color: #e6edf3;
  box-shadow: none;
}

.http-code-editor.burp-dark .editor-search-bar :deep(.btn) {
  border-color: #3d444d;
  background: linear-gradient(180deg, #2d333b 0%, #252b32 100%);
  color: #d0d7de;
}

.http-code-editor.burp-dark .editor-search-bar :deep(.btn.btn-active) {
  border-color: #4f7cac;
  background: linear-gradient(180deg, #23476b 0%, #1c3d5d 100%);
  color: #f0f6fc;
}
</style>
