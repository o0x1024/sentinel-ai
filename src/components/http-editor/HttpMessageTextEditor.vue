<template>
  <div
    class="http-code-editor"
    :class="{
      'fullscreen': fullscreen,
      'readonly-mode': readonly,
      'pretty-mode': displayMode === 'pretty',
      'raw-mode': displayMode === 'raw',
    }"
    :style="editorStyle"
  >
    <div ref="editorContainer" class="editor-container"></div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState, Compartment } from '@codemirror/state'
import { drawSelection, highlightActiveLineGutter, keymap, lineNumbers } from '@codemirror/view'
import { defaultKeymap, indentWithTab, history, undo, redo } from '@codemirror/commands'
import { getHttpCodeThemeExtensions, isDarkHttpEditorTheme } from './httpEditorTheme'
import { getHttpEditorLanguageExtensions, getHttpLanguageSignature } from './httpEditorHttpMode'
import { shouldHighlightTrafficMessageSyntax, useTrafficDisplaySettings, type TrafficMessageType } from '@/components/traffic/trafficDisplaySettings'

const scrollStateCache = new Map<string, { top: number; left: number }>()

const props = withDefaults(defineProps<{
  modelValue: string
  readonly?: boolean
  height?: string
  fullscreen?: boolean
  placeholder?: string
  messageType?: TrafficMessageType
  displayMode?: 'pretty' | 'raw'
  stateKey?: string
}>(), {
  modelValue: '',
  readonly: false,
  height: '100%',
  fullscreen: false,
  placeholder: '',
  messageType: 'generic',
  displayMode: 'raw',
  stateKey: '',
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const editorContainer = ref<HTMLDivElement>()
let editorView: EditorView | null = null
let currentLanguageSignature = ''
const readOnlyCompartment = new Compartment()
const editableCompartment = new Compartment()
const { settings } = useTrafficDisplaySettings()
const editorStyle = computed(() => ({
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
}))

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

function getLanguageSignature(content: string) {
  if (!shouldHighlightTrafficMessageSyntax(props.messageType)) {
    return `${props.messageType}:plain`
  }
  return `${props.messageType}:${getHttpLanguageSignature(content)}`
}

function getLanguageExtensions() {
  if (!shouldHighlightTrafficMessageSyntax(props.messageType)) return []
  currentLanguageSignature = getLanguageSignature(props.modelValue)
  return getHttpEditorLanguageExtensions(props.modelValue)
}

function getBaseExtensions() {
  if (!props.readonly) {
    return [
      basicSetup,
      keymap.of([...defaultKeymap, indentWithTab]),
      history(),
    ]
  }

  return [
    lineNumbers(),
    drawSelection(),
    highlightActiveLineGutter(),
  ]
}

function saveScrollStateForKey(stateKey: string) {
  if (!stateKey || !editorView) return
  const scroller = editorView.scrollDOM
  scrollStateCache.set(stateKey, {
    top: scroller.scrollTop,
    left: scroller.scrollLeft,
  })
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

function initEditor() {
  if (!editorContainer.value) return

  if (editorView) {
    saveScrollState()
    editorView.scrollDOM.removeEventListener('scroll', saveScrollState)
    editorView.destroy()
    editorView = null
  }

  editorContainer.value.innerHTML = ''

  const state = EditorState.create({
    doc: props.modelValue,
    extensions: [
      ...getBaseExtensions(),
      ...getLanguageExtensions(),
      ...getThemeExtensions(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !props.readonly) {
          emit('update:modelValue', update.state.doc.toString())
        }
      }),
      readOnlyCompartment.of(EditorState.readOnly.of(props.readonly)),
      editableCompartment.of(EditorView.editable.of(!props.readonly)),
      EditorView.lineWrapping,
    ],
  })

  editorView = new EditorView({
    state,
    parent: editorContainer.value,
  })

  editorView.scrollDOM.addEventListener('scroll', saveScrollState, { passive: true })
  requestAnimationFrame(() => {
    restoreScrollState()
  })
}

function updateContent(content: string) {
  if (!editorView) return
  const currentContent = editorView.state.doc.toString()
  if (currentContent !== content) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: currentContent.length,
        insert: content,
      },
    })
  }
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

defineExpose({
  focus: () => editorView?.focus(),
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

watch(() => props.modelValue, (newVal) => {
  if (getLanguageSignature(newVal) !== currentLanguageSignature) {
    initEditor()
    return
  }
  updateContent(newVal)
})

watch(() => props.readonly, (newVal) => {
  updateReadonly(newVal)
})

watch(() => props.stateKey, (newKey, oldKey) => {
  if (oldKey && oldKey !== newKey) {
    saveScrollStateForKey(oldKey)
  }

  requestAnimationFrame(() => {
    restoreScrollState()
  })
})

watch(
  () => [
    props.messageType,
    props.readonly,
    settings.value.highlightRequestSyntax,
    settings.value.highlightResponseSyntax,
  ],
  () => {
    initEditor()
  },
)

let themeObserver: MutationObserver | null = null

onMounted(async () => {
  await nextTick()
  initEditor()

  themeObserver = new MutationObserver(() => {
    initEditor()
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})

onUnmounted(() => {
  saveScrollState()
  if (editorView) {
    editorView.scrollDOM.removeEventListener('scroll', saveScrollState)
    editorView.destroy()
    editorView = null
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
  padding: 0 0 6px;
  user-select: text;
}

.http-code-editor.pretty-mode :deep(.cm-content) {
  padding: 2px 0 8px;
}

.http-code-editor.readonly-mode :deep(.cm-content) {
  padding: 1px 0 4px;
}

.http-code-editor.readonly-mode.pretty-mode :deep(.cm-content) {
  padding: 2px 0 6px;
}

.http-code-editor.raw-mode :deep(.cm-content) {
  padding: 0 0 2px;
}

:deep(.cm-line) {
  padding: 0 8px;
  min-height: 13px;
  line-height: 13px;
}

.http-code-editor.pretty-mode :deep(.cm-line) {
  padding: 0 10px 0 8px;
}

:deep(.cm-gutters) {
  min-width: 3rem;
  user-select: none;
}

:deep(.cm-gutterElement) {
  min-height: 13px;
  line-height: 13px;
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
</style>
