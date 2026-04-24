<template>
  <pre v-if="normalizedText" :class="blockClass">{{ normalizedText }}</pre>
</template>

<script setup lang="ts">
import { computed } from 'vue'

type SecurityEvidenceTextSize = 'sm' | 'xs'

const props = withDefaults(
  defineProps<{
    text?: string | null
    mono?: boolean
    size?: SecurityEvidenceTextSize
  }>(),
  {
    text: '',
    mono: false,
    size: 'sm',
  }
)

const normalizedText = computed(() => (props.text || '').trim())

const blockClass = computed(() => {
  const sizeClass = props.size === 'xs' ? 'text-xs' : 'text-sm'
  const breakClass = props.mono ? 'break-all font-mono' : 'break-words'
  return [
    'm-0 overflow-auto rounded-lg border border-base-300 bg-base-200 px-3 py-2 whitespace-pre-wrap text-base-content',
    sizeClass,
    breakClass,
  ]
})
</script>
