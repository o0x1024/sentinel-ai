<template>
  <div class="flex flex-wrap items-center gap-2">
    <span
      v-for="hint in hints"
      :key="`${hint.key}-${hint.label}`"
      class="inline-flex items-center gap-2 rounded-full border px-2.5 py-1 text-[11px]"
      :class="hint.active
        ? 'border-primary/30 bg-primary/10 text-primary'
        : 'border-base-300 bg-base-100 text-base-content/55'"
    >
      <kbd class="kbd kbd-xs">{{ hint.key }}</kbd>
      <span>{{ hint.label }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import type { CommandPaletteFooterHint } from '@/services/commandPaletteFooterHints'

defineProps<{
  hints: CommandPaletteFooterHint[]
}>()
</script>
