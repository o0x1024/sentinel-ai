<template>
  <div class="group/agent-identity rounded-lg border border-base-300 bg-base-200/40 p-4">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0 flex-1 space-y-2">
        <div v-if="eyebrow" class="text-xs font-medium uppercase tracking-wide text-base-content/50">
          {{ eyebrow }}
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <slot name="title">
            <div class="text-lg font-semibold text-base-content">{{ title }}</div>
          </slot>
          <slot name="badges" />
        </div>

        <slot name="description">
          <div v-if="description" class="text-sm leading-6 text-base-content/70">
            {{ description }}
          </div>
        </slot>

        <div v-if="normalizedMetaItems.length" class="flex flex-wrap gap-2 pt-1">
          <div
            v-for="item in normalizedMetaItems"
            :key="`${item.label}-${item.value}`"
            class="rounded-md border border-base-300 bg-base-100 px-3 py-1 text-xs text-base-content/70"
          >
            <span class="font-medium text-base-content/60">{{ item.label }}:</span>
            <span class="ml-1">{{ item.value }}</span>
          </div>
        </div>
      </div>

      <div v-if="$slots.actions" class="shrink-0">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

type AgentMetaItem = {
  label: string
  value: string
}

const props = defineProps<{
  title: string
  description?: string
  eyebrow?: string
  metaItems?: AgentMetaItem[]
}>()

const normalizedMetaItems = computed(() =>
  (props.metaItems || []).filter(item => item.label.trim() && item.value.trim())
)
</script>
