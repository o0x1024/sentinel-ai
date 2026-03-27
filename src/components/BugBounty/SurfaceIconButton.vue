<template>
  <button
    type="button"
    class="btn btn-ghost btn-xs min-h-0 h-7 px-1.5"
    :class="toneClass"
    :title="label"
    :aria-label="label"
    @click="$emit('click')"
  >
    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="1.8"
        :d="iconPath"
      />
    </svg>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  icon: 'edit' | 'assistant' | 'delete'
  tone?: 'default' | 'primary' | 'error'
}>()

defineEmits<{
  (e: 'click'): void
}>()

const iconPathMap: Record<string, string> = {
  edit: 'M16.862 4.487a2.25 2.25 0 1 1 3.182 3.182L8.25 19.463 4 20l.537-4.25L16.862 4.487ZM15.75 6.75l1.5 1.5',
  assistant: 'M9 10h.01M15 10h.01M9.75 15.5c.7.5 1.45.75 2.25.75s1.55-.25 2.25-.75M8 18h8a3 3 0 0 0 3-3V9a3 3 0 0 0-3-3h-1l-1.2-2h-3.6L9 6H8a3 3 0 0 0-3 3v6a3 3 0 0 0 3 3Z',
  delete: 'M4 7h16M10 11v6M14 11v6M6 7l1 12h10l1-12M9 7V4h6v3',
}

const toneClass = computed(() => {
  if (props.tone === 'primary') return 'text-primary'
  if (props.tone === 'error') return 'text-error'
  return ''
})

const iconPath = computed(() => iconPathMap[props.icon])
</script>
