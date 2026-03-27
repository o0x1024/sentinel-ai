<template>
  <div
    class="inline-flex h-4 w-4 items-center justify-center"
    :class="toneClass"
    :title="label"
    :aria-label="label"
  >
    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="1.8"
        :d="iconPath"
      />
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  type?: string
  label: string
}>()

const iconPathMap: Record<string, string> = {
  org: 'M4 20h16M6 20V8l6-4 6 4v12M9 11h.01M9 14h.01M9 17h.01M15 11h.01M15 14h.01M15 17h.01',
  domain: 'M12 21c4.97 0 9-4.03 9-9s-4.03-9-9-9-9 4.03-9 9 4.03 9 9 9Zm0 0c2.2 0 4-4.03 4-9s-1.8-9-4-9-4 4.03-4 9 1.8 9 4 9Zm-8-9h16',
  ip: 'M4 7h16M6 4h12a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm2 12v4m8-4v4m-6 0h4',
  host: 'M4 5h16v10H4zM8 19h8M10 15v4M14 15v4',
  port: 'M7 7h10v10H7zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4',
  service: 'M10 4h4l1 2h3v4l-2 1 2 1v4h-3l-1 2h-4l-1-2H6v-4l2-1-2-1V6h3l1-2Zm2 5.5A2.5 2.5 0 1 0 12 14.5a2.5 2.5 0 0 0 0-5Z',
  web: 'M4 6h16v12H4zM4 10h16M8 8h.01M12 8h.01',
  certificate: 'M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6l7-3Zm-2 9 1.5 1.5L15 10',
}

const toneClass = computed(() => {
  switch (props.type) {
    case 'web':
      return 'text-primary'
    case 'certificate':
      return 'text-success'
    case 'service':
      return 'text-secondary'
    case 'port':
      return 'text-warning'
    case 'ip':
    case 'host':
      return 'text-info'
    default:
      return 'text-base-content/70'
  }
})

const iconPath = computed(() => iconPathMap[props.type || ''] || iconPathMap.host)
</script>
