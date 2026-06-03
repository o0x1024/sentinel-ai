<template>
  <div v-if="items.length" class="space-y-2">
    <span :class="labelClass">{{ label }}</span>
    <div class="space-y-2">
      <template v-for="item in items" :key="item.term">
        <span
          v-if="item.compact"
          class="badge badge-outline badge-sm font-mono max-w-full break-all"
        >
          {{ item.term }}
        </span>
        <pre v-else :class="termBlockClass">{{ item.term }}</pre>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

type SecurityEvidenceHighlightTone = 'request' | 'response'

const LONG_TERM_THRESHOLD = 48

const props = defineProps<{
  label: string
  tone: SecurityEvidenceHighlightTone
  terms: string[]
}>()

const items = computed(() =>
  props.terms.map(term => ({
    term,
    compact: isCompactTerm(term),
  }))
)

const labelClass = computed(() =>
  props.tone === 'request' ? 'badge badge-warning badge-sm' : 'badge badge-success badge-sm'
)

const termBlockClass = computed(() =>
  props.tone === 'request'
    ? 'm-0 overflow-x-auto rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 font-mono text-xs whitespace-pre-wrap break-all text-base-content'
    : 'm-0 overflow-x-auto rounded-lg border border-success/30 bg-success/10 px-3 py-2 font-mono text-xs whitespace-pre-wrap break-all text-base-content'
)

function isCompactTerm(term: string) {
  const normalized = term.trim()
  if (!normalized) return false
  return !/\s/.test(normalized) && normalized.length <= LONG_TERM_THRESHOLD
}
</script>
