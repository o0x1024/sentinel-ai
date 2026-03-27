<template>
  <div ref="containerRef" class="h-full min-h-0 overflow-hidden rounded-lg border border-base-300 bg-base-100"></div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { MergeView } from '@codemirror/merge'
import { StreamLanguage } from '@codemirror/language'
import { http } from '@codemirror/legacy-modes/mode/http'
import { oneDark } from '@codemirror/theme-one-dark'

const httpLanguage = StreamLanguage.define(http)

const props = withDefaults(defineProps<{
  leftText: string
  rightText: string
}>(), {
  leftText: '',
  rightText: '',
})

const containerRef = ref<HTMLDivElement | null>(null)
let mergeView: MergeView | null = null

const diffTheme = EditorView.theme({
  '&': {
    height: '100%',
  },
  '.cm-mergeView': {
    height: '100%',
  },
  '.cm-editor': {
    height: '100%',
  },
  '.cm-scroller': {
    overflow: 'auto',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
}, { dark: true })

function initMergeView() {
  if (!containerRef.value) return

  containerRef.value.innerHTML = ''
  if (mergeView) {
    mergeView.destroy()
    mergeView = null
  }

  mergeView = new MergeView({
    a: {
      doc: props.leftText,
      extensions: [basicSetup, httpLanguage, oneDark, diffTheme, EditorState.readOnly.of(true), EditorView.lineWrapping],
    },
    b: {
      doc: props.rightText,
      extensions: [basicSetup, httpLanguage, oneDark, diffTheme, EditorState.readOnly.of(true), EditorView.lineWrapping],
    },
    parent: containerRef.value,
    collapseUnchanged: { margin: 3, minSize: 4 },
    orientation: 'a-b',
  })
}

onMounted(initMergeView)

watch(
  () => [props.leftText, props.rightText],
  () => {
    initMergeView()
  },
)

onUnmounted(() => {
  if (mergeView) {
    mergeView.destroy()
    mergeView = null
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
