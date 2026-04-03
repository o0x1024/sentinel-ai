<template>
  <div class="http-message-surface" :style="surfaceStyle">
    <HttpMessageTextEditor
      v-if="readonly"
      ref="surfaceEditor"
      :model-value="modelValue"
      readonly
      :message-type="messageType"
      :height="height"
      :display-mode="displayMode"
      :state-key="stateKey"
    />
    <HttpMessageTextEditor
      v-else
      ref="surfaceEditor"
      :model-value="modelValue"
      :readonly="readonly"
      :message-type="messageType"
      :height="height"
      :fullscreen="fullscreen"
      :placeholder="placeholder"
      :display-mode="displayMode"
      :state-key="stateKey"
      @update:model-value="emit('update:modelValue', $event)"
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

const surfaceEditor = ref<{
  focus?: () => void
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

defineExpose({
  focus: () => surfaceEditor.value?.focus?.(),
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
