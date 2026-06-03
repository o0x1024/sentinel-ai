<template>
  <div ref="editorRoot" class="h-full min-h-0 w-full overflow-hidden bg-base-100"></div>
</template>

<script setup lang="ts">
import { basicSetup } from 'codemirror'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { json } from '@codemirror/lang-json'
import { javascript } from '@codemirror/lang-javascript'
import { EditorState, type Extension } from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

const props = defineProps<{
  modelValue: string
  filePath?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const editorRoot = ref<HTMLElement | null>(null)
let editorView: EditorView | null = null
let applyingExternalValue = false

const fileExtension = computed(() => {
  const path = (props.filePath || '').toLowerCase()
  const index = path.lastIndexOf('.')
  return index >= 0 ? path.slice(index + 1) : ''
})

const languageExtension = computed<Extension | null>(() => {
  if (fileExtension.value === 'json') return json()
  if (['js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs'].includes(fileExtension.value)) {
    return javascript({ typescript: ['ts', 'tsx'].includes(fileExtension.value), jsx: ['jsx', 'tsx'].includes(fileExtension.value) })
  }
  return null
})

const buildEditorState = (doc: string) => {
  const extensions: Extension[] = [
    basicSetup,
    history(),
    keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap]),
    EditorView.lineWrapping,
    EditorView.theme({
      '&': {
        height: '100%',
        fontSize: 'inherit',
      },
      '.cm-scroller': {
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
        lineHeight: '1.5',
      },
      '.cm-content': {
        minHeight: '100%',
      },
    }),
    EditorView.updateListener.of((update) => {
      if (!update.docChanged || applyingExternalValue) return
      emit('update:modelValue', update.state.doc.toString())
    }),
  ]
  if (languageExtension.value) extensions.push(languageExtension.value)
  return EditorState.create({ doc, extensions })
}

const recreateEditor = () => {
  if (!editorRoot.value) return
  editorView?.destroy()
  editorView = new EditorView({
    parent: editorRoot.value,
    state: buildEditorState(props.modelValue),
  })
}

watch(
  () => props.modelValue,
  (value) => {
    if (!editorView) return
    const currentValue = editorView.state.doc.toString()
    if (currentValue === value) return
    applyingExternalValue = true
    editorView.dispatch({
      changes: { from: 0, to: editorView.state.doc.length, insert: value },
    })
    applyingExternalValue = false
  },
)

watch(fileExtension, () => {
  recreateEditor()
})

onMounted(() => {
  recreateEditor()
})

onUnmounted(() => {
  editorView?.destroy()
  editorView = null
})
</script>
