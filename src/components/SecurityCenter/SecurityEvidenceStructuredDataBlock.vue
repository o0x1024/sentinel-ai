<template>
  <div v-if="data != null" class="rounded-lg border border-base-300 bg-base-200 p-3 space-y-3">
    <div class="flex flex-wrap items-center gap-2 text-xs text-base-content/70">
      <span class="badge badge-outline badge-sm">{{ dataKindLabel }}</span>
      <span v-if="entryCountLabel" class="badge badge-ghost badge-sm">{{ entryCountLabel }}</span>
    </div>

    <div class="max-h-80 overflow-auto rounded-lg border border-base-300 bg-base-100 p-3">
      <JsonViewer :data="data" :root-name="''" :expanded="true" />
    </div>

    <details class="rounded-lg border border-base-300 bg-base-100">
      <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-base-content/70">
        原始 JSON
      </summary>
      <div class="border-t border-base-300 p-3">
        <pre class="text-xs font-mono whitespace-pre-wrap break-all overflow-auto">{{
          rawJson
        }}</pre>
      </div>
    </details>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import JsonViewer from '@/components/Tools/JsonViewer.vue'

const props = defineProps<{
  data?: unknown
}>()

const dataKindLabel = computed(() => {
  if (Array.isArray(props.data)) return 'Array'
  if (props.data && typeof props.data === 'object') return 'Object'
  return typeof props.data
})

const entryCountLabel = computed(() => {
  if (Array.isArray(props.data)) {
    return `${props.data.length} items`
  }
  if (props.data && typeof props.data === 'object') {
    return `${Object.keys(props.data as Record<string, unknown>).length} keys`
  }
  return ''
})

const rawJson = computed(() => JSON.stringify(props.data, null, 2))
</script>
