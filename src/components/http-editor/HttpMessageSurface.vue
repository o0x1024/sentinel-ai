<template>
  <div class="http-message-surface" :style="surfaceStyle">
    <HttpMessageTextEditor
      ref="surfaceEditor"
      :model-value="modelValue"
      :readonly="readonly"
      :custom-context-menu="customContextMenu"
      :message-type="messageType"
      :height="height"
      :fullscreen="fullscreen"
      :placeholder="placeholder"
      :display-mode="displayMode"
      :state-key="stateKey"
      :marker-mode="markerMode"
      :show-search-bar="showSearchBar"
      :search-placeholder="searchPlaceholder"
      :search-next-title="searchNextTitle"
      :search-previous-title="searchPreviousTitle"
      :search-case-sensitive-title="searchCaseSensitiveTitle"
      :search-regexp-title="searchRegexpTitle"
      :search-clear-title="searchClearTitle"
      :search-no-matches-text="searchNoMatchesText"
      :search-invalid-regexp-text="searchInvalidRegexpText"
      :show-display-toolbar="showDisplayToolbar"
      @update:model-value="handleModelValueUpdate"
      @contextmenu="emit('contextmenu', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { TrafficMessageType } from '@/components/traffic/trafficDisplaySettings'
import HttpMessageTextEditor from './HttpMessageTextEditor.vue'

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

const surfaceEditor = ref<{
  focus?: () => void
  focusSearch?: () => void
  getContent?: () => string
  getSelectionRange?: () => { from: number; to: number }
  undo?: () => void
  redo?: () => void
  selectAll?: () => void
  setSelection?: (from: number, to: number) => void
} | null>(null)

const surfaceStyle = computed(() => ({
  height: props.height,
}))

function handleModelValueUpdate(value: string) {
  if (props.readonly) return
  emit('update:modelValue', value)
}

defineExpose({
  focus: () => surfaceEditor.value?.focus?.(),
  focusSearch: () => surfaceEditor.value?.focusSearch?.(),
  getContent: () => surfaceEditor.value?.getContent?.() || props.modelValue,
  getSelectionRange: () => surfaceEditor.value?.getSelectionRange?.() || { from: 0, to: 0 },
  undo: () => surfaceEditor.value?.undo?.(),
  redo: () => surfaceEditor.value?.redo?.(),
  selectAll: () => surfaceEditor.value?.selectAll?.(),
  setSelection: (from: number, to: number) => surfaceEditor.value?.setSelection?.(from, to),
})
</script>

<style scoped>
.http-message-surface {
  width: 100%;
  min-height: 0;
}
</style>
