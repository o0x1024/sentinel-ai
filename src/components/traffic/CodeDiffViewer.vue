<template>
  <div
    ref="containerRef"
    class="h-full min-h-0 overflow-hidden rounded-lg border border-base-300 bg-base-100"
    :style="editorStyle"
  ></div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { EditorView } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { MergeView } from '@codemirror/merge'
import { drawSelection, highlightActiveLineGutter, lineNumbers } from '@codemirror/view'
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
}>(), {
  leftText: '',
  rightText: '',
  messageType: 'generic',
})

const emit = defineEmits<{
  (e: 'contextmenu', event: MouseEvent): void
}>()

const containerRef = ref<HTMLDivElement | null>(null)
let mergeView: MergeView | null = null
let themeObserver: MutationObserver | null = null
const { settings } = useTrafficDisplaySettings()
const editorStyle = computed(() => ({
  '--traffic-editor-font-size': `${settings.value.fontSize}px`,
  '--traffic-editor-font-family': settings.value.fontFamily,
}))

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

function getReadonlyExtensions(content: string) {
  const highlightEnabled = shouldHighlightTrafficMessageSyntax(props.messageType)
  return [
    lineNumbers(),
    drawSelection(),
    highlightActiveLineGutter(),
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

function initMergeView() {
  if (!containerRef.value) return

  containerRef.value.innerHTML = ''
  if (mergeView) {
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

  mergeView.dom.addEventListener('contextmenu', handleEditorContextMenu, { capture: true })
}

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
