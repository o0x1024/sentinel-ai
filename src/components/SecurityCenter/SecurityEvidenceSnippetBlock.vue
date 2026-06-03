<template>
  <div class="space-y-3">
    <div v-if="entries.length" class="grid gap-3 md:grid-cols-2">
      <div
        v-for="entry in entries"
        :key="`${entry.key}:${entry.value}`"
        class="rounded-lg border border-base-300 bg-base-200 p-3 space-y-2"
      >
        <p class="text-xs font-medium uppercase tracking-wide text-base-content/60">
          {{ entry.label }}
        </p>
        <span
          v-if="isCompactValue(entry.value)"
          class="badge badge-outline badge-sm font-mono max-w-full break-all"
        >
          {{ entry.value }}
        </span>
        <pre
          v-else
          class="m-0 overflow-x-auto rounded-lg border border-base-300 bg-base-100 px-3 py-2 font-mono text-xs whitespace-pre-wrap break-all text-base-content"
          >{{ entry.value }}</pre
        >
      </div>
    </div>

    <pre
      v-if="showPlainSnippet"
      class="bg-base-300 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words"
      >{{ snippet }}</pre
    >
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { parseSecurityEvidenceSnippetEntries } from './securityEvidenceSnippetSupport'

const COMPACT_VALUE_THRESHOLD = 56

const props = defineProps<{
  snippet?: string | null
}>()

const normalizedSnippet = computed(() => props.snippet || '')
const entries = computed(() => parseSecurityEvidenceSnippetEntries(normalizedSnippet.value))
const showPlainSnippet = computed(() => Boolean(normalizedSnippet.value.trim()))

function isCompactValue(value: string) {
  const normalized = value.trim()
  if (!normalized) return false
  return !/\s/.test(normalized) && normalized.length <= COMPACT_VALUE_THRESHOLD
}
</script>
