<template>
  <div class="http-code-editor" :class="{ 'fullscreen': fullscreen }">
    <div ref="editorContainer" class="editor-container"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState, Compartment } from '@codemirror/state'
import { keymap } from '@codemirror/view'
import { defaultKeymap, indentWithTab, history, undo, redo } from '@codemirror/commands'
import { StreamLanguage } from '@codemirror/language'
import { oneDark } from '@codemirror/theme-one-dark'

// HTTP 语法高亮定义
const httpLanguage = StreamLanguage.define({
  token(stream, state: any) {
    // 请求行: GET /path HTTP/1.1
    if (state.lineNumber === 0 || state.isFirstLine) {
      state.isFirstLine = false
      
      // HTTP Method
      if (stream.match(/^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS|CONNECT|TRACE)\b/)) {
        return 'keyword'
      }
      // HTTP Version
      if (stream.match(/HTTP\/[\d.]+/)) {
        return 'atom'
      }
      // URL Path
      if (stream.match(/\/[^\s]*/)) {
        return 'string'
      }
      // Status code
      if (stream.match(/\d{3}/)) {
        return 'number'
      }
      // Status text
      if (stream.match(/[A-Za-z\s]+$/)) {
        return 'comment'
      }
      stream.next()
      return null
    }
    
    // Empty line - body starts
    if (stream.sol() && stream.match(/^\s*$/)) {
      state.inBody = true
      stream.skipToEnd()
      return null
    }
    
    // Body content
    if (state.inBody) {
      // JSON key
      if (stream.match(/"[^"]+"\s*:/)) {
        return 'property'
      }
      // JSON string value
      if (stream.match(/"[^"]*"/)) {
        return 'string'
      }
      // JSON number
      if (stream.match(/-?\d+\.?\d*/)) {
        return 'number'
      }
      // JSON keywords
      if (stream.match(/\b(true|false|null)\b/)) {
        return 'keyword'
      }
      // HTML tag
      if (stream.match(/<\/?[a-zA-Z][a-zA-Z0-9-]*/)) {
        return 'tag'
      }
      // HTML attribute
      if (stream.match(/\s[a-zA-Z-]+=/)) {
        return 'attribute'
      }
      stream.next()
      return null
    }
    
    // Header name
    if (stream.sol() && stream.match(/^[A-Za-z-]+(?=:)/)) {
      return 'property'
    }
    
    // Header colon
    if (stream.match(/^:\s*/)) {
      return 'punctuation'
    }
    
    // Rest of header value
    stream.skipToEnd()
    return 'string'
  },
  startState() {
    return { lineNumber: 0, isFirstLine: true, inBody: false }
  },
  blankLine(state: any) {
    state.lineNumber++
    state.inBody = true
  },
  copyState(state: any) {
    return { 
      lineNumber: state.lineNumber, 
      isFirstLine: state.isFirstLine, 
      inBody: state.inBody 
    }
  }
})

const props = withDefaults(defineProps<{
  modelValue: string
  readonly?: boolean
  height?: string
  fullscreen?: boolean
  placeholder?: string
}>(), {
  modelValue: '',
  readonly: false,
  height: '100%',
  fullscreen: false,
  placeholder: ''
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const editorContainer = ref<HTMLDivElement>()
let editorView: EditorView | null = null
const readOnlyCompartment = new Compartment()

// Light theme
const lightTheme = EditorView.theme({
  '&': {
    backgroundColor: 'oklch(var(--b1))',
    color: 'oklch(var(--bc))',
  },
  '.cm-content': {
    caretColor: 'oklch(var(--bc))',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  '.cm-cursor': {
    borderLeftColor: 'oklch(var(--bc))',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: 'oklch(var(--p) / 0.2)',
  },
  '.cm-gutters': {
    backgroundColor: 'oklch(var(--b2))',
    color: 'oklch(var(--bc) / 0.4)',
    borderRight: '1px solid oklch(var(--b3))',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'oklch(var(--b3))',
  },
  '.cm-activeLine': {
    backgroundColor: 'oklch(var(--b2) / 0.5)',
  },
}, { dark: false })

// Dark theme
const darkTheme = EditorView.theme({
  '&': {
    backgroundColor: '#1e1e2e',
    color: '#cdd6f4',
  },
  '.cm-content': {
    caretColor: '#cdd6f4',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  '.cm-cursor': {
    borderLeftColor: '#cdd6f4',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: '#45475a',
  },
  '.cm-gutters': {
    backgroundColor: '#181825',
    color: '#6c7086',
    borderRight: '1px solid #313244',
  },
  '.cm-activeLineGutter': {
    backgroundColor: '#313244',
  },
  '.cm-activeLine': {
    backgroundColor: '#313244',
  },
}, { dark: true })

// Syntax highlighting for light theme
const lightHighlightStyle = EditorView.baseTheme({
  '.cm-keyword': { color: '#d20f39' },      // HTTP methods
  '.cm-atom': { color: '#7287fd' },         // HTTP version
  '.cm-string': { color: '#40a02b' },       // Values
  '.cm-number': { color: '#fe640b' },       // Numbers, status codes
  '.cm-property': { color: '#1e66f5' },     // Header names, JSON keys
  '.cm-punctuation': { color: '#6c6f85' },  // Colons
  '.cm-comment': { color: '#8c8fa1' },      // Status text
  '.cm-tag': { color: '#8839ef' },          // HTML tags
  '.cm-attribute': { color: '#e64553' },    // HTML attributes
})

// Syntax highlighting for dark theme
const darkHighlightStyle = EditorView.baseTheme({
  '.cm-keyword': { color: '#f38ba8' },      // HTTP methods
  '.cm-atom': { color: '#89b4fa' },         // HTTP version
  '.cm-string': { color: '#a6e3a1' },       // Values
  '.cm-number': { color: '#fab387' },       // Numbers, status codes
  '.cm-property': { color: '#89dceb' },     // Header names, JSON keys
  '.cm-punctuation': { color: '#9399b2' },  // Colons
  '.cm-comment': { color: '#6c7086' },      // Status text
  '.cm-tag': { color: '#cba6f7' },          // HTML tags
  '.cm-attribute': { color: '#eba0ac' },    // HTML attributes
})

function isDarkTheme(): boolean {
  return document.documentElement.getAttribute('data-theme') === 'dark'
}

function getThemeExtensions() {
  const dark = isDarkTheme()
  return dark 
    ? [oneDark, darkHighlightStyle]
    : [lightTheme, lightHighlightStyle]
}

function initEditor() {
  if (!editorContainer.value) return
  
  // Destroy existing
  if (editorView) {
    editorView.destroy()
    editorView = null
  }
  
  editorContainer.value.innerHTML = ''
  
  const state = EditorState.create({
    doc: props.modelValue,
    extensions: [
      basicSetup,
      httpLanguage,
      ...getThemeExtensions(),
      keymap.of([...defaultKeymap, indentWithTab]),
      history(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          emit('update:modelValue', update.state.doc.toString())
        }
      }),
      readOnlyCompartment.of(EditorView.editable.of(!props.readonly)),
      EditorView.lineWrapping,
    ],
  })
  
  editorView = new EditorView({
    state,
    parent: editorContainer.value,
  })
}

// Update content from outside
function updateContent(content: string) {
  if (!editorView) return
  const currentContent = editorView.state.doc.toString()
  if (currentContent !== content) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: currentContent.length,
        insert: content
      }
    })
  }
}

// Update readonly state
function updateReadonly(readonly: boolean) {
  if (!editorView) return
  editorView.dispatch({
    effects: readOnlyCompartment.reconfigure(EditorView.editable.of(!readonly))
  })
}

// Expose methods
defineExpose({
  focus: () => editorView?.focus(),
  getContent: () => editorView?.state.doc.toString() || '',
  undo: () => editorView && undo(editorView),
  redo: () => editorView && redo(editorView),
  selectAll: () => {
    if (!editorView) return
    editorView.dispatch({
      selection: { anchor: 0, head: editorView.state.doc.length }
    })
    editorView.focus()
  },
})

// Watch props
watch(() => props.modelValue, (newVal) => {
  updateContent(newVal)
})

watch(() => props.readonly, (newVal) => {
  updateReadonly(newVal)
})

// Watch theme changes
let themeObserver: MutationObserver | null = null

onMounted(async () => {
  await nextTick()
  initEditor()
  
  // Observe theme changes
  themeObserver = new MutationObserver(() => {
    initEditor()
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme']
  })
})

onUnmounted(() => {
  if (editorView) {
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
  font-size: var(--font-size-base, 14px);
}

:deep(.cm-scroller) {
  overflow: auto;
}

:deep(.cm-content) {
  padding: 8px 0;
}

:deep(.cm-line) {
  padding: 0 8px;
}

:deep(.cm-gutters) {
  min-width: 3rem;
}

:deep(.cm-gutter-lint) {
  width: 0;
}

/* Readonly styling */
:deep(.cm-editor.cm-focused) {
  outline: none;
}
</style>

